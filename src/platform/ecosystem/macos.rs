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

    fn get_image(&self) -> EcoResult<Option<Vec<u8>>> {
        let info = std::process::Command::new("osascript")
            .arg("-e")
            .arg("clipboard info")
            .output()
            .map_err(|e| EcoError::Clipboard(e.to_string()))?;
        let info_str = String::from_utf8_lossy(&info.stdout);
        let has_image = info_str.contains("TIFF") || info_str.contains("PNGf");
        if !has_image {
            return Ok(None);
        }

        let tmp_tiff = "/tmp/igris_clip_img.tiff";
        let tmp_png = "/tmp/igris_clip_img.png";

        let script = format!(
            r#"set img to (the clipboard as picture)
set tempPath to POSIX file "{}"
set fileRef to open for access tempPath with write permission
try
    write img to fileRef
end try
close access fileRef"#,
            tmp_tiff
        );
        std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| EcoError::Clipboard(e.to_string()))?;

        std::process::Command::new("/usr/bin/sips")
            .arg("-s")
            .arg("format")
            .arg("png")
            .arg(tmp_tiff)
            .arg("--out")
            .arg(tmp_png)
            .output()
            .map_err(|e| EcoError::Clipboard(e.to_string()))?;

        match std::fs::read(tmp_png) {
            Ok(data) => {
                let _ = std::fs::remove_file(tmp_tiff);
                let _ = std::fs::remove_file(tmp_png);
                Ok(Some(data))
            }
            Err(_) => Ok(None),
        }
    }

    fn set_image(&self, data: &[u8]) -> EcoResult<()> {
        let tmp_png = "/tmp/igris_clip_img.png";
        let mut file = std::fs::File::create(tmp_png)
            .map_err(|e| EcoError::Clipboard(e.to_string()))?;
        file.write_all(data)
            .map_err(|e| EcoError::Clipboard(e.to_string()))?;
        drop(file);

        let script = format!(
            r#"set the clipboard to (read (POSIX file "{}") as picture)"#,
            tmp_png
        );
        std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .output()
            .map_err(|e| EcoError::Clipboard(e.to_string()))?;

        let _ = std::fs::remove_file(tmp_png);
        Ok(())
    }
}
