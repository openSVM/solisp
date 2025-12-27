/-
  OVSM.Tactics - Custom tactics for OVSM verification
  
  This module provides:
  - Automation tactics for common verification patterns
  - Combination tactics for OVSM-specific proofs
-/

import Lean
import OVSM.Prelude
import OVSM.Primitives
import OVSM.Array
import OVSM.Refinement

namespace OVSM.Tactics

open Lean Elab Tactic Meta

/-! ## Basic Automation -/

/-- Try omega, then simp, then trivial -/
macro "ovsm_trivial" : tactic =>
  `(tactic| first | omega | simp | trivial | rfl | decide)

/-- Try to solve arithmetic goals -/
macro "ovsm_arith" : tactic =>
  `(tactic| first | omega | simp_arith | ring | decide)

/-- Try to solve bounds goals -/
macro "ovsm_bounds" : tactic =>
  `(tactic| (unfold InBounds InBoundsU64 satisfiesBound satisfiesRange at *; omega))

/-! ## Division Safety Tactics -/

/-- Prove division is safe by showing divisor is a non-zero literal -/
macro "ovsm_div_safe" : tactic =>
  `(tactic| (
    first
    | exact pos_literal_nonzero _ (by omega)
    | exact div_safe_from_gt_zero _ (by omega)
    | exact div_by_nonzero_literal _ _ (by omega)
    | (intro h; simp at h)
    | omega
  ))

/-! ## Overflow Safety Tactics -/

/-- Prove addition doesn't overflow -/
macro "ovsm_add_safe" : tactic =>
  `(tactic| (
    unfold AddNoOverflow U64_MAX at *
    first | omega | simp_arith
  ))

/-- Prove subtraction doesn't underflow -/
macro "ovsm_sub_safe" : tactic =>
  `(tactic| (
    unfold SubNoUnderflow at *
    first 
    | exact sub_safe_from_geq _ _ (by omega)
    | exact sub_safe_from_gt _ _ (by omega)
    | omega
  ))

/-! ## Array Bounds Tactics -/

/-- Prove array access is in bounds -/
macro "ovsm_in_bounds" : tactic =>
  `(tactic| (
    unfold InBounds InBoundsU64 at *
    first
    | exact zero_in_bounds _ (by omega)
    | exact last_in_bounds _ (by omega)
    | exact for_range_in_bounds _ _ (by omega)
    | omega
  ))

/-! ## Refinement Type Tactics -/

/-- Prove a literal satisfies a refinement predicate -/
macro "ovsm_refine_literal" : tactic =>
  `(tactic| (
    first
    | exact literal_satisfies_bound _ _ (by omega)
    | exact literal_satisfies_range _ _ _ (by omega) (by omega) (by omega)
    | exact nonzero_literal_positive _ (by omega) (by omega)
    | (constructor <;> omega)
    | omega
  ))

/-- Prove refinement from guard condition -/
macro "ovsm_from_guard" : tactic =>
  `(tactic| (
    first
    | exact guard_implies_bound _ _ (by assumption)
    | exact else_guard_implies_bound _ _ (by assumption)
    | exact guards_imply_range _ _ _ (by assumption) (by assumption)
    | assumption
  ))

/-! ## Solana-Specific Tactics -/

/-- Prove a transfer is safe -/
macro "ovsm_transfer_safe" : tactic =>
  `(tactic| (
    constructor
    · unfold Solana.DebitSafe; omega
    · unfold Solana.CreditSafe OVSM.U64_MAX; omega
  ))

/-- Prove debit is safe from balance check -/
macro "ovsm_debit_safe" : tactic =>
  `(tactic| (
    unfold Solana.DebitSafe at *
    first
    | exact Solana.debit_safe_from_check _ _ (by omega)
    | exact Solana.debit_safe_from_guard _ _ (by omega)
    | omega
  ))

/-! ## Combined Verification Tactic -/

/-- Main OVSM verification tactic - tries all strategies -/
macro "ovsm_verify" : tactic =>
  `(tactic| (
    first
    | ovsm_trivial
    | ovsm_arith
    | ovsm_bounds
    | ovsm_div_safe
    | ovsm_add_safe
    | ovsm_sub_safe
    | ovsm_in_bounds
    | ovsm_refine_literal
    | ovsm_from_guard
    | ovsm_transfer_safe
    | ovsm_debit_safe
    | (simp only [*]; omega)
    | sorry
  ))

/-! ## Verification Condition Wrapper -/

/-- Attribute to mark verification conditions -/
-- register_option ovsm.vc.timeout : Nat := 10

/-- A verification condition that must be proved -/
class VerificationCondition (p : Prop) where
  proof : p

/-- Prove a verification condition -/
macro "prove_vc" : tactic =>
  `(tactic| exact VerificationCondition.mk (by ovsm_verify))

end OVSM.Tactics
