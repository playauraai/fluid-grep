/*!
Caching layer for ripgrep to achieve 2-3ms search latency.

This module provides:
1. Directory caching - avoid re-scanning filesystem
2. Bounded search - stop after N matches
3. Result caching - reuse previous search results

Optimized for:
- fzf-style incremental search (<2ms)
- ripgrep-style directory caching
- VS Code IDE performance
*/

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Cache for directory entries to avoid re-scanning.
/// Uses Arc<Vec<PathBuf>> to avoid cloning large entry lists.
#[derive(Clone, Debug)]
pub(crate) struct DirectoryCache {
    entries: HashMap<PathBuf, (Arc<Vec<PathBuf>>, Instant)>,
    ttl: Duration,
}

impl DirectoryCache {
    /// Create a new directory cache with 5-second TTL.
    pub(crate) fn new() -> Self {
        DirectoryCache {
            entries: HashMap::new(),
            ttl: Duration::from_secs(5),
        }
    }

    /// Canonicalize path to avoid cache misses from different representations.
    fn canonicalize_key(dir: &Path) -> PathBuf {
        dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf())
    }

    /// Get cached entries or return None if expired.
    /// Returns Arc to avoid cloning large entry lists.
    pub(crate) fn get(&self, dir: &Path) -> Option<Arc<Vec<PathBuf>>> {
        let key = Self::canonicalize_key(dir);
        if let Some((entries, cached_at)) = self.entries.get(&key) {
            if cached_at.elapsed() < self.ttl {
                return Some(Arc::clone(entries));
            }
        }
        None
    }

    /// Insert entries into cache with canonicalized key.
    pub(crate) fn insert(&mut self, dir: PathBuf, entries: Vec<PathBuf>) {
        let key = Self::canonicalize_key(&dir);
        self.entries.insert(key, (Arc::new(entries), Instant::now()));
    }

    /// Clear expired entries.
    pub(crate) fn cleanup(&mut self) {
        self.entries.retain(|_, (_, cached_at)| cached_at.elapsed() < self.ttl);
    }
}

impl Default for DirectoryCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a cached search result with ranking and metadata.
/// Optimized for Fluid search mode with heuristic scoring.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CachedResult {
    /// Relevance score (0-1000)
    pub score: u32,
    /// File path
    pub path: PathBuf,
    /// Matched line content
    pub line: String,
    /// Line number in file
    pub line_number: usize,
}

/// Cache for search results to enable incremental search.
/// Supports both forward (typing) and backward (deletion) pattern matching.
#[derive(Clone, Debug)]
pub(crate) struct SearchResultCache {
    last_pattern: String,
    last_results: Vec<CachedResult>,
    cached_at: Instant,
    ttl: Duration,
}

impl SearchResultCache {
    /// Create a new search result cache.
    pub(crate) fn new() -> Self {
        SearchResultCache {
            last_pattern: String::new(),
            last_results: Vec::new(),
            cached_at: Instant::now(),
            ttl: Duration::from_secs(10),
        }
    }

    /// Check if cached results are still valid for a pattern.
    /// Supports both forward (typing) and backward (deletion) pattern matching.
    /// Forward: "f" → "fn" → "fnu" (reuse cache)
    /// Backward: "fnu" → "fn" → "f" (reuse cache)
    pub(crate) fn is_valid(&self, pattern: &str) -> bool {
        if self.cached_at.elapsed() >= self.ttl {
            return false;
        }
        // Forward match: new pattern extends last pattern
        pattern.starts_with(&self.last_pattern)
            // Backward match: last pattern extends new pattern (deletion)
            || self.last_pattern.starts_with(pattern)
    }

    /// Get cached results if valid.
    pub(crate) fn get(&self, pattern: &str) -> Option<Vec<CachedResult>> {
        if self.is_valid(pattern) {
            Some(self.last_results.clone())
        } else {
            None
        }
    }

    /// Update cache with new results.
    pub(crate) fn update(&mut self, pattern: String, results: Vec<CachedResult>) {
        self.last_pattern = pattern;
        self.last_results = results;
        self.cached_at = Instant::now();
    }

    /// Clear cache.
    pub(crate) fn clear(&mut self) {
        self.last_pattern.clear();
        self.last_results.clear();
    }
}

impl Default for SearchResultCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Bounded search configuration for fast IDE-like responses.
/// Optimized for VS Code and IDE performance.
#[derive(Clone, Debug)]
pub(crate) struct BoundedSearch {
    /// Maximum number of results to return
    pub max_results: usize,
    /// Minimum results before early stop (for better UX)
    pub min_results: usize,
    /// Whether to stop early when limit is reached
    pub stop_early: bool,
}

impl BoundedSearch {
    /// Create bounded search with custom limits.
    pub(crate) fn new(max_results: usize) -> Self {
        BoundedSearch {
            max_results,
            min_results: 10,
            stop_early: true,
        }
    }

    /// IDE-friendly defaults (50 results, min 10).
    pub(crate) fn ide_defaults() -> Self {
        BoundedSearch {
            max_results: 50,
            min_results: 10,
            stop_early: true,
        }
    }

    /// Unlimited results (original behavior).
    pub(crate) fn unlimited() -> Self {
        BoundedSearch {
            max_results: usize::MAX,
            min_results: 0,
            stop_early: false,
        }
    }

    /// Check if we should stop searching.
    /// Stops after max_results only if we have at least min_results.
    pub(crate) fn should_stop(&self, current_count: usize, files_matched: usize) -> bool {
        self.stop_early
            && current_count >= self.max_results
            && files_matched >= self.min_results
    }
}

impl Default for BoundedSearch {
    fn default() -> Self {
        Self::ide_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directory_cache() {
        let mut cache = DirectoryCache::new();
        let dir = PathBuf::from("/test");
        let entries = vec![PathBuf::from("/test/file1"), PathBuf::from("/test/file2")];

        cache.insert(dir.clone(), entries.clone());
        let cached = cache.get(&dir);
        assert!(cached.is_some());
        assert_eq!(*cached.unwrap(), entries);
    }

    #[test]
    fn test_directory_cache_expiry() {
        let mut cache = DirectoryCache::new();
        cache.ttl = Duration::from_millis(10);

        let dir = PathBuf::from("/test");
        let entries = vec![PathBuf::from("/test/file1")];

        cache.insert(dir.clone(), entries);
        std::thread::sleep(Duration::from_millis(20));

        assert_eq!(cache.get(&dir), None);
    }

    #[test]
    fn test_search_result_cache() {
        let mut cache = SearchResultCache::new();
        let results = vec![
            CachedResult {
                score: 900,
                path: PathBuf::from("/test/file1"),
                line: "result1".to_string(),
                line_number: 1,
            },
            CachedResult {
                score: 800,
                path: PathBuf::from("/test/file2"),
                line: "result2".to_string(),
                line_number: 2,
            },
        ];

        cache.update("test".to_string(), results.clone());
        assert!(cache.is_valid("test"));
        assert_eq!(cache.get("test"), Some(results));
    }

    #[test]
    fn test_search_result_cache_prefix() {
        let mut cache = SearchResultCache::new();
        let results = vec![CachedResult {
            score: 900,
            path: PathBuf::from("/test/file1"),
            line: "result1".to_string(),
            line_number: 1,
        }];

        cache.update("test".to_string(), results);
        assert!(cache.is_valid("test"));
        assert!(cache.is_valid("testing")); // forward match
        assert!(cache.is_valid("tes")); // backward match (deletion)
        assert!(!cache.is_valid("other")); // no match
    }

    #[test]
    fn test_bounded_search() {
        let bounded = BoundedSearch::ide_defaults();
        assert_eq!(bounded.max_results, 50);
        assert_eq!(bounded.min_results, 10);
        assert!(bounded.stop_early);
        assert!(!bounded.should_stop(49, 10)); // not enough results
        assert!(bounded.should_stop(50, 10)); // enough results
        assert!(!bounded.should_stop(50, 5)); // not enough files matched
    }

    #[test]
    fn test_bounded_search_unlimited() {
        let bounded = BoundedSearch::unlimited();
        assert_eq!(bounded.max_results, usize::MAX);
        assert!(!bounded.stop_early);
        assert!(!bounded.should_stop(1000, 0));
    }
}
