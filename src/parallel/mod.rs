//! Parallel execution support for Solisp
//!
//! Provides parallel map operations for processing arrays concurrently.

mod executor;

pub use executor::{parallel_map, ParallelConfig};
