//! Primitive types used by the specialized codegen: `RegisterType` (the
//! subset of Seq types that fit in an LLVM register) and `SpecSignature`
//! (the register-passing signature of a specialized word).

use crate::types::Type;

/// Register types that can be passed directly in LLVM registers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterType {
    /// 64-bit signed integer (maps to LLVM i64)
    I64,
    /// 64-bit floating point (maps to LLVM double)
    Double,
}

impl RegisterType {
    /// Convert a Seq Type to a RegisterType, if possible
    pub fn from_type(ty: &Type) -> Option<Self> {
        match ty {
            Type::Int | Type::Bool => Some(RegisterType::I64),
            Type::Float => Some(RegisterType::Double),
            _ => None,
        }
    }

    /// Get the LLVM type name for this register type
    pub fn llvm_type(&self) -> &'static str {
        match self {
            RegisterType::I64 => "i64",
            RegisterType::Double => "double",
        }
    }
}

/// Signature for a specialized function
#[derive(Debug, Clone)]
pub struct SpecSignature {
    /// Input types (bottom to top of stack)
    pub inputs: Vec<RegisterType>,
    /// Output types (bottom to top of stack)
    pub outputs: Vec<RegisterType>,
}

impl SpecSignature {
    /// Generate the specialized function suffix based on types
    /// For now: single Int -> "_i64", single Float -> "_f64"
    /// Multiple values will need struct returns in Phase 4
    pub fn suffix(&self) -> String {
        if self.inputs.len() == 1 && self.outputs.len() == 1 {
            match (self.inputs[0], self.outputs[0]) {
                (RegisterType::I64, RegisterType::I64) => "_i64".to_string(),
                (RegisterType::Double, RegisterType::Double) => "_f64".to_string(),
                (RegisterType::I64, RegisterType::Double) => "_i64_to_f64".to_string(),
                (RegisterType::Double, RegisterType::I64) => "_f64_to_i64".to_string(),
            }
        } else {
            let mut suffix = String::new();
            for ty in &self.inputs {
                suffix.push('_');
                suffix.push_str(match ty {
                    RegisterType::I64 => "i",
                    RegisterType::Double => "f",
                });
            }
            suffix.push_str("_to");
            for ty in &self.outputs {
                suffix.push('_');
                suffix.push_str(match ty {
                    RegisterType::I64 => "i",
                    RegisterType::Double => "f",
                });
            }
            suffix
        }
    }

    /// Check if this signature supports direct call (single output)
    pub fn is_direct_call(&self) -> bool {
        self.outputs.len() == 1
    }

    /// Get the LLVM return type for this signature
    ///
    /// Single output: `i64` or `double`
    /// Multiple outputs: `{ i64, i64 }` struct
    pub fn llvm_return_type(&self) -> String {
        if self.outputs.len() == 1 {
            self.outputs[0].llvm_type().to_string()
        } else {
            let types: Vec<_> = self.outputs.iter().map(|t| t.llvm_type()).collect();
            format!("{{ {} }}", types.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_type_from_type() {
        assert_eq!(RegisterType::from_type(&Type::Int), Some(RegisterType::I64));
        assert_eq!(
            RegisterType::from_type(&Type::Bool),
            Some(RegisterType::I64)
        );
        assert_eq!(
            RegisterType::from_type(&Type::Float),
            Some(RegisterType::Double)
        );
        assert_eq!(RegisterType::from_type(&Type::String), None);
    }

    #[test]
    fn test_spec_signature_suffix() {
        let sig = SpecSignature {
            inputs: vec![RegisterType::I64],
            outputs: vec![RegisterType::I64],
        };
        assert_eq!(sig.suffix(), "_i64");

        let sig2 = SpecSignature {
            inputs: vec![RegisterType::Double],
            outputs: vec![RegisterType::Double],
        };
        assert_eq!(sig2.suffix(), "_f64");
    }
}
