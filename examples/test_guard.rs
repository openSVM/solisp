use ovsm::{Evaluator, Parser, Scanner};

fn main() {
    println!("Testing GUARD clauses...\n");

    // Test 1: Guard passes
    let code = r#"
        $x = 10
        GUARD $x > 0 ELSE
            RETURN -1
        RETURN $x
    "#;

    let mut scanner = Scanner::new(code);
    let tokens = scanner.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut evaluator = Evaluator::new();
    let result = evaluator.execute(&program).unwrap();

    println!("Test 1 - Guard passes ($x = 10, guard $x > 0)");
    println!("  Result: {:?}", result);
    assert_eq!(result, ovsm::Value::Int(10));
    println!("  ✅ Passed\n");

    // Test 2: Guard fails
    let code2 = r#"
        $x = -5
        GUARD $x > 0 ELSE
            RETURN -1
        RETURN $x
    "#;

    let mut scanner = Scanner::new(code2);
    let tokens = scanner.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut evaluator = Evaluator::new();
    let result = evaluator.execute(&program).unwrap();

    println!("Test 2 - Guard fails ($x = -5, guard $x > 0)");
    println!("  Result: {:?}", result);
    assert_eq!(result, ovsm::Value::Int(-1));
    println!("  ✅ Passed\n");

    // Test 3: Multiple guards
    let code3 = r#"
        $x = 10
        $y = 20

        GUARD $x > 0 ELSE
            RETURN -1

        GUARD $y > 15 ELSE
            RETURN -2

        RETURN $x + $y
    "#;

    let mut scanner = Scanner::new(code3);
    let tokens = scanner.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut evaluator = Evaluator::new();
    let result = evaluator.execute(&program).unwrap();

    println!("Test 3 - Multiple guards (both pass)");
    println!("  Result: {:?}", result);
    assert_eq!(result, ovsm::Value::Int(30));
    println!("  ✅ Passed\n");

    // Test 4: Second guard fails
    let code4 = r#"
        $x = 10
        $y = 5

        GUARD $x > 0 ELSE
            RETURN -1

        GUARD $y > 15 ELSE
            RETURN -2

        RETURN $x + $y
    "#;

    let mut scanner = Scanner::new(code4);
    let tokens = scanner.scan_tokens().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut evaluator = Evaluator::new();
    let result = evaluator.execute(&program).unwrap();

    println!("Test 4 - Second guard fails ($y = 5, guard $y > 15)");
    println!("  Result: {:?}", result);
    assert_eq!(result, ovsm::Value::Int(-2));
    println!("  ✅ Passed\n");

    println!("✅ All GUARD clause tests passed!");
}
