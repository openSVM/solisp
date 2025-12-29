# Test BREAK and CONTINUE

## Test: Simple FOR without BREAK

```solisp
$sum = 0
FOR $i IN [1..5]:
    $sum = $sum + $i
RETURN $sum
```

## Test: FOR with BREAK

```solisp
$sum = 0
FOR $i IN [1..10]:
    IF $i > 5 THEN
        BREAK
    $sum = $sum + $i
RETURN $sum
```

## Test: Simpler BREAK

```solisp
FOR $i IN [1..10]:
    IF $i > 5 THEN
        BREAK
RETURN "done"
```
