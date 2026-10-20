use crate::{frame::Frame, video::Source};
use std::{
    mem::zeroed,
    ptr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use windows::{
    Foundation::TypedEventHandler,
    Graphics::{
        Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession},
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
    },
    Win32::{
        Foundation::{HMODULE, HWND},
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0},
            Direct3D11::{
                D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAP_READ,
                D3D11_MAPPED_SUBRESOURCE, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
                D3D11_USAGE_STAGING, D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext,
                ID3D11Texture2D,
            },
            Dxgi::IDXGIDevice,
        },
        System::WinRT::{
            Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess},
            Graphics::Capture::IGraphicsCaptureItemInterop,
        },
        UI::WindowsAndMessaging::{GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible},
    },
    core::{HRESULT, IInspectable, Interface, factory},
};

pub struct Capture {
    source: Source,
    item: GraphicsCaptureItem,
    closed_token: i64,
    closed: Arc<AtomicBool>,
    frame_pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    staging: ID3D11Texture2D,
    width: u32,
    height: u32,
}

impl Capture {
    pub fn new(source: Source) -> Result<Self, String> {
        let (device, context) = d3d_device()?;
        let dxgi: IDXGIDevice = device.cast().map_err(|e| e.to_string())?;
        let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi) }
            .map_err(|e| format!("Cannot prepare Windows Graphics Capture: {e}"))?;
        let winrt_device: IDirect3DDevice = inspectable.cast().map_err(|e| e.to_string())?;
        let interop: IGraphicsCaptureItemInterop =
            factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
                .map_err(|e| format!("Windows Graphics Capture is unavailable: {e}"))?;
        let item = unsafe {
            match source {
                Source::Window { handle, .. } => {
                    interop.CreateForWindow::<GraphicsCaptureItem>(HWND(handle as *mut _))
                }
                Source::Monitor { handle } => interop.CreateForMonitor::<GraphicsCaptureItem>(
                    windows::Win32::Graphics::Gdi::HMONITOR(handle as *mut _),
                ),
            }
        }
        .map_err(|e| format!("Cannot bind the recording source: {e}"))?;
        let size = item.Size().map_err(|e| e.to_string())?;
        if size.Width <= 0 || size.Height <= 0 {
            return Err("The recording source has no visible content.".into());
        }
        let width = size.Width as u32;
        let height = size.Height as u32;
        crate::video::dimensions(width, height)?;
        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &winrt_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size,
        )
        .map_err(|e| format!("Cannot create the recording frame pool: {e}"))?;
        let session = frame_pool
            .CreateCaptureSession(&item)
            .map_err(|e| e.to_string())?;
        let staging = staging_texture(&device, width, height)?;
        let closed = Arc::new(AtomicBool::new(false));
        let flag = closed.clone();
        let closed_token = item
            .Closed(
                &TypedEventHandler::<GraphicsCaptureItem, IInspectable>::new(move |_, _| {
                    flag.store(true, Ordering::Release);
                    Ok(())
                }),
            )
            .map_err(|e| e.to_string())?;
        session
            .StartCapture()
            .map_err(|e| format!("Cannot start Windows Graphics Capture: {e}"))?;
        Ok(Self {
            source,
            item,
            closed_token,
            closed,
            frame_pool,
            session,
            device,
            context,
            staging,
            width,
            height,
        })
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn is_live(&self) -> bool {
        if self.closed.load(Ordering::Acquire) {
            return false;
        }
        match self.source {
            Source::Window { handle, pid } => unsafe {
                let window = HWND(handle as *mut _);
                let mut current_pid = 0;
                IsWindow(Some(window)).as_bool()
                    && IsWindowVisible(window).as_bool()
                    && !IsIconic(window).as_bool()
                    && GetWindowThreadProcessId(window, Some(&mut current_pid)) != 0
                    && current_pid == pid
            },
            Source::Monitor { handle } => unsafe {
                let mut info: windows::Win32::Graphics::Gdi::MONITORINFO = zeroed();
                info.cbSize = std::mem::size_of_val(&info) as u32;
                windows::Win32::Graphics::Gdi::GetMonitorInfoW(
                    windows::Win32::Graphics::Gdi::HMONITOR(handle as *mut _),
                    &mut info,
                )
                .as_bool()
            },
        }
    }

    pub fn next(&self) -> Result<Option<Frame>, String> {
        if !self.is_live() {
            return Err("The recording source was closed or disconnected.".into());
        }
        unsafe { self.device.GetDeviceRemovedReason() }
            .map_err(|e| format!("The recording graphics device was lost: {e}"))?;
        let captured = match self.frame_pool.TryGetNextFrame() {
            Ok(frame) => frame,
            Err(error) if empty_frame_status(error.code()) => return Ok(None),
            Err(error) => return Err(format!("Cannot receive a recording frame: {error}")),
        };
        let size = captured.ContentSize().map_err(|e| e.to_string())?;
        if size.Width != self.width as i32 || size.Height != self.height as i32 {
            let _ = captured.Close();
            return Err("The recording source changed size.".into());
        }
        let surface = captured.Surface().map_err(|e| e.to_string())?;
        let access: IDirect3DDxgiInterfaceAccess = surface.cast().map_err(|e| e.to_string())?;
        let texture: ID3D11Texture2D =
            unsafe { access.GetInterface() }.map_err(|e| e.to_string())?;
        unsafe {
            self.context.CopyResource(&self.staging, &texture);
        }
        let frame = self.read_staging();
        let _ = captured.Close();
        frame.map(Some)
    }

    fn read_staging(&self) -> Result<Frame, String> {
        let mut mapped: D3D11_MAPPED_SUBRESOURCE = unsafe { zeroed() };
        unsafe {
            self.context
                .Map(&self.staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
        }
        .map_err(|e| format!("Cannot read a recording frame: {e}"))?;
        let result = (|| {
            let row = self.width as usize * 4;
            if mapped.pData.is_null() || (mapped.RowPitch as usize) < row {
                return Err("Windows returned an incomplete recording frame.".into());
            }
            let mut pixels = vec![0; Frame::byte_len(self.width, self.height)?];
            for y in 0..self.height as usize {
                unsafe {
                    ptr::copy_nonoverlapping(
                        (mapped.pData as *const u8).add(y * mapped.RowPitch as usize),
                        pixels.as_mut_ptr().add(y * row),
                        row,
                    );
                }
            }
            Frame::new(self.width, self.height, pixels)
        })();
        unsafe {
            self.context.Unmap(&self.staging, 0);
        }
        result
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        let _ = self.item.RemoveClosed(self.closed_token);
        let _ = self.session.Close();
        let _ = self.frame_pool.Close();
        let _ = &self.device;
    }
}

fn d3d_device() -> Result<(ID3D11Device, ID3D11DeviceContext), String> {
    let mut device = None;
    let mut context = None;
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&[D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )
    }
    .map_err(|e| format!("Cannot create a Direct3D recording device: {e}"))?;
    Ok((
        device.ok_or("Windows returned no Direct3D device.")?,
        context.ok_or("Windows returned no Direct3D context.")?,
    ))
}

fn staging_texture(
    device: &ID3D11Device,
    width: u32,
    height: u32,
) -> Result<ID3D11Texture2D, String> {
    let description = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_STAGING,
        BindFlags: 0,
        CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
        MiscFlags: 0,
    };
    let mut texture = None;
    unsafe { device.CreateTexture2D(&description, None, Some(&mut texture)) }
        .map_err(|e| format!("Cannot allocate the recording readback surface: {e}"))?;
    texture.ok_or_else(|| "Windows returned no recording readback surface.".into())
}

fn empty_frame_status(code: HRESULT) -> bool {
    code == HRESULT(0) || code == HRESULT(0x8000_4003u32 as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_frame_pool_statuses_are_not_treated_as_capture_failure() {
        assert!(empty_frame_status(HRESULT(0)));
        assert!(empty_frame_status(HRESULT(0x8000_4003u32 as i32)));
        assert!(!empty_frame_status(HRESULT(0x8000_0405u32 as i32)));
    }
}
