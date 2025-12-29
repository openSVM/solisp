# Debug FOR + RETURN

## Test 1: Simple FOR with RETURN

```solisp
FOR $i IN [1..5]:
    RETURN $i
```

## Test 2: FOR with assignment then RETURN

```solisp
FOR $i IN [1..5]:
    $x = $i
    RETURN $x
```

## Test 3: FOR without RETURN

```solisp
$result = 0
FOR $i IN [1..5]:
    $result = $i
RETURN $result
```

## Test 4: FOR with IF but no RETURN inside loop

```solisp
$result = 0
FOR $i IN [1..5]:
    IF $i > 3 THEN
        $result = 100
    ELSE
        $result = $i
RETURN $result
```
