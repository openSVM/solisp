//! Parallel executor for Solisp operations
//!
//! Uses Rayon for work-stealing parallelism with configurable limits.

use crate::error::{Error, Result};
use crate::runtime::Value;
use rayon::prelude::*;
use std::sync::Arc;
use std::time::Duration;

/// Configuration for parallel execution
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Maximum number of parallel tasks (default: num_cpus)
    pub max_parallelism: usize,
    /// Timeout per individual task (default: 30s)
    pub task_timeout: Duration,
    /// Fail fast on first error vs collect all results
    pub fail_fast: bool,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            max_parallelism: num_cpus::get(),
            task_timeout: Duration::from_secs(30),
            fail_fast: false,
        }
    }
}

/// Parallel map operation over an array of values
///
/// # Arguments
/// * `items` - Array of values to process
/// * `mapper` - Function to apply to each item (must be thread-safe)
/// * `config` - Parallel execution configuration
///
/// # Returns
/// * `Ok(Vec<Value>)` - Successful results (may include nulls if errors occurred with fail_fast=false)
/// * `Err(Error)` - First error encountered (if fail_fast=true)
///
/// # Performance
/// - Sequential: N items × T seconds = N×T total time
/// - Parallel: N items × T seconds / num_cpus ≈ (N×T)/cores total time
///
/// # Example
/// ```ignore
/// // Process 10 tokens in parallel (20s → 2s with 10 cores)
/// let results = parallel_map(
///     tokens,
///     |token| get_token_info(token),
///     ParallelConfig::default()
/// )?;
/// ```
pub fn parallel_map<F>(items: Vec<Value>, mapper: F, config: ParallelConfig) -> Result<Vec<Value>>
where
    F: Fn(&Value) -> Result<Value> + Send + Sync,
{
    // Empty array fast path
    if items.is_empty() {
        return Ok(Vec::new());
    }

    // Single item - no parallelism needed
    if items.len() == 1 {
        let result = mapper(&items[0])?;
        return Ok(vec![result]);
    }

    // Wrap mapper in Arc for sharing across threads
    let mapper = Arc::new(mapper);

    // Configure Rayon thread pool
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.max_parallelism.min(items.len()))
        .build()
        .map_err(|e| Error::RuntimeError(format!("Failed to create thread pool: {}", e)))?;

    // Execute parallel map
    pool.install(|| {
        if config.fail_fast {
            // Fail on first error
            items
                .par_iter()
                .map(|item| mapper(item))
                .collect::<Result<Vec<Value>>>()
        } else {
            // Collect all results, converting errors to null
            let results: Vec<Value> = items
                .par_iter()
                .map(|item| mapper(item).unwrap_or(Value::Null))
                .collect();
            Ok(results)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_map_basic() {
        let items = vec![Value::Int(1), Value::Int(2), Value::Int(3)];

        let results = parallel_map(
            items,
            |v| {
                if let Value::Int(n) = v {
                    Ok(Value::Int(n * 2))
                } else {
                    Err(Error::TypeError {
                        expected: "integer".to_string(),
                        got: format!("{:?}", v),
                    })
                }
            },
            ParallelConfig::default(),
        )
        .unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0], Value::Int(2));
        assert_eq!(results[1], Value::Int(4));
        assert_eq!(results[2], Value::Int(6));
    }

    #[test]
    fn test_parallel_map_empty() {
        let items: Vec<Value> = vec![];
        let results = parallel_map(items, |v| Ok(v.clone()), ParallelConfig::default()).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_parallel_map_error_fail_fast() {
        let items = vec![
            Value::Int(1),
            Value::String("bad".to_string()),
            Value::Int(3),
        ];

        let config = ParallelConfig {
            fail_fast: true,
            ..Default::default()
        };

        let result = parallel_map(
            items,
            |v| {
                if let Value::Int(n) = v {
                    Ok(Value::Int(n * 2))
                } else {
                    Err(Error::TypeError {
                        expected: "integer".to_string(),
                        got: "string".to_string(),
                    })
                }
            },
            config,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_parallel_map_error_collect_all() {
        let items = vec![
            Value::Int(1),
            Value::String("bad".to_string()),
            Value::Int(3),
        ];

        let config = ParallelConfig {
            fail_fast: false,
            ..Default::default()
        };

        let results = parallel_map(
            items,
            |v| {
                if let Value::Int(n) = v {
                    Ok(Value::Int(n * 2))
                } else {
                    Err(Error::TypeError {
                        expected: "integer".to_string(),
                        got: "string".to_string(),
                    })
                }
            },
            config,
        )
        .unwrap();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0], Value::Int(2));
        assert_eq!(results[1], Value::Null); // Error converted to null
        assert_eq!(results[2], Value::Int(6));
    }
}
