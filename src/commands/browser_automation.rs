use std::process::Command;

use crate::core::tts::speak_compat as speak;

/// A known site entry: (keyword, display_name, url_template)
/// url_template uses {} for the encoded query.
struct SiteEntry(&'static str, &'static str, &'static str);

/// Known sites and how to construct a search URL for each.
fn known_sites() -> Vec<SiteEntry> {
    vec![
        SiteEntry("google",       "Google",        "https://www.google.com/search?q={}"),
        SiteEntry("youtube",      "YouTube",       "https://www.youtube.com/results?search_query={}"),
        SiteEntry("amazon",       "Amazon",        "https://www.amazon.com/s?k={}"),
        SiteEntry("reddit",       "Reddit",        "https://www.reddit.com/search/?q={}"),
        SiteEntry("wikipedia",    "Wikipedia",     "https://en.wikipedia.org/wiki/Special:Search?search={}"),
        SiteEntry("twitter",      "Twitter/X",     "https://twitter.com/search?q={}"),
        SiteEntry("x",            "Twitter/X",     "https://twitter.com/search?q={}"),
        SiteEntry("github",       "GitHub",        "https://github.com/search?q={}"),
        SiteEntry("stackoverflow","Stack Overflow","https://stackoverflow.com/search?q={}"),
        SiteEntry("stack",        "Stack Overflow","https://stackoverflow.com/search?q={}"),
        SiteEntry("ebay",         "eBay",          "https://www.ebay.com/sch/i.html?_nkw={}"),
        SiteEntry("walmart",      "Walmart",       "https://www.walmart.com/search?q={}"),
        SiteEntry("bestbuy",      "Best Buy",      "https://www.bestbuy.com/site/searchpage.jsp?st={}"),
        SiteEntry("imdb",         "IMDb",          "https://www.imdb.com/find?q={}"),
        SiteEntry("spotify",      "Spotify",       "https://open.spotify.com/search/{}"),
        SiteEntry("netflix",      "Netflix",       "https://www.netflix.com/search?q={}"),
        SiteEntry("linkedin",     "LinkedIn",      "https://www.linkedin.com/search/results/all/?keywords={}"),
        SiteEntry("duckduckgo",   "DuckDuckGo",    "https://duckduckgo.com/?q={}"),
        SiteEntry("bing",         "Bing",          "https://www.bing.com/search?q={}"),
        SiteEntry("npm",          "npm",           "https://www.npmjs.com/search?q={}"),
        SiteEntry("crates",       "crates.io",     "https://crates.io/search?q={}"),
        SiteEntry("docs",         "docs.rs",       "https://docs.rs/releases/search?query={}"),
        SiteEntry("maps",         "Google Maps",   "https://www.google.com/maps/search/{}"),
        SiteEntry("news",         "Google News",   "https://news.google.com/search?q={}"),
    ]
}

/// Check if text mentions a known site.
pub fn detect_site(text: &str) -> Option<(&'static str, &'static str)> {
    let lower = text.to_lowercase();
    for SiteEntry(_keyword, name, url_template) in known_sites() {
        if lower.contains(name.to_lowercase().as_str()) || lower.contains(_keyword) {
            return Some((name, url_template));
        }
    }
    None
}

/// Extract a search query from text by removing the site name and action words.
/// "search for headphones on amazon" → "headphones"
/// "search youtube for rust tutorials" → "rust tutorials"
pub fn extract_search_query(text: &str) -> Option<String> {
    let text = text.trim();
    let lower = text.to_lowercase();

    // Detect site keyword first, so we can strip it
    let site_kw = known_sites().iter()
        .find(|SiteEntry(kw, _, _)| lower.contains(kw))
        .map(|SiteEntry(kw, _, _)| *kw);

    // Remove the site keyword from text to find the query
    let cleaned = if let Some(site) = site_kw {
        // Remove site keyword and the word before it if it's "on"/"in"/"at"/"for"
        let site_lower = site.to_lowercase();
        // Try patterns like "on amazon", "for youtube", "in github"
        let mut cleaned = text.to_string();
        for prefix in &[" on ", " in ", " at ", " for ", " via ", " using "] {
            let pattern = format!("{}{}", prefix, site_lower);
            if let Some(pos) = cleaned.to_lowercase().find(&pattern) {
                cleaned = cleaned[..pos].to_string() + &cleaned[pos + pattern.len()..];
                break;
            }
        }
        // Also try bare site keyword at start of after_action
        if cleaned.to_lowercase().contains(&site_lower) {
            cleaned = cleaned.to_lowercase().replace(&site_lower, "");
        }
        cleaned
    } else {
        text.to_string()
    };

    // Now strip action words from the front
    let query_starters = ["search for ", "search ", "look up ", "find ", "show me "];
    let mut result = cleaned.trim().to_string();

    for starter in &query_starters {
        if result.to_lowercase().starts_with(starter) {
            result = result[starter.len()..].trim().to_string();
            break;
        }
    }

    // Strip any remaining "for" at the start
    result = result.strip_prefix("for ").or_else(|| result.strip_prefix("for"))
        .unwrap_or(&result).trim().to_string();

    if !result.is_empty() && result.len() > 1 {
        return Some(result);
    }

    // Last resort: original verbs approach
    let verbs = ["search", "find", "look up", "show", "open"];
    for verb in &verbs {
        if let Some(pos) = lower.find(verb) {
            let rest = text[pos + verb.len()..].trim();
            if !rest.is_empty() && rest.len() > 2 {
                let rest = rest.strip_prefix("for ").or_else(|| rest.strip_prefix("up "))
                    .or_else(|| rest.strip_prefix("me ")).unwrap_or(rest);
                // If there's a site indicator, take the part before it
                for indicator in &[" on ", " in ", " at ", " using "] {
                    if let Some(p) = rest.find(indicator) {
                        return Some(rest[..p].trim().to_string());
                    }
                }
                return Some(rest.to_string());
            }
        }
    }

    None
}

/// Extract the site name from text.
/// "search for headphones on amazon" → Some(("amazon", "Amazon", url_template))
/// "youtube rust tutorials" → Some(("youtube", "YouTube", url_template))
pub fn extract_site(text: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let lower = text.to_lowercase();
    // Check after "on", "at", "in", "using"
    for indicator in &[" on ", " in ", " at ", " via ", " using ", " with "] {
        if let Some(pos) = lower.find(indicator) {
            let after = &lower[pos + indicator.len()..];
            let site_word = after.split_whitespace().next().unwrap_or("");
            for SiteEntry(kw, name, url_template) in known_sites() {
                if kw == site_word {
                    return Some((kw, name, url_template));
                }
            }
        }
    }
    // Site might be the first word or standalone
    for SiteEntry(keyword, name, url_template) in known_sites() {
        if lower.contains(keyword) {
            return Some((keyword, name, url_template));
        }
    }
    None
}

/// Perform a search on a known site by constructing the search URL.
/// Returns the URL and a user-friendly message.
pub fn search_on_site(site_name: &str, query: &str) -> (String, String) {
    let lower_site = site_name.to_lowercase();

    for SiteEntry(_kw, display_name, url_template) in known_sites() {
        if _kw == lower_site.as_str() || display_name.to_lowercase() == lower_site {
            let encoded = urlencoding::encode(query).replace("%20", "+");
            let url = url_template.replace("{}", &encoded);
            let msg = format!("Searching {} for {}", display_name, query);
            return (url, msg);
        }
    }

    // Unknown site — do a general Google search
    let combined = format!("{} {}", query, site_name);
    let encoded = urlencoding::encode(&combined).replace("%20", "+");
    let url = format!("https://www.google.com/search?q={}", encoded);
    let msg = format!("Searching for {} on {}", query, site_name);
    (url, msg)
}

/// Check if a command is a "search X on Y" pattern.
pub fn is_site_search_command(text: &str) -> bool {
    let lower = text.to_lowercase();
    let has_action = lower.contains("search") || lower.contains("look up")
        || lower.contains("find") || lower.contains("show me");
    let has_site = detect_site(text).is_some();
    has_action && has_site
}

/// Handle a "search X on Y" voice command.
/// Opens the search URL in the default browser.
pub async fn handle_site_search_command(text: &str) -> Option<String> {
    let query = extract_search_query(text)?;
    let (_keyword, display_name, url_template) = extract_site(text)?;

    let encoded = urlencoding::encode(&query).replace("%20", "+");
    let url = url_template.replace("{}", &encoded);

    let msg = format!("Searching {} for {}", display_name, query);
    let _ = speak(&msg);

    open_search_url(&url);

    Some(msg)
}

/// Open a URL in the browser (cross-platform).
fn open_search_url(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("cmd").args(["/C", "start", "", url]).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("xdg-open").arg(url).spawn();
    }
}

/// ── Browser UI Automation ────────────────────────────────────────────────
/// These functions control the browser's UI (type into search bar, click, etc.)
/// using platform-specific automation (AppleScript on macOS, xdotool on Linux).
/// On Windows, these fall back to URL-based approaches.

/// Type text into the currently focused browser tab's URL/search bar.
/// On macOS: activates the frontmost browser and types into the URL bar.
/// On Linux: uses xdotool to type.
pub fn type_into_browser(text: &str) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"tell application "System Events"
                set frontApp to name of first process whose frontmost is true
            end tell
            tell application frontApp
                activate
            end tell
            delay 0.1
            tell application "System Events"
                keystroke "{}"
            end tell
            return "Typed into " & frontApp"#,
            text.replace("\"", "\\\"")
        );
        match run_applescript(&script) {
            Ok(msg) => Ok(format!("Typed '{}' into browser", text)),
            Err(e) => {
                // Fallback: just use URL approach
                let encoded = urlencoding::encode(text).replace("%20", "+");
                let url = format!("https://www.google.com/search?q={}", encoded);
                open_search_url(&url);
                Ok(format!("Searched for '{}' via URL fallback", text))
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        match Command::new("xdotool").args(["type", "--delay", "50", text]).output() {
            Ok(_) => Ok(format!("Typed '{}' into browser", text)),
            Err(_) => {
                let encoded = urlencoding::encode(text).replace("%20", "+");
                let url = format!("https://www.google.com/search?q={}", encoded);
                open_search_url(&url);
                Ok(format!("Searched for '{}' via URL fallback", text))
            }
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let encoded = urlencoding::encode(text).replace("%20", "+");
        let url = format!("https://www.google.com/search?q={}", encoded);
        open_search_url(&url);
        Ok(format!("Searched for '{}' via URL (UI automation not supported on this platform)", text))
    }
}

/// Focus the browser's URL/search bar (Cmd+L / Ctrl+L).
pub fn focus_browser_url_bar() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let script = r#"tell application "System Events"
            set frontApp to name of first process whose frontmost is true
        end tell
        tell application frontApp
            activate
        end tell
        delay 0.1
        tell application "System Events"
            keystroke "l" using command down
        end tell
        return "Focused URL bar""#;
        match run_applescript(script) {
            Ok(msg) => Ok("Focused browser URL bar".to_string()),
            Err(e) => Err(format!("Failed to focus URL bar: {}", e)),
        }
    }
    #[cfg(target_os = "linux")]
    {
        match Command::new("xdotool").args(["key", "ctrl+l"]).output() {
            Ok(_) => Ok("Focused browser URL bar".to_string()),
            Err(e) => Err(format!("Failed to focus URL bar: {}", e)),
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err("URL bar focus not supported on this platform".to_string())
    }
}

/// Press Enter/Return in the browser.
pub fn press_enter_in_browser() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let script = r#"tell application "System Events"
            keystroke return
        end tell
        return "Pressed enter""#;
        run_applescript(script).map(|_| "Pressed Enter in browser".to_string())
            .or_else(|e| Err(format!("Failed: {}", e)))
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdotool").arg("key").arg("Return").output()
            .map(|_| "Pressed Enter in browser".to_string())
            .map_err(|e| format!("Failed: {}", e))
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err("Enter key not supported on this platform".to_string())
    }
}

/// General "search the web" via browser — opens Google with the query.
pub fn search_via_browser(query: &str) -> String {
    let encoded = urlencoding::encode(query).replace("%20", "+");
    let url = format!("https://www.google.com/search?q={}", encoded);
    open_search_url(&url);
    format!("Searching for '{}'", query)
}

/// ── Tab & File Management ──────────────────────────────────────────────────
/// These use keyboard shortcuts (Cmd+W / Ctrl+W etc.) to control tabs in the
/// currently focused application (browser, editor, file manager, etc.).

/// Close the currently focused tab (Cmd+W / Ctrl+W).
/// Works in browsers, editors (VS Code, Sublime), file managers, terminals, etc.
pub fn close_current_tab() -> String {
    send_keystroke(&["command down", "w"]) // Cmd+W on macOS, Ctrl+W on others
}

/// Switch to the previous tab (Cmd+Shift+Tab / Ctrl+Shift+Tab).
pub fn switch_previous_tab() -> String {
    send_keystroke(&["command down", "shift down", "tab"]) // Cmd+Shift+Tab
}

/// Switch to the previous window of the same app (Cmd+` / Alt+Tab).
pub fn switch_previous_window() -> String {
    send_keystroke(&["command down", "`"]) // Cmd+`
}

/// Send a keystroke to the focused application using AppleScript (macOS) or xdotool (Linux).
/// Falls back gracefully on unsupported platforms.
fn send_keystroke(keys: &[&str]) -> String {
    #[cfg(target_os = "macos")]
    {
        let modifiers: Vec<&str> = keys.iter()
            .filter(|k| k.contains("down") || k.contains("up"))
            .map(|k| *k)
            .collect();
        let key = keys.iter()
            .find(|k| !k.contains("down") && !k.contains("up") && !k.contains("shift"))
            .unwrap_or(&keys[keys.len() - 1]);
        let using = if modifiers.is_empty() {
            String::new()
        } else {
            format!(" using {}", modifiers.join(" and "))
        };
        let script = format!(
            r#"tell application "System Events" to keystroke "{}"{}"#,
            key, using
        );
        match run_applescript(&script) {
            Ok(_) => format!("Sent keystroke."),
            Err(e) => format!("Failed to send keystroke: {}", e),
        }
    }
    #[cfg(target_os = "linux")]
    {
        let combo = keys.iter()
            .map(|k| match *k {
                "command down" => "ctrl",
                "shift down" => "shift",
                "option down" => "alt",
                "control down" => "ctrl",
                "tab" => "Tab",
                "w" => "w",
                "`" => "grave",
                _ => k,
            })
            .filter(|k| !k.contains("up") && *k != "shift")
            .collect::<Vec<&str>>()
            .join("+");
        match std::process::Command::new("xdotool")
            .args(["key", &combo])
            .output()
        {
            Ok(_) => format!("Sent keystroke."),
            Err(e) => format!("Failed to send keystroke: {}", e),
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        format!("Keystroke not supported on this platform.")
    }
}

/// Run an AppleScript and return its stdout.
#[cfg(target_os = "macos")]
fn run_applescript(script: &str) -> Result<String, String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| format!("osascript error: {}", e))?;
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("AppleScript error: {}", stderr))
    }
}

/// Try to auto-detect the browser being used and return its name.
pub fn detect_active_browser() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let script = r#"tell application "System Events"
            set frontApp to name of first process whose frontmost is true
            return frontApp
        end tell"#;
        run_applescript(script).ok()
    }
    #[cfg(target_os = "linux")]
    {
        Command::new("xdotool")
            .args(["getactivewindow", "getwindowpid"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|pid| {
                // Read /proc/{pid}/comm to get process name
                let path = format!("/proc/{}/comm", pid.trim());
                std::fs::read_to_string(&path).ok()
            })
            .map(|s| s.trim().to_string())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_site() {
        assert!(detect_site("search for headphones on amazon").is_some());
        assert!(detect_site("search youtube for rust").is_some());
        assert!(detect_site("look up python on wikipedia").is_some());
        assert!(detect_site("find rust on github").is_some());
        assert!(detect_site("what is the weather").is_none());
    }

    #[test]
    fn test_extract_search_query() {
        assert_eq!(
            extract_search_query("search for headphones on amazon").as_deref(),
            Some("headphones")
        );
        assert_eq!(
            extract_search_query("search youtube for rust tutorials").as_deref(),
            Some("rust tutorials")
        );
        assert_eq!(
            extract_search_query("search amazon for laptop").as_deref(),
            Some("laptop")
        );
    }

    #[test]
    fn test_extract_site() {
        let result = extract_site("search for headphones on amazon");
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "amazon");

        let result = extract_site("search youtube for rust tutorials");
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, "youtube");
    }

    #[test]
    fn test_search_on_site() {
        let (url, _) = search_on_site("amazon", "headphones");
        assert!(url.contains("amazon.com"));
        assert!(url.contains("headphones"));

        let (url, _) = search_on_site("youtube", "rust tutorial");
        assert!(url.contains("youtube.com"));
        assert!(url.contains("rust+tutorial"));
    }

    #[test]
    fn test_is_site_search() {
        assert!(is_site_search_command("search for headphones on amazon"));
        assert!(is_site_search_command("search youtube for rust"));
        assert!(!is_site_search_command("open chrome"));
        assert!(!is_site_search_command("what time is it"));
    }

    #[test]
    fn test_known_sites_contains_key() {
        let sites = known_sites();
        let keywords: Vec<&str> = sites.iter().map(|SiteEntry(k, _, _)| *k).collect();
        assert!(keywords.contains(&"amazon"));
        assert!(keywords.contains(&"youtube"));
        assert!(keywords.contains(&"github"));
        assert!(keywords.contains(&"google"));
    }
}
