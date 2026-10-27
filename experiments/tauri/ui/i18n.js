export const messages = {
  en: {
    ready: "Ready", working: "Working…", trial: "Windows capture", area: "Area", window: "Window", scrolling: "Scrolling", video: "Video",
    areaTip: "Capture a selected area", windowTip: "Capture a window", desktopTip: "Capture the entire desktop", scrollTip: "Capture a scrolling area", videoTip: "Record screen or window",
    openTip: "Open image or project", languageTip: "Switch to Thai", moreTip: "More options", delayTip: "Delay for the next capture", scrollModeTip: "Scrolling mode", destinationTip: "Open captures in",
    none: "No delay", seconds: "{n} seconds", automatic: "Automatic", manual: "Manual", tab: "New tab", newWindow: "New window",
    fullscreen: "Entire desktop", hotkeys: "Keyboard shortcuts", about: "About SnapZy", close: "Close", delay: "Capture delay", destination: "Open in",
    videoTitle: "Record video", source: "Source", screen: "Screen", audio: "Audio", silent: "No audio", system: "System audio", microphone: "Microphone", both: "System + microphone", record: "Start recording",
    aboutText: "SnapZy 0.15.0-alpha.10", engineVersion: "Rust capture engine 0.15.0-alpha.10", license: "MIT License · © 2026 jairlinethai",
    desktopOnly: "Open SnapZy.exe to capture your screen. This browser view previews the interface.", failed: "The command could not finish.", dismiss: "Dismiss", zoom: "SnapZy",
  },
  th: {
    ready: "พร้อมใช้งาน", working: "กำลังทำงาน…", trial: "จับภาพบน Windows", area: "พื้นที่", window: "หน้าต่าง", scrolling: "ภาพยาว", video: "วิดีโอ",
    areaTip: "จับภาพเฉพาะพื้นที่ที่เลือก", windowTip: "จับภาพหน้าต่าง", desktopTip: "จับภาพเดสก์ท็อปทั้งหมด", scrollTip: "จับภาพพื้นที่ที่เลื่อนได้", videoTip: "อัดวิดีโอหน้าจอหรือหน้าต่าง",
    openTip: "เปิดภาพหรือไฟล์งาน", languageTip: "Switch to English", moreTip: "ตัวเลือกเพิ่มเติม", delayTip: "หน่วงเวลาจับภาพครั้งถัดไป", scrollModeTip: "โหมดจับภาพยาว", destinationTip: "เปิดภาพที่จับใน",
    none: "ไม่หน่วง", seconds: "{n} วินาที", automatic: "อัตโนมัติ", manual: "เลื่อนเอง", tab: "แท็บใหม่", newWindow: "หน้าต่างใหม่",
    fullscreen: "ทั้งเดสก์ท็อป", hotkeys: "ตั้งค่าคีย์ลัด", about: "เกี่ยวกับ SnapZy", close: "ปิด", delay: "หน่วงเวลาจับภาพ", destination: "เปิดภาพใน",
    videoTitle: "อัดวิดีโอ", source: "พื้นที่บันทึก", screen: "หน้าจอ", audio: "เสียง", silent: "ไม่บันทึกเสียง", system: "เสียงจากเครื่อง", microphone: "ไมโครโฟน", both: "เครื่อง + ไมโครโฟน", record: "เริ่มบันทึก",
    aboutText: "SnapZy 0.15.0-alpha.10", engineVersion: "ระบบจับภาพ Rust 0.15.0-alpha.10", license: "MIT License · © 2026 jairlinethai",
    desktopOnly: "เปิด SnapZy.exe เพื่อจับภาพหน้าจอ หน้านี้เป็นตัวอย่างหน้าตาในเบราว์เซอร์", failed: "ไม่สามารถทำคำสั่งให้เสร็จได้", dismiss: "ปิดข้อความ", zoom: "SnapZy",
  },
};

export function translate(language, key) {
  return messages[language]?.[key] ?? messages.en[key] ?? key;
}
