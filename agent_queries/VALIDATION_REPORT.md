
# OVSM Query Validation Report

## Summary Statistics

- **Total Queries:** 100
- **Average Score:** 66.8/100
- **High Quality (≥85):** 5 queries (5.0%)
- **Needs Improvement (<70):** 54 queries (54.0%)

## Score Distribution


| Dimension     | Average | Max |
|---------------|---------|-----|
| Correctness   | 33.3/40 | 40  |
| Clarity       | 15.0/30 | 30  |
| Educational   | 18.5/30 | 30  |
| **Total**     | **66.8/100** | **100** |


## Top 10 Queries

1. **040_double_values.ovsm** - Score: 90/100
2. **044_reverse_array.ovsm** - Score: 90/100
3. **045_string_builder.ovsm** - Score: 90/100
4. **046_array_contains.ovsm** - Score: 90/100
5. **061_array_push_pattern.ovsm** - Score: 85/100
6. **007_variable_definition.ovsm** - Score: 80/100
7. **008_variable_mutation.ovsm** - Score: 80/100
8. **010_simple_if.ovsm** - Score: 80/100
9. **019_cond_multiple.ovsm** - Score: 80/100
10. **020_range_creation.ovsm** - Score: 80/100

## Queries Needing Improvement

- **075_deep_nested_structure.ovsm** - Score: 45/100
  - ⚠️ Missing LISP parentheses
  - ⚠️ Description could be more specific
  - ⚠️ Add inline comments to explain complex logic
  - 💡 Add comments explaining the logic flow
- **094_blockchain_timestamp.ovsm** - Score: 45/100
  - ⚠️ Expected output not clearly typed
  - ⚠️ Description could be more specific
  - ⚠️ Add inline comments to explain complex logic
  - 💡 Add comments explaining the logic flow
- **002_variadic_addition.ovsm** - Score: 50/100
  - ⚠️ Expected output not clearly typed
  - ⚠️ Add inline comments to explain complex logic
  - ⚠️ Unclear what concept this teaches
  - 💡 Add comments explaining the logic flow
- **009_constant_definition.ovsm** - Score: 50/100
  - ⚠️ Expected output not clearly typed
  - ⚠️ Add inline comments to explain complex logic
  - ⚠️ Unclear what concept this teaches
  - 💡 Add comments explaining the logic flow
- **023_arithmetic_precedence.ovsm** - Score: 50/100
  - ⚠️ Expected output not clearly typed
  - ⚠️ Add inline comments to explain complex logic
  - ⚠️ Unclear what concept this teaches
  - 💡 Add comments explaining the logic flow
- **026_simple_while.ovsm** - Score: 52/100
  - ⚠️ Expected output not clearly typed
  - ⚠️ Use descriptive variable names instead of single letters
  - ⚠️ Add inline comments to explain complex logic
  - 💡 Rename variables: x -> count, n -> number, etc.
- **003_multiplication.ovsm** - Score: 55/100
  - ⚠️ Expected output not clearly typed
  - ⚠️ Description could be more specific
  - ⚠️ Unclear what concept this teaches
- **004_division.ovsm** - Score: 55/100
  - ⚠️ Expected output not clearly typed
  - ⚠️ Description could be more specific
  - ⚠️ Unclear what concept this teaches
- **006_nested_arithmetic.ovsm** - Score: 55/100
  - ⚠️ Expected output not clearly typed
  - ⚠️ Description could be more specific
  - ⚠️ Unclear what concept this teaches
- **022_log_message.ovsm** - Score: 55/100
  - ⚠️ Description could be more specific
  - ⚠️ Add inline comments to explain complex logic
  - ⚠️ Unclear what concept this teaches
  - 💡 Add comments explaining the logic flow

## Most Common Issues

- Add inline comments to explain complex logic: 84 queries (84.0%)
- Unclear what concept this teaches: 61 queries (61.0%)
- Expected output not clearly typed: 57 queries (57.0%)
- Description could be more specific: 46 queries (46.0%)
- Missing LISP parentheses: 10 queries (10.0%)
- Found 1 anti-pattern(s): 8 queries (8.0%)
- Some variable names could be more descriptive: 6 queries (6.0%)
- Use descriptive variable names instead of single letters: 2 queries (2.0%)
