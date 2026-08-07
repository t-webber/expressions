//! Walks a generic ast, updating state and creating symbols and basic blocks.

use crate::lineariser::basic_block::{BasicBlocks, Id, Instruction};
use crate::lineariser::state::LState;
use crate::lineariser::symbol::Value;
use crate::lineariser::types::Type;
use crate::parser::api::{Ast, ControlFlowNode, Unary, VariableName, VariableValue};

impl Ast {
    /// Pushes some content into the basic blocks.
    pub fn push_in(self, bbs: &mut BasicBlocks, state: &mut LState) -> Option<Id> {
        #[cfg(feature = "debug")]
        crate::lgp!(notab: "Pushing ast {self}");
        match self {
            Self::ControlFlow(ControlFlowNode::Ast(return_ctrl)) => {
                let value = return_ctrl.into_value();
                let loc = value.location();
                value.push_in(bbs, state).map_or_else(
                    || {
                        state.stat_not_expr(loc, "return");
                    },
                    |ret| {
                        if let Id::Found(id, _) = ret {
                            bbs.add(Instruction::Return(id));
                        }
                    },
                );
                None
            }
            Self::FunctionCall(func) => func.push_in(bbs, state),
            Self::Empty => None,
            Self::Variable(var) => match var.into_value() {
                VariableValue::AttributeVariable(attr) => {
                    attr.push_in(bbs, state);
                    None
                }
                VariableValue::VariableName(loc, VariableName::UserDefined(vname)) =>
                    #[expect(clippy::option_if_let_else, reason = "clippy bug")]
                    if let Some(decl) = state.find_declaration(&vname) {
                        Some(Id::Found(decl.metadata.id, decl.metadata.ty.clone()))
                    } else {
                        state.push_error(loc.fail(format!("Use of undeclared variable {vname}")));
                        Some(Id::NotFound)
                    },
                VariableValue::VariableName(loc, VariableName::Keyword(kwd)) => {
                    state.push_error(
                        loc.fail(format!(
                            "Keyword {kwd} is a function, but no arguments were given"
                        )),
                    );
                    Some(Id::NotFound)
                }
            },
            Self::Leaf(lit) => {
                let ty = Type::from_lit(lit.as_value());
                Some(Id::Found(state.push_literal(lit.drop_location()), ty))
            }
            Self::BracedBlock(bb) => {
                state.increment_depth();
                for elt in bb.elts {
                    elt.push_in(bbs, state);
                }
                state.decrement_depth();
                None
            }
            Self::Binary(bin) => Some(bin.push_in(bbs, state)),
            Self::Ternary(ter) => Some(ter.push_in(bbs, state)),
            Self::Unary(Unary { arg, op }) => {
                let loc = if arg.is_empty() {
                    op.as_location()
                } else {
                    arg.location()
                };
                match arg.push_in(bbs, state) {
                    Some(Id::NotFound) => Some(Id::NotFound),
                    Some(Id::Found(id, ty)) => {
                        let result = state.store_errors(ty.apply_unary(&op));
                        Some(Id::Found(
                            state.push_element(Value::Unary(*op.as_value(), id), result.clone()),
                            result,
                        ))
                    }
                    None => {
                        state.stat_not_expr(loc, "unary");
                        Some(Id::NotFound)
                    }
                }
            }
            Self::ParensBlock(parens) => {
                let (inner, loc) = parens.into_inner();
                let id = inner.push_in(bbs, state);
                if id.is_none() {
                    state.stat_not_expr(loc, "parenthesised expression");
                }
                id
            }
            Self::Cast(_)
            | Self::FunctionArgsBuild(..)
            | Self::ListInitialiser(_)
            | Self::ControlFlow(_) => todo!("{self:?}"),
        }
    }
}
