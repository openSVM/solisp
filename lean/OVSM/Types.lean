/-
  Solisp.Types - Type system definitions matching Solisp's type system
  
  This module defines Lean 4 representations of all Solisp types,
  enabling formal reasoning about Solisp programs.
-/

import Solisp.Prelude

namespace Solisp

/-! ## Primitive Types -/

/-- Solisp primitive types -/
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

/-! ## Solisp Type Representation -/

/-- Full Solisp type representation -/
inductive SolispType where
  | prim (t : PrimType)
  | array (elem : SolispType) (size : Nat)
  | dynArray (elem : SolispType)
  | tuple (types : List SolispType)
  | struct (name : String) (fields : List (String × SolispType))
  | pubkey
  | string
  | ptr (pointee : SolispType)
  | ref (referent : SolispType)
  | refMut (referent : SolispType)
  | fn (params : List SolispType) (ret : SolispType)
  | any
  | never
  | unknown
  | refined (base : SolispType) (var : String) (pred : String)
  deriving Repr, BEq, Inhabited

/-! ## Type Predicates -/

/-- Check if type is numeric -/
def SolispType.isNumeric : SolispType → Bool
  | .prim .u8 | .prim .u16 | .prim .u32 | .prim .u64 => true
  | .prim .i8 | .prim .i16 | .prim .i32 | .prim .i64 => true
  | .prim .f32 | .prim .f64 => true
  | _ => false

/-- Check if type is an integer type -/
def SolispType.isInteger : SolispType → Bool
  | .prim .u8 | .prim .u16 | .prim .u32 | .prim .u64 => true
  | .prim .i8 | .prim .i16 | .prim .i32 | .prim .i64 => true
  | _ => false

/-- Check if type is unsigned -/
def SolispType.isUnsigned : SolispType → Bool
  | .prim .u8 | .prim .u16 | .prim .u32 | .prim .u64 => true
  | _ => false

/-- Check if type is signed -/  
def SolispType.isSigned : SolispType → Bool
  | .prim .i8 | .prim .i16 | .prim .i32 | .prim .i64 => true
  | _ => false

/-- Get the base type of a refined type -/
def SolispType.baseType : SolispType → SolispType
  | .refined base _ _ => base.baseType
  | t => t

/-! ## Type Size Calculation -/

/-- Calculate size of a type in bytes (for fixed-size types) -/
partial def SolispType.sizeBytes : SolispType → Option Nat
  | .prim p => some p.sizeBytes
  | .array elem size => do
    let elemSize ← elem.sizeBytes
    some (elemSize * size)
  | .tuple types => do
    let sizes ← types.mapM SolispType.sizeBytes
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
def u64Type : SolispType := .prim .u64

/-- i64 type -/
def i64Type : SolispType := .prim .i64

/-- bool type -/
def boolType : SolispType := .prim .bool

/-- Array of u8 with given size -/
def byteArrayType (size : Nat) : SolispType := .array (.prim .u8) size

end Solisp
