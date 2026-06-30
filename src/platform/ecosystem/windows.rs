use crate::eco::errors::{EcoError, EcoResult};
use super::PlatformClipboard;

pub struct WindowsClipboard;

impl PlatformClipboard for WindowsClipboard {
    fn get_text(&self) -> EcoResult<String> {
        let output = std::process::Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg("Get-Clipboard")
            .output()
            .map_err(|e| EcoError::Clipboard(e.to_string()))?;
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(text.trim().to_string())
    }

    fn set_text(&self, text: &str) -> EcoResult<()> {
        let mut child = std::process::Command::new("powershell")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(&format!("Set-Clipboard -Value '{}'", text.replace('\'', "''")))
            .spawn()
            .map_err(|e| EcoError::Clipboard(e.to_string()))?;

        child.wait()
            .map_err(|e| EcoError::Clipboard(e.to_string()))?;
        Ok(())
    }
}
