//! Implementation of the `typedef` keyword.

use core::fmt;

use crate::EMPTY;
use crate::errors::api::{CompileError, ErrorLocation, Located};
use crate::parser::api::Attribute;
use crate::parser::keyword::control_flow::node::ControlFlowNode;
use crate::parser::keyword::control_flow::traits::ControlFlow;
use crate::parser::keyword::control_flow::types::ident_block::IdentBlockCtrl;
use crate::parser::modifiers::push::Push;
use crate::parser::operators::api::OperatorConversions;
use crate::parser::tree::Ast;
use crate::parser::variable::Variable;
use crate::parser::variable::api::VariableConversion as _;
use crate::utils::{display, repr_fullness, repr_option};

/// Content of the typedef, i.e., what it aliases and to what.
///
/// This is a parsing state, meaning it may contain unfinished nodes. For the
/// finished type, refer to [`TypedefValidContent`].
#[derive(Debug, Default)]
enum TypedefContent {
    /// Typedef in a type definition
    ///
    /// # Examples
    ///
    /// ```c
    /// typedef struct {} name;
    /// ```
    Definition(Box<ControlFlowNode>, Option<Located<String>>),
    /// Typedef without any arguments
    #[default]
    None,
    /// Typedef in a type redefinition
    ///
    /// # Examples
    ///
    /// ```c
    /// typedef struct A name;
    /// typedef int name2;
    /// ```
    Type(Variable),
}

/// Content of the typedef, i.e., what it aliases and to what.
#[derive(Debug)]
pub enum TypedefValidContent {
    /// Typedef in a type definition
    ///
    /// # Examples
    ///
    /// ```c
    /// typedef struct {} name;
    /// ```
    Definition(Box<IdentBlockCtrl>, Located<String>),
    /// Typedef in a type redefinition
    ///
    /// # Examples
    ///
    /// ```c
    /// typedef struct A name;
    /// typedef int name2;
    /// ```
    Type(Vec<Located<Attribute>>),
}

/// Control flow for `typedef` keyword.
#[derive(Debug)]
pub struct TypedefCtrl(TypedefContent, ErrorLocation);

impl TypedefCtrl {
    /// Returns the inner declaration of the typedef.
    pub fn into_inner(self) -> Result<TypedefValidContent, CompileError> {
        let loc = self.location();
        match self.0 {
            TypedefContent::Definition(ctrl, maybe_name) =>
                if let ControlFlowNode::IdentBlock(ident_block_ctrl) = *ctrl {
                    maybe_name.map_or_else(|| Err(loc.fail("Missing name after aggregate type in typedef.".to_owned())), |name| Ok(TypedefValidContent::Definition(Box::new(ident_block_ctrl), name)))
                } else {
                    Err(loc.fail("Typedef expected type, found control flow.".to_owned()))
                },
            TypedefContent::None =>
                Err(loc.fail("Missing aliased type and name after typedef.".to_owned())),
            TypedefContent::Type(variable) => variable
                .into_attrs()
                .map_err(|err| loc.fail(err))
                .map(TypedefValidContent::Type),
        }
    }
}

impl ControlFlow for TypedefCtrl {
    type Keyword = ErrorLocation;

    fn as_ast(&self) -> Option<&Ast> {
        match &self.0 {
            TypedefContent::Definition(ctrl, None) => ctrl.as_ast(),
            TypedefContent::None
            | TypedefContent::Type(_)
            | TypedefContent::Definition(_, Some(_)) => None,
        }
    }

    fn as_ast_mut(&mut self) -> Option<&mut Ast> {
        match &mut self.0 {
            TypedefContent::Definition(ctrl, None) => ctrl.as_ast_mut(),
            TypedefContent::None
            | TypedefContent::Type(_)
            | TypedefContent::Definition(_, Some(_)) => None,
        }
    }

    fn fill(&mut self) {}

    fn from_keyword(keyword: ErrorLocation) -> Self {
        Self(TypedefContent::None, keyword)
    }

    fn is_full(&self) -> bool {
        match &self.0 {
            TypedefContent::Definition(_, ident) => ident.is_some(),
            TypedefContent::None => false,
            TypedefContent::Type(var) => var.is_full(),
        }
    }

    fn location(&self) -> ErrorLocation {
        match &self.0 {
            TypedefContent::Definition(_, Some(name)) => name.as_location(),
            TypedefContent::Definition(block, None) => block.location(),
            TypedefContent::None => self.1,
            TypedefContent::Type(var) => var.location(),
        }
        .into_extended(self.1)
    }

    fn push_colon(&mut self) -> bool {
        match &mut self.0 {
            TypedefContent::Definition(ctrl, None) => ctrl.push_colon(),
            TypedefContent::Definition(_, Some(_))
            | TypedefContent::None
            | TypedefContent::Type(_) => false,
        }
    }

    fn push_semicolon(&mut self) -> bool {
        match &mut self.0 {
            TypedefContent::Definition(ctrl, None) => ctrl.push_semicolon(),
            TypedefContent::Definition(_, Some(_))
            | TypedefContent::None
            | TypedefContent::Type(_) => false,
        }
    }
}

impl Push for TypedefCtrl {
    fn push_block_as_leaf(&mut self, ast: Ast) -> Result<(), String> {
        #[cfg(feature = "debug")]
        crate::errors::api::Print::push_leaf(&ast, self, "typedef");
        debug_assert!(!self.is_full(), "");
        if let Ast::Variable(mut new_var) = ast {
            match &mut self.0 {
                TypedefContent::Definition(_, Some(_)) => unreachable!("typedef full"),
                TypedefContent::Definition(ctrl, current_name @ None) => {
                    if let Some(child) = ctrl.as_ast_mut() {
                        child.push_block_as_leaf(Ast::Variable(new_var))?;
                    } else if current_name.is_none()
                        && let Some(new_name) = new_var.take_user_defined()
                    {
                        *current_name = Some(new_name);
                    } else {
                        return Err(format!(
                            "Tried to push variable {new_var} in partially full typedef {self}."
                        ));
                    }
                }
                TypedefContent::Type(current_var) => current_var.extend(new_var)?,
                TypedefContent::None => self.0 = TypedefContent::Type(new_var),
            }
            Ok(())
        } else if let Ast::ControlFlow(new_ctrl) = ast {
            match &mut self.0 {
                TypedefContent::Definition(_, Some(_)) => unreachable!("typedef full"),
                TypedefContent::Definition(ctrl, None) =>
                    ctrl.push_block_as_leaf(Ast::ControlFlow(new_ctrl)),
                TypedefContent::None => {
                    self.0 = TypedefContent::Definition(Box::new(new_ctrl), None);
                    Ok(())
                }
                TypedefContent::Type(_) => Err("Found control flow after typedef name.".to_owned()),
            }
        } else {
            match &mut self.0 {
                TypedefContent::Definition(ctrl, None) => ctrl.push_block_as_leaf(ast),
                TypedefContent::Type(var) =>
                    if let Some((user_type, name)) = var.as_partial_typedef() {
                        let mut ctrl = user_type.into_control_flow(name, None);
                        ctrl.push_block_as_leaf(ast)?;
                        self.0 = TypedefContent::Definition(Box::new(ctrl), None);
                        Ok(())
                    } else {
                        Err(format!("Tried to push illegal ast {ast} in typedef {self}."))
                    },
                TypedefContent::Definition(_, Some(_)) | TypedefContent::None =>
                    Err(format!("Tried to push illegal ast {ast} in typedef {self}.")),
            }
        }
    }

    fn push_op<T>(&mut self, op: T) -> Result<(), String>
    where
        T: OperatorConversions + fmt::Display,
    {
        #[cfg(feature = "debug")]
        crate::errors::api::Print::push_op(&op, self, "typedef");
        debug_assert!(!self.is_full(), "");
        match &mut self.0 {
            TypedefContent::Definition(_, Some(_)) => unreachable!("Pushed in full"),
            TypedefContent::Definition(ctrl, None) => ctrl.push_op(op),
            TypedefContent::None => Err(format!("Illegal symbol {op} for typedef.")),
            TypedefContent::Type(var) => op.as_star().map_or_else(
                || Err(format!("Can't use {op} in typedef declarations.")),
                |loc| var.push_indirection(loc),
            ),
        }
    }
}

display!(
    TypedefCtrl,
    self,
    f,
    write!(
        f,
        "<typedef {}{}>",
        match &self.0 {
            TypedefContent::Definition(node, name) => format!("{node} {}", repr_option(name)),
            TypedefContent::Type(variable) => variable.to_string(),
            TypedefContent::None => EMPTY.to_owned(),
        },
        repr_fullness(self.is_full())
    )
);
