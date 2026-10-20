use serde::{Deserialize, Serialize};

pub const RATE: u32 = 48_000;
pub const CHANNELS: usize = 2;
pub const BLOCK_FRAMES: usize = 480;
pub const CAPACITY_FRAMES: usize = 48_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    None,
    System,
    Microphone,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    System,
    Microphone,
}

impl Mode {
    pub fn enabled(self) -> bool {
        self != Self::None
    }
    pub fn includes(self, kind: Kind) -> bool {
        matches!(
            (self, kind),
            (Self::Both, _) | (Self::System, Kind::System) | (Self::Microphone, Kind::Microphone)
        )
    }
}

pub struct Block {
    pub first_frame: u64,
    pub samples: Vec<i16>,
}
struct Ring {
    samples: Vec<[i16; CHANNELS]>,
    stamps: Vec<u64>,
}
pub struct Mixer {
    inputs: [Option<Ring>; 2],
    cursor: u64,
}
impl Mixer {
    pub fn new(mode: Mode) -> Self {
        Self {
            inputs: [Kind::System, Kind::Microphone].map(|kind| {
                mode.includes(kind).then(|| Ring {
                    samples: vec![[0; CHANNELS]; CAPACITY_FRAMES],
                    stamps: vec![u64::MAX; CAPACITY_FRAMES],
                })
            }),
            cursor: 0,
        }
    }
    pub fn push(&mut self, kind: Kind, start: i64, samples: &[i16]) -> Result<(), String> {
        let index = if kind == Kind::System { 0 } else { 1 };
        let ring = self.inputs[index]
            .as_mut()
            .ok_or("The audio input is not selected.")?;
        if samples.len() % CHANNELS != 0 || samples.len() / CHANNELS > CAPACITY_FRAMES {
            return Err("Invalid or excessive audio packet.".into());
        }
        let end = start as i128 + (samples.len() / CHANNELS) as i128;
        if end > self.cursor as i128 + CAPACITY_FRAMES as i128 {
            return Err("Audio timestamps exceed the bounded capture window.".into());
        }
        for (i, frame) in samples.chunks_exact(CHANNELS).enumerate() {
            let position = start as i128 + i as i128;
            if position < self.cursor as i128 {
                continue;
            }
            let position = position as u64;
            let slot = (position % CAPACITY_FRAMES as u64) as usize;
            ring.samples[slot] = [frame[0], frame[1]];
            ring.stamps[slot] = position;
        }
        Ok(())
    }
    pub fn take(&mut self, frames: usize) -> Result<Block, String> {
        let count = self.inputs.iter().flatten().count();
        if count == 0 || frames == 0 || frames > BLOCK_FRAMES {
            return Err("Invalid audio output block.".into());
        }
        let end = self
            .cursor
            .checked_add(frames as u64)
            .ok_or("Audio position overflowed.")?;
        let mut samples = Vec::with_capacity(frames * CHANNELS);
        for position in self.cursor..end {
            let slot = (position % CAPACITY_FRAMES as u64) as usize;
            let mut total = [0i32; CHANNELS];
            for ring in self.inputs.iter_mut().flatten() {
                if ring.stamps[slot] == position {
                    for (sum, sample) in total.iter_mut().zip(ring.samples[slot]) {
                        *sum += sample as i32;
                    }
                    ring.stamps[slot] = u64::MAX;
                }
            }
            samples.extend(total.map(|sample| (sample / count as i32) as i16));
        }
        let block = Block {
            first_frame: self.cursor,
            samples,
        };
        self.cursor = end;
        Ok(block)
    }
    pub fn due(&self, elapsed_ms: u64, finalizing: bool) -> usize {
        if self.inputs.iter().all(Option::is_none) {
            return 0;
        }
        let end = ((elapsed_ms as u128 * RATE as u128) / 1000).min(u64::MAX as u128) as u64;
        let target = if finalizing {
            end
        } else {
            end.saturating_sub(RATE as u64 / 10)
        };
        let available = target.saturating_sub(self.cursor).min(BLOCK_FRAMES as u64) as usize;
        if finalizing || available == BLOCK_FRAMES {
            available
        } else {
            0
        }
    }
}
pub fn frame_time(frame: u64) -> Result<i64, String> {
    i64::try_from(frame as u128 * 10_000_000 / RATE as u128)
        .map_err(|_| "Audio timestamp overflowed.".into())
}
pub fn packet_frame(qpc: u64, epoch: u64) -> Result<i64, String> {
    i64::try_from((qpc as i128 - epoch as i128) * RATE as i128 / 10_000_000)
        .map_err(|_| "Audio capture clock overflowed.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sample_timeline_trims_prestart_data_and_fills_gaps_with_silence() {
        let mut mixer = Mixer::new(Mode::System);
        mixer
            .push(Kind::System, -1, &[11, 12, 21, 22, 31, 32])
            .unwrap();
        mixer.push(Kind::System, 3, &[41, 42]).unwrap();
        let block = mixer.take(4).unwrap();
        assert_eq!(block.first_frame, 0);
        assert_eq!(block.samples, [21, 22, 31, 32, 0, 0, 41, 42]);
        mixer.push(Kind::System, 0, &[1, 2, 3, 4]).unwrap();
        assert_eq!(mixer.take(1).unwrap().samples, [0, 0]);
    }
    #[test]
    fn combined_inputs_mix_with_headroom_and_disabled_inputs_are_rejected() {
        let mut mixer = Mixer::new(Mode::Both);
        mixer
            .push(Kind::System, 0, &[30_000, -30_000, 10_000, 20_000])
            .unwrap();
        mixer.push(Kind::Microphone, 0, &[30_000, -30_000]).unwrap();
        assert_eq!(
            mixer.take(2).unwrap().samples,
            [30_000, -30_000, 5_000, 10_000]
        );
        let mut mic = Mixer::new(Mode::Microphone);
        assert!(mic.push(Kind::System, 0, &[1, 2]).is_err());
        mic.push(Kind::Microphone, 0, &[7, -8]).unwrap();
        assert_eq!(mic.take(1).unwrap().samples, [7, -8]);
    }
    #[test]
    fn audio_queue_is_bounded_and_bad_packets_cannot_mutate_buffered_audio() {
        let mut mixer = Mixer::new(Mode::System);
        mixer.push(Kind::System, 0, &[8, 9]).unwrap();
        assert!(
            mixer
                .push(Kind::System, CAPACITY_FRAMES as i64, &[1, 2])
                .is_err()
        );
        assert!(mixer.push(Kind::System, 0, &[1]).is_err());
        assert!(mixer.take(BLOCK_FRAMES + 1).is_err());
        assert_eq!(mixer.take(1).unwrap().samples, [8, 9]);
    }
    #[test]
    fn delivery_waits_for_capture_latency_then_flushes_exactly_to_video_end() {
        let mut mixer = Mixer::new(Mode::System);
        assert_eq!(mixer.due(100, false), 0);
        assert_eq!(mixer.due(110, false), 480);
        mixer.take(480).unwrap();
        assert_eq!(mixer.due(111, false), 0);
        assert_eq!(mixer.due(15, true), 240);
        let tail = mixer.take(240).unwrap();
        assert_eq!(tail.first_frame, 480);
        assert_eq!(mixer.due(15, true), 0);
        assert_eq!(mixer.due(360_000_000, false), 480);
        assert_eq!(Mixer::new(Mode::None).due(360_000_000, true), 0);
    }
    #[test]
    fn endpoint_and_media_clocks_keep_exact_sample_positions_over_long_durations() {
        assert_eq!(packet_frame(9_999_000, 10_000_000).unwrap(), -4);
        assert_eq!(packet_frame(20_000_000, 10_000_000).unwrap(), 48_000);
        assert_eq!(frame_time(48_000).unwrap(), 10_000_000);
        assert_eq!(frame_time(4_147_200_000).unwrap(), 864_000_000_000);
        assert!(frame_time(u64::MAX).is_err());
    }
}
