//! Walks a generic ast, updating state and creating symbols and basic blocks.

use crate::lineariser::basic_block::{BasicBlocks, Id, Instruction};
use crate::lineariser::state::LState;
use crate::lineariser::symbol::Value;
use crate::lineariser::types::Type;
use crate::parser::api::{Ast, Cast, ControlFlowNode, TypedefValidContent};

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
            Self::Variable(var) => var.push_in(bbs, state),
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
            Self::Unary(un) => Some(un.push_in(bbs, state)),
            Self::ParensBlock(parens) => {
                let (inner, loc) = parens.into_inner();
                let id = inner.push_in(bbs, state);
                if id.is_none() {
                    state.stat_not_expr(loc, "parenthesised expression");
                }
                id
            }
            Self::Cast(Cast { dest_type, value, .. }) => {
                let value_loc = value.location();
                match value.push_in(bbs, state) {
                    Some(Id::Found(id, _)) => {
                        let ty = state.store_errors(Type::from_attributes(&dest_type));
                        Some(Id::Found(state.push_element(Value::Variable(id), ty.clone()), ty))
                    }
                    Some(Id::NotFound) => Some(Id::NotFound),
                    None => {
                        state.stat_not_expr(value_loc, "cast");
                        Some(Id::NotFound)
                    }
                }
            }
            Self::ControlFlow(ControlFlowNode::Typedef(typedef)) => {
                match typedef.into_inner() {
                    Ok((TypedefValidContent::Definition(..), _)) => todo!(),
                    Ok((TypedefValidContent::Type(_), _)) => todo!(),
                    Err(err) => state.push_error(err),
                }
                None
            }
            Self::FunctionArgsBuild(..) | Self::ListInitialiser(_) | Self::ControlFlow(_) =>
                todo!("{self:?}"),
        }
    }
}
