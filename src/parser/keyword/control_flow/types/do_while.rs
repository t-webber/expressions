//!Implement the `do-while` control flow

use core::fmt;

use crate::errors::api::ErrorLocation;
use crate::parser::keyword::control_flow::node::{ControlFlowNode, try_push_semicolon_control};
use crate::parser::keyword::control_flow::traits::ControlFlow;
use crate::parser::modifiers::push::Push;
use crate::parser::operators::api::OperatorConversions;
use crate::parser::symbols::api::{BracedBlock, ParensBlock};
use crate::parser::tree::Ast;
use crate::utils::{display, repr_option};

/// `do` keyword
#[derive(Debug, Default)]
pub struct DoWhileCtrl {
    /// looping condition, after the `while` keyword
    condition: Option<ParensBlock>,
    /// Location of `do` keyword
    keyword_location: ErrorLocation,
    /// [`Ast`] executed at each interaction
    loop_block: Box<Ast>,
    /// `while` keyword found or not.
    ///
    /// Used from `loop_block` to `condition`
    ///
    /// If the while was found, this contains the location of the while, else it
    /// is None
    while_found: Option<ErrorLocation>,
}

impl ControlFlow for DoWhileCtrl {
    type Keyword = ErrorLocation;

    fn as_ast(&self) -> Option<&Ast> {
        self.while_found.is_none().then(|| self.loop_block.as_ref())
    }

    fn as_ast_mut(&mut self) -> Option<&mut Ast> {
        self.while_found.is_none().then(|| self.loop_block.as_mut())
    }

    fn fill(&mut self) {}

    fn from_keyword(keyword: ErrorLocation) -> Self {
        Self { keyword_location: keyword, ..Self::default() }
    }

    fn is_full(&self) -> bool {
        self.condition.is_some()
    }

    fn location(&self) -> ErrorLocation {
        self.while_found.map_or_else(
            || {
                if self.loop_block.is_empty() {
                    self.keyword_location
                } else {
                    self.loop_block
                        .location()
                        .into_extended(self.keyword_location)
                }
            },
            |while_loc| while_loc.into_extended(self.keyword_location),
        )
    }

    fn push_colon(&mut self) -> bool {
        false
    }

    fn push_semicolon(&mut self) -> bool {
        if self.while_found.is_some() {
            false
        } else {
            try_push_semicolon_control(&mut self.loop_block) || {
                if let Ast::BracedBlock(BracedBlock { elts, full: false, .. }) =
                    &mut *self.loop_block
                {
                    elts.push(Ast::Empty);
                } else if !self.loop_block.is_empty() {
                    self.loop_block.brace();
                }
                true
            }
        }
    }
}

impl Push for DoWhileCtrl {
    fn push_block_as_leaf(&mut self, ast: Ast) -> Result<(), String> {
        #[cfg(feature = "debug")]
        crate::errors::api::Print::push_leaf(&ast, self, "do-while");
        debug_assert!(!self.is_full(), "");
        if self.while_found.is_some() {
            debug_assert!(self.condition.is_none(), "");
            if let Ast::ParensBlock(parens) = ast {
                self.condition = Some(parens);
                Ok(())
            } else {
                Err("Missing condition: expect (.".to_owned())
            }
        } else if let Ast::ControlFlow(ControlFlowNode::ParensBlock(ctrl)) = &ast
            && let Some(while_location) = ctrl.as_while()?
        {
            self.loop_block.fill();
            self.while_found = Some(while_location);
            Ok(())
        } else {
            self.loop_block.push_block_as_leaf(ast)
        }
    }

    fn push_op<T>(&mut self, op: T) -> Result<(), String>
    where
        T: OperatorConversions + fmt::Display,
    {
        #[cfg(feature = "debug")]
        crate::errors::api::Print::push_op(&op, self, "do-while");
        debug_assert!(!self.is_full(), "");
        if self.while_found.is_some() {
            Err("Expected condition, found operator".to_owned())
        } else {
            self.loop_block.push_op(op)
        }
    }
}

display!(
    DoWhileCtrl,
    self,
    f,
    write!(
        f,
        "<do {}{}>",
        self.loop_block,
        if self.while_found.is_some() {
            format!(" while {}", repr_option(&self.condition))
        } else {
            self.condition
                .as_ref()
                .map_or_else(|| "..".to_owned(), |cond| format!("{cond}"))
        }
    )
);
