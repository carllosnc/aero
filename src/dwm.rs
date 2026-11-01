use windows::Win32::Foundation::{BOOL, HWND};
use windows::Win32::Graphics::Dwm::{
    DwmEnableBlurBehindWindow, DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE,
    DWMWA_USE_IMMERSIVE_DARK_MODE, DWM_BB_ENABLE, DWM_BLURBEHIND,
};
use windows::Win32::Graphics::Gdi::HRGN;

/// Applies modern Windows 11 / 10 backdrop effects (Acrylic, Mica, or DWM blur-behind)
pub fn apply_dwm_effects(hwnd: HWND) {
    unsafe {
        // 1. Enable immersive dark mode for the window frame
        let dark_mode = BOOL(1);
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &dark_mode as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<BOOL>() as u32,
        );

        // 2. Windows 11 22H2+ System Backdrop:
        //    DWMSBT_AUTO = 0
        //    DWMSBT_NONE = 1
        //    DWMSBT_MAINWINDOW (Mica) = 2
        //    DWMSBT_TRANSIENTWINDOW (Acrylic) = 3
        //    DWMSBT_TABBEDWINDOW (Mica Alt) = 4
        let acrylic_type: u32 = 3;
        let res = DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            &acrylic_type as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );

        // 3. Fallback for Windows 10: DwmEnableBlurBehindWindow
        if res.is_err() {
            let bb = DWM_BLURBEHIND {
                dwFlags: DWM_BB_ENABLE,
                fEnable: BOOL(1),
                hRgnBlur: HRGN(std::ptr::null_mut()),
                fTransitionOnMaximized: BOOL(0),
            };
            let _ = DwmEnableBlurBehindWindow(hwnd, &bb);
            tracing::info!("Applied legacy Windows DWM blur-behind effect");
        } else {
            tracing::info!("Applied Windows 11 Acrylic system backdrop effect");
        }
    }
}
