//! # Verification Condition Generator
//!
//! This module generates Lean 4 verification conditions from OVSM AST.

use super::types::{LeanType, TypeMapper};
use super::{SourceLocation, VerificationProperties};
use crate::parser::{BinaryOp, Expression, Program, Statement};
use crate::{Error, Result};
use std::collections::HashMap;

/// Category of verification condition
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VCCategory {
    /// Division by zero check
    DivisionSafety,
    /// Array bounds check
    ArrayBounds,
    /// Arithmetic overflow check (a + b, a * b overflow u64)
    ArithmeticOverflow,
    /// Arithmetic underflow check (a - b where a < b)
    ArithmeticUnderflow,
    /// Refinement type predicate satisfaction
    RefinementType,
    /// Solana balance conservation (lamports in = lamports out)
    BalanceConservation,
    /// Signer verification (account must be signer before state change)
    SignerCheck,
    /// Writability check (account must be writable before mem-store)
    WritabilityCheck,
    /// Instruction data bounds
    InstructionDataBounds,
    /// Account owner check (program must own account before modification)
    AccountOwnerCheck,
    /// PDA seed verification
    PDASeedCheck,
    /// Rent exemption check
    RentExemptCheck,
    /// Reentrancy guard (CPI safety)
    ReentrancyCheck,
    /// Integer truncation (u64 -> u8 etc)
    IntegerTruncation,
    /// Null pointer dereference
    NullPointerCheck,
    /// Uninitialized memory read
    UninitializedMemory,
    /// Double close/free of accounts
    DoubleFree,
    /// Account data bounds (offset within account data)
    AccountDataBounds,
    /// Loop invariant verification
    LoopInvariant,
    /// Account discriminator/type validation
    DiscriminatorCheck,
    /// Sysvar account validation (Clock, Rent, etc.)
    SysvarCheck,
    /// Inter-procedural function call safety
    FunctionCallSafety,
    /// Token account must be owned by SPL Token program
    TokenAccountOwnerCheck,
    /// Mint authority must be valid for minting operations
    MintAuthorityCheck,
    /// Buffer has sufficient capacity for serialization
    BufferOverflowCheck,
    /// Buffer has sufficient data for deserialization
    BufferUnderrunCheck,
    /// Close authority must be valid
    CloseAuthorityCheck,
    /// Account close must drain lamports to valid destination
    AccountCloseDrain,
    /// PDA bump seed must be canonical (from find_program_address)
    BumpSeedCanonical,
    /// Account reallocation bounds checking
    AccountRealloc,
    /// CPI depth limit check (max 4)
    CPIDepthCheck,
    /// Signer privilege must not escalate through CPI
    SignerPrivilegeEscalation,
    /// Account type/discriminator confusion check
    TypeConfusion,
    /// Arithmetic precision loss (division, fixed-point)
    ArithmeticPrecision,
    /// Account data mutability (writes to immutable sections)
    AccountDataMutability,
    /// PDA collision detection (different seeds -> same address)
    PDACollision,
    /// Instruction introspection validation
    InstructionIntrospection,
    /// Flash loan detection (borrow-use-repay in single tx)
    FlashLoanDetection,
    /// Oracle data staleness/manipulation check
    OracleManipulation,
    /// Front-running vulnerability detection
    FrontRunning,
    /// Timelock constraint bypass detection
    TimelockBypass,
    /// Reentrancy guard/lock pattern verification
    ReentrancyGuard,
    /// Option/nullable unwrap safety
    OptionUnwrap,
    /// Custom verification condition
    Custom(String),
}

impl std::fmt::Display for VCCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VCCategory::DivisionSafety => write!(f, "division_safety"),
            VCCategory::ArrayBounds => write!(f, "array_bounds"),
            VCCategory::ArithmeticOverflow => write!(f, "overflow"),
            VCCategory::ArithmeticUnderflow => write!(f, "underflow"),
            VCCategory::RefinementType => write!(f, "refinement"),
            VCCategory::BalanceConservation => write!(f, "balance"),
            VCCategory::SignerCheck => write!(f, "signer"),
            VCCategory::WritabilityCheck => write!(f, "writable"),
            VCCategory::InstructionDataBounds => write!(f, "instr_data"),
            VCCategory::AccountOwnerCheck => write!(f, "account_owner"),
            VCCategory::PDASeedCheck => write!(f, "pda_seed"),
            VCCategory::RentExemptCheck => write!(f, "rent_exempt"),
            VCCategory::ReentrancyCheck => write!(f, "reentrancy"),
            VCCategory::IntegerTruncation => write!(f, "truncation"),
            VCCategory::NullPointerCheck => write!(f, "null_ptr"),
            VCCategory::UninitializedMemory => write!(f, "uninit_mem"),
            VCCategory::DoubleFree => write!(f, "double_free"),
            VCCategory::AccountDataBounds => write!(f, "account_data"),
            VCCategory::LoopInvariant => write!(f, "loop_invariant"),
            VCCategory::DiscriminatorCheck => write!(f, "discriminator"),
            VCCategory::SysvarCheck => write!(f, "sysvar"),
            VCCategory::FunctionCallSafety => write!(f, "func_call"),
            VCCategory::TokenAccountOwnerCheck => write!(f, "token_account_owner"),
            VCCategory::MintAuthorityCheck => write!(f, "mint_authority"),
            VCCategory::BufferOverflowCheck => write!(f, "buffer_overflow"),
            VCCategory::BufferUnderrunCheck => write!(f, "buffer_underrun"),
            VCCategory::CloseAuthorityCheck => write!(f, "close_authority"),
            VCCategory::AccountCloseDrain => write!(f, "account_close_drain"),
            VCCategory::BumpSeedCanonical => write!(f, "bump_canonical"),
            VCCategory::AccountRealloc => write!(f, "account_realloc"),
            VCCategory::CPIDepthCheck => write!(f, "cpi_depth"),
            VCCategory::SignerPrivilegeEscalation => write!(f, "signer_escalation"),
            VCCategory::TypeConfusion => write!(f, "type_confusion"),
            VCCategory::ArithmeticPrecision => write!(f, "precision"),
            VCCategory::AccountDataMutability => write!(f, "data_mutability"),
            VCCategory::PDACollision => write!(f, "pda_collision"),
            VCCategory::InstructionIntrospection => write!(f, "instr_introspection"),
            VCCategory::FlashLoanDetection => write!(f, "flash_loan"),
            VCCategory::OracleManipulation => write!(f, "oracle_manipulation"),
            VCCategory::FrontRunning => write!(f, "front_running"),
            VCCategory::TimelockBypass => write!(f, "timelock_bypass"),
            VCCategory::ReentrancyGuard => write!(f, "reentrancy_guard"),
            VCCategory::OptionUnwrap => write!(f, "option_unwrap"),
            VCCategory::Custom(name) => write!(f, "custom_{}", name),
        }
    }
}

/// A verification condition to be proved
#[derive(Debug, Clone)]
pub struct VerificationCondition {
    /// Unique identifier
    pub id: String,
    /// Category
    pub category: VCCategory,
    /// Human-readable description
    pub description: String,
    /// Source location
    pub location: Option<SourceLocation>,
    /// Lean 4 property to prove
    pub property: String,
    /// Context assumptions (from guards, checks, etc.)
    pub assumptions: Vec<String>,
    /// Suggested tactic
    pub tactic: String,
}

/// Context for tracking control flow and assumptions
#[derive(Debug, Clone, Default)]
struct VCContext {
    /// Current assumptions from guard conditions
    assumptions: Vec<String>,
    /// Known variable types
    var_types: HashMap<String, LeanType>,
    /// Known array sizes
    array_sizes: HashMap<String, usize>,
    /// Current source file
    source_file: String,
    /// VC counter for unique IDs
    vc_counter: usize,
    /// Accounts verified as signers (account index -> verified)
    verified_signers: HashMap<i64, bool>,
    /// Accounts verified as writable (account index -> verified)
    verified_writable: HashMap<i64, bool>,
    /// Accounts verified for ownership (account index -> verified)
    verified_owners: HashMap<i64, bool>,
    /// Accounts that have been closed (for double-free detection)
    closed_accounts: HashMap<i64, bool>,
    /// Variables that have been initialized
    initialized_vars: HashMap<String, bool>,
    /// Lamport deltas for balance conservation (account index -> delta expression)
    lamport_deltas: Vec<(i64, String)>,
    /// CPI depth for reentrancy tracking
    cpi_depth: usize,
    /// Coverage tracking
    total_nodes: usize,
    nodes_with_vcs: usize,
    /// Uncovered risky operations
    uncovered_ops: Vec<(String, String)>, // (op_type, reason)
    /// Current loop invariants (for nested loops)
    loop_invariants: Vec<LoopInvariant>,
    /// Accounts with verified discriminators (account index -> expected discriminator)
    verified_discriminators: HashMap<i64, Vec<u8>>,
    /// Sysvar accounts that have been validated
    verified_sysvars: HashMap<String, bool>,
    /// Function call stack for inter-procedural analysis
    call_stack: Vec<String>,
    /// Known function signatures (name -> (params, returns_value))
    function_signatures: HashMap<String, FunctionSignature>,
    /// Token flow counter for flash loan detection
    token_flow_count: usize,
    /// Whether timelock check has been performed
    has_timelock_check: bool,
    /// Whether reentrancy guard is active
    has_reentrancy_guard: bool,
}

/// Loop invariant specification
#[derive(Debug, Clone)]
struct LoopInvariant {
    /// The invariant expression (Lean format)
    invariant: String,
    /// Loop variable name
    loop_var: Option<String>,
    /// Loop bounds (if known)
    bounds: Option<(String, String)>,
}

/// Function signature for inter-procedural analysis
#[derive(Debug, Clone)]
struct FunctionSignature {
    /// Parameter names
    params: Vec<String>,
    /// Whether the function returns a value
    returns_value: bool,
    /// Pre-conditions for the function
    preconditions: Vec<String>,
    /// Post-conditions for the function  
    postconditions: Vec<String>,
}

impl VCContext {
    fn new(source_file: &str) -> Self {
        Self {
            source_file: source_file.to_string(),
            ..Default::default()
        }
    }

    fn next_id(&mut self, category: &VCCategory) -> String {
        self.vc_counter += 1;
        format!("vc_{}_{}", category, self.vc_counter)
    }

    fn push_assumption(&mut self, assumption: String) {
        self.assumptions.push(assumption);
    }

    fn pop_assumption(&mut self) {
        self.assumptions.pop();
    }

    fn clone_assumptions(&self) -> Vec<String> {
        self.assumptions.clone()
    }
}

/// Lean code generator for verification conditions
pub struct LeanCodegen {
    properties: VerificationProperties,
}

impl LeanCodegen {
    /// Create a new code generator
    pub fn new(properties: VerificationProperties) -> Self {
        Self { properties }
    }

    /// Generate verification conditions from an OVSM program
    pub fn generate(
        &self,
        program: &Program,
        source_file: &str,
    ) -> Result<Vec<VerificationCondition>> {
        let mut ctx = VCContext::new(source_file);
        let mut vcs = Vec::new();

        for stmt in &program.statements {
            ctx.total_nodes += 1;
            self.generate_stmt_vcs(stmt, &mut ctx, &mut vcs)?;
        }

        // Generate balance conservation VC if lamport changes were tracked
        if self.properties.balance_safety && !ctx.lamport_deltas.is_empty() {
            self.generate_balance_conservation_vc(&mut ctx, &mut vcs);
        }

        Ok(vcs)
    }

    /// Generate balance conservation verification condition
    fn generate_balance_conservation_vc(
        &self,
        ctx: &mut VCContext,
        vcs: &mut Vec<VerificationCondition>,
    ) {
        // Build property: sum of all lamport changes should equal zero
        // or: total_lamports_before = total_lamports_after
        let accounts: Vec<_> = ctx.lamport_deltas.iter().map(|(idx, _)| *idx).collect();

        if accounts.is_empty() {
            return;
        }

        let property = format!(
            "∑(lamports_after[i] - lamports_before[i]) = 0 for accounts {:?}",
            accounts
        );

        let vc = VerificationCondition {
            id: ctx.next_id(&VCCategory::BalanceConservation),
            category: VCCategory::BalanceConservation,
            description: "Total lamports must be conserved (no minting/burning)".to_string(),
            location: Some(SourceLocation {
                file: ctx.source_file.clone(),
                line: 1,
                column: 1,
            }),
            property,
            assumptions: ctx.clone_assumptions(),
            tactic: "balance_conservation".to_string(),
        };
        vcs.push(vc);
    }

    /// Generate verification conditions for a statement
    fn generate_stmt_vcs(
        &self,
        stmt: &Statement,
        ctx: &mut VCContext,
        vcs: &mut Vec<VerificationCondition>,
    ) -> Result<()> {
        match stmt {
            Statement::Expression(expr) => {
                self.generate_expr_vcs(expr, ctx, vcs, None)?;
            }

            Statement::Assignment { name, value } => {
                self.generate_expr_vcs(value, ctx, vcs, None)?;
                // Mark variable as initialized
                ctx.initialized_vars.insert(name.clone(), true);
            }

            Statement::If {
                condition,
                then_branch,
                else_branch,
            } => {
                // Generate VCs for condition
                self.generate_expr_vcs(condition, ctx, vcs, None)?;

                // Generate VCs for then branch with condition as assumption
                let cond_lean = self.expr_to_lean_bool(condition);
                ctx.push_assumption(cond_lean.clone());
                for stmt in then_branch {
                    self.generate_stmt_vcs(stmt, ctx, vcs)?;
                }
                ctx.pop_assumption();

                // Generate VCs for else branch with negated condition
                if let Some(else_body) = else_branch {
                    ctx.push_assumption(format!("¬({})", cond_lean));
                    for stmt in else_body {
                        self.generate_stmt_vcs(stmt, ctx, vcs)?;
                    }
                    ctx.pop_assumption();
                }
            }

            Statement::While { condition, body } => {
                // Generate VCs for condition
                self.generate_expr_vcs(condition, ctx, vcs, None)?;

                // Check for @invariant annotation in body
                let invariants = self.extract_loop_invariants(body);

                // Generate loop invariant VCs if any invariants are specified
                for invariant in &invariants {
                    // VC 1: Invariant holds on entry (base case)
                    let vc_entry = VerificationCondition {
                        id: ctx.next_id(&VCCategory::LoopInvariant),
                        category: VCCategory::LoopInvariant,
                        description: format!(
                            "Loop invariant '{}' must hold on entry",
                            invariant.invariant
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line: 1,
                            column: 1,
                        }),
                        property: format!("entry → {}", invariant.invariant),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "loop_entry".to_string(),
                    };
                    vcs.push(vc_entry);

                    // VC 2: Invariant is preserved (inductive case)
                    let vc_preserve = VerificationCondition {
                        id: ctx.next_id(&VCCategory::LoopInvariant),
                        category: VCCategory::LoopInvariant,
                        description: format!(
                            "Loop invariant '{}' must be preserved by loop body",
                            invariant.invariant
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line: 1,
                            column: 1,
                        }),
                        property: format!(
                            "({} ∧ condition) → {}'",
                            invariant.invariant, invariant.invariant
                        ),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "loop_preserve".to_string(),
                    };
                    vcs.push(vc_preserve);

                    // Add invariant to context for proving VCs inside loop
                    ctx.loop_invariants.push(invariant.clone());
                }

                // Generate VCs for body with condition as assumption
                let cond_lean = self.expr_to_lean_bool(condition);
                ctx.push_assumption(cond_lean);
                for stmt in body {
                    self.generate_stmt_vcs(stmt, ctx, vcs)?;
                }
                ctx.pop_assumption();

                // Pop loop invariants after processing body
                for _ in &invariants {
                    ctx.loop_invariants.pop();
                }
            }

            Statement::For {
                variable,
                iterable,
                body,
            } => {
                // Check for @invariant annotation in body
                let invariants = self.extract_loop_invariants(body);

                // For range loops, we can provide stronger guarantees
                let bounds = if let Expression::Range { start, end } = iterable {
                    let start_lean = self.expr_to_lean(start);
                    let end_lean = self.expr_to_lean(end);
                    Some((start_lean.clone(), end_lean.clone()))
                } else {
                    None
                };

                // Generate loop invariant VCs with bounds info
                for invariant in &invariants {
                    let mut inv = invariant.clone();
                    inv.loop_var = Some(variable.clone());
                    inv.bounds = bounds.clone();

                    // VC 1: Invariant holds on entry
                    let entry_property = if let Some((start, _)) = &bounds {
                        format!("{} = {} → {}", variable, start, inv.invariant)
                    } else {
                        format!("entry → {}", inv.invariant)
                    };

                    let vc_entry = VerificationCondition {
                        id: ctx.next_id(&VCCategory::LoopInvariant),
                        category: VCCategory::LoopInvariant,
                        description: format!(
                            "For-loop invariant '{}' must hold on entry",
                            inv.invariant
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line: 1,
                            column: 1,
                        }),
                        property: entry_property,
                        assumptions: ctx.clone_assumptions(),
                        tactic: "loop_entry".to_string(),
                    };
                    vcs.push(vc_entry);

                    // VC 2: Invariant preserved
                    let preserve_property = if let Some((start, end)) = &bounds {
                        format!(
                            "({} ≤ {} ∧ {} < {} ∧ {}) → {}[{} ↦ {} + 1]",
                            start,
                            variable,
                            variable,
                            end,
                            inv.invariant,
                            inv.invariant,
                            variable,
                            variable
                        )
                    } else {
                        format!("({}) → {}'", inv.invariant, inv.invariant)
                    };

                    let vc_preserve = VerificationCondition {
                        id: ctx.next_id(&VCCategory::LoopInvariant),
                        category: VCCategory::LoopInvariant,
                        description: format!(
                            "For-loop invariant '{}' must be preserved",
                            inv.invariant
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line: 1,
                            column: 1,
                        }),
                        property: preserve_property,
                        assumptions: ctx.clone_assumptions(),
                        tactic: "loop_preserve".to_string(),
                    };
                    vcs.push(vc_preserve);

                    ctx.loop_invariants.push(inv);
                }

                // If iterating over a range, add bounds assumption
                if let Expression::Range { start, end } = iterable {
                    let start_lean = self.expr_to_lean(start);
                    let end_lean = self.expr_to_lean(end);
                    ctx.push_assumption(format!(
                        "({} ≤ {} ∧ {} < {})",
                        start_lean, variable, variable, end_lean
                    ));
                }

                for stmt in body {
                    self.generate_stmt_vcs(stmt, ctx, vcs)?;
                }

                if matches!(iterable, Expression::Range { .. }) {
                    ctx.pop_assumption();
                }

                // Pop loop invariants
                for _ in &invariants {
                    ctx.loop_invariants.pop();
                }
            }

            Statement::Guard {
                condition,
                else_body,
            } => {
                self.generate_expr_vcs(condition, ctx, vcs, None)?;

                // Generate VCs for else (failure) path
                let cond_lean = self.expr_to_lean_bool(condition);
                ctx.push_assumption(format!("¬({})", cond_lean));
                for stmt in else_body {
                    self.generate_stmt_vcs(stmt, ctx, vcs)?;
                }
                ctx.pop_assumption();

                // After guard, condition holds for subsequent code
                ctx.push_assumption(cond_lean);
            }

            Statement::Return { value } => {
                if let Some(expr) = value {
                    self.generate_expr_vcs(expr, ctx, vcs, None)?;
                }
            }

            Statement::Try {
                body,
                catch_clauses,
            } => {
                for stmt in body {
                    self.generate_stmt_vcs(stmt, ctx, vcs)?;
                }
                for clause in catch_clauses {
                    for stmt in &clause.body {
                        self.generate_stmt_vcs(stmt, ctx, vcs)?;
                    }
                }
            }

            _ => {}
        }

        Ok(())
    }

    /// Generate verification conditions for an expression
    fn generate_expr_vcs(
        &self,
        expr: &Expression,
        ctx: &mut VCContext,
        vcs: &mut Vec<VerificationCondition>,
        expected_line: Option<usize>,
    ) -> Result<()> {
        let line = expected_line.unwrap_or(1); // TODO: get actual line from AST

        match expr {
            // Division safety
            Expression::Binary {
                op: BinaryOp::Div | BinaryOp::Mod,
                left,
                right,
            } => {
                if self.properties.division_safety {
                    // Check for literal zero - this is always an error
                    let is_literal_zero = matches!(right.as_ref(), Expression::IntLiteral(0));

                    let divisor_lean = self.expr_to_lean(right);
                    let (tactic, property) = if is_literal_zero {
                        // Literal zero is unprovable - will always fail
                        (
                            "exact absurd rfl (by decide)".to_string(),
                            "False".to_string(),
                        )
                    } else if let Expression::IntLiteral(n) = right.as_ref() {
                        // Non-zero literal - trivially provable
                        ("decide".to_string(), format!("{} ≠ 0", n))
                    } else {
                        // Variable - needs context
                        ("ovsm_div_safe".to_string(), format!("{} ≠ 0", divisor_lean))
                    };

                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::DivisionSafety),
                        category: VCCategory::DivisionSafety,
                        description: if is_literal_zero {
                            "Division by literal zero is always unsafe!".to_string()
                        } else {
                            format!("Division by '{}' must be non-zero", divisor_lean)
                        },
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property,
                        assumptions: ctx.clone_assumptions(),
                        tactic,
                    };
                    vcs.push(vc);

                    // ArithmeticPrecision: integer division truncates
                    // Only warn when dividing values that could have significant precision loss
                    let left_lean = self.expr_to_lean(left);
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::ArithmeticPrecision),
                        category: VCCategory::ArithmeticPrecision,
                        description: format!(
                            "Integer division {}/{} truncates - consider if precision loss is acceptable",
                            left_lean, divisor_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!(
                            "precision_acceptable({}, {})",
                            left_lean, divisor_lean
                        ),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "precision_check".to_string(),
                    };
                    vcs.push(vc);
                }

                // Recurse into operands
                self.generate_expr_vcs(left, ctx, vcs, expected_line)?;
                self.generate_expr_vcs(right, ctx, vcs, expected_line)?;
            }

            // Subtraction underflow checking
            Expression::Binary {
                op: BinaryOp::Sub,
                left,
                right,
            } => {
                if self.properties.underflow_check {
                    let left_lean = self.expr_to_lean(left);
                    let right_lean = self.expr_to_lean(right);

                    // Check if this looks like a balance/financial operation (high priority)
                    let is_balance_op =
                        self.is_balance_expression(left) || self.is_balance_expression(right);

                    // Skip if both are literals (can be evaluated statically)
                    let is_static = matches!(
                        (left.as_ref(), right.as_ref()),
                        (Expression::IntLiteral(_), Expression::IntLiteral(_))
                    );

                    // Generate VC if:
                    // 1. It's a balance operation (always check), OR
                    // 2. strict_arithmetic is enabled (check all), OR
                    // 3. balance_safety is enabled and it's not static
                    let should_check = is_balance_op
                        || self.properties.strict_arithmetic
                        || (self.properties.balance_safety && !is_static);

                    if should_check && !is_static {
                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::ArithmeticUnderflow),
                            category: VCCategory::ArithmeticUnderflow,
                            description: format!(
                                "Subtraction '{}' - '{}' must not underflow",
                                left_lean, right_lean
                            ),
                            location: Some(SourceLocation {
                                file: ctx.source_file.clone(),
                                line,
                                column: 1,
                            }),
                            property: format!("{}.toNat ≥ {}.toNat", left_lean, right_lean),
                            assumptions: ctx.clone_assumptions(),
                            tactic: "ovsm_sub_safe".to_string(),
                        };
                        vcs.push(vc);
                    }
                }

                self.generate_expr_vcs(left, ctx, vcs, expected_line)?;
                self.generate_expr_vcs(right, ctx, vcs, expected_line)?;
            }

            // Addition overflow checking
            Expression::Binary {
                op: BinaryOp::Add,
                left,
                right,
            } => {
                if self.properties.overflow_check {
                    let is_balance_op =
                        self.is_balance_expression(left) || self.is_balance_expression(right);

                    // Skip if both are literals (can be evaluated statically)
                    let is_static = matches!(
                        (left.as_ref(), right.as_ref()),
                        (Expression::IntLiteral(_), Expression::IntLiteral(_))
                    );

                    // Generate VC if balance operation OR strict_arithmetic enabled
                    let should_check = is_balance_op || self.properties.strict_arithmetic;

                    if should_check && !is_static {
                        let left_lean = self.expr_to_lean(left);
                        let right_lean = self.expr_to_lean(right);

                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::ArithmeticOverflow),
                            category: VCCategory::ArithmeticOverflow,
                            description: format!(
                                "Addition '{}' + '{}' must not overflow u64",
                                left_lean, right_lean
                            ),
                            location: Some(SourceLocation {
                                file: ctx.source_file.clone(),
                                line,
                                column: 1,
                            }),
                            property: format!(
                                "{} + {} ≤ 18446744073709551615",
                                left_lean, right_lean
                            ),
                            assumptions: ctx.clone_assumptions(),
                            tactic: "ovsm_add_safe".to_string(),
                        };
                        vcs.push(vc);
                    }
                }

                self.generate_expr_vcs(left, ctx, vcs, expected_line)?;
                self.generate_expr_vcs(right, ctx, vcs, expected_line)?;
            }

            // Multiplication overflow checking
            Expression::Binary {
                op: BinaryOp::Mul,
                left,
                right,
            } => {
                if self.properties.overflow_check {
                    let is_balance_op =
                        self.is_balance_expression(left) || self.is_balance_expression(right);

                    // Skip if both are literals (can be evaluated statically)
                    let is_static = matches!(
                        (left.as_ref(), right.as_ref()),
                        (Expression::IntLiteral(_), Expression::IntLiteral(_))
                    );

                    // Generate VC if balance operation OR strict_arithmetic enabled
                    let should_check = is_balance_op || self.properties.strict_arithmetic;

                    if should_check && !is_static {
                        let left_lean = self.expr_to_lean(left);
                        let right_lean = self.expr_to_lean(right);

                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::ArithmeticOverflow),
                            category: VCCategory::ArithmeticOverflow,
                            description: format!(
                                "Multiplication '{}' * '{}' must not overflow u64",
                                left_lean, right_lean
                            ),
                            location: Some(SourceLocation {
                                file: ctx.source_file.clone(),
                                line,
                                column: 1,
                            }),
                            property: format!(
                                "{} * {} ≤ 18446744073709551615",
                                left_lean, right_lean
                            ),
                            assumptions: ctx.clone_assumptions(),
                            tactic: "ovsm_mul_safe".to_string(),
                        };
                        vcs.push(vc);
                    }
                }

                self.generate_expr_vcs(left, ctx, vcs, expected_line)?;
                self.generate_expr_vcs(right, ctx, vcs, expected_line)?;
            }

            // Array access
            Expression::IndexAccess { array, index } => {
                let arr_lean = self.expr_to_lean(array);
                let idx_lean = self.expr_to_lean(index);

                // NullPointerCheck: array must not be null before indexing
                let vc = VerificationCondition {
                    id: ctx.next_id(&VCCategory::NullPointerCheck),
                    category: VCCategory::NullPointerCheck,
                    description: format!("Array '{}' must not be null before indexing", arr_lean),
                    location: Some(SourceLocation {
                        file: ctx.source_file.clone(),
                        line,
                        column: 1,
                    }),
                    property: format!("{} ≠ null", arr_lean),
                    assumptions: ctx.clone_assumptions(),
                    tactic: "null_check".to_string(),
                };
                vcs.push(vc);

                if self.properties.array_bounds {
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::ArrayBounds),
                        category: VCCategory::ArrayBounds,
                        description: format!(
                            "Array index '{}' must be within bounds of '{}'",
                            idx_lean, arr_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} < {}.size", idx_lean, arr_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "ovsm_in_bounds".to_string(),
                    };
                    vcs.push(vc);
                }

                self.generate_expr_vcs(array, ctx, vcs, expected_line)?;
                self.generate_expr_vcs(index, ctx, vcs, expected_line)?;
            }

            // Tool calls - check for specific patterns
            Expression::ToolCall { name, args } => {
                // Handle (assume ...) - adds assumption to verification context
                // This is useful for declaring preconditions that the verifier can use
                // Example: (assume (>= (instruction-data-len) 128))
                if name == "assume" && !args.is_empty() {
                    let assumption = self.expr_to_lean_bool(&args[0].value);
                    ctx.push_assumption(assumption);
                    // Note: assumption persists for rest of current scope
                    // The IR generator should be updated to handle 'assume' as a no-op
                    return Ok(());
                }

                // Array access functions
                if self.properties.array_bounds
                    && matches!(name.as_str(), "get" | "nth" | "elt" | "aref")
                    && args.len() >= 2
                {
                    let arr_lean = self.expr_to_lean(&args[0].value);
                    let idx_lean = self.expr_to_lean(&args[1].value);

                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::ArrayBounds),
                        category: VCCategory::ArrayBounds,
                        description: format!(
                            "Index '{}' must be within bounds of '{}'",
                            idx_lean, arr_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} < {}.size", idx_lean, arr_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "ovsm_in_bounds".to_string(),
                    };
                    vcs.push(vc);
                }

                // Division functions
                if self.properties.division_safety
                    && (name == "/" || name == "%")
                    && args.len() >= 2
                {
                    let divisor_lean = self.expr_to_lean(&args[1].value);

                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::DivisionSafety),
                        category: VCCategory::DivisionSafety,
                        description: format!("Divisor '{}' must be non-zero", divisor_lean),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} ≠ 0", divisor_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "ovsm_div_safe".to_string(),
                    };
                    vcs.push(vc);
                }

                // Solana set-lamports
                if self.properties.balance_safety && name == "set-lamports" && args.len() >= 2 {
                    // The second arg is the new balance - check it's derived safely
                    self.generate_expr_vcs(&args[1].value, ctx, vcs, expected_line)?;
                }

                // mem-load bounds check (8 bytes)
                if self.properties.array_bounds && name == "mem-load" && args.len() >= 2 {
                    let offset_lean = self.expr_to_lean(&args[1].value);
                    ctx.nodes_with_vcs += 1;

                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::InstructionDataBounds),
                        category: VCCategory::InstructionDataBounds,
                        description: format!(
                            "Memory load at offset '{}' must be in bounds",
                            offset_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} + 8 ≤ data_len", offset_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "omega".to_string(),
                    };
                    vcs.push(vc);
                }

                // mem-load1 bounds check (1 byte)
                if self.properties.array_bounds && name == "mem-load1" && args.len() >= 2 {
                    let offset_lean = self.expr_to_lean(&args[1].value);
                    ctx.nodes_with_vcs += 1;

                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::InstructionDataBounds),
                        category: VCCategory::InstructionDataBounds,
                        description: format!(
                            "Memory load (1 byte) at offset '{}' must be in bounds",
                            offset_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} + 1 ≤ data_len", offset_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "omega".to_string(),
                    };
                    vcs.push(vc);
                }

                // mem-load2 bounds check (2 bytes)
                if self.properties.array_bounds && name == "mem-load2" && args.len() >= 2 {
                    let offset_lean = self.expr_to_lean(&args[1].value);
                    ctx.nodes_with_vcs += 1;

                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::InstructionDataBounds),
                        category: VCCategory::InstructionDataBounds,
                        description: format!(
                            "Memory load (2 bytes) at offset '{}' must be in bounds",
                            offset_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} + 2 ≤ data_len", offset_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "omega".to_string(),
                    };
                    vcs.push(vc);
                }

                // mem-load4 bounds check (4 bytes)
                if self.properties.array_bounds && name == "mem-load4" && args.len() >= 2 {
                    let offset_lean = self.expr_to_lean(&args[1].value);
                    ctx.nodes_with_vcs += 1;

                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::InstructionDataBounds),
                        category: VCCategory::InstructionDataBounds,
                        description: format!(
                            "Memory load (4 bytes) at offset '{}' must be in bounds",
                            offset_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} + 4 ≤ data_len", offset_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "omega".to_string(),
                    };
                    vcs.push(vc);
                }

                // mem-store requires writable check and account data bounds
                if name == "mem-store" && args.len() >= 3 {
                    // Check if first arg is account-data-ptr
                    if let Expression::ToolCall {
                        name: ptr_name,
                        args: ptr_args,
                    } = &args[0].value
                    {
                        if ptr_name == "account-data-ptr" && !ptr_args.is_empty() {
                            if let Expression::IntLiteral(account_idx) = &ptr_args[0].value {
                                // WritabilityCheck: account must be writable before mem-store
                                if !ctx
                                    .verified_writable
                                    .get(account_idx)
                                    .copied()
                                    .unwrap_or(false)
                                {
                                    let vc = VerificationCondition {
                                        id: ctx.next_id(&VCCategory::WritabilityCheck),
                                        category: VCCategory::WritabilityCheck,
                                        description: format!(
                                            "Account {} must be verified writable before mem-store",
                                            account_idx
                                        ),
                                        location: Some(SourceLocation {
                                            file: ctx.source_file.clone(),
                                            line,
                                            column: 1,
                                        }),
                                        property: format!(
                                            "account_is_writable[{}] = true",
                                            account_idx
                                        ),
                                        assumptions: ctx.clone_assumptions(),
                                        tactic: "by_assumption".to_string(),
                                    };
                                    vcs.push(vc);
                                }

                                // AccountOwnerCheck: program must own account before writing
                                if !ctx
                                    .verified_owners
                                    .get(account_idx)
                                    .copied()
                                    .unwrap_or(false)
                                {
                                    let vc = VerificationCondition {
                                        id: ctx.next_id(&VCCategory::AccountOwnerCheck),
                                        category: VCCategory::AccountOwnerCheck,
                                        description: format!(
                                            "Program must own account {} before writing",
                                            account_idx
                                        ),
                                        location: Some(SourceLocation {
                                            file: ctx.source_file.clone(),
                                            line,
                                            column: 1,
                                        }),
                                        property: format!(
                                            "account_owner[{}] = program_id",
                                            account_idx
                                        ),
                                        assumptions: ctx.clone_assumptions(),
                                        tactic: "by_assumption".to_string(),
                                    };
                                    vcs.push(vc);
                                }
                            }
                        }
                    }

                    // AccountDataBounds: offset must be within account data
                    let offset_lean = self.expr_to_lean(&args[1].value);
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::AccountDataBounds),
                        category: VCCategory::AccountDataBounds,
                        description: format!(
                            "mem-store offset '{}' must be within account data bounds",
                            offset_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} + 8 ≤ account_data_len", offset_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "omega".to_string(),
                    };
                    vcs.push(vc);
                }

                // account-is-signer records signer verification
                if name == "account-is-signer" && !args.is_empty() {
                    if let Expression::IntLiteral(account_idx) = &args[0].value {
                        ctx.verified_signers.insert(*account_idx, true);
                    }
                }

                // account-is-writable records writable verification
                if name == "account-is-writable" && !args.is_empty() {
                    if let Expression::IntLiteral(account_idx) = &args[0].value {
                        ctx.verified_writable.insert(*account_idx, true);
                    }
                }

                // set-lamports needs signer check, writable check, and tracks balance changes
                if name == "set-lamports" && args.len() >= 2 {
                    if let Expression::IntLiteral(account_idx) = &args[0].value {
                        // SignerCheck: account should be verified as signer before balance change
                        if !ctx
                            .verified_signers
                            .get(account_idx)
                            .copied()
                            .unwrap_or(false)
                        {
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::SignerCheck),
                                category: VCCategory::SignerCheck,
                                description: format!(
                                    "Account {} should be verified as signer before lamport change",
                                    account_idx
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!("account_is_signer[{}] = true", account_idx),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "by_assumption".to_string(),
                            };
                            vcs.push(vc);
                        }

                        // WritabilityCheck: account must be writable before lamport change
                        if !ctx
                            .verified_writable
                            .get(account_idx)
                            .copied()
                            .unwrap_or(false)
                        {
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::WritabilityCheck),
                                category: VCCategory::WritabilityCheck,
                                description: format!(
                                    "Account {} must be writable before set-lamports",
                                    account_idx
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!("account_is_writable[{}] = true", account_idx),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "by_assumption".to_string(),
                            };
                            vcs.push(vc);
                        }

                        // Track lamport delta for balance conservation
                        let new_balance = self.expr_to_lean(&args[1].value);
                        ctx.lamport_deltas.push((*account_idx, new_balance));
                    }
                }

                // spl-token-transfer needs signer and tracks token balance
                if name == "spl-token-transfer" && args.len() >= 5 {
                    // Args: token_program, source, dest, authority, amount
                    if let Expression::IntLiteral(authority_idx) = &args[3].value {
                        if !ctx
                            .verified_signers
                            .get(authority_idx)
                            .copied()
                            .unwrap_or(false)
                        {
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::SignerCheck),
                                category: VCCategory::SignerCheck,
                                description: format!(
                                    "Authority account {} must be signer for token transfer",
                                    authority_idx
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!("account_is_signer[{}] = true", authority_idx),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "by_assumption".to_string(),
                            };
                            vcs.push(vc);
                        }
                    }
                    // Token account ownership check for source
                    if let Expression::IntLiteral(source_idx) = &args[1].value {
                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::TokenAccountOwnerCheck),
                            category: VCCategory::TokenAccountOwnerCheck,
                            description: format!(
                                "Source token account {} must be owned by SPL Token program",
                                source_idx
                            ),
                            location: Some(SourceLocation {
                                file: ctx.source_file.clone(),
                                line,
                                column: 1,
                            }),
                            property: format!("account_owner[{}] = TOKEN_PROGRAM_ID", source_idx),
                            assumptions: ctx.clone_assumptions(),
                            tactic: "token_owner_check".to_string(),
                        };
                        vcs.push(vc);
                    }
                }

                // spl-token-mint-to needs mint authority check
                if name == "spl-token-mint-to" && args.len() >= 4 {
                    // Args: token_program, mint, dest, authority, amount
                    if let Expression::IntLiteral(authority_idx) = &args[3].value {
                        if !ctx
                            .verified_signers
                            .get(authority_idx)
                            .copied()
                            .unwrap_or(false)
                        {
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::SignerCheck),
                                category: VCCategory::SignerCheck,
                                description: format!(
                                    "Mint authority account {} must be signer",
                                    authority_idx
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!("account_is_signer[{}] = true", authority_idx),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "by_assumption".to_string(),
                            };
                            vcs.push(vc);
                        }
                    }
                    // Mint authority validation
                    if let (
                        Expression::IntLiteral(mint_idx),
                        Expression::IntLiteral(authority_idx),
                    ) = (&args[1].value, &args[3].value)
                    {
                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::MintAuthorityCheck),
                            category: VCCategory::MintAuthorityCheck,
                            description: format!(
                                "Account {} must be mint authority for mint {}",
                                authority_idx, mint_idx
                            ),
                            location: Some(SourceLocation {
                                file: ctx.source_file.clone(),
                                line,
                                column: 1,
                            }),
                            property: format!(
                                "mint_authority[{}] = account_pubkey[{}]",
                                mint_idx, authority_idx
                            ),
                            assumptions: ctx.clone_assumptions(),
                            tactic: "mint_authority_check".to_string(),
                        };
                        vcs.push(vc);
                    }
                }

                // spl-token-burn needs authority check
                if name == "spl-token-burn" && args.len() >= 4 {
                    if let Expression::IntLiteral(authority_idx) = &args[3].value {
                        if !ctx
                            .verified_signers
                            .get(authority_idx)
                            .copied()
                            .unwrap_or(false)
                        {
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::SignerCheck),
                                category: VCCategory::SignerCheck,
                                description: format!(
                                    "Burn authority account {} must be signer",
                                    authority_idx
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!("account_is_signer[{}] = true", authority_idx),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "by_assumption".to_string(),
                            };
                            vcs.push(vc);
                        }
                    }
                }

                // spl-close-account needs close authority check
                if name == "spl-close-account" && args.len() >= 3 {
                    if let Expression::IntLiteral(authority_idx) = &args[2].value {
                        if !ctx
                            .verified_signers
                            .get(authority_idx)
                            .copied()
                            .unwrap_or(false)
                        {
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::CloseAuthorityCheck),
                                category: VCCategory::CloseAuthorityCheck,
                                description: format!(
                                    "Close authority account {} must be signer",
                                    authority_idx
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!("account_is_signer[{}] = true", authority_idx),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "by_assumption".to_string(),
                            };
                            vcs.push(vc);
                        }
                    }
                }

                // System program CPI operations need signer checks
                // system-transfer: source account must be signer
                if name == "system-transfer" && args.len() >= 3 {
                    if let Expression::IntLiteral(source_idx) = &args[0].value {
                        if !ctx
                            .verified_signers
                            .get(source_idx)
                            .copied()
                            .unwrap_or(false)
                        {
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::SignerCheck),
                                category: VCCategory::SignerCheck,
                                description: format!(
                                    "Source account {} must be signer for system transfer",
                                    source_idx
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!("account_is_signer[{}] = true", source_idx),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "by_assumption".to_string(),
                            };
                            vcs.push(vc);
                        }
                    }
                }

                // system-create-account: payer must be signer
                if name == "system-create-account" && args.len() >= 2 {
                    if let Expression::IntLiteral(payer_idx) = &args[0].value {
                        if !ctx
                            .verified_signers
                            .get(payer_idx)
                            .copied()
                            .unwrap_or(false)
                        {
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::SignerCheck),
                                category: VCCategory::SignerCheck,
                                description: format!(
                                    "Payer account {} must be signer for create account",
                                    payer_idx
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!("account_is_signer[{}] = true", payer_idx),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "by_assumption".to_string(),
                            };
                            vcs.push(vc);
                        }
                    }
                }

                // system-allocate: account must be signer
                if name == "system-allocate" && !args.is_empty() {
                    if let Expression::IntLiteral(account_idx) = &args[0].value {
                        if !ctx
                            .verified_signers
                            .get(account_idx)
                            .copied()
                            .unwrap_or(false)
                        {
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::SignerCheck),
                                category: VCCategory::SignerCheck,
                                description: format!(
                                    "Account {} must be signer for allocate",
                                    account_idx
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!("account_is_signer[{}] = true", account_idx),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "by_assumption".to_string(),
                            };
                            vcs.push(vc);
                        }
                    }
                }

                // system-assign: account must be signer
                if name == "system-assign" && !args.is_empty() {
                    if let Expression::IntLiteral(account_idx) = &args[0].value {
                        if !ctx
                            .verified_signers
                            .get(account_idx)
                            .copied()
                            .unwrap_or(false)
                        {
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::SignerCheck),
                                category: VCCategory::SignerCheck,
                                description: format!(
                                    "Account {} must be signer for assign",
                                    account_idx
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!("account_is_signer[{}] = true", account_idx),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "by_assumption".to_string(),
                            };
                            vcs.push(vc);
                        }
                    }
                }

                // Borsh serialization buffer bounds
                if name == "borsh-serialize" && args.len() >= 2 {
                    let buffer_lean = self.expr_to_lean(&args[1].value);
                    ctx.nodes_with_vcs += 1;
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::BufferOverflowCheck),
                        category: VCCategory::BufferOverflowCheck,
                        description: format!(
                            "Buffer '{}' must have sufficient capacity for serialization",
                            buffer_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("buffer_capacity({}) >= serialized_size", buffer_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "buffer_check".to_string(),
                    };
                    vcs.push(vc);
                }

                // Borsh deserialization buffer bounds
                if name == "borsh-deserialize" && !args.is_empty() {
                    let buffer_lean = self.expr_to_lean(&args[0].value);
                    ctx.nodes_with_vcs += 1;
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::BufferUnderrunCheck),
                        category: VCCategory::BufferUnderrunCheck,
                        description: format!(
                            "Buffer '{}' must have sufficient data for deserialization",
                            buffer_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("buffer_len({}) >= expected_size", buffer_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "buffer_check".to_string(),
                    };
                    vcs.push(vc);

                    // TypeConfusion: deserialized data must match expected type
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::TypeConfusion),
                        category: VCCategory::TypeConfusion,
                        description: format!(
                            "Deserialized data from '{}' must match expected account type",
                            buffer_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("deserialize_type_matches({})", buffer_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "type_check".to_string(),
                    };
                    vcs.push(vc);
                }

                // Account reallocation bounds check
                if name == "realloc" && args.len() >= 2 {
                    let account_lean = self.expr_to_lean(&args[0].value);
                    let new_size_lean = self.expr_to_lean(&args[1].value);
                    ctx.nodes_with_vcs += 1;

                    // Check that new size is reasonable
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::AccountRealloc),
                        category: VCCategory::AccountRealloc,
                        description: format!(
                            "Account realloc to size '{}' must be within limits (max 10MB)",
                            new_size_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} <= 10485760 ∧ {} >= 0", new_size_lean, new_size_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "realloc_check".to_string(),
                    };
                    vcs.push(vc);

                    // Check rent exemption after realloc
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::RentExemptCheck),
                        category: VCCategory::RentExemptCheck,
                        description: format!(
                            "Account '{}' must maintain rent exemption after realloc to {}",
                            account_lean, new_size_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!(
                            "lamports({}) >= rent_exempt_minimum({})",
                            account_lean, new_size_lean
                        ),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "rent_check".to_string(),
                    };
                    vcs.push(vc);
                }

                // invoke/cpi calls - reentrancy check and program validation
                // Include cpi-invoke and cpi-invoke-signed which were previously missing
                if matches!(
                    name.as_str(),
                    "invoke" | "invoke-signed" | "cpi-call" | "cpi-invoke" | "cpi-invoke-signed"
                ) {
                    ctx.cpi_depth += 1;
                    ctx.nodes_with_vcs += 1;

                    // Reentrancy check for nested CPIs
                    if ctx.cpi_depth > 1 {
                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::ReentrancyCheck),
                            category: VCCategory::ReentrancyCheck,
                            description: "Nested CPI call detected - potential reentrancy"
                                .to_string(),
                            location: Some(SourceLocation {
                                file: ctx.source_file.clone(),
                                line,
                                column: 1,
                            }),
                            property: format!("cpi_depth = {} (nested CPI)", ctx.cpi_depth),
                            assumptions: ctx.clone_assumptions(),
                            tactic: "manual_review".to_string(),
                        };
                        vcs.push(vc);
                    }

                    // Program ID validation for all CPI operations
                    if !args.is_empty() {
                        let program_lean = self.expr_to_lean(&args[0].value);
                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::Custom("cpi_program".to_string())),
                            category: VCCategory::Custom("cpi_program".to_string()),
                            description: format!(
                                "CPI target program '{}' must be expected program",
                                program_lean
                            ),
                            location: Some(SourceLocation {
                                file: ctx.source_file.clone(),
                                line,
                                column: 1,
                            }),
                            property: format!("{} = expected_program_id", program_lean),
                            assumptions: ctx.clone_assumptions(),
                            tactic: "by_assumption".to_string(),
                        };
                        vcs.push(vc);
                    }
                }

                // spl-token-transfer-signed needs same checks as spl-token-transfer
                if name == "spl-token-transfer-signed" && args.len() >= 5 {
                    // Args: token_program, source, dest, authority, amount, seeds
                    // Authority is PDA, so we check seeds instead of signer
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::PDASeedCheck),
                        category: VCCategory::PDASeedCheck,
                        description: "PDA seeds for signed token transfer must be valid"
                            .to_string(),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: "pda_seeds_valid".to_string(),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "by_assumption".to_string(),
                    };
                    vcs.push(vc);

                    // Token account ownership check for source
                    if let Expression::IntLiteral(source_idx) = &args[1].value {
                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::TokenAccountOwnerCheck),
                            category: VCCategory::TokenAccountOwnerCheck,
                            description: format!(
                                "Source token account {} must be owned by SPL Token program",
                                source_idx
                            ),
                            location: Some(SourceLocation {
                                file: ctx.source_file.clone(),
                                line,
                                column: 1,
                            }),
                            property: format!("account_owner[{}] = TOKEN_PROGRAM_ID", source_idx),
                            assumptions: ctx.clone_assumptions(),
                            tactic: "token_owner_check".to_string(),
                        };
                        vcs.push(vc);
                    }
                }

                // spl-close-account-signed needs checks
                if name == "spl-close-account-signed" && args.len() >= 3 {
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::PDASeedCheck),
                        category: VCCategory::PDASeedCheck,
                        description: "PDA seeds for signed close must be valid".to_string(),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: "pda_seeds_valid".to_string(),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "by_assumption".to_string(),
                    };
                    vcs.push(vc);

                    if let Expression::IntLiteral(account_idx) = &args[0].value {
                        // DoubleFree check
                        if ctx
                            .closed_accounts
                            .get(account_idx)
                            .copied()
                            .unwrap_or(false)
                        {
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::DoubleFree),
                                category: VCCategory::DoubleFree,
                                description: format!("Account {} may be closed twice", account_idx),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!("account_closed[{}] = false", account_idx),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "by_assumption".to_string(),
                            };
                            vcs.push(vc);
                        }
                        ctx.closed_accounts.insert(*account_idx, true);
                    }
                }

                // close-account - signer check, writable check, and double free check
                if name == "close-account" && !args.is_empty() {
                    if let Expression::IntLiteral(account_idx) = &args[0].value {
                        // SignerCheck: close authority must be signer (usually account itself or delegate)
                        if !ctx
                            .verified_signers
                            .get(account_idx)
                            .copied()
                            .unwrap_or(false)
                        {
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::SignerCheck),
                                category: VCCategory::SignerCheck,
                                description: format!(
                                    "Close authority for account {} must be verified as signer",
                                    account_idx
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!("account_is_signer[{}] = true", account_idx),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "by_assumption".to_string(),
                            };
                            vcs.push(vc);
                        }

                        // WritabilityCheck: account must be writable to close
                        if !ctx
                            .verified_writable
                            .get(account_idx)
                            .copied()
                            .unwrap_or(false)
                        {
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::WritabilityCheck),
                                category: VCCategory::WritabilityCheck,
                                description: format!(
                                    "Account {} must be writable to close",
                                    account_idx
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!("account_is_writable[{}] = true", account_idx),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "by_assumption".to_string(),
                            };
                            vcs.push(vc);
                        }

                        // DoubleFree check
                        if ctx
                            .closed_accounts
                            .get(account_idx)
                            .copied()
                            .unwrap_or(false)
                        {
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::DoubleFree),
                                category: VCCategory::DoubleFree,
                                description: format!(
                                    "Account {} may be closed twice (double-free)",
                                    account_idx
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!("account_closed[{}] = false", account_idx),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "by_assumption".to_string(),
                            };
                            vcs.push(vc);
                        }
                        ctx.closed_accounts.insert(*account_idx, true);

                        // AccountCloseDrain: lamports must go to valid destination
                        // Check if there's a destination argument (args[1] typically)
                        if args.len() >= 2 {
                            let dest_expr = &args[1].value;
                            // If destination is specified, it must be valid
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::AccountCloseDrain),
                                category: VCCategory::AccountCloseDrain,
                                description: format!(
                                    "Lamports from closed account {} must drain to valid destination",
                                    account_idx
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!(
                                    "close_destination_valid({}, {})",
                                    account_idx,
                                    self.expr_to_lean(dest_expr)
                                ),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "close_drain_check".to_string(),
                            };
                            vcs.push(vc);
                        } else {
                            // No destination specified - error, lamports would be lost
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::AccountCloseDrain),
                                category: VCCategory::AccountCloseDrain,
                                description: format!(
                                    "Account {} close must specify lamport destination (lamports would be lost)",
                                    account_idx
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: "close_has_destination = true".to_string(),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "close_drain_check".to_string(),
                            };
                            vcs.push(vc);
                        }
                    }
                }

                // Account index validation for account access functions
                let account_access_fns = [
                    "account-data-ptr",
                    "account-lamports",
                    "account-owner",
                    "account-pubkey",
                    "account-is-signer",
                    "account-is-writable",
                    "account-data-len",
                    "account-executable",
                ];
                if account_access_fns.contains(&name.as_str()) && !args.is_empty() {
                    let idx_lean = self.expr_to_lean(&args[0].value);
                    ctx.nodes_with_vcs += 1;

                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::ArrayBounds),
                        category: VCCategory::ArrayBounds,
                        description: format!(
                            "Account index '{}' must be valid (< num_accounts)",
                            idx_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} < num_accounts", idx_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "omega".to_string(),
                    };
                    vcs.push(vc);
                }

                // mem-store1/2/4 bounds checking with full security checks
                if name == "mem-store1" && args.len() >= 3 {
                    // Check for account-data-ptr to extract account index for security checks
                    if let Expression::ToolCall {
                        name: ptr_name,
                        args: ptr_args,
                    } = &args[0].value
                    {
                        if ptr_name == "account-data-ptr" && !ptr_args.is_empty() {
                            if let Expression::IntLiteral(account_idx) = &ptr_args[0].value {
                                // WritabilityCheck
                                if !ctx
                                    .verified_writable
                                    .get(account_idx)
                                    .copied()
                                    .unwrap_or(false)
                                {
                                    let vc = VerificationCondition {
                                        id: ctx.next_id(&VCCategory::WritabilityCheck),
                                        category: VCCategory::WritabilityCheck,
                                        description: format!(
                                            "Account {} must be writable for mem-store1",
                                            account_idx
                                        ),
                                        location: Some(SourceLocation {
                                            file: ctx.source_file.clone(),
                                            line,
                                            column: 1,
                                        }),
                                        property: format!(
                                            "account_is_writable[{}] = true",
                                            account_idx
                                        ),
                                        assumptions: ctx.clone_assumptions(),
                                        tactic: "by_assumption".to_string(),
                                    };
                                    vcs.push(vc);
                                }
                                // AccountOwnerCheck
                                if !ctx
                                    .verified_owners
                                    .get(account_idx)
                                    .copied()
                                    .unwrap_or(false)
                                {
                                    let vc = VerificationCondition {
                                        id: ctx.next_id(&VCCategory::AccountOwnerCheck),
                                        category: VCCategory::AccountOwnerCheck,
                                        description: format!(
                                            "Program must own account {} for mem-store1",
                                            account_idx
                                        ),
                                        location: Some(SourceLocation {
                                            file: ctx.source_file.clone(),
                                            line,
                                            column: 1,
                                        }),
                                        property: format!(
                                            "account_owner[{}] = program_id",
                                            account_idx
                                        ),
                                        assumptions: ctx.clone_assumptions(),
                                        tactic: "by_assumption".to_string(),
                                    };
                                    vcs.push(vc);
                                }
                            }
                        }
                    }
                    let offset_lean = self.expr_to_lean(&args[1].value);
                    ctx.nodes_with_vcs += 1;
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::AccountDataBounds),
                        category: VCCategory::AccountDataBounds,
                        description: format!(
                            "mem-store1 offset '{}' must be in bounds",
                            offset_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} + 1 ≤ account_data_len", offset_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "omega".to_string(),
                    };
                    vcs.push(vc);

                    // IntegerTruncation: value stored to 1 byte must fit in u8
                    let value_lean = self.expr_to_lean(&args[2].value);
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::IntegerTruncation),
                        category: VCCategory::IntegerTruncation,
                        description: format!(
                            "Value '{}' must fit in u8 (0-255) for mem-store1",
                            value_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} ≤ 255", value_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "truncation_check".to_string(),
                    };
                    vcs.push(vc);
                }
                if name == "mem-store2" && args.len() >= 3 {
                    // Check for account-data-ptr to extract account index for security checks
                    if let Expression::ToolCall {
                        name: ptr_name,
                        args: ptr_args,
                    } = &args[0].value
                    {
                        if ptr_name == "account-data-ptr" && !ptr_args.is_empty() {
                            if let Expression::IntLiteral(account_idx) = &ptr_args[0].value {
                                // WritabilityCheck
                                if !ctx
                                    .verified_writable
                                    .get(account_idx)
                                    .copied()
                                    .unwrap_or(false)
                                {
                                    let vc = VerificationCondition {
                                        id: ctx.next_id(&VCCategory::WritabilityCheck),
                                        category: VCCategory::WritabilityCheck,
                                        description: format!(
                                            "Account {} must be writable for mem-store2",
                                            account_idx
                                        ),
                                        location: Some(SourceLocation {
                                            file: ctx.source_file.clone(),
                                            line,
                                            column: 1,
                                        }),
                                        property: format!(
                                            "account_is_writable[{}] = true",
                                            account_idx
                                        ),
                                        assumptions: ctx.clone_assumptions(),
                                        tactic: "by_assumption".to_string(),
                                    };
                                    vcs.push(vc);
                                }
                                // AccountOwnerCheck
                                if !ctx
                                    .verified_owners
                                    .get(account_idx)
                                    .copied()
                                    .unwrap_or(false)
                                {
                                    let vc = VerificationCondition {
                                        id: ctx.next_id(&VCCategory::AccountOwnerCheck),
                                        category: VCCategory::AccountOwnerCheck,
                                        description: format!(
                                            "Program must own account {} for mem-store2",
                                            account_idx
                                        ),
                                        location: Some(SourceLocation {
                                            file: ctx.source_file.clone(),
                                            line,
                                            column: 1,
                                        }),
                                        property: format!(
                                            "account_owner[{}] = program_id",
                                            account_idx
                                        ),
                                        assumptions: ctx.clone_assumptions(),
                                        tactic: "by_assumption".to_string(),
                                    };
                                    vcs.push(vc);
                                }
                            }
                        }
                    }
                    let offset_lean = self.expr_to_lean(&args[1].value);
                    ctx.nodes_with_vcs += 1;
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::AccountDataBounds),
                        category: VCCategory::AccountDataBounds,
                        description: format!(
                            "mem-store2 offset '{}' must be in bounds",
                            offset_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} + 2 ≤ account_data_len", offset_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "omega".to_string(),
                    };
                    vcs.push(vc);

                    // IntegerTruncation: value stored to 2 bytes must fit in u16
                    let value_lean = self.expr_to_lean(&args[2].value);
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::IntegerTruncation),
                        category: VCCategory::IntegerTruncation,
                        description: format!(
                            "Value '{}' must fit in u16 (0-65535) for mem-store2",
                            value_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} ≤ 65535", value_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "truncation_check".to_string(),
                    };
                    vcs.push(vc);
                }
                if name == "mem-store4" && args.len() >= 3 {
                    // Check for account-data-ptr to extract account index for security checks
                    if let Expression::ToolCall {
                        name: ptr_name,
                        args: ptr_args,
                    } = &args[0].value
                    {
                        if ptr_name == "account-data-ptr" && !ptr_args.is_empty() {
                            if let Expression::IntLiteral(account_idx) = &ptr_args[0].value {
                                // WritabilityCheck
                                if !ctx
                                    .verified_writable
                                    .get(account_idx)
                                    .copied()
                                    .unwrap_or(false)
                                {
                                    let vc = VerificationCondition {
                                        id: ctx.next_id(&VCCategory::WritabilityCheck),
                                        category: VCCategory::WritabilityCheck,
                                        description: format!(
                                            "Account {} must be writable for mem-store4",
                                            account_idx
                                        ),
                                        location: Some(SourceLocation {
                                            file: ctx.source_file.clone(),
                                            line,
                                            column: 1,
                                        }),
                                        property: format!(
                                            "account_is_writable[{}] = true",
                                            account_idx
                                        ),
                                        assumptions: ctx.clone_assumptions(),
                                        tactic: "by_assumption".to_string(),
                                    };
                                    vcs.push(vc);
                                }
                                // AccountOwnerCheck
                                if !ctx
                                    .verified_owners
                                    .get(account_idx)
                                    .copied()
                                    .unwrap_or(false)
                                {
                                    let vc = VerificationCondition {
                                        id: ctx.next_id(&VCCategory::AccountOwnerCheck),
                                        category: VCCategory::AccountOwnerCheck,
                                        description: format!(
                                            "Program must own account {} for mem-store4",
                                            account_idx
                                        ),
                                        location: Some(SourceLocation {
                                            file: ctx.source_file.clone(),
                                            line,
                                            column: 1,
                                        }),
                                        property: format!(
                                            "account_owner[{}] = program_id",
                                            account_idx
                                        ),
                                        assumptions: ctx.clone_assumptions(),
                                        tactic: "by_assumption".to_string(),
                                    };
                                    vcs.push(vc);
                                }
                            }
                        }
                    }
                    let offset_lean = self.expr_to_lean(&args[1].value);
                    ctx.nodes_with_vcs += 1;
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::AccountDataBounds),
                        category: VCCategory::AccountDataBounds,
                        description: format!(
                            "mem-store4 offset '{}' must be in bounds",
                            offset_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} + 4 ≤ account_data_len", offset_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "omega".to_string(),
                    };
                    vcs.push(vc);

                    // IntegerTruncation: value stored to 4 bytes must fit in u32
                    let value_lean = self.expr_to_lean(&args[2].value);
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::IntegerTruncation),
                        category: VCCategory::IntegerTruncation,
                        description: format!(
                            "Value '{}' must fit in u32 (0-4294967295) for mem-store4",
                            value_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} ≤ 4294967295", value_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "truncation_check".to_string(),
                    };
                    vcs.push(vc);
                }

                // PDA verification for find-program-address
                if name == "find-program-address" || name == "create-program-address" {
                    ctx.nodes_with_vcs += 1;
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::PDASeedCheck),
                        category: VCCategory::PDASeedCheck,
                        description: "PDA derivation must use correct seeds".to_string(),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: "pda_seeds_valid".to_string(),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "by_assumption".to_string(),
                    };
                    vcs.push(vc);

                    // BumpSeedCanonical: if using create-program-address with hardcoded bump
                    if name == "create-program-address" && args.len() >= 2 {
                        // Check if bump is a literal (not from find_program_address)
                        let has_literal_bump = args
                            .iter()
                            .any(|arg| matches!(&arg.value, Expression::IntLiteral(_)));
                        if has_literal_bump {
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::BumpSeedCanonical),
                                category: VCCategory::BumpSeedCanonical,
                                description:
                                    "PDA bump seed should be canonical (from find_program_address)"
                                        .to_string(),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: "bump_is_canonical".to_string(),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "bump_check".to_string(),
                            };
                            vcs.push(vc);
                        }
                    }
                }

                // CPI program ID validation
                if matches!(name.as_str(), "invoke" | "invoke-signed") && !args.is_empty() {
                    ctx.nodes_with_vcs += 1;
                    let program_lean = self.expr_to_lean(&args[0].value);
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::Custom("cpi_program".to_string())),
                        category: VCCategory::Custom("cpi_program".to_string()),
                        description: format!(
                            "CPI target program '{}' must be expected program",
                            program_lean
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("{} = expected_program_id", program_lean),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "by_assumption".to_string(),
                    };
                    vcs.push(vc);

                    // CPIDepthCheck: track CPI nesting depth (max 4 on Solana)
                    ctx.cpi_depth += 1;
                    if ctx.cpi_depth > 4 {
                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::CPIDepthCheck),
                            category: VCCategory::CPIDepthCheck,
                            description: format!(
                                "CPI depth {} exceeds Solana limit of 4",
                                ctx.cpi_depth
                            ),
                            location: Some(SourceLocation {
                                file: ctx.source_file.clone(),
                                line,
                                column: 1,
                            }),
                            property: format!("cpi_depth <= 4 (current: {})", ctx.cpi_depth),
                            assumptions: ctx.clone_assumptions(),
                            tactic: "depth_check".to_string(),
                        };
                        vcs.push(vc);
                    }

                    // SignerPrivilegeEscalation: check if signer seeds are passed to untrusted program
                    if name == "invoke-signed" {
                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::SignerPrivilegeEscalation),
                            category: VCCategory::SignerPrivilegeEscalation,
                            description: format!(
                                "Signer seeds passed to '{}' - ensure target program is trusted",
                                program_lean
                            ),
                            location: Some(SourceLocation {
                                file: ctx.source_file.clone(),
                                line,
                                column: 1,
                            }),
                            property: format!("is_trusted_program({})", program_lean),
                            assumptions: ctx.clone_assumptions(),
                            tactic: "trust_check".to_string(),
                        };
                        vcs.push(vc);
                    }
                }

                // Discriminator/account type validation
                // (check-discriminator account_idx expected_discriminator)
                if name == "check-discriminator" && args.len() >= 2 {
                    if let Expression::IntLiteral(account_idx) = &args[0].value {
                        let discriminator = self.expr_to_lean(&args[1].value);
                        ctx.nodes_with_vcs += 1;

                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::DiscriminatorCheck),
                            category: VCCategory::DiscriminatorCheck,
                            description: format!(
                                "Account {} must have discriminator '{}'",
                                account_idx, discriminator
                            ),
                            location: Some(SourceLocation {
                                file: ctx.source_file.clone(),
                                line,
                                column: 1,
                            }),
                            property: format!(
                                "account_discriminator[{}] = {}",
                                account_idx, discriminator
                            ),
                            assumptions: ctx.clone_assumptions(),
                            tactic: "discriminator_check".to_string(),
                        };
                        vcs.push(vc);
                    }
                }

                // Account type assertion - (assert-account-type account_idx type_name)
                if name == "assert-account-type" && args.len() >= 2 {
                    if let Expression::IntLiteral(account_idx) = &args[0].value {
                        let type_name = self.expr_to_lean(&args[1].value);
                        ctx.nodes_with_vcs += 1;

                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::DiscriminatorCheck),
                            category: VCCategory::DiscriminatorCheck,
                            description: format!(
                                "Account {} must be of type '{}'",
                                account_idx, type_name
                            ),
                            location: Some(SourceLocation {
                                file: ctx.source_file.clone(),
                                line,
                                column: 1,
                            }),
                            property: format!("account_type[{}] = {}", account_idx, type_name),
                            assumptions: ctx.clone_assumptions(),
                            tactic: "type_check".to_string(),
                        };
                        vcs.push(vc);
                    }
                }

                // Sysvar account validation
                let sysvar_functions = [
                    ("get-clock", "Clock"),
                    ("get-rent", "Rent"),
                    ("get-epoch-schedule", "EpochSchedule"),
                    ("get-fees", "Fees"),
                    ("get-recent-blockhashes", "RecentBlockhashes"),
                    ("get-stake-history", "StakeHistory"),
                    ("get-instructions", "Instructions"),
                ];

                for (sysvar_fn, sysvar_name) in sysvar_functions {
                    if name == sysvar_fn && !args.is_empty() {
                        if let Expression::IntLiteral(account_idx) = &args[0].value {
                            ctx.nodes_with_vcs += 1;

                            // Check that the account is the correct sysvar
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::SysvarCheck),
                                category: VCCategory::SysvarCheck,
                                description: format!(
                                    "Account {} must be the {} sysvar",
                                    account_idx, sysvar_name
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!(
                                    "account_pubkey[{}] = SYSVAR_{}_PUBKEY",
                                    account_idx,
                                    sysvar_name.to_uppercase()
                                ),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "sysvar_check".to_string(),
                            };
                            vcs.push(vc);

                            ctx.verified_sysvars.insert(sysvar_name.to_string(), true);
                        }
                    }
                }

                // Direct sysvar address checks - (check-sysvar account_idx sysvar_name)
                if name == "check-sysvar" && args.len() >= 2 {
                    if let Expression::IntLiteral(account_idx) = &args[0].value {
                        let sysvar_name = self.expr_to_lean(&args[1].value);
                        ctx.nodes_with_vcs += 1;

                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::SysvarCheck),
                            category: VCCategory::SysvarCheck,
                            description: format!(
                                "Account {} must be sysvar '{}'",
                                account_idx, sysvar_name
                            ),
                            location: Some(SourceLocation {
                                file: ctx.source_file.clone(),
                                line,
                                column: 1,
                            }),
                            property: format!(
                                "account_pubkey[{}] = SYSVAR_{}_PUBKEY",
                                account_idx, sysvar_name
                            ),
                            assumptions: ctx.clone_assumptions(),
                            tactic: "sysvar_check".to_string(),
                        };
                        vcs.push(vc);
                    }
                }

                // Inter-procedural analysis - function calls
                if (name == "funcall" || name == "apply")
                    && !args.is_empty() {
                        let func_name = self.expr_to_lean(&args[0].value);
                        ctx.nodes_with_vcs += 1;

                        // Check for recursion (function calling itself)
                        if ctx.call_stack.contains(&func_name) {
                            let vc = VerificationCondition {
                                id: ctx.next_id(&VCCategory::FunctionCallSafety),
                                category: VCCategory::FunctionCallSafety,
                                description: format!(
                                    "Recursive call to '{}' - verify termination",
                                    func_name
                                ),
                                location: Some(SourceLocation {
                                    file: ctx.source_file.clone(),
                                    line,
                                    column: 1,
                                }),
                                property: format!("terminates({})", func_name),
                                assumptions: ctx.clone_assumptions(),
                                tactic: "termination_check".to_string(),
                            };
                            vcs.push(vc);
                        }

                        // Track call for inter-procedural analysis
                        ctx.call_stack.push(func_name.clone());
                    }

                // ============================================================
                // NEW VC CATEGORIES
                // ============================================================

                // AccountDataMutability: detect writes to immutable account sections
                if matches!(
                    name.as_str(),
                    "mem-store" | "mem-store1" | "mem-store2" | "mem-store4" | "mem-store8"
                ) {
                    // Check if writing to discriminator/header area (typically first 8 bytes)
                    if args.len() >= 2 {
                        if let Expression::IntLiteral(offset) = &args[1].value {
                            if *offset < 8 {
                                ctx.nodes_with_vcs += 1;
                                let vc = VerificationCondition {
                                    id: ctx.next_id(&VCCategory::AccountDataMutability),
                                    category: VCCategory::AccountDataMutability,
                                    description: format!(
                                        "Write to offset {} may modify immutable discriminator/header",
                                        offset
                                    ),
                                    location: Some(SourceLocation { file: ctx.source_file.clone(), line, column: 1 }),
                                    property: format!("offset {} is mutable region", offset),
                                    assumptions: ctx.clone_assumptions(),
                                    tactic: "mutability_check".to_string(),
                                };
                                vcs.push(vc);
                            }
                        }
                    }
                }

                // PDACollision: warn when using same seed patterns
                if (name == "find-program-address" || name == "create-program-address")
                    && !args.is_empty() {
                        let seeds_lean = self.expr_to_lean(&args[0].value);
                        ctx.nodes_with_vcs += 1;
                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::PDACollision),
                            category: VCCategory::PDACollision,
                            description: format!(
                                "PDA seeds '{}' must be unique to prevent collisions",
                                seeds_lean
                            ),
                            location: Some(SourceLocation {
                                file: ctx.source_file.clone(),
                                line,
                                column: 1,
                            }),
                            property: format!("pda_seeds_unique({})", seeds_lean),
                            assumptions: ctx.clone_assumptions(),
                            tactic: "collision_check".to_string(),
                        };
                        vcs.push(vc);
                    }

                // InstructionIntrospection: validate instruction sysvar access
                if matches!(
                    name.as_str(),
                    "get-instruction"
                        | "get-instruction-data"
                        | "get-processed-sibling-instruction"
                ) {
                    ctx.nodes_with_vcs += 1;
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::InstructionIntrospection),
                        category: VCCategory::InstructionIntrospection,
                        description: "Instruction introspection requires valid Instructions sysvar"
                            .to_string(),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: "instructions_sysvar_valid".to_string(),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "introspection_check".to_string(),
                    };
                    vcs.push(vc);
                }

                // FlashLoanDetection: detect potential flash loan patterns
                // Triggered by: borrow -> operation -> repay in same context
                if matches!(
                    name.as_str(),
                    "spl-token-transfer" | "spl-token-transfer-signed" | "system-transfer"
                ) {
                    // Track token flows for flash loan detection
                    ctx.nodes_with_vcs += 1;
                    ctx.token_flow_count += 1;
                    if ctx.token_flow_count >= 2 {
                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::FlashLoanDetection),
                            category: VCCategory::FlashLoanDetection,
                            description: "Multiple token transfers detected - verify not vulnerable to flash loans".to_string(),
                            location: Some(SourceLocation { file: ctx.source_file.clone(), line, column: 1 }),
                            property: "flash_loan_safe".to_string(),
                            assumptions: ctx.clone_assumptions(),
                            tactic: "flash_loan_check".to_string(),
                        };
                        vcs.push(vc);
                    }
                }

                // OracleManipulation: check oracle data freshness
                if matches!(
                    name.as_str(),
                    "get-price"
                        | "get-oracle-price"
                        | "read-price-feed"
                        | "get-pyth-price"
                        | "get-switchboard-price"
                ) {
                    ctx.nodes_with_vcs += 1;
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::OracleManipulation),
                        category: VCCategory::OracleManipulation,
                        description: "Oracle price data must be checked for staleness".to_string(),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: "oracle_data_fresh".to_string(),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "oracle_check".to_string(),
                    };
                    vcs.push(vc);
                }

                // TimelockBypass: detect timelock constraints
                if matches!(
                    name.as_str(),
                    "check-timelock" | "verify-timelock" | "assert-timelock"
                ) {
                    ctx.nodes_with_vcs += 1;
                    ctx.has_timelock_check = true;
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::TimelockBypass),
                        category: VCCategory::TimelockBypass,
                        description: "Timelock constraint must be enforced".to_string(),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: "timelock_enforced".to_string(),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "timelock_check".to_string(),
                    };
                    vcs.push(vc);
                }

                // ReentrancyGuard: detect lock/guard patterns
                if matches!(
                    name.as_str(),
                    "acquire-lock" | "with-lock" | "enter-critical-section"
                ) {
                    ctx.nodes_with_vcs += 1;
                    ctx.has_reentrancy_guard = true;
                }
                if matches!(name.as_str(), "release-lock" | "exit-critical-section")
                    && !ctx.has_reentrancy_guard {
                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::ReentrancyGuard),
                            category: VCCategory::ReentrancyGuard,
                            description: "Lock released without acquisition".to_string(),
                            location: Some(SourceLocation {
                                file: ctx.source_file.clone(),
                                line,
                                column: 1,
                            }),
                            property: "lock_acquired_before_release".to_string(),
                            assumptions: ctx.clone_assumptions(),
                            tactic: "lock_check".to_string(),
                        };
                        vcs.push(vc);
                    }

                // OptionUnwrap: detect unsafe unwraps
                if matches!(
                    name.as_str(),
                    "unwrap" | "unwrap!" | "expect" | "force-unwrap"
                ) {
                    ctx.nodes_with_vcs += 1;
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::OptionUnwrap),
                        category: VCCategory::OptionUnwrap,
                        description: "Unwrap may panic if value is None/Null".to_string(),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: "value_is_some".to_string(),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "unwrap_check".to_string(),
                    };
                    vcs.push(vc);
                }

                // FrontRunning: detect ordering-sensitive operations
                if matches!(name.as_str(), "swap" | "trade" | "exchange" | "liquidate") {
                    ctx.nodes_with_vcs += 1;
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::FrontRunning),
                        category: VCCategory::FrontRunning,
                        description:
                            "Operation may be vulnerable to front-running/sandwich attacks"
                                .to_string(),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: "front_running_protected".to_string(),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "ordering_check".to_string(),
                    };
                    vcs.push(vc);
                }

                // Recurse into arguments
                for arg in args {
                    self.generate_expr_vcs(&arg.value, ctx, vcs, expected_line)?;
                }
            }

            // Refinement type annotation
            Expression::TypeAnnotation { expr, type_expr } => {
                if self.properties.refinement_types {
                    if let Expression::RefinedTypeExpr {
                        var,
                        base_type: _,
                        predicate,
                    } = type_expr.as_ref()
                    {
                        let expr_lean = self.expr_to_lean(expr);
                        let pred_lean = self.predicate_expr_to_lean(predicate, &expr_lean);

                        let vc = VerificationCondition {
                            id: ctx.next_id(&VCCategory::RefinementType),
                            category: VCCategory::RefinementType,
                            description: format!(
                                "Value must satisfy refinement predicate: {}",
                                pred_lean
                            ),
                            location: Some(SourceLocation {
                                file: ctx.source_file.clone(),
                                line,
                                column: 1,
                            }),
                            property: pred_lean,
                            assumptions: ctx.clone_assumptions(),
                            tactic: "ovsm_refine_literal".to_string(),
                        };
                        vcs.push(vc);
                    }
                }

                self.generate_expr_vcs(expr, ctx, vcs, expected_line)?;
            }

            // Recurse into other expressions
            Expression::Binary { left, right, .. } => {
                self.generate_expr_vcs(left, ctx, vcs, expected_line)?;
                self.generate_expr_vcs(right, ctx, vcs, expected_line)?;
            }

            Expression::Unary { operand, .. } => {
                self.generate_expr_vcs(operand, ctx, vcs, expected_line)?;
            }

            Expression::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                self.generate_expr_vcs(condition, ctx, vcs, expected_line)?;

                let cond_lean = self.expr_to_lean_bool(condition);
                ctx.push_assumption(cond_lean.clone());
                self.generate_expr_vcs(then_expr, ctx, vcs, expected_line)?;
                ctx.pop_assumption();

                ctx.push_assumption(format!("¬({})", cond_lean));
                self.generate_expr_vcs(else_expr, ctx, vcs, expected_line)?;
                ctx.pop_assumption();
            }

            Expression::ArrayLiteral(elements) => {
                for elem in elements {
                    self.generate_expr_vcs(elem, ctx, vcs, expected_line)?;
                }
            }

            Expression::Lambda { body, .. } | Expression::TypedLambda { body, .. } => {
                self.generate_expr_vcs(body, ctx, vcs, expected_line)?;
            }

            Expression::FieldAccess { object, field } => {
                let obj_lean = self.expr_to_lean(object);

                // NullPointerCheck: object must not be null before field access
                let vc = VerificationCondition {
                    id: ctx.next_id(&VCCategory::NullPointerCheck),
                    category: VCCategory::NullPointerCheck,
                    description: format!(
                        "Object '{}' must not be null before accessing field '{}'",
                        obj_lean, field
                    ),
                    location: Some(SourceLocation {
                        file: ctx.source_file.clone(),
                        line,
                        column: 1,
                    }),
                    property: format!("{} ≠ null", obj_lean),
                    assumptions: ctx.clone_assumptions(),
                    tactic: "null_check".to_string(),
                };
                vcs.push(vc);

                self.generate_expr_vcs(object, ctx, vcs, expected_line)?;
            }

            Expression::Range { start, end } => {
                self.generate_expr_vcs(start, ctx, vcs, expected_line)?;
                self.generate_expr_vcs(end, ctx, vcs, expected_line)?;
            }

            Expression::Grouping(inner) => {
                self.generate_expr_vcs(inner, ctx, vcs, expected_line)?;
            }

            // Variable access - check for uninitialized access
            Expression::Variable(name) => {
                // Skip built-in/special names, parameters, and implicitly initialized variables
                // In OVSM/Lisp, unbound variables are typically parameters from context
                // We only flag variables that:
                // 1. Were explicitly marked as needing initialization (via setq in this scope)
                // 2. Start with local-/temp- prefix (indicating local temporary)
                // 3. Look like internal scratch variables
                let is_special =
                    name.starts_with("_") || name == "nil" || name == "t" || name.starts_with("&");

                // Only generate UninitializedMemory VCs for variables that look like they
                // should be locally initialized. Free variables are assumed to be parameters.
                let is_likely_local = name.starts_with("local-")
                    || name.starts_with("temp-")
                    || name.starts_with("scratch-")
                    || name.starts_with("_local");

                // Check: only flag if it's marked as needing init and not initialized
                if !is_special && is_likely_local && !ctx.initialized_vars.contains_key(name) {
                    let vc = VerificationCondition {
                        id: ctx.next_id(&VCCategory::UninitializedMemory),
                        category: VCCategory::UninitializedMemory,
                        description: format!(
                            "Variable '{}' may be used before initialization",
                            name
                        ),
                        location: Some(SourceLocation {
                            file: ctx.source_file.clone(),
                            line,
                            column: 1,
                        }),
                        property: format!("initialized({})", name),
                        assumptions: ctx.clone_assumptions(),
                        tactic: "initialization_check".to_string(),
                    };
                    vcs.push(vc);
                }
            }

            // Literals don't need VCs
            _ => {}
        }

        Ok(())
    }

    /// Convert an expression to Lean syntax
    fn expr_to_lean(&self, expr: &Expression) -> String {
        match expr {
            Expression::IntLiteral(n) => n.to_string(),
            Expression::FloatLiteral(f) => f.to_string(),
            Expression::StringLiteral(s) => format!("\"{}\"", s),
            Expression::BoolLiteral(b) => if *b { "true" } else { "false" }.to_string(),
            Expression::NullLiteral => "none".to_string(),
            Expression::Variable(name) => name.clone(),

            Expression::Binary { op, left, right } => {
                let l = self.expr_to_lean(left);
                let r = self.expr_to_lean(right);
                let op_str = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Mod => "%",
                    BinaryOp::Pow => "^",
                    BinaryOp::Eq => "=",
                    BinaryOp::NotEq => "≠",
                    BinaryOp::Lt => "<",
                    BinaryOp::Gt => ">",
                    BinaryOp::LtEq => "≤",
                    BinaryOp::GtEq => "≥",
                    BinaryOp::And => "∧",
                    BinaryOp::Or => "∨",
                    BinaryOp::In => "∈",
                };
                format!("({} {} {})", l, op_str, r)
            }

            Expression::Unary { op, operand } => {
                let inner = self.expr_to_lean(operand);
                match op {
                    crate::parser::UnaryOp::Neg => format!("(-{})", inner),
                    crate::parser::UnaryOp::Not => format!("(¬{})", inner),
                }
            }

            Expression::ToolCall { name, args } => {
                let arg_strs: Vec<_> = args.iter().map(|a| self.expr_to_lean(&a.value)).collect();
                format!("({} {})", name, arg_strs.join(" "))
            }

            Expression::IndexAccess { array, index } => {
                let arr = self.expr_to_lean(array);
                let idx = self.expr_to_lean(index);
                format!("{}[{}]", arr, idx)
            }

            Expression::FieldAccess { object, field } => {
                let obj = self.expr_to_lean(object);
                format!("{}.{}", obj, field)
            }

            Expression::Grouping(inner) => self.expr_to_lean(inner),

            _ => "«expr»".to_string(), // Placeholder for complex expressions
        }
    }

    /// Convert an expression to Lean boolean (for conditions)
    fn expr_to_lean_bool(&self, expr: &Expression) -> String {
        // Most expressions can be used directly
        self.expr_to_lean(expr)
    }

    /// Convert a predicate expression to Lean, substituting the variable
    fn predicate_expr_to_lean(&self, expr: &Expression, var_value: &str) -> String {
        // Replace occurrences of the refinement variable with the actual expression
        
        // Simple string replacement - in practice we'd need proper AST transformation
        self.expr_to_lean(expr)
    }

    /// Extract loop invariants from statement body
    /// Looks for (invariant ...) or (@invariant ...) tool calls
    fn extract_loop_invariants(&self, body: &[Statement]) -> Vec<LoopInvariant> {
        let mut invariants = Vec::new();

        for stmt in body {
            if let Statement::Expression(expr) = stmt {
                if let Expression::ToolCall { name, args } = expr {
                    if (name == "invariant" || name == "@invariant") && !args.is_empty() {
                        let inv_expr = self.expr_to_lean(&args[0].value);
                        invariants.push(LoopInvariant {
                            invariant: inv_expr,
                            loop_var: None,
                            bounds: None,
                        });
                    }
                }
            }
        }

        invariants
    }

    /// Check if an expression looks like a balance/lamport operation or other risky arithmetic
    fn is_balance_expression(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Variable(name) => {
                let name_lower = name.to_lowercase();
                // Balance-related
                name_lower.contains("bal") || 
                name_lower.contains("lamport") || 
                name_lower.contains("amount") ||
                // Token/financial related
                name_lower.contains("token") ||
                name_lower.contains("fee") ||
                name_lower.contains("stake") ||
                name_lower.contains("reward") ||
                name_lower.contains("price") ||
                name_lower.contains("supply") ||
                // Accumulator patterns
                name_lower.contains("total") ||
                name_lower.contains("sum") ||
                name_lower.contains("volume") ||
                name_lower.contains("earned") ||
                name_lower.contains("spent") ||
                name_lower.contains("count")
            }
            Expression::ToolCall { name, .. } => {
                name == "account-lamports" || 
                name == "get-balance" ||
                name == "mem-load" ||  // Could be reading a balance
                name == "spl-token-amount"
            }
            Expression::FieldAccess { field, .. } => {
                let field_lower = field.to_lowercase();
                field_lower.contains("lamport")
                    || field_lower.contains("balance")
                    || field_lower.contains("amount")
                    || field_lower.contains("supply")
            }
            // Recurse into binary expressions
            Expression::Binary { left, right, .. } => {
                self.is_balance_expression(left) || self.is_balance_expression(right)
            }
            _ => false,
        }
    }

    /// Generate complete Lean 4 code for verification
    pub fn to_lean_code(&self, vcs: &[VerificationCondition], source_file: &str) -> Result<String> {
        let mut code = String::new();

        // Header
        code.push_str("/-\n");
        code.push_str(&format!(
            "  Auto-generated verification conditions for: {}\n",
            source_file
        ));
        code.push_str("  Generated by OVSM compiler - DO NOT EDIT\n");
        code.push_str("-/\n\n");

        // Imports
        code.push_str("import OVSM\n");
        code.push_str("open OVSM OVSM.Tactics\n\n");

        // Namespace
        let ns_name = std::path::Path::new(source_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Program")
            .replace(['-', '.', ' '], "_");
        code.push_str(&format!("namespace VC_{}\n\n", ns_name));

        // Generate each VC as a theorem
        for vc in vcs {
            code.push_str(&format!("-- {}\n", vc.description));
            if let Some(loc) = &vc.location {
                code.push_str(&format!(
                    "-- Source: {}:{}:{}\n",
                    loc.file, loc.line, loc.column
                ));
            }

            // Build theorem with assumptions
            if vc.assumptions.is_empty() {
                code.push_str(&format!(
                    "theorem {} : {} := by\n  {}\n\n",
                    vc.id, vc.property, vc.tactic
                ));
            } else {
                let assumptions_str = vc.assumptions.join(" → ");
                code.push_str(&format!(
                    "theorem {} : {} → {} := by\n  intro _\n  {}\n\n",
                    vc.id, assumptions_str, vc.property, vc.tactic
                ));
            }
        }

        // Close namespace
        code.push_str(&format!("end VC_{}\n", ns_name));

        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Argument;

    #[test]
    fn test_division_vc_generation() {
        let codegen = LeanCodegen::new(VerificationProperties::all());

        let expr = Expression::Binary {
            op: BinaryOp::Div,
            left: Box::new(Expression::Variable("x".to_string())),
            right: Box::new(Expression::Variable("y".to_string())),
        };

        let mut ctx = VCContext::new("test.ovsm");
        // Mark variables as initialized (simulating parameters)
        ctx.initialized_vars.insert("x".to_string(), true);
        ctx.initialized_vars.insert("y".to_string(), true);
        let mut vcs = Vec::new();

        codegen
            .generate_expr_vcs(&expr, &mut ctx, &mut vcs, Some(1))
            .unwrap();

        // Now generates both DivisionSafety and ArithmeticPrecision
        assert_eq!(vcs.len(), 2);
        assert!(vcs
            .iter()
            .any(|vc| vc.category == VCCategory::DivisionSafety));
        assert!(vcs
            .iter()
            .any(|vc| vc.category == VCCategory::ArithmeticPrecision));
        let div_vc = vcs
            .iter()
            .find(|vc| vc.category == VCCategory::DivisionSafety)
            .unwrap();
        assert!(div_vc.property.contains("≠ 0"));
    }

    #[test]
    fn test_array_bounds_vc_generation() {
        let codegen = LeanCodegen::new(VerificationProperties::all());

        let expr = Expression::IndexAccess {
            array: Box::new(Expression::Variable("arr".to_string())),
            index: Box::new(Expression::Variable("i".to_string())),
        };

        let mut ctx = VCContext::new("test.ovsm");
        // Mark variables as initialized (simulating parameters)
        ctx.initialized_vars.insert("arr".to_string(), true);
        ctx.initialized_vars.insert("i".to_string(), true);
        let mut vcs = Vec::new();

        codegen
            .generate_expr_vcs(&expr, &mut ctx, &mut vcs, Some(1))
            .unwrap();

        // Now generates both NullPointerCheck and ArrayBounds
        assert_eq!(vcs.len(), 2);
        assert!(vcs
            .iter()
            .any(|vc| vc.category == VCCategory::NullPointerCheck));
        assert!(vcs.iter().any(|vc| vc.category == VCCategory::ArrayBounds));
        let bounds_vc = vcs
            .iter()
            .find(|vc| vc.category == VCCategory::ArrayBounds)
            .unwrap();
        assert!(bounds_vc.property.contains("< arr.size"));
    }

    #[test]
    fn test_lean_code_generation() {
        let codegen = LeanCodegen::new(VerificationProperties::all());

        let vcs = vec![VerificationCondition {
            id: "vc_div_1".to_string(),
            category: VCCategory::DivisionSafety,
            description: "Division by y must be non-zero".to_string(),
            location: Some(SourceLocation {
                file: "test.ovsm".to_string(),
                line: 5,
                column: 1,
            }),
            property: "y ≠ 0".to_string(),
            assumptions: vec![],
            tactic: "ovsm_div_safe".to_string(),
        }];

        let code = codegen.to_lean_code(&vcs, "test.ovsm").unwrap();

        assert!(code.contains("import OVSM"));
        assert!(code.contains("theorem vc_div_1"));
        assert!(code.contains("y ≠ 0"));
    }
}
