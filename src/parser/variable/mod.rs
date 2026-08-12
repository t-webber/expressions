//! Implementation of variables, i.e. identifiers.
//!
//! Note that labels (as in `goto: label`) are considered variables before being
//! pushed to the control flow.
//!
//! Else, variables can either be declarations (if attributes are applied to the
//! variable) or names (else). In the RHS, variables must be names.

#![expect(clippy::inline_modules, reason = "clearer api")]
pub mod api {
    //! Api module to choose what functions to export.

    #![allow(clippy::pub_use, reason = "expose simple API")]

    pub use super::Variable;
    pub use super::attr_var::AttributeVariable;
    pub use super::declaration::{Declaration, DeclarationValue};
    pub use super::name::VariableName;
    pub use super::traits::{PureType, VariableConversion};
    pub use super::value::VariableValue;
}

mod apply_attr_var;
mod apply_var;
mod attr_var;
mod declaration;
mod name;
mod traits;
mod value;

use core::fmt;
use core::mem::take;

use attr_var::AttributeVariable;
use name::VariableName;
use value::VariableValue;

use super::keyword::attributes::AttributeKeyword;
use super::keyword::functions::FunctionKeyword;
use super::literal::Attribute;
use super::modifiers::push::Push as _;
use super::tree::api::Ast;
use crate::errors::api::{ErrorLocation, Located};
use crate::parser::keyword::control_flow::types::colon_ast::ColonAstCtrl;
use crate::utils::{display, repr_fullness};

/// Different variable cases
#[derive(Debug)]
pub struct Variable {
    /// Indicated if the variable is full
    full: bool,
    /// Contains the actual value of the variable
    value: VariableValue,
}

impl Variable {
    /// Returns the variable as an attribute value if it is not a lone variable
    /// name.
    pub const fn as_attribute_variable_mut(&mut self) -> Option<&mut AttributeVariable> {
        if let VariableValue::AttributeVariable(attribute_variable) = &mut self.value {
            Some(attribute_variable)
        } else {
            None
        }
    }

    /// Merges a [`Variable`] with another [`Variable`] and returns the result.
    pub fn extend(&mut self, other: Self) -> Result<(), String> {
        if self.full {
            Err("Can't extend full variable".to_owned())
        } else {
            let other_full = other.full;
            self.value.extend(other)?;
            if other_full {
                self.full = true;
            }
            Ok(())
        }
    }

    /// Makes the variable full
    pub const fn fill(&mut self) {
        self.full = true;
    }

    /// Checks if the variable contains attributes
    pub const fn has_empty_attrs(&self) -> bool {
        self.value.has_empty_attrs()
    }

    /// Takes the attributes from inside self it is a type;
    pub fn into_type(self) -> Option<Vec<Located<Attribute>>> {
        match self.value {
            VariableValue::AttributeVariable(attr) => attr.into_type(),
            VariableValue::VariableName(..) => None,
        }
    }

    /// Returns the variable name if the variable is a user defined variable
    pub fn into_user_defined_name(self) -> Result<Located<String>, &'static str> {
        self.value.into_user_defined_name()
    }

    /// Returns the value of the variable.
    pub fn into_value(self) -> VariableValue {
        self.value
    }

    /// Checks if the variable is a user defined variable
    pub const fn is_declaration(&self) -> bool {
        matches!(self.value, VariableValue::AttributeVariable(_))
    }

    /// Checks if an [`Ast`] is valid as a whole and represents an expression.
    pub const fn is_finished_expr(&self) -> bool {
        matches!(self.value, VariableValue::VariableName(_, VariableName::UserDefined(_)))
    }

    /// Checks if the variable is full
    pub const fn is_full(&self) -> bool {
        self.full
    }

    /// Returns the location of the entire variable.
    pub fn location(&self) -> ErrorLocation {
        match &self.value {
            VariableValue::AttributeVariable(attr) => attr.location(),
            VariableValue::VariableName(loc, _) => *loc,
        }
    }

    /// Adds an attribute to the variable
    fn push_attr(&mut self, attr: Located<Attribute>) -> Result<(), String> {
        if self.full {
            Err("Can't push attribute to full variable".to_owned())
        } else {
            self.value.push_attr(attr)
        }
    }

    /// Pushes an ast as leaf in the current variable.
    pub fn push_block_as_leaf(&mut self, ast: Ast) -> Result<(), String> {
        #[cfg(feature = "debug")]
        crate::errors::api::Print::push_leaf(&ast, self, "var");
        if self.full {
            Err("Can't push ast to full variable".to_owned())
        } else if let Ast::Variable(var) = ast {
            self.extend(var)
        } else {
            match &mut self.value {
                VariableValue::AttributeVariable(decl) => decl.push_block_as_leaf(ast),
                VariableValue::VariableName(_, name) => {
                    unreachable!("tried to push block {ast} on non-declaration variable {name}")
                }
            }
        }
    }

    /// Pushes a colon `:` into a variable node.
    pub fn push_colon(&mut self, colon_location: ErrorLocation) -> Result<Option<Ast>, String> {
        match &mut self.value {
            VariableValue::VariableName(loc, VariableName::UserDefined(label)) =>
                Ok(Some(ColonAstCtrl::from_label_with_colon(take(loc).wrap(take(label))))),
            VariableValue::VariableName(_, VariableName::Keyword(kwd)) => Err(format!(
                "found `:` after keyword {kwd}: colon is only valid after user-defined label"
            )),
            VariableValue::AttributeVariable(_) if self.full => {
                Err("Colon unexpected in this context: neither variable declaration not ternary operator.".to_owned())
            }
            VariableValue::AttributeVariable(attr) => attr.push_colon(colon_location).map(|()| None),
        }
    }

    /// Adds a `*` indirection attribute to the variable
    pub fn push_indirection(&mut self, location: ErrorLocation) -> Result<(), String> {
        self.push_attr(location.wrap(Attribute::Indirection))
    }

    /// Adds a `*` indirection attribute to the variable
    pub fn push_keyword(&mut self, keyword: Located<AttributeKeyword>) -> Result<(), String> {
        self.push_attr(keyword.transfer(Attribute::Keyword))
    }

    /// Takes the value of `self` and puts a placeholder in its place.
    pub fn take(&mut self) -> Self {
        Self { full: self.full, value: self.value.take() }
    }

    /// Tries transforming the [`Self`] into a user defined variable name.
    pub fn take_user_defined(&mut self) -> Option<Located<String>> {
        self.value.take_user_defined()
    }
}

impl From<Located<AttributeKeyword>> for Variable {
    fn from(value: Located<AttributeKeyword>) -> Self {
        Self {
            full: false,
            value: VariableValue::AttributeVariable(AttributeVariable::from(value)),
        }
    }
}

impl From<Located<FunctionKeyword>> for Variable {
    fn from(value: Located<FunctionKeyword>) -> Self {
        let (inner, loc) = value.into_inner();
        Self {
            full: false,
            value: VariableValue::VariableName(loc, VariableName::Keyword(inner)),
        }
    }
}

impl From<Located<String>> for Variable {
    fn from(value: Located<String>) -> Self {
        let (inner, loc) = value.into_inner();
        Self {
            full: false,
            value: VariableValue::VariableName(loc, VariableName::UserDefined(inner)),
        }
    }
}

display!(Variable, self, f, write!(f, "{}{}", self.value, repr_fullness(self.full)));

/// Makes an error for values found after a [`FunctionKeyword`].
fn after_keyword_err<T: fmt::Display>(name: &str, value: T, keyword: &FunctionKeyword) -> String {
    format!("Found {name} {value} after function keyword {keyword}, but this is not allowed.")
}
