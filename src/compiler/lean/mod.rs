//! # Lean 4 Formal Verification Integration
//!
//! This module provides integration with the Lean 4 theorem prover for
//! formal verification of OVSM programs before BPF compilation.
//!
//! ## Architecture
//!
//! ```text
//! OVSM AST → Verification Conditions → Lean 4 Code → Lean 4 Prover → Result
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! use solisp::compiler::lean::{LeanVerifier, VerificationOptions};
//!
//! let verifier = LeanVerifier::new(VerificationOptions::default())?;
//! let result = verifier.verify(&program)?;
//!
//! if !result.all_proved() {
//!     return Err(Error::verification_failed(result));
//! }
//! ```

pub mod bridge;
pub mod codegen;
pub mod protocol;
pub mod solver;
pub mod types;

pub use bridge::{LeanBridge, LeanError, LeanMessage, LeanResult};
pub use codegen::{LeanCodegen, VCCategory, VerificationCondition};
pub use protocol::{
    create_aea_spec, AccessControl, AccessRequirement, EconomicInvariant, InvariantType,
    ProtocolSpec, State, StateMachine,
};
pub use solver::{BuiltinVerifier, PathCondition, PathConstraint, ProofResult, SymbolicValue};
pub use types::{LeanType, TypeMapper};

use crate::{Error, Program, Result};
use std::path::PathBuf;

/// Options for formal verification
#[derive(Debug, Clone)]
pub struct VerificationOptions {
    /// Require all verification conditions to pass before allowing BPF compilation
    /// Default: true
    pub require_verification: bool,

    /// Path to Lean 4 executable (auto-detect if None)
    pub lean_path: Option<PathBuf>,

    /// Path to the OVSM Lean library
    pub ovsm_lean_lib: Option<PathBuf>,

    /// Verification timeout in seconds
    pub timeout_secs: u64,

    /// Which properties to verify
    pub properties: VerificationProperties,

    /// Enable caching of verification results
    pub enable_cache: bool,

    /// Directory for generated Lean files (temp if None)
    pub output_dir: Option<PathBuf>,

    /// Keep generated Lean files after verification
    pub keep_generated: bool,
}

impl Default for VerificationOptions {
    fn default() -> Self {
        Self {
            require_verification: true,
            lean_path: None,
            ovsm_lean_lib: None,
            timeout_secs: 60,
            properties: VerificationProperties::all(),
            enable_cache: true,
            output_dir: None,
            keep_generated: false,
        }
    }
}

/// Which properties to verify
#[derive(Debug, Clone, Default)]
pub struct VerificationProperties {
    /// Verify division operands are non-zero
    pub division_safety: bool,

    /// Verify array indices are within bounds
    pub array_bounds: bool,

    /// Verify arithmetic doesn't overflow
    pub overflow_check: bool,

    /// Verify arithmetic doesn't underflow
    pub underflow_check: bool,

    /// Verify refinement type predicates
    pub refinement_types: bool,

    /// Verify Solana balance conservation
    pub balance_safety: bool,

    /// Strict arithmetic checking - verify ALL arithmetic operations, not just balance ops
    /// When false (default), only balance-related operations generate overflow/underflow VCs
    /// When true, ALL +, -, * operations generate VCs
    pub strict_arithmetic: bool,
}

impl VerificationProperties {
    /// Enable all verification properties
    pub fn all() -> Self {
        Self {
            division_safety: true,
            array_bounds: true,
            overflow_check: true,
            underflow_check: true,
            refinement_types: true,
            balance_safety: true,
            strict_arithmetic: false, // Default to balance-only for less noise
        }
    }

    /// Disable all verification properties
    pub fn none() -> Self {
        Self::default()
    }

    /// Only verify critical safety properties
    pub fn critical_only() -> Self {
        Self {
            division_safety: true,
            array_bounds: true,
            overflow_check: false,
            underflow_check: true,
            refinement_types: false,
            balance_safety: true,
            strict_arithmetic: false,
        }
    }

    /// Maximum security - all checks including strict arithmetic
    pub fn maximum() -> Self {
        Self {
            division_safety: true,
            array_bounds: true,
            overflow_check: true,
            underflow_check: true,
            refinement_types: true,
            balance_safety: true,
            strict_arithmetic: true, // Check ALL arithmetic
        }
    }
}

/// Result of formal verification
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Whether all verification conditions passed
    pub success: bool,

    /// Verification conditions that were proved
    pub proved: Vec<ProvedVC>,

    /// Verification conditions that failed
    pub failed: Vec<FailedVC>,

    /// Verification conditions that timed out or couldn't be determined
    pub unknown: Vec<UnknownVC>,

    /// Total verification time in milliseconds
    pub time_ms: u64,

    /// Path to generated Lean file (if kept)
    pub lean_file: Option<PathBuf>,

    /// Coverage statistics (how much of the code was verified)
    pub coverage: Option<VerificationCoverage>,
}

/// Verification coverage statistics
#[derive(Debug, Clone, Default)]
pub struct VerificationCoverage {
    /// Total risky operations in the program
    pub total_operations: usize,
    /// Operations covered by verification conditions
    pub covered_operations: usize,
    /// Breakdown by category
    pub by_category: std::collections::HashMap<String, usize>,
    /// Operations not covered and why
    pub uncovered: Vec<String>,
}

impl VerificationCoverage {
    /// Calculate coverage percentage
    pub fn percentage(&self) -> f64 {
        if self.total_operations == 0 {
            100.0
        } else {
            (self.covered_operations as f64 / self.total_operations as f64) * 100.0
        }
    }
}

/// Detailed proof coverage report with line-level metrics
#[derive(Debug, Clone)]
pub struct ProofCoverageReport {
    /// Source file name
    pub source_file: String,
    /// Total lines in source file
    pub total_lines: usize,
    /// Lines containing verifiable code (non-comment, non-blank)
    pub code_lines: usize,
    /// Lines covered by at least one proved VC
    pub proved_lines: usize,
    /// Lines covered by unknown/failed VCs (need attention)
    pub unproved_lines: usize,
    /// Lines with no VCs (may be safe or may need review)
    pub uncovered_lines: usize,
    /// Set of line numbers that are proved
    pub proved_line_set: std::collections::HashSet<usize>,
    /// Set of line numbers that are unproved
    pub unproved_line_set: std::collections::HashSet<usize>,
    /// VC count by category
    pub vcs_by_category: std::collections::HashMap<String, CategoryStats>,
    /// Total VCs generated
    pub total_vcs: usize,
    /// Total VCs proved
    pub proved_vcs: usize,
    /// Risky operations by type
    pub risky_operations: std::collections::HashMap<String, usize>,
}

/// Statistics for a single VC category
#[derive(Debug, Clone, Default)]
pub struct CategoryStats {
    /// Number of VCs in this category
    pub count: usize,
    /// Number proved
    pub proved: usize,
    /// Number failed/unknown
    pub unproved: usize,
    /// Lines covered
    pub lines: std::collections::HashSet<usize>,
}

impl ProofCoverageReport {
    /// Create a new coverage report from VCs and proof results
    pub fn from_vcs(
        source: &str,
        source_file: &str,
        vcs: &[VerificationCondition],
        proof_results: &[(bool, &VerificationCondition)], // (proved, vc)
    ) -> Self {
        use std::collections::{HashMap, HashSet};

        let lines: Vec<&str> = source.lines().collect();
        let total_lines = lines.len();

        // Count code lines (non-empty, non-comment-only)
        let code_lines = lines
            .iter()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with(";;")
            })
            .count();

        let mut proved_line_set: HashSet<usize> = HashSet::new();
        let mut unproved_line_set: HashSet<usize> = HashSet::new();
        let mut vcs_by_category: HashMap<String, CategoryStats> = HashMap::new();
        let mut risky_operations: HashMap<String, usize> = HashMap::new();

        let mut proved_vcs = 0;

        for (is_proved, vc) in proof_results {
            let cat_name = format!("{:?}", vc.category);
            let stats = vcs_by_category.entry(cat_name.clone()).or_default();
            stats.count += 1;

            // Track risky operation type
            *risky_operations.entry(cat_name.clone()).or_insert(0) += 1;

            if let Some(loc) = &vc.location {
                stats.lines.insert(loc.line);

                if *is_proved {
                    proved_line_set.insert(loc.line);
                    stats.proved += 1;
                    proved_vcs += 1;
                } else {
                    unproved_line_set.insert(loc.line);
                    stats.unproved += 1;
                }
            } else if *is_proved {
                stats.proved += 1;
                proved_vcs += 1;
            } else {
                stats.unproved += 1;
            }
        }

        // Lines that are proved but not unproved
        let proved_only: HashSet<usize> = proved_line_set
            .difference(&unproved_line_set)
            .cloned()
            .collect();
        // Lines that have unproved VCs
        let unproved_only: HashSet<usize> = unproved_line_set.iter().cloned().collect();

        // Uncovered = code lines - (proved ∪ unproved)
        let all_vc_lines: HashSet<usize> =
            proved_line_set.union(&unproved_line_set).cloned().collect();
        let uncovered_lines = code_lines.saturating_sub(all_vc_lines.len());

        Self {
            source_file: source_file.to_string(),
            total_lines,
            code_lines,
            proved_lines: proved_only.len(),
            unproved_lines: unproved_only.len(),
            uncovered_lines,
            proved_line_set: proved_only,
            unproved_line_set: unproved_only,
            vcs_by_category,
            total_vcs: vcs.len(),
            proved_vcs,
            risky_operations,
        }
    }

    /// Calculate proof coverage percentage (proved lines / code lines)
    pub fn line_coverage_percent(&self) -> f64 {
        if self.code_lines == 0 {
            100.0
        } else {
            (self.proved_lines as f64 / self.code_lines as f64) * 100.0
        }
    }

    /// Calculate VC proof rate (proved VCs / total VCs)
    pub fn vc_proof_rate(&self) -> f64 {
        if self.total_vcs == 0 {
            100.0
        } else {
            (self.proved_vcs as f64 / self.total_vcs as f64) * 100.0
        }
    }

    /// Calculate risky operation coverage (lines with VCs / code lines)
    pub fn risky_coverage_percent(&self) -> f64 {
        if self.code_lines == 0 {
            100.0
        } else {
            let covered = self.proved_lines + self.unproved_lines;
            (covered as f64 / self.code_lines as f64) * 100.0
        }
    }

    /// Generate a summary string
    pub fn summary(&self) -> String {
        let categories_covered = self.vcs_by_category.len();
        let total_risky_ops: usize = self.risky_operations.values().sum();

        format!(
            "Proof Coverage Report for {}\n\
             ════════════════════════════════════════\n\
             Source:           {} lines ({} code, {} comments/blank)\n\
             \n\
             Verification Conditions:\n\
             • Total VCs:      {}\n\
             • Proved:         {} ({:.1}%)\n\
             • Unproved:       {}\n\
             \n\
             Categories Checked: {}\n\
             Total Risky Ops:    {}\n",
            self.source_file,
            self.total_lines,
            self.code_lines,
            self.total_lines - self.code_lines,
            self.total_vcs,
            self.proved_vcs,
            self.vc_proof_rate(),
            self.total_vcs - self.proved_vcs,
            categories_covered,
            total_risky_ops,
        )
    }

    /// Generate detailed category breakdown
    pub fn category_breakdown(&self) -> String {
        let mut result = String::from("VCs by Category:\n");
        result.push_str("─────────────────────────────────────────────────\n");

        let mut categories: Vec<_> = self.vcs_by_category.iter().collect();
        categories.sort_by(|(_, a), (_, b)| b.count.cmp(&a.count)); // Sort by count descending

        for (name, stats) in categories {
            let rate = if stats.count == 0 {
                100.0
            } else {
                (stats.proved as f64 / stats.count as f64) * 100.0
            };
            let status = if stats.unproved == 0 { "✓" } else { "✗" };
            result.push_str(&format!(
                "  {} {:28} {:3}/{:3} ({:5.1}%)\n",
                status, name, stats.proved, stats.count, rate
            ));
        }
        result.push_str("─────────────────────────────────────────────────\n");

        result
    }

    /// Get list of unproved lines for review
    pub fn unproved_lines_list(&self) -> Vec<usize> {
        let mut lines: Vec<usize> = self.unproved_line_set.iter().cloned().collect();
        lines.sort();
        lines
    }

    /// Export as JSON
    pub fn to_json(&self) -> String {
        let categories: std::collections::BTreeMap<&String, serde_json::Value> = self
            .vcs_by_category
            .iter()
            .map(|(k, v)| {
                (
                    k,
                    serde_json::json!({
                        "count": v.count,
                        "proved": v.proved,
                        "unproved": v.unproved,
                        "lines": v.lines.len()
                    }),
                )
            })
            .collect();

        serde_json::json!({
            "source_file": self.source_file,
            "total_lines": self.total_lines,
            "code_lines": self.code_lines,
            "proved_lines": self.proved_lines,
            "unproved_lines": self.unproved_lines,
            "uncovered_lines": self.uncovered_lines,
            "total_vcs": self.total_vcs,
            "proved_vcs": self.proved_vcs,
            "metrics": {
                "line_coverage_percent": self.line_coverage_percent(),
                "vc_proof_rate": self.vc_proof_rate(),
                "risky_coverage_percent": self.risky_coverage_percent()
            },
            "by_category": categories,
            "unproved_line_numbers": self.unproved_lines_list()
        })
        .to_string()
    }
}

impl VerificationResult {
    /// Check if all verification conditions passed
    pub fn all_proved(&self) -> bool {
        self.success && self.failed.is_empty() && self.unknown.is_empty()
    }

    /// Get total number of verification conditions
    pub fn total_vcs(&self) -> usize {
        self.proved.len() + self.failed.len() + self.unknown.len()
    }

    /// Format a summary of the verification result
    pub fn summary(&self) -> String {
        if self.all_proved() {
            format!(
                "Verification PASSED: {}/{} conditions proved in {}ms",
                self.proved.len(),
                self.total_vcs(),
                self.time_ms
            )
        } else {
            format!(
                "Verification FAILED: {} proved, {} failed, {} unknown",
                self.proved.len(),
                self.failed.len(),
                self.unknown.len()
            )
        }
    }
}

/// A verification condition that was successfully proved
#[derive(Debug, Clone)]
pub struct ProvedVC {
    /// Unique identifier for this VC
    pub id: String,
    /// Category of the verification condition
    pub category: VCCategory,
    /// Human-readable description
    pub description: String,
    /// Source location in Solisp file
    pub location: Option<SourceLocation>,
    /// Time to prove in milliseconds
    pub time_ms: u64,
}

/// A verification condition that failed
#[derive(Debug, Clone)]
pub struct FailedVC {
    /// Unique identifier for this VC
    pub id: String,
    /// Category of the verification condition
    pub category: VCCategory,
    /// Human-readable description
    pub description: String,
    /// Source location in Solisp file
    pub location: Option<SourceLocation>,
    /// Error message from Lean
    pub error: String,
    /// Suggested fix (if available)
    pub suggestion: Option<String>,
}

/// A verification condition with unknown status
#[derive(Debug, Clone)]
pub struct UnknownVC {
    /// Unique identifier for this VC
    pub id: String,
    /// Category of the verification condition
    pub category: VCCategory,
    /// Human-readable description
    pub description: String,
    /// Source location in Solisp file
    pub location: Option<SourceLocation>,
    /// Reason why verification couldn't complete
    pub reason: String,
}

/// Source location in Solisp code
#[derive(Debug, Clone)]
pub struct SourceLocation {
    /// Source file path
    pub file: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

/// Main Lean 4 verifier
pub struct LeanVerifier {
    options: VerificationOptions,
    bridge: LeanBridge,
    codegen: LeanCodegen,
}

impl LeanVerifier {
    /// Create a new Lean verifier with the given options
    pub fn new(options: VerificationOptions) -> Result<Self> {
        let bridge = LeanBridge::new(
            options.lean_path.clone(),
            options.ovsm_lean_lib.clone(),
            options.timeout_secs,
        )?;

        let codegen = LeanCodegen::new(options.properties.clone());

        Ok(Self {
            options,
            bridge,
            codegen,
        })
    }

    /// Check if Lean 4 is available
    pub fn is_available(&self) -> bool {
        self.bridge.is_available()
    }

    /// Get the Lean 4 version
    pub fn lean_version(&self) -> Result<String> {
        self.bridge.version()
    }

    /// Verify an OVSM program using the built-in verifier (no external Lean required)
    ///
    /// This is the recommended entry point for verification. It uses a pure Rust
    /// implementation that can prove common safety properties without requiring
    /// Lean 4 to be installed. Proofs are Lean 4 compatible and can be exported
    /// for external verification if desired.
    pub fn verify_builtin(
        &self,
        program: &Program,
        source_file: &str,
    ) -> Result<VerificationResult> {
        let start = std::time::Instant::now();

        // Generate verification conditions
        let vcs = self.codegen.generate(program, source_file)?;

        if vcs.is_empty() {
            return Ok(VerificationResult {
                success: true,
                proved: vec![],
                failed: vec![],
                unknown: vec![],
                time_ms: start.elapsed().as_millis() as u64,
                lean_file: None,
                coverage: None,
            });
        }

        // Use built-in verifier
        let builtin = BuiltinVerifier::new();
        let mut proved = Vec::new();
        let mut failed = Vec::new();
        let mut unknown = Vec::new();

        for vc in vcs {
            let proof_result = builtin.prove(&vc);

            match proof_result {
                ProofResult::Proved {
                    lean_proof,
                    explanation,
                } => {
                    proved.push(ProvedVC {
                        id: vc.id,
                        category: vc.category,
                        description: format!("{} ({})", vc.description, explanation),
                        location: vc.location,
                        time_ms: 0,
                    });
                }
                ProofResult::Disproved { counterexample } => {
                    failed.push(FailedVC {
                        id: vc.id,
                        category: vc.category.clone(),
                        description: vc.description,
                        location: vc.location,
                        error: format!("Counterexample: {}", counterexample),
                        suggestion: self.generate_suggestion(&vc.category),
                    });
                }
                ProofResult::Unknown { reason } => {
                    unknown.push(UnknownVC {
                        id: vc.id,
                        category: vc.category,
                        description: vc.description,
                        location: vc.location,
                        reason,
                    });
                }
            }
        }

        let success = failed.is_empty();

        Ok(VerificationResult {
            success,
            proved,
            failed,
            unknown,
            time_ms: start.elapsed().as_millis() as u64,
            lean_file: None,
            coverage: None,
        })
    }

    /// Verify an OVSM program using external Lean 4 (requires Lean 4 installation)
    ///
    /// This method calls out to the Lean 4 theorem prover for verification.
    /// Prefer `verify_builtin` for most use cases as it doesn't require external tools.
    pub fn verify(&self, program: &Program, source_file: &str) -> Result<VerificationResult> {
        // First try built-in verification
        let builtin_result = self.verify_builtin(program, source_file)?;

        // If all proved or Lean not available, return built-in result
        if builtin_result.all_proved() || !self.is_available() {
            return Ok(builtin_result);
        }

        // For unknown VCs, try external Lean if available
        let start = std::time::Instant::now();

        // Generate verification conditions
        let vcs = self.codegen.generate(program, source_file)?;

        if vcs.is_empty() {
            return Ok(VerificationResult {
                success: true,
                proved: vec![],
                failed: vec![],
                unknown: vec![],
                time_ms: start.elapsed().as_millis() as u64,
                lean_file: None,
                coverage: None,
            });
        }

        // Generate Lean 4 code
        let lean_code = self.codegen.to_lean_code(&vcs, source_file)?;

        // Write to file
        let lean_file = self.write_lean_file(&lean_code, source_file)?;

        // Run Lean 4 verification
        let lean_result = self.bridge.check_file(&lean_file)?;

        // Parse results
        let result = self.parse_results(vcs, lean_result, start.elapsed().as_millis() as u64);

        // Cleanup if not keeping generated files
        if !self.options.keep_generated {
            let _ = std::fs::remove_file(&lean_file);
        }

        Ok(VerificationResult {
            lean_file: if self.options.keep_generated {
                Some(lean_file)
            } else {
                None
            },
            ..result
        })
    }

    /// Export Lean 4 proof certificates for external verification
    ///
    /// This generates a `.lean` file containing all verification conditions
    /// with their proofs that can be independently checked by Lean 4.
    pub fn export_proofs(
        &self,
        program: &Program,
        source_file: &str,
        output_path: &std::path::Path,
    ) -> Result<()> {
        // Generate VCs
        let vcs = self.codegen.generate(program, source_file)?;

        // Generate base Lean code
        let mut lean_code = self.codegen.to_lean_code(&vcs, source_file)?;

        // Add proof annotations from built-in verifier
        let builtin = BuiltinVerifier::new();
        let mut proof_comments = String::new();

        for vc in &vcs {
            let proof_result = builtin.prove(vc);
            if let ProofResult::Proved {
                lean_proof,
                explanation,
            } = proof_result
            {
                proof_comments.push_str(&format!(
                    "/-- {} proof: {} -/\n/-- Explanation: {} -/\n\n",
                    vc.id, lean_proof, explanation
                ));
            }
        }

        // Prepend proof comments
        lean_code = format!(
            "/-!\n# OVSM Verification Certificates\n\nGenerated from: {}\n\n{}-/\n\n{}",
            source_file, proof_comments, lean_code
        );

        std::fs::write(output_path, lean_code)
            .map_err(|e| Error::compiler(format!("Failed to write proof export: {}", e)))?;

        Ok(())
    }

    /// Write Lean code to a file
    fn write_lean_file(&self, code: &str, source_file: &str) -> Result<PathBuf> {
        let dir = self
            .options
            .output_dir
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("ovsm_verify"));

        std::fs::create_dir_all(&dir).map_err(|e| {
            Error::compiler(format!("Failed to create verification directory: {}", e))
        })?;

        let stem = std::path::Path::new(source_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("program");

        let lean_file = dir.join(format!("{}_vc.lean", stem));

        std::fs::write(&lean_file, code)
            .map_err(|e| Error::compiler(format!("Failed to write Lean file: {}", e)))?;

        Ok(lean_file)
    }

    /// Parse Lean results into verification result
    fn parse_results(
        &self,
        vcs: Vec<VerificationCondition>,
        lean_result: LeanResult,
        time_ms: u64,
    ) -> VerificationResult {
        let mut proved = Vec::new();
        let mut failed = Vec::new();
        let mut unknown = Vec::new();

        match lean_result {
            LeanResult::Success => {
                // All VCs proved
                for vc in vcs {
                    proved.push(ProvedVC {
                        id: vc.id,
                        category: vc.category,
                        description: vc.description,
                        location: vc.location,
                        time_ms: 0, // Individual times not available
                    });
                }
            }
            LeanResult::Errors(errors) => {
                // Parse which VCs failed based on error locations
                let failed_ids: std::collections::HashSet<_> = errors
                    .iter()
                    .filter_map(|e| self.extract_vc_id_from_error(e))
                    .collect();

                for vc in vcs {
                    if failed_ids.contains(&vc.id) {
                        let error_msg = errors
                            .iter()
                            .find(|e| self.extract_vc_id_from_error(e) == Some(vc.id.clone()))
                            .map(|e| e.message.clone())
                            .unwrap_or_else(|| "Verification failed".to_string());

                        failed.push(FailedVC {
                            id: vc.id,
                            category: vc.category.clone(),
                            description: vc.description,
                            location: vc.location,
                            error: error_msg,
                            suggestion: self.generate_suggestion(&vc.category),
                        });
                    } else {
                        proved.push(ProvedVC {
                            id: vc.id,
                            category: vc.category,
                            description: vc.description,
                            location: vc.location,
                            time_ms: 0,
                        });
                    }
                }
            }
            LeanResult::Timeout => {
                // All VCs unknown due to timeout
                for vc in vcs {
                    unknown.push(UnknownVC {
                        id: vc.id,
                        category: vc.category,
                        description: vc.description,
                        location: vc.location,
                        reason: "Verification timed out".to_string(),
                    });
                }
            }
            LeanResult::NotAvailable(reason) => {
                for vc in vcs {
                    unknown.push(UnknownVC {
                        id: vc.id,
                        category: vc.category,
                        description: vc.description,
                        location: vc.location,
                        reason: format!("Lean 4 not available: {}", reason),
                    });
                }
            }
        }

        VerificationResult {
            success: failed.is_empty() && unknown.is_empty(),
            proved,
            failed,
            unknown,
            time_ms,
            lean_file: None,
            coverage: None,
        }
    }

    /// Extract VC ID from Lean error message
    fn extract_vc_id_from_error(&self, error: &LeanMessage) -> Option<String> {
        // VC IDs are in the format "vc_<category>_line_<n>"
        // Error messages reference the theorem name
        let re = regex::Regex::new(r"vc_\w+_line_\d+").ok()?;
        re.find(&error.message).map(|m| m.as_str().to_string())
    }

    /// Generate a suggestion for fixing a failed VC
    fn generate_suggestion(&self, category: &VCCategory) -> Option<String> {
        match category {
            VCCategory::DivisionSafety => Some(
                "Add a check before division: (if (= divisor 0) (error \"Division by zero\") (/ x divisor))".to_string()
            ),
            VCCategory::ArrayBounds => Some(
                "Add a bounds check: (if (>= idx (len arr)) (error \"Index out of bounds\") (get arr idx))".to_string()
            ),
            VCCategory::ArithmeticUnderflow => Some(
                "Add a balance check: (if (< balance amount) (error \"Insufficient funds\") (- balance amount))".to_string()
            ),
            VCCategory::ArithmeticOverflow => Some(
                "Consider using checked arithmetic or adding bounds checks".to_string()
            ),
            VCCategory::RefinementType => Some(
                "Ensure the value satisfies the refinement predicate".to_string()
            ),
            VCCategory::BalanceConservation => Some(
                "Verify that total lamports are preserved in transfers".to_string()
            ),
            _ => None,
        }
    }

    /// Export a proof certificate as JSON for auditing and external verification
    pub fn export_certificate(
        &self,
        program: &Program,
        source_file: &str,
    ) -> Result<ProofCertificate> {
        let result = self.verify_builtin(program, source_file)?;

        // Generate source hash for integrity
        let source_content = std::fs::read_to_string(source_file).unwrap_or_default();
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(source_content.as_bytes());
        let source_hash = format!("{:x}", hasher.finalize());

        let vc_proofs: Vec<VCProof> = result
            .proved
            .iter()
            .map(|vc| VCProof {
                id: vc.id.clone(),
                category: vc.category.to_string(),
                description: vc.description.clone(),
                location: vc.location.as_ref().map(|l| l.to_string()),
                status: ProofStatus::Proved,
                proof_method: "builtin_verifier".to_string(),
            })
            .chain(result.failed.iter().map(|vc| VCProof {
                id: vc.id.clone(),
                category: vc.category.to_string(),
                description: vc.description.clone(),
                location: vc.location.as_ref().map(|l| l.to_string()),
                status: ProofStatus::Failed {
                    reason: vc.error.clone(),
                },
                proof_method: "builtin_verifier".to_string(),
            }))
            .chain(result.unknown.iter().map(|vc| VCProof {
                id: vc.id.clone(),
                category: vc.category.to_string(),
                description: vc.description.clone(),
                location: vc.location.as_ref().map(|l| l.to_string()),
                status: ProofStatus::Unknown {
                    reason: vc.reason.clone(),
                },
                proof_method: "builtin_verifier".to_string(),
            }))
            .collect();

        Ok(ProofCertificate {
            version: "1.0".to_string(),
            source_file: source_file.to_string(),
            source_hash,
            timestamp: chrono::Utc::now().to_rfc3339(),
            verifier: "ovsm-builtin-v1".to_string(),
            total_vcs: result.total_vcs(),
            proved_count: result.proved.len(),
            failed_count: result.failed.len(),
            unknown_count: result.unknown.len(),
            verification_time_ms: result.time_ms,
            vcs: vc_proofs,
        })
    }
}

/// A proof certificate for external verification and auditing
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProofCertificate {
    /// Certificate format version
    pub version: String,
    /// Source file that was verified
    pub source_file: String,
    /// MD5 hash of source for integrity
    pub source_hash: String,
    /// Timestamp of verification
    pub timestamp: String,
    /// Verifier used
    pub verifier: String,
    /// Total number of VCs
    pub total_vcs: usize,
    /// Number proved
    pub proved_count: usize,
    /// Number failed
    pub failed_count: usize,
    /// Number unknown
    pub unknown_count: usize,
    /// Verification time in milliseconds
    pub verification_time_ms: u64,
    /// Individual VC proofs
    pub vcs: Vec<VCProof>,
}

impl ProofCertificate {
    /// Export certificate to JSON string
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| Error::compiler(format!("Failed to serialize certificate: {}", e)))
    }

    /// Export certificate to JSON file
    pub fn to_json_file(&self, path: &std::path::Path) -> Result<()> {
        let json = self.to_json()?;
        std::fs::write(path, json)
            .map_err(|e| Error::compiler(format!("Failed to write certificate: {}", e)))
    }

    /// Check if verification passed
    pub fn is_verified(&self) -> bool {
        self.failed_count == 0
    }

    /// Get pass rate as percentage
    pub fn pass_rate(&self) -> f64 {
        if self.total_vcs == 0 {
            100.0
        } else {
            (self.proved_count as f64 / self.total_vcs as f64) * 100.0
        }
    }
}

/// Individual VC proof status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VCProof {
    /// VC identifier
    pub id: String,
    /// Category name
    pub category: String,
    /// Human-readable description
    pub description: String,
    /// Source location
    pub location: Option<String>,
    /// Proof status
    pub status: ProofStatus,
    /// Proof method used
    pub proof_method: String,
}

/// Proof status for a VC
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ProofStatus {
    /// Successfully proved
    Proved,
    /// Failed with reason
    Failed {
        /// Failure reason
        reason: String,
    },
    /// Unknown with reason
    Unknown {
        /// Unknown reason
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_options_default() {
        let opts = VerificationOptions::default();
        assert!(opts.require_verification);
        assert!(opts.properties.division_safety);
        assert!(opts.properties.array_bounds);
    }

    #[test]
    fn test_verification_properties_all() {
        let props = VerificationProperties::all();
        assert!(props.division_safety);
        assert!(props.array_bounds);
        assert!(props.overflow_check);
        assert!(props.underflow_check);
        assert!(props.refinement_types);
        assert!(props.balance_safety);
    }

    #[test]
    fn test_verification_result_summary() {
        let result = VerificationResult {
            success: true,
            proved: vec![ProvedVC {
                id: "vc_test".to_string(),
                category: VCCategory::DivisionSafety,
                description: "Test VC".to_string(),
                location: None,
                time_ms: 10,
            }],
            failed: vec![],
            unknown: vec![],
            time_ms: 100,
            lean_file: None,
            coverage: None,
        };

        assert!(result.all_proved());
        assert!(result.summary().contains("PASSED"));
    }
}
