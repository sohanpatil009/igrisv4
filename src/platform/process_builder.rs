// src/platform/process_builder.rs - Cross-platform process builder extension

use std::process::Command;

/// Extension trait for cross-platform process building
pub trait ProcessBuilderExt {
    /// Create a command with platform-specific defaults (e.g., hidden window on Windows)
    fn new_hidden(program: &str) -> Self;
}

impl ProcessBuilderExt for Command {
    fn new_hidden(program: &str) -> Self {
        #[allow(unused_mut)]
        let mut cmd = Command::new(program);
        
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        
        cmd
    }
}
