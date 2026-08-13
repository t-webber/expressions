//! Implements the [`Push`] trait for [`Ast`]

use core::cmp::Ordering;
use core::fmt;
use core::mem::take;

use super::Ast;
use super::can_push::PushAttribute;
use crate::errors::api::Located;
use crate::parser::keyword::control_flow::traits::ControlFlow as _;
use crate::parser::literal::Attribute;
use crate::parser::modifiers::push::Push;
use crate::parser::operators::api::{
    Associativity, Binary, Operator as _, OperatorConversions, Ternary, TernaryOperator, Unary
};
use crate::parser::symbols::api::{BracedBlock, Cast, ListInitialiser};
use crate::parser::variable::api::PureType as _;

impl Push for Ast {
    fn push_block_as_leaf(&mut self, ast: Self) -> Result<(), String> {
        #[cfg(feature = "debug")]
        crate::errors::api::Print::push_leaf(&ast, self, "ast");
        match self {
            Self::Empty => {
                *self = ast;
                Ok(())
            }
            // full: ok, but create a new block
            // Example: {a}b
            Self::BracedBlock(BracedBlock { full: true, .. }) => {
                self.brace();
                self.push_block_as_leaf(ast)
            }
            // previous is incomplete variable: waiting for variable name
            Self::Variable(var) => var.push_block_as_leaf(ast),
            Self::ParensBlock(old) => {
                let (parens, parens_location) = take(old).into_inner();
                if let Some(ty) = parens.into_type() {
                    let full = if let Self::ParensBlock(parens_value) = &ast
                        && parens_value.is_pure_type()
                    {
                        false
                    } else {
                        true
                    };
                    *self = Self::Cast(Cast {
                        dest_type: ty,
                        full,
                        value: ast.into_box(),
                        parens_location,
                    });
                    Ok(())
                } else {
                    Err(successive_literal_error("Parenthesis group", old, ast))
                }
            }
            Self::Leaf(old) => Err(successive_literal_error("Literal", old, ast)),
            Self::FunctionCall(_) => Err(successive_literal_error("Function call", self, ast)),
            Self::ListInitialiser(ListInitialiser { full: true, .. }) =>
                Err(successive_literal_error("List initialiser", self, ast)),
            Self::Unary(Unary { arg, .. })
            | Self::Binary(Binary { arg_r: arg, .. })
            | Self::Ternary(
                Ternary { failure: Some((_, arg)), .. } | Ternary { success: arg, .. },
            ) => arg.push_block_as_leaf(ast),
            Self::FunctionArgsBuild(vec, ..)
            | Self::ListInitialiser(ListInitialiser { elts: vec, full: false, .. })
            | Self::BracedBlock(BracedBlock { elts: vec, full: false, .. }) =>
                (Self::push_block_as_leaf_in_vec(vec, ast)?).map_or(Ok(()), |err_node| {
                    Err(successive_literal_error("block", self, err_node))
                }),
            Self::Cast(Cast { full, value, .. }) =>
                if *full {
                    Err(successive_literal_error("cast", self, ast))
                } else {
                    value.push_block_as_leaf(ast)
                },
            Self::ControlFlow(ctrl) if ctrl.is_complete() => {
                self.brace();
                self.push_block_as_leaf(ast)
            }
            Self::ControlFlow(ctrl) => ctrl.push_block_as_leaf(ast),
        }
    }

    fn push_op<T>(&mut self, op: T) -> Result<(), String>
    where
        T: OperatorConversions + fmt::Display,
    {
        #[cfg(feature = "debug")]
        crate::errors::api::Print::push_op(&op, self, "ast");
        match self {
            Self::Empty => op.try_convert_and_erase_node(self),
            Self::Variable(var) => {
                if !var.is_full()
                    && !op.is_array_subscript()
                    && let Some(attr) = var.as_attribute_variable_mut()
                {
                    attr.push_op(op)
                } else {
                    op.try_push_op_as_root(self)
                }
            }
            Self::Cast(cast) => {
                match Cast::precedence().cmp(&op.precedence()) {
                    Ordering::Less => op.try_push_op_as_root(self),
                    // doing whatever works for [`Ordering::Equal`] ? no ! e.g.: !g(!x) gives !!g(x)
                    // for `op.try_push_op_as_root(self)`
                    Ordering::Greater | Ordering::Equal => cast.value.push_op(op),
                }
            }
            // parens: check for casts
            Self::ParensBlock(parens) => parens.take_ast_with_op(op).map(|new| *self = new),
            // self is a non-modifiable block: Op -> Self
            Self::Leaf(_)
            | Self::FunctionCall(_)
            | Self::ListInitialiser(ListInitialiser { full: true, .. }) =>
                op.try_push_op_as_root(self),
            // full block: make space: Self = [Self, Empty]
            Self::BracedBlock(BracedBlock { full: true, .. }) => {
                self.brace();
                self.push_op(op)
            }
            // pushable list: self.last.push_op(op)
            Self::FunctionArgsBuild(vec, ..)
            | Self::BracedBlock(BracedBlock { elts: vec, full: false, .. })
            | Self::ListInitialiser(ListInitialiser { elts: vec, full: false, .. }) => {
                if let Some(last) = vec.last_mut() {
                    last.push_op(op)
                } else {
                    vec.push(op.try_to_node()?);
                    Ok(())
                }
            }
            Self::Unary(Unary { op: old_op, arg }) => {
                match old_op.precedence().cmp(&op.precedence()) {
                    Ordering::Less =>
                        if arg.is_finished_expr() {
                            op.try_push_op_as_root(self)
                        } else if op.is_binary() {
                            // * in ~*a shouldn't be considered a binary op
                            Err(String::new())
                        } else {
                            arg.push_op(op)
                        },
                    // doing whatever works for [`Ordering::Equal`] ? no ! e.g.: !g(!x) gives !!g(x)
                    // for `op.try_push_op_as_root(self)`
                    Ordering::Greater | Ordering::Equal => arg.push_op(op),
                }
            }
            Self::Binary(Binary { op: old_op, arg_r: arg, .. }) => {
                let associativity = op.associativity(); // same associativity for same precedence
                match (old_op.precedence().cmp(&op.precedence()), associativity) {
                    (Ordering::Less, _) | (Ordering::Equal, Associativity::LeftToRight) => {
                        if arg.is_empty() {
                            Err(format!("Missing RHS for binary operator '{old_op}'."))
                        } else {
                            op.try_push_op_as_root(self)
                        }
                    }
                    (Ordering::Greater, _) | (Ordering::Equal, Associativity::RightToLeft) =>
                        arg.push_op(op),
                }
            }
            Self::Ternary(Ternary { failure: Some((_, arg)), .. }) => {
                let associativity = op.associativity(); // same associativity for same precedence
                match (TernaryOperator.precedence().cmp(&op.precedence()), associativity) {
                    (Ordering::Less, _) | (Ordering::Equal, Associativity::LeftToRight) =>
                        op.try_push_op_as_root(self),
                    (Ordering::Greater, _) | (Ordering::Equal, Associativity::RightToLeft) =>
                        arg.push_op(op),
                }
            }
            // explicit derogation clause on success block of a ternary operator
            Self::Ternary(Ternary { success: arg, .. }) => arg.push_op(op),
            // Control flows
            Self::ControlFlow(ctrl) => ctrl.push_op(op),
        }
    }
}

impl PushAttribute for Ast {
    fn add_attribute_to_left_variable(
        &mut self,
        previous_attrs: Vec<Located<Attribute>>,
    ) -> Result<(), String> {
        #[cfg(feature = "debug")]
        crate::lgp!(
            "\tAdding attrs {} to ast {self}",
            crate::utils::repr_vec(&previous_attrs, ", ")
        );
        let make_error = |msg: &str| Err(format!("LHS: {msg} are illegal in type declarations."));
        match self {
            Self::Empty => Err("LHS: Missing argument.".to_owned()),
            Self::Variable(var) => var.add_attribute_to_left_variable(previous_attrs),
            Self::Leaf(_) => make_error("constant"),
            Self::ParensBlock(_) => make_error("parenthesis"),
            Self::Unary(Unary { arg, .. }) | Self::Binary(Binary { arg_l: arg, .. }) =>
                arg.add_attribute_to_left_variable(previous_attrs),
            Self::Ternary(Ternary { condition, .. }) =>
                condition.add_attribute_to_left_variable(previous_attrs),
            Self::Cast(_) => make_error("Casts"),
            Self::FunctionArgsBuild(..) => make_error("Functions arguments"),
            Self::FunctionCall(_) => make_error("Functions"),
            Self::ListInitialiser(_) => make_error("List initialisers"),
            Self::BracedBlock(_) => make_error("Blocks"),
            Self::ControlFlow(_) => make_error("Control flow keywords"),
        }
    }
}

/// Makes an error [`String`] for consecutive literals.
///
/// If two consecutive literals are found, the [`crate::parser`] fails, and this
/// is the generic function to make the uniformed-string-value-error.
fn successive_literal_error<T: fmt::Display, U: fmt::Display>(
    old_type: &str,
    old: T,
    new: U,
) -> String {
    format!("Found 2 consecutive literals: {old_type} {old} followed by {new}.")
}
