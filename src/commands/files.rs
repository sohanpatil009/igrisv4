// src/commands/files.rs - Cross-platform file operations with async multi-threaded search
use crate::platform_utils::get_file_system;
use crate::core::tts::speak_compat as speak;
use crate::{SEARCH_STATE, SearchResultData};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

/// Search result with drive info
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: PathBuf,
    pub drive: String,
    pub score: u32,
}

/// Create a new file
pub fn create_file(file_name: &str, extension: &str) -> Result<String, Box<dyn Error>> {
    let clean_name = file_name.trim().replace(" ", "_");
    let clean_ext = extension.trim().trim_start_matches('.');
    let file_path = format!("{}.{}", clean_name, clean_ext);

    if Path::new(&file_path).exists() {
        return Err(format!("File {} already exists!", file_path).into());
    }

    std::fs::File::create(&file_path)?;
    println!("✅ File created: {}", file_path);
    get_file_system().open_file(&file_path)?;

    Ok(format!("File {} created and opened!", file_path))
}

/// Delete a file
pub fn delete_file(file_name: &str, extension: &str) -> Result<String, Box<dyn Error>> {
    let clean_name = file_name.trim();
    let clean_ext = extension.trim().trim_start_matches('.');
    let file_path = format!("{}.{}", clean_name, clean_ext);

    if Path::new(&file_path).exists() {
        std::fs::remove_file(&file_path)?;
        return Ok(format!("File {} deleted!", file_path));
    }

    // Try searching
    let results = multi_threaded_file_search(clean_name, Some(clean_ext), 1)?;
    if let Some(result) = results.first() {
        std::fs::remove_file(&result.path)?;
        return Ok(format!("File {} deleted!", result.path.display()));
    }

    Err(format!("File {}.{} not found!", clean_name, clean_ext).into())
}

/// Async file search with progress tracking and cancellation support
pub async fn async_file_search(
    query: &str,
    extension: Option<&str>,
    max_results: usize,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    use tokio::task;
    use std::sync::atomic::{AtomicBool, Ordering};
    
    let query_lower = query.to_lowercase();
    let ext_lower = extension.map(|e| e.to_lowercase());
    
    // Get all drives
    let drives = get_system_drives()?;
    println!("🔍 Async searching for '{}' across {} drives...", query, drives.len());

    // Update progress - starting search
    {
        let mut state = SEARCH_STATE.lock().unwrap();
        state.is_open = true;
        state.is_searching = true;
    }

    // Cancellation flag
    let cancelled = Arc::new(AtomicBool::new(false));
    
    // Results per drive (ordered)
    let results_map: Arc<Mutex<HashMap<String, Vec<SearchResult>>>> = 
        Arc::new(Mutex::new(HashMap::new()));
    
    let mut tasks = vec![];

    // Spawn async task per drive using spawn_blocking for I/O intensive work
    for drive in drives.clone() {
        let query = query_lower.clone();
        let ext = ext_lower.clone();
        let results_map = Arc::clone(&results_map);
        let drive_clone = drive.clone();
        let cancelled_flag = Arc::clone(&cancelled);

        let task = task::spawn_blocking(move || {
            let mut drive_results = Vec::new();
            let drive_path = Path::new(&drive_clone);
            
            if drive_path.exists() && !cancelled_flag.load(Ordering::Relaxed) {
                // Update progress - current drive
                {
                    let mut state = SEARCH_STATE.lock().unwrap();
                    state.query = drive_clone.clone();
                }
                
                println!("  🔎 Searching drive: {}", drive_clone);
                search_drive_recursive_with_progress(
                    drive_path,
                    &query,
                    ext.as_deref(),
                    &mut drive_results,
                    &drive_clone,
                    &cancelled_flag,
                );
                
                if !cancelled_flag.load(Ordering::Relaxed) {
                    println!("  ✓ Drive {} complete: {} results", drive_clone, drive_results.len());
                }
            }

            // Store results for this drive
            if !cancelled_flag.load(Ordering::Relaxed) {
                if let Ok(mut map) = results_map.lock() {
                    map.insert(drive_clone.clone(), drive_results);
                }
                
                // Update progress - drive completed
                {
                    let mut state = SEARCH_STATE.lock().unwrap();
                    state.is_searching = true;
                }
            }
        });

        tasks.push(task);
    }

    // Wait for all tasks
    for task in tasks {
        let _ = task.await;
    }

    // Check if search was cancelled
    if cancelled.load(Ordering::Relaxed) {
        println!("🚫 Search cancelled by user");
        return Ok(vec![]);
    }

    // Combine results in drive order (C:, D:, E:, ...)
    let mut all_results = Vec::new();
    let map = results_map.lock().unwrap();
    
    // Sort drives alphabetically
    let mut sorted_drives = drives.clone();
    sorted_drives.sort();

    for drive in sorted_drives {
        if let Some(drive_results) = map.get(&drive) {
            all_results.extend(drive_results.clone());
        }
    }

    // Sort by score within each drive group, then limit
    all_results.sort_by(|a, b| {
        // First by drive order (already done), then by score
        b.score.cmp(&a.score)
    });

    if max_results > 0 && all_results.len() > max_results {
        all_results.truncate(max_results);
    }

    // Update final progress
    {
        let mut state = SEARCH_STATE.lock().unwrap();
        state.is_searching = false;
    }

    println!("✅ Async search complete! Found {} total results", all_results.len());
    Ok(all_results)
}

/// Legacy sync version for compatibility (calls async version)
pub fn multi_threaded_file_search(
    query: &str,
    extension: Option<&str>,
    max_results: usize,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    // Use tokio runtime to run async search
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async_file_search(query, extension, max_results))
}

/// Async folder search with progress tracking
pub async fn async_folder_search(
    query: &str,
    max_results: usize,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    use tokio::task;
    use std::sync::atomic::{AtomicBool, Ordering};
    
    let query_lower = query.to_lowercase();
    let drives = get_system_drives()?;
    
    println!("🔍 Async searching for folder '{}' across {} drives...", query, drives.len());

    // Update progress - starting search
    {
        let mut state = SEARCH_STATE.lock().unwrap();
        state.is_open = true;
        state.is_searching = true;
    }

    // Cancellation flag
    let cancelled = Arc::new(AtomicBool::new(false));

    let results_map: Arc<Mutex<HashMap<String, Vec<SearchResult>>>> = 
        Arc::new(Mutex::new(HashMap::new()));
    
    let mut tasks = vec![];

    for drive in drives.clone() {
        let query = query_lower.clone();
        let results_map = Arc::clone(&results_map);
        let drive_clone = drive.clone();
        let cancelled_flag = Arc::clone(&cancelled);

        let task = task::spawn_blocking(move || {
            let mut drive_results = Vec::new();
            let drive_path = Path::new(&drive_clone);
            
            if drive_path.exists() && !cancelled_flag.load(Ordering::Relaxed) {
                // Update progress - current drive
                {
                    let mut state = SEARCH_STATE.lock().unwrap();
                    state.query = drive_clone.clone();
                }
                
                println!("  🔎 Searching drive: {}", drive_clone);
                search_folders_recursive_with_progress(
                    drive_path,
                    &query,
                    &mut drive_results,
                    &drive_clone,
                    &cancelled_flag,
                );
                
                if !cancelled_flag.load(Ordering::Relaxed) {
                    println!("  ✓ Drive {} complete: {} folders", drive_clone, drive_results.len());
                }
            }

            if !cancelled_flag.load(Ordering::Relaxed) {
                if let Ok(mut map) = results_map.lock() {
                    map.insert(drive_clone.clone(), drive_results);
                }
                
                // Update progress - drive completed
                {
                    let mut state = SEARCH_STATE.lock().unwrap();
                    state.is_searching = true;
                }
            }
        });

        tasks.push(task);
    }

    // Wait for all tasks
    for task in tasks {
        let _ = task.await;
    }

    // Check if search was cancelled
    if cancelled.load(Ordering::Relaxed) {
        println!("🚫 Folder search cancelled by user");
        return Ok(vec![]);
    }

    let mut all_results = Vec::new();
    let map = results_map.lock().unwrap();
    
    let mut sorted_drives = drives;
    sorted_drives.sort();

    for drive in sorted_drives {
        if let Some(drive_results) = map.get(&drive) {
            all_results.extend(drive_results.clone());
        }
    }

    all_results.sort_by(|a, b| b.score.cmp(&a.score));

    if max_results > 0 && all_results.len() > max_results {
        all_results.truncate(max_results);
    }

    // Update final progress
    {
        let mut state = SEARCH_STATE.lock().unwrap();
        state.is_searching = false;
    }

    println!("✅ Async folder search complete! Found {} results", all_results.len());
    Ok(all_results)
}

/// Legacy sync version for compatibility (calls async version)
pub fn multi_threaded_folder_search(
    query: &str,
    max_results: usize,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async_folder_search(query, max_results))
}

/// Recursive file search within a drive with progress tracking (no depth limit)
fn search_drive_recursive_with_progress(
    dir: &Path,
    query: &str,
    extension: Option<&str>,
    results: &mut Vec<SearchResult>,
    drive: &str,
    cancelled: &Arc<AtomicBool>,
) {
    if cancelled.load(Ordering::Relaxed) {
        return;
    }
    
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return, // Skip inaccessible directories
    };

    for entry in entries.filter_map(|e| e.ok()) {
        if cancelled.load(Ordering::Relaxed) {
            return;
        }
        
        let path = entry.path();

        if path.is_dir() {
            // Skip system/protected directories
            if should_skip_directory(&path) {
                continue;
            }
            // Recurse into subdirectory (no depth limit)
            search_drive_recursive_with_progress(&path, query, extension, results, drive, cancelled);
        } else if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                let file_name_lower = file_name.to_lowercase();

                // Check extension if specified
                if let Some(ext) = extension {
                    if ext != "*" {
                        if !file_name_lower.ends_with(&format!(".{}", ext)) {
                            continue;
                        }
                    }
                }

                // Calculate match score
                if let Some(score) = calculate_file_score(&file_name_lower, query) {
                    results.push(SearchResult {
                        path: path.clone(),
                        drive: drive.to_string(),
                        score,
                    });
                }
            }
        }
    }
}

/// Recursive folder search within a drive with progress tracking (no depth limit)
fn search_folders_recursive_with_progress(
    dir: &Path,
    query: &str,
    results: &mut Vec<SearchResult>,
    drive: &str,
    cancelled: &Arc<AtomicBool>,
) {
    if cancelled.load(Ordering::Relaxed) {
        return;
    }
    
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        if cancelled.load(Ordering::Relaxed) {
            return;
        }
        
        let path = entry.path();

        if path.is_dir() {
            if should_skip_directory(&path) {
                continue;
            }

            // Check if this folder matches
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                let dir_name_lower = dir_name.to_lowercase();

                if let Some(score) = calculate_folder_score(&dir_name_lower, query) {
                    results.push(SearchResult {
                        path: path.clone(),
                        drive: drive.to_string(),
                        score,
                    });
                }
            }

            // Recurse (no depth limit)
            search_folders_recursive_with_progress(&path, query, results, drive, cancelled);
        }
    }
}

/// Recursive file search within a drive (no depth limit)
fn search_drive_recursive(
    dir: &Path,
    query: &str,
    extension: Option<&str>,
    results: &mut Vec<SearchResult>,
    drive: &str,
) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return, // Skip inaccessible directories
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_dir() {
            // Skip system/protected directories
            if should_skip_directory(&path) {
                continue;
            }
            // Recurse into subdirectory (no depth limit)
            search_drive_recursive(&path, query, extension, results, drive);
        } else if path.is_file() {
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                let file_name_lower = file_name.to_lowercase();

                // Check extension if specified
                if let Some(ext) = extension {
                    if ext != "*" {
                        if !file_name_lower.ends_with(&format!(".{}", ext)) {
                            continue;
                        }
                    }
                }

                // Calculate match score
                if let Some(score) = calculate_file_score(&file_name_lower, query) {
                    results.push(SearchResult {
                        path: path.clone(),
                        drive: drive.to_string(),
                        score,
                    });
                }
            }
        }
    }
}

/// Recursive folder search within a drive (no depth limit)
fn search_folders_recursive(
    dir: &Path,
    query: &str,
    results: &mut Vec<SearchResult>,
    drive: &str,
) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_dir() {
            if should_skip_directory(&path) {
                continue;
            }

            // Check if this folder matches
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                let dir_name_lower = dir_name.to_lowercase();

                if let Some(score) = calculate_folder_score(&dir_name_lower, query) {
                    results.push(SearchResult {
                        path: path.clone(),
                        drive: drive.to_string(),
                        score,
                    });
                }
            }

            // Recurse (no depth limit)
            search_folders_recursive(&path, query, results, drive);
        }
    }
}

/// Calculate match score for files
fn calculate_file_score(filename: &str, query: &str) -> Option<u32> {
    let name_without_ext = filename.rsplit('.').last().unwrap_or(filename);

    // Exact match
    if filename == query || name_without_ext == query {
        return Some(100);
    }

    // Starts with query
    if filename.starts_with(query) || name_without_ext.starts_with(query) {
        return Some(80);
    }

    // Contains query
    if filename.contains(query) || name_without_ext.contains(query) {
        return Some(60);
    }

    // Fuzzy match
    if fuzzy_match(name_without_ext, query) {
        return Some(40);
    }

    None
}

/// Calculate match score for folders
fn calculate_folder_score(dirname: &str, query: &str) -> Option<u32> {
    if dirname == query {
        return Some(100);
    }
    if dirname.starts_with(query) {
        return Some(80);
    }
    if dirname.contains(query) {
        return Some(60);
    }
    if fuzzy_match(dirname, query) {
        return Some(40);
    }
    None
}

/// Fuzzy match - all query chars appear in order
fn fuzzy_match(text: &str, query: &str) -> bool {
    let mut text_chars = text.chars().peekable();
    for query_char in query.chars() {
        loop {
            match text_chars.next() {
                Some(c) if c == query_char => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

/// Check if directory should be skipped
fn should_skip_directory(path: &Path) -> bool {
    if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
        let excluded = [
            "node_modules", ".git", ".svn", "__pycache__", ".cache",
            "$Recycle.Bin", "System Volume Information", "Windows",
            "Program Files", "Program Files (x86)", "ProgramData",
            "Recovery", "PerfLogs", "MSOCache",
        ];

        if excluded.iter().any(|e| dir_name.eq_ignore_ascii_case(e)) {
            return true;
        }

        // Skip hidden and system folders
        if dir_name.starts_with('$') {
            return true;
        }
    }
    false
}

/// Get system drives
fn get_system_drives() -> Result<Vec<String>, Box<dyn Error>> {
    get_file_system().get_system_drives()
}

/// Get user's common directories
fn get_user_directories() -> Vec<PathBuf> {
    let mut dirs_list = Vec::new();
    
    if let Some(home) = dirs::home_dir() {
        dirs_list.push(home.join("Desktop"));
        dirs_list.push(home.join("Documents"));
        dirs_list.push(home.join("Downloads"));
        dirs_list.push(home.join("Pictures"));
        dirs_list.push(home.join("Videos"));
        dirs_list.push(home.join("Music"));
        dirs_list.push(home);
    }
    
    dirs_list
}

/// Quick search in user directories first
pub fn quick_search_file(
    query: &str,
    extension: Option<&str>,
    max_results: usize,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let query_lower = query.to_lowercase();
    let ext_lower = extension.map(|e| e.to_lowercase());
    let mut results = Vec::new();

    // First search user directories (fast)
    for dir in get_user_directories() {
        if dir.exists() {
            search_drive_recursive(
                &dir,
                &query_lower,
                ext_lower.as_deref(),
                &mut results,
                &dir.to_string_lossy(),
            );
        }
    }

    // Sort by score
    results.sort_by(|a, b| b.score.cmp(&a.score));

    if results.len() >= max_results {
        results.truncate(max_results);
        return Ok(results);
    }

    // If not enough results, do full multi-threaded search
    let full_results = multi_threaded_file_search(query, extension, max_results)?;
    
    // Merge, avoiding duplicates
    for r in full_results {
        if !results.iter().any(|existing| existing.path == r.path) {
            results.push(r);
        }
    }

    results.sort_by(|a, b| b.score.cmp(&a.score));
    results.truncate(max_results);
    
    Ok(results)
}

/// Open a file
pub fn open_file(file_path: &str) -> Result<String, Box<dyn Error>> {
    if Path::new(file_path).exists() {
        get_file_system().open_file(file_path)?;
        return Ok(format!("Opened {}", file_path));
    }

    // Try to find it
    let filename = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_path);
    let ext = Path::new(file_path).extension().and_then(|s| s.to_str());

    let results = quick_search_file(filename, ext, 1)?;
    if let Some(result) = results.first() {
        get_file_system().open_file(&result.path.to_string_lossy())?;
        return Ok(format!("Opened {}", result.path.display()));
    }

    Err(format!("File {} not found!", file_path).into())
}

/// Open a folder
pub fn open_folder(folder_path: &str) -> Result<String, Box<dyn Error>> {
    let path = Path::new(folder_path);

    if path.exists() && path.is_dir() {
        get_file_system().open_folder(folder_path)?;
        return Ok(format!("Opened folder {}", folder_path));
    }

    // Try to find it
    let results = multi_threaded_folder_search(folder_path, 1)?;
    if let Some(result) = results.first() {
        get_file_system().open_folder(&result.path.to_string_lossy())?;
        return Ok(format!("Opened folder {}", result.path.display()));
    }

    Err(format!("Folder {} not found!", folder_path).into())
}

/// Read text from file
pub fn read_text_from_file(file_path: &str) -> Result<String, Box<dyn Error>> {
    if Path::new(file_path).exists() {
        return Ok(fs::read_to_string(file_path)?);
    }

    let filename = Path::new(file_path).file_stem().and_then(|s| s.to_str()).unwrap_or(file_path);
    let ext = Path::new(file_path).extension().and_then(|s| s.to_str());

    let results = quick_search_file(filename, ext, 1)?;
    if let Some(result) = results.first() {
        return Ok(fs::read_to_string(&result.path)?);
    }

    Err(format!("File {} not found!", file_path).into())
}

/// Write text to file
pub fn write_text_to_file(file_path: &str, content: &str) -> Result<String, Box<dyn Error>> {
    fs::write(file_path, content)?;
    Ok(format!("Wrote to {}", file_path))
}

/// Process file voice commands with async search
pub async fn process_file_command_async(text: &str) -> Option<String> {
    let text_lower = text.to_lowercase();

    // Create file
    if text_lower.contains("create") && (text_lower.contains("file") || text_lower.contains("document")) {
        if let Some((name, ext)) = extract_file_info(&text_lower) {
            match create_file(&name, &ext) {
                Ok(msg) => return Some(msg),
                Err(e) => return Some(format!("Error: {}", e)),
            }
        }
        return Some("Please specify a filename.".to_string());
    }

    // Delete file
    if text_lower.contains("delete") && text_lower.contains("file") {
        if let Some((name, ext)) = extract_file_info(&text_lower) {
            match delete_file(&name, &ext) {
                Ok(msg) => return Some(msg),
                Err(e) => return Some(format!("Error: {}", e)),
            }
        }
        return Some("Please specify which file to delete.".to_string());
    }

    // Search file
    if (text_lower.contains("search") || text_lower.contains("find") || text_lower.contains("locate"))
        && text_lower.contains("file")
    {
        if let Some((name, ext)) = extract_file_info(&text_lower) {
            let ext_opt = if ext == "txt" && !text_lower.contains(".txt") {
                None
            } else {
                Some(ext.as_str())
            };

            let _ = speak(&format!("Searching for {}...", name));
            
            // Set search state to show UI with loading
            {
                let mut state = SEARCH_STATE.lock().unwrap();
                state.is_open = true;
                state.is_searching = true;
                state.query = name.clone();
                state.results.clear();
            }

            match async_file_search(&name, ext_opt, 50).await {
                Ok(results) => {
                    // Update search state with results
                    {
                        let mut state = SEARCH_STATE.lock().unwrap();
                        state.is_searching = false;
                        state.results = results.iter().map(|r| {
                            let file_name = r.path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Unknown")
                                .to_string();
                            SearchResultData {
                                path: r.path.to_string_lossy().to_string(),
                                name: file_name,
                                drive: r.drive.clone(),
                                score: r.score,
                                is_folder: false,
                            }
                        }).collect();
                    }
                    
                    if results.is_empty() {
                        let _ = speak(&format!("No files found matching {}", name));
                        return Some(format!("No files found matching {}", name));
                    }

                    println!("\n📁 Search Results (ordered by drive):");
                    let mut current_drive = String::new();
                    for (i, result) in results.iter().enumerate() {
                        if result.drive != current_drive {
                            current_drive = result.drive.clone();
                            println!("\n  Drive {}:", current_drive);
                        }
                        println!("    {}. {} (score: {})", i + 1, result.path.display(), result.score);
                    }

                    let _ = speak(&format!("Found {} files matching {}", results.len(), name));
                    return Some(format!("Found {} files", results.len()));
                }
                Err(e) => {
                    // Clear searching state on error
                    {
                        let mut state = SEARCH_STATE.lock().unwrap();
                        state.is_searching = false;
                    }
                    return Some(format!("Search error: {}", e));
                }
            }
        }
        return Some("Please specify what to search for.".to_string());
    }

    // Search folder
    if (text_lower.contains("search") || text_lower.contains("find")) && text_lower.contains("folder") {
        if let Some(folder_name) = extract_folder_name(&text_lower) {
            let _ = speak(&format!("Searching for folder {}...", folder_name));
            
            // Set search state to show UI with loading
            {
                let mut state = SEARCH_STATE.lock().unwrap();
                state.is_open = true;
                state.is_searching = true;
                state.query = folder_name.clone();
                state.results.clear();
            }

            match async_folder_search(&folder_name, 50).await {
                Ok(results) => {
                    // Update search state with results
                    {
                        let mut state = SEARCH_STATE.lock().unwrap();
                        state.is_searching = false;
                        state.results = results.iter().map(|r| {
                            let dir_name = r.path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Unknown")
                                .to_string();
                            SearchResultData {
                                path: r.path.to_string_lossy().to_string(),
                                name: dir_name,
                                drive: r.drive.clone(),
                                score: r.score,
                                is_folder: true,
                            }
                        }).collect();
                    }
                    
                    if results.is_empty() {
                        let _ = speak(&format!("No folders found matching {}", folder_name));
                        return Some(format!("No folders found matching {}", folder_name));
                    }

                    println!("\n📂 Folder Search Results:");
                    let mut current_drive = String::new();
                    for (i, result) in results.iter().enumerate() {
                        if result.drive != current_drive {
                            current_drive = result.drive.clone();
                            println!("\n  Drive {}:", current_drive);
                        }
                        println!("    {}. {}", i + 1, result.path.display());
                    }

                    let _ = speak(&format!("Found {} folders matching {}", results.len(), folder_name));
                    return Some(format!("Found {} folders", results.len()));
                }
                Err(e) => {
                    // Clear searching state on error
                    {
                        let mut state = SEARCH_STATE.lock().unwrap();
                        state.is_searching = false;
                    }
                    return Some(format!("Search error: {}", e));
                }
            }
        }
    }

    // Read file
    if text_lower.contains("read") && text_lower.contains("file") {
        if let Some((name, ext)) = extract_file_info(&text_lower) {
            let file_path = format!("{}.{}", name, ext);
            match read_text_from_file(&file_path) {
                Ok(content) => {
                    let preview = if content.len() > 200 {
                        format!("{}... ({} more chars)", &content[..200], content.len() - 200)
                    } else {
                        content.clone()
                    };
                    let _ = speak(&format!("Read {}: {}", file_path, preview));
                    return Some(format!("Content of {}:\n{}", file_path, content));
                }
                Err(e) => return Some(format!("Error reading file: {}", e)),
            }
        }
    }

    // Write to file
    if text_lower.contains("write") && text_lower.contains("file") {
        // Extract content after "write to file" or "write file"
        let content_patterns = ["write to file ", "write file ", "write to "];
        for pattern in &content_patterns {
            if let Some(pos) = text_lower.find(pattern) {
                let after = &text[pattern.len() + pos..];
                // Split by "that says" or "with content" to separate file path from content
                if let Some(sep_pos) = after.find(" that says ") {
                    let file_ref = &after[..sep_pos].trim();
                    let content = after[sep_pos + 11..].trim();
                    match write_text_to_file(file_ref, content) {
                        Ok(msg) => return Some(msg),
                        Err(e) => return Some(format!("Error writing file: {}", e)),
                    }
                } else if let Some(sep_pos) = after.find(" with ") {
                    let file_ref = &after[..sep_pos].trim();
                    let content = after[sep_pos + 6..].trim();
                    match write_text_to_file(file_ref, content) {
                        Ok(msg) => return Some(msg),
                        Err(e) => return Some(format!("Error writing file: {}", e)),
                    }
                } else {
                    // Try extracting filename and content from the rest
                    let words: Vec<&str> = after.split_whitespace().collect();
                    if let Some((name, ext)) = extract_file_info(text) {
                        let file_path = format!("{}.{}", name, ext);
                        // Rest of the text after the filename is the content
                        let file_pattern = format!("{}.{}", name, ext);
                        if let Some(content_pos) = after.find(&file_pattern) {
                            let content = after[content_pos + file_pattern.len()..].trim();
                            if !content.is_empty() {
                                match write_text_to_file(&file_path, content) {
                                    Ok(msg) => return Some(msg),
                                    Err(e) => return Some(format!("Error writing file: {}", e)),
                                }
                            }
                        }
                    }
                }
            }
        }
        return Some("Please specify what to write. Try: write to file notes.txt that says hello world".to_string());
    }

    // For other file operations, use the sync version
    process_file_command(text)
}

/// Process file voice commands (sync version)
pub fn process_file_command(text: &str) -> Option<String> {
    let text_lower = text.to_lowercase();

    // Create file
    if text_lower.contains("create") && (text_lower.contains("file") || text_lower.contains("document")) {
        if let Some((name, ext)) = extract_file_info(&text_lower) {
            match create_file(&name, &ext) {
                Ok(msg) => return Some(msg),
                Err(e) => return Some(format!("Error: {}", e)),
            }
        }
        return Some("Please specify a filename.".to_string());
    }

    // Delete file
    if text_lower.contains("delete") && text_lower.contains("file") {
        if let Some((name, ext)) = extract_file_info(&text_lower) {
            match delete_file(&name, &ext) {
                Ok(msg) => return Some(msg),
                Err(e) => return Some(format!("Error: {}", e)),
            }
        }
        return Some("Please specify which file to delete.".to_string());
    }

    // Search file
    if (text_lower.contains("search") || text_lower.contains("find") || text_lower.contains("locate"))
        && text_lower.contains("file")
    {
        if let Some((name, ext)) = extract_file_info(&text_lower) {
            let ext_opt = if ext == "txt" && !text_lower.contains(".txt") {
                None
            } else {
                Some(ext.as_str())
            };

            let _ = speak(&format!("Searching for {}...", name));
            
            // Set search state to show UI with loading
            {
                let mut state = SEARCH_STATE.lock().unwrap();
                state.is_open = true;
                state.is_searching = true;
                state.query = name.clone();
                state.results.clear();
            }

            match multi_threaded_file_search(&name, ext_opt, 50) {
                Ok(results) => {
                    // Update search state with results
                    {
                        let mut state = SEARCH_STATE.lock().unwrap();
                        state.is_searching = false;
                        state.results = results.iter().map(|r| {
                            let file_name = r.path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Unknown")
                                .to_string();
                            SearchResultData {
                                path: r.path.to_string_lossy().to_string(),
                                name: file_name,
                                drive: r.drive.clone(),
                                score: r.score,
                                is_folder: false,
                            }
                        }).collect();
                    }
                    
                    if results.is_empty() {
                        let _ = speak(&format!("No files found matching {}", name));
                        return Some(format!("No files found matching {}", name));
                    }

                    println!("\n📁 Search Results (ordered by drive):");
                    let mut current_drive = String::new();
                    for (i, result) in results.iter().enumerate() {
                        if result.drive != current_drive {
                            current_drive = result.drive.clone();
                            println!("\n  Drive {}:", current_drive);
                        }
                        println!("    {}. {} (score: {})", i + 1, result.path.display(), result.score);
                    }

                    let _ = speak(&format!("Found {} files matching {}", results.len(), name));
                    return Some(format!("Found {} files", results.len()));
                }
                Err(e) => {
                    // Clear searching state on error
                    {
                        let mut state = SEARCH_STATE.lock().unwrap();
                        state.is_searching = false;
                    }
                    return Some(format!("Search error: {}", e));
                }
            }
        }
        return Some("Please specify what to search for.".to_string());
    }

    // Open file
    if text_lower.contains("open") && text_lower.contains("file") {
        if let Some((name, ext)) = extract_file_info(&text_lower) {
            let file_path = format!("{}.{}", name, ext);
            match open_file(&file_path) {
                Ok(msg) => return Some(msg),
                Err(e) => return Some(format!("Error: {}", e)),
            }
        }
    }

    // Open folder
    if text_lower.contains("open") && text_lower.contains("folder") {
        // Common folder shortcuts
        if text_lower.contains("download") {
            if let Some(home) = dirs::home_dir() {
                let downloads = home.join("Downloads");
                if downloads.exists() {
                    let _ = get_file_system().open_folder(&downloads.to_string_lossy());
                    return Some("Opened Downloads folder".to_string());
                }
            }
        }
        if text_lower.contains("document") {
            if let Some(home) = dirs::home_dir() {
                let docs = home.join("Documents");
                if docs.exists() {
                    let _ = get_file_system().open_folder(&docs.to_string_lossy());
                    return Some("Opened Documents folder".to_string());
                }
            }
        }
        if text_lower.contains("desktop") {
            if let Some(home) = dirs::home_dir() {
                let desktop = home.join("Desktop");
                if desktop.exists() {
                    let _ = get_file_system().open_folder(&desktop.to_string_lossy());
                    return Some("Opened Desktop folder".to_string());
                }
            }
        }
        if text_lower.contains("picture") || text_lower.contains("photo") {
            if let Some(home) = dirs::home_dir() {
                let pics = home.join("Pictures");
                if pics.exists() {
                    let _ = get_file_system().open_folder(&pics.to_string_lossy());
                    return Some("Opened Pictures folder".to_string());
                }
            }
        }
        if text_lower.contains("video") {
            if let Some(home) = dirs::home_dir() {
                let vids = home.join("Videos");
                if vids.exists() {
                    let _ = get_file_system().open_folder(&vids.to_string_lossy());
                    return Some("Opened Videos folder".to_string());
                }
            }
        }
        if text_lower.contains("music") {
            if let Some(home) = dirs::home_dir() {
                let music = home.join("Music");
                if music.exists() {
                    let _ = get_file_system().open_folder(&music.to_string_lossy());
                    return Some("Opened Music folder".to_string());
                }
            }
        }

        // Custom folder search
        if let Some(folder_name) = extract_folder_name(&text_lower) {
            match open_folder(&folder_name) {
                Ok(msg) => return Some(msg),
                Err(e) => return Some(format!("Error: {}", e)),
            }
        }
    }  // Close the "Open folder" if block

    // Read file
    if text_lower.contains("read") && text_lower.contains("file") {
        if let Some((name, ext)) = extract_file_info(&text_lower) {
            let file_path = format!("{}.{}", name, ext);
            match read_text_from_file(&file_path) {
                Ok(content) => {
                    let preview = if content.len() > 200 {
                        format!("{}... ({} more chars)", &content[..200], content.len() - 200)
                    } else {
                        content.clone()
                    };
                    let _ = speak(&format!("Read {}: {}", file_path, preview));
                    return Some(format!("Content of {}:\n{}", file_path, content));
                }
                Err(e) => return Some(format!("Error reading file: {}", e)),
            }
        }
    }

    // Write to file
    if text_lower.contains("write") && text_lower.contains("file") {
        let content_patterns = ["write to file ", "write file ", "write to "];
        for pattern in &content_patterns {
            if let Some(pos) = text_lower.find(pattern) {
                let after = &text[pattern.len() + pos..];
                if let Some(sep_pos) = after.find(" that says ") {
                    let file_ref = &after[..sep_pos].trim();
                    let content = after[sep_pos + 11..].trim();
                    match write_text_to_file(file_ref, content) {
                        Ok(msg) => return Some(msg),
                        Err(e) => return Some(format!("Error writing file: {}", e)),
                    }
                } else if let Some(sep_pos) = after.find(" with ") {
                    let file_ref = &after[..sep_pos].trim();
                    let content = after[sep_pos + 6..].trim();
                    match write_text_to_file(file_ref, content) {
                        Ok(msg) => return Some(msg),
                        Err(e) => return Some(format!("Error writing file: {}", e)),
                    }
                } else if let Some((name, ext)) = extract_file_info(text) {
                    let file_path = format!("{}.{}", name, ext);
                    let file_pattern = format!("{}.{}", name, ext);
                    if let Some(content_pos) = after.find(&file_pattern) {
                        let content = after[content_pos + file_pattern.len()..].trim();
                        if !content.is_empty() {
                            match write_text_to_file(&file_path, content) {
                                Ok(msg) => return Some(msg),
                                Err(e) => return Some(format!("Error writing file: {}", e)),
                            }
                        }
                    }
                }
            }
        }
        return Some("Please specify what to write. Try: write to file notes.txt that says hello world".to_string());
    }

    // Search folder
    if (text_lower.contains("search") || text_lower.contains("find")) && text_lower.contains("folder") {
        if let Some(folder_name) = extract_folder_name(&text_lower) {
            let _ = speak(&format!("Searching for folder {}...", folder_name));
            
            // Set search state to show UI with loading
            {
                let mut state = SEARCH_STATE.lock().unwrap();
                state.is_open = true;
                state.is_searching = true;
                state.query = folder_name.clone();
                state.results.clear();
            }

            match multi_threaded_folder_search(&folder_name, 50) {
                Ok(results) => {
                    // Update search state with results
                    {
                        let mut state = SEARCH_STATE.lock().unwrap();
                        state.is_searching = false;
                        state.results = results.iter().map(|r| {
                            let dir_name = r.path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("Unknown")
                                .to_string();
                            SearchResultData {
                                path: r.path.to_string_lossy().to_string(),
                                name: dir_name,
                                drive: r.drive.clone(),
                                score: r.score,
                                is_folder: true,
                            }
                        }).collect();
                    }
                    
                    if results.is_empty() {
                        let _ = speak(&format!("No folders found matching {}", folder_name));
                        return Some(format!("No folders found matching {}", folder_name));
                    }

                    println!("\n📂 Folder Search Results:");
                    let mut current_drive = String::new();
                    for (i, result) in results.iter().enumerate() {
                        if result.drive != current_drive {
                            current_drive = result.drive.clone();
                            println!("\n  Drive {}:", current_drive);
                        }
                        println!("    {}. {}", i + 1, result.path.display());
                    }

                    let _ = speak(&format!("Found {} folders matching {}", results.len(), folder_name));
                    return Some(format!("Found {} folders", results.len()));
                }
                Err(e) => {
                    // Clear searching state on error
                    {
                        let mut state = SEARCH_STATE.lock().unwrap();
                        state.is_searching = false;
                    }
                    return Some(format!("Search error: {}", e));
                }
            }
        }
    }

    None
}

/// Extract filename and extension from text
fn extract_file_info(text: &str) -> Option<(String, String)> {
    let patterns = ["called ", "named ", "file "];

    for pattern in patterns {
        if let Some(pos) = text.find(pattern) {
            let after = &text[pos + pattern.len()..];
            let name = after
                .split_whitespace()
                .next()?
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '_' && c != '-');

            if name.is_empty() {
                continue;
            }

            if let Some(dot_pos) = name.rfind('.') {
                let file_name = &name[..dot_pos];
                let ext = &name[dot_pos + 1..];
                if !file_name.is_empty() && !ext.is_empty() {
                    return Some((file_name.to_string(), ext.to_string()));
                }
            }

            let ext = if text.contains(".pdf") { "pdf" }
                else if text.contains(".doc") { "docx" }
                else if text.contains(".py") { "py" }
                else if text.contains(".rs") { "rs" }
                else if text.contains(".js") { "js" }
                else if text.contains(".json") { "json" }
                else if text.contains(".md") { "md" }
                else { "txt" };

            return Some((name.to_string(), ext.to_string()));
        }
    }

    // Find any word with extension
    for word in text.split_whitespace() {
        if word.contains('.') && word.len() > 2 {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '_');
            if let Some(dot_pos) = clean.rfind('.') {
                let name = &clean[..dot_pos];
                let ext = &clean[dot_pos + 1..];
                if !name.is_empty() && !ext.is_empty() && ext.len() <= 5 {
                    return Some((name.to_string(), ext.to_string()));
                }
            }
        }
    }

    None
}

/// Extract folder name from text
fn extract_folder_name(text: &str) -> Option<String> {
    let patterns = ["folder called ", "folder named ", "folder "];

    for pattern in patterns {
        if let Some(pos) = text.find(pattern) {
            let after = &text[pos + pattern.len()..];
            let name = after
                .split_whitespace()
                .next()?
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-');

            if !name.is_empty() && name != "in" && name != "the" && name != "my" {
                return Some(name.to_string());
            }
        }
    }

    None
}
