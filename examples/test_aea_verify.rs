//! Test formal verification on AEA Protocol
use ovsm::compiler::{CompileOptions, Compiler, VerificationMode};

fn main() {
    let source = std::fs::read_to_string("../../examples/ovsm_scripts/aea/aea_protocol.ovsm")
        .or_else(|_| {
            std::fs::read_to_string(
                "/home/larp/larpdevs/osvm-cli/examples/ovsm_scripts/aea/aea_protocol.ovsm",
            )
        })
        .expect("Failed to read aea_protocol.ovsm");

    println!("=== Testing Formal Verification on AEA Protocol ===\n");
    println!(
        "Source size: {} bytes, {} lines\n",
        source.len(),
        source.lines().count()
    );

    // First try with Warn mode to see all VCs
    let mut options = CompileOptions::default();
    options.verification_mode = VerificationMode::Warn;

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

                if !fv.proved.is_empty() && fv.proved.len() <= 20 {
                    println!("\n   ✓ Proved:");
                    for p in &fv.proved {
                        println!("     - {}", p.description);
                    }
                } else if !fv.proved.is_empty() {
                    println!(
                        "\n   ✓ Proved: {} conditions (showing first 10)",
                        fv.proved.len()
                    );
                    for p in fv.proved.iter().take(10) {
                        println!("     - {}", p.description);
                    }
                }

                if !fv.failed.is_empty() {
                    println!("\n   ✗ Failed:");
                    for f in fv.failed.iter().take(10) {
                        println!("     - {}", f.description);
                        println!("       Error: {}", f.error);
                    }
                    if fv.failed.len() > 10 {
                        println!("     ... and {} more", fv.failed.len() - 10);
                    }
                }

                if !fv.unknown.is_empty() {
                    println!("\n   ? Unknown:");
                    for u in fv.unknown.iter().take(10) {
                        println!("     - {}", u.description);
                        println!("       Reason: {}", u.reason);
                    }
                    if fv.unknown.len() > 10 {
                        println!("     ... and {} more", fv.unknown.len() - 10);
                    }
                }

                // Calculate verification rate
                let total = fv.proved.len() + fv.failed.len() + fv.unknown.len();
                if total > 0 {
                    let rate = (fv.proved.len() as f64 / total as f64) * 100.0;
                    println!(
                        "\n📊 Verification Rate: {:.1}% ({}/{})",
                        rate,
                        fv.proved.len(),
                        total
                    );
                }

                // Show coverage breakdown by category
                println!("\n📈 VC Coverage by Category:");
                let mut by_category: std::collections::HashMap<String, (usize, usize, usize)> =
                    std::collections::HashMap::new();
                for p in &fv.proved {
                    let cat = format!("{:?}", p.category);
                    let entry = by_category.entry(cat).or_insert((0, 0, 0));
                    entry.0 += 1;
                }
                for f in &fv.failed {
                    let cat = format!("{:?}", f.category);
                    let entry = by_category.entry(cat).or_insert((0, 0, 0));
                    entry.1 += 1;
                }
                for u in &fv.unknown {
                    let cat = format!("{:?}", u.category);
                    let entry = by_category.entry(cat).or_insert((0, 0, 0));
                    entry.2 += 1;
                }
                let mut cats: Vec<_> = by_category.iter().collect();
                cats.sort_by(|a, b| (b.1 .0 + b.1 .1 + b.1 .2).cmp(&(a.1 .0 + a.1 .1 + a.1 .2)));
                for (cat, (proved, failed, unknown)) in cats {
                    let total = proved + failed + unknown;
                    let status = if *failed > 0 {
                        "✗"
                    } else if *unknown > 0 {
                        "?"
                    } else {
                        "✓"
                    };
                    println!(
                        "   {} {:25} {:>3} proved, {:>2} failed, {:>2} unknown",
                        status, cat, proved, failed, unknown
                    );
                }
            }
        }
        Err(e) => {
            println!("❌ Compilation FAILED:\n{}", e);
        }
    }
}
