/// Split a user utterance into discrete command segments at natural boundaries.
///
/// Examples:
///   "open chrome and search for cats" → ["open chrome", "search for cats"]
///   "tell me a joke, take a screenshot" → ["tell me a joke", "take a screenshot"]
///   "open chrome" → ["open chrome"]
///   "open chrome and then search for cats" → ["open chrome", "search for cats"]
///
/// Uses longest-match-first strategy so "and then" is matched before "and".
/// Avoids false positives by requiring word-boundary separators.
pub fn split_utterance(input: &str) -> Vec<String> {
    let input = input.trim();
    if input.is_empty() {
        return vec![];
    }

    // Normalize whitespace: collapse multiple spaces and join
    let input: Vec<&str> = input.split_whitespace().collect();
    if input.is_empty() {
        return vec![];
    }
    let input = input.join(" ");

    // Step 1: Split on comma-space and semicolon-space first (most explicit)
    let comma_split: Vec<String> = input
        .split(", ")
        .flat_map(|s| s.split("; ").map(|p| p.trim().to_string()))
        .filter(|s| !s.is_empty())
        .collect();

    // Step 2: Within each comma-separated part, split on conjunctions
    // Use longest separators first to avoid "and then" being split by "and"
    let separators = [" and then ", " , then ", " then ", " and ", " but ", " also "];

    let mut result: Vec<String> = Vec::new();
    for part in &comma_split {
        let sub_segments = split_on_conjunctions(part, &separators);
        for seg in sub_segments {
            let trimmed = seg.trim().to_string();
            if !trimmed.is_empty() && !result.contains(&trimmed) {
                result.push(trimmed);
            }
        }
    }

    // Step 3: Filter out segments that are just filler words
    let fillers = ["and", "then", "but", "also", "or", "so"];
    result
        .into_iter()
        .filter(|s| {
            let lower = s.to_lowercase();
            let words: Vec<&str> = lower.split_whitespace().collect();
            // Keep segments with meaningful content (not just conjunctions)
            !words.is_empty() && words.iter().any(|w| !fillers.contains(w))
        })
        .collect()
}

/// Recursively split a segment on conjunctions, longest match first.
fn split_on_conjunctions(segment: &str, separators: &[&str]) -> Vec<String> {
    if separators.is_empty() {
        return vec![segment.to_string()];
    }

    let sep = separators[0];
    let rest = &separators[1..];

    let parts: Vec<&str> = segment.splitn(2, sep).collect();
    if parts.len() > 1 {
        let left = parts[0].trim();
        let right = parts[1].trim();

        let mut result = Vec::new();
        // Recurse on both sides with ALL separators (the same sep may appear again on right)
        result.extend(split_on_conjunctions(left, separators));
        result.extend(split_on_conjunctions(right, separators));
        result
    } else {
        // Also try matching the conjunction at the start of the segment (no leading space)
        // e.g., "and tell me a joke" should split into "tell me a joke"
        let leading_sep = sep.trim_start(); // "and " instead of " and "
        if let Some(stripped) = segment.strip_prefix(leading_sep) {
            let trimmed = stripped.trim();
            return split_on_conjunctions(trimmed, rest);
        }

        // No match for this separator, try the next one
        split_on_conjunctions(segment, rest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_and() {
        let result = split_utterance("open chrome and search for cats");
        assert_eq!(result, vec!["open chrome", "search for cats"]);
    }

    #[test]
    fn test_and_then() {
        let result = split_utterance("open chrome and then search for cats");
        assert_eq!(result, vec!["open chrome", "search for cats"]);
    }

    #[test]
    fn test_comma_split() {
        let result = split_utterance("tell me a joke, take a screenshot");
        assert_eq!(result, vec!["tell me a joke", "take a screenshot"]);
    }

    #[test]
    fn test_single_command() {
        let result = split_utterance("open chrome");
        assert_eq!(result, vec!["open chrome"]);
    }

    #[test]
    fn test_multiple_actions() {
        let result = split_utterance("open chrome and search for cats and set alarm for 7 am");
        assert_eq!(
            result,
            vec!["open chrome", "search for cats", "set alarm for 7 am"]
        );
    }

    #[test]
    fn test_with_but() {
        let result = split_utterance("open chrome but don't search");
        assert_eq!(result, vec!["open chrome", "don't search"]);
    }

    #[test]
    fn test_complex_sentence() {
        let result = split_utterance("open chrome, search for cats, and tell me a joke");
        assert_eq!(
            result,
            vec!["open chrome", "search for cats", "tell me a joke"]
        );
    }

    #[test]
    fn test_not_splitting_inside_words() {
        let result = split_utterance("I understand");
        assert_eq!(result, vec!["I understand"]);
        let result = split_utterance("open android studio");
        assert_eq!(result, vec!["open android studio"]);
        let result = split_utterance("open command prompt");
        assert_eq!(result, vec!["open command prompt"]);
    }

    #[test]
    fn test_empty_input() {
        let result = split_utterance("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_also() {
        let result = split_utterance("open chrome also search for cats");
        assert_eq!(result, vec!["open chrome", "search for cats"]);
    }

    #[test]
    fn test_three_way() {
        let result = split_utterance("open chrome, search cats, tell a joke");
        assert_eq!(result, vec!["open chrome", "search cats", "tell a joke"]);
    }

    #[test]
    fn test_then_at_start() {
        let result = split_utterance("then open chrome");
        assert_eq!(result, vec!["open chrome"]);
    }

    #[test]
    fn test_verbose_sentence() {
        let result = split_utterance("hey igris can you open chrome and search for cats and then set an alarm for 7 am");
        assert_eq!(
            result,
            vec![
                "hey igris can you open chrome",
                "search for cats",
                "set an alarm for 7 am"
            ]
        );
    }

    #[test]
    fn test_only_conjunctions() {
        let result = split_utterance("and then but also");
        assert!(result.is_empty());
    }

    #[test]
    fn test_semicolon() {
        let result = split_utterance("open chrome; search for cats");
        assert_eq!(result, vec!["open chrome", "search for cats"]);
    }
}
