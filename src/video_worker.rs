use crate::{
    audio::Mixer,
    audio_capture::Capture as AudioCapture,
    video::{self, Command, Control, Event, Request},
    video_capture::Capture,
    video_file::{self, Staging},
    video_media::Encoder,
};
use std::{
    io::{BufReader, BufWriter, Write},
    path::Path,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

pub fn worker_main() -> Result<(), String> {
    let mut input = BufReader::new(std::io::stdin());
    let request: Request =
        video_file::read_message(&mut input)?.ok_or("Missing video recording request.")?;
    if request.session == 0 {
        return Err("Invalid video session.".into());
    }
    let output_parent = request
        .output
        .parent()
        .ok_or("Invalid recording destination.")?;
    if !video::can_start(video_file::free_space(output_parent)) {
        return send_terminal(Event::Failed {
            session: request.session,
            detail: "At least 512 MiB of free disk space is required.".into(),
            partial: None,
        });
    }
    let audio = match AudioCapture::new(request.audio) {
        Ok(audio) => audio,
        Err(detail) => {
            return send_terminal(Event::Failed {
                session: request.session,
                detail,
                partial: None,
            });
        }
    };
    let capture = match Capture::new(request.source.clone()) {
        Ok(capture) => capture,
        Err(detail) => {
            return send_terminal(Event::Failed {
                session: request.session,
                detail,
                partial: None,
            });
        }
    };
    let (width, height) = capture.dimensions();
    send(&Event::Ready {
        session: request.session,
        width,
        height,
    })?;

    let (commands, command_rx) = mpsc::channel();
    std::thread::spawn(move || {
        loop {
            match video_file::read_message::<Control>(&mut input) {
                Ok(Some(control)) => {
                    if commands.send(Ok(control)).is_err() {
                        break;
                    }
                }
                Ok(None) => {
                    let _ = commands.send(Err("Video controller disconnected.".into()));
                    break;
                }
                Err(error) => {
                    let _ = commands.send(Err(error));
                    break;
                }
            }
        }
    });

    loop {
        match command_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(control))
                if control.session == request.session && control.command == Command::Start =>
            {
                return record(request, capture, audio, command_rx);
            }
            Ok(Ok(control))
                if control.session == request.session && control.command == Command::Stop =>
            {
                return send_terminal(Event::Cancelled {
                    session: request.session,
                });
            }
            Ok(Ok(_)) => {}
            Ok(Err(detail)) => {
                return send_terminal(Event::Failed {
                    session: request.session,
                    detail,
                    partial: None,
                });
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if !capture.is_live() {
                    return send_terminal(Event::Failed {
                        session: request.session,
                        detail: "The recording source was closed or disconnected.".into(),
                        partial: None,
                    });
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("Video controller disconnected.".into());
            }
        }
    }
}

fn record(
    request: Request,
    capture: Capture,
    mut audio: AudioCapture,
    commands: mpsc::Receiver<Result<Control, String>>,
) -> Result<(), String> {
    let staging = match Staging::new(&request.output, request.session) {
        Ok(staging) => staging,
        Err(detail) => {
            return send_terminal(Event::Failed {
                session: request.session,
                detail,
                partial: None,
            });
        }
    };
    let mut encoder = match Encoder::new(
        &staging.path,
        capture.dimensions().0,
        capture.dimensions().1,
        true,
        request.audio.enabled(),
    ) {
        Ok(encoder) => encoder,
        Err(hardware_error) => {
            let _ = std::fs::remove_file(&staging.path);
            match Encoder::new(
                &staging.path,
                capture.dimensions().0,
                capture.dimensions().1,
                false,
                request.audio.enabled(),
            ) {
                Ok(encoder) => encoder,
                Err(software_error) => {
                    return send_terminal(Event::Failed {
                        session: request.session,
                        detail: format!(
                            "Hardware encoder: {hardware_error}; software encoder: {software_error}"
                        ),
                        partial: staging.path.exists().then(|| staging.path.clone()),
                    });
                }
            }
        }
    };
    let mut started = None::<Instant>;
    let mut latest = None::<Arc<crate::frame::Frame>>;
    let mut pending = None::<(Arc<crate::frame::Frame>, u64)>;
    let mut next_tick = Instant::now();
    let mut next_space_check = Instant::now();
    let mut next_progress = Instant::now();
    let mut frames = 0u64;
    let mut first_sample = false;
    let mut reason = "Stopped".to_string();
    let mut failure = None;
    let mut mixer = Mixer::new(request.audio);

    'recording: loop {
        while let Ok(command) = commands.try_recv() {
            match command {
                Ok(control)
                    if control.session == request.session && control.command == Command::Stop =>
                {
                    break 'recording;
                }
                Ok(_) => {}
                Err(detail) => {
                    failure = Some(detail);
                    break 'recording;
                }
            }
        }
        match capture.next() {
            Ok(Some(frame)) => latest = Some(Arc::new(frame)),
            Ok(None) => {}
            Err(detail) => {
                reason = source_end_reason(&detail);
                if !first_sample && pending.is_none() {
                    failure = Some(detail);
                }
                break;
            }
        }
        if latest.is_some() && started.is_none() {
            started = Some(Instant::now());
            if let Err(detail) = audio.start() {
                failure = Some(detail);
                break;
            }
            next_tick = Instant::now();
            next_progress = Instant::now();
        }
        let now = Instant::now();
        if let Some(clock) = started {
            if let Err(detail) = audio.poll(&mut mixer) {
                reason = format!("Audio source ended: {detail}");
                break;
            }
            let elapsed = clock.elapsed().as_millis().min(u64::MAX as u128) as u64;
            if let Err(detail) = write_audio(&mut mixer, &mut encoder, elapsed, false) {
                failure = Some(detail);
                break;
            }
        }
        if now >= next_space_check {
            if video::must_stop(video_file::free_space(
                request.output.parent().unwrap_or(Path::new(".")),
            )) {
                reason = "Low disk space".into();
                break;
            }
            next_space_check = now + Duration::from_secs(1);
        }
        if now >= next_tick {
            let elapsed = started
                .map(|clock| clock.elapsed().as_millis().min(u64::MAX as u128) as u64)
                .unwrap_or(0);
            if let Some((frame, timestamp)) = pending.take() {
                let duration = elapsed.saturating_sub(timestamp).max(1);
                if let Err(detail) = encoder.write(
                    &frame,
                    video::media_time(timestamp)?,
                    video::media_time(duration)?,
                ) {
                    failure = Some(detail);
                    break;
                }
                frames += 1;
                if !first_sample {
                    send(&Event::Started {
                        session: request.session,
                    })?;
                    first_sample = true;
                }
            }
            if let Some(frame) = latest.clone() {
                pending = Some((frame, elapsed));
            }
            next_tick += Duration::from_micros(33_333);
            if next_tick <= now {
                next_tick = now + Duration::from_micros(33_333);
            }
        }
        if now >= next_progress {
            send(&Event::Progress {
                session: request.session,
                elapsed_ms: started
                    .map(|clock| clock.elapsed().as_millis().min(u64::MAX as u128) as u64)
                    .unwrap_or(0),
                frames,
            })?;
            next_progress = now + Duration::from_millis(500);
        }
        std::thread::sleep(Duration::from_millis(2));
    }

    if started.is_some() {
        let _ = audio.poll(&mut mixer);
    }
    audio.stop();
    let elapsed = started
        .map(|clock| clock.elapsed().as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0);
    if let Some((frame, timestamp)) = pending.take() {
        let duration = elapsed.saturating_sub(timestamp).max(1);
        if let Err(detail) = encoder.write(
            &frame,
            video::media_time(timestamp)?,
            video::media_time(duration)?,
        ) {
            failure.get_or_insert(detail);
        } else {
            frames += 1;
            if !first_sample {
                send(&Event::Started {
                    session: request.session,
                })?;
                first_sample = true;
            }
        }
    }
    if failure.is_none() && first_sample {
        if let Err(detail) = write_audio(&mut mixer, &mut encoder, elapsed, true) {
            failure = Some(detail);
        }
    }
    send(&Event::Finalizing {
        session: request.session,
        reason: reason.clone(),
    })?;
    if let Some(detail) = failure {
        drop(encoder);
        return send_terminal(Event::Failed {
            session: request.session,
            detail,
            partial: staging.path.exists().then(|| staging.path.clone()),
        });
    }
    if !first_sample {
        drop(encoder);
        return send_terminal(Event::Failed {
            session: request.session,
            detail: "No recording frames were received.".into(),
            partial: staging.path.exists().then(|| staging.path.clone()),
        });
    }
    if let Err(detail) = encoder.finish() {
        return send_terminal(Event::Failed {
            session: request.session,
            detail,
            partial: staging.path.exists().then(|| staging.path.clone()),
        });
    }
    let partial_path = staging.path.clone();
    match staging.publish() {
        Ok(path) => send_terminal(Event::Saved {
            session: request.session,
            path,
            elapsed_ms: elapsed,
            frames,
            reason,
        }),
        Err(detail) => send_terminal(Event::Failed {
            session: request.session,
            detail,
            partial: partial_path.exists().then_some(partial_path),
        }),
    }
}

fn write_audio(
    mixer: &mut Mixer,
    encoder: &mut Encoder,
    elapsed: u64,
    finalizing: bool,
) -> Result<(), String> {
    let limit = if finalizing {
        crate::audio::CAPACITY_FRAMES / crate::audio::BLOCK_FRAMES + 20
    } else {
        4
    };
    for _ in 0..limit {
        let due = mixer.due(elapsed, finalizing);
        if due == 0 {
            return Ok(());
        }
        let block = mixer.take(due)?;
        encoder.write_audio(block.first_frame, &block.samples)?;
    }
    if finalizing && mixer.due(elapsed, true) != 0 {
        return Err("Audio finalization exceeded its backlog limit.".into());
    }
    Ok(())
}

fn send(event: &Event) -> Result<(), String> {
    let stdout = std::io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    serde_json::to_writer(&mut output, event).map_err(|e| e.to_string())?;
    output.write_all(b"\n").map_err(|e| e.to_string())?;
    output.flush().map_err(|e| e.to_string())
}

fn send_terminal(event: Event) -> Result<(), String> {
    send(&event)
}

fn source_end_reason(detail: &str) -> String {
    format!("Source ended: {detail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completed_recording_retains_the_capture_end_detail() {
        assert_eq!(
            source_end_reason("The recording source changed size."),
            "Source ended: The recording source changed size."
        );
    }
    #[test]
    fn controls_and_events_are_session_scoped_and_bounded_json_lines() {
        let control = Control {
            session: 7,
            command: Command::Stop,
        };
        let bytes = serde_json::to_vec(&control).unwrap();
        assert!(bytes.len() < video::MESSAGE_LIMIT);
        let decoded: Control = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.session, 7);
        assert_eq!(decoded.command, Command::Stop);
        let event = Event::Ready {
            session: 7,
            width: 1920,
            height: 1080,
        };
        let bytes = serde_json::to_vec(&event).unwrap();
        assert!(bytes.len() < video::MESSAGE_LIMIT);
        assert!(matches!(
            serde_json::from_slice::<Event>(&bytes).unwrap(),
            Event::Ready { session: 7, .. }
        ));
    }
}
