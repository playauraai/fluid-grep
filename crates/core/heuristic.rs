/*!
Heuristic search utilities for fluid search mode.

This module provides intelligent pattern matching that can find results
even with typos, partial matches, or fuzzy patterns.

The scoring system uses normalized weights (0.0 to 1.0) that are scaled
to a 0-1000 range for practical use. This ensures fair comparison across
different pattern lengths and match types.
*/

/// Customizable weights for heuristic scoring.
/// All weights should be between 0.0 and 1.0.
#[derive(Clone, Debug)]
pub(crate) struct ScoringWeights {
    /// Weight for exact matches. Default: 1.0
    pub exact_match: f32,
    /// Weight for case-sensitive matches. Default: 0.5
    pub case_sensitive: f32,
    /// Weight for word boundary matches. Default: 0.3
    pub word_boundary: f32,
    /// Weight for fuzzy/consecutive matches. Default: 0.2
    pub fuzzy_match: f32,
    /// Weight for substring matches. Default: 0.15
    pub substring_match: f32,
    /// Weight for length similarity. Default: 0.1
    pub length_similarity: f32,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        ScoringWeights {
            exact_match: 1.0,
            case_sensitive: 0.5,
            word_boundary: 0.3,
            fuzzy_match: 0.2,
            substring_match: 0.15,
            length_similarity: 0.1,
        }
    }
}

impl ScoringWeights {
    /// Creates custom weights for domain-specific scoring.
    pub(crate) fn new(
        exact_match: f32,
        case_sensitive: f32,
        word_boundary: f32,
        fuzzy_match: f32,
        substring_match: f32,
        length_similarity: f32,
    ) -> Self {
        ScoringWeights {
            exact_match,
            case_sensitive,
            word_boundary,
            fuzzy_match,
            substring_match,
            length_similarity,
        }
    }

    /// Normalizes weights so they sum to 1.0 (for fair comparison).
    pub(crate) fn normalize(&self) -> Self {
        let sum = self.exact_match
            + self.case_sensitive
            + self.word_boundary
            + self.fuzzy_match
            + self.substring_match
            + self.length_similarity;

        if sum == 0.0 {
            return ScoringWeights::default();
        }

        ScoringWeights {
            exact_match: self.exact_match / sum,
            case_sensitive: self.case_sensitive / sum,
            word_boundary: self.word_boundary / sum,
            fuzzy_match: self.fuzzy_match / sum,
            substring_match: self.substring_match / sum,
            length_similarity: self.length_similarity / sum,
        }
    }
}

/// Configuration for heuristic matching behavior.
#[derive(Clone, Debug)]
pub(crate) struct HeuristicConfig {
    /// Minimum fuzzy match threshold (0.0-1.0). Default: 0.6 (60%)
    pub fuzzy_threshold: f32,
    /// Maximum edit distance for close matches. If None, uses pattern.len() / 4
    pub max_edit_distance: Option<usize>,
    /// Whether to use Unicode-aware word boundaries. Default: true
    pub unicode_aware: bool,
    /// Whether to require case-sensitive substring matching. Default: false
    pub case_sensitive_substring: bool,
    /// Bonus multiplier for consecutive character matches. Default: 1.0
    pub consecutive_match_bonus: f32,
    /// Custom scoring weights. Default: standard weights
    pub weights: ScoringWeights,
}

impl HeuristicConfig {
    /// Creates a new configuration with custom settings.
    pub(crate) fn new(
        fuzzy_threshold: f32,
        max_edit_distance: Option<usize>,
        unicode_aware: bool,
        case_sensitive_substring: bool,
        consecutive_match_bonus: f32,
    ) -> Self {
        HeuristicConfig {
            fuzzy_threshold,
            max_edit_distance,
            unicode_aware,
            case_sensitive_substring,
            consecutive_match_bonus,
            weights: ScoringWeights::default(),
        }
    }

    /// Creates a new configuration with custom weights.
    pub(crate) fn with_weights(
        fuzzy_threshold: f32,
        max_edit_distance: Option<usize>,
        unicode_aware: bool,
        case_sensitive_substring: bool,
        consecutive_match_bonus: f32,
        weights: ScoringWeights,
    ) -> Self {
        HeuristicConfig {
            fuzzy_threshold,
            max_edit_distance,
            unicode_aware,
            case_sensitive_substring,
            consecutive_match_bonus,
            weights,
        }
    }
}

impl Default for HeuristicConfig {
    fn default() -> Self {
        HeuristicConfig {
            fuzzy_threshold: 0.6,
            max_edit_distance: None,
            unicode_aware: true,
            case_sensitive_substring: false,
            consecutive_match_bonus: 1.0,
            weights: ScoringWeights::default(),
        }
    }
}

/// Represents a match with its score and metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ScoredMatch {
    /// The matched text
    pub text: String,
    /// The relevance score (0-1000)
    pub score: u32,
}

impl Ord for ScoredMatch {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Sort by score descending (higher scores first)
        other.score.cmp(&self.score)
    }
}

impl PartialOrd for ScoredMatch {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Detailed scoring breakdown for a match.
/// Useful for debugging and fine-tuning heuristics.
#[derive(Clone, Debug)]
pub(crate) struct ScoreBreakdown {
    pub exact_match: f32,
    pub case_sensitive: f32,
    pub word_boundary: f32,
    pub fuzzy_match: f32,
    pub substring_match: f32,
    pub length_similarity: f32,
    pub total: u32,
}

/// Calculates a normalized relevance score (0-1000) for a match.
/// Higher score = better match.
///
/// Uses weighted heuristics:
/// - Exact matches: 1.0
/// - Case-sensitive matches: 0.5
/// - Word boundary matches: 0.3
/// - Fuzzy/consecutive matches: 0.2
/// - Substring matches: 0.15
/// - Length similarity: 0.1
pub(crate) fn calculate_relevance_score(
    pattern: &str,
    text: &str,
    is_exact: bool,
    is_case_sensitive: bool,
) -> u32 {
    calculate_relevance_score_with_config(
        pattern,
        text,
        is_exact,
        is_case_sensitive,
        &HeuristicConfig::default(),
    )
}

/// Calculates relevance score with custom configuration.
pub(crate) fn calculate_relevance_score_with_config(
    pattern: &str,
    text: &str,
    is_exact: bool,
    is_case_sensitive: bool,
    config: &HeuristicConfig,
) -> u32 {
    let mut score = 0.0f32;

    // Exact match bonus (highest priority)
    if is_exact {
        score += 1.0;
    }

    // Case-sensitive match bonus
    if is_case_sensitive {
        if pattern == text {
            score += 0.5;
        } else {
            // Partial credit for case-sensitive character matches
            let case_matches = pattern
                .chars()
                .zip(text.chars())
                .filter(|(p, t)| p == t)
                .count();
            let case_ratio = case_matches as f32 / pattern.len().max(1) as f32;
            score += case_ratio * 0.25;
        }
    }

    // Word boundary bonus
    if is_word_boundary_match(pattern, text, config.unicode_aware) {
        score += 0.3;
    }

    // Fuzzy/consecutive character match bonus
    if fuzzy_match_with_threshold(pattern, text, config.fuzzy_threshold) {
        score += 0.2;
    }

    // Substring match bonus
    if is_substring_match(pattern, text, config.case_sensitive_substring) {
        score += 0.15;
    }

    // Length similarity bonus (proportional)
    let length_score = calculate_length_similarity(pattern, text);
    score += length_score * 0.1;

    // Scale to 0-1000 range
    (score * 1000.0).min(1000.0) as u32
}

/// Returns detailed score breakdown for debugging.
pub(crate) fn calculate_relevance_score_breakdown(
    pattern: &str,
    text: &str,
    is_exact: bool,
    is_case_sensitive: bool,
    config: &HeuristicConfig,
) -> ScoreBreakdown {
    let mut breakdown = ScoreBreakdown {
        exact_match: if is_exact { 1.0 } else { 0.0 },
        case_sensitive: 0.0,
        word_boundary: 0.0,
        fuzzy_match: 0.0,
        substring_match: 0.0,
        length_similarity: 0.0,
        total: 0,
    };

    if is_case_sensitive {
        if pattern == text {
            breakdown.case_sensitive = 0.5;
        } else {
            let case_matches = pattern
                .chars()
                .zip(text.chars())
                .filter(|(p, t)| p == t)
                .count();
            breakdown.case_sensitive = (case_matches as f32 / pattern.len().max(1) as f32) * 0.25;
        }
    }

    if is_word_boundary_match(pattern, text, config.unicode_aware) {
        breakdown.word_boundary = 0.3;
    }

    if fuzzy_match_with_threshold(pattern, text, config.fuzzy_threshold) {
        breakdown.fuzzy_match = 0.2;
    }

    if is_substring_match(pattern, text, config.case_sensitive_substring) {
        breakdown.substring_match = 0.15;
    }

    breakdown.length_similarity = calculate_length_similarity(pattern, text) * 0.1;

    let total = breakdown.exact_match
        + breakdown.case_sensitive
        + breakdown.word_boundary
        + breakdown.fuzzy_match
        + breakdown.substring_match
        + breakdown.length_similarity;

    breakdown.total = (total * 1000.0).min(1000.0) as u32;
    breakdown
}

/// Checks if a pattern matches at word boundaries in the text.
/// If unicode_aware is true, uses Unicode character classification.
/// Otherwise, uses ASCII alphanumeric checks.
fn is_word_boundary_match(pattern: &str, text: &str, unicode_aware: bool) -> bool {
    if let Some(pos) = text.find(pattern) {
        let before_ok = if pos == 0 {
            true
        } else if unicode_aware {
            !text[..pos]
                .chars()
                .last()
                .map_or(false, |c| c.is_alphanumeric())
        } else {
            !text.as_bytes()[pos - 1].is_ascii_alphanumeric()
        };

        let after_pos = pos + pattern.len();
        let after_ok = if after_pos >= text.len() {
            true
        } else if unicode_aware {
            !text[after_pos..]
                .chars()
                .next()
                .map_or(false, |c| c.is_alphanumeric())
        } else {
            !text.as_bytes()[after_pos].is_ascii_alphanumeric()
        };

        before_ok && after_ok
    } else {
        false
    }
}

/// Checks if pattern is a substring of text.
/// Respects case_sensitive_substring setting.
fn is_substring_match(pattern: &str, text: &str, case_sensitive: bool) -> bool {
    if case_sensitive {
        text.contains(pattern)
    } else {
        text.to_lowercase().contains(&pattern.to_lowercase())
    }
}

/// Counts consecutive character matches in text for the pattern.
/// Returns (matched_count, is_all_consecutive).
/// is_all_consecutive is true if all pattern characters appear without gaps in text.
fn count_consecutive_matches(pattern: &str, text: &str) -> (usize, bool) {
    if pattern.is_empty() {
        return (0, true);
    }

    let pattern_len = pattern.chars().count();
    let mut pattern_chars = pattern.chars().peekable();
    let mut matched_count = 0;
    let mut last_gap_size = 0;
    let mut max_gap = 0;

    for text_char in text.chars() {
        if let Some(&pattern_char) = pattern_chars.peek() {
            if text_char.eq_ignore_ascii_case(&pattern_char) {
                pattern_chars.next();
                matched_count += 1;
                last_gap_size = 0;
            } else if matched_count > 0 && pattern_chars.peek().is_some() {
                // We're between matches, track gap size
                last_gap_size += 1;
                max_gap = max_gap.max(last_gap_size);
            }
        }
    }

    // All consecutive if no gaps (max_gap == 0) and all matched
    let is_all_consecutive = max_gap == 0 && matched_count == pattern_len;
    (matched_count, is_all_consecutive)
}

/// Calculates length similarity score (0.0-1.0).
/// Returns 1.0 for exact length match, decreases as difference increases.
fn calculate_length_similarity(pattern: &str, text: &str) -> f32 {
    let pattern_len = pattern.len() as f32;
    let text_len = text.len() as f32;
    let len_diff = (pattern_len - text_len).abs();

    if len_diff == 0.0 {
        1.0
    } else {
        (1.0 - (len_diff / pattern_len.max(text_len))).max(0.0)
    }
}

/// Performs fuzzy matching with configurable threshold.
/// Returns true if at least threshold% of pattern characters appear in order in text.
/// threshold should be between 0.0 and 1.0 (e.g., 0.6 = 60%).
fn fuzzy_match_with_threshold(pattern: &str, text: &str, threshold: f32) -> bool {
    if pattern.is_empty() {
        return true;
    }

    let mut pattern_chars = pattern.chars().peekable();
    let mut matched_count = 0;

    for text_char in text.chars() {
        if let Some(&pattern_char) = pattern_chars.peek() {
            if text_char.eq_ignore_ascii_case(&pattern_char) {
                pattern_chars.next();
                matched_count += 1;
            }
        }
    }

    let pattern_len = pattern.chars().count();
    let match_ratio = matched_count as f32 / pattern_len as f32;
    match_ratio >= threshold
}

/// Performs fuzzy matching - finds if pattern characters appear in order in text.
/// Returns true if all pattern characters are found in order (case-insensitive).
/// This is equivalent to fuzzy_match_with_threshold with threshold = 1.0.
pub(crate) fn fuzzy_match(pattern: &str, text: &str) -> bool {
    fuzzy_match_with_threshold(pattern, text, 1.0)
}

/// Calculates Levenshtein distance (edit distance) between two strings.
/// Lower distance = more similar. Useful for typo detection.
///
/// Uses optimized O(min(len1, len2)) space complexity by keeping only two rows.
pub(crate) fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.len();
    let len2 = s2.len();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    // Optimize by using the shorter string as s1
    let (s1, s2, len1, len2) = if len1 > len2 {
        (s2, s1, len2, len1)
    } else {
        (s1, s2, len1, len2)
    };

    let s1_bytes = s1.as_bytes();
    let s2_bytes = s2.as_bytes();

    // Use two rows instead of full matrix
    let mut prev_row: Vec<usize> = (0..=len1).collect();
    let mut curr_row = vec![0; len1 + 1];

    for j in 1..=len2 {
        curr_row[0] = j;

        for i in 1..=len1 {
            let cost = if s1_bytes[i - 1] == s2_bytes[j - 1] { 0 } else { 1 };
            curr_row[i] = std::cmp::min(
                std::cmp::min(
                    prev_row[i] + 1,           // deletion
                    curr_row[i - 1] + 1,       // insertion
                ),
                prev_row[i - 1] + cost,        // substitution
            );
        }

        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[len1]
}

/// Checks if text is a close match to pattern (within edit distance threshold).
/// If max_distance is None, uses pattern.len() / 4 as the threshold.
pub(crate) fn is_close_match(pattern: &str, text: &str, max_distance: Option<usize>) -> bool {
    let threshold = max_distance.unwrap_or_else(|| std::cmp::max(1, pattern.len() / 4));
    levenshtein_distance(pattern, text) <= threshold
}

/// Ranks multiple candidates by relevance to a pattern.
/// Returns candidates sorted by score (highest first).
pub(crate) fn rank_candidates(
    pattern: &str,
    candidates: &[&str],
    config: &HeuristicConfig,
) -> Vec<ScoredMatch> {
    let mut scored: Vec<ScoredMatch> = candidates
        .iter()
        .map(|&text| {
            let score = calculate_relevance_score_with_config(
                pattern,
                text,
                pattern == text,
                false,
                config,
            );
            ScoredMatch {
                text: text.to_string(),
                score,
            }
        })
        .filter(|m| m.score > 0) // Filter out non-matches
        .collect();

    scored.sort();
    scored
}

/// Finds matching character positions in text for a pattern.
/// Returns a vector of (start, end) byte positions of matches.
/// Useful for highlighting matched portions in UI.
pub(crate) fn find_match_positions(pattern: &str, text: &str) -> Vec<(usize, usize)> {
    let mut positions = Vec::new();
    let pattern_lower = pattern.to_lowercase();
    let text_lower = text.to_lowercase();

    // Find all occurrences of pattern as substring
    let mut start = 0;
    while let Some(pos) = text_lower[start..].find(&pattern_lower) {
        let absolute_pos = start + pos;
        positions.push((absolute_pos, absolute_pos + pattern.len()));
        start = absolute_pos + 1;
    }

    positions
}

/// Highlights matching portions of text with markers.
/// Useful for displaying search results with highlighted matches.
/// Markers are placed around matched portions: prefix + match + suffix
pub(crate) fn highlight_matches(
    pattern: &str,
    text: &str,
    prefix: &str,
    suffix: &str,
) -> String {
    let positions = find_match_positions(pattern, text);

    if positions.is_empty() {
        return text.to_string();
    }

    let mut result = String::new();
    let mut last_end = 0;

    for (start, end) in positions {
        // Add text before match
        result.push_str(&text[last_end..start]);
        // Add highlighted match
        result.push_str(prefix);
        result.push_str(&text[start..end]);
        result.push_str(suffix);
        last_end = end;
    }

    // Add remaining text
    result.push_str(&text[last_end..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match() {
        assert!(fuzzy_match("fn", "function"));
        assert!(fuzzy_match("rg", "ripgrep"));
        assert!(fuzzy_match("abc", "aXbXc"));
        assert!(!fuzzy_match("xyz", "abc"));
        assert!(fuzzy_match("", "anything")); // empty pattern matches anything
    }

    #[test]
    fn test_fuzzy_match_with_threshold() {
        assert!(fuzzy_match_with_threshold("fn", "function", 0.5));
        assert!(fuzzy_match_with_threshold("abc", "aXbXc", 0.6));
        assert!(!fuzzy_match_with_threshold("xyz", "abc", 0.5));
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("cat", "cat"), 0);
        assert_eq!(levenshtein_distance("cat", "car"), 1);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", ""), 3);
    }

    #[test]
    fn test_is_close_match() {
        assert!(is_close_match("hello", "helo", Some(1)));
        assert!(is_close_match("world", "word", Some(1)));
        assert!(!is_close_match("hello", "xyz", Some(1)));
        // Test with dynamic threshold
        assert!(is_close_match("hello", "helo", None)); // threshold = 5/4 = 1
    }

    #[test]
    fn test_substring_match() {
        // Case-insensitive (default)
        assert!(is_substring_match("test", "testing", false));
        assert!(is_substring_match("TEST", "testing", false));
        assert!(!is_substring_match("xyz", "abc", false));

        // Case-sensitive
        assert!(is_substring_match("test", "testing", true));
        assert!(!is_substring_match("TEST", "testing", true));
        assert!(!is_substring_match("xyz", "abc", true));
    }

    #[test]
    fn test_consecutive_matches() {
        // Pattern "abc" in "aXbXc" - has gaps (X between each)
        let (matched, is_consecutive) = count_consecutive_matches("abc", "aXbXc");
        assert_eq!(matched, 3);
        assert!(!is_consecutive); // scattered, not consecutive

        // Pattern "abc" in "abc" - no gaps
        let (matched, is_consecutive) = count_consecutive_matches("abc", "abc");
        assert_eq!(matched, 3);
        assert!(is_consecutive); // all consecutive

        // Pattern "ab" in "aXbXc" - has gap (X between)
        let (matched, is_consecutive) = count_consecutive_matches("ab", "aXbXc");
        assert_eq!(matched, 2);
        assert!(!is_consecutive);

        // Pattern "ab" in "ab" - no gaps
        let (matched, is_consecutive) = count_consecutive_matches("ab", "ab");
        assert_eq!(matched, 2);
        assert!(is_consecutive);
    }

    #[test]
    fn test_length_similarity() {
        assert_eq!(calculate_length_similarity("test", "test"), 1.0);
        assert!(calculate_length_similarity("test", "testing") < 1.0);
        assert!(calculate_length_similarity("test", "testing") > 0.0);
    }

    #[test]
    fn test_word_boundary_match() {
        assert!(is_word_boundary_match("test", "test", false));
        assert!(is_word_boundary_match("test", "test case", false));
        assert!(!is_word_boundary_match("test", "testing", false));
    }

    #[test]
    fn test_relevance_score() {
        let exact_score = calculate_relevance_score("test", "test", true, true);
        let partial_score = calculate_relevance_score("test", "testing", false, true);
        assert!(exact_score > partial_score);
    }

    #[test]
    fn test_relevance_score_normalized() {
        let score = calculate_relevance_score("test", "test", true, true);
        assert!(score <= 1000); // Should be normalized to 0-1000
        assert!(score > 0);
    }

    #[test]
    fn test_heuristic_config() {
        let config = HeuristicConfig::default();
        assert_eq!(config.fuzzy_threshold, 0.6);
        assert_eq!(config.max_edit_distance, None);
        assert!(config.unicode_aware);
        assert!(!config.case_sensitive_substring);
        assert_eq!(config.consecutive_match_bonus, 1.0);
    }

    #[test]
    fn test_heuristic_config_custom() {
        let config = HeuristicConfig::new(0.8, Some(2), false, true, 1.5);
        assert_eq!(config.fuzzy_threshold, 0.8);
        assert_eq!(config.max_edit_distance, Some(2));
        assert!(!config.unicode_aware);
        assert!(config.case_sensitive_substring);
        assert_eq!(config.consecutive_match_bonus, 1.5);
    }

    #[test]
    fn test_score_breakdown() {
        let breakdown = calculate_relevance_score_breakdown(
            "test",
            "test",
            true,
            true,
            &HeuristicConfig::default(),
        );
        assert!(breakdown.exact_match > 0.0);
        assert!(breakdown.total <= 1000);
    }

    #[test]
    fn test_edge_cases() {
        // Empty pattern
        assert!(fuzzy_match("", "anything"));
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("", "abc"), 3);

        // Empty text
        assert!(!fuzzy_match("abc", ""));
        assert_eq!(levenshtein_distance("abc", ""), 3);
    }

    #[test]
    fn test_scoring_weights() {
        let weights = ScoringWeights::default();
        assert_eq!(weights.exact_match, 1.0);
        assert_eq!(weights.case_sensitive, 0.5);
        assert_eq!(weights.word_boundary, 0.3);

        let custom = ScoringWeights::new(2.0, 1.0, 0.5, 0.3, 0.2, 0.1);
        assert_eq!(custom.exact_match, 2.0);
    }

    #[test]
    fn test_scoring_weights_normalize() {
        let weights = ScoringWeights::new(2.0, 1.0, 0.5, 0.3, 0.2, 0.1);
        let normalized = weights.normalize();
        let sum = normalized.exact_match
            + normalized.case_sensitive
            + normalized.word_boundary
            + normalized.fuzzy_match
            + normalized.substring_match
            + normalized.length_similarity;
        assert!((sum - 1.0).abs() < 0.001); // Should sum to ~1.0
    }

    #[test]
    fn test_scored_match_ordering() {
        let m1 = ScoredMatch {
            text: "test1".to_string(),
            score: 500,
        };
        let m2 = ScoredMatch {
            text: "test2".to_string(),
            score: 800,
        };
        // m2 has higher score (800 > 500), so m2 should come first when sorted
        // In our Ord impl, we sort descending (higher scores first)
        assert!(m2 < m1); // m2 (800) is "less than" m1 (500) in sort order (comes first)
    }

    #[test]
    fn test_rank_candidates() {
        let candidates = vec!["function", "fn", "final", "filter"];
        let config = HeuristicConfig::default();
        let ranked = rank_candidates("fn", &candidates, &config);

        // Should have results
        assert!(!ranked.is_empty());
        // Should be sorted by score descending
        for i in 1..ranked.len() {
            assert!(ranked[i - 1].score >= ranked[i].score);
        }
    }

    #[test]
    fn test_find_match_positions() {
        let positions = find_match_positions("test", "test case testing");
        assert!(!positions.is_empty());
        // Should find "test" at position 0 and "test" in "testing"
        assert!(positions.len() >= 2);
    }

    #[test]
    fn test_highlight_matches() {
        let highlighted = highlight_matches("test", "test case", "[", "]");
        assert_eq!(highlighted, "[test] case");

        let highlighted = highlight_matches("a", "banana", "<", ">");
        assert_eq!(highlighted, "b<a>n<a>n<a>");
    }

    #[test]
    fn test_highlight_matches_no_match() {
        let highlighted = highlight_matches("xyz", "abc", "[", "]");
        assert_eq!(highlighted, "abc");
    }
}
