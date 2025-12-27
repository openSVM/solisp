use ovsm::{Parser, Scanner};

fn main() {
    let source = r#"
        $x = 42
        $y = 10
        $result = $x + $y * 2

        IF $result > 50 THEN
            $message = "high"
        ELSE
            $message = "low"

        RETURN $message
    "#;

    println!("=== OVSM Parser Demo ===\n");
    println!("Source code:\n{}\n", source);

    // Tokenize
    let mut scanner = Scanner::new(source);
    let tokens = match scanner.scan_tokens() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            return;
        }
    };

    println!("Tokens: {} generated\n", tokens.len());

    // Parse
    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(program) => {
            println!("✓ Parse successful!\n");
            println!("AST structure:");
            println!("{:#?}", program);
        }
        Err(e) => {
            eprintln!("✗ Parse error: {}", e);
        }
    }
}
