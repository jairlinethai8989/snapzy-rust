# SnapZy Rust Preview

**Version:** 0.1.0-alpha.1 | **Platform:** Windows 10/11, x64 | **License:** MIT

SnapZy Rust Preview is an early, standalone Rust implementation for measuring
startup and capture responsiveness against the existing SnapZy Windows app.
It is an experiment, not a claim that Rust is faster. The two applications use
separate settings, process identity, hotkeys and installation locations.

## Features in this preview

- Compact native Windows launcher with English as the default and a Thai
  language toggle.
- System tray icon with restore and capture commands.
- Region, visible-window and full virtual-desktop capture across monitors.
- Escape cancels region or window selection.
- Original-resolution image preview, copy to clipboard and PNG export.
- Optional configurable global capture hotkey. It is off by default, checks
  conflicts and reports whether registration succeeded.
- Separate settings and performance logs under
  `%LOCALAPPDATA%\SnapZyRustPreview`.
- SnapZy blue icon with an R badge to distinguish the preview app.

## Known limits

This preview does not include image editing, scrolling capture, OCR, video
recording, `.neosnap` projects, an installer, Windows startup registration, or
settings migration from SnapZy or Neo Snap. Window capture reads the visible
desktop snapshot and refuses a window that is obscured by another window.
Protected surfaces and the secure desktop cannot be captured. Clipboard paste
and physical hotkey delivery still need tester confirmation on target PCs.

## Run the preview

1. Download and extract `SnapZy-Rust-0.1.0-alpha.1-win-x64.zip`.
2. Run `SnapZy-Rust.exe`.
3. Use **Area**, **Window**, or **Desktop**. Press Escape to cancel a selection.
4. Copy the preview or save it as PNG. Close an unsaved preview to discard it.

This alpha is a portable ZIP; it does not install or update either existing
application. Use a current 64-bit Windows 10/11 PC.

## Build from source

Install Rust `1.99.0` for `x86_64-pc-windows-msvc`, Visual Studio C++ Build
Tools, and the Windows 10/11 SDK. Then run in PowerShell:

```powershell
cargo build --release --locked
cargo test --locked
```

The executable is written to
`target\x86_64-pc-windows-msvc\release\snapzy-rust.exe`.

## Tester checks

The automated smoke test covers launch, language switching, selection cancel,
region/window/desktop capture, pixel-preserving PNG export (including a Thai
filename), hotkey conflict and success handling, and exit:

```powershell
.\tools\smoke-test.ps1
```

Please also check tray restore/menu behavior, paste into another application,
and a physical hotkey on the target Windows setup. The recorded test timings
measure when windows and controls are ready; they are not paint-completion
latencies or a direct benchmark against the .NET build.

## Initial tester feedback

On October 8, 2026, the tester reported that `0.1.0-alpha.1` felt much faster
than the .NET app on their PC. This is initial hands-on feedback, not a measured
comparison of equivalent feature sets. Repeatable side-by-side benchmarks are
the next performance check as the preview gains features.

---

# SnapZy Rust Preview (ภาษาไทย)

**เวอร์ชัน:** 0.1.0-alpha.1 | **ระบบ:** Windows 10/11, 64 บิต | **สัญญาอนุญาต:** MIT

SnapZy Rust Preview เป็นโปรแกรมทดลองที่แยกจาก SnapZy และ Neo Snap ใช้สำหรับ
วัดเวลาเริ่มโปรแกรมและตอบสนองการจับภาพเทียบกับรุ่น Windows เดิม รุ่นนี้ยังไม่ใช่
ข้อสรุปว่า Rust ทำงานเร็วกว่า .NET

## ความสามารถในรุ่นทดลอง

- หน้าหลักขนาดกะทัดรัด ใช้ภาษาอังกฤษเป็นค่าเริ่มต้นและสลับเป็นภาษาไทยได้
- ทำงานใน System Tray พร้อมเรียกหน้าหลักและคำสั่งจับภาพจากเมนู
- จับภาพพื้นที่ หน้าต่างที่มองเห็น และ desktop ทุกจอที่เชื่อมต่อ
- กด Escape เพื่อยกเลิกการเลือกพื้นที่หรือหน้าต่าง
- แสดงภาพต้นฉบับที่ความละเอียดเดิม คัดลอกลง clipboard และบันทึกเป็น PNG
- ตั้งคีย์ลัดจับภาพส่วนกลางได้ โดยปิดไว้เป็นค่าเริ่มต้น พร้อมตรวจการชนและแจ้งผล
- แยกการตั้งค่าและ performance log ไว้ที่
  `%LOCALAPPDATA%\SnapZyRustPreview`
- ใช้ไอคอน SnapZy สีน้ำเงินพร้อมตัว R เพื่อแยกจากโปรแกรมรุ่นปกติ

## ข้อจำกัด

รุ่นนี้ยังไม่มีเครื่องมือตกแต่งภาพ จับภาพแบบเลื่อน OCR บันทึกวิดีโอ ไฟล์โครงการ
`.neosnap` ตัวติดตั้ง การเริ่มพร้อม Windows หรือการย้ายการตั้งค่าจาก SnapZy และ
Neo Snap การจับภาพหน้าต่างอ่านจากภาพ desktop ที่มองเห็น และจะแจ้งเตือนหากมี
หน้าต่างอื่นบังอยู่ ไม่สามารถจับภาพหน้าจอความปลอดภัยหรือเนื้อหาที่ Windows
ป้องกันไว้ได้ การวาง clipboard และการกดคีย์ลัดจริงยังต้องตรวจบนเครื่อง tester

## ทดลองใช้

1. ดาวน์โหลดและแตกไฟล์ `SnapZy-Rust-0.1.0-alpha.1-win-x64.zip`
2. เปิด `SnapZy-Rust.exe`
3. เลือก **Area**, **Window** หรือ **Desktop** และกด Escape เพื่อยกเลิก
4. คัดลอกภาพหรือบันทึกเป็น PNG หากปิดภาพที่ยังไม่บันทึก โปรแกรมจะถามยืนยัน

รุ่น alpha นี้เป็นไฟล์ ZIP แบบ portable ไม่ติดตั้งหรืออัปเดตโปรแกรมเดิม รองรับ
Windows 10/11 รุ่น 64 บิต

## ทดสอบจากซอร์สโค้ด

ติดตั้ง Rust `1.99.0` สำหรับ `x86_64-pc-windows-msvc`, Visual Studio C++ Build
Tools และ Windows 10/11 SDK แล้วใช้ PowerShell:

```powershell
cargo build --release --locked
cargo test --locked
```

ไฟล์โปรแกรมจะอยู่ที่
`target\x86_64-pc-windows-msvc\release\snapzy-rust.exe`

## รายการที่ขอให้ tester ตรวจ

ชุดทดสอบอัตโนมัติตรวจการเปิดโปรแกรม สลับภาษา ยกเลิกการเลือก จับภาพพื้นที่/
หน้าต่าง/desktop ตรวจพิกเซลและความละเอียดของ PNG (รวมชื่อไฟล์ภาษาไทย) ตรวจคีย์ลัด
ชน/ตั้งสำเร็จ และการปิดโปรแกรม

กรุณาตรวจเพิ่มการเรียกคืนหน้าหลักและเมนูจาก Tray การวางภาพในโปรแกรมอื่น และ
การกดคีย์ลัดจริงบนเครื่อง Windows เป้าหมาย เวลาในผลทดสอบวัดจนหน้าต่างและ
คอนโทรลพร้อมใช้งาน ไม่ใช่เวลาวาดหน้าจอเสร็จหรือผลเปรียบเทียบกับรุ่น .NET

## ผลทดสอบเบื้องต้น

วันที่ 8 ตุลาคม 2026 ผู้ทดสอบรายงานว่า `0.1.0-alpha.1` ตอบสนองเร็วกว่า
รุ่น .NET มากบนเครื่องที่ทดลอง เป็นผลจากการใช้งานจริงเบื้องต้น ยังไม่ใช่
ผลวัดเปรียบเทียบรุ่นที่มีความสามารถเท่ากัน ขั้นต่อไปจะทดสอบเทียบแบบทำซ้ำได้
และตรวจความเร็วต่อเนื่องเมื่อเพิ่มความสามารถในรุ่นทดลอง
