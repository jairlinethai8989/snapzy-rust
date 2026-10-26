#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture_results;
mod editor_registry;
#[allow(dead_code)]
#[path = "../../../../src/desktop_bridge.rs"]
mod protocol;

use capture_results::adopt;
use editor_registry::{EditorWorkspace, SessionRegistry};
use protocol::{Action, CaptureKind, EngineEvent, Request};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    os::windows::process::CommandExt,
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, LogicalSize, Manager, Size, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM},
    System::DataExchange::COPYDATASTRUCT,
    UI::WindowsAndMessaging::{
        AllowSetForegroundWindow, EnumWindows, GetClassNameW, GetWindowThreadProcessId,
        SMTO_ABORTIFHUNG, SMTO_BLOCK, SendMessageTimeoutW, WM_COPYDATA,
    },
};

struct Engine {
    child: Mutex<Option<Child>>,
    editors: Mutex<EditorState>,
    working: AtomicBool,
    closing: AtomicBool,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            child: Mutex::new(None),
            editors: Mutex::new(EditorState {
                sessions: SessionRegistry::new(16),
                workspace: EditorWorkspace::default(),
            }),
            working: AtomicBool::new(false),
            closing: AtomicBool::new(false),
        }
    }
}

struct EditorState {
    sessions: SessionRegistry<EditorSession>,
    workspace: EditorWorkspace,
}

struct EditorSession {
    process: EditorProcess,
    title: String,
    project_path: Option<PathBuf>,
}

#[derive(Clone, Serialize)]
struct EditorTab {
    session_id: String,
    title: String,
}

#[derive(Serialize)]
struct EditorBootstrap {
    tabs: Vec<EditorTab>,
    active: Option<String>,
}

struct EditorProcess {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    directory: PathBuf,
}

#[derive(Deserialize)]
struct EditorResponse {
    ok: bool,
    view: Option<Value>,
    error: Option<String>,
}

impl EditorProcess {
    fn start(app: &tauri::AppHandle) -> Result<Self, String> {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("SnapZy-Tauri-Editor-{}-{id}", std::process::id()));
        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let mut child = Command::new(engine_path(app)?)
            .arg("--editor-service")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|e| format!("Cannot start the editor service: {e}"))?;
        let input = child
            .stdin
            .take()
            .ok_or("Cannot open the editor input channel.")?;
        let output = BufReader::new(
            child
                .stdout
                .take()
                .ok_or("Cannot open the editor output channel.")?,
        );
        Ok(Self {
            child,
            input,
            output,
            directory,
        })
    }

    fn request(&mut self, request: Value) -> Result<Option<Value>, String> {
        serde_json::to_writer(&mut self.input, &request).map_err(|e| e.to_string())?;
        self.input.write_all(b"\n").map_err(|e| e.to_string())?;
        self.input.flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        if self
            .output
            .read_line(&mut line)
            .map_err(|e| e.to_string())?
            == 0
        {
            return Err("The editor service closed unexpectedly.".into());
        }
        let response: EditorResponse = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        if response.ok {
            Ok(response.view)
        } else {
            Err(response
                .error
                .unwrap_or_else(|| "The editor command failed.".into()))
        }
    }

    fn open(&mut self, bytes: &[u8]) -> Result<Value, String> {
        if bytes.is_empty() || bytes.len() > 256 * 1024 * 1024 {
            return Err("Choose a supported image smaller than 256 MiB.".into());
        }
        let path = self.directory.join("source-image");
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        let result = self
            .request(json!({ "type": "open", "path": path }))?
            .ok_or_else(|| "The editor did not return a document.".to_string());
        let _ = std::fs::remove_file(self.directory.join("source-image"));
        result
    }

    fn open_project(&mut self, path: &PathBuf) -> Result<Value, String> {
        self.request(json!({ "type": "open_project", "path": path }))?
            .ok_or_else(|| "The editor did not return a project.".to_string())
    }

    fn save_project(&mut self, path: &PathBuf) -> Result<Value, String> {
        self.request(json!({ "type": "save_project", "path": path }))?
            .ok_or_else(|| "The editor did not return a project.".to_string())
    }

    fn apply(&mut self, command: Value) -> Result<Value, String> {
        self.request(json!({ "type": "apply", "command": command }))?
            .ok_or_else(|| "The editor did not return a document.".into())
    }

    fn view(&mut self) -> Result<Value, String> {
        self.request(json!({ "type": "view" }))?
            .ok_or_else(|| "The editor did not return a document.".into())
    }

    fn preview(&mut self) -> Result<Vec<u8>, String> {
        let path = self.directory.join("preview.png");
        self.request(json!({ "type": "preview", "path": path }))?;
        std::fs::read(&path).map_err(|e| e.to_string())
    }

    fn shutdown(&mut self) {
        let _ = self.request(json!({ "type": "shutdown" }));
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(2) {
            if self.child.try_wait().ok().flatten().is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(25));
        }
        let _ = self.child.kill();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

impl Drop for EditorProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

struct WindowSearch {
    process_id: u32,
    window: usize,
}

unsafe extern "system" fn find_engine_window(window: HWND, context: LPARAM) -> i32 {
    let search = unsafe { &mut *(context as *mut WindowSearch) };
    let mut process_id = 0;
    unsafe {
        GetWindowThreadProcessId(window, &mut process_id);
    }
    if process_id == search.process_id {
        let mut name = [0u16; 128];
        let length = unsafe { GetClassNameW(window, name.as_mut_ptr(), name.len() as i32) };
        if String::from_utf16_lossy(&name[..length.max(0) as usize]) == "SnapZyRustPreview.Launcher"
        {
            search.window = window as usize;
            return 0;
        }
    }
    1
}

fn find_window(process_id: u32) -> Option<usize> {
    let mut search = WindowSearch {
        process_id,
        window: 0,
    };
    unsafe {
        EnumWindows(Some(find_engine_window), &mut search as *mut _ as LPARAM);
    }
    (search.window != 0).then_some(search.window)
}

fn engine_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let beside = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("Cannot locate the trial folder.")?
        .join("engine/snapzy-engine.exe");
    if beside.is_file() {
        return Ok(beside);
    }
    let bundled = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("engine/snapzy-engine.exe");
    if bundled.is_file() {
        return Ok(bundled);
    }
    if cfg!(debug_assertions) {
        let development =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/engine/snapzy-engine.exe");
        if development.is_file() {
            return Ok(development);
        }
    }
    Err("The native capture engine is missing. Keep the engine folder beside SnapZy.exe.".into())
}

fn editor_bootstrap(state: &EditorState, label: &str) -> EditorBootstrap {
    EditorBootstrap {
        tabs: state
            .workspace
            .sessions(label)
            .iter()
            .filter_map(|session_id| {
                state.sessions.get(session_id).map(|session| EditorTab {
                    session_id: session_id.clone(),
                    title: session.title.clone(),
                })
            })
            .collect(),
        active: state.workspace.active(label).map(str::to_string),
    }
}

fn editor_session_mut<'a>(
    state: &'a mut EditorState,
    session_id: &str,
) -> Result<&'a mut EditorSession, String> {
    let session = state
        .sessions
        .get_mut(session_id)
        .ok_or_else(|| "The editor session is no longer available.".to_string())?;
    if session
        .process
        .child
        .try_wait()
        .map_err(|e| e.to_string())?
        .is_some()
    {
        return Err("The editor service closed unexpectedly.".into());
    }
    Ok(session)
}

fn clean_title(name: &str) -> String {
    let value = name
        .chars()
        .filter(|character| !character.is_control())
        .take(120)
        .collect::<String>();
    if value.trim().is_empty() {
        "Untitled".into()
    } else {
        value
    }
}

#[tauri::command]
fn editor_window_mode(window: tauri::WebviewWindow) -> Result<(), String> {
    window.set_resizable(true).map_err(|e| e.to_string())?;
    window.set_maximizable(true).map_err(|e| e.to_string())?;
    window
        .set_min_size(Some(Size::Logical(LogicalSize::new(780., 560.))))
        .map_err(|e| e.to_string())?;
    window
        .set_size(Size::Logical(LogicalSize::new(1180., 680.)))
        .map_err(|e| e.to_string())?;
    window.center().map_err(|e| e.to_string())
}

#[tauri::command]
fn editor_new_window(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<Engine>>,
) -> Result<(), String> {
    let label = {
        let mut state = engine
            .editors
            .lock()
            .map_err(|_| "The editor service is unavailable.")?;
        state.workspace.create_host()
    };
    if let Err(error) = create_editor_window(&app, &label) {
        if let Ok(mut state) = engine.editors.lock() {
            state.workspace.remove_host(&label);
        }
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
fn editor_open(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    engine: tauri::State<'_, Arc<Engine>>,
    bytes: Vec<u8>,
    name: String,
) -> Result<Value, String> {
    let label = window.label().to_string();
    if !label.starts_with("editor-") {
        return Err("Open images from an editor window.".into());
    }
    let mut process = EditorProcess::start(&app)?;
    let view = process.open(&bytes)?;
    let mut state = engine
        .editors
        .lock()
        .map_err(|_| "The editor service is unavailable.")?;
    let session_id = state.sessions.insert(EditorSession {
        process,
        title: clean_title(&name),
        project_path: None,
    })?;
    if !state.workspace.add_to_host(&label, &session_id) {
        if let Some(mut session) = state.sessions.remove(&session_id) {
            session.process.shutdown();
        }
        return Err("The editor window is no longer available.".into());
    }
    Ok(json!({ "session_id": session_id, "view": view }))
}

fn project_dialog(window: &tauri::WebviewWindow, save: bool) -> Result<Option<PathBuf>, String> {
    use windows_sys::Win32::UI::Controls::Dialogs::{
        GetOpenFileNameW, GetSaveFileNameW, OPENFILENAMEW, OFN_EXPLORER, OFN_FILEMUSTEXIST,
        OFN_HIDEREADONLY, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST,
    };
    let filter: Vec<u16> = "SnapZy project (*.snapzy)\0*.snapzy\0All files (*.*)\0*.*\0\0"
        .encode_utf16()
        .collect();
    let title: Vec<u16> = if save {
        "Save SnapZy project\0".encode_utf16().collect()
    } else {
        "Open SnapZy project\0".encode_utf16().collect()
    };
    let default_extension: Vec<u16> = "snapzy\0".encode_utf16().collect();
    let mut file = vec![0u16; 32_768];
    let mut dialog = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: window.hwnd().map_err(|e| e.to_string())?.0,
        hInstance: std::ptr::null_mut(),
        lpstrFilter: filter.as_ptr(),
        lpstrCustomFilter: std::ptr::null_mut(),
        nMaxCustFilter: 0,
        nFilterIndex: 1,
        lpstrFile: file.as_mut_ptr(),
        nMaxFile: file.len() as u32,
        lpstrFileTitle: std::ptr::null_mut(),
        nMaxFileTitle: 0,
        lpstrInitialDir: std::ptr::null(),
        lpstrTitle: title.as_ptr(),
        Flags: OFN_EXPLORER | OFN_HIDEREADONLY | OFN_PATHMUSTEXIST
            | if save { OFN_OVERWRITEPROMPT } else { OFN_FILEMUSTEXIST },
        nFileOffset: 0,
        nFileExtension: 0,
        lpstrDefExt: default_extension.as_ptr(),
        lCustData: 0,
        lpfnHook: None,
        lpTemplateName: std::ptr::null(),
        pvReserved: std::ptr::null_mut(),
        dwReserved: 0,
        FlagsEx: 0,
    };
    let accepted = unsafe {
        if save { GetSaveFileNameW(&mut dialog) } else { GetOpenFileNameW(&mut dialog) }
    };
    if accepted == 0 {
        return Ok(None);
    }
    let length = file.iter().position(|value| *value == 0).unwrap_or(file.len());
    Ok(Some(PathBuf::from(String::from_utf16_lossy(&file[..length]))))
}

#[tauri::command]
fn editor_open_project(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    engine: tauri::State<'_, Arc<Engine>>,
) -> Result<Value, String> {
    let Some(path) = project_dialog(&window, false)? else {
        return Err("Project open cancelled.".into());
    };
    let mut process = EditorProcess::start(&app)?;
    let view = process.open_project(&path)?;
    let title = clean_title(path.file_name().and_then(|name| name.to_str()).unwrap_or("Project"));
    let mut state = engine
        .editors
        .lock()
        .map_err(|_| "The editor service is unavailable.")?;
    let session_id = state.sessions.insert(EditorSession {
        process,
        title,
        project_path: Some(path),
    })?;
    if !state.workspace.add_to_host(window.label(), &session_id) {
        if let Some(mut session) = state.sessions.remove(&session_id) {
            session.process.shutdown();
        }
        return Err("The editor window is no longer available.".into());
    }
    Ok(json!({ "session_id": session_id, "view": view }))
}

#[tauri::command]
fn editor_save_project(
    window: tauri::WebviewWindow,
    engine: tauri::State<'_, Arc<Engine>>,
    session_id: String,
    save_as: bool,
) -> Result<Value, String> {
    let existing = {
        let state = engine
            .editors
            .lock()
            .map_err(|_| "The editor service is unavailable.")?;
        if state.workspace.host_for(&session_id) != Some(window.label()) {
            return Err("The editor session does not belong to this window.".into());
        }
        state.sessions.get(&session_id).and_then(|session| session.project_path.clone())
    };
    let path = if !save_as {
        existing.unwrap_or(project_dialog(&window, true)?.ok_or("Project save cancelled.")?)
    } else {
        project_dialog(&window, true)?.ok_or("Project save cancelled.")?
    };
    let mut state = engine
        .editors
        .lock()
        .map_err(|_| "The editor service is unavailable.")?;
    if state.workspace.host_for(&session_id) != Some(window.label()) {
        return Err("The editor session does not belong to this window.".into());
    }
    let session = editor_session_mut(&mut state, &session_id)?;
    let view = session.process.save_project(&path)?;
    session.project_path = Some(path.clone());
    session.title = clean_title(path.file_name().and_then(|name| name.to_str()).unwrap_or("Project"));
    Ok(json!({ "view": view, "path": path }))
}

#[tauri::command]
fn editor_tabs(
    window: tauri::WebviewWindow,
    engine: tauri::State<'_, Arc<Engine>>,
) -> Result<EditorBootstrap, String> {
    let state = engine
        .editors
        .lock()
        .map_err(|_| "The editor service is unavailable.")?;
    Ok(editor_bootstrap(&state, window.label()))
}

#[tauri::command]
fn editor_activate(
    window: tauri::WebviewWindow,
    engine: tauri::State<'_, Arc<Engine>>,
    session_id: String,
) -> Result<Value, String> {
    let mut state = engine
        .editors
        .lock()
        .map_err(|_| "The editor service is unavailable.")?;
    if !state.workspace.activate(window.label(), &session_id) {
        return Err("The editor session does not belong to this window.".into());
    }
    editor_session_mut(&mut state, &session_id)?.process.view()
}

#[tauri::command]
fn editor_apply(
    window: tauri::WebviewWindow,
    engine: tauri::State<'_, Arc<Engine>>,
    session_id: String,
    command: Value,
) -> Result<Value, String> {
    let mut state = engine
        .editors
        .lock()
        .map_err(|_| "The editor service is unavailable.")?;
    if state.workspace.host_for(&session_id) != Some(window.label()) {
        return Err("The editor session does not belong to this window.".into());
    }
    editor_session_mut(&mut state, &session_id)?
        .process
        .apply(command)
}

#[tauri::command]
fn editor_preview(
    window: tauri::WebviewWindow,
    engine: tauri::State<'_, Arc<Engine>>,
    session_id: String,
) -> Result<tauri::ipc::Response, String> {
    let mut state = engine
        .editors
        .lock()
        .map_err(|_| "The editor service is unavailable.")?;
    if state.workspace.host_for(&session_id) != Some(window.label()) {
        return Err("The editor session does not belong to this window.".into());
    }
    let bytes = editor_session_mut(&mut state, &session_id)?
        .process
        .preview()?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
fn editor_close(
    window: tauri::WebviewWindow,
    engine: tauri::State<'_, Arc<Engine>>,
    session_id: String,
) -> Result<EditorBootstrap, String> {
    let mut state = engine
        .editors
        .lock()
        .map_err(|_| "The editor service is unavailable.")?;
    if state.workspace.host_for(&session_id) != Some(window.label()) {
        return Err("The editor session does not belong to this window.".into());
    }
    let owner = state.workspace.remove(&session_id);
    debug_assert_eq!(owner.as_deref(), Some(window.label()));
    if let Some(mut session) = state.sessions.remove(&session_id) {
        session.process.shutdown();
    }
    Ok(editor_bootstrap(&state, window.label()))
}

#[tauri::command]
fn editor_close_window(
    window: tauri::WebviewWindow,
    engine: tauri::State<'_, Arc<Engine>>,
    application: bool,
) -> Result<(), String> {
    let sessions = {
        let mut state = engine
            .editors
            .lock()
            .map_err(|_| "The editor service is unavailable.")?;
        let ids = state.workspace.remove_host(window.label());
        ids.into_iter()
            .filter_map(|id| state.sessions.remove(&id))
            .collect::<Vec<_>>()
    };
    for mut session in sessions {
        session.process.shutdown();
    }
    if application {
        let no_sessions = engine
            .editors
            .lock()
            .map(|state| state.sessions.len() == 0)
            .unwrap_or(false);
        if no_sessions {
            if let Some(main) = window.app_handle().get_webview_window("main") {
                let _ = main.close();
            }
        }
    }
    Ok(())
}

fn capture_title(kind: CaptureKind) -> &'static str {
    match kind {
        CaptureKind::Area => "Area capture",
        CaptureKind::Window => "Window capture",
        CaptureKind::Desktop => "Desktop capture",
        CaptureKind::Scrolling => "Scrolling capture",
    }
}

fn create_editor_window(app: &tauri::AppHandle, label: &str) -> Result<(), String> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("editor.html".into()))
        .title(format!(
            "SnapZy {} | Image editor",
            env!("CARGO_PKG_VERSION")
        ))
        .inner_size(1180., 680.)
        .min_inner_size(780., 560.)
        .decorations(true)
        .resizable(true)
        .maximizable(true)
        .minimizable(true)
        .closable(true)
        .center()
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn remove_editor_session(engine: &Engine, session_id: &str) {
    if let Ok(mut state) = engine.editors.lock() {
        state.workspace.remove(session_id);
        if let Some(mut session) = state.sessions.remove(session_id) {
            session.process.shutdown();
        }
    }
}

fn open_capture_result(
    app: &tauri::AppHandle,
    engine: &Arc<Engine>,
    directory: &std::path::Path,
    event: &EngineEvent,
) -> Result<(), String> {
    let capture = adopt(directory, event)?;
    let mut process = EditorProcess::start(app)?;
    process.open(&capture.bytes)?;
    let title = capture_title(capture.kind).to_string();
    let (session_id, assignment) = {
        let mut state = engine
            .editors
            .lock()
            .map_err(|_| "The editor service is unavailable.")?;
        let session_id = match state.sessions.insert(EditorSession { process, title, project_path: None }) {
            Ok(session_id) => session_id,
            Err(error) => return Err(error),
        };
        let assignment = match state.workspace.assign(&session_id, capture.destination) {
            Ok(assignment) => assignment,
            Err(error) => {
                if let Some(mut session) = state.sessions.remove(&session_id) {
                    session.process.shutdown();
                }
                return Err(error);
            }
        };
        (session_id, assignment)
    };
    if assignment.created
        && let Err(error) = create_editor_window(app, &assignment.label)
    {
        remove_editor_session(engine, &session_id);
        return Err(error);
    }
    app.emit_to(&assignment.label, "editor-sessions-changed", ())
        .map_err(|e| e.to_string())?;
    if let Some(window) = app.get_webview_window(&assignment.label) {
        let _ = window.show();
        let _ = window.set_focus();
    }
    Ok(())
}

fn report_capture_error(app: &tauri::AppHandle, error: String) {
    let _ = app.emit_to("main", "capture-result-error", error);
}

fn read_capture_events(
    app: tauri::AppHandle,
    engine: Arc<Engine>,
    directory: PathBuf,
    output: ChildStdout,
) {
    let mut reader = BufReader::new(output);
    loop {
        let mut line = Vec::new();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) => break,
            Ok(_) => {
                if line.last() == Some(&b'\n') {
                    line.pop();
                    if line.last() == Some(&b'\r') {
                        line.pop();
                    }
                }
                match EngineEvent::decode(&line)
                    .and_then(|event| open_capture_result(&app, &engine, &directory, &event))
                {
                    Ok(()) => {}
                    Err(error) => report_capture_error(&app, error),
                }
            }
            Err(error) => {
                report_capture_error(&app, error.to_string());
                break;
            }
        }
    }
    let _ = std::fs::remove_dir_all(directory);
}

impl Engine {
    fn window(self: &Arc<Self>, app: &tauri::AppHandle) -> Result<usize, String> {
        let executable = engine_path(app)?;
        let mut guard = self
            .child
            .lock()
            .map_err(|_| "The capture engine is unavailable.")?;
        if let Some(child) = guard.as_mut() {
            if child.try_wait().map_err(|e| e.to_string())?.is_none() {
                return find_window(child.id())
                    .ok_or_else(|| "The capture engine is still starting.".into());
            }
            *guard = None;
        }
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let result_directory =
            std::env::temp_dir().join(format!("SnapZy-Tauri-Captures-{}-{id}", std::process::id()));
        std::fs::create_dir_all(&result_directory).map_err(|e| e.to_string())?;
        let mut child = match Command::new(executable)
            .arg("--tauri-engine")
            .arg("--tauri-result-dir")
            .arg(&result_directory)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                let _ = std::fs::remove_dir_all(&result_directory);
                return Err(format!("Cannot start the capture engine: {error}"));
            }
        };
        let Some(output) = child.stdout.take() else {
            let _ = child.kill();
            let _ = std::fs::remove_dir_all(&result_directory);
            return Err("Cannot open the capture result channel.".into());
        };
        let process_id = child.id();
        *guard = Some(child);
        thread::spawn({
            let app = app.clone();
            let engine = Arc::clone(self);
            move || read_capture_events(app, engine, result_directory, output)
        });
        let started = Instant::now();
        loop {
            if let Some(window) = find_window(process_id) {
                return Ok(window);
            }
            if guard
                .as_mut()
                .unwrap()
                .try_wait()
                .map_err(|e| e.to_string())?
                .is_some()
            {
                *guard = None;
                return Err("The capture engine closed before it was ready. Close another Tauri trial and try again.".into());
            }
            if started.elapsed() > Duration::from_secs(10) {
                return Err("The capture engine did not become ready in time.".into());
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    fn exited(&self) -> Result<bool, String> {
        let mut guard = self
            .child
            .lock()
            .map_err(|_| "The capture engine is unavailable.")?;
        let Some(child) = guard.as_mut() else {
            return Ok(true);
        };
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            *guard = None;
            return Ok(true);
        }
        Ok(false)
    }
}

fn send(window: usize, request: &Request) -> Result<isize, String> {
    let bytes = serde_json::to_vec(request).map_err(|e| e.to_string())?;
    Request::decode(&bytes)?;
    let data = COPYDATASTRUCT {
        dwData: protocol::COPYDATA_ID,
        cbData: bytes.len() as u32,
        lpData: bytes.as_ptr().cast_mut().cast(),
    };
    let mut reply = 0;
    let success = unsafe {
        SendMessageTimeoutW(
            window as HWND,
            WM_COPYDATA,
            protocol::owner(),
            &data as *const _ as LPARAM,
            SMTO_ABORTIFHUNG | SMTO_BLOCK,
            2000,
            &mut reply,
        )
    };
    if success == 0 {
        return Err("The capture engine is not responding. Your open work has been kept.".into());
    }
    Ok(reply as isize)
}

#[tauri::command]
async fn desktop_action(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Arc<Engine>>,
    request: Request,
) -> Result<(), String> {
    if matches!(request.action, Action::Status | Action::Shutdown) {
        return Err("Use a capture or menu command.".into());
    }
    let window = app
        .get_webview_window("main")
        .ok_or("The launcher is unavailable.")?;
    protocol::set_owner(window.hwnd().map_err(|e| e.to_string())?.0 as usize);
    let engine = Arc::clone(engine.inner());
    if engine.working.swap(true, Ordering::AcqRel) {
        return Err("Finish the current operation first.".into());
    }
    let result = tauri::async_runtime::spawn_blocking({
        let engine = Arc::clone(&engine);
        let window = window.clone();
        move || -> Result<(), String> {
            let native = engine.window(&app)?;
            if send(
                native,
                &Request {
                    action: Action::Status,
                    preferences: None,
                },
            )? != protocol::READY
            {
                return Err("Finish the current capture or close its dialog first.".into());
            }
            {
                let guard = engine
                    .child
                    .lock()
                    .map_err(|_| "The capture engine is unavailable.")?;
                if let Some(child) = guard.as_ref() {
                    unsafe {
                        AllowSetForegroundWindow(child.id());
                    }
                }
            }
            if matches!(
                request.action,
                Action::Area
                    | Action::Window
                    | Action::Desktop
                    | Action::Scrolling
                    | Action::VideoMonitor
                    | Action::VideoWindow
            ) {
                window.hide().map_err(|e| e.to_string())?;
                thread::sleep(Duration::from_millis(180));
            }
            if send(native, &request)? != protocol::READY {
                return Err("The capture engine could not accept the command.".into());
            }
            thread::sleep(Duration::from_millis(300));
            loop {
                if engine.exited()? {
                    return Err("The capture engine closed.".into());
                }
                match send(
                    native,
                    &Request {
                        action: Action::Status,
                        preferences: None,
                    },
                )? {
                    protocol::READY => return Ok(()),
                    protocol::BUSY => thread::sleep(Duration::from_millis(200)),
                    _ => return Err("The capture engine returned an invalid status.".into()),
                }
            }
        }
    })
    .await
    .map_err(|e| e.to_string())
    .and_then(|r| r);
    engine.working.store(false, Ordering::Release);
    let _ = window.show();
    result
}

fn shutdown(engine: &Engine) -> Result<bool, String> {
    if let Ok(mut state) = engine.editors.lock() {
        for mut session in state.sessions.take_all() {
            session.process.shutdown();
        }
        state.workspace = EditorWorkspace::default();
    }
    let native = {
        let guard = engine
            .child
            .lock()
            .map_err(|_| "The capture engine is unavailable.")?;
        let Some(child) = guard.as_ref() else {
            return Ok(true);
        };
        find_window(child.id())
    };
    if let Some(native) = native {
        send(
            native,
            &Request {
                action: Action::Shutdown,
                preferences: None,
            },
        )?;
        thread::sleep(Duration::from_millis(350));
        let started = Instant::now();
        loop {
            if engine.exited()? {
                return Ok(true);
            }
            if started.elapsed() > Duration::from_secs(60) {
                return Err("Finish closing the native editor and try again.".into());
            }
            match send(
                native,
                &Request {
                    action: Action::Status,
                    preferences: None,
                },
            ) {
                Ok(protocol::READY) => return Ok(false),
                _ => thread::sleep(Duration::from_millis(200)),
            }
        }
    }
    engine.exited()
}

fn main() {
    tauri::Builder::default()
        .manage(Arc::new(Engine::default()))
        .invoke_handler(tauri::generate_handler![
            desktop_action,
            editor_window_mode,
            editor_new_window,
            editor_open,
            editor_tabs,
            editor_activate,
            editor_apply,
            editor_preview,
            editor_close,
            editor_open_project,
            editor_save_project,
            editor_close_window
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let engine = Arc::clone(window.state::<Arc<Engine>>().inner());
                if window.label() != "main" {
                    let has_sessions = engine
                        .editors
                        .lock()
                        .map(|state| !state.workspace.sessions(window.label()).is_empty())
                        .unwrap_or(false);
                    if has_sessions {
                        api.prevent_close();
                        let _ = window.emit("editor-window-close-requested", ());
                    }
                    return;
                }
                let editor_hosts = engine
                    .editors
                    .lock()
                    .map(|state| state.workspace.hosts())
                    .unwrap_or_default();
                if !editor_hosts.is_empty() {
                    api.prevent_close();
                    for label in editor_hosts {
                        let _ = window.emit_to(&label, "application-close-requested", ());
                    }
                    return;
                }
                api.prevent_close();
                if engine.closing.swap(true, Ordering::AcqRel) {
                    return;
                }
                let window = window.clone();
                tauri::async_runtime::spawn(async move {
                    let copy = Arc::clone(&engine);
                    let result =
                        tauri::async_runtime::spawn_blocking(move || shutdown(&copy)).await;
                    if matches!(result, Ok(Ok(true))) {
                        window.app_handle().exit(0);
                    } else {
                        engine.closing.store(false, Ordering::Release);
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("Cannot start SnapZy");
}
