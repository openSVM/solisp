use ovsm::{Evaluator, Parser, Scanner};

fn main() {
    let source = r#"
(let* ((x 10)
       (y (+ x 5)))
  y)
"#;

    println!("Testing: {}", source);

    let mut scanner = Scanner::new(source);
    let tokens = scanner.scan_tokens().expect("Scan failed");

    println!("\nTokens ({}):", tokens.len());
    for (i, token) in tokens.iter().enumerate() {
        println!("  [{}] {:?}", i, token);
    }

    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Parse failed");

    println!("\nProgram parsed OK");

    let mut evaluator = Evaluator::new();
    let result = evaluator.execute(&program).expect("Eval failed");

    println!("\nResult: {:?}", result);
}
