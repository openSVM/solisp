/-
  OVSM.Primitives - Checked arithmetic operations and safety lemmas
  
  This module provides:
  - Checked arithmetic operations that require safety proofs
  - Lemmas for proving arithmetic safety properties
  - Division and modulo safety
-/

import OVSM.Prelude

namespace OVSM

/-! ## Division Safety -/

/-- Proof that a value is non-zero (division safety precondition) -/
def NonZero (n : UInt64) : Prop := n ≠ 0

/-- Proof that a natural number is non-zero -/
def NonZeroNat (n : Nat) : Prop := n ≠ 0

/-- Safe division: requires proof that divisor is non-zero -/
def safeDiv (x y : UInt64) (h : NonZero y) : UInt64 := x / y

/-- Safe modulo: requires proof that divisor is non-zero -/
def safeMod (x y : UInt64) (h : NonZero y) : UInt64 := x % y

/-- Division by non-zero literal is safe -/
theorem div_by_nonzero_literal (x : UInt64) (n : UInt64) (hn : n ≠ 0) : NonZero n := hn

/-- Division safety from comparison -/
theorem div_safe_from_gt_zero (y : UInt64) (h : y > 0) : NonZero y := by
  intro heq
  simp [heq] at h

/-! ## Overflow Safety -/

/-- Proof that addition won't overflow -/
def AddNoOverflow (x y : UInt64) : Prop :=
  x.toNat + y.toNat ≤ U64_MAX

/-- Proof that subtraction won't underflow -/
def SubNoUnderflow (x y : UInt64) : Prop :=
  y.toNat ≤ x.toNat

/-- Proof that multiplication won't overflow -/
def MulNoOverflow (x y : UInt64) : Prop :=
  x.toNat * y.toNat ≤ U64_MAX

/-- Safe addition with overflow check -/
def safeAdd (x y : UInt64) (h : AddNoOverflow x y) : UInt64 := x + y

/-- Safe subtraction with underflow check -/
def safeSub (x y : UInt64) (h : SubNoUnderflow x y) : UInt64 := x - y

/-- Safe multiplication with overflow check -/
def safeMul (x y : UInt64) (h : MulNoOverflow x y) : UInt64 := x * y

/-! ## Arithmetic Lemmas -/

/-- Addition of small values doesn't overflow -/
theorem add_small_no_overflow (x y : UInt64) 
    (hx : x.toNat ≤ U64_MAX / 2) 
    (hy : y.toNat ≤ U64_MAX / 2) : 
    AddNoOverflow x y := by
  unfold AddNoOverflow U64_MAX
  omega

/-- Subtraction is safe when RHS ≤ LHS -/
theorem sub_safe_when_leq (x y : UInt64) (h : y ≤ x) : SubNoUnderflow x y := by
  unfold SubNoUnderflow
  exact UInt64.le_def.mp h

/-- Zero is safe divisor check fails -/
theorem zero_not_nonzero : ¬NonZero 0 := by
  intro h
  exact h rfl

/-- Positive literal is non-zero -/
theorem pos_literal_nonzero (n : UInt64) (h : n.toNat > 0) : NonZero n := by
  intro heq
  simp [heq] at h

/-! ## Comparison-Based Safety -/

/-- If we've checked x ≥ y, then subtraction is safe -/
theorem sub_safe_from_geq (x y : UInt64) (h : x ≥ y) : SubNoUnderflow x y := by
  unfold SubNoUnderflow
  exact UInt64.le_def.mp h

/-- If we've checked x > y, then subtraction is safe and non-zero -/
theorem sub_safe_from_gt (x y : UInt64) (h : x > y) : SubNoUnderflow x y := by
  unfold SubNoUnderflow
  have : y ≤ x := Nat.lt_succ_iff.mp (Nat.lt_succ_of_lt (UInt64.lt_def.mp h))
  omega

/-- If check ¬(x < y) passed, then x ≥ y -/
theorem geq_from_not_lt (x y : UInt64) (h : ¬(x < y)) : x ≥ y := by
  exact Nat.not_lt.mp (fun hlt => h (UInt64.lt_def.mpr hlt))

/-! ## Checked Arithmetic Results -/

/-- Result of checked addition -/
inductive CheckedAddResult (x y : UInt64) where
  | ok (result : UInt64) (h : result.toNat = x.toNat + y.toNat)
  | overflow

/-- Result of checked subtraction -/
inductive CheckedSubResult (x y : UInt64) where
  | ok (result : UInt64) (h : result.toNat = x.toNat - y.toNat)
  | underflow

/-- Checked addition that returns overflow indicator -/
def checkedAdd (x y : UInt64) : CheckedAddResult x y :=
  if h : x.toNat + y.toNat ≤ U64_MAX then
    -- When sum is in range, UInt64 addition equals natural addition
    .ok (x + y) (by
      simp only [UInt64.toNat_add]
      have hmod : (x.toNat + y.toNat) % UInt64.size = x.toNat + y.toNat := by
        apply Nat.mod_eq_of_lt
        unfold U64_MAX at h
        omega
      exact hmod)
  else
    .overflow

/-- Checked subtraction that returns underflow indicator -/
def checkedSub (x y : UInt64) : CheckedSubResult x y :=
  if h : y.toNat ≤ x.toNat then
    -- When y ≤ x, UInt64 subtraction equals natural subtraction
    .ok (x - y) (by
      simp only [UInt64.toNat_sub]
      · have hsub : x.toNat - y.toNat < UInt64.size := by
          have hx : x.toNat < UInt64.size := x.val.isLt
          omega
        rw [Nat.mod_eq_of_lt hsub]
      · exact h)
  else
    .underflow

end OVSM
