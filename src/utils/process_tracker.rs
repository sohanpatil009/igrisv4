// src/utils/process_tracker.rs - Track processes opened by IGRIS
// Uses process NAME based tracking (not PID) because Windows `start` command
// spawns cmd.exe which exits immediately, while the actual app runs separately
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use lazy_static::lazy_static;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Process info - tracks by name, not PID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProcessInfo {
    pub name: String,           // e.g., "chrome", "notepad"
    pub exe_name: String,       // e.g., "chrome.exe", "notepad.exe"
    pub category: ProcessCategory,
}

/// Process categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessCategory {
    App,      // Regular apps (chrome, notepad, etc)
    Camera,   // Camera related processes
    Media,    // Media players
    System,   // System utilities
}

lazy_static! {
    /// Global process tracker
    pub static ref PROCESS_TRACKER: Mutex<ProcessTracker> = Mutex::new(ProcessTracker::new());
    /// Maps opened website/site names to the (browser_name, browser_exe) that opened them.
    /// e.g., "youtube" → ("Google Chrome", "Google Chrome")
    pub static ref OPENED_SITES: Mutex<HashMap<String, (String, String)>> = Mutex::new(HashMap::new());
}

pub struct ProcessTracker {
    processes: HashSet<ProcessInfo>,
}

impl ProcessTracker {
    pub fn new() -> Self {
        Self {
            processes: HashSet::new(),
        }
    }
    
    /// Add a process to tracking by name
    pub fn add(&mut self, name: &str, exe_name: &str, category: ProcessCategory) {
        let cat_str = format!("{:?}", category);
        let info = ProcessInfo {
            name: name.to_string(),
            exe_name: exe_name.to_string(),
            category,
        };
        self.processes.insert(info);
        println!("[TRACKER] Added {} ({}) - {}", name, exe_name, cat_str);
    }
    
    /// Remove a process from tracking
    pub fn remove(&mut self, exe_name: &str) {
        self.processes.retain(|p| p.exe_name != exe_name);
        println!("[TRACKER] Removed {}", exe_name);
    }
    
    /// Get all processes by category
    pub fn get_by_category(&self, category: ProcessCategory) -> Vec<ProcessInfo> {
        self.processes.iter()
            .filter(|p| p.category == category)
            .cloned()
            .collect()
    }
    
    /// Get all tracked processes
    pub fn get_all(&self) -> Vec<ProcessInfo> {
        self.processes.iter().cloned().collect()
    }
    
    /// Kill all processes in a category
    pub fn kill_category(&mut self, category: ProcessCategory) -> Result<usize, String> {
        let to_kill: Vec<String> = self.processes.iter()
            .filter(|p| p.category == category)
            .map(|p| p.exe_name.clone())
            .collect();
        
        let mut killed = 0;
        for exe_name in &to_kill {
            if kill_process_by_name(exe_name).is_ok() {
                killed += 1;
            }
        }
        
        // Remove killed processes from tracker
        self.processes.retain(|p| p.category != category);
        
        Ok(killed)
    }
    
    /// Kill all tracked processes
    pub fn kill_all(&mut self) -> Result<usize, String> {
        let to_kill: Vec<String> = self.processes.iter()
            .map(|p| p.exe_name.clone())
            .collect();
        
        let mut killed = 0;
        for exe_name in &to_kill {
            if kill_process_by_name(exe_name).is_ok() {
                killed += 1;
            }
        }
        
        self.processes.clear();
        Ok(killed)
    }
    
    /// Check if a process is still running by exe name
    pub fn is_running(&self, exe_name: &str) -> bool {
        is_process_running_by_name(exe_name)
    }
    
    /// Clean up dead processes
    pub fn cleanup(&mut self) {
        self.processes.retain(|p| is_process_running_by_name(&p.exe_name));
    }
}

// ============================================================================
// Helper functions - Kill by process NAME (not PID)
// ============================================================================

/// Kill a process by exe name (e.g., "chrome.exe")
#[cfg(target_os = "windows")]
pub fn kill_process_by_name(exe_name: &str) -> Result<(), String> {
    let output = std::process::Command::new("taskkill")
        .args(["/F", "/IM", exe_name])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .map_err(|e| e.to_string())?;
    
    if output.status.success() {
        println!("[TRACKER] Killed {}", exe_name);
        Ok(())
    } else {
        // Check if process wasn't found (not an error)
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not found") || stderr.contains("no tasks") {
            Ok(()) // Process already closed
        } else {
            Err(format!("Failed to kill {}", exe_name))
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn kill_process_by_name(exe_name: &str) -> Result<(), String> {
    // Remove .exe extension for Unix
    let name = exe_name.trim_end_matches(".exe");
    let output = std::process::Command::new("pkill")
        .args(["-f", name])
        .output()
        .map_err(|e| e.to_string())?;
    
    println!("[TRACKER] Killed {}", name);
    Ok(())
}

/// Check if process is running by exe name
#[cfg(target_os = "windows")]
pub fn is_process_running_by_name(exe_name: &str) -> bool {
    let output = std::process::Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {}", exe_name)])
        .creation_flags(0x08000000)
        .output();
    
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.to_lowercase().contains(&exe_name.to_lowercase())
        }
        Err(_) => false,
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_process_running_by_name(exe_name: &str) -> bool {
    let name = exe_name.trim_end_matches(".exe");
    let output = std::process::Command::new("pgrep")
        .args(["-f", name])
        .output();
    
    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

// ============================================================================
// Public API functions
// ============================================================================

/// Track a new process by name
pub fn track_process(name: &str, exe_name: &str, category: ProcessCategory) {
    if let Ok(mut tracker) = PROCESS_TRACKER.lock() {
        tracker.add(name, exe_name, category);
    }
}

/// Kill all app processes
pub fn close_all_apps() -> Result<String, String> {
    let mut tracker = PROCESS_TRACKER.lock().map_err(|e| e.to_string())?;
    let count = tracker.kill_category(ProcessCategory::App)?;
    if count > 0 {
        Ok(format!("Closed {} apps", count))
    } else {
        Ok("No apps to close".to_string())
    }
}

/// Kill all camera processes
pub fn close_all_camera() -> Result<String, String> {
    let mut tracker = PROCESS_TRACKER.lock().map_err(|e| e.to_string())?;
    let count = tracker.kill_category(ProcessCategory::Camera)?;
    if count > 0 {
        Ok(format!("Closed {} camera processes", count))
    } else {
        Ok("No camera processes to close".to_string())
    }
}

/// Kill all tracked processes
pub fn close_all_processes() -> Result<String, String> {
    let mut tracker = PROCESS_TRACKER.lock().map_err(|e| e.to_string())?;
    let count = tracker.kill_all()?;
    if count > 0 {
        Ok(format!("Closed {} processes", count))
    } else {
        Ok("No processes to close".to_string())
    }
}

/// Get count of tracked processes by category
pub fn get_process_count(category: ProcessCategory) -> usize {
    if let Ok(tracker) = PROCESS_TRACKER.lock() {
        tracker.get_by_category(category).len()
    } else {
        0
    }
}

/// Get all tracked process names (for context awareness)
pub fn get_tracked_app_names() -> Vec<String> {
    if let Ok(tracker) = PROCESS_TRACKER.lock() {
        tracker.get_by_category(ProcessCategory::App)
            .iter()
            .map(|p| p.name.clone())
            .collect()
    } else {
        vec![]
    }
}

/// Get total count of all tracked processes
pub fn get_total_tracked_count() -> usize {
    if let Ok(tracker) = PROCESS_TRACKER.lock() {
        tracker.get_all().len()
    } else {
        0
    }
}

/// Get tracked app count (convenience wrapper)
pub fn get_tracked_app_count() -> usize {
    get_process_count(ProcessCategory::App)
}

// ============================================================================
// Web resource tracking - maps opened sites to the browser that opened them
// ============================================================================

/// Extract a clean site name from a URL (e.g., "https://youtube.com/watch?v=..." → "youtube")
pub fn extract_site_name(url: &str) -> String {
    let url = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.");
    if let Some(dot_pos) = url.find('.') {
        url[..dot_pos].to_string()
    } else {
        url.split('/').next().unwrap_or(url).to_string()
    }
}

/// Track that a website was opened, mapping its site name to the browser used.
/// This allows "close youtube" to find and close the browser that opened it.
pub fn track_site(url: &str, browser_exe: &str, browser_name: &str) {
    let site = extract_site_name(url);
    if let Ok(mut sites) = OPENED_SITES.lock() {
        sites.insert(site.clone(), (browser_name.to_string(), browser_exe.to_string()));
    }
    // Also track the browser itself as an opened app
    track_process(browser_name, browser_exe, ProcessCategory::App);
}

/// Find the browser that was used to open a given site.
/// Returns `(browser_name, browser_exe)`.
pub fn find_browser_for_site(site: &str) -> Option<(String, String)> {
    let site_lower = site.to_lowercase();
    if let Ok(sites) = OPENED_SITES.lock() {
        // Direct match
        if let Some((browser_name, browser_exe)) = sites.get(&site_lower) {
            return Some((browser_name.clone(), browser_exe.clone()));
        }
        // Check if any tracked site name is contained in the input
        for (tracked_site, (browser_name, browser_exe)) in sites.iter() {
            if site_lower.contains(tracked_site) || tracked_site.contains(&site_lower) {
                return Some((browser_name.clone(), browser_exe.clone()));
            }
        }
    }
    None
}

/// Close a site by closing the browser that opened it.
/// Returns a message describing what was closed.
pub fn close_site(site: &str) -> Result<String, String> {
    let (browser, exe_name) = find_browser_for_site(site).ok_or_else(|| {
        format!("No browser found for site '{}'", site)
    })?;

    kill_process_by_name(&exe_name)?;

    // Clean up all sites that were using this browser
    if let Ok(mut sites) = OPENED_SITES.lock() {
        sites.retain(|_, (_, v_exe)| v_exe != &exe_name);
    }
    if let Ok(mut tracker) = PROCESS_TRACKER.lock() {
        tracker.remove(&exe_name);
    }

    Ok(format!("Closed {} (browser: {})", site, browser))
}

/// Check if a given name refers to a tracked web resource.
pub fn is_tracked_site(name: &str) -> bool {
    find_browser_for_site(name).is_some()
}

/// Get all tracked sites
pub fn get_tracked_sites() -> Vec<(String, String, String)> {
    if let Ok(sites) = OPENED_SITES.lock() {
        sites.iter().map(|(k, (n, e))| (k.clone(), n.clone(), e.clone())).collect()
    } else {
        Vec::new()
    }
}
