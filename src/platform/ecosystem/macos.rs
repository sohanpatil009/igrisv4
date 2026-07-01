use crate::eco::errors::{EcoError, EcoResult};
use super::PlatformClipboard;
use std::io::Write;

pub struct MacosClipboard;

impl PlatformClipboard for MacosClipboard {
    fn get_text(&self) -> EcoResult<String> {
        let output = std::process::Command::new("pbpaste")
            .output()
            .map_err(|e| EcoError::Clipboard(e.to_string()))?;
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(text)
    }

    fn set_text(&self, text: &str) -> EcoResult<()> {
        let mut child = std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| EcoError::Clipboard(e.to_string()))?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())
                .map_err(|e| EcoError::Clipboard(e.to_string()))?;
        }

        child.wait()
            .map_err(|e| EcoError::Clipboard(e.to_string()))?;
        Ok(())
    }
}
