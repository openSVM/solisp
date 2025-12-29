//! # Protocol Enforcement Module
//!
//! This module provides built-in constructs for enforcing protocol correctness
//! in Solisp programs. It adds:
//!
//! 1. State Machine Definitions - Enforce valid state transitions
//! 2. Access Control - Enforce authorization requirements
//! 3. Economic Invariants - Enforce balance conservation
//! 4. Automatic VC Generation - Generate proofs for these properties
//!
//! ## Usage in Solisp:
//!
//! ```lisp
//! ;; Define a state machine
//! (defstate OrderStatus
//!   :states (Created Accepted InProgress Delivered Completed Disputed Refunded Cancelled)
//!   :initial Created
//!   :terminal (Completed Refunded Cancelled)
//!   :transitions
//!     ((Created -> Accepted Cancelled)
//!      (Accepted -> InProgress Refunded)
//!      (InProgress -> Delivered Disputed Refunded)
//!      (Delivered -> Completed Disputed)
//!      (Disputed -> Completed Refunded)))
//!
//! ;; Define access control
//! (defaccess AcceptOrder
//!   :requires (signer-is order.provider)
//!   :precondition (= order.status Created))
//!
//! ;; Define economic invariant
//! (definvariant StakeAccounting
//!   (= config.total_staked (sum-of participant.stake_amount)))
//! ```

use super::{SourceLocation, VCCategory, VerificationCondition};
use crate::parser::Expression;
use std::collections::{HashMap, HashSet};

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 1: STATE MACHINE DEFINITIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// A state in a state machine
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct State {
    pub name: String,
    pub value: u8,
}

/// A state machine definition
#[derive(Debug, Clone)]
pub struct StateMachine {
    /// Name of the state machine (e.g., "OrderStatus")
    pub name: String,
    /// All possible states
    pub states: Vec<State>,
    /// Initial state
    pub initial: State,
    /// Terminal states (no outgoing transitions)
    pub terminal: HashSet<String>,
    /// Valid transitions: from_state -> set of to_states
    pub transitions: HashMap<String, HashSet<String>>,
}

impl StateMachine {
    /// Create a new state machine
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            states: Vec::new(),
            initial: State {
                name: String::new(),
                value: 0,
            },
            terminal: HashSet::new(),
            transitions: HashMap::new(),
        }
    }

    /// Add a state
    pub fn add_state(&mut self, name: &str, value: u8) {
        self.states.push(State {
            name: name.to_string(),
            value,
        });
    }

    /// Set initial state
    pub fn set_initial(&mut self, name: &str) {
        if let Some(s) = self.states.iter().find(|s| s.name == name) {
            self.initial = s.clone();
        }
    }

    /// Mark states as terminal
    pub fn set_terminal(&mut self, names: &[&str]) {
        for name in names {
            self.terminal.insert(name.to_string());
        }
    }

    /// Add a valid transition
    pub fn add_transition(&mut self, from: &str, to: &str) {
        self.transitions
            .entry(from.to_string())
            .or_default()
            .insert(to.to_string());
    }

    /// Check if a transition is valid
    pub fn is_valid_transition(&self, from: &str, to: &str) -> bool {
        self.transitions
            .get(from)
            .map(|tos| tos.contains(to))
            .unwrap_or(false)
    }

    /// Check if a state is terminal
    pub fn is_terminal(&self, state: &str) -> bool {
        self.terminal.contains(state)
    }

    /// Get all valid next states from a given state
    pub fn next_states(&self, from: &str) -> Vec<&str> {
        self.transitions
            .get(from)
            .map(|tos| tos.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Generate VCs for state machine correctness
    pub fn generate_vcs(&self, source_file: &str) -> Vec<VerificationCondition> {
        let mut vcs = Vec::new();

        // VC: Terminal states have no outgoing transitions
        for terminal in &self.terminal {
            vcs.push(VerificationCondition {
                id: format!("sm_{}_terminal_{}", self.name, terminal),
                category: VCCategory::Custom(format!("state_machine_{}", self.name)),
                description: format!("Terminal state '{}' has no outgoing transitions", terminal),
                location: Some(SourceLocation {
                    file: source_file.to_string(),
                    line: 1,
                    column: 1,
                }),
                property: format!("∀ s'. ¬validTransition({}, {}, s')", self.name, terminal),
                assumptions: vec![format!("isTerminal({}, {})", self.name, terminal)],
                tactic: "simp [validTransition, isTerminal]".to_string(),
            });
        }

        // VC: All transitions are in the allowed set
        for (from, tos) in &self.transitions {
            for to in tos {
                vcs.push(VerificationCondition {
                    id: format!("sm_{}_trans_{}_{}", self.name, from, to),
                    category: VCCategory::Custom(format!("state_machine_{}", self.name)),
                    description: format!("Transition {} -> {} is valid", from, to),
                    location: Some(SourceLocation {
                        file: source_file.to_string(),
                        line: 1,
                        column: 1,
                    }),
                    property: format!("validTransition({}, {}, {})", self.name, from, to),
                    assumptions: vec![],
                    tactic: "rfl".to_string(),
                });
            }
        }

        vcs
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 2: ACCESS CONTROL DEFINITIONS
// ═══════════════════════════════════════════════════════════════════════════════

/// Access control requirement types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessRequirement {
    /// Signer must match a specific account field
    SignerIs { account: String, field: String },
    /// Signer must be the admin
    IsAdmin,
    /// Account must have specific status
    HasStatus { account: String, status: String },
    /// Account must be active
    IsActive { account: String },
    /// Custom predicate
    Custom(String),
}

/// An access control definition for an instruction
#[derive(Debug, Clone)]
pub struct AccessControl {
    /// Name of the instruction
    pub instruction: String,
    /// Required access checks
    pub requirements: Vec<AccessRequirement>,
    /// Preconditions that must hold
    pub preconditions: Vec<String>,
}

impl AccessControl {
    pub fn new(instruction: &str) -> Self {
        Self {
            instruction: instruction.to_string(),
            requirements: Vec::new(),
            preconditions: Vec::new(),
        }
    }

    /// Add a signer requirement
    pub fn require_signer(&mut self, account: &str, field: &str) {
        self.requirements.push(AccessRequirement::SignerIs {
            account: account.to_string(),
            field: field.to_string(),
        });
    }

    /// Add admin requirement
    pub fn require_admin(&mut self) {
        self.requirements.push(AccessRequirement::IsAdmin);
    }

    /// Add status requirement
    pub fn require_status(&mut self, account: &str, status: &str) {
        self.requirements.push(AccessRequirement::HasStatus {
            account: account.to_string(),
            status: status.to_string(),
        });
    }

    /// Add active requirement
    pub fn require_active(&mut self, account: &str) {
        self.requirements.push(AccessRequirement::IsActive {
            account: account.to_string(),
        });
    }

    /// Add a precondition
    pub fn add_precondition(&mut self, condition: &str) {
        self.preconditions.push(condition.to_string());
    }

    /// Generate VCs for access control
    pub fn generate_vcs(&self, source_file: &str) -> Vec<VerificationCondition> {
        let mut vcs = Vec::new();

        for (i, req) in self.requirements.iter().enumerate() {
            let (description, property) = match req {
                AccessRequirement::SignerIs { account, field } => (
                    format!("{}: Signer must be {}.{}", self.instruction, account, field),
                    format!("signer = {}.{}", account, field),
                ),
                AccessRequirement::IsAdmin => (
                    format!("{}: Signer must be admin", self.instruction),
                    "signer = config.admin".to_string(),
                ),
                AccessRequirement::HasStatus { account, status } => (
                    format!(
                        "{}: {} must have status {}",
                        self.instruction, account, status
                    ),
                    format!("{}.status = {}", account, status),
                ),
                AccessRequirement::IsActive { account } => (
                    format!("{}: {} must be active", self.instruction, account),
                    format!("{}.status = Active", account),
                ),
                AccessRequirement::Custom(pred) => (
                    format!("{}: Custom requirement", self.instruction),
                    pred.clone(),
                ),
            };

            vcs.push(VerificationCondition {
                id: format!("ac_{}_{}", self.instruction, i),
                category: VCCategory::SignerCheck,
                description,
                location: Some(SourceLocation {
                    file: source_file.to_string(),
                    line: 1,
                    column: 1,
                }),
                property,
                assumptions: vec![],
                tactic: "access_control".to_string(),
            });
        }

        for (i, precond) in self.preconditions.iter().enumerate() {
            vcs.push(VerificationCondition {
                id: format!("ac_{}_pre_{}", self.instruction, i),
                category: VCCategory::Custom("precondition".to_string()),
                description: format!("{}: Precondition must hold", self.instruction),
                location: Some(SourceLocation {
                    file: source_file.to_string(),
                    line: 1,
                    column: 1,
                }),
                property: precond.clone(),
                assumptions: vec![],
                tactic: "precondition".to_string(),
            });
        }

        vcs
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 3: ECONOMIC INVARIANTS
// ═══════════════════════════════════════════════════════════════════════════════

/// Types of economic invariants
#[derive(Debug, Clone)]
pub enum InvariantType {
    /// Sum equality: field = sum of other fields
    SumEquality {
        lhs: String,
        rhs_collection: String,
        rhs_field: String,
    },
    /// Conservation: total in = total out
    Conservation {
        inflows: Vec<String>,
        outflows: Vec<String>,
    },
    /// Bound: value within range
    Bounded {
        field: String,
        min: Option<i64>,
        max: Option<i64>,
    },
    /// Non-negative
    NonNegative { field: String },
    /// Custom predicate
    Custom { name: String, predicate: String },
}

/// An economic invariant that must always hold
#[derive(Debug, Clone)]
pub struct EconomicInvariant {
    /// Name of the invariant
    pub name: String,
    /// Type of invariant
    pub invariant_type: InvariantType,
    /// Human-readable description
    pub description: String,
}

impl EconomicInvariant {
    /// Create a sum equality invariant
    pub fn sum_equality(name: &str, lhs: &str, collection: &str, field: &str) -> Self {
        Self {
            name: name.to_string(),
            invariant_type: InvariantType::SumEquality {
                lhs: lhs.to_string(),
                rhs_collection: collection.to_string(),
                rhs_field: field.to_string(),
            },
            description: format!("{} = Σ {}.{}", lhs, collection, field),
        }
    }

    /// Create a conservation invariant
    pub fn conservation(name: &str, inflows: Vec<&str>, outflows: Vec<&str>) -> Self {
        Self {
            name: name.to_string(),
            invariant_type: InvariantType::Conservation {
                inflows: inflows.iter().map(|s| s.to_string()).collect(),
                outflows: outflows.iter().map(|s| s.to_string()).collect(),
            },
            description: format!("Σ({}) = Σ({})", inflows.join(" + "), outflows.join(" + ")),
        }
    }

    /// Create a bounded invariant
    pub fn bounded(name: &str, field: &str, min: Option<i64>, max: Option<i64>) -> Self {
        let desc = match (min, max) {
            (Some(lo), Some(hi)) => format!("{} ≤ {} ≤ {}", lo, field, hi),
            (Some(lo), None) => format!("{} ≤ {}", lo, field),
            (None, Some(hi)) => format!("{} ≤ {}", field, hi),
            (None, None) => format!("{} is bounded", field),
        };
        Self {
            name: name.to_string(),
            invariant_type: InvariantType::Bounded {
                field: field.to_string(),
                min,
                max,
            },
            description: desc,
        }
    }

    /// Create a non-negative invariant
    pub fn non_negative(name: &str, field: &str) -> Self {
        Self {
            name: name.to_string(),
            invariant_type: InvariantType::NonNegative {
                field: field.to_string(),
            },
            description: format!("{} ≥ 0", field),
        }
    }

    /// Create a custom invariant
    pub fn custom(name: &str, predicate: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            invariant_type: InvariantType::Custom {
                name: name.to_string(),
                predicate: predicate.to_string(),
            },
            description: description.to_string(),
        }
    }

    /// Generate VC for this invariant
    pub fn generate_vc(&self, source_file: &str) -> VerificationCondition {
        let property = match &self.invariant_type {
            InvariantType::SumEquality {
                lhs,
                rhs_collection,
                rhs_field,
            } => {
                format!("{} = sum({}, λ x. x.{})", lhs, rhs_collection, rhs_field)
            }
            InvariantType::Conservation { inflows, outflows } => {
                format!(
                    "sum([{}]) = sum([{}])",
                    inflows.join(", "),
                    outflows.join(", ")
                )
            }
            InvariantType::Bounded { field, min, max } => match (min, max) {
                (Some(lo), Some(hi)) => format!("{} ≤ {} ∧ {} ≤ {}", lo, field, field, hi),
                (Some(lo), None) => format!("{} ≤ {}", lo, field),
                (None, Some(hi)) => format!("{} ≤ {}", field, hi),
                (None, None) => "true".to_string(),
            },
            InvariantType::NonNegative { field } => {
                format!("{} ≥ 0", field)
            }
            InvariantType::Custom { predicate, .. } => predicate.clone(),
        };

        VerificationCondition {
            id: format!("inv_{}", self.name),
            category: VCCategory::BalanceConservation,
            description: format!("Invariant: {}", self.description),
            location: Some(SourceLocation {
                file: source_file.to_string(),
                line: 1,
                column: 1,
            }),
            property,
            assumptions: vec![],
            tactic: "invariant".to_string(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 4: PROTOCOL SPECIFICATION
// ═══════════════════════════════════════════════════════════════════════════════

/// Complete protocol specification
#[derive(Debug, Clone, Default)]
pub struct ProtocolSpec {
    /// Name of the protocol
    pub name: String,
    /// State machines
    pub state_machines: Vec<StateMachine>,
    /// Access control rules
    pub access_controls: Vec<AccessControl>,
    /// Economic invariants
    pub invariants: Vec<EconomicInvariant>,
}

impl ProtocolSpec {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            state_machines: Vec::new(),
            access_controls: Vec::new(),
            invariants: Vec::new(),
        }
    }

    /// Add a state machine
    pub fn add_state_machine(&mut self, sm: StateMachine) {
        self.state_machines.push(sm);
    }

    /// Add access control
    pub fn add_access_control(&mut self, ac: AccessControl) {
        self.access_controls.push(ac);
    }

    /// Add an invariant
    pub fn add_invariant(&mut self, inv: EconomicInvariant) {
        self.invariants.push(inv);
    }

    /// Generate all VCs for the protocol specification
    pub fn generate_all_vcs(&self, source_file: &str) -> Vec<VerificationCondition> {
        let mut vcs = Vec::new();

        for sm in &self.state_machines {
            vcs.extend(sm.generate_vcs(source_file));
        }

        for ac in &self.access_controls {
            vcs.extend(ac.generate_vcs(source_file));
        }

        for inv in &self.invariants {
            vcs.push(inv.generate_vc(source_file));
        }

        vcs
    }

    /// Generate OVSM code for runtime checks
    /// This produces code that can be injected into the program
    pub fn generate_runtime_checks(&self) -> Vec<String> {
        let mut checks = Vec::new();

        // Generate state transition validation function
        for sm in &self.state_machines {
            checks.push(sm.generate_transition_validator());
        }

        // Generate access control check functions
        for ac in &self.access_controls {
            checks.push(ac.generate_runtime_check());
        }

        checks
    }

    /// Extract a ProtocolSpec from parsed OVSM expressions
    /// This processes __defstate__, __defaccess__, __definvariant__ tool calls
    pub fn from_expressions(exprs: &[crate::parser::Expression]) -> Self {
        use crate::parser::Expression;

        let mut spec = ProtocolSpec::new("Extracted");

        for expr in exprs {
            if let Expression::ToolCall { name, args } = expr {
                match name.as_str() {
                    "__defstate__" => {
                        if let Some(sm) = Self::parse_defstate_args(args) {
                            spec.add_state_machine(sm);
                        }
                    }
                    "__defaccess__" => {
                        if let Some(ac) = Self::parse_defaccess_args(args) {
                            spec.add_access_control(ac);
                        }
                    }
                    "__definvariant__" => {
                        if let Some(inv) = Self::parse_definvariant_args(args) {
                            spec.add_invariant(inv);
                        }
                    }
                    _ => {}
                }
            }
        }

        spec
    }

    fn parse_defstate_args(args: &[crate::parser::Argument]) -> Option<StateMachine> {
        use crate::parser::Expression;

        // Args: name, states[], initial, terminal[], transitions[][]
        if args.len() < 5 {
            return None;
        }

        let name = match &args[0].value {
            Expression::StringLiteral(s) => s.clone(),
            _ => return None,
        };

        let mut sm = StateMachine::new(&name);

        // Parse states
        if let Expression::ArrayLiteral(states) = &args[1].value {
            for (i, state) in states.iter().enumerate() {
                if let Expression::StringLiteral(s) = state {
                    sm.add_state(s, i as u8);
                }
            }
        }

        // Parse initial
        if let Expression::StringLiteral(s) = &args[2].value {
            sm.set_initial(s);
        }

        // Parse terminal states
        if let Expression::ArrayLiteral(terminals) = &args[3].value {
            let term_names: Vec<&str> = terminals
                .iter()
                .filter_map(|e| {
                    if let Expression::StringLiteral(s) = e {
                        Some(s.as_str())
                    } else {
                        None
                    }
                })
                .collect();
            sm.set_terminal(&term_names);
        }

        // Parse transitions
        if let Expression::ArrayLiteral(transitions) = &args[4].value {
            for trans in transitions {
                if let Expression::ArrayLiteral(pair) = trans {
                    if pair.len() == 2 {
                        if let (Expression::StringLiteral(from), Expression::StringLiteral(to)) =
                            (&pair[0], &pair[1])
                        {
                            sm.add_transition(from, to);
                        }
                    }
                }
            }
        }

        Some(sm)
    }

    fn parse_defaccess_args(args: &[crate::parser::Argument]) -> Option<AccessControl> {
        use crate::parser::Expression;

        // Args: instruction, signer_reqs[][], requires_admin, active_reqs[], preconditions[]
        if args.len() < 5 {
            return None;
        }

        let instruction = match &args[0].value {
            Expression::StringLiteral(s) => s.clone(),
            _ => return None,
        };

        let mut ac = AccessControl::new(&instruction);

        // Parse signer requirements
        if let Expression::ArrayLiteral(reqs) = &args[1].value {
            for req in reqs {
                if let Expression::ArrayLiteral(pair) = req {
                    if pair.len() == 2 {
                        if let (Expression::StringLiteral(acc), Expression::StringLiteral(field)) =
                            (&pair[0], &pair[1])
                        {
                            ac.require_signer(acc, field);
                        }
                    }
                }
            }
        }

        // Parse admin requirement
        if let Expression::BoolLiteral(b) = &args[2].value {
            if *b {
                ac.require_admin();
            }
        }

        // Parse active requirements
        if let Expression::ArrayLiteral(reqs) = &args[3].value {
            for req in reqs {
                if let Expression::StringLiteral(s) = req {
                    ac.require_active(s);
                }
            }
        }

        // Preconditions are stored but would need more complex handling
        // to convert back to strings

        Some(ac)
    }

    fn parse_definvariant_args(args: &[crate::parser::Argument]) -> Option<EconomicInvariant> {
        use crate::parser::Expression;

        // Args: name, description, predicate
        if args.len() < 3 {
            return None;
        }

        let name = match &args[0].value {
            Expression::StringLiteral(s) => s.clone(),
            _ => return None,
        };

        let description = match &args[1].value {
            Expression::StringLiteral(s) => s.clone(),
            _ => name.clone(),
        };

        // For now, store predicate as string representation
        let predicate = format!("{:?}", args[2].value);

        Some(EconomicInvariant::custom(&name, &predicate, &description))
    }

    /// Extract a ProtocolSpec directly from a parsed Program AST
    /// This processes DefState, DefAccess, DefInvariant, DefProtocol statements
    pub fn from_program(program: &crate::parser::Program) -> Self {
        use crate::parser::Statement;

        let mut spec = ProtocolSpec::new("Extracted");

        for stmt in &program.statements {
            spec.extract_from_statement(stmt);
        }

        spec
    }

    /// Recursively extract specs from a statement
    fn extract_from_statement(&mut self, stmt: &crate::parser::Statement) {
        use crate::parser::Statement;

        match stmt {
            Statement::DefState {
                name,
                states,
                initial,
                terminal,
                transitions,
            } => {
                let mut sm = StateMachine::new(name);
                for (i, state) in states.iter().enumerate() {
                    sm.add_state(state, i as u8);
                }
                sm.set_initial(initial);
                let term_refs: Vec<&str> = terminal.iter().map(|s| s.as_str()).collect();
                sm.set_terminal(&term_refs);
                for (from, to) in transitions {
                    sm.add_transition(from, to);
                }
                self.add_state_machine(sm);
            }

            Statement::DefAccess {
                instruction,
                signer_requirements,
                requires_admin,
                active_requirements,
                preconditions: _,
            } => {
                let mut ac = AccessControl::new(instruction);
                for (account, field) in signer_requirements {
                    ac.require_signer(account, field);
                }
                if *requires_admin {
                    ac.require_admin();
                }
                for account in active_requirements {
                    ac.require_active(account);
                }
                self.add_access_control(ac);
            }

            Statement::DefInvariant {
                name,
                description,
                predicate,
            } => {
                let pred_str = format!("{:?}", predicate);
                let inv = EconomicInvariant::custom(name, &pred_str, description);
                self.add_invariant(inv);
            }

            Statement::DefProtocol { name, body } => {
                // Update spec name to protocol name
                self.name = name.clone();
                // Process all statements in the protocol body
                for inner_stmt in body {
                    self.extract_from_statement(inner_stmt);
                }
            }

            // Recurse into control flow structures that might contain specs
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                for s in then_branch {
                    self.extract_from_statement(s);
                }
                if let Some(else_stmts) = else_branch {
                    for s in else_stmts {
                        self.extract_from_statement(s);
                    }
                }
            }

            Statement::While { body, .. } | Statement::For { body, .. } => {
                for s in body {
                    self.extract_from_statement(s);
                }
            }

            Statement::Parallel { tasks } => {
                for s in tasks {
                    self.extract_from_statement(s);
                }
            }

            Statement::Try {
                body,
                catch_clauses,
            } => {
                for s in body {
                    self.extract_from_statement(s);
                }
                for clause in catch_clauses {
                    for s in &clause.body {
                        self.extract_from_statement(s);
                    }
                }
            }

            Statement::Guard { else_body, .. } => {
                for s in else_body {
                    self.extract_from_statement(s);
                }
            }

            // Handle Expression statements containing __defstate__, __defaccess__, __definvariant__ tool calls
            // The parser converts defstate/defaccess/definvariant syntax to these tool calls
            Statement::Expression(expr) => {
                self.extract_from_expression(expr);
            }

            // Other statement types don't contain protocol specs
            _ => {}
        }
    }

    /// Extract specs from an expression (handles __defstate__, __defaccess__, __definvariant__ tool calls)
    fn extract_from_expression(&mut self, expr: &crate::parser::Expression) {
        use crate::parser::Expression;

        match expr {
            Expression::ToolCall { name, args } => {
                match name.as_str() {
                    "__defstate__" => {
                        if let Some(sm) = Self::parse_defstate_args(args) {
                            self.add_state_machine(sm);
                        }
                    }
                    "__defaccess__" => {
                        if let Some(ac) = Self::parse_defaccess_args(args) {
                            self.add_access_control(ac);
                        }
                    }
                    "__definvariant__" => {
                        if let Some(inv) = Self::parse_definvariant_args(args) {
                            self.add_invariant(inv);
                        }
                    }
                    "__defprotocol__" => {
                        // Update spec name from first arg
                        if let Some(arg) = args.first() {
                            if let Expression::StringLiteral(s) = &arg.value {
                                self.name = s.clone();
                            }
                        }
                        // Process remaining args as body expressions
                        for arg in args.iter().skip(1) {
                            self.extract_from_expression(&arg.value);
                        }
                    }
                    _ => {}
                }
            }
            // Recurse into grouped expressions
            Expression::Grouping(inner) => {
                self.extract_from_expression(inner);
            }
            // Recurse into block expressions (do blocks)
            Expression::Loop(loop_data) => {
                for expr in &loop_data.body {
                    self.extract_from_expression(expr);
                }
            }
            _ => {}
        }
    }

    /// Check if any protocol specs were extracted
    pub fn has_specs(&self) -> bool {
        !self.state_machines.is_empty()
            || !self.access_controls.is_empty()
            || !self.invariants.is_empty()
    }
}

impl StateMachine {
    /// Generate OVSM code for a state transition validator
    pub fn generate_transition_validator(&self) -> String {
        let mut code = format!(";; State transition validator for {}\n", self.name);
        code.push_str(&format!(
            "(defn validate-{}-transition (from to)\n",
            self.name.to_lowercase()
        ));
        code.push_str("  (cond\n");

        // Generate cases for each valid transition
        for (from, tos) in &self.transitions {
            let from_val = self
                .states
                .iter()
                .find(|s| &s.name == from)
                .map(|s| s.value)
                .unwrap_or(0);

            for to in tos {
                let to_val = self
                    .states
                    .iter()
                    .find(|s| &s.name == to)
                    .map(|s| s.value)
                    .unwrap_or(0);

                code.push_str(&format!(
                    "    ((and (= from {}) (= to {})) true)  ;; {} -> {}\n",
                    from_val, to_val, from, to
                ));
            }
        }

        // Default: invalid transition
        code.push_str("    (true (do\n");
        code.push_str(&format!(
            "      (sol_log_ \"ERROR: Invalid {} transition\")\n",
            self.name
        ));
        code.push_str("      (sol_log_64_ from)\n");
        code.push_str("      (sol_log_64_ to)\n");
        code.push_str("      false))))\n");

        code
    }

    /// Generate assertion code for a specific transition
    pub fn generate_transition_assert(&self, from_val: u8, to_val: u8) -> Option<String> {
        // Find state names
        let from_name = self
            .states
            .iter()
            .find(|s| s.value == from_val)
            .map(|s| &s.name)?;
        let to_name = self
            .states
            .iter()
            .find(|s| s.value == to_val)
            .map(|s| &s.name)?;

        // Check if transition is valid
        if !self.is_valid_transition(from_name, to_name) {
            return Some(format!(
                ";; ERROR: Invalid state transition {} ({}) -> {} ({})\n\
                 (do (sol_log_ \"FATAL: Invalid state transition\") 1)",
                from_name, from_val, to_name, to_val
            ));
        }

        // Valid transition - no assertion needed
        None
    }
}

impl AccessControl {
    /// Generate OVSM code for runtime access control check
    pub fn generate_runtime_check(&self) -> String {
        let mut code = format!(";; Access control check for {}\n", self.instruction);
        code.push_str(&format!(
            "(defn check-{}-access (signer accounts)\n",
            self.instruction.to_lowercase().replace(" ", "-")
        ));
        code.push_str("  (and\n");

        // Generate checks based on requirements
        for req in &self.requirements {
            match req {
                AccessRequirement::SignerIs { account, field } => {
                    code.push_str(&format!("    ;; Signer must be {}.{}\n", account, field));
                    code.push_str(&format!(
                        "    (= signer (get-field {} \"{}\"))\n",
                        account, field
                    ));
                }
                AccessRequirement::IsAdmin => {
                    code.push_str("    ;; Signer must be admin\n");
                    code.push_str("    (= signer (get-field config \"admin\"))\n");
                }
                AccessRequirement::IsActive { account } => {
                    code.push_str(&format!("    ;; {} must be active\n", account));
                    code.push_str(&format!(
                        "    (= (get-field {} \"status\") 1)\n", // 1 = Active
                        account
                    ));
                }
                AccessRequirement::HasStatus { account, status } => {
                    code.push_str(&format!("    ;; {} must have status {}\n", account, status));
                    code.push_str(&format!(
                        "    (= (get-field {} \"status\") \"{}\")\n",
                        account, status
                    ));
                }
                AccessRequirement::Custom(predicate) => {
                    code.push_str(&format!("    ;; Custom: {}\n", predicate));
                    code.push_str(&format!("    {}\n", predicate));
                }
            }
        }

        code.push_str("    true))\n");
        code
    }

    /// Generate inline assertion for this access control
    pub fn generate_inline_assert(&self, signer_account_idx: usize) -> String {
        let mut checks = Vec::new();

        for req in &self.requirements {
            match req {
                AccessRequirement::SignerIs { account, field: _ } => {
                    checks.push(format!(
                        "(if (= (account-is-signer {}) 0)\n\
                           (do (sol_log_ \"ERROR: {} not signer for {}\") 1)\n\
                           null)",
                        signer_account_idx, account, self.instruction
                    ));
                }
                AccessRequirement::IsAdmin => {
                    checks.push(format!(
                        ";; Admin check for {}\n\
                         (define admin_pk_0 (mem-load (account-data-ptr 0) 80))\n\
                         (define signer_pk_0 (mem-load (account-pubkey {}) 0))\n\
                         (if (!= admin_pk_0 signer_pk_0)\n\
                           (do (sol_log_ \"ERROR: Not admin\") 1)\n\
                           null)",
                        self.instruction, signer_account_idx
                    ));
                }
                AccessRequirement::IsActive { account } => {
                    checks.push(format!(
                        ";; Active check for {}\n\
                         (if (= (get-field {} \"status\") 0)\n\
                           (do (sol_log_ \"ERROR: {} is not active\") 1)\n\
                           null)",
                        account, account, account
                    ));
                }
                AccessRequirement::HasStatus { account, status } => {
                    checks.push(format!(
                        ";; Status check for {}\n\
                         (if (!= (get-field {} \"status\") \"{}\")\n\
                           (do (sol_log_ \"ERROR: {} does not have status {}\") 1)\n\
                           null)",
                        account, account, status, account, status
                    ));
                }
                AccessRequirement::Custom(predicate) => {
                    checks.push(format!(
                        ";; Custom check\n\
                         (if (not {})\n\
                           (do (sol_log_ \"ERROR: Custom check failed\") 1)\n\
                           null)",
                        predicate
                    ));
                }
            }
        }

        if checks.is_empty() {
            String::new()
        } else {
            format!(
                ";; Access control: {}\n{}",
                self.instruction,
                checks.join("\n")
            )
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SECTION 5: AEA PROTOCOL SPECIFICATION (BUILT-IN)
// ═══════════════════════════════════════════════════════════════════════════════

/// Create the complete AEA protocol specification
pub fn create_aea_spec() -> ProtocolSpec {
    let mut spec = ProtocolSpec::new("AEA");

    // ─────────────────────────────────────────────────────────────────────────
    // Order State Machine
    // ─────────────────────────────────────────────────────────────────────────
    let mut order_sm = StateMachine::new("OrderStatus");
    order_sm.add_state("Created", 0);
    order_sm.add_state("Accepted", 1);
    order_sm.add_state("InProgress", 2);
    order_sm.add_state("Delivered", 3);
    order_sm.add_state("Completed", 4);
    order_sm.add_state("Disputed", 5);
    order_sm.add_state("Refunded", 6);
    order_sm.add_state("Cancelled", 7);

    order_sm.set_initial("Created");
    order_sm.set_terminal(&["Completed", "Refunded", "Cancelled"]);

    // Valid transitions
    order_sm.add_transition("Created", "Accepted");
    order_sm.add_transition("Created", "Cancelled");
    order_sm.add_transition("Accepted", "InProgress");
    order_sm.add_transition("Accepted", "Refunded");
    order_sm.add_transition("InProgress", "Delivered");
    order_sm.add_transition("InProgress", "Disputed");
    order_sm.add_transition("InProgress", "Refunded");
    order_sm.add_transition("Delivered", "Completed");
    order_sm.add_transition("Delivered", "Disputed");
    order_sm.add_transition("Disputed", "Completed");
    order_sm.add_transition("Disputed", "Refunded");

    spec.add_state_machine(order_sm);

    // ─────────────────────────────────────────────────────────────────────────
    // Participant State Machine
    // ─────────────────────────────────────────────────────────────────────────
    let mut participant_sm = StateMachine::new("ParticipantStatus");
    participant_sm.add_state("Inactive", 0);
    participant_sm.add_state("Active", 1);
    participant_sm.add_state("Cooldown", 2);
    participant_sm.add_state("Slashed", 3);
    participant_sm.add_state("Suspended", 4);

    participant_sm.set_initial("Inactive");
    participant_sm.set_terminal(&["Slashed"]);

    participant_sm.add_transition("Inactive", "Active");
    participant_sm.add_transition("Active", "Cooldown");
    participant_sm.add_transition("Active", "Slashed");
    participant_sm.add_transition("Active", "Suspended");
    participant_sm.add_transition("Cooldown", "Inactive");
    participant_sm.add_transition("Cooldown", "Active");
    participant_sm.add_transition("Cooldown", "Slashed");
    participant_sm.add_transition("Suspended", "Active");
    participant_sm.add_transition("Suspended", "Slashed");

    spec.add_state_machine(participant_sm);

    // ─────────────────────────────────────────────────────────────────────────
    // Access Control Rules
    // ─────────────────────────────────────────────────────────────────────────

    // InitializeProtocol - anyone can initialize, becomes admin
    let mut init_ac = AccessControl::new("InitializeProtocol");
    init_ac.add_precondition("config.initialized = false");
    spec.add_access_control(init_ac);

    // UpdateConfig - admin only
    let mut update_config_ac = AccessControl::new("UpdateConfig");
    update_config_ac.require_admin();
    update_config_ac.add_precondition("config.initialized = true");
    spec.add_access_control(update_config_ac);

    // RegisterUser/Agent/Provider - signer is authority
    for instr in &[
        "RegisterUser",
        "RegisterAgent",
        "RegisterProvider",
        "RegisterValidator",
    ] {
        let mut ac = AccessControl::new(instr);
        ac.require_signer("participant", "authority");
        ac.add_precondition("config.initialized = true");
        ac.add_precondition("participant.status = Inactive");
        spec.add_access_control(ac);
    }

    // UpdateProfile - authority only, must be active
    let mut update_profile_ac = AccessControl::new("UpdateProfile");
    update_profile_ac.require_signer("participant", "authority");
    update_profile_ac.require_active("participant");
    spec.add_access_control(update_profile_ac);

    // CreateOrder - buyer must be active
    let mut create_order_ac = AccessControl::new("CreateOrder");
    create_order_ac.require_signer("buyer", "authority");
    create_order_ac.require_active("buyer");
    create_order_ac.require_active("provider");
    create_order_ac.add_precondition("service.is_active = true");
    create_order_ac.add_precondition("buyer.reputation >= service.min_reputation");
    spec.add_access_control(create_order_ac);

    // AcceptOrder - provider only
    let mut accept_order_ac = AccessControl::new("AcceptOrder");
    accept_order_ac.require_signer("order", "provider");
    accept_order_ac.add_precondition("order.status = Created");
    spec.add_access_control(accept_order_ac);

    // ConfirmDelivery - buyer only
    let mut confirm_ac = AccessControl::new("ConfirmDelivery");
    confirm_ac.require_signer("order", "buyer");
    confirm_ac.add_precondition("order.status = Delivered");
    spec.add_access_control(confirm_ac);

    // CancelOrder - buyer only, before acceptance
    let mut cancel_ac = AccessControl::new("CancelOrder");
    cancel_ac.require_signer("order", "buyer");
    cancel_ac.add_precondition("order.status = Created");
    spec.add_access_control(cancel_ac);

    // OpenDispute - buyer or provider
    let mut dispute_ac = AccessControl::new("OpenDispute");
    dispute_ac.requirements.push(AccessRequirement::Custom(
        "signer = order.buyer ∨ signer = order.provider".to_string(),
    ));
    dispute_ac.add_precondition("order.status = Delivered ∨ order.status = InProgress");
    dispute_ac.add_precondition("now ≤ order.dispute_deadline");
    spec.add_access_control(dispute_ac);

    // ResolveDispute - admin only
    let mut resolve_ac = AccessControl::new("ResolveDispute");
    resolve_ac.require_admin();
    resolve_ac.add_precondition("order.status = Disputed");
    spec.add_access_control(resolve_ac);

    // SlashParticipant - admin only
    let mut slash_ac = AccessControl::new("SlashParticipant");
    slash_ac.require_admin();
    slash_ac.add_precondition("participant.status ≠ Slashed");
    spec.add_access_control(slash_ac);

    // SuspendParticipant - admin only
    let mut suspend_ac = AccessControl::new("SuspendParticipant");
    suspend_ac.require_admin();
    suspend_ac.add_precondition("participant.status ≠ Slashed");
    spec.add_access_control(suspend_ac);

    // ─────────────────────────────────────────────────────────────────────────
    // Economic Invariants
    // ─────────────────────────────────────────────────────────────────────────

    // Stake accounting
    spec.add_invariant(EconomicInvariant::sum_equality(
        "StakeAccounting",
        "config.total_staked",
        "participants",
        "stake_amount",
    ));

    // Escrow accounting
    spec.add_invariant(EconomicInvariant::custom(
        "EscrowAccounting",
        "escrow_total = sum(orders.filter(λ o. ¬isTerminal(o.status)), λ o. o.amount + o.fee_amount)",
        "Escrow balance = sum of active order amounts + fees",
    ));

    // Participant count
    spec.add_invariant(EconomicInvariant::custom(
        "ParticipantCount",
        "config.total_participants = length(participants)",
        "Participant count matches actual count",
    ));

    // Non-negative stakes
    spec.add_invariant(EconomicInvariant::custom(
        "NonNegativeStakes",
        "∀ p ∈ participants. p.stake_amount ≥ 0",
        "All stakes are non-negative",
    ));

    // Fee calculation
    spec.add_invariant(EconomicInvariant::custom(
        "FeeCalculation",
        "∀ o ∈ orders. o.fee_amount = (o.amount * config.escrow_fee_bps) / 10000",
        "Fees are calculated correctly",
    ));

    // Service order limits
    spec.add_invariant(EconomicInvariant::custom(
        "ServiceOrderLimits",
        "∀ s ∈ services. s.active_orders ≤ s.max_concurrent",
        "Active orders don't exceed service limits",
    ));

    // Balance conservation (simplified)
    spec.add_invariant(EconomicInvariant::conservation(
        "BalanceConservation",
        vec!["tokens_deposited", "stake_deposits", "escrow_deposits"],
        vec![
            "tokens_withdrawn",
            "stake_withdrawals",
            "escrow_releases",
            "fees_collected",
        ],
    ));

    spec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_state_machine() {
        let spec = create_aea_spec();
        let order_sm = spec
            .state_machines
            .iter()
            .find(|sm| sm.name == "OrderStatus")
            .unwrap();

        // Valid transitions
        assert!(order_sm.is_valid_transition("Created", "Accepted"));
        assert!(order_sm.is_valid_transition("Created", "Cancelled"));
        assert!(order_sm.is_valid_transition("Delivered", "Completed"));
        assert!(order_sm.is_valid_transition("Disputed", "Refunded"));

        // Invalid transitions
        assert!(!order_sm.is_valid_transition("Created", "Completed"));
        assert!(!order_sm.is_valid_transition("Completed", "Disputed"));
        assert!(!order_sm.is_valid_transition("Cancelled", "Accepted"));

        // Terminal states
        assert!(order_sm.is_terminal("Completed"));
        assert!(order_sm.is_terminal("Refunded"));
        assert!(order_sm.is_terminal("Cancelled"));
        assert!(!order_sm.is_terminal("Created"));
        assert!(!order_sm.is_terminal("Disputed"));
    }

    #[test]
    fn test_participant_state_machine() {
        let spec = create_aea_spec();
        let sm = spec
            .state_machines
            .iter()
            .find(|sm| sm.name == "ParticipantStatus")
            .unwrap();

        // Valid transitions
        assert!(sm.is_valid_transition("Inactive", "Active"));
        assert!(sm.is_valid_transition("Active", "Cooldown"));
        assert!(sm.is_valid_transition("Active", "Slashed"));

        // Invalid transitions
        assert!(!sm.is_valid_transition("Slashed", "Active"));
        assert!(!sm.is_valid_transition("Inactive", "Slashed"));

        // Terminal
        assert!(sm.is_terminal("Slashed"));
        assert!(!sm.is_terminal("Active"));
    }

    #[test]
    fn test_access_control_generation() {
        let spec = create_aea_spec();

        let confirm_ac = spec
            .access_controls
            .iter()
            .find(|ac| ac.instruction == "ConfirmDelivery")
            .unwrap();

        assert!(confirm_ac.requirements.iter().any(|r| matches!(r,
            AccessRequirement::SignerIs { account, field }
            if account == "order" && field == "buyer"
        )));

        assert!(confirm_ac
            .preconditions
            .contains(&"order.status = Delivered".to_string()));
    }

    #[test]
    fn test_vc_generation() {
        let spec = create_aea_spec();
        let vcs = spec.generate_all_vcs("test.ovsm");

        // Should have VCs for state machines, access control, and invariants
        assert!(!vcs.is_empty());

        // Check for specific VCs
        assert!(vcs.iter().any(|vc| vc.id.contains("StakeAccounting")));
        assert!(vcs.iter().any(|vc| vc.id.contains("OrderStatus")));
        assert!(vcs.iter().any(|vc| vc.id.contains("ConfirmDelivery")));
    }

    #[test]
    fn test_runtime_check_generation() {
        let spec = create_aea_spec();
        let checks = spec.generate_runtime_checks();

        // Should have checks for state machines and access controls
        assert!(!checks.is_empty());

        // Check that state machine validator is generated
        let order_validator = checks
            .iter()
            .find(|c| c.contains("validate-orderstatus-transition"))
            .expect("Should have OrderStatus validator");

        // Should have valid transitions
        assert!(order_validator.contains("Created -> Accepted"));
        assert!(order_validator.contains("Delivered -> Completed"));

        // Should have error handling
        assert!(order_validator.contains("Invalid"));

        println!("Generated state machine validator:\n{}", order_validator);
    }

    #[test]
    fn test_transition_assert_generation() {
        let spec = create_aea_spec();
        let order_sm = spec
            .state_machines
            .iter()
            .find(|sm| sm.name == "OrderStatus")
            .unwrap();

        // Valid transition should not generate an error
        let valid_assert = order_sm.generate_transition_assert(0, 1); // Created -> Accepted
        assert!(
            valid_assert.is_none(),
            "Valid transition should not generate assertion"
        );

        // Invalid transition should generate an error
        let invalid_assert = order_sm.generate_transition_assert(0, 4); // Created -> Completed (invalid!)
        assert!(
            invalid_assert.is_some(),
            "Invalid transition should generate assertion"
        );

        let err_code = invalid_assert.unwrap();
        assert!(err_code.contains("ERROR"));
        assert!(err_code.contains("Invalid"));

        println!("Invalid transition error code:\n{}", err_code);
    }

    #[test]
    fn test_access_control_runtime_check() {
        let spec = create_aea_spec();

        let confirm_ac = spec
            .access_controls
            .iter()
            .find(|ac| ac.instruction == "ConfirmDelivery")
            .unwrap();

        let check_code = confirm_ac.generate_runtime_check();

        // Should check signer
        assert!(check_code.contains("Signer must be order.buyer"));

        println!("Access control check code:\n{}", check_code);
    }

    #[test]
    fn test_access_control_inline_assert() {
        let spec = create_aea_spec();

        let slash_ac = spec
            .access_controls
            .iter()
            .find(|ac| ac.instruction == "SlashParticipant")
            .unwrap();

        let assert_code = slash_ac.generate_inline_assert(2);

        // Should check admin
        assert!(assert_code.contains("Admin check"));
        assert!(assert_code.contains("Not admin"));

        println!("Inline admin check:\n{}", assert_code);
    }

    #[test]
    fn test_spec_extraction_from_expressions() {
        use crate::parser::{Argument, Expression};

        // Simulate parsed __defstate__ call
        let defstate_expr = Expression::ToolCall {
            name: "__defstate__".to_string(),
            args: vec![
                Argument::positional(Expression::StringLiteral("TestStatus".to_string())),
                Argument::positional(Expression::ArrayLiteral(vec![
                    Expression::StringLiteral("A".to_string()),
                    Expression::StringLiteral("B".to_string()),
                    Expression::StringLiteral("C".to_string()),
                ])),
                Argument::positional(Expression::StringLiteral("A".to_string())),
                Argument::positional(Expression::ArrayLiteral(vec![Expression::StringLiteral(
                    "C".to_string(),
                )])),
                Argument::positional(Expression::ArrayLiteral(vec![
                    Expression::ArrayLiteral(vec![
                        Expression::StringLiteral("A".to_string()),
                        Expression::StringLiteral("B".to_string()),
                    ]),
                    Expression::ArrayLiteral(vec![
                        Expression::StringLiteral("B".to_string()),
                        Expression::StringLiteral("C".to_string()),
                    ]),
                ])),
            ],
        };

        let spec = ProtocolSpec::from_expressions(&[defstate_expr]);

        assert_eq!(spec.state_machines.len(), 1);
        let sm = &spec.state_machines[0];
        assert_eq!(sm.name, "TestStatus");
        assert_eq!(sm.states.len(), 3);
        assert!(sm.is_valid_transition("A", "B"));
        assert!(sm.is_valid_transition("B", "C"));
        assert!(!sm.is_valid_transition("A", "C"));
        assert!(sm.is_terminal("C"));
    }
}
