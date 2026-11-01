use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_OEM_3, VK_OEM_7, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
};
use winit::event_loop::EventLoopProxy;

static HOOK_CALLBACK: Mutex<Option<Box<dyn Fn() + Send + 'static>>> = Mutex::new(None);

unsafe extern "system" fn low_level_keyboard_proc(
    ncode: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if ncode >= 0 {
        let msg = wparam.0 as u32;
        if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
            let kbd = *(lparam.0 as *const KBDLLHOOKSTRUCT);
            let vk = kbd.vkCode;

            let alt_down = (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0;
            let ctrl_down = (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
            let shift_down = (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;

            let is_ctrl_quote = (vk == VK_OEM_7.0 as u32 || vk == VK_OEM_3.0 as u32)
                && ctrl_down
                && !alt_down
                && !shift_down;

            if is_ctrl_quote {
                tracing::info!("Global hotkey triggered via low-level hook (vk=0x{:X})", vk);
                if let Ok(guard) = HOOK_CALLBACK.lock() {
                    if let Some(cb) = guard.as_ref() {
                        cb();
                    }
                }
                // Return 1 to consume the key so nothing else processes it
                return LRESULT(1);
            }
        }
    }
    CallNextHookEx(HHOOK::default(), ncode, wparam, lparam)
}

#[allow(dead_code)]
pub struct HotkeyManager {
    thread_id: Arc<AtomicU32>,
    active_hotkey_name: Arc<RwLock<String>>,
    join_handle: Option<std::thread::JoinHandle<()>>,
}

#[allow(dead_code)]
impl HotkeyManager {
    pub fn new<T: 'static + Send>(
        proxy: EventLoopProxy<T>,
        on_hotkey: impl Fn() -> T + Send + 'static,
    ) -> Self {
        let thread_id = Arc::new(AtomicU32::new(0));
        let tid_clone = thread_id.clone();
        let active_hotkey_name = Arc::new(RwLock::new(String::from("Ctrl + '")));
        let name_clone = active_hotkey_name.clone();

        {
            let mut cb = HOOK_CALLBACK.lock().unwrap();
            let proxy_clone = proxy.clone();
            *cb = Some(Box::new(move || {
                let event = on_hotkey();
                let _ = proxy_clone.send_event(event);
            }));
        }

        let join_handle = std::thread::spawn(move || {
            unsafe {
                let tid = GetCurrentThreadId();
                tid_clone.store(tid, Ordering::SeqCst);

                let hook_res = SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(low_level_keyboard_proc),
                    HINSTANCE(std::ptr::null_mut()),
                    0,
                );

                match hook_res {
                    Ok(hook) => {
                        *name_clone.write().unwrap() = "Ctrl + '".to_string();
                        tracing::info!("Global low-level keyboard hook installed successfully (Ctrl + ')");

                        let mut msg = MSG::default();
                        while GetMessageW(&mut msg, HWND(std::ptr::null_mut()), 0, 0).as_bool() {
                            let _ = TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }

                        let _ = UnhookWindowsHookEx(hook);
                        tracing::info!("Global keyboard hook uninstalled on shutdown");
                    }
                    Err(e) => {
                        tracing::error!("Failed to install global keyboard hook: {:?}", e);
                    }
                }
            }
        });

        Self {
            thread_id,
            active_hotkey_name,
            join_handle: Some(join_handle),
        }
    }

    pub fn active_hotkey(&self) -> String {
        self.active_hotkey_name.read().unwrap().clone()
    }
}

impl Drop for HotkeyManager {
    fn drop(&mut self) {
        {
            let mut cb = HOOK_CALLBACK.lock().unwrap();
            *cb = None;
        }
        let tid = self.thread_id.load(Ordering::SeqCst);
        if tid != 0 {
            unsafe {
                let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}
