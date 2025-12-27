//! Test formal verification on real OVSM programs
use ovsm::compiler::{CompileOptions, Compiler, VerificationMode};

fn main() {
    let source = std::fs::read_to_string("examples/real_world/sol_transfer.ovsm")
        .expect("Failed to read sol_transfer.ovsm");

    println!("=== Testing Formal Verification on sol_transfer.ovsm ===\n");

    // First try with Require mode
    let mut options = CompileOptions::default();
    options.verification_mode = VerificationMode::Require;

    let compiler = Compiler::new(options);

    match compiler.compile(&source) {
        Ok(result) => {
            println!("✅ Compilation SUCCESS!");
            println!("   ELF size: {} bytes", result.elf_bytes.len());
            println!("   IR instructions: {}", result.ir_instruction_count);
            println!("   sBPF instructions: {}", result.sbpf_instruction_count);

            if let Some(fv) = &result.formal_verification {
                println!("\n📋 Formal Verification Results:");
                println!("   Proved:  {}", fv.proved.len());
                println!("   Failed:  {}", fv.failed.len());
                println!("   Unknown: {}", fv.unknown.len());

                if !fv.proved.is_empty() {
                    println!("\n   ✓ Proved:");
                    for p in &fv.proved {
                        println!("     - {}", p.description);
                    }
                }
                if !fv.failed.is_empty() {
                    println!("\n   ✗ Failed:");
                    for f in &fv.failed {
                        println!("     - {}", f.description);
                        println!("       Error: {}", f.error);
                    }
                }
                if !fv.unknown.is_empty() {
                    println!("\n   ? Unknown:");
                    for u in &fv.unknown {
                        println!("     - {}", u.description);
                        println!("       Reason: {}", u.reason);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Compilation BLOCKED by formal verification:\n");
            println!("{}", e);
        }
    }
}
