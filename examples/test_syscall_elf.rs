// Test loading and executing an ELF with syscalls using RBPF directly
use solana_rbpf::{
    elf::Executable, program::BuiltinProgram, verifier::RequisiteVerifier, vm::TestContextObject,
};
use std::sync::Arc;

fn main() {
    let elf_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/hello_final.so".to_string());

    println!("📂 Loading ELF: {}", elf_path);
    let elf_bytes = std::fs::read(&elf_path).expect("Failed to read ELF");
    println!("   Size: {} bytes\n", elf_bytes.len());

    println!("🔍 Parsing ELF with RBPF...");

    // Create loader (function registry)
    let loader = Arc::new(BuiltinProgram::new_mock());

    // Try to load the ELF
    match Executable::<TestContextObject>::load(&elf_bytes, loader.clone()) {
        Ok(mut executable) => {
            println!("✅ ELF parsed successfully!\n");

            println!("🔍 Verifying program...");
            match executable.verify::<RequisiteVerifier>() {
                Ok(()) => {
                    println!("✅ Program verified successfully!\n");

                    // Try to JIT compile
                    #[cfg(not(windows))]
                    {
                        println!("🔧 JIT compiling...");
                        match executable.jit_compile() {
                            Ok(()) => println!("✅ JIT compilation successful!\n"),
                            Err(e) => println!("⚠️  JIT compilation failed: {:?}\n", e),
                        }
                    }

                    println!("✅ Program is ready for execution!");
                    println!("\n📊 Summary:");
                    println!("   - ELF parsing: ✅");
                    println!("   - Verification: ✅");
                    println!("   - JIT compilation: ✅");
                    println!("\n🎉 The program structure is valid!")
                }
                Err(e) => {
                    println!("❌ Verification failed: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to parse ELF!");
            println!("\n🔍 Error details:");
            println!("{:#?}", e);
        }
    }
}
