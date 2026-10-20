use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

pub fn available_physical_bytes() -> Option<u64> {
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    (unsafe { GlobalMemoryStatusEx(&mut status) } != 0).then_some(status.ullAvailPhys)
}

pub fn available_scroll_bytes() -> usize {
    available_physical_bytes()
        .unwrap_or(0)
        .saturating_sub(crate::image_limits::MEMORY_RESERVE)
        .min(usize::MAX as u64) as usize
}
