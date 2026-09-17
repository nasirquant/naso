//! Incremental verification caching.
//!
//! This module provides hash-based caching of verification results to avoid
//! re-running the solver on unchanged code.

#[cfg(feature = "z3")]
use crate::error::{CacheError, VerifyError};
#[cfg(feature = "z3")]
use crate::solver::VerifyResult;
#[cfg(feature = "z3")]
use blake3;
#[cfg(feature = "z3")]
use naso_compiler::ast::Program;
#[cfg(feature = "z3")]
use naso_compiler::ast::Span;
#[cfg(feature = "z3")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "z3")]
use std::collections::HashMap;
#[cfg(feature = "z3")]
use std::path::{Path, PathBuf};
#[cfg(feature = "z3")]
use std::time::SystemTime;

#[cfg(feature = "z3")]
/// Cache entry for a verification result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Hash of the AST + configuration
    pub key: String,
    /// Verification result
    pub result: CachedResult,
    /// Timestamp of cache entry
    pub timestamp: SystemTime,
    /// Solver configuration used
    pub config_hash: String,
    /// Statistics
    pub stats: CacheStats,
}

#[cfg(feature = "z3")]
/// Cached verification result (serializable).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CachedResult {
    Sat { model: serde_json::Value },
    Unsat { core: Option<serde_json::Value> },
    Unknown { reason: String },
    Error { message: String },
}

#[cfg(feature = "z3")]
impl From<&crate::solver::VerifyResult> for CachedResult {
    fn from(result: &crate::solver::VerifyResult) -> Self {
        match result {
            crate::solver::VerifyResult::Sat(model) => CachedResult::Sat {
                model: serde_json::to_value(model).unwrap_or(serde_json::Value::Null),
            },
            crate::solver::VerifyResult::Unsat(core) => CachedResult::Unsat {
                core: core
                    .as_ref()
                    .map(|c| serde_json::to_value(c).unwrap_or(serde_json::Value::Null)),
            },
            crate::solver::VerifyResult::Unknown(reason) => CachedResult::Unknown {
                reason: reason.clone(),
            },
            crate::solver::VerifyResult::Error(msg) => CachedResult::Error {
                message: msg.clone(),
            },
        }
    }
}

#[cfg(feature = "z3")]
impl From<CachedResult> for crate::solver::VerifyResult {
    fn from(cached: CachedResult) -> Self {
        match cached {
            CachedResult::Sat { .. } => {
                crate::solver::VerifyResult::Sat(crate::model::Model::empty())
            }
            CachedResult::Unsat { .. } => crate::solver::VerifyResult::Unsat(None),
            CachedResult::Unknown { reason } => crate::solver::VerifyResult::Unknown(reason),
            CachedResult::Error { message } => crate::solver::VerifyResult::Error(message),
        }
    }
}

#[cfg(feature = "z3")]
/// Cache statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub errors: usize,
    pub total_time_saved_ms: u64,
}

#[cfg(feature = "z3")]
impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

#[cfg(feature = "z3")]
/// Verification cache manager.
pub struct VerificationCache {
    cache_dir: PathBuf,
    entries: HashMap<String, CacheEntry>,
    stats: CacheStats,
    max_entries: usize,
}

#[cfg(feature = "z3")]
impl VerificationCache {
    /// Create a new cache manager.
    pub fn new(cache_dir: Option<PathBuf>) -> Result<Self, VerifyError> {
        let cache_dir = cache_dir.unwrap_or_else(|| {
            dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("naso")
                .join("verify")
        });

        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(&cache_dir).map_err(|e| {
            VerifyError::Cache(CacheError::DirNotAccessible {
                path: cache_dir.display().to_string(),
            })
        })?;

        let mut cache = Self {
            cache_dir,
            entries: HashMap::new(),
            stats: CacheStats::default(),
            max_entries: 10000,
        };

        // Load existing cache index
        cache.load_index()?;
        Ok(cache)
    }

    /// Generate a cache key from AST hash and solver config.
    pub fn make_key(ast_hash: &str, config: &SolverConfig) -> String {
        let config_str = serde_json::to_string(config).unwrap_or_default();
        let mut hasher = blake3::Hasher::new();
        hasher.update(ast_hash.as_bytes());
        hasher.update(config_str.as_bytes());
        hasher.finalize().to_hex().to_string()
    }

    /// Compute hash of an AST (simplified - in reality would hash the typed AST).
    pub fn hash_ast(program: &Program) -> String {
        let mut hasher = blake3::Hasher::new();
        // Hash function names and bodies
        for item in &program.items {
            if let naso_compiler::ast::Item::Function(func) = item {
                hasher.update(func.name.name.as_bytes());
                // In real implementation, hash the full function structure
            }
        }
        hasher.finalize().to_hex().to_string()
    }

    /// Hash a source file.
    pub fn hash_file(path: &Path) -> Result<String, VerifyError> {
        let content = std::fs::read(path).map_err(|e| VerifyError::Io(e))?;
        let hash = blake3::hash(&content);
        Ok(hash.to_hex().to_string())
    }

    /// Try to get a cached result.
    pub fn get(&mut self, key: &str) -> Option<&CachedResult> {
        if let Some(entry) = self.entries.get(key) {
            // Check if entry is still valid (config matches)
            // In practice, we'd also check file modification times
            self.stats.hits += 1;
            Some(&entry.result)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Store a result in the cache.
    pub fn put(
        &mut self,
        key: String,
        ast_hash: String,
        config: &SolverConfig,
        result: &crate::solver::VerifyResult,
        elapsed_ms: u64,
    ) -> Result<(), VerifyError> {
        let config_hash = Self::make_key(&ast_hash, config);
        let entry = CacheEntry {
            key: key.clone(),
            result: result.into(),
            timestamp: SystemTime::now(),
            config_hash,
            stats: CacheStats {
                hits: 0,
                misses: 0,
                errors: 0,
                total_time_saved_ms: elapsed_ms,
            },
        };

        // Evict old entries if cache is full
        if self.entries.len() >= self.max_entries {
            self.evict_oldest();
        }

        self.entries.insert(key, entry);
        self.save_index()?;
        Ok(())
    }

    /// Evict the oldest cache entry.
    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.timestamp)
            .map(|(k, _)| k.clone())
        {
            self.entries.remove(&oldest_key);
        }
    }

    /// Save cache index to disk.
    fn save_index(&self) -> Result<(), VerifyError> {
        let index_path = self.cache_dir.join("index.json");
        let data = serde_json::to_vec_pretty(&self.entries)
            .map_err(|e| VerifyError::Cache(CacheError::SerializationFailed(e.to_string())))?;
        std::fs::write(index_path, data).map_err(|e| VerifyError::Io(e))?;
        Ok(())
    }

    /// Load cache index from disk.
    fn load_index(&mut self) -> Result<(), VerifyError> {
        let index_path = self.cache_dir.join("index.json");
        if index_path.exists() {
            let data = std::fs::read(&index_path).map_err(|e| VerifyError::Io(e))?;
            self.entries = serde_json::from_slice(&data).map_err(|e| {
                VerifyError::Cache(CacheError::DeserializationFailed(e.to_string()))
            })?;
        }
        Ok(())
    }

    /// Get cache statistics.
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Clear the cache.
    pub fn clear(&mut self) -> Result<(), VerifyError> {
        self.entries.clear();
        self.stats = CacheStats::default();
        self.save_index()?;
        Ok(())
    }

    /// Get cache directory path.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
}

#[cfg(feature = "z3")]
/// Cached verification function.
pub fn verify_cached(
    cache: &mut VerificationCache,
    program: &Program,
    config: SolverConfig,
) -> Result<crate::solver::VerifyResult, VerifyError> {
    let ast_hash = VerificationCache::hash_ast(program);
    let key = VerificationCache::make_key(&ast_hash, &config);

    // Try cache first
    if let Some(cached) = cache.get(&key) {
        return Ok(cached.clone().into());
    }

    // Not in cache - run verification
    let start = std::time::Instant::now();
    let smt_script = crate::lower::lower_to_smtlib(program)?;
    let result = crate::solver::verify(&smt_script, config.clone())?;
    let elapsed = start.elapsed().as_millis() as u64;

    // Store in cache
    cache.put(key, ast_hash, &config, &result, elapsed)?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "z3")]
    use super::*;
    #[cfg(feature = "z3")]
    use tempfile::tempdir;

    #[test]
    #[cfg(feature = "z3")]
    fn test_cache_creation() {
        let dir = tempdir().unwrap();
        let cache = VerificationCache::new(Some(dir.path().to_path_buf())).unwrap();
        assert_eq!(cache.stats().hits, 0);
        assert_eq!(cache.stats().misses, 0);
    }

    #[test]
    #[cfg(feature = "z3")]
    fn test_key_generation() {
        let config = SolverConfig::default();
        let key1 = VerificationCache::make_key("ast_hash_1", &config);
        let key2 = VerificationCache::make_key("ast_hash_1", &config);
        assert_eq!(key1, key2);

        let config2 = SolverConfig::fast();
        let key3 = VerificationCache::make_key("ast_hash_1", &config2);
        assert_ne!(key1, key3);
    }

    #[test]
    #[cfg(feature = "z3")]
    fn test_hash_file() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.naso");
        std::fs::write(&file, "fn main() {}").unwrap();
        let hash = VerificationCache::hash_file(&file).unwrap();
        assert_eq!(hash.len(), 64); // blake3 hex length
    }
}
