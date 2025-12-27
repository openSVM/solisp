//! Debug VC generation
use ovsm::compiler::lean::{LeanCodegen, VerificationProperties};
use ovsm::{SExprParser as Parser, SExprScanner as Scanner};

fn main() {
    let source = r#"
(do
  (define src-bal 100)
  (define amount 50)
  (if (< src-bal amount)
      1
      (- src-bal amount)))
"#;

    let mut scanner = Scanner::new(source);
    let tokens = scanner.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();

    let codegen = LeanCodegen::new(VerificationProperties::all());
    let vcs = codegen.generate(&program, "test.ovsm").unwrap();

    println!("Generated {} VCs:\n", vcs.len());
    for vc in &vcs {
        println!("ID: {}", vc.id);
        println!("Category: {:?}", vc.category);
        println!("Property: {}", vc.property);
        println!("Assumptions: {:?}", vc.assumptions);
        println!("Description: {}", vc.description);
        println!();
    }
}
