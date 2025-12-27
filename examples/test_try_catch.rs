use ovsm::{Evaluator, Parser, Scanner};

fn main() {
    println!("Testing TRY-CATCH blocks...\n");

    // Test 1: Catch division by zero
    let code1 = r#"
        TRY:
            $x = 10 / 0
        CATCH:
            $x = -1
        RETURN $x
    "#;

    let mut scanner = Scanner::new(code1);
    let tokens = scanner.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut evaluator = Evaluator::new();
    let result = evaluator.execute(&program).unwrap();

    println!("Test 1 - Catch division by zero");
    println!("  Result: {:?}", result);
    assert_eq!(result, ovsm::Value::Int(-1));
    println!("  ✅ Passed\n");

    // Test 2: No error, TRY succeeds
    let code2 = r#"
        TRY:
            $x = 10 / 2
        CATCH:
            $x = -1
        RETURN $x
    "#;

    let mut scanner = Scanner::new(code2);
    let tokens = scanner.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut evaluator = Evaluator::new();
    let result = evaluator.execute(&program).unwrap();

    println!("Test 2 - No error, TRY succeeds");
    println!("  Result: {:?}", result);
    assert_eq!(result, ovsm::Value::Int(5));
    println!("  ✅ Passed\n");

    // Test 3: Catch undefined variable error
    let code3 = r#"
        TRY:
            $result = $undefined_var
        CATCH:
            $result = "caught error"
        RETURN $result
    "#;

    let mut scanner = Scanner::new(code3);
    let tokens = scanner.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut evaluator = Evaluator::new();
    let result = evaluator.execute(&program).unwrap();

    println!("Test 3 - Catch undefined variable");
    println!("  Result: {:?}", result);
    assert_eq!(result, ovsm::Value::String("caught error".to_string()));
    println!("  ✅ Passed\n");

    // Test 4: Catch tool error
    let code4 = r#"
        TRY:
            $arr = []
            $first = FIRST($arr)
        CATCH:
            $first = "empty"
        RETURN $first
    "#;

    let mut scanner = Scanner::new(code4);
    let tokens = scanner.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut evaluator = Evaluator::new();
    let result = evaluator.execute(&program).unwrap();

    println!("Test 4 - Catch tool error (empty collection)");
    println!("  Result: {:?}", result);
    assert_eq!(result, ovsm::Value::String("empty".to_string()));
    println!("  ✅ Passed\n");

    // Test 5: Nested TRY-CATCH
    let code5 = r#"
        TRY:
            TRY:
                $x = 10 / 0
            CATCH:
                $x = 99
            $result = $x
        CATCH:
            $result = -1
        RETURN $result
    "#;

    let mut scanner = Scanner::new(code5);
    let tokens = scanner.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut evaluator = Evaluator::new();
    let result = evaluator.execute(&program).unwrap();

    println!("Test 5 - Nested TRY-CATCH");
    println!("  Result: {:?}", result);
    assert_eq!(result, ovsm::Value::Int(99));
    println!("  ✅ Passed\n");

    println!("✅ All TRY-CATCH tests passed!");
}
