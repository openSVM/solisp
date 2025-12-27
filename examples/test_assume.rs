use ovsm::compiler::{CompileOptions, Compiler, VerificationMode};
use std::fs;

fn main() {
    let source = fs::read_to_string("/tmp/test_assume.ovsm").expect("Failed to read file");

    let mut opts = CompileOptions::default();
    opts.verification_mode = VerificationMode::Require;

    let compiler = Compiler::new(opts);

    match compiler.compile(&source) {
        Ok(result) => {
            println!("Compilation SUCCESS!");
            println!("ELF size: {} bytes", result.elf_bytes.len());

            if let Some(ref fv) = result.formal_verification {
                println!("\nFormal Verification Results:");
                println!("  Proved:  {}", fv.proved.len());
                println!("  Failed:  {}", fv.failed.len());
                println!("  Unknown: {}", fv.unknown.len());

                for p in &fv.proved {
                    println!("  ✓ {}", p.description);
                }
                for u in &fv.unknown {
                    println!("  ? {} - {}", u.description, u.reason);
                }
            }
        }
        Err(e) => {
            println!("Compilation FAILED: {}", e);
        }
    }
}
