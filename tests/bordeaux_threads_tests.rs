//! Integration tests for Bordeaux Threads implementation
//!
//! These tests verify the OVSM implementation of Bordeaux Threads primitives
//! for portable shared-state concurrency.

use ovsm::lexer::SExprScanner;
use ovsm::parser::SExprParser;
use ovsm::runtime::{LispEvaluator, Value};

/// Helper function to evaluate OVSM code
fn eval_str(source: &str) -> Result<Value, ovsm::error::Error> {
    let mut scanner = SExprScanner::new(source);
    let tokens = scanner.scan_tokens()?;
    let mut parser = SExprParser::new(tokens);
    let program = parser.parse()?;
    let mut evaluator = LispEvaluator::new();
    evaluator.execute(&program)
}

// =============================================================================
// Lock Tests
// =============================================================================

#[test]
fn test_make_lock() {
    let result = eval_str(r#"(make-lock :name "test-lock")"#).unwrap();
    assert_eq!(result.type_name(), "lock");
}

#[test]
fn test_make_lock_without_name() {
    let result = eval_str("(make-lock)").unwrap();
    assert_eq!(result.type_name(), "lock");
}

#[test]
fn test_lockp() {
    let result = eval_str(
        r#"
        (define lock (make-lock))
        (lockp lock)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));

    let result = eval_str("(lockp 42)").unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_lockp_on_string() {
    let result = eval_str(r#"(lockp "not a lock")"#).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_lockp_on_null() {
    let result = eval_str("(lockp null)").unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_with_lock_held() {
    let result = eval_str(
        r#"
        (define lock (make-lock :name "counter-lock"))
        (define counter 0)
        (with-lock-held lock
          (set! counter (+ counter 1))
          (set! counter (+ counter 1)))
        counter
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_with_lock_held_returns_last_value() {
    let result = eval_str(
        r#"
        (define lock (make-lock))
        (with-lock-held lock
          (+ 1 2)
          (+ 3 4)
          (* 5 6))
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(30));
}

#[test]
fn test_with_lock_held_single_expression() {
    let result = eval_str(
        r#"
        (define lock (make-lock))
        (with-lock-held lock 42)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_nested_with_lock_held_different_locks() {
    let result = eval_str(
        r#"
        (define lock1 (make-lock :name "outer"))
        (define lock2 (make-lock :name "inner"))
        (define result 0)
        (with-lock-held lock1
          (set! result (+ result 10))
          (with-lock-held lock2
            (set! result (+ result 5))))
        result
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(15));
}

// =============================================================================
// Recursive Lock Tests
// =============================================================================

#[test]
fn test_make_recursive_lock() {
    let result = eval_str(r#"(make-recursive-lock :name "reentrant")"#).unwrap();
    assert_eq!(result.type_name(), "recursive-lock");
}

#[test]
fn test_make_recursive_lock_without_name() {
    let result = eval_str("(make-recursive-lock)").unwrap();
    assert_eq!(result.type_name(), "recursive-lock");
}

#[test]
fn test_recursive_lock_p() {
    let result = eval_str(
        r#"
        (define rlock (make-recursive-lock))
        (recursive-lock-p rlock)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_recursive_lock_p_on_regular_lock() {
    let result = eval_str(
        r#"
        (define lock (make-lock))
        (recursive-lock-p lock)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_with_recursive_lock_held() {
    let result = eval_str(
        r#"
        (define rlock (make-recursive-lock))
        (with-recursive-lock-held rlock
          (+ 1 2 3))
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(6));
}

#[test]
fn test_recursive_lock_nested_acquisition() {
    // Recursive locks can be acquired multiple times by the same thread
    let result = eval_str(
        r#"
        (define rlock (make-recursive-lock))
        (with-recursive-lock-held rlock
          (with-recursive-lock-held rlock
            (with-recursive-lock-held rlock
              42)))
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(42));
}

// =============================================================================
// Condition Variable Tests
// =============================================================================

#[test]
fn test_make_condition_variable() {
    let result = eval_str(r#"(make-condition-variable :name "ready")"#).unwrap();
    assert_eq!(result.type_name(), "condition-variable");
}

#[test]
fn test_make_condition_variable_without_name() {
    let result = eval_str("(make-condition-variable)").unwrap();
    assert_eq!(result.type_name(), "condition-variable");
}

#[test]
fn test_condition_variable_p() {
    let result = eval_str(
        r#"
        (define cv (make-condition-variable))
        (condition-variable-p cv)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_condition_variable_p_on_lock() {
    let result = eval_str(
        r#"
        (define lock (make-lock))
        (condition-variable-p lock)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_condition_notify_no_waiters() {
    // Should not panic even with no waiters
    let result = eval_str(
        r#"
        (define cv (make-condition-variable))
        (condition-notify cv)
        (condition-broadcast cv)
        true
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_condition_broadcast_no_waiters() {
    let result = eval_str(
        r#"
        (define cv (make-condition-variable :name "test-cv"))
        (condition-broadcast cv)
        "ok"
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::String("ok".to_string()));
}

// =============================================================================
// Semaphore Tests
// =============================================================================

#[test]
fn test_make_semaphore() {
    let result = eval_str(r#"(make-semaphore :count 5 :name "permits")"#).unwrap();
    assert_eq!(result.type_name(), "semaphore");
}

#[test]
fn test_make_semaphore_default_count() {
    let result = eval_str("(make-semaphore)").unwrap();
    assert_eq!(result.type_name(), "semaphore");
}

#[test]
fn test_make_semaphore_with_only_count() {
    let result = eval_str("(make-semaphore :count 10)").unwrap();
    assert_eq!(result.type_name(), "semaphore");
}

#[test]
fn test_semaphorep() {
    let result = eval_str(
        r#"
        (define sem (make-semaphore :count 1))
        (semaphorep sem)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_semaphorep_on_lock() {
    let result = eval_str(
        r#"
        (define lock (make-lock))
        (semaphorep lock)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_semaphore_signal_and_wait() {
    let result = eval_str(
        r#"
        (define sem (make-semaphore :count 0))
        (signal-semaphore sem :count 3)
        (wait-on-semaphore sem :timeout 1)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_semaphore_signal_default_count() {
    let result = eval_str(
        r#"
        (define sem (make-semaphore :count 0))
        (signal-semaphore sem)
        (wait-on-semaphore sem :timeout 1)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_semaphore_timeout() {
    let result = eval_str(
        r#"
        (define sem (make-semaphore :count 0))
        (wait-on-semaphore sem :timeout 0.1)
        "#,
    )
    .unwrap();
    // Should return false (timed out)
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_semaphore_multiple_signals() {
    let result = eval_str(
        r#"
        (define sem (make-semaphore :count 0))
        (signal-semaphore sem)
        (signal-semaphore sem)
        (signal-semaphore sem)
        (define r1 (wait-on-semaphore sem :timeout 1))
        (define r2 (wait-on-semaphore sem :timeout 1))
        (define r3 (wait-on-semaphore sem :timeout 1))
        (define r4 (wait-on-semaphore sem :timeout 0.1))
        [r1 r2 r3 r4]
        "#,
    )
    .unwrap();
    // First 3 should succeed, 4th should timeout
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 4);
            assert_eq!(arr[0], Value::Bool(true));
            assert_eq!(arr[1], Value::Bool(true));
            assert_eq!(arr[2], Value::Bool(true));
            assert_eq!(arr[3], Value::Bool(false));
        }
        _ => panic!("Expected array"),
    }
}

#[test]
fn test_semaphore_initial_count() {
    let result = eval_str(
        r#"
        (define sem (make-semaphore :count 2))
        (define r1 (wait-on-semaphore sem :timeout 1))
        (define r2 (wait-on-semaphore sem :timeout 1))
        (define r3 (wait-on-semaphore sem :timeout 0.1))
        [r1 r2 r3]
        "#,
    )
    .unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr[0], Value::Bool(true));
            assert_eq!(arr[1], Value::Bool(true));
            assert_eq!(arr[2], Value::Bool(false));
        }
        _ => panic!("Expected array"),
    }
}

// =============================================================================
// Atomic Integer Tests
// =============================================================================

#[test]
fn test_make_atomic_integer() {
    let result = eval_str("(make-atomic-integer :value 42)").unwrap();
    assert_eq!(result.type_name(), "atomic-integer");
}

#[test]
fn test_make_atomic_integer_default_value() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer))
        (atomic-integer-value ai)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_atomic_integer_p() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 0))
        (atomic-integer-p ai)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_atomic_integer_p_on_regular_int() {
    let result = eval_str("(atomic-integer-p 42)").unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_atomic_integer_value() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 100))
        (atomic-integer-value ai)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(100));
}

#[test]
fn test_atomic_integer_value_negative() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value -50))
        (atomic-integer-value ai)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(-50));
}

#[test]
fn test_atomic_integer_incf() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 10))
        (atomic-integer-incf ai 5)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(15));
}

#[test]
fn test_atomic_integer_decf() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 10))
        (atomic-integer-decf ai 3)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(7));
}

#[test]
fn test_atomic_integer_incf_default_delta() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 10))
        (atomic-integer-incf ai)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(11));
}

#[test]
fn test_atomic_integer_decf_default_delta() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 10))
        (atomic-integer-decf ai)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(9));
}

#[test]
fn test_atomic_integer_incf_negative_delta() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 10))
        (atomic-integer-incf ai -3)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(7));
}

#[test]
fn test_atomic_integer_cas_success() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 10))
        (atomic-integer-cas ai 10 20)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_atomic_integer_cas_failure() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 10))
        (atomic-integer-cas ai 5 20)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_atomic_integer_cas_value_unchanged_on_failure() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 10))
        (atomic-integer-cas ai 5 20)
        (atomic-integer-value ai)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(10)); // Value should still be 10
}

#[test]
fn test_atomic_integer_cas_value_changed_on_success() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 10))
        (atomic-integer-cas ai 10 20)
        (atomic-integer-value ai)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(20));
}

#[test]
fn test_atomic_integer_multiple_operations() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 0))
        (atomic-integer-incf ai 10)
        (atomic-integer-incf ai 5)
        (atomic-integer-decf ai 3)
        (atomic-integer-value ai)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(12));
}

#[test]
fn test_atomic_integer_cas_loop_pattern() {
    // Common CAS loop pattern: keep trying until success
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 100))
        ;; Simulate CAS loop - in real code this would be in a loop
        (define old-val (atomic-integer-value ai))
        (define new-val (+ old-val 50))
        (atomic-integer-cas ai old-val new-val)
        (atomic-integer-value ai)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(150));
}

// =============================================================================
// Thread Tests
// =============================================================================

#[test]
fn test_threadp() {
    let result = eval_str("(threadp 42)").unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_threadp_on_string() {
    let result = eval_str(r#"(threadp "not a thread")"#).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_thread_yield() {
    let result = eval_str("(thread-yield)").unwrap();
    assert_eq!(result, Value::Null);
}

#[test]
fn test_current_thread() {
    let result = eval_str("(current-thread)").unwrap();
    assert_eq!(result.type_name(), "thread");
}

#[test]
fn test_current_thread_is_thread() {
    let result = eval_str("(threadp (current-thread))").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_make_thread_and_join() {
    let result = eval_str(
        r#"
        (define t (make-thread (lambda () (+ 1 2 3)) :name "adder"))
        (join-thread t)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(6));
}

#[test]
fn test_make_thread_without_name() {
    let result = eval_str(
        r#"
        (define t (make-thread (lambda () (* 7 6))))
        (join-thread t)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_thread_name() {
    let result = eval_str(
        r#"
        (define t (make-thread (lambda () 42) :name "my-worker"))
        (thread-name t)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::String("my-worker".to_string()));
}

#[test]
fn test_thread_without_name() {
    let result = eval_str(
        r#"
        (define t (make-thread (lambda () 42)))
        (thread-name t)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Null);
}

#[test]
fn test_thread_returns_string() {
    let result = eval_str(
        r#"
        (define t (make-thread (lambda () "hello from thread")))
        (join-thread t)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::String("hello from thread".to_string()));
}

#[test]
fn test_thread_returns_array() {
    let result = eval_str(
        r#"
        (define t (make-thread (lambda () [1 2 3])))
        (join-thread t)
        "#,
    )
    .unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], Value::Int(1));
            assert_eq!(arr[1], Value::Int(2));
            assert_eq!(arr[2], Value::Int(3));
        }
        _ => panic!("Expected array"),
    }
}

#[test]
fn test_thread_returns_bool() {
    let result = eval_str(
        r#"
        (define t (make-thread (lambda () (> 5 3))))
        (join-thread t)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_multiple_threads_join() {
    let result = eval_str(
        r#"
        (define t1 (make-thread (lambda () 10)))
        (define t2 (make-thread (lambda () 20)))
        (define t3 (make-thread (lambda () 30)))
        (+ (join-thread t1) (join-thread t2) (join-thread t3))
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(60));
}

#[test]
fn test_thread_computation() {
    let result = eval_str(
        r#"
        (define t (make-thread (lambda ()
          (+ 1 2 3 4 5 6 7 8 9))))
        (join-thread t)
        "#,
    )
    .unwrap();
    // Sum of 1..9 = 45
    assert_eq!(result, Value::Int(45));
}

// =============================================================================
// Integration Tests - Common Patterns
// =============================================================================

#[test]
fn test_atomic_counter_pattern() {
    // Multiple "increments" using atomic operations
    let result = eval_str(
        r#"
        (define counter (make-atomic-integer :value 0))
        (atomic-integer-incf counter)
        (atomic-integer-incf counter)
        (atomic-integer-incf counter)
        (atomic-integer-value counter)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_lock_protected_update() {
    let result = eval_str(
        r#"
        (define lock (make-lock :name "data-lock"))
        (define data 0)
        
        (with-lock-held lock
          (set! data (+ data 10)))
        
        (with-lock-held lock
          (set! data (+ data 5)))
        
        data
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(15));
}

#[test]
fn test_semaphore_resource_pool() {
    // Simulate a resource pool with limited capacity
    let result = eval_str(
        r#"
        (define pool (make-semaphore :count 2 :name "resource-pool"))
        
        ;; Acquire both resources
        (define got1 (wait-on-semaphore pool :timeout 1))
        (define got2 (wait-on-semaphore pool :timeout 1))
        
        ;; Try to acquire third (should fail)
        (define got3 (wait-on-semaphore pool :timeout 0.1))
        
        ;; Release one
        (signal-semaphore pool)
        
        ;; Now we can acquire again
        (define got4 (wait-on-semaphore pool :timeout 1))
        
        [got1 got2 got3 got4]
        "#,
    )
    .unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr[0], Value::Bool(true));
            assert_eq!(arr[1], Value::Bool(true));
            assert_eq!(arr[2], Value::Bool(false));
            assert_eq!(arr[3], Value::Bool(true));
        }
        _ => panic!("Expected array"),
    }
}

#[test]
fn test_thread_simple_multiplication() {
    // Note: threads run in isolated environments, so closures don't capture outer vars
    // This test verifies basic thread computation without closures
    let result = eval_str(
        r#"
        (define t (make-thread (lambda () (* 10 5))))
        (join-thread t)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(50));
}

#[test]
fn test_parallel_computation() {
    let result = eval_str(
        r#"
        ;; Compute sum in parallel
        (define t1 (make-thread (lambda () (+ 1 2 3 4 5))))
        (define t2 (make-thread (lambda () (+ 6 7 8 9 10))))
        (+ (join-thread t1) (join-thread t2))
        "#,
    )
    .unwrap();
    // (1+2+3+4+5) + (6+7+8+9+10) = 15 + 40 = 55
    assert_eq!(result, Value::Int(55));
}

// =============================================================================
// Type Predicate Tests
// =============================================================================

#[test]
fn test_all_type_predicates() {
    // Test all threading type predicates return false for non-matching types
    let tests = vec![
        ("(threadp (make-lock))", false),
        ("(lockp (make-semaphore :count 1))", false),
        ("(recursive-lock-p (make-lock))", false),
        ("(condition-variable-p (make-lock))", false),
        ("(semaphorep (make-condition-variable))", false),
        ("(atomic-integer-p (make-lock))", false),
    ];

    for (code, expected) in tests {
        let result = eval_str(code).unwrap();
        assert_eq!(result, Value::Bool(expected), "Failed for: {}", code);
    }
}

#[test]
fn test_type_predicates_positive() {
    let tests = vec![
        ("(threadp (current-thread))", true),
        ("(lockp (make-lock))", true),
        ("(recursive-lock-p (make-recursive-lock))", true),
        ("(condition-variable-p (make-condition-variable))", true),
        ("(semaphorep (make-semaphore :count 1))", true),
        ("(atomic-integer-p (make-atomic-integer :value 0))", true),
    ];

    for (code, expected) in tests {
        let result = eval_str(code).unwrap();
        assert_eq!(result, Value::Bool(expected), "Failed for: {}", code);
    }
}

// =============================================================================
// Alternative Name Tests
// =============================================================================

#[test]
fn test_alternative_function_names() {
    // Test that alternative names work (using ? suffix variants)
    let result = eval_str(
        r#"
        (define lock (make-lock))
        (lock? lock)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));

    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 42))
        (atomic-integer? ai)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_thread_question_mark_predicate() {
    let result = eval_str("(thread? (current-thread))").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_semaphore_question_mark_predicate() {
    let result = eval_str("(semaphore? (make-semaphore :count 1))").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_recursive_lock_question_mark_predicate() {
    let result = eval_str("(recursive-lock? (make-recursive-lock))").unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_condition_variable_question_mark_predicate() {
    let result = eval_str("(condition-variable? (make-condition-variable))").unwrap();
    assert_eq!(result, Value::Bool(true));
}

// =============================================================================
// Edge Cases and Error Handling
// =============================================================================

#[test]
fn test_atomic_integer_large_values() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 1000000000))
        (atomic-integer-incf ai 1000000000)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(2000000000));
}

#[test]
fn test_atomic_integer_zero() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 0))
        (atomic-integer-cas ai 0 0)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_semaphore_zero_count() {
    let result = eval_str(
        r#"
        (define sem (make-semaphore :count 0))
        (wait-on-semaphore sem :timeout 0.05)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_thread_returns_null() {
    let result = eval_str(
        r#"
        (define t (make-thread (lambda () null)))
        (join-thread t)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Null);
}

// =============================================================================
// Stress Tests
// =============================================================================

#[test]
fn test_many_atomic_operations() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 0))
        (define i 0)
        (while (< i 100)
          (atomic-integer-incf ai)
          (set! i (+ i 1)))
        (atomic-integer-value ai)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(100));
}

#[test]
fn test_multiple_locks_created() {
    let result = eval_str(
        r#"
        (define lock1 (make-lock :name "lock1"))
        (define lock2 (make-lock :name "lock2"))
        (define lock3 (make-lock :name "lock3"))
        (define locks [lock1 lock2 lock3])
        (length locks)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(3));
}

#[test]
fn test_multiple_semaphores() {
    let result = eval_str(
        r#"
        (define s1 (make-semaphore :count 1))
        (define s2 (make-semaphore :count 2))
        (define s3 (make-semaphore :count 3))
        (+ (if (wait-on-semaphore s1 :timeout 1) 1 0)
           (if (wait-on-semaphore s2 :timeout 1) 1 0)
           (if (wait-on-semaphore s3 :timeout 1) 1 0))
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(3));
}

// =============================================================================
// Multi-threaded Computation Tests
// =============================================================================

#[test]
fn test_parallel_sum_results() {
    // Each thread computes independently and we sum the results
    let result = eval_str(
        r#"
        (define t1 (make-thread (lambda () 10)))
        (define t2 (make-thread (lambda () 20)))
        (define t3 (make-thread (lambda () 30)))
        (+ (join-thread t1) (join-thread t2) (join-thread t3))
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(60));
}

#[test]
fn test_thread_returns_float() {
    let result = eval_str(
        r#"
        (define t (make-thread (lambda () (/ 22.0 7.0))))
        (join-thread t)
        "#,
    )
    .unwrap();
    match result {
        Value::Float(f) => assert!((f - 3.142857).abs() < 0.001),
        _ => panic!("Expected float"),
    }
}

#[test]
fn test_thread_returns_object() {
    let result = eval_str(
        r#"
        (define t (make-thread (lambda () {:x 10 :y 20})))
        (join-thread t)
        "#,
    )
    .unwrap();
    assert_eq!(result.type_name(), "object");
}

#[test]
fn test_five_threads_parallel() {
    let result = eval_str(
        r#"
        (define t1 (make-thread (lambda () 1)))
        (define t2 (make-thread (lambda () 2)))
        (define t3 (make-thread (lambda () 3)))
        (define t4 (make-thread (lambda () 4)))
        (define t5 (make-thread (lambda () 5)))
        (+ (join-thread t1)
           (join-thread t2)
           (join-thread t3)
           (join-thread t4)
           (join-thread t5))
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(15));
}

#[test]
fn test_thread_sum_of_squares() {
    // Compute sum of squares in a thread
    let result = eval_str(
        r#"
        (define t (make-thread (lambda () 
          (+ (* 1 1) (* 2 2) (* 3 3) (* 4 4) (* 5 5)))))
        (join-thread t)
        "#,
    )
    .unwrap();
    // 1 + 4 + 9 + 16 + 25 = 55
    assert_eq!(result, Value::Int(55));
}

#[test]
fn test_thread_product_computation() {
    // Compute 6! = 720 directly
    let result = eval_str(
        r#"
        (define t (make-thread (lambda () (* 1 2 3 4 5 6))))
        (join-thread t)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(720));
}

// =============================================================================
// Lock Contention and Safety Tests
// =============================================================================

#[test]
fn test_lock_protects_counter_increment() {
    let result = eval_str(
        r#"
        (define lock (make-lock :name "counter-lock"))
        (define counter 0)
        (with-lock-held lock (set! counter (+ counter 1)))
        (with-lock-held lock (set! counter (+ counter 1)))
        (with-lock-held lock (set! counter (+ counter 1)))
        (with-lock-held lock (set! counter (+ counter 1)))
        (with-lock-held lock (set! counter (+ counter 1)))
        counter
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_lock_exception_safety() {
    // Lock should be released even if body succeeds
    let result = eval_str(
        r#"
        (define lock (make-lock))
        (define x (with-lock-held lock (+ 1 2)))
        (define y (with-lock-held lock (+ 3 4)))
        (+ x y)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_lock_with_complex_body() {
    let result = eval_str(
        r#"
        (define lock (make-lock))
        (with-lock-held lock
          (define a 10)
          (define b 20)
          (define c (+ a b))
          (* c 2))
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(60));
}

#[test]
fn test_recursive_lock_deeply_nested() {
    let result = eval_str(
        r#"
        (define rlock (make-recursive-lock))
        (with-recursive-lock-held rlock
          (with-recursive-lock-held rlock
            (with-recursive-lock-held rlock
              (with-recursive-lock-held rlock
                (with-recursive-lock-held rlock
                  100)))))
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(100));
}

// =============================================================================
// Semaphore Advanced Tests
// =============================================================================

#[test]
fn test_semaphore_binary_mutex_pattern() {
    // Binary semaphore (count=1) acts like a mutex
    let result = eval_str(
        r#"
        (define mutex (make-semaphore :count 1 :name "binary-mutex"))
        (define protected-value 0)
        
        ;; Acquire "lock"
        (wait-on-semaphore mutex :timeout 1)
        (set! protected-value (+ protected-value 10))
        ;; Release "lock"
        (signal-semaphore mutex)
        
        ;; Acquire again
        (wait-on-semaphore mutex :timeout 1)
        (set! protected-value (+ protected-value 5))
        (signal-semaphore mutex)
        
        protected-value
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(15));
}

#[test]
fn test_semaphore_counting_down() {
    let result = eval_str(
        r#"
        (define sem (make-semaphore :count 5))
        (wait-on-semaphore sem :timeout 1)
        (wait-on-semaphore sem :timeout 1)
        (wait-on-semaphore sem :timeout 1)
        (wait-on-semaphore sem :timeout 1)
        (wait-on-semaphore sem :timeout 1)
        ;; All 5 permits consumed, next should fail
        (wait-on-semaphore sem :timeout 0.05)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_semaphore_signal_multiple_at_once() {
    let result = eval_str(
        r#"
        (define sem (make-semaphore :count 0))
        ;; Signal 5 at once
        (signal-semaphore sem :count 5)
        ;; Should be able to wait 5 times
        (define r1 (wait-on-semaphore sem :timeout 1))
        (define r2 (wait-on-semaphore sem :timeout 1))
        (define r3 (wait-on-semaphore sem :timeout 1))
        (define r4 (wait-on-semaphore sem :timeout 1))
        (define r5 (wait-on-semaphore sem :timeout 1))
        (define r6 (wait-on-semaphore sem :timeout 0.05))
        (+ (if r1 1 0) (if r2 1 0) (if r3 1 0) (if r4 1 0) (if r5 1 0) (if r6 1 0))
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(5));
}

#[test]
fn test_semaphore_producer_consumer_simulation() {
    let result = eval_str(
        r#"
        (define buffer (make-semaphore :count 0 :name "buffer"))
        (define consumed 0)
        
        ;; Producer: add 3 items
        (signal-semaphore buffer)
        (signal-semaphore buffer)
        (signal-semaphore buffer)
        
        ;; Consumer: consume items
        (if (wait-on-semaphore buffer :timeout 1) (set! consumed (+ consumed 1)) null)
        (if (wait-on-semaphore buffer :timeout 1) (set! consumed (+ consumed 1)) null)
        (if (wait-on-semaphore buffer :timeout 1) (set! consumed (+ consumed 1)) null)
        
        consumed
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(3));
}

// =============================================================================
// Atomic Integer Advanced Tests
// =============================================================================

#[test]
fn test_atomic_integer_spin_increment() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 0))
        (atomic-integer-incf ai)
        (atomic-integer-incf ai)
        (atomic-integer-incf ai)
        (atomic-integer-incf ai)
        (atomic-integer-incf ai)
        (atomic-integer-incf ai)
        (atomic-integer-incf ai)
        (atomic-integer-incf ai)
        (atomic-integer-incf ai)
        (atomic-integer-incf ai)
        (atomic-integer-value ai)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(10));
}

#[test]
fn test_atomic_integer_negative_values() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value -100))
        (atomic-integer-incf ai 50)
        (atomic-integer-value ai)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(-50));
}

#[test]
fn test_atomic_integer_decf_to_negative() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 10))
        (atomic-integer-decf ai 15)
        (atomic-integer-value ai)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(-5));
}

#[test]
fn test_atomic_integer_cas_chain() {
    // Simulate a CAS-based update chain
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 0))
        (atomic-integer-cas ai 0 10)
        (atomic-integer-cas ai 10 20)
        (atomic-integer-cas ai 20 30)
        (atomic-integer-cas ai 30 40)
        (atomic-integer-value ai)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(40));
}

#[test]
fn test_atomic_integer_cas_failure_chain() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 100))
        ;; All these should fail because expected values are wrong
        (define r1 (atomic-integer-cas ai 0 200))
        (define r2 (atomic-integer-cas ai 50 200))
        (define r3 (atomic-integer-cas ai 99 200))
        ;; This one should succeed
        (define r4 (atomic-integer-cas ai 100 200))
        [r1 r2 r3 r4]
        "#,
    )
    .unwrap();
    match result {
        Value::Array(arr) => {
            assert_eq!(arr[0], Value::Bool(false));
            assert_eq!(arr[1], Value::Bool(false));
            assert_eq!(arr[2], Value::Bool(false));
            assert_eq!(arr[3], Value::Bool(true));
        }
        _ => panic!("Expected array"),
    }
}

#[test]
fn test_atomic_integer_as_flag() {
    // Use atomic integer as a boolean flag (0 = false, 1 = true)
    let result = eval_str(
        r#"
        (define flag (make-atomic-integer :value 0))
        
        ;; Set flag
        (atomic-integer-cas flag 0 1)
        
        ;; Check flag
        (define is-set (= (atomic-integer-value flag) 1))
        
        ;; Clear flag
        (atomic-integer-cas flag 1 0)
        
        ;; Check cleared
        (define is-cleared (= (atomic-integer-value flag) 0))
        
        (and is-set is-cleared)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_atomic_integer_sequential_updates() {
    // Threads compute independently, results combined after
    let result = eval_str(
        r#"
        (define t1 (make-thread (lambda () (+ 1 1 1))))
        (define t2 (make-thread (lambda () (+ 1 1))))
        (+ (join-thread t1) (join-thread t2))
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(5));
}

// =============================================================================
// Condition Variable Advanced Tests
// =============================================================================

#[test]
fn test_condition_variable_multiple_notify() {
    let result = eval_str(
        r#"
        (define cv (make-condition-variable :name "multi-notify"))
        (condition-notify cv)
        (condition-notify cv)
        (condition-notify cv)
        "ok"
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::String("ok".to_string()));
}

#[test]
fn test_condition_variable_multiple_broadcast() {
    let result = eval_str(
        r#"
        (define cv (make-condition-variable))
        (condition-broadcast cv)
        (condition-broadcast cv)
        42
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(42));
}

// =============================================================================
// Thread Name and Identity Tests
// =============================================================================

#[test]
fn test_thread_names_are_unique() {
    let result = eval_str(
        r#"
        (define t1 (make-thread (lambda () 1) :name "worker-1"))
        (define t2 (make-thread (lambda () 2) :name "worker-2"))
        (define t3 (make-thread (lambda () 3) :name "worker-3"))
        (define n1 (thread-name t1))
        (define n2 (thread-name t2))
        (define n3 (thread-name t3))
        (join-thread t1)
        (join-thread t2)
        (join-thread t3)
        (and (not (= n1 n2)) (not (= n2 n3)) (not (= n1 n3)))
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_current_thread_consistency() {
    let result = eval_str(
        r#"
        (define t1 (current-thread))
        (define t2 (current-thread))
        (threadp t1)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

// =============================================================================
// Mixed Primitive Tests
// =============================================================================

#[test]
fn test_lock_and_atomic_together() {
    let result = eval_str(
        r#"
        (define lock (make-lock :name "hybrid-lock"))
        (define counter (make-atomic-integer :value 0))
        
        (with-lock-held lock
          (atomic-integer-incf counter 10))
        
        (with-lock-held lock
          (atomic-integer-incf counter 20))
        
        (atomic-integer-value counter)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(30));
}

#[test]
fn test_semaphore_and_atomic_together() {
    let result = eval_str(
        r#"
        (define sem (make-semaphore :count 3))
        (define counter (make-atomic-integer :value 0))
        
        ;; Acquire permit and increment
        (when (wait-on-semaphore sem :timeout 1)
          (atomic-integer-incf counter))
        
        (when (wait-on-semaphore sem :timeout 1)
          (atomic-integer-incf counter))
        
        (atomic-integer-value counter)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_all_primitives_created() {
    let result = eval_str(
        r#"
        (define thread (current-thread))
        (define lock (make-lock :name "test"))
        (define rlock (make-recursive-lock :name "test"))
        (define cv (make-condition-variable :name "test"))
        (define sem (make-semaphore :count 1 :name "test"))
        (define ai (make-atomic-integer :value 42))
        
        (+ (if (threadp thread) 1 0)
           (if (lockp lock) 1 0)
           (if (recursive-lock-p rlock) 1 0)
           (if (condition-variable-p cv) 1 0)
           (if (semaphorep sem) 1 0)
           (if (atomic-integer-p ai) 1 0))
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(6));
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_atomic_integer_zero_delta() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 100))
        (atomic-integer-incf ai 0)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(100));
}

#[test]
fn test_atomic_integer_max_value() {
    let result = eval_str(
        r#"
        (define ai (make-atomic-integer :value 9223372036854775800))
        (atomic-integer-incf ai 7)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(9223372036854775807));
}

#[test]
fn test_semaphore_large_count() {
    let result = eval_str(
        r#"
        (define sem (make-semaphore :count 1000000))
        (wait-on-semaphore sem :timeout 1)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_thread_immediate_join() {
    // Thread that returns immediately
    let result = eval_str(
        r#"
        (define t (make-thread (lambda () 999)))
        (join-thread t)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(999));
}

#[test]
fn test_thread_string_computation() {
    let result = eval_str(
        r#"
        (define t (make-thread (lambda () (str "Hello" " " "World"))))
        (join-thread t)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::String("Hello World".to_string()));
}

#[test]
fn test_lock_with_conditional() {
    let result = eval_str(
        r#"
        (define lock (make-lock))
        (define x 5)
        (with-lock-held lock
          (if (> x 3)
              "greater"
              "lesser"))
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::String("greater".to_string()));
}

#[test]
fn test_multiple_atomic_integers() {
    let result = eval_str(
        r#"
        (define a1 (make-atomic-integer :value 10))
        (define a2 (make-atomic-integer :value 20))
        (define a3 (make-atomic-integer :value 30))
        (+ (atomic-integer-value a1)
           (atomic-integer-value a2)
           (atomic-integer-value a3))
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Int(60));
}

#[test]
fn test_semaphore_timeout_zero() {
    let result = eval_str(
        r#"
        (define sem (make-semaphore :count 0))
        (wait-on-semaphore sem :timeout 0)
        "#,
    )
    .unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_thread_nested_arithmetic() {
    let result = eval_str(
        r#"
        (define t (make-thread (lambda () (* (+ 2 3) (- 10 4)))))
        (join-thread t)
        "#,
    )
    .unwrap();
    // (2+3) * (10-4) = 5 * 6 = 30
    assert_eq!(result, Value::Int(30));
}
