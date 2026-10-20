use super::*;

pub(super) struct OpenOrigin {
    route: crate::workspace::Route,
    owner: usize,
    identity: Option<u64>,
    previous_status: String,
    opening_status: String,
}
impl OpenOrigin {
    fn begin(owner: HWND, identity: Option<u64>, opening_status: String) -> Self {
        let previous_status = status_text(owner);
        status(owner, &opening_status);
        Self {
            route: crate::workspace::Route {
                target: None,
                origin: None,
            },
            owner: owner as usize,
            identity,
            previous_status,
            opening_status,
        }
    }
    fn finish(&self, main: usize, identity: Option<u64>) -> usize {
        let live = match self.identity {
            Some(expected) => identity == Some(expected),
            None => self.owner == main,
        };
        if !live {
            return main;
        }
        // Do not replace status from an operation started while the file loaded.
        if status_text(self.owner as HWND) == self.opening_status {
            status(self.owner as HWND, &self.previous_status);
        }
        self.owner
    }
}
fn status_text(owner: HWND) -> String {
    unsafe {
        let control = GetDlgItem(owner, STATUS);
        let mut buffer = vec![0u16; GetWindowTextLengthW(control).max(0) as usize + 1];
        let length = GetWindowTextW(control, buffer.as_mut_ptr(), buffer.len() as i32);
        String::from_utf16_lossy(&buffer[..length.max(0) as usize])
    }
}

fn is_project(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("snapzy"))
}
fn is_neosnap(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("neosnap"))
}
#[derive(Debug)]
pub(super) enum OpenError {
    Other(String),
    Neosnap(crate::neosnap::Failure),
}
impl From<String> for OpenError {
    fn from(value: String) -> Self {
        Self::Other(value)
    }
}
impl From<&str> for OpenError {
    fn from(value: &str) -> Self {
        Self::Other(value.into())
    }
}
fn load_input(path: &std::path::Path, budget: usize) -> Result<crate::project::Project, OpenError> {
    if is_project(path) {
        return crate::project::load(path, budget).map_err(Into::into);
    }
    if is_neosnap(path) {
        return crate::neosnap::load(path, budget).map_err(OpenError::Neosnap);
    }
    use std::io::Read;
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > crate::image_io::INPUT_FILE_LIMIT as u64
    {
        return Err("Image file exceeds the 256 MiB input limit.".into());
    }
    let mut bytes = vec![];
    file.take(crate::image_io::INPUT_FILE_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let frame = Arc::new(crate::image_io::decode(&bytes, budget)?);
    let document = crate::editor::Document::new(frame.width, frame.height);
    Ok(crate::project::Project { frame, document })
}

pub(super) struct ExitPlan {
    pub host: Option<crate::workspace::Key>,
    pub windows: Vec<(usize, u64, u64)>,
    pub index: usize,
    pub decisions: Vec<(usize, u64, u64)>,
    pub waiting: Option<(usize, u64, u64)>,
}
fn current_completion(
    actual_identity: u64,
    actual_operation: u64,
    identity: u64,
    operation: u64,
) -> bool {
    actual_identity == identity && actual_operation == operation
}

pub(super) fn text(en: &str, th: &str) -> String {
    if STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th") {
        th.into()
    } else {
        en.into()
    }
}
pub(super) struct Modal;
impl Modal {
    pub(super) fn enter() -> Option<Self> {
        STATE.with(|s| {
            let mut b = s.borrow_mut();
            let a = b.as_mut()?;
            if a.project_modal {
                None
            } else {
                a.project_modal = true;
                Some(Self)
            }
        })
    }
}
impl Drop for Modal {
    fn drop(&mut self) {
        STATE.with(|s| {
            if let Some(a) = s.borrow_mut().as_mut() {
                a.project_modal = false;
            }
        });
    }
}

fn chooser(
    owner: HWND,
    save: bool,
    initial: Option<&std::path::Path>,
) -> Result<Option<PathBuf>, String> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    let mut filename = vec![0u16; 32768];
    let initial: Vec<u16> = initial
        .map(|p| p.as_os_str().encode_wide().collect())
        .unwrap_or_else(|| "SnapZy-work.snapzy".encode_utf16().collect());
    if initial.len() >= filename.len() {
        return Err("Project path is too long".into());
    }
    filename[..initial.len()].copy_from_slice(&initial);
    let filter = wide(if save {
        "SnapZy project (*.snapzy)\0*.snapzy\0All files (*.*)\0*.*\0\0"
    } else {
        "Images and editable projects\0*.snapzy;*.neosnap;*.png;*.jpg;*.jpeg;*.bmp\0SnapZy project (*.snapzy)\0*.snapzy\0.NET project (*.neosnap)\0*.neosnap\0All files (*.*)\0*.*\0\0"
    });
    let extension = wide("snapzy");
    let title = wide(&if save {
        text("Save editable project", "บันทึกไฟล์งาน")
    } else {
        text("Open image or editable project", "เปิดภาพหรือไฟล์งาน")
    });
    let mut d: OPENFILENAMEW = unsafe { zeroed() };
    d.lStructSize = size_of::<OPENFILENAMEW>() as u32;
    d.hwndOwner = workspace_ui::native_owner(owner);
    d.lpstrFile = filename.as_mut_ptr();
    d.nMaxFile = filename.len() as u32;
    d.lpstrFilter = filter.as_ptr();
    d.lpstrDefExt = extension.as_ptr();
    d.lpstrTitle = title.as_ptr();
    d.nFilterIndex = 1;
    d.Flags = OFN_NOCHANGEDIR
        | OFN_PATHMUSTEXIST
        | OFN_EXPLORER
        | if save {
            OFN_OVERWRITEPROMPT
        } else {
            OFN_FILEMUSTEXIST
        };
    let ok = unsafe {
        if save {
            GetSaveFileNameW(&mut d)
        } else {
            GetOpenFileNameW(&mut d)
        }
    };
    if ok == 0 {
        let err = unsafe { CommDlgExtendedError() };
        return if err == 0 {
            Ok(None)
        } else {
            Err(format!("File dialog failed ({err})"))
        };
    }
    let n = filename
        .iter()
        .position(|c| *c == 0)
        .unwrap_or(filename.len());
    let path = PathBuf::from(std::ffi::OsString::from_wide(&filename[..n]));
    if !is_project(&path)
        && (save
            || (!is_neosnap(&path)
                && path.extension().is_none_or(|e| {
                    !["png", "jpg", "jpeg", "bmp"]
                        .iter()
                        .any(|ext| e.eq_ignore_ascii_case(ext))
                })))
    {
        return Err(text(
            "Save editable work as .snapzy. Open supports .snapzy, compatible .neosnap, PNG, JPEG and BMP.",
            "บันทึกงานเป็น .snapzy เปิดได้ทั้ง .snapzy, .neosnap ที่รองรับ และภาพ PNG, JPEG, BMP",
        ));
    }
    Ok(Some(path))
}

pub(super) fn open(owner: HWND) {
    workspace_ui::commit_numeric();
    let route = workspace_ui::route(owner);
    let parent = workspace_ui::native_owner(owner);
    if class_name(parent) == workspace_ui::HOST && !workspace_ui::can_switch(parent) {
        return;
    }
    if STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        a.project_opening
            || a.project_exit.is_some()
            || a.capture_busy
            || a.selection.is_some()
            || a.scrolling.is_some()
            || a.previews.len() >= 16
    }) {
        return;
    }
    let Some(_modal) = Modal::enter() else { return };
    let path = match chooser(owner, false, None) {
        Ok(Some(p)) => p,
        Ok(None) => return,
        Err(e) => {
            notify(owner, &e, true);
            return;
        }
    };
    let job = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        let budget = crate::image_io::SOURCE_LIMIT.saturating_sub(source_bytes(a));
        if a.previews.len() >= 16 || budget == 0 {
            return None;
        }
        a.project_opening = true;
        a.project_reserved = budget;
        let identity = a
            .previews
            .get(&(owner as usize))
            .map(|p| p.guard.token.identity);
        Some((a.tx.clone(), a.main, budget, identity))
    });
    let Some((tx, main, budget, identity)) = job else {
        notify(
            owner,
            &text(
                "Close an older editor before opening this project.",
                "กรุณาปิดงานเก่าก่อนเปิดไฟล์งานนี้",
            ),
            true,
        );
        return;
    };
    let mut origin = OpenOrigin::begin(
        owner,
        identity,
        text("Opening project...", "กำลังเปิดไฟล์งาน..."),
    );
    origin.route = route;
    std::thread::spawn(move || {
        let started = Instant::now();
        let result = load_input(&path, budget);
        let _ = tx.send(Work::ProjectOpened {
            origin,
            path,
            started,
            result,
        });
        unsafe {
            PostMessageW(main as HWND, COMPLETE, 0, 0);
        }
    });
}

pub(super) fn save_menu(owner: HWND) {
    let menu = unsafe { CreatePopupMenu() };
    if menu.is_null() {
        return;
    }
    unsafe {
        AppendMenuW(
            menu,
            MF_STRING,
            1,
            wide(&text("Save project", "บันทึกงาน")).as_ptr(),
        );
        AppendMenuW(
            menu,
            MF_STRING,
            2,
            wide(&text("Save project as...", "บันทึกงานเป็น...")).as_ptr(),
        );
        let mut r: RECT = zeroed();
        GetWindowRect(GetDlgItem(owner, ec::PROJECT_SAVE as i32), &mut r);
        let chosen = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY,
            r.left,
            r.bottom,
            0,
            owner,
            null(),
        );
        DestroyMenu(menu);
        if chosen != 0 {
            save(owner, chosen == 2, false);
        }
    }
}

pub(super) fn save(owner: HWND, save_as: bool, close_after: bool) -> bool {
    workspace_ui::commit_numeric();
    let context = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref()?;
        let p = a.previews.get(&(owner as usize))?;
        if a.saving > 0
            || p.saving
            || p.importing
            || a.text_entry.is_some()
            || a.effects_open
            || workspace_ui::unfinished(&p.session)
        {
            return None;
        }
        Some((p.guard.token.identity, p.project_path.clone()))
    });
    let Some((identity, associated)) = context else {
        status(
            owner,
            &text(
                "Finish the current operation before saving the project.",
                "กรุณารอให้งานปัจจุบันเสร็จก่อนบันทึกไฟล์งาน",
            ),
        );
        return false;
    };
    let Some(_modal) = Modal::enter() else {
        return false;
    };
    if save_as || associated.is_none() {
        let warning = text(
            "Editable projects retain original images and information beneath blur or covering shapes. Share exported PNG instead. Continue saving this project?",
            "ไฟล์งานเก็บภาพต้นฉบับ รวมข้อมูลใต้เบลอและรูปทรงที่ปิดทับ หากต้องการแชร์ภาพให้ส่งออก PNG แทน ต้องการบันทึกไฟล์งานต่อหรือไม่?",
        );
        if unsafe {
            MessageBoxW(
                workspace_ui::native_owner(owner),
                wide(&warning).as_ptr(),
                wide(DIALOG_TITLE).as_ptr(),
                MB_OKCANCEL | MB_ICONWARNING | MB_DEFBUTTON2,
            )
        } != IDOK
        {
            return false;
        }
    }
    let path = if save_as || associated.is_none() {
        match chooser(owner, true, associated.as_deref()) {
            Ok(Some(p)) => p,
            Ok(None) => return false,
            Err(e) => {
                notify(owner, &e, true);
                return false;
            }
        }
    } else {
        associated.unwrap()
    };
    let job = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut()?;
        let p = a.previews.get_mut(&(owner as usize))?;
        if p.guard.token.identity != identity || p.saving {
            return None;
        }
        let operation = p.project_operation.checked_add(1)?;
        p.session.cancel();
        p.saving = true;
        p.close_after_save = close_after;
        p.project_operation = operation;
        a.saving += 1;
        Some((
            p.frame.clone(),
            p.session.document.render_snapshot(),
            p.project_operation,
            a.tx.clone(),
            a.main,
        ))
    });
    let Some((frame, document, operation, tx, main)) = job else {
        return false;
    };
    status(owner, &text("Saving project...", "กำลังบันทึกไฟล์งาน..."));
    update_editor(owner);
    let owner = owner as usize;
    std::thread::spawn(move || {
        let started = Instant::now();
        let revision = document.revision;
        let result = crate::project::save(&path, &frame, &document);
        let _ = tx.send(Work::ProjectSaved {
            owner,
            identity,
            operation,
            revision,
            path,
            started,
            result,
        });
        unsafe {
            PostMessageW(main as HWND, COMPLETE, 0, 0);
        }
    });
    true
}

pub(super) fn opened(
    origin: OpenOrigin,
    path: PathBuf,
    started: Instant,
    result: Result<crate::project::Project, OpenError>,
) {
    let (main, identity) = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        a.project_opening = false;
        a.project_reserved = 0;
        (
            a.main,
            a.previews
                .get(&origin.owner)
                .map(|p| p.guard.token.identity),
        )
    });
    let owner = origin.finish(main, identity);
    if STATE.with(|s| {
        s.borrow()
            .as_ref()
            .unwrap()
            .workspace
            .resolve(origin.route)
            .is_err()
    }) {
        return;
    }
    match result {
        Ok(p) => {
            trace::record("project.open", started);
            if is_neosnap(&path) {
                let label = path.file_name().map(|s| s.to_string_lossy().into_owned());
                present_project_cached(
                    p.frame,
                    Some(p.document),
                    None,
                    started,
                    None,
                    origin.route,
                    label,
                );
            } else {
                let associated = is_project(&path).then_some(path);
                present_project(p.frame, Some(p.document), associated, started, origin.route);
            }
        }
        Err(OpenError::Neosnap(e)) => {
            let thai = STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th");
            notify(owner as HWND, &e.message(thai), true);
        }
        Err(OpenError::Other(e)) => notify(
            owner as HWND,
            &text(
                &format!("Cannot open project: {e}"),
                &format!("เปิดไฟล์งานไม่สำเร็จ: {e}"),
            ),
            true,
        ),
    }
}

pub(super) fn saved(
    owner: usize,
    identity: u64,
    operation: u64,
    revision: u64,
    path: PathBuf,
    started: Instant,
    result: Result<(), String>,
) {
    let accepted = STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        a.saving = a.saving.saturating_sub(1);
        let p = a.previews.get_mut(&owner)?;
        if !current_completion(
            p.guard.token.identity,
            p.project_operation,
            identity,
            operation,
        ) {
            return None;
        }
        p.saving = false;
        let close = p.close_after_save;
        p.close_after_save = false;
        if result.is_ok() {
            p.project_path = Some(path);
            p.session.document.mark_project_saved(revision);
        }
        let exiting = a
            .project_exit
            .as_ref()
            .is_some_and(|plan| plan.waiting == Some((owner, identity, operation)));
        if exiting {
            a.project_exit.as_mut().unwrap().waiting = None;
        }
        Some((close, p.session.document.is_project_saved(), exiting))
    });
    let Some((close, clean, exiting)) = accepted else {
        return;
    };
    update_editor(owner as HWND);
    match result {
        Ok(()) => {
            trace::record("project.save", started);
            status(
                owner as HWND,
                &text(
                    if clean {
                        "Project saved"
                    } else {
                        "Saved snapshot; newer edits are not saved"
                    },
                    if clean {
                        "บันทึกงานแล้ว"
                    } else {
                        "บันทึกภาพงานก่อนหน้าแล้ว แต่ยังมีการแก้ไขใหม่ที่ไม่บันทึก"
                    },
                ),
            );
            if exiting {
                if clean {
                    advance_exit();
                } else {
                    abort_exit();
                }
            } else if close && clean {
                close_window(owner as HWND);
            }
        }
        Err(e) => {
            abort_exit();
            status(
                owner as HWND,
                &text(
                    &format!("Cannot save project: {e}"),
                    &format!("บันทึกไฟล์งานไม่สำเร็จ: {e}"),
                ),
            );
        }
    }
}

fn decision(owner: HWND) -> i32 {
    let previous = workspace_ui::selection(owner);
    workspace_ui::reveal(owner);
    let chosen = decision_revealed(owner);
    workspace_ui::restore_selection(previous);
    chosen
}
fn compact_dialog_name(name: &str) -> String {
    let name = name.trim();
    let mut chars = name.chars();
    let compact: String = chars.by_ref().take(42).collect();
    if chars.next().is_some() {
        format!("{compact}…")
    } else {
        compact
    }
}

fn project_close_content(project_name: &str, thai: bool) -> String {
    let project_name = if thai && project_name == "Untitled project" {
        "งานใหม่"
    } else {
        project_name
    };
    let project_name = compact_dialog_name(project_name);
    if thai {
        format!("งาน: {project_name}\nคัดลอกหรือส่งออก PNG ไม่ได้บันทึกงานที่แก้ไขต่อได้")
    } else {
        format!("Project: {project_name}\nCopying or exporting PNG does not save editable work.")
    }
}

fn decision_revealed(owner: HWND) -> i32 {
    use windows_sys::Win32::UI::Controls::{
        TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TDF_ALLOW_DIALOG_CANCELLATION, TaskDialogIndirect,
    };
    let Some(_modal) = Modal::enter() else {
        return IDCANCEL;
    };
    let instruction = wide(&text(
        "Save this editable project before closing?",
        "บันทึกไฟล์งานนี้ก่อนปิดหรือไม่?",
    ));
    let thai = STATE.with(|s| s.borrow().as_ref().unwrap().settings.language == "th");
    let labels = [
        wide(&text("Save project", "บันทึกงาน")),
        wide(&text("Discard", "ทิ้งงาน")),
        wide(&text("Cancel", "ยกเลิก")),
    ];
    let buttons = [IDYES, IDNO, IDCANCEL].map(|id| TASKDIALOG_BUTTON {
        nButtonID: id,
        pszButtonText: labels[if id == IDYES {
            0
        } else if id == IDNO {
            1
        } else {
            2
        }]
        .as_ptr(),
    });
    let name = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        a.previews
            .get(&(owner as usize))
            .and_then(|p| p.project_path.as_ref())
            .and_then(|p| p.file_name())
            .map(|p| p.to_string_lossy().into_owned())
            .or_else(|| {
                a.previews
                    .get(&(owner as usize))
                    .and_then(|p| p.source_label.clone())
            })
            .unwrap_or_else(|| "Untitled project".into())
    });
    let title = wide(DIALOG_TITLE);
    let content = wide(&project_close_content(&name, thai));
    let config = TASKDIALOGCONFIG {
        cbSize: size_of::<TASKDIALOGCONFIG>() as u32,
        hwndParent: workspace_ui::native_owner(owner),
        dwFlags: TDF_ALLOW_DIALOG_CANCELLATION,
        pszWindowTitle: title.as_ptr(),
        pszMainInstruction: instruction.as_ptr(),
        pszContent: content.as_ptr(),
        cButtons: 3,
        pButtons: buttons.as_ptr(),
        nDefaultButton: IDCANCEL,
        cxWidth: TASK_DIALOG_WIDTH,
        ..Default::default()
    };
    let mut chosen = IDCANCEL;
    if unsafe { TaskDialogIndirect(&config, &mut chosen, null_mut(), null_mut()) } >= 0 {
        return chosen;
    }
    // Keep a safe close prompt if the system cannot create its themed dialog.
    let detail = String::from_utf16_lossy(&content[..content.len().saturating_sub(1)]);
    let message = if thai {
        format!("ยังไม่ได้บันทึกไฟล์งานนี้\nYes: บันทึกงาน\nNo: ทิ้งงานและปิด\nCancel: กลับไปทำงานต่อ\n{detail}")
    } else {
        format!(
            "This editable project has not been saved.\nYes: Save project\nNo: Discard and close\nCancel: Keep working\n{detail}"
        )
    };
    unsafe {
        MessageBoxW(
            workspace_ui::native_owner(owner),
            wide(&message).as_ptr(),
            title.as_ptr(),
            MB_YESNOCANCEL | MB_ICONQUESTION | MB_DEFBUTTON3,
        )
    }
}
pub(super) fn allow_close(owner: HWND) -> bool {
    let state = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        if a.project_modal || a.project_exit.is_some() || a.video_exit.is_some() {
            return None;
        }
        a.previews
            .get(&(owner as usize))
            .map(|p| (p.saving, p.session.document.is_project_saved()))
    });
    match state {
        Some((true, _)) => false,
        Some((false, false)) => match decision(owner) {
            IDYES => {
                save(owner, false, true);
                false
            }
            IDNO => true,
            _ => false,
        },
        Some((false, true)) => true,
        None => false,
    }
}
fn abort_exit() {
    STATE.with(|s| {
        s.borrow_mut().as_mut().unwrap().project_exit = None;
    });
}
pub(super) fn start_exit() {
    start_close(None);
}
pub(super) fn close_host(host: HWND) {
    workspace_ui::commit_numeric();
    if !workspace_ui::can_switch(host) {
        return;
    }
    let context = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref()?;
        let h = a
            .workspace
            .hosts
            .iter()
            .find(|h| h.key.window == host as usize)?;
        Some((h.key, h.tabs.len(), h.active))
    });
    let Some((key, count, active)) = context else {
        return;
    };
    if count == 1 {
        if let Some(doc) = active {
            super::close_window(doc.window as HWND);
        }
        return;
    }
    use windows_sys::Win32::UI::Controls::*;
    let labels = [
        wide(&text("Close current tab", "ปิดแท็บปัจจุบัน")),
        wide(&text("Close all tabs in this window", "ปิดทุกแท็บในหน้าต่างนี้")),
        wide(&tr("cancel")),
    ];
    let buttons = [IDYES, IDNO, IDCANCEL].map(|id| TASKDIALOG_BUTTON {
        nButtonID: id,
        pszButtonText: labels[if id == IDYES {
            0
        } else if id == IDNO {
            1
        } else {
            2
        }]
        .as_ptr(),
    });
    let heading = wide(&text("Close image editor", "ปิดหน้าแก้ไขภาพ"));
    let title = wide(DIALOG_TITLE);
    let config = TASKDIALOGCONFIG {
        cbSize: size_of::<TASKDIALOGCONFIG>() as u32,
        hwndParent: host,
        dwFlags: TDF_ALLOW_DIALOG_CANCELLATION,
        pszWindowTitle: title.as_ptr(),
        pszMainInstruction: heading.as_ptr(),
        cButtons: 3,
        pButtons: buttons.as_ptr(),
        nDefaultButton: IDCANCEL,
        ..Default::default()
    };
    let mut chosen = IDCANCEL;
    {
        let Some(_modal) = Modal::enter() else {
            return;
        };
        unsafe {
            TaskDialogIndirect(&config, &mut chosen, null_mut(), null_mut());
        }
    }
    match chosen {
        IDYES => {
            if let Some(doc) = active {
                super::close_window(doc.window as HWND);
            }
        }
        IDNO => start_close(Some(key)),
        _ => (),
    }
}
fn start_close(host: Option<crate::workspace::Key>) {
    workspace_ui::commit_numeric();
    let allowed = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref().unwrap();
        a.saving == 0
            && !a.project_opening
            && !a.project_modal
            && a.project_exit.is_none()
            && a.video_exit.is_none()
            && !a.importing
            && a.text_entry.is_none()
            && !a.effects_open
            && a.previews
                .values()
                .all(|p| !workspace_ui::unfinished(&p.session))
    });
    if !allowed {
        return;
    }
    if STATE.with(|s| s.borrow().as_ref().unwrap().scrolling.is_some()) {
        let main = STATE.with(|s| s.borrow().as_ref().unwrap().main);
        let Some(_modal) = Modal::enter() else { return };
        if unsafe {
            MessageBoxW(
                main as HWND,
                wide(&tr("exit_unsaved")).as_ptr(),
                wide(DIALOG_TITLE).as_ptr(),
                MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
            )
        } != IDYES
        {
            return;
        }
        cancel_scroll();
    }
    STATE.with(|s| {
        let mut b = s.borrow_mut();
        let a = b.as_mut().unwrap();
        let mut windows: Vec<_> = a
            .previews
            .iter()
            .filter(|(h, p)| {
                host.is_none_or(|host| {
                    a.workspace.host_for(crate::workspace::Key {
                        window: **h,
                        identity: p.guard.token.identity,
                    }) == Some(host)
                })
            })
            .map(|(h, p)| (*h, p.guard.token.identity, p.session.document.revision))
            .collect();
        windows.sort_by_key(|x| x.0);
        a.project_exit = Some(ExitPlan {
            host,
            windows,
            index: 0,
            decisions: vec![],
            waiting: None,
        });
    });
    advance_exit();
}
fn advance_exit() {
    loop {
        let next = STATE.with(|s| {
            let b = s.borrow();
            let a = b.as_ref()?;
            let p = a.project_exit.as_ref()?;
            Some(p.windows.get(p.index).copied())
        });
        let Some(next) = next else { return };
        let Some((owner, identity, _)) = next else {
            let valid = STATE.with(|s| {
                let b = s.borrow();
                let a = b.as_ref().unwrap();
                let plan = a.project_exit.as_ref().unwrap();
                a.saving == 0
                    && !a.importing
                    && !a.capture_busy
                    && crate::workspace::close_set_matches(
                        &plan
                            .decisions
                            .iter()
                            .map(|(window, identity, revision)| {
                                (
                                    crate::workspace::Key {
                                        window: *window,
                                        identity: *identity,
                                    },
                                    *revision,
                                )
                            })
                            .collect::<Vec<_>>(),
                        &a.previews
                            .iter()
                            .filter_map(|(window, p)| {
                                let key = crate::workspace::Key {
                                    window: *window,
                                    identity: p.guard.token.identity,
                                };
                                plan.host
                                    .is_none_or(|host| a.workspace.host_for(key) == Some(host))
                                    .then_some((key, p.session.document.revision))
                            })
                            .collect::<Vec<_>>(),
                    )
            });
            if valid {
                let plan = STATE.with(|s| {
                    s.borrow_mut()
                        .as_mut()
                        .unwrap()
                        .project_exit
                        .take()
                        .unwrap()
                });
                if plan.host.is_some() {
                    for (window, _, _) in plan.windows {
                        super::remove_preview(window as HWND);
                    }
                } else {
                    super::finish_exit();
                }
            } else {
                abort_exit();
            }
            return;
        };
        let state = STATE.with(|s| {
            let b = s.borrow();
            let a = b.as_ref().unwrap();
            let p = a.previews.get(&owner)?;
            if p.guard.token.identity != identity {
                return None;
            }
            Some((
                p.session.document.revision,
                p.session.document.is_project_saved(),
            ))
        });
        let Some((revision, clean)) = state else {
            abort_exit();
            return;
        };
        let choice = if clean { IDNO } else { decision(owner as HWND) };
        if choice == IDCANCEL {
            abort_exit();
            return;
        }
        STATE.with(|s| {
            let mut b = s.borrow_mut();
            let p = b.as_mut().unwrap().project_exit.as_mut().unwrap();
            p.index += 1;
            p.decisions.push((owner, identity, revision));
        });
        if choice == IDYES {
            if !save(owner as HWND, false, false) {
                abort_exit();
            } else {
                STATE.with(|s| {
                    let mut b = s.borrow_mut();
                    let a = b.as_mut().unwrap();
                    let operation = a.previews[&owner].project_operation;
                    if let Some(plan) = a.project_exit.as_mut() {
                        plan.waiting = Some((owner, identity, operation));
                    }
                });
            }
            return;
        }
    }
}
pub(super) fn update_title(owner: HWND) {
    let title = STATE.with(|s| {
        let b = s.borrow();
        let a = b.as_ref()?;
        let p = a.previews.get(&(owner as usize))?;
        let name = p
            .project_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .or_else(|| p.source_label.clone())
            .unwrap_or_else(|| {
                if a.settings.language == "th" {
                    "งานใหม่".into()
                } else {
                    "Untitled project".into()
                }
            });
        Some(format!(
            "{TITLE} - {}{name}",
            if p.session.document.is_project_saved() {
                ""
            } else {
                "* "
            }
        ))
    });
    if let Some(title) = title {
        unsafe {
            SetWindowTextW(owner, wide(&title).as_ptr());
        }
    }
    workspace_ui::refresh_doc(owner);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsaved_project_prompt_is_compact_and_names_the_project() {
        let english = project_close_content("Untitled project", false);
        let thai = project_close_content("ภาพ 5", true);

        assert!(english.contains("Untitled project"));
        assert!(english.contains("Copying or exporting PNG does not save editable work."));
        assert!(thai.contains("งาน: ภาพ 5"));
        assert!(thai.contains("คัดลอกหรือส่งออก PNG ไม่ได้บันทึกงานที่แก้ไขต่อได้"));
        assert!(TASK_DIALOG_WIDTH < 340);
        assert_eq!(DIALOG_TITLE, "SnapZy");
    }

    #[test]
    fn unsaved_project_prompt_shortens_extreme_file_names() {
        let long_name = "a".repeat(100);
        let content = project_close_content(&long_name, false);
        assert!(content.chars().count() < 140);
        assert!(content.contains('…'));
    }

    #[test]
    fn neosnap_opens_unsaved_and_source_remains_unchanged_after_edit_and_save() {
        use base64::{Engine, engine::general_purpose::STANDARD};
        let dir = std::env::temp_dir().join(format!("snapzy-neosnap-open-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("source.NEOSNAP");
        let frame = Frame::new(8, 6, vec![255; 8 * 6 * 4]).unwrap();
        let mut png = vec![];
        frame.write_png(&mut png).unwrap();
        let input = serde_json::to_vec(&serde_json::json!({
            "format":"neo-snap","version":1,"width":8,"height":6,
            "background":"base","cropRect":{"x":0,"y":0,"width":8,"height":6},
            "assets":[{"id":"base","dataUrl":format!("data:image/png;base64,{}",STANDARD.encode(png))}],
            "objects":[]
        })).unwrap();
        std::fs::write(&path, &input).unwrap();
        let mut opened = load_input(&path, frame.bgra.len()).unwrap();
        let independent = load_input(&path, frame.bgra.len()).unwrap();
        assert!(!is_project(&path));
        assert!(!opened.document.is_project_saved());
        opened.document.add(
            Tool::Line,
            vec![Point::new(1., 1.), Point::new(5., 4.)],
            crate::editor::Style::default(),
        );
        assert!(independent.document.objects.is_empty());
        let destination = dir.join("converted.snapzy");
        crate::project::save(&destination, &opened.frame, &opened.document).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), input);
        assert_eq!(
            load_input(&destination, frame.bgra.len())
                .unwrap()
                .document
                .objects
                .len(),
            1
        );
        std::fs::remove_file(destination).unwrap();
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    #[test]
    fn opening_standalone_image_preserves_pixels_and_requires_a_project_save() {
        let dir = std::env::temp_dir().join(format!("snapzy-tabs-open-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("standalone.png");
        let frame = Frame::new(8, 6, vec![255; 8 * 6 * 4]).unwrap();
        write_atomic_png(&frame, &path).unwrap();
        let opened = load_input(&path, frame.bgra.len()).unwrap();
        assert_eq!(opened.frame.bgra, frame.bgra);
        assert!(!opened.document.is_project_saved());
        assert!(load_input(&path, frame.bgra.len() - 1).is_err());
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
    struct StatusWindow(HWND);
    impl StatusWindow {
        fn new() -> Self {
            unsafe {
                let parent = CreateWindowExW(
                    0,
                    wide("STATIC").as_ptr(),
                    null(),
                    0,
                    0,
                    0,
                    100,
                    30,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null(),
                );
                assert!(!parent.is_null());
                let child = CreateWindowExW(
                    0,
                    wide("STATIC").as_ptr(),
                    wide("Ready").as_ptr(),
                    WS_CHILD,
                    0,
                    0,
                    100,
                    30,
                    parent,
                    STATUS as usize as _,
                    null_mut(),
                    null(),
                );
                assert!(!child.is_null());
                Self(parent)
            }
        }
    }
    impl Drop for StatusWindow {
        fn drop(&mut self) {
            unsafe {
                DestroyWindow(self.0);
            }
        }
    }
    #[test]
    fn opening_completion_restores_launcher_and_original_editor_status() {
        for identity in [None, Some(7)] {
            let window = StatusWindow::new();
            let origin = OpenOrigin::begin(window.0, identity, "Opening project...".into());
            assert_eq!(status_text(window.0), "Opening project...");
            let main = if identity.is_none() {
                window.0 as usize
            } else {
                1
            };
            assert_eq!(origin.finish(main, identity), window.0 as usize);
            assert_eq!(status_text(window.0), "Ready");
        }
    }
    #[test]
    fn opening_completion_preserves_newer_status_and_reused_editor() {
        let window = StatusWindow::new();
        let origin = OpenOrigin::begin(window.0, Some(7), "Opening project...".into());
        status(window.0, "Saving project...");
        origin.finish(1, Some(7));
        assert_eq!(status_text(window.0), "Saving project...");
        status(window.0, "Opening project...");
        assert_eq!(origin.finish(1, Some(8)), 1);
        assert_eq!(status_text(window.0), "Opening project...");
        assert_eq!(origin.finish(1, None), 1);
        assert_eq!(status_text(window.0), "Opening project...");
    }
    #[test]
    fn late_save_cannot_update_a_reused_window_or_newer_operation() {
        assert!(current_completion(7, 3, 7, 3));
        assert!(!current_completion(8, 3, 7, 3));
        assert!(!current_completion(7, 4, 7, 3));
    }
}
