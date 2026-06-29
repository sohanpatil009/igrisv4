// src/global_hotkey.rs
// Global hotkey registration for activating IGRIS from anywhere

use std::sync::{Arc, Mutex};
use std::thread;

/// Hotkey callback type
pub type HotkeyCallback = Arc<Mutex<dyn FnMut() + Send + 'static>>;

/// Global hotkey manager
pub struct GlobalHotkey {
    callback: Option<HotkeyCallback>,
    #[cfg(target_os = "windows")]
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl GlobalHotkey {
    pub fn new() -> Self {
        Self {
            callback: None,
            #[cfg(target_os = "windows")]
            thread_handle: None,
        }
    }

    /// Register Ctrl+Shift+Space hotkey
    pub fn register<F>(&mut self, callback: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut() + Send + 'static,
    {
        self.callback = Some(Arc::new(Mutex::new(callback)));

        #[cfg(target_os = "windows")]
        {
            self.register_windows()?;
        }

        #[cfg(target_os = "linux")]
        {
            self.register_linux()?;
        }

        #[cfg(target_os = "macos")]
        {
            self.register_macos()?;
        }

        println!("[HOTKEY] Registered Ctrl+Shift+Space");
        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn register_windows(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use winapi::um::winuser::{
            RegisterHotKey, GetMessageW, MSG,
            MOD_CONTROL, MOD_SHIFT, WM_HOTKEY,
        };
        use std::ptr::null_mut;

        let callback = self.callback.clone().unwrap();

        let handle = thread::spawn(move || {
            unsafe {
                // Register Ctrl+Shift+Space (VK_SPACE = 0x20)
                let hotkey_id = 1;
                let modifiers = MOD_CONTROL | MOD_SHIFT;
                let vk_space = 0x20;

                if RegisterHotKey(null_mut(), hotkey_id, modifiers as u32, vk_space) == 0 {
                    eprintln!("[HOTKEY] Failed to register hotkey - may be in use by another app");
                    return;
                }

                println!("[HOTKEY] Listening for Ctrl+Shift+Space...");

                // Message loop
                let mut msg: MSG = std::mem::zeroed();
                loop {
                    if GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                        if msg.message == WM_HOTKEY {
                            println!("[HOTKEY] Hotkey pressed!");
                            if let Ok(mut cb) = callback.lock() {
                                cb();
                            }
                        }
                    }
                }
            }
        });

        self.thread_handle = Some(handle);
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn register_linux(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Linux implementation using X11
        // Note: Requires x11-rs crate for full implementation
        
        let callback = self.callback.clone().unwrap();
        
        thread::spawn(move || {
            // This is a simplified version
            // Full implementation would use X11 XGrabKey
            println!("[HOTKEY] Linux hotkey support requires X11 implementation");
            println!("[HOTKEY] Please use the application window for now");
            
            // Placeholder - would need x11-rs for actual implementation
            // Example: XGrabKey with Ctrl+Shift+Space
        });

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn register_macos(&self) -> Result<(), Box<dyn std::error::Error>> {
        // macOS implementation using Carbon or Cocoa
        // Note: Requires cocoa-rs or carbon-rs crate
        
        let callback = self.callback.clone().unwrap();
        
        thread::spawn(move || {
            // This is a simplified version
            // Full implementation would use Carbon RegisterEventHotKey
            println!("[HOTKEY] macOS hotkey support requires Carbon/Cocoa implementation");
            println!("[HOTKEY] Please use the application window for now");
            
            // Placeholder - would need cocoa-rs for actual implementation
        });

        Ok(())
    }

    /// Unregister hotkey (called on drop)
    pub fn unregister(&mut self) {
        #[cfg(target_os = "windows")]
        {
            // Hotkey will be unregistered when thread exits
            println!("[HOTKEY] Unregistering hotkey...");
        }
    }
}

impl Drop for GlobalHotkey {
    fn drop(&mut self) {
        self.unregister();
    }
}

impl Default for GlobalHotkey {
    fn default() -> Self {
        Self::new()
    }
}

lazy_static::lazy_static! {
    /// Global hotkey instance
    pub static ref GLOBAL_HOTKEY: Arc<Mutex<GlobalHotkey>> = 
        Arc::new(Mutex::new(GlobalHotkey::new()));
}

/// Register the global hotkey with a callback
pub fn register_global_hotkey<F>(callback: F) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut() + Send + 'static,
{
    GLOBAL_HOTKEY.lock().unwrap().register(callback)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_creation() {
        let hotkey = GlobalHotkey::new();
        assert!(hotkey.callback.is_none());
    }
}
