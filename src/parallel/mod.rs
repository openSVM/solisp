//! Parallel execution support for OVSM
//!
//! Provides parallel map operations for processing arrays concurrently.

mod executor;

pub use executor::{parallel_map, ParallelConfig};
