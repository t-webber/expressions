//!Implement the control flow with a parenthesised block and an ast, such as
//!`for`, `switch` and `while.`

use core::fmt;

use crate::errors::api::{ErrorLocation, Located};
use crate::parser::keyword::control_flow::node::try_push_semicolon_control;
use crate::parser::keyword::control_flow::traits::ControlFlow;
use crate::parser::modifiers::push::Push;
use crate::parser::operators::api::OperatorConversions;
use crate::parser::symbols::api::ParensBlock;
use crate::parser::tree::Ast;
use crate::parser::tree::api::AstPushContext;
use crate::utils::{display, repr_fullness, repr_option};

/// Keyword expects a parenthesised block and a braced block: `switch (cond){}`
#[derive(Debug)]
pub struct ParensBlockCtrl {
    /// Block expression after parens
    block: Box<Ast>,
    /// Fullness of the block
    full: bool,
    /// Control flow keyword
    keyword: Located<ParensBlockKeyword>,
    /// Parens expression
    parens: Option<ParensBlock>,
}

impl ControlFlow for ParensBlockCtrl {
    type Keyword = Located<ParensBlockKeyword>;

    fn as_ast(&self) -> Option<&Ast> {
        (!self.full).then(|| self.block.as_ref())
    }

    fn as_ast_mut(&mut self) -> Option<&mut Ast> {
        (!self.full).then(|| self.block.as_mut())
    }

    fn as_while(&self) -> Result<Option<ErrorLocation>, String> {
        if *self.keyword.as_value() == ParensBlockKeyword::While {
            if self.parens.is_some() {
                Err("Expected a lone keyword `while` after `do` block, but found parenthesis"
                    .to_owned())
            } else {
                Ok(Some(self.keyword.as_location()))
            }
        } else {
            Ok(None)
        }
    }

    fn fill(&mut self) {
        self.full = true;
    }

    fn from_keyword(keyword: Self::Keyword) -> Self {
        Self { keyword, parens: None, block: Ast::empty_box(), full: false }
    }

    fn is_full(&self) -> bool {
        self.full
    }

    fn is_switch(&self) -> bool {
        *self.keyword.as_value() == ParensBlockKeyword::Switch
    }

    fn location(&self) -> ErrorLocation {
        if self.block.is_empty() {
            self.keyword.as_location()
        } else {
            self.block
                .location()
                .into_extended(self.keyword.as_location())
        }
    }

    fn push_colon(&mut self) -> bool {
        false
    }

    fn push_semicolon(&mut self) -> bool {
        if self.full {
            false
        } else {
            if try_push_semicolon_control(&mut self.block) {
                if !self.block.can_push_leaf_with_ctx(AstPushContext::Any) {
                    self.full = true;
                }
            } else {
                self.full = true;
            }
            true
        }
    }
}

impl Push for ParensBlockCtrl {
    fn push_block_as_leaf(&mut self, ast: Ast) -> Result<(), String> {
        #[cfg(feature = "debug")]
        crate::errors::api::Print::push_leaf(&ast, self, "parens block");
        debug_assert!(!self.is_full(), "");
        if self.parens.is_some() {
            if let Ast::BracedBlock(braced_block) = ast {
                if self.block.is_empty() {
                    self.block = Ast::BracedBlock(braced_block).into_box();
                    self.full = true;
                } else {
                    self.block.push_braced_block(braced_block)?;
                    if !self
                        .block
                        .can_push_leaf_with_ctx(AstPushContext::UserVariable)
                    {
                        self.full = true;
                    }
                }
            } else {
                self.block.push_block_as_leaf(ast)?;
            }
            Ok(())
        } else if let Ast::ParensBlock(parens) = ast {
            self.parens = Some(parens);
            Ok(())
        } else {
            Err("Missing (.".to_owned())
        }
    }

    fn push_op<T>(&mut self, op: T) -> Result<(), String>
    where
        T: OperatorConversions + fmt::Display,
    {
        #[cfg(feature = "debug")]
        crate::errors::api::Print::push_op(&op, self, "parens block");
        debug_assert!(!self.is_full(), "");
        if self.parens.is_some() {
            self.block.push_op(op)
        } else {
            Err("Expected (, found operator".to_owned())
        }
    }
}

display!(
    ParensBlockCtrl,
    self,
    f,
    write!(
        f,
        "<{} {} {}{}>",
        self.keyword,
        repr_option(&self.parens),
        self.block,
        repr_fullness(self.full),
    )
);

/// C control flow keywords that have the [`ParensBlockCtrl`] structure.
#[derive(Debug, PartialEq, Eq)]
pub enum ParensBlockKeyword {
    /// `for (...) { }`
    For,
    /// `switch (...) { }`
    Switch,
    /// `while (...) { }`
    While,
}

display!(
    ParensBlockKeyword,
    self,
    f,
    match self {
        Self::For => "for".fmt(f),
        Self::Switch => "switch".fmt(f),
        Self::While => "while".fmt(f),
    }
);
