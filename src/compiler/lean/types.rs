//! # OVSM to Lean 4 Type Mapping
//!
//! This module provides conversion between OVSM types and Lean 4 types.

use crate::types::{RefinementType, Type};

/// Lean 4 type representation
#[derive(Debug, Clone, PartialEq)]
pub enum LeanType {
    /// UInt8
    UInt8,
    /// UInt16
    UInt16,
    /// UInt32
    UInt32,
    /// UInt64
    UInt64,
    /// Int8
    Int8,
    /// Int16
    Int16,
    /// Int32
    Int32,
    /// Int64
    Int64,
    /// Float (32-bit)
    Float32,
    /// Float (64-bit)
    Float64,
    /// Bool
    Bool,
    /// Unit
    Unit,
    /// String
    String,
    /// Nat (for indices and sizes)
    Nat,
    /// Int (arbitrary precision)
    Int,
    /// Array with element type and optional size
    Array {
        /// Element type of the array
        element: Box<LeanType>,
        /// Optional fixed size
        size: Option<usize>,
    },
    /// Tuple of types
    Tuple(Vec<LeanType>),
    /// Function type
    Function {
        /// Parameter types
        params: Vec<LeanType>,
        /// Return type
        ret: Box<LeanType>,
    },
    /// Subtype (refinement type)
    Subtype {
        /// Base type being refined
        base: Box<LeanType>,
        /// Variable name in the predicate
        var: String,
        /// Predicate expression in Lean syntax
        predicate: String,
    },
    /// Fin n (bounded natural number)
    Fin(usize),
    /// ByteArray with optional size
    ByteArray(Option<usize>),
    /// Custom type name
    Custom(String),
    /// Any type (for gradual typing)
    Any,
}

impl LeanType {
    /// Convert to Lean 4 syntax string
    pub fn to_lean(&self) -> String {
        match self {
            LeanType::UInt8 => "UInt8".to_string(),
            LeanType::UInt16 => "UInt16".to_string(),
            LeanType::UInt32 => "UInt32".to_string(),
            LeanType::UInt64 => "UInt64".to_string(),
            LeanType::Int8 => "Int8".to_string(),
            LeanType::Int16 => "Int16".to_string(),
            LeanType::Int32 => "Int32".to_string(),
            LeanType::Int64 => "Int64".to_string(),
            LeanType::Float32 => "Float".to_string(),
            LeanType::Float64 => "Float".to_string(),
            LeanType::Bool => "Bool".to_string(),
            LeanType::Unit => "Unit".to_string(),
            LeanType::String => "String".to_string(),
            LeanType::Nat => "Nat".to_string(),
            LeanType::Int => "Int".to_string(),
            LeanType::Array { element, size } => {
                if let Some(n) = size {
                    format!("Array {} {}", element.to_lean(), n)
                } else {
                    format!("Array {}", element.to_lean())
                }
            }
            LeanType::Tuple(types) => {
                let type_strs: Vec<_> = types.iter().map(|t| t.to_lean()).collect();
                type_strs.join(" × ")
            }
            LeanType::Function { params, ret } => {
                let param_strs: Vec<_> = params.iter().map(|t| t.to_lean()).collect();
                format!("{} → {}", param_strs.join(" → "), ret.to_lean())
            }
            LeanType::Subtype {
                base,
                var,
                predicate,
            } => {
                format!("{{ {} : {} // {} }}", var, base.to_lean(), predicate)
            }
            LeanType::Fin(n) => format!("Fin {}", n),
            LeanType::ByteArray(size) => {
                if let Some(n) = size {
                    format!("ByteArray {}", n)
                } else {
                    "ByteArray".to_string()
                }
            }
            LeanType::Custom(name) => name.clone(),
            LeanType::Any => "OVSM.Any".to_string(),
        }
    }
}

/// Type mapper for converting OVSM types to Lean types
pub struct TypeMapper;

impl TypeMapper {
    /// Convert an OVSM Type to a Lean type
    pub fn ovsm_to_lean(ty: &Type) -> LeanType {
        match ty {
            Type::U8 => LeanType::UInt8,
            Type::U16 => LeanType::UInt16,
            Type::U32 => LeanType::UInt32,
            Type::U64 => LeanType::UInt64,
            Type::I8 => LeanType::Int8,
            Type::I16 => LeanType::Int16,
            Type::I32 => LeanType::Int32,
            Type::I64 => LeanType::Int64,
            Type::F32 => LeanType::Float32,
            Type::F64 => LeanType::Float64,
            Type::Bool => LeanType::Bool,
            Type::Unit => LeanType::Unit,
            Type::String => LeanType::String,
            Type::Pubkey => LeanType::ByteArray(Some(32)),
            Type::Array { element, size } => LeanType::Array {
                element: Box::new(Self::ovsm_to_lean(element)),
                size: Some(*size),
            },
            Type::Tuple(types) => LeanType::Tuple(types.iter().map(Self::ovsm_to_lean).collect()),
            Type::Fn { params, ret } => LeanType::Function {
                params: params.iter().map(Self::ovsm_to_lean).collect(),
                ret: Box::new(Self::ovsm_to_lean(ret)),
            },
            Type::Struct(name) => LeanType::Custom(format!("OVSM.{}", name)),
            Type::Ptr(inner) => Self::ovsm_to_lean(inner), // Pointers become their pointee type
            Type::Ref(inner) => Self::ovsm_to_lean(inner),
            Type::RefMut(inner) => Self::ovsm_to_lean(inner),
            Type::Refined(refined) => Self::refinement_to_lean(refined),
            Type::Any => LeanType::Any,
            Type::Never => LeanType::Custom("Empty".to_string()),
            Type::Unknown | Type::Var(_) => LeanType::Any,
        }
    }

    /// Convert an OVSM refinement type to a Lean subtype
    pub fn refinement_to_lean(refined: &RefinementType) -> LeanType {
        let base = Self::ovsm_to_lean(&refined.base);
        let predicate = Self::predicate_to_lean(&refined.predicate, &refined.var);

        LeanType::Subtype {
            base: Box::new(base),
            var: refined.var.clone(),
            predicate,
        }
    }

    /// Convert an OVSM predicate to Lean syntax
    pub fn predicate_to_lean(predicate: &crate::types::refinement::Predicate, var: &str) -> String {
        use crate::types::refinement::{CompareOp, Predicate, PredicateExpr};

        match predicate {
            Predicate::True => "True".to_string(),
            Predicate::False => "False".to_string(),
            Predicate::Compare { op, left, right } => {
                let l = Self::pred_expr_to_lean(left, var);
                let r = Self::pred_expr_to_lean(right, var);
                let op_str = match op {
                    CompareOp::Lt => "<",
                    CompareOp::LtEq => "≤",
                    CompareOp::Gt => ">",
                    CompareOp::GtEq => "≥",
                    CompareOp::Eq => "=",
                    CompareOp::NotEq => "≠",
                };
                format!("{} {} {}", l, op_str, r)
            }
            Predicate::And(p, q) => {
                let lp = Self::predicate_to_lean(p, var);
                let rp = Self::predicate_to_lean(q, var);
                format!("({} ∧ {})", lp, rp)
            }
            Predicate::Or(p, q) => {
                let lp = Self::predicate_to_lean(p, var);
                let rp = Self::predicate_to_lean(q, var);
                format!("({} ∨ {})", lp, rp)
            }
            Predicate::Not(p) => {
                let inner = Self::predicate_to_lean(p, var);
                format!("¬{}", inner)
            }
            Predicate::Implies(p, q) => {
                let lp = Self::predicate_to_lean(p, var);
                let rp = Self::predicate_to_lean(q, var);
                format!("({} → {})", lp, rp)
            }
            Predicate::Opaque(id) => format!("opaque_{}", id),
        }
    }

    /// Convert a predicate expression to Lean syntax
    fn pred_expr_to_lean(expr: &crate::types::refinement::PredicateExpr, var: &str) -> String {
        use crate::types::refinement::PredicateExpr;

        match expr {
            PredicateExpr::Var => format!("{}.toNat", var),
            PredicateExpr::Const(n) => n.to_string(),
            PredicateExpr::Add(l, r) => {
                let ll = Self::pred_expr_to_lean(l, var);
                let rr = Self::pred_expr_to_lean(r, var);
                format!("({} + {})", ll, rr)
            }
            PredicateExpr::Sub(l, r) => {
                let ll = Self::pred_expr_to_lean(l, var);
                let rr = Self::pred_expr_to_lean(r, var);
                format!("({} - {})", ll, rr)
            }
            PredicateExpr::Mul(l, r) => {
                let ll = Self::pred_expr_to_lean(l, var);
                let rr = Self::pred_expr_to_lean(r, var);
                format!("({} * {})", ll, rr)
            }
            PredicateExpr::Len(arr_name) => format!("{}.size", arr_name),
            PredicateExpr::Field(obj, field) => format!("{}.{}", obj, field),
        }
    }

    /// Get the Lean type for array indices
    pub fn index_type_for_array(size: usize) -> LeanType {
        LeanType::Fin(size)
    }

    /// Get the Lean type for a bounded value
    pub fn bounded_u64(bound: i64) -> LeanType {
        LeanType::Subtype {
            base: Box::new(LeanType::UInt64),
            var: "x".to_string(),
            predicate: format!("x.toNat < {}", bound),
        }
    }

    /// Get the Lean type for a ranged value
    pub fn ranged_u64(lo: i64, hi: i64) -> LeanType {
        LeanType::Subtype {
            base: Box::new(LeanType::UInt64),
            var: "x".to_string(),
            predicate: format!("({} ≤ x.toNat ∧ x.toNat < {})", lo, hi),
        }
    }

    /// Get the Lean type for non-zero values
    pub fn non_zero_u64() -> LeanType {
        LeanType::Subtype {
            base: Box::new(LeanType::UInt64),
            var: "x".to_string(),
            predicate: "x ≠ 0".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_to_lean() {
        assert_eq!(TypeMapper::ovsm_to_lean(&Type::U64).to_lean(), "UInt64");
        assert_eq!(TypeMapper::ovsm_to_lean(&Type::I64).to_lean(), "Int64");
        assert_eq!(TypeMapper::ovsm_to_lean(&Type::Bool).to_lean(), "Bool");
    }

    #[test]
    fn test_array_to_lean() {
        let arr_ty = Type::Array {
            element: Box::new(Type::U8),
            size: 32,
        };
        assert_eq!(
            TypeMapper::ovsm_to_lean(&arr_ty).to_lean(),
            "Array UInt8 32"
        );
    }

    #[test]
    fn test_function_to_lean() {
        let fn_ty = Type::Fn {
            params: vec![Type::I64, Type::I64],
            ret: Box::new(Type::I64),
        };
        assert_eq!(
            TypeMapper::ovsm_to_lean(&fn_ty).to_lean(),
            "Int64 → Int64 → Int64"
        );
    }

    #[test]
    fn test_bounded_u64() {
        let bounded = TypeMapper::bounded_u64(10);
        assert_eq!(bounded.to_lean(), "{ x : UInt64 // x.toNat < 10 }");
    }

    #[test]
    fn test_non_zero() {
        let nz = TypeMapper::non_zero_u64();
        assert_eq!(nz.to_lean(), "{ x : UInt64 // x ≠ 0 }");
    }
}
