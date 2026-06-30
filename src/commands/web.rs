// src/web_search.rs
// Web search functionality with browser integration and result scraping

use std::process::Command;
use scraper::{Html, Selector};

/// Supported search engines
#[derive(Debug, Clone, PartialEq)]
pub enum SearchEngine {
    Google,
    Bing,
    DuckDuckGo,
    Yahoo,
}

/// Search result with snippet
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub snippet: String,
    pub url: String,
}

impl SearchEngine {
    /// Get search URL for the engine
    pub fn get_url(&self, query: &str) -> String {
        let encoded_query = urlencoding::encode(query);
        match self {
            SearchEngine::Google => format!("https://www.google.com/search?q={}", encoded_query),
            SearchEngine::Bing => format!("https://www.bing.com/search?q={}", encoded_query),
            SearchEngine::DuckDuckGo => format!("https://duckduckgo.com/?q={}", encoded_query),
            SearchEngine::Yahoo => format!("https://search.yahoo.com/search?p={}", encoded_query),
        }
    }
    
    /// Get engine name
    pub fn name(&self) -> &str {
        match self {
            SearchEngine::Google => "Google",
            SearchEngine::Bing => "Bing",
            SearchEngine::DuckDuckGo => "DuckDuckGo",
            SearchEngine::Yahoo => "Yahoo",
        }
    }
}

/// Default search engine
pub fn get_default_search_engine() -> SearchEngine {
    SearchEngine::Google
}

/// Detect search engine from command
pub fn detect_search_engine(command: &str) -> SearchEngine {
    let cmd_lower = command.to_lowercase();
    
    if cmd_lower.contains("bing") {
        SearchEngine::Bing
    } else if cmd_lower.contains("duckduckgo") || cmd_lower.contains("duck duck go") {
        SearchEngine::DuckDuckGo
    } else if cmd_lower.contains("yahoo") {
        SearchEngine::Yahoo
    } else {
        SearchEngine::Google
    }
}

/// Extract search query from command
pub fn extract_search_query(command: &str) -> Option<String> {
    let cmd_lower = command.to_lowercase();
    
    // Common search patterns
    let patterns = vec![
        "search for ",
        "search ",
        "look up ",
        "find ",
        "google ",
        "bing ",
        "look for ",
        "search about ",
        "find information about ",
        "find info about ",
    ];
    
    for pattern in patterns {
        if let Some(pos) = cmd_lower.find(pattern) {
            let query = command[pos + pattern.len()..].trim();
            if !query.is_empty() {
                return Some(query.to_string());
            }
        }
    }
    
    // If command starts with "what is", "who is", "where is", etc.
    let question_patterns = vec![
        "what is ",
        "what are ",
        "who is ",
        "who are ",
        "where is ",
        "where are ",
        "when is ",
        "when was ",
        "why is ",
        "how to ",
        "how do ",
        "how can ",
    ];
    
    for pattern in question_patterns {
        if cmd_lower.starts_with(pattern) {
            return Some(command.to_string());
        }
    }
    
    None
}

/// Open search in default browser
pub fn search_in_browser(query: &str, engine: SearchEngine) -> Result<String, Box<dyn std::error::Error>> {
    let url = engine.get_url(query);
    
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", &url])
            .spawn()?;
    }
    
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&url)
            .spawn()?;
    }
    
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&url)
            .spawn()?;
    }
    
    Ok(format!("Searching {} for: {}", engine.name(), query))
}

/// Open search in specific browser
pub fn search_in_specific_browser(
    query: &str,
    browser: &str,
    engine: SearchEngine,
) -> Result<String, Box<dyn std::error::Error>> {
    let url = engine.get_url(query);
    
    let browser_cmd = match browser.to_lowercase().as_str() {
        "chrome" | "google chrome" => {
            #[cfg(target_os = "windows")]
            { "chrome" }
            #[cfg(target_os = "linux")]
            { "google-chrome" }
            #[cfg(target_os = "macos")]
            { "Google Chrome" }
        }
        "firefox" => {
            #[cfg(target_os = "windows")]
            { "firefox" }
            #[cfg(target_os = "linux")]
            { "firefox" }
            #[cfg(target_os = "macos")]
            { "Firefox" }
        }
        "edge" | "microsoft edge" => {
            #[cfg(target_os = "windows")]
            { "msedge" }
            #[cfg(target_os = "linux")]
            { "microsoft-edge" }
            #[cfg(target_os = "macos")]
            { "Microsoft Edge" }
        }
        "safari" => {
            #[cfg(target_os = "macos")]
            { "Safari" }
            #[cfg(not(target_os = "macos"))]
            { return Err("Safari is only available on macOS".into()); }
        }
        "brave" => {
            #[cfg(target_os = "windows")]
            { "brave" }
            #[cfg(target_os = "linux")]
            { "brave-browser" }
            #[cfg(target_os = "macos")]
            { "Brave Browser" }
        }
        _ => {
            return Err(format!("Unsupported browser: {}", browser).into());
        }
    };
    
    #[cfg(target_os = "windows")]
    {
        Command::new(browser_cmd)
            .arg(&url)
            .spawn()?;
    }
    
    #[cfg(target_os = "linux")]
    {
        Command::new(browser_cmd)
            .arg(&url)
            .spawn()?;
    }
    
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", browser_cmd, &url])
            .spawn()?;
    }
    
    Ok(format!("Searching {} in {} for: {}", engine.name(), browser, query))
}

/// Process web search command
pub async fn process_search_command(command: &str) -> Option<String> {
    let cmd_lower = command.to_lowercase();
    
    // Check if it's a search command
    if !is_search_command(&cmd_lower) {
        return None;
    }
    
    // Extract search query
    let query = match extract_search_query(command) {
        Some(q) => q,
        None => return None,
    };
    
    // Detect search engine
    let engine = detect_search_engine(command);
    
    // Check if user wants to just open browser or get answer
    let should_read_results = cmd_lower.contains("what") 
        || cmd_lower.contains("who") 
        || cmd_lower.contains("where")
        || cmd_lower.contains("when")
        || cmd_lower.contains("why")
        || cmd_lower.contains("how");
    
    if should_read_results {
        if let Some(answer) = search_and_read_results(&query).await {
            let _ = search_in_browser(&query, engine);
            return Some(answer);
        }
    }
    
    // Check if specific browser is mentioned
    let browsers = vec!["chrome", "firefox", "edge", "safari", "brave"];
    let mut specific_browser: Option<&str> = None;
    
    for browser in &browsers {
        if cmd_lower.contains(browser) {
            specific_browser = Some(browser);
            break;
        }
    }
    
    // Perform search
    let result = if let Some(browser) = specific_browser {
        search_in_specific_browser(&query, browser, engine)
    } else {
        search_in_browser(&query, engine)
    };
    
    match result {
        Ok(msg) => Some(msg),
        Err(e) => Some(format!("Search failed: {}", e)),
    }
}

/// Check if command is a search command
pub fn is_search_command(command: &str) -> bool {
    let cmd_lower = command.to_lowercase();
    
    let search_keywords = vec![
        "search", "google", "bing", "look up", "find",
        "what is", "who is", "where is", "when is", "why is",
        "how to", "how do", "how can",
    ];
    
    search_keywords.iter().any(|keyword| cmd_lower.contains(keyword))
}

/// Fetch search results from Google and extract featured snippet
fn async_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to create HTTP client")
}

pub async fn fetch_search_results(query: &str) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
    let encoded_query = urlencoding::encode(query);
    let url = format!("https://www.google.com/search?q={}", encoded_query);
    
    let response = async_client().get(&url).send().await?;
    let html_content = response.text().await?;
    
    parse_google_results(&html_content)
}

/// Parse Google search results HTML
fn parse_google_results(html: &str) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
    let document = Html::parse_document(html);
    let mut results = Vec::new();
    
    // Try to extract featured snippet first (the highlighted answer box)
    if let Some(featured) = extract_featured_snippet(&document) {
        results.push(featured);
    }
    
    // Extract regular search results
    let result_selector = Selector::parse("div.g").unwrap_or_else(|_| {
        Selector::parse("div").unwrap()
    });
    
    let title_selector = Selector::parse("h3").unwrap();
    let snippet_selector = Selector::parse("div[data-sncf], div.VwiC3b, span.aCOpRe").unwrap();
    let link_selector = Selector::parse("a").unwrap();
    
    for element in document.select(&result_selector).take(5) {
        let title = element
            .select(&title_selector)
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default();
        
        let snippet = element
            .select(&snippet_selector)
            .next()
            .map(|e| e.text().collect::<String>())
            .unwrap_or_default();
        
        let url = element
            .select(&link_selector)
            .next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or_default()
            .to_string();
        
        if !title.is_empty() && !snippet.is_empty() {
            results.push(SearchResult {
                title: title.trim().to_string(),
                snippet: snippet.trim().to_string(),
                url: url.trim().to_string(),
            });
        }
    }
    
    Ok(results)
}

/// Extract featured snippet (highlighted answer box)
fn extract_featured_snippet(document: &Html) -> Option<SearchResult> {
    // Google featured snippet selectors
    let selectors = vec![
        "div.kp-blk div.kno-rdesc span",  // Knowledge panel
        "div.IZ6rdc",                      // Featured snippet
        "div.hgKElc",                      // Answer box
        "div.kp-header div.kno-rdesc",    // Knowledge graph
        "div[data-attrid='description'] span", // Description
    ];
    
    for selector_str in selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                let text = element.text().collect::<String>();
                if !text.trim().is_empty() && text.len() > 20 {
                    return Some(SearchResult {
                        title: "Featured Answer".to_string(),
                        snippet: text.trim().to_string(),
                        url: String::new(),
                    });
                }
            }
        }
    }
    
    None
}

/// Search and read results aloud
pub async fn search_and_read_results(query: &str) -> Option<String> {
    // Fetch search results
    match fetch_search_results(query).await {
        Ok(results) => {
            if results.is_empty() {
                return Some("I couldn't find any results for that query.".to_string());
            }
            
            // Get the first result (usually featured snippet or top result)
            let top_result = &results[0];
            
            // Limit snippet to reasonable length for TTS
            let snippet = if top_result.snippet.len() > 300 {
                format!("{}...", &top_result.snippet[..300])
            } else {
                top_result.snippet.clone()
            };
            
            // Return the answer
            Some(format!("Here's what I found: {}", snippet))
        }
        Err(e) => {
            eprintln!("Search error: {}", e);
            Some("I had trouble fetching the search results. Opening browser instead.".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_search_query() {
        assert_eq!(
            extract_search_query("search for rust programming"),
            Some("rust programming".to_string())
        );
        assert_eq!(
            extract_search_query("what is artificial intelligence"),
            Some("what is artificial intelligence".to_string())
        );
    }
    
    #[test]
    fn test_detect_search_engine() {
        assert_eq!(detect_search_engine("search on bing"), SearchEngine::Bing);
        assert_eq!(detect_search_engine("google this"), SearchEngine::Google);
    }
    
    #[test]
    fn test_is_search_command() {
        assert!(is_search_command("search for something"));
        assert!(is_search_command("what is rust"));
        assert!(!is_search_command("open chrome"));
    }
}
