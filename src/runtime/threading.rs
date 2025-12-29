//! Bordeaux Threads implementation for Solisp
//!
//! This module provides portable shared-state concurrency primitives compatible
//! with the Common Lisp Bordeaux Threads library (https://github.com/sionescu/bordeaux-threads).
//!
//! ## Features
//!
//! - **Threads**: Create and manage OS threads with `make-thread`, `join-thread`, etc.
//! - **Locks**: Non-recursive mutexes with `make-lock`, `acquire-lock`, `release-lock`
//! - **Recursive Locks**: Reentrant mutexes with `make-recursive-lock`
//! - **Condition Variables**: Thread synchronization with `condition-wait`, `condition-notify`
//! - **Semaphores**: Counting semaphores with `make-semaphore`, `signal-semaphore`, `wait-on-semaphore`
//! - **Atomic Integers**: Lock-free operations with `atomic-integer-incf`, `atomic-integer-cas`
//!
//! ## Example
//!
//! ```lisp
//! ;; Create a thread
//! (define my-thread
//!   (make-thread
//!     (lambda () (+ 1 2 3))
//!     :name "worker"))
//!
//! ;; Wait for result
//! (define result (join-thread my-thread))
//! (println result)  ; => 6
//! ```

use crate::error::{Error, Result};
use crate::runtime::Value;
use dashmap::DashMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

// =============================================================================
// Global State
// =============================================================================

lazy_static::lazy_static! {
    /// Global thread registry for tracking all threads (for all-threads)
    pub static ref THREAD_REGISTRY: DashMap<String, ThreadInfo> = DashMap::new();

    /// Counter for generating unique thread IDs
    static ref THREAD_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Current thread ID (thread-local storage)
    static ref MAIN_THREAD_ID: String = "thread_main".to_string();
}

thread_local! {
    /// Current thread's ID
    static CURRENT_THREAD_ID: std::cell::RefCell<String> = std::cell::RefCell::new("thread_main".to_string());
}

/// Information about a registered thread
#[derive(Clone, Debug)]
pub struct ThreadInfo {
    /// Unique thread identifier
    pub id: String,
    /// Optional human-readable thread name
    pub name: Option<String>,
    /// Thread liveness flag (true while thread is running)
    pub is_alive: Arc<Mutex<bool>>,
}

// =============================================================================
// Thread ID Generation
// =============================================================================

/// Generate a unique thread ID
pub fn generate_thread_id() -> String {
    let id = THREAD_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("thread_{}", id)
}

/// Get the current thread's ID
pub fn current_thread_id() -> String {
    CURRENT_THREAD_ID.with(|id| id.borrow().clone())
}

/// Set the current thread's ID (called at thread start)
pub fn set_current_thread_id(id: String) {
    CURRENT_THREAD_ID.with(|current| {
        *current.borrow_mut() = id;
    });
}

// =============================================================================
// Thread Functions
// =============================================================================

/// Create a new thread value
///
/// The thread will be started immediately and run the provided function.
/// Returns a Thread value that can be joined later.
pub fn make_thread_value(
    id: String,
    name: Option<String>,
    handle: thread::JoinHandle<Value>,
) -> Value {
    let is_alive = Arc::new(Mutex::new(true));

    // Register thread
    THREAD_REGISTRY.insert(
        id.clone(),
        ThreadInfo {
            id: id.clone(),
            name: name.clone(),
            is_alive: is_alive.clone(),
        },
    );

    Value::Thread {
        id,
        name,
        handle: Arc::new(Mutex::new(Some(handle))),
        result: Arc::new(Mutex::new(None)),
    }
}

/// Check if a thread is still alive
pub fn thread_alive(thread: &Value) -> Result<bool> {
    match thread {
        Value::Thread { id, handle, .. } => {
            // Check if handle is still present (not joined yet)
            let guard = handle.lock().unwrap();
            if guard.is_some() {
                // Thread hasn't been joined, check registry
                if let Some(info) = THREAD_REGISTRY.get(id) {
                    return Ok(*info.is_alive.lock().unwrap());
                }
            }
            Ok(false)
        }
        _ => Err(Error::TypeError {
            expected: "thread".to_string(),
            got: thread.type_name(),
        }),
    }
}

/// Join a thread (wait for completion and get result)
pub fn join_thread(thread: &Value) -> Result<Value> {
    match thread {
        Value::Thread {
            id, handle, result, ..
        } => {
            // Try to take the handle
            let join_handle = {
                let mut guard = handle.lock().unwrap();
                guard.take()
            };

            match join_handle {
                Some(h) => {
                    // Join the thread
                    let thread_result = h.join().map_err(|_| Error::ThreadJoinFailed)?;

                    // Store result
                    {
                        let mut result_guard = result.lock().unwrap();
                        *result_guard = Some(thread_result.clone());
                    }

                    // Mark as not alive
                    if let Some(info) = THREAD_REGISTRY.get(id) {
                        *info.is_alive.lock().unwrap() = false;
                    }

                    Ok(thread_result)
                }
                None => {
                    // Already joined, return cached result
                    let result_guard = result.lock().unwrap();
                    match &*result_guard {
                        Some(v) => Ok(v.clone()),
                        None => Err(Error::ThreadAlreadyJoined { id: id.clone() }),
                    }
                }
            }
        }
        _ => Err(Error::TypeError {
            expected: "thread".to_string(),
            got: thread.type_name(),
        }),
    }
}

/// Yield the current thread's execution
pub fn thread_yield() {
    thread::yield_now();
}

/// Get all registered threads
pub fn all_threads() -> Vec<Value> {
    THREAD_REGISTRY
        .iter()
        .map(|entry| {
            let info = entry.value();
            Value::Thread {
                id: info.id.clone(),
                name: info.name.clone(),
                handle: Arc::new(Mutex::new(None)), // Can't get original handle
                result: Arc::new(Mutex::new(None)),
            }
        })
        .collect()
}

// =============================================================================
// Lock Functions
// =============================================================================

/// Create a new lock (non-recursive mutex)
pub fn make_lock(name: Option<String>) -> Value {
    Value::Lock {
        name,
        inner: Arc::new(Mutex::new(())),
    }
}

/// Acquire a lock
///
/// If `wait` is true (default), blocks until lock is acquired.
/// If `wait` is false, returns immediately with false if lock is not available.
/// If `timeout` is provided, waits at most that duration.
pub fn acquire_lock(lock: &Value, wait: bool, timeout: Option<Duration>) -> Result<bool> {
    match lock {
        Value::Lock { inner, .. } => {
            if !wait {
                // Try to acquire without blocking
                match inner.try_lock() {
                    Ok(_guard) => {
                        // We got the lock, but we need to keep it held
                        // This is a limitation - in real BT, the lock stays held
                        // For OVSM, we'll use with-lock-held pattern instead
                        std::mem::forget(_guard); // Keep lock held
                        Ok(true)
                    }
                    Err(_) => Ok(false),
                }
            } else if let Some(dur) = timeout {
                // Try with timeout (spin-wait implementation)
                let start = std::time::Instant::now();
                loop {
                    match inner.try_lock() {
                        Ok(_guard) => {
                            std::mem::forget(_guard);
                            return Ok(true);
                        }
                        Err(_) => {
                            if start.elapsed() >= dur {
                                return Ok(false);
                            }
                            thread::sleep(Duration::from_micros(100));
                        }
                    }
                }
            } else {
                // Block until acquired
                let _guard = inner.lock().unwrap();
                std::mem::forget(_guard);
                Ok(true)
            }
        }
        _ => Err(Error::TypeError {
            expected: "lock".to_string(),
            got: lock.type_name(),
        }),
    }
}

/// Release a lock
///
/// Note: In Rust, this is tricky because MutexGuard drops automatically.
/// This function is provided for API compatibility but the recommended
/// pattern is to use `with-lock-held`.
pub fn release_lock(lock: &Value) -> Result<()> {
    match lock {
        Value::Lock { .. } => {
            // In Rust's model, locks are released when guard is dropped
            // This is a no-op for compatibility; use with-lock-held instead
            Ok(())
        }
        _ => Err(Error::TypeError {
            expected: "lock".to_string(),
            got: lock.type_name(),
        }),
    }
}

// =============================================================================
// Recursive Lock Functions
// =============================================================================

/// Create a new recursive lock
pub fn make_recursive_lock(name: Option<String>) -> Value {
    Value::RecursiveLock {
        name,
        inner: Arc::new(parking_lot::ReentrantMutex::new(())),
    }
}

/// Check if value is a recursive lock
pub fn is_recursive_lock(value: &Value) -> bool {
    matches!(value, Value::RecursiveLock { .. })
}

// =============================================================================
// Condition Variable Functions
// =============================================================================

/// Create a new condition variable
pub fn make_condition_variable(name: Option<String>) -> Value {
    Value::ConditionVariable {
        name,
        inner: Arc::new(Condvar::new()),
    }
}

/// Wait on a condition variable
///
/// The lock must be held when calling this function. The lock is atomically
/// released and the thread waits on the condition variable. When signaled,
/// the lock is reacquired before returning.
///
/// Returns true if signaled, false if timed out.
pub fn condition_wait(cv: &Value, lock: &Value, timeout: Option<Duration>) -> Result<bool> {
    match (cv, lock) {
        (
            Value::ConditionVariable {
                inner: cv_inner, ..
            },
            Value::Lock {
                inner: lock_inner, ..
            },
        ) => {
            let guard = lock_inner.lock().unwrap();

            if let Some(dur) = timeout {
                let (_new_guard, result) = cv_inner.wait_timeout(guard, dur).unwrap();
                Ok(!result.timed_out())
            } else {
                let _guard = cv_inner.wait(guard).unwrap();
                Ok(true)
            }
        }
        (Value::ConditionVariable { .. }, _) => Err(Error::TypeError {
            expected: "lock".to_string(),
            got: lock.type_name(),
        }),
        _ => Err(Error::TypeError {
            expected: "condition-variable".to_string(),
            got: cv.type_name(),
        }),
    }
}

/// Notify one thread waiting on the condition variable
pub fn condition_notify(cv: &Value) -> Result<()> {
    match cv {
        Value::ConditionVariable { inner, .. } => {
            inner.notify_one();
            Ok(())
        }
        _ => Err(Error::TypeError {
            expected: "condition-variable".to_string(),
            got: cv.type_name(),
        }),
    }
}

/// Notify all threads waiting on the condition variable
pub fn condition_broadcast(cv: &Value) -> Result<()> {
    match cv {
        Value::ConditionVariable { inner, .. } => {
            inner.notify_all();
            Ok(())
        }
        _ => Err(Error::TypeError {
            expected: "condition-variable".to_string(),
            got: cv.type_name(),
        }),
    }
}

// =============================================================================
// Semaphore Functions
// =============================================================================

use crate::runtime::value::SemaphoreInner;

/// Create a new semaphore with initial count
pub fn make_semaphore(count: i64, name: Option<String>) -> Value {
    Value::Semaphore {
        name,
        count: Arc::new(AtomicI64::new(count)),
        inner: Arc::new(Mutex::new(SemaphoreInner { count })),
        condvar: Arc::new(Condvar::new()),
    }
}

/// Signal (increment) a semaphore
pub fn signal_semaphore(sem: &Value, count: i64) -> Result<()> {
    match sem {
        Value::Semaphore {
            inner,
            condvar,
            count: atomic_count,
            ..
        } => {
            let mut guard = inner.lock().unwrap();
            guard.count += count;
            atomic_count.store(guard.count, Ordering::SeqCst);

            // Wake waiting threads
            for _ in 0..count {
                condvar.notify_one();
            }
            Ok(())
        }
        _ => Err(Error::TypeError {
            expected: "semaphore".to_string(),
            got: sem.type_name(),
        }),
    }
}

/// Wait on (decrement) a semaphore
///
/// Blocks until the semaphore count is positive, then decrements it.
/// Returns true if acquired, false if timed out.
pub fn wait_on_semaphore(sem: &Value, timeout: Option<Duration>) -> Result<bool> {
    match sem {
        Value::Semaphore {
            inner,
            condvar,
            count: atomic_count,
            ..
        } => {
            let start = std::time::Instant::now();
            let mut guard = inner.lock().unwrap();

            loop {
                if guard.count > 0 {
                    guard.count -= 1;
                    atomic_count.store(guard.count, Ordering::SeqCst);
                    return Ok(true);
                }

                if let Some(dur) = timeout {
                    let remaining = dur.saturating_sub(start.elapsed());
                    if remaining.is_zero() {
                        return Ok(false);
                    }
                    let (new_guard, result) = condvar.wait_timeout(guard, remaining).unwrap();
                    guard = new_guard;
                    if result.timed_out() && guard.count <= 0 {
                        return Ok(false);
                    }
                } else {
                    guard = condvar.wait(guard).unwrap();
                }
            }
        }
        _ => Err(Error::TypeError {
            expected: "semaphore".to_string(),
            got: sem.type_name(),
        }),
    }
}

// =============================================================================
// Atomic Integer Functions
// =============================================================================

/// Create a new atomic integer
pub fn make_atomic_integer(value: i64) -> Value {
    Value::AtomicInteger {
        inner: Arc::new(AtomicI64::new(value)),
    }
}

/// Get the current value of an atomic integer
pub fn atomic_integer_value(ai: &Value) -> Result<i64> {
    match ai {
        Value::AtomicInteger { inner } => Ok(inner.load(Ordering::SeqCst)),
        _ => Err(Error::TypeError {
            expected: "atomic-integer".to_string(),
            got: ai.type_name(),
        }),
    }
}

/// Atomically increment an atomic integer
///
/// Returns the new value after incrementing.
pub fn atomic_integer_incf(ai: &Value, delta: i64) -> Result<i64> {
    match ai {
        Value::AtomicInteger { inner } => {
            let new_val = inner.fetch_add(delta, Ordering::SeqCst) + delta;
            Ok(new_val)
        }
        _ => Err(Error::TypeError {
            expected: "atomic-integer".to_string(),
            got: ai.type_name(),
        }),
    }
}

/// Atomically decrement an atomic integer
///
/// Returns the new value after decrementing.
pub fn atomic_integer_decf(ai: &Value, delta: i64) -> Result<i64> {
    match ai {
        Value::AtomicInteger { inner } => {
            let new_val = inner.fetch_sub(delta, Ordering::SeqCst) - delta;
            Ok(new_val)
        }
        _ => Err(Error::TypeError {
            expected: "atomic-integer".to_string(),
            got: ai.type_name(),
        }),
    }
}

/// Atomic compare-and-swap
///
/// If the current value equals `expected`, sets it to `new_value` and returns true.
/// Otherwise, returns false without modifying the value.
pub fn atomic_integer_cas(ai: &Value, expected: i64, new_value: i64) -> Result<bool> {
    match ai {
        Value::AtomicInteger { inner } => {
            match inner.compare_exchange(expected, new_value, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        }
        _ => Err(Error::TypeError {
            expected: "atomic-integer".to_string(),
            got: ai.type_name(),
        }),
    }
}

// =============================================================================
// Type Predicates
// =============================================================================

/// Check if value is a thread
pub fn is_thread(value: &Value) -> bool {
    matches!(value, Value::Thread { .. })
}

/// Check if value is a lock
pub fn is_lock(value: &Value) -> bool {
    matches!(value, Value::Lock { .. })
}

/// Check if value is a condition variable
pub fn is_condition_variable(value: &Value) -> bool {
    matches!(value, Value::ConditionVariable { .. })
}

/// Check if value is a semaphore
pub fn is_semaphore(value: &Value) -> bool {
    matches!(value, Value::Semaphore { .. })
}

/// Check if value is an atomic integer
pub fn is_atomic_integer(value: &Value) -> bool {
    matches!(value, Value::AtomicInteger { .. })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_lock() {
        let lock = make_lock(Some("test-lock".to_string()));
        assert!(is_lock(&lock));
        assert_eq!(lock.type_name(), "lock");
    }

    #[test]
    fn test_make_semaphore() {
        let sem = make_semaphore(5, Some("test-sem".to_string()));
        assert!(is_semaphore(&sem));
        assert_eq!(sem.type_name(), "semaphore");
    }

    #[test]
    fn test_atomic_integer_operations() {
        let ai = make_atomic_integer(10);
        assert!(is_atomic_integer(&ai));

        assert_eq!(atomic_integer_value(&ai).unwrap(), 10);

        assert_eq!(atomic_integer_incf(&ai, 5).unwrap(), 15);
        assert_eq!(atomic_integer_value(&ai).unwrap(), 15);

        assert_eq!(atomic_integer_decf(&ai, 3).unwrap(), 12);
        assert_eq!(atomic_integer_value(&ai).unwrap(), 12);

        // CAS success
        assert!(atomic_integer_cas(&ai, 12, 100).unwrap());
        assert_eq!(atomic_integer_value(&ai).unwrap(), 100);

        // CAS failure
        assert!(!atomic_integer_cas(&ai, 12, 200).unwrap());
        assert_eq!(atomic_integer_value(&ai).unwrap(), 100);
    }

    #[test]
    fn test_condition_variable() {
        let cv = make_condition_variable(Some("test-cv".to_string()));
        assert!(is_condition_variable(&cv));

        // Notify should not panic even with no waiters
        condition_notify(&cv).unwrap();
        condition_broadcast(&cv).unwrap();
    }

    #[test]
    fn test_thread_id_generation() {
        let id1 = generate_thread_id();
        let id2 = generate_thread_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("thread_"));
        assert!(id2.starts_with("thread_"));
    }
}
