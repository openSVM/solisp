# Feature Coverage Test - All v1.1.0 Features

## Test 1: Variables

```solisp
$x = 10
RETURN $x
```

## Test 2: Constants

```solisp
CONST PI = 3.14
RETURN PI
```

## Test 3: Arithmetic (all operators)

```solisp
$a = 10 + 5
$b = 10 - 5
$c = 10 * 5
$d = 10 / 5
$e = 10 % 3
$f = 2 ** 8
RETURN $f
```

## Test 4: Comparisons

```solisp
$a = 10 > 5
$b = 10 < 5
$c = 10 >= 10
$d = 10 <= 10
$e = 10 == 10
$f = 10 != 5
RETURN $f
```

## Test 5: Logical operators

```solisp
$a = true AND true
$b = true OR false
$c = NOT false
RETURN $c
```

## Test 6: IF-THEN-ELSE

```solisp
$x = 10
IF $x > 5 THEN
    $result = "greater"
ELSE
    $result = "smaller"
RETURN $result
```

## Test 7: WHILE loop

```solisp
$i = 0
$sum = 0
WHILE $i < 5:
    $sum = $sum + $i
    $i = $i + 1
RETURN $sum
```

## Test 8: FOR loop

```solisp
$sum = 0
FOR $i IN [1..5]:
    $sum = $sum + $i
RETURN $sum
```

## Test 9: BREAK

```solisp
$sum = 0
FOR $i IN [1..10]:
    IF $i > 5 THEN
        BREAK
    $sum = $sum + $i
RETURN $sum
```

## Test 10: CONTINUE

```solisp
$sum = 0
FOR $i IN [1..10]:
    IF $i % 2 == 0 THEN
        CONTINUE
    $sum = $sum + $i
RETURN $sum
```

## Test 11: GUARD clause

```solisp
$x = 10
GUARD $x > 0 ELSE
    RETURN "negative"
RETURN "positive"
```

## Test 12: TRY-CATCH

```solisp
TRY:
    $result = 10 / 0
CATCH:
    $result = -1
RETURN $result
```

## Test 13: Arrays

```solisp
$arr = [1, 2, 3, 4, 5]
RETURN $arr[2]
```

## Test 14: Objects

```solisp
$obj = {name: "Alice", age: 30}
RETURN $obj.name
```

## Test 15: Ranges

```solisp
$sum = 0
FOR $i IN [1..10]:
    $sum = $sum + 1
RETURN $sum
```

## Test 16: SUM tool

```solisp
$numbers = [1, 2, 3, 4, 5]
RETURN SUM($numbers)
```

## Test 17: MAX tool

```solisp
$numbers = [5, 2, 8, 1, 9]
RETURN MAX($numbers)
```

## Test 18: MIN tool

```solisp
$numbers = [5, 2, 8, 1, 9]
RETURN MIN($numbers)
```

## Test 19: COUNT tool

```solisp
$arr = [1, 2, 3, 4, 5]
RETURN COUNT($arr)
```

## Test 20: APPEND tool

```solisp
$arr = [1, 2, 3]
$new = APPEND($arr, 4)
RETURN $new
```
