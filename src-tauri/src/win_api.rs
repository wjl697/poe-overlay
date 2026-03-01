use tauri::{Runtime, WebviewWindow};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_TRANSPARENT, WS_EX_LAYERED,
};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DwmExtendFrameIntoClientArea,
    DWMWA_BORDER_COLOR,
};
use windows::Win32::UI::Controls::MARGINS;

/// Toggles the mouse passthrough state for a given window.
pub fn set_window_passthrough<R: Runtime>(window: &WebviewWindow<R>, passthrough: bool) {
    let hwnd = match window.hwnd() {
        Ok(hwnd) => hwnd,
        Err(_) => return,
    };

    let hwnd = HWND(hwnd.0 as *mut core::ffi::c_void);
    set_window_passthrough_internal(hwnd, passthrough);
}

pub fn set_window_passthrough_internal(hwnd: HWND, passthrough: bool) {
    unsafe {
        let mut ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        
        if passthrough {
            ex_style |= WS_EX_TRANSPARENT.0 as isize;
            ex_style |= WS_EX_LAYERED.0 as isize;
        } else {
            ex_style &= !(WS_EX_TRANSPARENT.0 as isize);
        }

        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style);
    }
}

/// Control window shadow and always remove the Windows system border.
/// For transparent borderless overlay windows we NEVER want the DWM accent border.
/// The `shadow` parameter only controls the CSS-level shadow; the DWM border is always removed.
pub fn set_window_shadow<R: Runtime>(window: &WebviewWindow<R>, _shadow: bool) {
    // Always disable the DWM shadow + border so no GDI fallback frame appears.
    // CSS box-shadows in the WebView handle the visual depth instead.
    let _ = window.set_shadow(false);
    
    // Additionally, always clear the Windows 11 accent-color 1px border via DWM attribute.
    if let Ok(hwnd_raw) = window.hwnd() {
        let hwnd = HWND(hwnd_raw.0 as *mut core::ffi::c_void);
        unsafe {
            let color: u32 = 0xFFFFFFFE; // DWMWA_COLOR_NONE
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_BORDER_COLOR,
                &color as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            );
        }
    }
}
