//! Methods to test if push a specific token into an [`Ast`] is possible

use super::Ast;
use crate::errors::api::Located;
use crate::parser::literal::Attribute;

impl CanPush for Ast {
    fn can_push_leaf(&self) -> bool {
        self.can_push_leaf_with_ctx(AstPushContext::None)
    }
}

/// Context to specify what are we trying to push into the [`Ast`].
///
/// See [`Ast::can_push_leaf`] for more information.
#[derive(Debug, Default, PartialEq, Eq)]
pub enum AstPushContext {
    /// Any context is good
    Any,
    /// Castable object.
    Cast,
    /// Trying to see if an `else` block ca be added
    Else,
    /// Nothing particular
    #[default]
    None,
    /// Trying to see if the last element of the [`Ast`] waiting for variables.
    UserVariable,
}

impl AstPushContext {
    /// Checks if the context can have a cast.
    pub const fn is_castable(&self) -> bool {
        matches!(self, &Self::Any | &Self::UserVariable | &Self::Cast)
    }

    /// Checks if the context can have an `else`
    pub const fn is_else(&self) -> bool {
        matches!(self, &Self::Any | &Self::Else)
    }

    /// Checks if the context can have a variable
    pub const fn is_user_variable(&self) -> bool {
        matches!(self, &Self::Any | &Self::UserVariable)
    }
}

/// Methods to check if a node is pushable
///
/// # Note
///
/// Whether an [`Ast`] is pushable or not sometimes depends on what it is we
/// are trying to push. This is goal of the [`AstPushContext`].
///
/// # Examples
///
/// When pushing an `else` keyword into an
/// [`ControlFlowNode`](crate::parser::keyword::control_flow::node::ControlFlowNode),
/// the latter is pushable iff the control flow is complete (not
/// necessary full)! But when pushing a literal into a
/// control flow, the latter is not pushable iff the control flow is full (not
/// only complete). See
/// [`is_full`](crate::parser::keyword::control_flow::traits::ControlFlow::is_full)
/// and [`is_complete`](crate::parser::keyword::control_flow::traits::ControlFlow::is_complete)
/// to see the difference.
pub trait CanPush {
    /// See [`CanPush`] documentation.
    fn can_push_leaf(&self) -> bool;
}

/// Finds the leaf the most left possible, checks it is a variable and
/// pushes it some attributes.
///
/// This is used when finding an identifier or keyword after a full [`Ast`],
/// where it is meant as a type declaration.
pub trait PushAttribute {
    /// See [`PushAttribute`].
    fn add_attribute_to_left_variable(
        &mut self,
        previous_attrs: Vec<Located<Attribute>>,
    ) -> Result<(), String>;
}
