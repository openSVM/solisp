use ovsm::{Scanner, TokenKind};

fn main() {
    let source = r#"
        // Simple OVSM program
        $x = 42
        $y = 10
        $result = $x + $y

        IF $result > 50 THEN
            $message = "high"
        ELSE
            $message = "low"

        RETURN $message
    "#;

    println!("=== OVSM Lexer Demo ===\n");
    println!("Source code:\n{}\n", source);

    let mut scanner = Scanner::new(source);
    match scanner.scan_tokens() {
        Ok(tokens) => {
            println!("Tokens generated: {}\n", tokens.len());
            println!("Token stream:");
            for (i, token) in tokens.iter().enumerate() {
                if !matches!(token.kind, TokenKind::Newline | TokenKind::Eof) {
                    println!("  {}: {:?} at line {}", i, token.kind, token.line);
                }
            }
        }
        Err(e) => {
            eprintln!("Lexer error: {}", e);
        }
    }
}
