#[cfg(test)]
mod tests {
    use super::{
        ENG, Language, Model, THA, Word, install_model, join_words, model_path, recognize,
        valid_input, verify_model,
    };
    use std::path::Path;

    #[test]
    fn accepts_only_the_pinned_model_bytes() {
        let model = Model {
            name: "test.traineddata",
            length: 3,
            sha256: "039058C6F2C0CB492C533B0A4D14EF77CC0F78ABCCCED5287D84A1A2011CFB81",
        };
        assert!(verify_model(&model, &[1, 2, 3]));
        assert!(!verify_model(&model, &[1, 2, 4]));
        assert!(!verify_model(&model, &[1, 2]));
    }

    #[test]
    fn joins_adjacent_thai_words_and_separates_latin_words_by_geometry() {
        let words = [
            Word {
                text: "ภาษา".into(),
                x: 0,
                right: 40,
                height: 16,
            },
            Word {
                text: "ไทย".into(),
                x: 42,
                right: 70,
                height: 16,
            },
            Word {
                text: "Snap".into(),
                x: 82,
                right: 122,
                height: 16,
            },
            Word {
                text: "Zy".into(),
                x: 131,
                right: 151,
                height: 16,
            },
        ];
        assert_eq!(join_words(&words), "ภาษาไทย Snap Zy");
    }

    #[test]
    fn rejects_ocr_images_above_pixel_or_byte_limits() {
        assert!(valid_input(2000, 2000, 16_000_000));
        assert!(!valid_input(3000, 3000, 36_000_000));
        assert!(!valid_input(
            1000,
            1000,
            crate::image_limits::IMAGE_LIMIT + 1
        ));
        assert!(!valid_input(0, 1, 4));
    }

    #[test]
    fn install_model_replaces_a_previous_valid_file_with_verified_bytes() {
        let path = std::env::temp_dir().join(format!("snapzy-ocr-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        let model = Model {
            name: "test.traineddata",
            length: 3,
            sha256: "039058C6F2C0CB492C533B0A4D14EF77CC0F78ABCCCED5287D84A1A2011CFB81",
        };
        std::fs::create_dir_all(model_path(&path, &model).parent().unwrap()).unwrap();
        std::fs::write(model_path(&path, &model), [9, 9, 9]).unwrap();
        install_model(&path, &model, &[1, 2, 3]).unwrap();
        assert_eq!(std::fs::read(model_path(&path, &model)).unwrap(), [1, 2, 3]);
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn recognizes_english_thai_and_mixed_fixture_images_when_local_models_are_configured() {
        let Ok(models) = std::env::var("SNAPZY_OCR_TEST_MODELS") else {
            return;
        };
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ocr");
        let read = |name: &str| {
            let image = image::open(fixtures.join(format!("{name}.png")))
                .unwrap()
                .to_rgba8();
            let (width, height) = image.dimensions();
            let bgra = image
                .into_raw()
                .chunks_exact(4)
                .flat_map(|p| [p[2], p[1], p[0], p[3]])
                .collect();
            crate::frame::Frame::new(width, height, bgra).unwrap()
        };
        let model_dir = std::env::temp_dir()
            .join(format!("snapzy-ocr-models-{}", std::process::id()))
            .join("OcrModels");
        std::fs::create_dir_all(&model_dir).unwrap();
        for model in [&ENG, &THA] {
            std::fs::copy(
                Path::new(&models).join(model.name),
                model_path(&model_dir, model),
            )
            .unwrap();
        }
        let english = recognize(&read("english"), &model_dir, Language::English).unwrap();
        assert!(
            english.to_ascii_lowercase().contains("snapzy"),
            "{english:?}"
        );
        let thai = recognize(&read("thai"), &model_dir, Language::ThaiEnglish).unwrap();
        assert!(thai.contains("ยินดี") || thai.contains("ภาษา"), "{thai:?}");
        let mixed = recognize(&read("mixed"), &model_dir, Language::ThaiEnglish).unwrap();
        assert!(mixed.to_ascii_lowercase().contains("snapzy"), "{mixed:?}");
        assert!(
            recognize(&read("blank"), &model_dir, Language::ThaiEnglish)
                .unwrap()
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(model_dir.parent().unwrap());
    }

    #[test]
    fn downloads_only_the_pinned_https_model_when_network_check_is_opted_in() {
        if std::env::var_os("SNAPZY_OCR_TEST_DOWNLOAD").is_none() {
            return;
        }
        for model in [super::ENG, super::THA] {
            let bytes = super::download_model(&model).unwrap();
            assert!(super::verify_model(&model, &bytes), "{}", model.name);
        }
    }
}

use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub const MAX_PIXELS: u32 = 8_000_000;
pub const MAX_INPUT_BYTES: usize = crate::image_limits::IMAGE_LIMIT;
pub const MODEL_REVISION: &str = "87416418657359cb625c412a48b6e1d6d41c29bd";

pub const ENG: Model = Model {
    name: "eng.traineddata",
    length: 4_113_088,
    sha256: "7D4322BD2A7749724879683FC3912CB542F19906C83BCC1A52132556427170B2",
};
pub const THA: Model = Model {
    name: "tha.traineddata",
    length: 1_072_600,
    sha256: "294227CC2D1292B0ACB28D61D4115C88252B96D466CA90B417CF4CF0C67BF07C",
};

#[derive(Clone, Copy)]
pub struct Model {
    pub name: &'static str,
    pub length: usize,
    pub sha256: &'static str,
}

#[derive(Debug)]
struct Word {
    text: String,
    x: i32,
    right: i32,
    height: i32,
}

pub fn verify_model(model: &Model, bytes: &[u8]) -> bool {
    bytes.len() == model.length && format!("{:X}", Sha256::digest(bytes)) == model.sha256
}

pub fn valid_input(width: u32, height: u32, bytes: usize) -> bool {
    width > 0
        && height > 0
        && width.saturating_mul(height) <= MAX_PIXELS
        && (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            == Some(bytes)
        && bytes <= MAX_INPUT_BYTES
}

fn join_words(words: &[Word]) -> String {
    let mut output = String::new();
    let mut previous: Option<&Word> = None;
    for word in words.iter().filter(|word| !word.text.trim().is_empty()) {
        if let Some(prev) = previous {
            let gap = word.x - prev.right;
            let height = prev.height.max(word.height).max(1);
            let separated = gap > 3.max((height as f32 * 0.55).round() as i32);
            let latin = is_latin_word(&prev.text) && is_latin_word(&word.text);
            if (separated || latin) && !output.ends_with(char::is_whitespace) {
                output.push(' ');
            }
        }
        output.push_str(word.text.trim());
        previous = Some(word);
    }
    output
}

fn is_latin_word(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_alphabetic())
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c.is_ascii_punctuation())
}

pub fn model_path(directory: &Path, model: &Model) -> PathBuf {
    directory.join(model.name)
}

pub fn required_models(language: Language) -> &'static [Model] {
    match language {
        Language::English => &[ENG],
        Language::ThaiEnglish => &[THA, ENG],
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Language {
    ThaiEnglish,
    English,
}

pub fn missing_or_invalid(directory: &Path, language: Language) -> Vec<Model> {
    let valid = required_models(language)
        .iter()
        .filter(|model| {
            fs::read(model_path(directory, model)).is_ok_and(|bytes| verify_model(model, &bytes))
        })
        .map(|model| model.name)
        .collect::<std::collections::HashSet<_>>();
    required_models(language)
        .iter()
        .copied()
        .filter(|model| !valid.contains(model.name))
        .collect()
}

pub fn install_model(directory: &Path, model: &Model, bytes: &[u8]) -> Result<(), String> {
    if !verify_model(model, bytes) {
        return Err(format!(
            "Downloaded {} failed integrity verification.",
            model.name
        ));
    }
    fs::create_dir_all(directory)
        .map_err(|error| format!("Cannot create OCR model folder: {error}"))?;
    let destination = model_path(directory, model);
    let staging = directory.join(format!("{}.download.tmp", model.name));
    let result: Result<(), String> = (|| {
        let mut file = fs::File::create(&staging).map_err(|error| error.to_string())?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        atomic_replace(&staging, &destination)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&staging);
    }
    result.map_err(|error| format!("Cannot install {}: {error}", model.name))
}

#[cfg(windows)]
fn atomic_replace(staging: &Path, destination: &Path) -> Result<(), String> {
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    let source = crate::ui::wide(&staging.to_string_lossy());
    let target = crate::ui::wide(&destination.to_string_lossy());
    let moved = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(format!(
            "Windows could not safely replace the OCR model ({})",
            std::io::Error::last_os_error()
        ))
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn atomic_replace(staging: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(staging, destination).map_err(|error| error.to_string())
}

pub fn recognize(
    frame: &crate::frame::Frame,
    directory: &Path,
    language: Language,
) -> Result<String, String> {
    if !valid_input(frame.width, frame.height, frame.bgra.len()) {
        return Err("This image is too large for OCR. Crop it smaller and try again.".into());
    }
    for model in required_models(language) {
        let bytes = fs::read(model_path(directory, model)).map_err(|_| {
            format!(
                "OCR language data is missing: {}. Choose Read text to install it.",
                model.name
            )
        })?;
        if !verify_model(model, &bytes) {
            return Err(format!(
                "OCR language data is damaged: {}. Choose Read text to reinstall it.",
                model.name
            ));
        }
    }
    let gray = grayscale(&frame.bgra);
    native_recognize(frame.width, frame.height, &gray, directory, language)
}

fn grayscale(bgra: &[u8]) -> Vec<u8> {
    let (pixels, _) = bgra.as_chunks::<4>();
    pixels
        .iter()
        .map(|p| {
            let alpha = p[3] as u32;
            let b = (p[0] as u32 * alpha + 255 * (255 - alpha)) / 255;
            let g = (p[1] as u32 * alpha + 255 * (255 - alpha)) / 255;
            let r = (p[2] as u32 * alpha + 255 * (255 - alpha)) / 255;
            ((r * 299 + g * 587 + b * 114) / 1000) as u8
        })
        .collect()
}

#[cfg(windows)]
fn native_recognize(
    width: u32,
    height: u32,
    gray: &[u8],
    directory: &Path,
    language: Language,
) -> Result<String, String> {
    use std::{
        ffi::{CStr, CString, c_char, c_int, c_void},
        mem::transmute,
        ptr,
    };
    use windows_sys::Win32::{
        Foundation::FreeLibrary,
        System::LibraryLoader::{
            GetProcAddress, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
            LoadLibraryExW,
        },
    };
    type Handle = *mut c_void;
    type Create = unsafe extern "C" fn() -> Handle;
    type Init = unsafe extern "C" fn(Handle, *const c_char, *const c_char) -> c_int;
    type SetImage = unsafe extern "C" fn(Handle, *const u8, c_int, c_int, c_int, c_int);
    type SetPageMode = unsafe extern "C" fn(Handle, c_int);
    type Recognize = unsafe extern "C" fn(Handle, *mut c_void) -> c_int;
    type GetIterator = unsafe extern "C" fn(Handle) -> Handle;
    type GetText = unsafe extern "C" fn(Handle, c_int) -> *mut c_char;
    type GetPage = unsafe extern "C" fn(Handle) -> Handle;
    type BoundingBox = unsafe extern "C" fn(
        Handle,
        c_int,
        *mut c_int,
        *mut c_int,
        *mut c_int,
        *mut c_int,
    ) -> c_int;
    type AtBeginning = unsafe extern "C" fn(Handle, c_int) -> c_int;
    type Next = unsafe extern "C" fn(Handle, c_int) -> c_int;
    type DeleteText = unsafe extern "C" fn(*mut c_char);
    type DeleteIterator = unsafe extern "C" fn(Handle);
    type End = unsafe extern "C" fn(Handle);
    type DeleteApi = unsafe extern "C" fn(Handle);
    type ApiFunctions = (
        Create,
        Init,
        SetImage,
        SetPageMode,
        Recognize,
        GetIterator,
        GetText,
        GetPage,
        BoundingBox,
        AtBeginning,
        Next,
        DeleteText,
        DeleteIterator,
        End,
        DeleteApi,
    );

    struct Libraries {
        tess: isize,
        lepto: isize,
    }
    impl Drop for Libraries {
        fn drop(&mut self) {
            unsafe {
                if self.tess != 0 {
                    FreeLibrary(self.tess as _);
                }
                if self.lepto != 0 {
                    FreeLibrary(self.lepto as _);
                }
            }
        }
    }
    macro_rules! symbol {
        ($library:expr, $name:literal, $kind:ty) => {{
            let address = unsafe { GetProcAddress($library as _, concat!($name, "\0").as_ptr()) }
                .ok_or_else(|| format!("OCR engine is missing symbol {}", $name))?;
            unsafe { transmute::<unsafe extern "system" fn() -> isize, $kind>(address) }
        }};
    }

    let executable_root = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("Cannot locate the OCR runtime folder.")?
        .to_owned();
    let root = if executable_root.join("leptonica-1.82.0.dll").is_file() {
        executable_root
    } else if cfg!(debug_assertions) {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/ocr-runtime")
    } else {
        executable_root
    };
    let flags = LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS;
    let lepto_path = crate::ui::wide(&root.join("leptonica-1.82.0.dll").to_string_lossy());
    let tess_path = crate::ui::wide(&root.join("tesseract50.dll").to_string_lossy());
    let lepto = unsafe { LoadLibraryExW(lepto_path.as_ptr(), ptr::null_mut(), flags) };
    if lepto.is_null() {
        return Err("Cannot load the OCR image runtime (Leptonica).".into());
    }
    let tess = unsafe { LoadLibraryExW(tess_path.as_ptr(), ptr::null_mut(), flags) };
    if tess.is_null() {
        unsafe {
            FreeLibrary(lepto);
        }
        return Err("Cannot load the OCR engine. Check the installed runtime files.".into());
    }
    let libs = Libraries {
        tess: tess as isize,
        lepto: lepto as isize,
    };
    let (
        create,
        init,
        set_image,
        set_mode,
        recognize_fn,
        get_iterator,
        get_text,
        get_page,
        bbox,
        beginning,
        next,
        delete_text,
        delete_iterator,
        end,
        delete_api,
    ): ApiFunctions = {
        (
            symbol!(libs.tess, "TessBaseAPICreate", Create),
            symbol!(libs.tess, "TessBaseAPIInit3", Init),
            symbol!(libs.tess, "TessBaseAPISetImage", SetImage),
            symbol!(libs.tess, "TessBaseAPISetPageSegMode", SetPageMode),
            symbol!(libs.tess, "TessBaseAPIRecognize", Recognize),
            symbol!(libs.tess, "TessBaseAPIGetIterator", GetIterator),
            symbol!(libs.tess, "TessResultIteratorGetUTF8Text", GetText),
            symbol!(libs.tess, "TessResultIteratorGetPageIterator", GetPage),
            symbol!(libs.tess, "TessPageIteratorBoundingBox", BoundingBox),
            symbol!(libs.tess, "TessPageIteratorIsAtBeginningOf", AtBeginning),
            symbol!(libs.tess, "TessResultIteratorNext", Next),
            symbol!(libs.tess, "TessDeleteText", DeleteText),
            symbol!(libs.tess, "TessResultIteratorDelete", DeleteIterator),
            symbol!(libs.tess, "TessBaseAPIEnd", End),
            symbol!(libs.tess, "TessBaseAPIDelete", DeleteApi),
        )
    };
    let api = unsafe { create() };
    if api.is_null() {
        return Err("Cannot create the OCR engine.".into());
    }
    struct ApiGuard {
        api: Handle,
        end: End,
        delete: DeleteApi,
    }
    impl Drop for ApiGuard {
        fn drop(&mut self) {
            unsafe {
                (self.end)(self.api);
                (self.delete)(self.api);
            }
        }
    }
    let guard = ApiGuard {
        api,
        end,
        delete: delete_api,
    };
    let data_path = CString::new(directory.to_string_lossy().as_bytes())
        .map_err(|_| "Invalid OCR model path.")?;
    let language_name = CString::new(match language {
        Language::English => "eng",
        Language::ThaiEnglish => "tha+eng",
    })
    .unwrap();
    let init_result = unsafe { init(guard.api, data_path.as_ptr(), language_name.as_ptr()) };
    if init_result != 0 {
        return Err("The OCR engine could not load its language data.".into());
    }
    unsafe {
        set_image(
            guard.api,
            gray.as_ptr(),
            width as c_int,
            height as c_int,
            1,
            width as c_int,
        );
        set_mode(guard.api, 3);
    }
    if unsafe { recognize_fn(guard.api, ptr::null_mut()) } != 0 {
        return Err("Text recognition failed. Try cropping to a smaller area.".into());
    }
    let iterator = unsafe { get_iterator(guard.api) };
    if iterator.is_null() {
        return Ok(String::new());
    }
    struct IteratorGuard {
        iterator: Handle,
        delete: DeleteIterator,
    }
    impl Drop for IteratorGuard {
        fn drop(&mut self) {
            unsafe {
                (self.delete)(self.iterator);
            }
        }
    }
    let iter_guard = IteratorGuard {
        iterator,
        delete: delete_iterator,
    };
    let mut lines: Vec<Vec<Word>> = Vec::new();
    let mut current = Vec::new();
    loop {
        let page = unsafe { get_page(iter_guard.iterator) };
        let mut left = 0;
        let mut top = 0;
        let mut right = 0;
        let mut bottom = 0;
        let valid = !page.is_null()
            && unsafe { bbox(page, 3, &mut left, &mut top, &mut right, &mut bottom) } != 0;
        if valid {
            if !current.is_empty() && unsafe { beginning(page, 2) } != 0 {
                lines.push(std::mem::take(&mut current));
            }
            let text_ptr = unsafe { get_text(iter_guard.iterator, 3) };
            if !text_ptr.is_null() {
                let text = unsafe { CStr::from_ptr(text_ptr) }
                    .to_string_lossy()
                    .trim()
                    .to_owned();
                unsafe {
                    delete_text(text_ptr);
                }
                if !text.is_empty() {
                    current.push(Word {
                        text,
                        x: left,
                        right,
                        height: (bottom - top).max(1),
                    });
                }
            }
        }
        if unsafe { next(iter_guard.iterator, 3) } == 0 {
            break;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    let result = lines
        .iter()
        .map(|line| join_words(line))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\r\n");
    Ok(result)
}

#[cfg(not(windows))]
fn native_recognize(_: u32, _: u32, _: &[u8], _: &Path, _: Language) -> Result<String, String> {
    Err("OCR is available in the Windows x64 build only.".into())
}

#[cfg(windows)]
pub fn download_model(model: &Model) -> Result<Vec<u8>, String> {
    use std::{ffi::c_void, ptr};
    use windows_sys::Win32::Networking::WinHttp::*;
    struct H(*mut c_void);
    impl Drop for H {
        fn drop(&mut self) {
            unsafe {
                if !self.0.is_null() {
                    WinHttpCloseHandle(self.0);
                }
            }
        }
    }
    unsafe fn check(result: i32, message: &str) -> Result<(), String> {
        if result == 0 {
            Err(format!("{message} (Windows error {})", unsafe {
                windows_sys::Win32::Foundation::GetLastError()
            }))
        } else {
            Ok(())
        }
    }
    let agent = crate::ui::wide("SnapZy OCR/0.9");
    let session = H(unsafe {
        WinHttpOpen(
            agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            ptr::null(),
            ptr::null(),
            0,
        )
    });
    if session.0.is_null() {
        return Err("Cannot start a secure connection for OCR models.".into());
    }
    unsafe {
        check(
            WinHttpSetTimeouts(session.0, 8000, 8000, 15000, 30000),
            "Cannot set model download timeouts",
        )?;
    }
    let host = crate::ui::wide("raw.githubusercontent.com");
    let connection =
        H(unsafe { WinHttpConnect(session.0, host.as_ptr(), INTERNET_DEFAULT_HTTPS_PORT, 0) });
    if connection.0.is_null() {
        return Err("Cannot connect to the OCR model host.".into());
    }
    let path = crate::ui::wide(&format!(
        "/tesseract-ocr/tessdata_fast/{MODEL_REVISION}/{}",
        model.name
    ));
    let request = H(unsafe {
        WinHttpOpenRequest(
            connection.0,
            crate::ui::wide("GET").as_ptr(),
            path.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            WINHTTP_FLAG_SECURE,
        )
    });
    if request.0.is_null() {
        return Err("Cannot create the OCR model request.".into());
    }
    unsafe {
        check(
            WinHttpSendRequest(request.0, ptr::null(), 0, ptr::null(), 0, 0, 0),
            "Cannot request OCR model",
        )?;
        check(
            WinHttpReceiveResponse(request.0, ptr::null_mut()),
            "Cannot receive OCR model",
        )?;
    }
    let mut status = 0u32;
    let mut status_len = 4u32;
    let queried = unsafe {
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            ptr::null(),
            (&mut status as *mut u32).cast(),
            &mut status_len,
            ptr::null_mut(),
        )
    };
    if queried == 0 || status != 200 {
        return Err("The OCR model host did not return the requested file.".into());
    }
    let mut output = Vec::with_capacity(model.length);
    loop {
        let mut available = 0;
        unsafe {
            check(
                WinHttpQueryDataAvailable(request.0, &mut available),
                "Cannot read the OCR model response",
            )?;
        }
        if available == 0 {
            break;
        }
        if output.len().saturating_add(available as usize) > model.length {
            return Err("The OCR model download exceeded its size limit.".into());
        }
        let start = output.len();
        output.resize(start + available as usize, 0);
        let mut read = 0;
        unsafe {
            check(
                WinHttpReadData(
                    request.0,
                    output[start..].as_mut_ptr().cast(),
                    available,
                    &mut read,
                ),
                "OCR model download was interrupted",
            )?;
        }
        output.truncate(start + read as usize);
    }
    if !verify_model(model, &output) {
        return Err("The downloaded OCR model failed its size or SHA-256 check.".into());
    }
    Ok(output)
}

#[cfg(not(windows))]
pub fn download_model(_: &Model) -> Result<Vec<u8>, String> {
    Err("OCR model download is available in Windows x64 only.".into())
}
