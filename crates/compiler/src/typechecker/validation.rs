//! Type-name parsing plus validation of effects, stacks, and union field types.

use crate::ast::Program;
use crate::types::{Effect, StackType, Type};

use super::TypeChecker;

impl TypeChecker {
    pub(super) fn parse_type_name(&self, name: &str) -> Type {
        match name {
            "Int" => Type::Int,
            "Float" => Type::Float,
            "Bool" => Type::Bool,
            "String" => Type::String,
            "Channel" => Type::Channel,
            // Any other name is assumed to be a union type reference
            other => Type::Union(other.to_string()),
        }
    }

    /// Check if a type name is a known valid type
    ///
    /// Returns true for built-in types (Int, Float, Bool, String, Channel) and
    /// registered union type names
    pub(super) fn is_valid_type_name(&self, name: &str) -> bool {
        matches!(name, "Int" | "Float" | "Bool" | "String" | "Channel")
            || self.unions.contains_key(name)
    }

    /// Validate that all field types in union definitions reference known types
    ///
    /// Note: Field count validation happens earlier in generate_constructors()
    pub(super) fn validate_union_field_types(&self, program: &Program) -> Result<(), String> {
        for union_def in &program.unions {
            for variant in &union_def.variants {
                for field in &variant.fields {
                    if !self.is_valid_type_name(&field.type_name) {
                        return Err(format!(
                            "Unknown type '{}' in field '{}' of variant '{}' in union '{}'. \
                             Valid types are: Int, Float, Bool, String, Channel, or a defined union name.",
                            field.type_name, field.name, variant.name, union_def.name
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate that all types in a stack effect are known types
    ///
    /// RFC #345: This catches cases where an uppercase identifier was parsed as a type
    /// variable but should have been a union type (e.g., from an include that wasn't
    /// available when parsing). Type variables should be single uppercase letters (T, U, V).
    pub(super) fn validate_effect_types(
        &self,
        effect: &Effect,
        word_name: &str,
    ) -> Result<(), String> {
        self.validate_stack_types(&effect.inputs, word_name)?;
        self.validate_stack_types(&effect.outputs, word_name)?;
        Ok(())
    }

    /// Validate types in a stack type
    pub(super) fn validate_stack_types(
        &self,
        stack: &StackType,
        word_name: &str,
    ) -> Result<(), String> {
        match stack {
            StackType::Empty | StackType::RowVar(_) => Ok(()),
            StackType::Cons { rest, top } => {
                self.validate_type(top, word_name)?;
                self.validate_stack_types(rest, word_name)
            }
        }
    }

    /// Validate a single type
    ///
    /// Type variables are allowed - they're used for polymorphism.
    /// Only Type::Union types are validated to ensure they're registered.
    pub(super) fn validate_type(&self, ty: &Type, word_name: &str) -> Result<(), String> {
        match ty {
            Type::Var(_) => {
                // Type variables are always valid - they represent polymorphic types
                // Examples: T, U, V, Ctx, Handle, Acc, etc.
                // After fixup_union_types(), any union name that was mistakenly parsed
                // as a type variable will have been converted to Type::Union
                Ok(())
            }
            Type::Quotation(effect) => self.validate_effect_types(effect, word_name),
            Type::Closure { effect, captures } => {
                self.validate_effect_types(effect, word_name)?;
                for cap in captures {
                    self.validate_type(cap, word_name)?;
                }
                Ok(())
            }
            // Concrete types are always valid
            Type::Int
            | Type::Float
            | Type::Bool
            | Type::String
            | Type::Symbol
            | Type::Channel
            | Type::Variant => Ok(()),
            // Union types are valid if they're registered
            Type::Union(name) => {
                if !self.unions.contains_key(name) {
                    return Err(format!(
                        "In word '{}': Unknown union type '{}' in stack effect.\n\
                         Make sure this union is defined before the word that uses it.",
                        word_name, name
                    ));
                }
                Ok(())
            }
        }
    }
}
