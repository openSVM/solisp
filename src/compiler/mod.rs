//! # OVSM Compiler - LISP to sBPF Bytecode
//!
//! This module compiles OVSM LISP programs to Solana BPF (sBPF) bytecode
//! that can be deployed and executed on the Solana blockchain.
//!
//! ## Architecture
//!
//! ```text
//! OVSM Source → AST → Type Check → IR → Optimize → sBPF → ELF (.so)
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! use solisp::compiler::{Compiler, CompileOptions};
//!
//! let source = "(define x 42)";
//! let compiler = Compiler::new(CompileOptions::default());
//! let elf_bytes = compiler.compile(source)?;
//! std::fs::write("program.so", elf_bytes)?;
//! ```

pub mod anchor_idl;
pub mod debug;
pub mod elf;
pub mod formal_verification;
pub mod graph_coloring;
pub mod ir;
pub mod lean;
pub mod optimizer;
pub mod regalloc_analyzer;
pub mod runtime;
pub mod sbpf_codegen;
pub mod solana_abi;
pub mod types;
pub mod verifier;

pub use debug::{debug_compile, disassemble_sbpf, dump_ir, extract_text_section, validate_sbpf};
pub use elf::ElfWriter;
pub use ir::{IrGenerator, IrInstruction, IrProgram, IrReg};
pub use optimizer::Optimizer;
pub use regalloc_analyzer::{InstructionAnalysis, RegAllocAnalyzer, RegAllocIssue, RegAllocReport};
pub use runtime::{ArrayRuntime, HeapAllocator, StackFrame, StringRuntime};
pub use sbpf_codegen::{
    memory, syscall_hash, SbpfCodegen, SbpfInstruction, SbpfReg, SolanaSymbols,
};
pub use types::{OvsmType, TypeChecker, TypeEnv};
pub use verifier::{Verifier, VerifyError, VerifyResult};

use crate::{Error, Program, Result, SExprParser as Parser, SExprScanner as Scanner};

/// SBPF bytecode version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SbpfVersion {
    /// V1 with relocations (devnet, current production)
    V1,
    /// V2 with static syscalls (future)
    V2,
}

/// Type checking mode for compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TypeCheckMode {
    /// No additional type checking (use existing IR-level checker only)
    #[default]
    Legacy,
    /// Gradual typing - untyped code gets Type::Any, no errors for missing annotations
    Gradual,
    /// Strict typing - all type mismatches are errors
    Strict,
}

/// Formal verification mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerificationMode {
    /// Skip formal verification entirely
    Skip,
    /// Warn on verification failures but continue compilation
    Warn,
    /// Require all verification conditions to pass (default for safety)
    #[default]
    Require,
}

/// Compilation options
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Optimization level (0-3)
    pub opt_level: u8,
    /// Target compute budget (for CU estimation)
    pub compute_budget: u64,
    /// Emit debug info
    pub debug_info: bool,
    /// Generate source map
    pub source_map: bool,
    /// SBPF version to generate (V1 with relocations or V2 with static calls)
    pub sbpf_version: SbpfVersion,
    /// Enable Solana ABI compliant entrypoint with deserialization
    pub enable_solana_abi: bool,
    /// Enable bidirectional type inference (gradual = false for strict checking)
    pub type_check_mode: TypeCheckMode,
    /// Formal verification mode (Lean 4 theorem proving)
    pub verification_mode: VerificationMode,
    /// Formal verification options (when verification_mode != Skip)
    pub verification_options: lean::VerificationOptions,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            opt_level: 2,
            compute_budget: 200_000,
            debug_info: false,
            source_map: false,
            sbpf_version: SbpfVersion::V1, // V1 with relocations for comparison
            enable_solana_abi: false,      // Temporarily disabled while fixing opcode issues
            type_check_mode: TypeCheckMode::Legacy, // Use existing checker by default
            verification_mode: VerificationMode::Require, // Require formal verification by default
            verification_options: lean::VerificationOptions::default(),
        }
    }
}

/// Compilation result with metadata
#[derive(Debug)]
pub struct CompileResult {
    /// ELF binary bytes (ready to deploy)
    pub elf_bytes: Vec<u8>,
    /// Estimated compute units
    pub estimated_cu: u64,
    /// Number of IR instructions
    pub ir_instruction_count: usize,
    /// Number of sBPF instructions
    pub sbpf_instruction_count: usize,
    /// Warnings generated during compilation
    pub warnings: Vec<String>,
    /// Verification result (sBPF bytecode verification)
    pub verification: Option<VerifyResult>,
    /// Type errors from bidirectional checker (if enabled)
    pub type_errors: Vec<String>,
    /// Formal verification result (Lean 4 theorem proving)
    pub formal_verification: Option<lean::VerificationResult>,
}

/// OVSM to sBPF Compiler
pub struct Compiler {
    options: CompileOptions,
}

impl Compiler {
    /// Create a new compiler with options
    pub fn new(options: CompileOptions) -> Self {
        Self { options }
    }

    /// Compile OVSM source code to ELF binary
    pub fn compile(&self, source: &str) -> Result<CompileResult> {
        // Phase 1: Parse
        let mut scanner = Scanner::new(source);
        let tokens = scanner.scan_tokens()?;
        let mut parser = Parser::new(tokens);
        let mut program = parser.parse()?;

        // Phase 1.25: Protocol spec extraction and runtime check injection
        let protocol_spec = lean::ProtocolSpec::from_program(&program);
        if protocol_spec.has_specs() {
            // Generate runtime checks from protocol specs
            let runtime_checks = protocol_spec.generate_runtime_checks();

            // Parse the generated checks and prepend to program
            if !runtime_checks.is_empty() {
                // Join all check functions into a single source string
                let checks_source = runtime_checks.join("\n\n");
                let mut check_scanner = Scanner::new(&checks_source);
                if let Ok(check_tokens) = check_scanner.scan_tokens() {
                    let mut check_parser = Parser::new(check_tokens);
                    if let Ok(check_program) = check_parser.parse() {
                        // Prepend runtime check functions to the program
                        let mut new_statements = check_program.statements;
                        new_statements.append(&mut program.statements);
                        program.statements = new_statements;
                    }
                }
            }
        }

        // Phase 1.5: Bidirectional type checking (if enabled)
        let mut type_errors = Vec::new();
        if self.options.type_check_mode != TypeCheckMode::Legacy {
            use crate::types::BidirectionalChecker;

            let mut bidir_checker = match self.options.type_check_mode {
                TypeCheckMode::Strict => BidirectionalChecker::strict(),
                _ => BidirectionalChecker::new(), // Gradual mode
            };

            // Run bidirectional type inference
            for stmt in &program.statements {
                if let crate::parser::Statement::Expression(expr) = stmt {
                    bidir_checker.synth(expr);
                }
            }

            // Collect any errors
            for err in bidir_checker.errors() {
                type_errors.push(err.to_string());
            }

            // In strict mode, fail on type errors
            if self.options.type_check_mode == TypeCheckMode::Strict && !type_errors.is_empty() {
                return Err(Error::compiler(format!(
                    "Type errors: {}",
                    type_errors.join("; ")
                )));
            }
        }

        // Phase 1.75: Formal verification (Lean 4)
        let formal_verification = self.run_formal_verification(&program, "<source>")?;

        // Phase 2: Type check (legacy IR-level checker)
        let mut type_checker = TypeChecker::new();
        let typed_program = type_checker.check(&program)?;

        // Phase 3: Generate IR
        let mut ir_gen = IrGenerator::new();
        let mut ir_program = ir_gen.generate(&typed_program)?;

        // Inject Solana entrypoint wrapper for proper ABI handling
        if self.options.enable_solana_abi {
            solana_abi::inject_entrypoint_wrapper(&mut ir_program.instructions);
        }

        // Phase 4: Optimize
        if self.options.opt_level > 0 {
            let mut optimizer = Optimizer::new(self.options.opt_level);
            optimizer.optimize(&mut ir_program);
        }

        // Phase 5: Generate sBPF
        let mut codegen = SbpfCodegen::new(self.options.sbpf_version);
        let sbpf_program = codegen.generate(&ir_program)?;

        // Phase 6: Verify
        let verifier = Verifier::new();
        let verification = verifier.verify(&sbpf_program);

        // Check for fatal verification errors
        if !verification.valid {
            let error_msgs: Vec<String> =
                verification.errors.iter().map(|e| e.to_string()).collect();
            return Err(Error::compiler(format!(
                "Verification failed: {}",
                error_msgs.join("; ")
            )));
        }

        // Phase 7: Package as ELF
        let mut elf_writer = ElfWriter::new();

        // Convert syscall call sites to ELF relocation format
        let syscall_refs: Vec<crate::compiler::elf::SyscallRef> = codegen
            .syscall_sites
            .iter()
            .map(|site| crate::compiler::elf::SyscallRef {
                offset: site.offset,
                name: site.name.clone(),
            })
            .collect();

        // Convert string load sites to ELF format for patching
        let string_load_refs: Vec<crate::compiler::elf::StringLoadRef> = codegen
            .string_load_sites
            .iter()
            .map(|site| crate::compiler::elf::StringLoadRef {
                offset: site.offset,
                rodata_offset: site.rodata_offset,
            })
            .collect();

        // V1 requires relocations, V2 embeds syscall hashes statically
        let elf_bytes = match self.options.sbpf_version {
            SbpfVersion::V1 => {
                // V1: Must use write_with_syscalls to generate relocations
                elf_writer.write_with_syscalls(
                    &sbpf_program,
                    &syscall_refs,
                    &string_load_refs,
                    &codegen.rodata,
                    self.options.debug_info,
                    self.options.sbpf_version,
                )?
            }
            SbpfVersion::V2 => {
                // V2: No relocations needed, syscalls are embedded
                elf_writer.write(
                    &sbpf_program,
                    self.options.debug_info,
                    self.options.sbpf_version,
                )?
            }
        };

        // Combine warnings
        let mut warnings = type_checker.warnings().to_vec();
        warnings.extend(verification.warnings.clone());

        // Add formal verification warnings
        if let Some(ref fv) = formal_verification {
            for failed in &fv.failed {
                warnings.push(format!(
                    "Verification warning: {} at {:?}",
                    failed.description, failed.location
                ));
            }
        }

        Ok(CompileResult {
            elf_bytes,
            estimated_cu: verification.stats.estimated_cu,
            ir_instruction_count: ir_program.instructions.len(),
            sbpf_instruction_count: sbpf_program.len(),
            warnings,
            verification: Some(verification),
            type_errors,
            formal_verification,
        })
    }

    /// Run formal verification using built-in verifier (Lean 4 compatible)
    ///
    /// This uses a pure Rust verification engine that doesn't require external tools.
    /// Proofs are Lean 4 compatible and can be exported for external verification.
    fn run_formal_verification(
        &self,
        program: &Program,
        source_file: &str,
    ) -> Result<Option<lean::VerificationResult>> {
        // Skip if verification is disabled
        if self.options.verification_mode == VerificationMode::Skip {
            return Ok(None);
        }

        // Create verifier - this always succeeds since we use built-in verification
        let verifier = lean::LeanVerifier::new(self.options.verification_options.clone())
            .map_err(|e| Error::compiler(format!("Failed to initialize verifier: {}", e)))?;

        // Run built-in verification (no external Lean 4 required)
        let result = verifier.verify_builtin(program, source_file)?;

        // Handle failures based on mode
        if !result.all_proved() {
            match self.options.verification_mode {
                VerificationMode::Skip => {
                    // Shouldn't reach here, but just in case
                    return Ok(Some(result));
                }
                VerificationMode::Warn => {
                    for failed in &result.failed {
                        tracing::warn!(
                            "Verification condition failed: {} ({})",
                            failed.description,
                            failed.error
                        );
                    }
                    for unknown in &result.unknown {
                        tracing::warn!(
                            "Verification condition unknown: {} ({})",
                            unknown.description,
                            unknown.reason
                        );
                    }
                    return Ok(Some(result));
                }
                VerificationMode::Require => {
                    let mut error_msg =
                        String::from("Formal verification failed - compilation blocked:\n\n");

                    if !result.failed.is_empty() {
                        error_msg.push_str("FAILED (definitely unsafe):\n");
                        for failed in &result.failed {
                            error_msg.push_str(&format!("  {} {}\n", "✗", failed.description));
                            if let Some(loc) = &failed.location {
                                error_msg.push_str(&format!("    at {}\n", loc));
                            }
                            error_msg.push_str(&format!("    reason: {}\n", failed.error));
                            if let Some(suggestion) = &failed.suggestion {
                                error_msg.push_str(&format!("    fix: {}\n", suggestion));
                            }
                            error_msg.push('\n');
                        }
                    }

                    if !result.unknown.is_empty() {
                        error_msg.push_str("UNVERIFIED (cannot prove safety):\n");
                        for unknown in &result.unknown {
                            error_msg.push_str(&format!("  {} {}\n", "?", unknown.description));
                            if let Some(loc) = &unknown.location {
                                error_msg.push_str(&format!("    at {}\n", loc));
                            }
                            error_msg.push_str(&format!("    reason: {}\n", unknown.reason));
                            error_msg.push('\n');
                        }
                    }

                    error_msg.push_str("To compile anyway, use --verification-mode=warn or --verification-mode=skip\n");
                    return Err(Error::compiler(error_msg));
                }
            }
        }

        Ok(Some(result))
    }

    /// Compile from already-parsed AST
    pub fn compile_ast(&self, program: &Program) -> Result<CompileResult> {
        // Bidirectional type checking (if enabled)
        let mut type_errors = Vec::new();
        if self.options.type_check_mode != TypeCheckMode::Legacy {
            use crate::types::BidirectionalChecker;

            let mut bidir_checker = match self.options.type_check_mode {
                TypeCheckMode::Strict => BidirectionalChecker::strict(),
                _ => BidirectionalChecker::new(),
            };

            for stmt in &program.statements {
                if let crate::parser::Statement::Expression(expr) = stmt {
                    bidir_checker.synth(expr);
                }
            }

            for err in bidir_checker.errors() {
                type_errors.push(err.to_string());
            }

            if self.options.type_check_mode == TypeCheckMode::Strict && !type_errors.is_empty() {
                return Err(Error::compiler(format!(
                    "Type errors: {}",
                    type_errors.join("; ")
                )));
            }
        }

        // Formal verification (Lean 4)
        let formal_verification = self.run_formal_verification(program, "<ast>")?;

        let mut type_checker = TypeChecker::new();
        let typed_program = type_checker.check(program)?;

        let mut ir_gen = IrGenerator::new();
        let mut ir_program = ir_gen.generate(&typed_program)?;

        // Inject Solana entrypoint wrapper for proper ABI handling
        if self.options.enable_solana_abi {
            solana_abi::inject_entrypoint_wrapper(&mut ir_program.instructions);
        }

        if self.options.opt_level > 0 {
            let mut optimizer = Optimizer::new(self.options.opt_level);
            optimizer.optimize(&mut ir_program);
        }

        let mut codegen = SbpfCodegen::new(self.options.sbpf_version);
        let sbpf_program = codegen.generate(&ir_program)?;

        // Verify
        let verifier = Verifier::new();
        let verification = verifier.verify(&sbpf_program);

        if !verification.valid {
            let error_msgs: Vec<String> =
                verification.errors.iter().map(|e| e.to_string()).collect();
            return Err(Error::compiler(format!(
                "Verification failed: {}",
                error_msgs.join("; ")
            )));
        }

        // Convert syscall call sites to ELF relocation format
        let syscall_refs: Vec<crate::compiler::elf::SyscallRef> = codegen
            .syscall_sites
            .iter()
            .map(|site| crate::compiler::elf::SyscallRef {
                offset: site.offset,
                name: site.name.clone(),
            })
            .collect();

        // Convert string load sites to ELF format for patching
        let string_load_refs: Vec<crate::compiler::elf::StringLoadRef> = codegen
            .string_load_sites
            .iter()
            .map(|site| crate::compiler::elf::StringLoadRef {
                offset: site.offset,
                rodata_offset: site.rodata_offset,
            })
            .collect();

        let mut elf_writer = ElfWriter::new();

        // V1 requires relocations, V2 embeds syscall hashes statically
        let elf_bytes = match self.options.sbpf_version {
            SbpfVersion::V1 => {
                // V1: Must use write_with_syscalls to generate relocations
                elf_writer.write_with_syscalls(
                    &sbpf_program,
                    &syscall_refs,
                    &string_load_refs,
                    &codegen.rodata,
                    self.options.debug_info,
                    self.options.sbpf_version,
                )?
            }
            SbpfVersion::V2 => {
                // V2: No relocations needed, syscalls are embedded
                elf_writer.write(
                    &sbpf_program,
                    self.options.debug_info,
                    self.options.sbpf_version,
                )?
            }
        };

        let mut warnings = type_checker.warnings().to_vec();
        warnings.extend(verification.warnings.clone());

        // Add formal verification warnings
        if let Some(ref fv) = formal_verification {
            for failed in &fv.failed {
                warnings.push(format!(
                    "Verification warning: {} at {:?}",
                    failed.description, failed.location
                ));
            }
        }

        Ok(CompileResult {
            elf_bytes,
            estimated_cu: verification.stats.estimated_cu,
            ir_instruction_count: ir_program.instructions.len(),
            sbpf_instruction_count: sbpf_program.len(),
            warnings,
            verification: Some(verification),
            type_errors,
            formal_verification,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_creation() {
        let compiler = Compiler::new(CompileOptions::default());
        assert_eq!(compiler.options.opt_level, 2);
    }
}
