# Solisp Examples

This directory contains example Solisp scripts and Rust programs demonstrating the interpreter.

## Running Examples

### Execute Solisp Script Files

```bash
cargo run --example run_file <script.solisp>
```

### Available Scripts

| Script | Description | Output |
|--------|-------------|--------|
| `hello_world.solisp` | Simple hello world | `String("Hello from Solisp! 🚀")` |
| `factorial.solisp` | Calculate 5! with FOR loop | `Int(120)` |
| `fibonacci.solisp` | Calculate 10th Fibonacci number | `Int(55)` |
| `array_operations.solisp` | Array iteration and average | `Int(3)` |
| `conditional_logic.solisp` | Nested IF/ELSE for grading | `String("Grade: B...")` |
| `loop_control.solisp` | BREAK and CONTINUE demo | `Int(64)` |

### Run All Examples

```bash
# Hello World
cargo run --example run_file examples/hello_world.solisp

# Factorial (5! = 120)
cargo run --example run_file examples/factorial.solisp

# Fibonacci (10th number = 55)
cargo run --example run_file examples/fibonacci.solisp

# Array operations (average = 3)
cargo run --example run_file examples/array_operations.solisp

# Conditional logic (grade based on score)
cargo run --example run_file examples/conditional_logic.solisp

# Loop control (BREAK/CONTINUE)
cargo run --example run_file examples/loop_control.solisp
```

## Example Rust Programs

| File | Description |
|------|-------------|
| `run_file.rs` | Execute Solisp scripts from files |
| `complete_demo.rs` | Comprehensive feature demonstration |
| `comprehensive_tools.rs` | Tool system examples |
| `qa_test_runner.rs` | QA test suite runner |
| `showcase_new_features.rs` | Feature showcase |
| `tools_demo.rs` | Standard library tools |

### Running Rust Examples

```bash
cargo run --example complete_demo
cargo run --example tools_demo
# etc.
```

## Creating Your Own Scripts

### Basic Template

```solisp
// your_script.solisp
$result = 0

FOR $i IN [1..10]:
    $result = $result + $i

RETURN $result
```

### Run It

```bash
cargo run --example run_file your_script.solisp
```

## Important Notes

1. **Block Termination:** Use explicit ELSE branches or BREAK statements to avoid ambiguity
2. **Variable Scope:** Define variables before loops that need them
3. **Ranges:** `[1..10]` creates range 1-9 (exclusive end)
4. **Comments:** Use `//` for single-line comments

## Real-World Examples (`real_world/`)

Production-ready Solisp scripts demonstrating advanced patterns:

### Async & Streaming
| Script | Description |
|--------|-------------|
| `async_stream_example.solisp` | Async stream processing with backpressure handling |
| `event_driven_stream.solisp` | Event-driven architecture patterns |

### AI & Agents
| Script | Description |
|--------|-------------|
| `ai_compatibility_demo.solisp` | AI model integration patterns |
| `onchain_ai_agent.solisp` | Fully on-chain AI agent (11KB) |

### DeFi & Governance
| Script | Description |
|--------|-------------|
| `dao_simulation.solisp` | Complete DAO governance simulation |
| `pumpfun_graduation_tracker.solisp` | Track pump.fun token graduations to Raydium |
| `grad_tracker_final.solisp` | Optimized graduation tracker |
| `whale_hunter.solisp` | Whale wallet detection and tracking (11KB) |

### Wallet & Token Analysis
| Script | Description |
|--------|-------------|
| `wallet_discovery_deep.solisp` | Multi-hop wallet relationship discovery |
| `analyze_wallet.solisp` | Basic wallet activity analysis |
| `token_analysis_paginated.solisp` | Paginated token flow analysis |
| `token_flow_analysis.solisp` | Track token movement patterns |
| `working_token_analysis.solisp` | Complete token analysis workflow |

### On-Chain Operations
| Script | Description |
|--------|-------------|
| `sol_transfer.solisp` | Native SOL transfer via system program |
| `spl_token_transfer_signed.solisp` | SPL token transfer with PDA signing |
| `solisp_full_analysis.solisp` | Comprehensive market analysis |
| `final_demo.solisp` | Feature demonstration script |

### Running Real-World Examples

```bash
# Using OSVM CLI
solisp run crates/solisp/examples/real_world/whale_hunter.solisp

# Check syntax only
solisp check crates/solisp/examples/real_world/dao_simulation.solisp

# Compile to Solana BPF
solisp compile crates/solisp/examples/real_world/onchain_ai_agent.solisp -o agent.so
```

## Deployed Programs (Devnet)

These Solisp programs are live on Solana devnet:

| Program | Program ID | Description |
|---------|-----------|-------------|
| Strategy Registry | `1yEJEpCk1cKDmVGC7iFp8pf9EUcvKkczUmgHWKeX7p2` | Hot-swappable agent strategies |
| Basic Strategy | `2uiPuUnNjh4R1x3bwiX3ZRzqr7gbQyYtZ2PvXwU68xyh` | Entry-level agent strategy |

## See Also

- `../USAGE_GUIDE.md` - Complete language reference
- `../TEST_RESULTS_SUMMARY.md` - Implementation status
- `../tests/` - Unit test examples
- `../../examples/solisp_scripts/` - More Solisp program examples
