/-
  OVSM.Prelude - Common imports and basic definitions
  
  This module provides the foundational types and utilities used throughout
  the OVSM verification library.
-/

-- Standard library imports
import Lean

namespace OVSM

/-! ## Basic Type Aliases -/

/-- 64-bit unsigned integer (primary numeric type in OVSM/Solana) -/
abbrev U64 := UInt64

/-- 64-bit signed integer -/
abbrev I64 := Int64

/-- 32-bit unsigned integer -/
abbrev U32 := UInt32

/-- 32-bit signed integer -/
abbrev I32 := Int32

/-- 16-bit unsigned integer -/
abbrev U16 := UInt16

/-- 8-bit unsigned integer -/
abbrev U8 := UInt8

/-! ## Constants -/

/-- Maximum value for U64 -/
def U64_MAX : Nat := 2^64 - 1

/-- Maximum value for I64 (positive) -/
def I64_MAX : Int := 2^63 - 1

/-- Minimum value for I64 (negative) -/
def I64_MIN : Int := -(2^63)

/-! ## Utility Functions -/

/-- Convert UInt64 to Nat for arithmetic reasoning -/
@[inline] def U64.IsValid (n : Nat) : Prop := n ≤ U64_MAX

/-- Proof that a natural number fits in U64 -/
structure U64Bounded (n : Nat) : Prop where
  isValid : n ≤ U64_MAX

/-! ## Source Location Tracking -/

/-- Source location in OVSM program -/
structure SourceLoc where
  file : String
  line : Nat
  column : Nat
  deriving Repr, BEq

/-- Create a source location -/
def mkLoc (file : String) (line col : Nat) : SourceLoc :=
  { file := file, line := line, column := col }

/-! ## Verification Result Types -/

/-- Result of a verification condition check -/
inductive VCResult where
  | proved : VCResult
  | failed (reason : String) : VCResult
  | unknown (reason : String) : VCResult
  deriving Repr

/-- A verification condition with metadata -/
structure VerificationCondition where
  name : String
  description : String
  location : Option SourceLoc
  property : Prop
  deriving Repr

end OVSM
