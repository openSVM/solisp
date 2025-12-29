# Minimal Reproduction

## Test 1: FOR with assignment (works)

```solisp
FOR $i IN [1..3]:
    $x = 1
RETURN "ok"
```

## Test 2: FOR with IF (fails?)

```solisp
FOR $i IN [1..3]:
    IF true THEN
        $x = 1
RETURN "ok"
```

## Test 3: Access $i before IF

```solisp
FOR $i IN [1..3]:
    $before = $i
    IF true THEN
        $x = 1
RETURN "ok"
```

## Test 4: Access $i before AND after IF

```solisp
FOR $i IN [1..3]:
    $before = $i
    IF true THEN
        $x = 1
    $after = $i
RETURN "ok"
```
