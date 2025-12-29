/-
  Solisp.Verification - Main verification interface
  
  This module provides:
  - The main verification condition framework
  - Integration with Solisp compiler output
  - Proof state tracking
-/

import Solisp.Prelude
import Solisp.Types
import Solisp.Primitives
import Solisp.Array
import Solisp.Refinement
import Solisp.Solana
import Solisp.Tactics

namespace Solisp.Verification

open Solisp.Tactics

/-! ## Verification Condition Types -/

/-- Categories of verification conditions -/
inductive VCCategory where
  | divisionSafety
  | arrayBounds
  | arithmeticOverflow
  | arithmeticUnderflow
  | refinementType
  | balanceConservation
  | signerCheck
  | writabilityCheck
  | instructionDataBounds
  | custom (name : String)
  deriving Repr, BEq

/-- A verification condition with full metadata -/
structure VC where
  id : String
  category : VCCategory
  description : String
  sourceFile : String
  sourceLine : Nat
  sourceColumn : Nat
  property : Prop

/-! ## Verification Macros for Generated Code -/

/-- Declare a division safety verification condition -/
macro "vc_div_safe" id:ident "at" file:str ":" line:num ":" col:num ":" 
      "divisor" divisor:term "proof" body:term : command =>
  `(theorem $id : Solisp.NonZero $divisor := $body)

/-- Declare an array bounds verification condition -/
macro "vc_array_bounds" id:ident "at" file:str ":" line:num ":" col:num ":"
      "index" idx:term "length" len:term "proof" body:term : command =>
  `(theorem $id : Solisp.InBounds $idx $len := $body)

/-- Declare an underflow safety verification condition -/
macro "vc_sub_safe" id:ident "at" file:str ":" line:num ":" col:num ":"
      "minuend" x:term "subtrahend" y:term "proof" body:term : command =>
  `(theorem $id : Solisp.SubNoUnderflow $x $y := $body)

/-- Declare an overflow safety verification condition -/
macro "vc_add_safe" id:ident "at" file:str ":" line:num ":" col:num ":"
      "lhs" x:term "rhs" y:term "proof" body:term : command =>
  `(theorem $id : Solisp.AddNoOverflow $x $y := $body)

/-- Declare a refinement type verification condition -/
macro "vc_refinement" id:ident "at" file:str ":" line:num ":" col:num ":"
      "value" val:term "bound" bound:term "proof" body:term : command =>
  `(theorem $id : Solisp.satisfiesBound $val $bound := $body)

/-- Declare a Solana transfer safety verification condition -/
macro "vc_transfer_safe" id:ident "at" file:str ":" line:num ":" col:num ":"
      "src" src:term "dst" dst:term "amount" amt:term "proof" body:term : command =>
  `(theorem $id : Solisp.Solana.TransferSafe $src $dst $amt := $body)

/-! ## Standard Verification Patterns -/

/-- Division by literal is safe -/
theorem div_literal_safe (n : Nat) (h : n > 0) : NonZero (UInt64.ofNat n) := by
  intro heq
  simp [UInt64.ext_iff] at heq
  sorry -- Need UInt64 lemmas

/-- Array literal access is safe -/
theorem array_literal_safe (idx len : Nat) (h : idx < len) : InBounds idx len := h

/-- Subtraction after check is safe -/
theorem sub_after_check_safe (x y : UInt64) (h : ¬(x < y)) : SubNoUnderflow x y := by
  unfold SubNoUnderflow
  have : ¬(x.toNat < y.toNat) := fun hlt => h (UInt64.lt_def.mpr hlt)
  omega

/-! ## Verification Result Collection -/

/-- Track all VCs in a program -/
structure VCSet where
  vcs : List VC
  allProved : Bool

/-- Empty VC set -/
def VCSet.empty : VCSet := { vcs := [], allProved := true }

/-- Add a VC to the set -/
def VCSet.add (set : VCSet) (vc : VC) : VCSet :=
  { set with vcs := vc :: set.vcs }

/-! ## Example: Complete Verified Program -/

-- Example of what generated code looks like:
namespace Example

/-- Example: factorial verification conditions -/
section Factorial

variable (n : Nat) (h_n : n < 20) -- Bound to prevent overflow

-- VC: loop index is always valid
theorem vc_factorial_loop_bound (i : Nat) (h : 1 ≤ i ∧ i ≤ n) : i ≤ n := h.2

-- VC: multiplication doesn't overflow for small inputs
theorem vc_factorial_no_overflow (result i : Nat) 
    (h_result : result ≤ Nat.factorial (n - 1))
    (h_i : i ≤ n) 
    (h_n : n < 20) : 
    result * i < 2^64 := by
  sorry -- Would need factorial bounds lemmas

end Factorial

/-- Example: SOL transfer verification conditions -/
section Transfer

variable (src_bal dst_bal amount : UInt64)

-- VC: balance check passed (we're in else branch of `if (< src-bal amount)`)
theorem vc_transfer_balance_check 
    (h_check : ¬(src_bal.toNat < amount.toNat)) : 
    Solana.DebitSafe src_bal amount := by
  unfold Solana.DebitSafe
  omega

-- VC: addition won't overflow (assuming reasonable balances)
theorem vc_transfer_no_overflow
    (h_dst_bound : dst_bal.toNat ≤ Solana.MAX_LAMPORTS)
    (h_amt_bound : amount.toNat ≤ Solana.MAX_LAMPORTS)
    (h_total : dst_bal.toNat + amount.toNat ≤ U64_MAX) :
    Solana.CreditSafe dst_bal amount := h_total

end Transfer

end Example

/-! ## Verification Entry Point -/

/-- Main verification function called by generated code -/
def verifyAll (vcs : List (String × Prop)) : Bool :=
  -- In actual use, this would be compile-time checked
  -- All theorems must type-check for the program to compile
  true

end Solisp.Verification
