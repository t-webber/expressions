//!Implement the `return` control flow

use core::fmt;

use crate::errors::api::ErrorLocation;
use crate::parser::keyword::control_flow::node::try_push_semicolon_control;
use crate::parser::keyword::control_flow::traits::ControlFlow;
use crate::parser::modifiers::push::Push;
use crate::parser::operators::api::OperatorConversions;
use crate::parser::tree::Ast;
use crate::parser::tree::api::CanPush as _;
use crate::utils::{display, repr_fullness};

/// Keyword expects a node: `return 3+4`
#[derive(Debug, Default)]
pub struct ReturnCtrl {
    /// fullness of the value
    full: bool,
    /// Location of the return keyword.
    return_location: ErrorLocation,
    /// [`Ast`] that is returned.
    value: Box<Ast>,
}

impl ReturnCtrl {
    /// Returns the value returned by the control flow.
    pub fn into_value(self) -> Box<Ast> {
        self.value
    }
}

impl ControlFlow for ReturnCtrl {
    type Keyword = ErrorLocation;

    fn as_ast(&self) -> Option<&Ast> {
        (!self.full).then(|| self.value.as_ref())
    }

    fn as_ast_mut(&mut self) -> Option<&mut Ast> {
        (!self.full).then(|| self.value.as_mut())
    }

    fn fill(&mut self) {
        self.full = true;
    }

    fn from_keyword(keyword: ErrorLocation) -> Self {
        Self { return_location: keyword, ..Self::default() }
    }

    fn is_full(&self) -> bool {
        self.full
    }

    fn location(&self) -> ErrorLocation {
        if self.value.is_empty() {
            self.return_location
        } else {
            self.value.location().into_extended(self.return_location)
        }
    }

    fn push_colon(&mut self) -> bool {
        false
    }

    fn push_semicolon(&mut self) -> bool {
        if self.full {
            false
        } else {
            if try_push_semicolon_control(&mut self.value) {
                if !self.value.can_push_leaf() {
                    self.full = true;
                }
            } else {
                self.full = true;
            }
            true
        }
    }
}

impl Push for ReturnCtrl {
    fn push_block_as_leaf(&mut self, ast: Ast) -> Result<(), String> {
        #[cfg(feature = "debug")]
        crate::errors::api::Print::push_leaf(&ast, self, "return");
        debug_assert!(!self.is_full(), "");
        self.value.push_block_as_leaf(ast)
    }

    fn push_op<T>(&mut self, op: T) -> Result<(), String>
    where
        T: OperatorConversions + fmt::Display,
    {
        #[cfg(feature = "debug")]
        crate::errors::api::Print::push_op(&op, self, "return");
        debug_assert!(!self.is_full(), "");
        self.value.push_op(op)
    }
}

display!(
    ReturnCtrl,
    self,
    f,
    write!(f, "<return {}{}>", self.value, repr_fullness(self.full))
);
