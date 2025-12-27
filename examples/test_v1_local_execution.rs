//! Test V1 SBPF program execution locally with solana-rbpf
//!
//! Note: This example was designed for an older version of solana_rbpf.
//! The API has since changed significantly:
//! - `register_syscalls` has been removed
//! - `BuiltinProgram` is no longer public
//! - `execute_program` method has been removed
//!
//! For ELF testing, see instead:
//! - validate_elf.rs - ELF validation
//! - debug_elf_parser.rs - Uses goblin crate for ELF parsing
//! - debug_elf_symbols.rs - Symbol table inspection

fn main() {
    println!("⚠️  This example is deprecated.");
    println!();
    println!("The solana_rbpf API has changed significantly in newer versions:");
    println!("  - register_syscalls has been removed");
    println!("  - BuiltinProgram is no longer public");
    println!("  - execute_program method has been removed");
    println!();
    println!("For ELF testing, use these alternatives:");
    println!();
    println!("  cargo run --example validate_elf -- <file.so>");
    println!("  cargo run --example debug_elf_parser -- <file.so>");
    println!("  cargo run --example debug_elf_symbols -- <file.so>");
}
