/-
  OVSM.Array - Array bounds verification
  
  This module provides:
  - Array bounds checking lemmas
  - Safe array access functions requiring bounds proofs
  - Index type for statically-bounded access
-/

import OVSM.Prelude
import OVSM.Primitives

namespace OVSM

/-! ## Array Bounds Predicates -/

/-- Proof that an index is within array bounds -/
def InBounds (idx : Nat) (len : Nat) : Prop := idx < len

/-- Proof that an index is within bounds for UInt64 index -/
def InBoundsU64 (idx : UInt64) (len : Nat) : Prop := idx.toNat < len

/-- A bounded index type (like Fin but for OVSM) -/
structure BoundedIndex (len : Nat) where
  val : Nat
  isLt : val < len
  deriving Repr

/-- Convert Fin to BoundedIndex -/
def BoundedIndex.ofFin {n : Nat} (i : Fin n) : BoundedIndex n :=
  { val := i.val, isLt := i.isLt }

/-- Convert BoundedIndex to Fin -/
def BoundedIndex.toFin {n : Nat} (i : BoundedIndex n) : Fin n :=
  ⟨i.val, i.isLt⟩

/-! ## Safe Array Access -/

/-- Safe array access requiring bounds proof -/
def safeGet {α : Type} (arr : Array α) (idx : Nat) (h : InBounds idx arr.size) : α :=
  arr[idx]'h

/-- Safe array access with UInt64 index -/
def safeGetU64 {α : Type} (arr : Array α) (idx : UInt64) (h : InBoundsU64 idx arr.size) : α :=
  arr[idx.toNat]'h

/-- Safe array set requiring bounds proof -/
def safeSet {α : Type} (arr : Array α) (idx : Nat) (val : α) (h : InBounds idx arr.size) : Array α :=
  arr.set ⟨idx, h⟩ val

/-! ## Bounds Lemmas -/

/-- Zero is always in bounds for non-empty arrays -/
theorem zero_in_bounds (len : Nat) (h : len > 0) : InBounds 0 len := h

/-- Last valid index is len - 1 -/
theorem last_in_bounds (len : Nat) (h : len > 0) : InBounds (len - 1) len := by
  unfold InBounds
  omega

/-- If idx < len and idx' < idx, then idx' < len -/
theorem in_bounds_trans (idx idx' len : Nat) (h1 : InBounds idx len) (h2 : idx' < idx) : 
    InBounds idx' len := by
  unfold InBounds at *
  omega

/-- Index from range [0, n) is in bounds for array of size ≥ n -/
theorem range_in_bounds (idx n len : Nat) (h1 : idx < n) (h2 : n ≤ len) : InBounds idx len := by
  unfold InBounds
  omega

/-- If we checked idx < len, then InBounds holds -/
theorem in_bounds_from_lt (idx len : Nat) (h : idx < len) : InBounds idx len := h

/-- If we checked idx < len with UInt64, then InBoundsU64 holds -/
theorem in_bounds_from_lt_u64 (idx : UInt64) (len : Nat) (h : idx.toNat < len) : 
    InBoundsU64 idx len := h

/-! ## Array Slicing Safety -/

/-- Proof that a slice range is valid -/
def ValidSlice (start stop len : Nat) : Prop :=
  start ≤ stop ∧ stop ≤ len

/-- Safe array slice -/
def safeSlice {α : Type} (arr : Array α) (start stop : Nat) (h : ValidSlice start stop arr.size) : 
    Array α :=
  arr.extract start stop

/-- Slice from 0 to n is valid if n ≤ len -/
theorem prefix_slice_valid (n len : Nat) (h : n ≤ len) : ValidSlice 0 n len := by
  unfold ValidSlice
  constructor
  · omega
  · exact h

/-- Slice from n to len is valid if n ≤ len -/
theorem suffix_slice_valid (n len : Nat) (h : n ≤ len) : ValidSlice n len len := by
  unfold ValidSlice
  constructor
  · exact h
  · le_refl

/-! ## Loop Iteration Bounds -/

/-- Proof that a loop variable is in bounds during iteration -/
theorem loop_var_in_bounds (i start stop : Nat) (h1 : start ≤ i) (h2 : i < stop) (h3 : stop ≤ len) :
    InBounds i len := by
  unfold InBounds
  omega

/-- For loop index in [0, n) is valid for array of size n -/
theorem for_range_in_bounds (i n : Nat) (h : i < n) : InBounds i n := h

/-! ## Concatenation Bounds -/

/-- Size of concatenated arrays -/
theorem concat_size {α : Type} (a b : Array α) : (a ++ b).size = a.size + b.size := 
  Array.size_append a b

/-- Index in first array is in bounds after concat -/
theorem concat_left_in_bounds {α : Type} (a b : Array α) (idx : Nat) (h : InBounds idx a.size) :
    InBounds idx (a ++ b).size := by
  unfold InBounds at *
  simp [Array.size_append]
  omega

/-- Index in second array (offset) is in bounds after concat -/
theorem concat_right_in_bounds {α : Type} (a b : Array α) (idx : Nat) (h : InBounds idx b.size) :
    InBounds (a.size + idx) (a ++ b).size := by
  unfold InBounds at *
  simp [Array.size_append]
  omega

end OVSM
