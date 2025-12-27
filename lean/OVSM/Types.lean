/-
  OVSM.Types - Type system definitions matching OVSM's type system
  
  This module defines Lean 4 representations of all OVSM types,
  enabling formal reasoning about OVSM programs.
-/

import OVSM.Prelude

namespace OVSM

/-! ## Primitive Types -/

/-- OVSM primitive types -/
inductive PrimType where
  | u8 | u16 | u32 | u64
  | i8 | i16 | i32 | i64
  | f32 | f64
  | bool
  | unit
  deriving Repr, BEq, Inhabited

/-- Size in bytes of a primitive type -/
def PrimType.sizeBytes : PrimType → Nat
  | .u8 | .i8 => 1
  | .u16 | .i16 => 2
  | .u32 | .i32 | .f32 => 4
  | .u64 | .i64 | .f64 => 8
  | .bool => 1
  | .unit => 0

/-- Maximum value for unsigned integer types -/
def PrimType.maxUnsigned : PrimType → Nat
  | .u8 => 2^8 - 1
  | .u16 => 2^16 - 1
  | .u32 => 2^32 - 1
  | .u64 => 2^64 - 1
  | _ => 0

/-! ## Compound Types -/

/-- Solana public key (32 bytes) -/
structure Pubkey where
  bytes : ByteArray
  size_eq : bytes.size = 32
  deriving Repr

/-- Create a zero pubkey -/
def Pubkey.zero : Pubkey := {
  bytes := ByteArray.mk (Array.mkArray 32 0)
  size_eq := by native_decide
}

/-! ## OVSM Type Representation -/

/-- Full OVSM type representation -/
inductive OVSMType where
  | prim (t : PrimType)
  | array (elem : OVSMType) (size : Nat)
  | dynArray (elem : OVSMType)
  | tuple (types : List OVSMType)
  | struct (name : String) (fields : List (String × OVSMType))
  | pubkey
  | string
  | ptr (pointee : OVSMType)
  | ref (referent : OVSMType)
  | refMut (referent : OVSMType)
  | fn (params : List OVSMType) (ret : OVSMType)
  | any
  | never
  | unknown
  | refined (base : OVSMType) (var : String) (pred : String)
  deriving Repr, BEq, Inhabited

/-! ## Type Predicates -/

/-- Check if type is numeric -/
def OVSMType.isNumeric : OVSMType → Bool
  | .prim .u8 | .prim .u16 | .prim .u32 | .prim .u64 => true
  | .prim .i8 | .prim .i16 | .prim .i32 | .prim .i64 => true
  | .prim .f32 | .prim .f64 => true
  | _ => false

/-- Check if type is an integer type -/
def OVSMType.isInteger : OVSMType → Bool
  | .prim .u8 | .prim .u16 | .prim .u32 | .prim .u64 => true
  | .prim .i8 | .prim .i16 | .prim .i32 | .prim .i64 => true
  | _ => false

/-- Check if type is unsigned -/
def OVSMType.isUnsigned : OVSMType → Bool
  | .prim .u8 | .prim .u16 | .prim .u32 | .prim .u64 => true
  | _ => false

/-- Check if type is signed -/  
def OVSMType.isSigned : OVSMType → Bool
  | .prim .i8 | .prim .i16 | .prim .i32 | .prim .i64 => true
  | _ => false

/-- Get the base type of a refined type -/
def OVSMType.baseType : OVSMType → OVSMType
  | .refined base _ _ => base.baseType
  | t => t

/-! ## Type Size Calculation -/

/-- Calculate size of a type in bytes (for fixed-size types) -/
partial def OVSMType.sizeBytes : OVSMType → Option Nat
  | .prim p => some p.sizeBytes
  | .array elem size => do
    let elemSize ← elem.sizeBytes
    some (elemSize * size)
  | .tuple types => do
    let sizes ← types.mapM OVSMType.sizeBytes
    some (sizes.foldl (· + ·) 0)
  | .pubkey => some 32
  | .prim .bool => some 1
  | .prim .unit => some 0
  | .ptr _ => some 8  -- 64-bit pointers
  | .ref _ => some 8
  | .refMut _ => some 8
  | .refined base _ _ => base.sizeBytes
  | _ => none  -- Dynamic types have no fixed size

/-! ## Common Type Constructors -/

/-- u64 type -/
def u64Type : OVSMType := .prim .u64

/-- i64 type -/
def i64Type : OVSMType := .prim .i64

/-- bool type -/
def boolType : OVSMType := .prim .bool

/-- Array of u8 with given size -/
def byteArrayType (size : Nat) : OVSMType := .array (.prim .u8) size

end OVSM
