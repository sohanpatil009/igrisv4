// src/platform/file_system.rs - Cross-platform file system abstraction

use std::error::Error;
use std::path::Path;

/// Trait for cross-platform file system operations
pub trait FileSystemProvider: Send + Sync {
    /// Get system drives/mount points
    fn get_system_drives(&self) -> Result<Vec<String>, Box<dyn Error>>;
    
    /// Open a file with default application
    fn open_file(&self, file_path: &str) -> Result<String, Box<dyn Error>>;
    
    /// Open a folder in file manager
    fn open_folder(&self, folder_path: &str) -> Result<String, Box<dyn Error>>;
    
    /// Get excluded directories for file operations
    fn get_excluded_dirs(&self) -> Vec<&'static str>;
}

/// Platform-specific implementation selector
pub struct FileSystemProviderImpl;

impl FileSystemProviderImpl {
    pub fn new() -> Box<dyn FileSystemProvider> {
        #[cfg(target_os = "windows")]
        {
            Box::new(WindowsFileSystemProvider)
        }
        
        #[cfg(target_os = "linux")]
        {
            Box::new(LinuxFileSystemProvider)
        }
        
        #[cfg(target_os = "macos")]
        {
            Box::new(MacOSFileSystemProvider)
        }
    }
}

// ============================================================================
// WINDOWS IMPLEMENTATION
// ============================================================================

#[cfg(target_os = "windows")]
struct WindowsFileSystemProvider;

#[cfg(target_os = "windows")]
impl FileSystemProvider for WindowsFileSystemProvider {
    fn get_system_drives(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut drives = Vec::new();
        
        for letter in b'C'..=b'Z' {
            let drive = format!("{}:\\", letter as char);
            if Path::new(&drive).exists() {
                drives.push(drive);
            }
        }
        
        Ok(drives)
    }
    
    fn open_file(&self, file_path: &str) -> Result<String, Box<dyn Error>> {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        
        if !Path::new(file_path).is_file() {
            return Err(format!("File {} not found!", file_path).into());
        }
        
        Command::new("cmd")
            .args(&["/C", "start", "", file_path])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
        
        Ok(format!("Opened file: {}", file_path))
    }
    
    fn open_folder(&self, folder_path: &str) -> Result<String, Box<dyn Error>> {
        use std::process::Command;
        
        if !Path::new(folder_path).is_dir() {
            return Err(format!("Folder {} not found!", folder_path).into());
        }
        
        Command::new("explorer").arg(folder_path).spawn()?;
        Ok(format!("Opened folder in Explorer: {}", folder_path))
    }
    
    fn get_excluded_dirs(&self) -> Vec<&'static str> {
        vec!["AppData", "Windows", "System32", "ProgramFiles", "ProgramFiles(x86)"]
    }
}

// ============================================================================
// LINUX IMPLEMENTATION
// ============================================================================

#[cfg(target_os = "linux")]
struct LinuxFileSystemProvider;

#[cfg(target_os = "linux")]
impl FileSystemProvider for LinuxFileSystemProvider {
    fn get_system_drives(&self) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(vec!["/".to_string()])
    }
    
    fn open_file(&self, file_path: &str) -> Result<String, Box<dyn Error>> {
        use std::process::Command;
        
        if !Path::new(file_path).is_file() {
            return Err(format!("File {} not found!", file_path).into());
        }
        
        Command::new("xdg-open").arg(file_path).spawn()?;
        Ok(format!("Opened file: {}", file_path))
    }
    
    fn open_folder(&self, folder_path: &str) -> Result<String, Box<dyn Error>> {
        use std::process::Command;
        
        if !Path::new(folder_path).is_dir() {
            return Err(format!("Folder {} not found!", folder_path).into());
        }
        
        Command::new("xdg-open").arg(folder_path).spawn()?;
        Ok(format!("Opened folder: {}", folder_path))
    }
    
    fn get_excluded_dirs(&self) -> Vec<&'static str> {
        vec![".cache", ".config", ".local", ".ssh", ".gnupg"]
    }
}

// ============================================================================
// MACOS IMPLEMENTATION
// ============================================================================

#[cfg(target_os = "macos")]
struct MacOSFileSystemProvider;

#[cfg(target_os = "macos")]
impl FileSystemProvider for MacOSFileSystemProvider {
    fn get_system_drives(&self) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(vec!["/".to_string()])
    }
    
    fn open_file(&self, file_path: &str) -> Result<String, Box<dyn Error>> {
        use std::process::Command;
        
        if !Path::new(file_path).is_file() {
            return Err(format!("File {} not found!", file_path).into());
        }
        
        Command::new("open").arg(file_path).spawn()?;
        Ok(format!("Opened file: {}", file_path))
    }
    
    fn open_folder(&self, folder_path: &str) -> Result<String, Box<dyn Error>> {
        use std::process::Command;
        
        if !Path::new(folder_path).is_dir() {
            return Err(format!("Folder {} not found!", folder_path).into());
        }
        
        Command::new("open").arg(folder_path).spawn()?;
        Ok(format!("Opened folder: {}", folder_path))
    }
    
    fn get_excluded_dirs(&self) -> Vec<&'static str> {
        vec![".cache", ".config", ".local", ".ssh", ".gnupg", "Library"]
    }
}
