# How to Use Solisp

This guide shows you all the ways to execute Solisp scripts and programs.

## Table of Contents

1. [Quick Start](#quick-start)
2. [Method 1: File Execution](#method-1-file-execution-recommended)
3. [Method 2: Interactive REPL](#method-2-interactive-repl)
4. [Method 3: Programmatic Usage](#method-3-programmatic-usage)
5. [Method 4: Unit Tests](#method-4-unit-tests)
6. [Common Issues](#common-issues)

---

## Quick Start

**Fastest way to run a Solisp script:**

```bash
cd /home/runner/work/solisp/solisp
cargo run --example run_file examples/real_world/sol_transfer.solisp
```

Expected output:
```
🚀 Executing: examples/real_world/sol_transfer.solisp
============================================================
✅ Result: ...
```

---

## Method 1: File Execution (Recommended)

### Step 1: Create a Script File

Create a file named `my_script.solisp`:

```lisp
;; my_script.solisp
(define sum 0)

(for (i (range 1 11))
  (set! sum (+ sum i)))

sum
```

### Step 2: Execute It

```bash
cargo run --example run_file my_script.solisp
```

### Available Example Scripts

```bash
# Real-world examples
cargo run --example run_file examples/real_world/sol_transfer.solisp
cargo run --example run_file examples/real_world/whale_hunter.solisp
cargo run --example run_file examples/real_world/pumpfun_graduation_tracker.solisp
```

---

## Method 2: Interactive REPL

Launch the interactive Read-Eval-Print Loop:

```bash
cargo run --example simple_repl
```

### Example Session

```
╔══════════════════════════════════════════╗
║   Solisp Interactive REPL v1.0.0         ║
╚══════════════════════════════════════════╝

Type Solisp expressions and press Enter.
Type 'exit' or press Ctrl+C to quit.
Type 'help' for examples.

solisp[1]> (+ 2 (* 3 4))
  ⇒ Int(14)

solisp[2]> (> 10 5)
  ⇒ Bool(true)

solisp[3]> (if (> 10 5) "yes" "no")
  ⇒ String("yes")

solisp[4]> exit
Goodbye! 👋
```

### REPL Commands

- `help` - Show examples
- `clear` - Clear environment
- `exit` or `quit` - Exit REPL

---

## Method 3: Programmatic Usage

Use Solisp as a library in your Rust programs.

### Create a Binary Project

```bash
cargo new my_solisp_app
cd my_solisp_app
```

### Add Dependency

Edit `Cargo.toml`:

```toml
[dependencies]
solisp = "1.0.0"
```

### Write Your Code

Edit `src/main.rs`:

```rust
use solisp::{Evaluator, Parser, Scanner, Value};

fn execute_solisp(code: &str) -> Result<Value, Box<dyn std::error::Error>> {
    // Tokenize (Scanner)
    let mut scanner = Scanner::new(code);
    let tokens = scanner.scan_tokens()?;

    // Parse (Parser)
    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;

    // Execute (Evaluator)
    let mut evaluator = Evaluator::new();
    Ok(evaluator.execute(&program)?)
}

fn main() {
    // Example 1: Simple arithmetic
    let code1 = "(+ 2 (* 3 4))";
    match execute_solisp(code1) {
        Ok(result) => println!("Result: {:?}", result), // Int(14)
        Err(err) => eprintln!("Error: {}", err),
    }

    // Example 2: Variables and control flow
    let code2 = r#"
        (define x 10)
        (define y 20)

        (if (< x y)
            "x is less"
            "x is greater or equal")
    "#;

    match execute_solisp(code2) {
        Ok(result) => println!("Result: {:?}", result),
        Err(err) => eprintln!("Error: {}", err),
    }

    // Example 3: Loop with accumulator
    let code3 = r#"
        (define sum 0)
        (for (i (range 1 11))
          (set! sum (+ sum i)))
        sum
    "#;

    match execute_solisp(code3) {
        Ok(result) => println!("Sum 1-10: {:?}", result), // Int(55)
        Err(err) => eprintln!("Error: {}", err),
    }
}
```

### Run It

```bash
cargo run
```

Expected output:
```
Result: Int(14)
Result: String("x is less")
Sum 1-10: Int(55)
```

---

## Method 4: Unit Tests

Run existing tests to see more examples:

```bash
# Run all tests
cargo test --package solisp

# Run specific test file
cargo test --package solisp --test lisp_e2e_tests

# Run with output
cargo test --package solisp -- --show-output

# Run specific test
cargo test --package solisp test_let_star

# Run unit tests only
cargo test --package solisp --lib
```

### Writing Your Own Tests

Create `tests/my_test.rs`:

```rust
use solisp::{Evaluator, Parser, Scanner, Value};

fn execute(code: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let mut scanner = Scanner::new(code);
    let tokens = scanner.scan_tokens()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;
    let mut evaluator = Evaluator::new();
    Ok(evaluator.execute(&program)?)
}

#[test]
fn test_my_feature() {
    let result = execute(r#"
        (define result 0)
        (for (i (range 1 6))
          (set! result (+ result i)))
        result
    "#).unwrap();

    assert_eq!(result, Value::Int(15)); // 1+2+3+4+5
}
```

Run it:
```bash
cargo test test_my_feature
```

---

## Common Issues

### Issue 1: "Undefined variable" Error

**Problem:**
```lisp
(for (i (range 1 5))
  (set! sum (+ sum i)))  ; ❌ sum not defined!
```

**Solution:**
```lisp
(define sum 0)  ; ✅ Define first
(for (i (range 1 5))
  (set! sum (+ sum i)))
```

### Issue 2: Division by Zero

**Problem:**
```lisp
(/ 10 0)  ; ❌ Runtime error
```

**Solution:**
```lisp
(define x 10)
(define y 0)

(if (= y 0)
    "Error: division by zero"
    (/ x y))
```

---

## Language Features Quick Reference

### ✅ Fully Working

- Variables: `(define var value)`
- Constants: `(const NAME value)`
- Control flow: `if`, `cond`, `when`, `unless`, `for`, `while`
- Functions: `defun`, `lambda`, closures
- Macros: `defmacro`, quasiquote, unquote
- Operators: `+`, `-`, `*`, `/`, `%`, `<`, `>`, `=`, `!=`, `and`, `or`, `not`
- Data types: Int, Float, String, Bool, Null, Arrays, Objects, Ranges
- Advanced: `let`, `let*`, `flet`, `labels`, `case`, `typecase`, multiple values

### ⚠️ Experimental

- `try/catch` - Basic error handling

---

## Next Steps

1. **Try Examples:** Run scripts in `examples/real_world/` directory
2. **Read Guide:** Check `USAGE_GUIDE.md` for complete language reference
3. **Check Docs:** See `BUILTIN_FUNCTIONS.md` for all built-in functions
4. **Write Code:** Create your own `.solisp` scripts
5. **Run Tests:** Explore test files in `tests/` for more examples

---

## Resources

| File | Description |
|------|-------------|
| `USAGE_GUIDE.md` | Complete language reference and syntax |
| `BUILTIN_FUNCTIONS.md` | All built-in functions glossary |
| `examples/README.md` | Example scripts documentation |
| `tests/` | Unit test examples |

---

**Happy coding with Solisp! 🚀**
