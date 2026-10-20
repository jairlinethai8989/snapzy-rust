use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/snapzy-rust.ico");
    println!("cargo:rerun-if-changed=assets/app.manifest");
    println!("cargo:rerun-if-changed=assets/ocr-runtime/tesseract50.dll");
    println!("cargo:rerun-if-changed=assets/ocr-runtime/leptonica-1.82.0.dll");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    if let Some(profile) = out.ancestors().nth(3) {
        for name in ["tesseract50.dll", "leptonica-1.82.0.dll"] {
            fs::copy(
                root.join("assets/ocr-runtime").join(name),
                profile.join(name),
            )
            .unwrap_or_else(|error| panic!("Cannot copy OCR runtime {name}: {error}"));
        }
    }
    let escaped = |p: PathBuf| p.to_string_lossy().replace('\\', "\\\\");
    let package_version = env::var("CARGO_PKG_VERSION").unwrap();
    let numeric_version = package_version
        .split_once('-')
        .map(|(core, prerelease)| {
            let build = prerelease
                .rsplit_once('.')
                .and_then(|(_, number)| number.parse::<u16>().ok())
                .unwrap_or(0);
            format!("{core},{build}")
        })
        .unwrap_or_else(|| package_version.replace('.', ","));
    let rc = format!(
        r#"
1 ICON "{}"
1 24 "{}"
1 VERSIONINFO
 FILEVERSION {numeric_version}
PRODUCTVERSION {numeric_version}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x2L
FILEOS 0x40004L
FILETYPE 0x1L
BEGIN
 BLOCK "StringFileInfo"
 BEGIN
  BLOCK "040904b0"
  BEGIN
   VALUE "CompanyName", "jairlinethai\0"
   VALUE "FileDescription", "SnapZy Rust Preview\0"
   VALUE "FileVersion", "{package_version}\0"
   VALUE "ProductName", "SnapZy Rust Preview\0"
   VALUE "ProductVersion", "{package_version}\0"
   VALUE "OriginalFilename", "SnapZy-Rust.exe\0"
   VALUE "LegalCopyright", "Copyright (c) 2026 jairlinethai\0"
  END
 END
 BLOCK "VarFileInfo"
 BEGIN
  VALUE "Translation", 0x409, 1200
 END
END
"#,
        escaped(root.join("assets/snapzy-rust.ico")),
        escaped(root.join("assets/app.manifest")),
        numeric_version = numeric_version,
        package_version = package_version
    );
    let source = out.join("app.rc");
    let resource = out.join("app.res");
    fs::write(&source, rc).unwrap();
    let compiler = env::var_os("RC").map(PathBuf::from).unwrap_or_else(|| {
        let base =
            PathBuf::from(env::var_os("ProgramFiles(x86)").unwrap()).join("Windows Kits/10/bin");
        let mut versions: Vec<_> = fs::read_dir(base)
            .unwrap()
            .flatten()
            .map(|e| e.path().join("x64/rc.exe"))
            .filter(|p| p.is_file())
            .collect();
        versions.sort();
        versions
            .pop()
            .expect("Install the Windows SDK resource compiler")
    });
    let status = Command::new(compiler)
        .arg("/nologo")
        .arg("/fo")
        .arg(&resource)
        .arg(&source)
        .status()
        .unwrap();
    assert!(status.success(), "Windows resource compilation failed");
    println!("cargo:rustc-link-arg={}", resource.display());
}
