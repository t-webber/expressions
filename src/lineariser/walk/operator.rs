//! Walks a ternary expression, updating state and creating symbols and basic
//! blocks.

use crate::lineariser::basic_block::{BasicBlocks, Id};
use crate::lineariser::state::LState;
use crate::lineariser::symbol::Value;
use crate::parser::api::{Binary, Ternary, Unary};

impl Unary {
    /// Pushes some content into the [`BasicBlocks`].
    pub fn push_in(self, bbs: &mut BasicBlocks, state: &mut LState) -> Id {
        let loc = if self.arg.is_empty() {
            self.op.as_location()
        } else {
            self.arg.location()
        };
        match self.arg.push_in(bbs, state) {
            Some(Id::NotFound) => Id::NotFound,
            Some(Id::Found(id, ty)) => {
                let result = state.store_errors(ty.apply_unary(&self.op));
                Id::Found(
                    state.push_element(Value::Unary(*self.op.as_value(), id), result.clone()),
                    result,
                )
            }
            None => {
                state.stat_not_expr(loc, "unary");
                Id::NotFound
            }
        }
    }
}

impl Binary {
    /// Pushes some content into the [`BasicBlocks`].
    pub fn push_in(self, bbs: &mut BasicBlocks, state: &mut LState) -> Id {
        #[cfg(feature = "debug")]
        crate::lgp!(notab: "Pushing binary {self}");
        let Self { arg_l, arg_r, op } = self;
        let loc_l = arg_l.location();
        if arg_r.is_empty() {
            state.push_error(
                loc_l
                    .into_extended(op.as_location())
                    .fail("Missing RHS of binary operator".to_owned()),
            );
            return Id::NotFound;
        }
        let loc_r = arg_r.location();
        match (arg_l.push_in(bbs, state), arg_r.push_in(bbs, state)) {
            (Some(Id::NotFound), _) | (_, Some(Id::NotFound)) => Id::NotFound,
            res @ ((None, _) | (_, None)) => {
                if res.0.is_none() {
                    state.stat_not_expr(loc_l, "binary lhs");
                }
                if res.1.is_none() {
                    state.stat_not_expr(loc_r, "binary rhs");
                }
                Id::NotFound
            }
            (Some(Id::Found(id_l, ty_l)), Some(Id::Found(id_r, ty_r))) => {
                let ty = state.store_errors(ty_l.apply_binary(&op, &ty_r));
                Id::Found(
                    state.push_element(Value::Binary(op.drop_location(), id_l, id_r), ty.clone()),
                    ty,
                )
            }
        }
    }
}

impl Ternary {
    /// Pushes some content into the [`BasicBlocks`].
    pub fn push_in(self, bbs: &mut BasicBlocks, state: &mut LState) -> Id {
        #[cfg(feature = "debug")]
        crate::lgp!(notab: "Pushing ternary {self}");
        match self {
            Self { condition, success, failure: None, .. } => {
                state.push_error(
                    condition
                        .location()
                        .into_extended(success.location())
                        .fail("Missing ':' after ternary operator".to_owned()),
                );
                Id::NotFound
            }
            Self { condition, failure: Some((loc, failure)), .. } if failure.is_empty() => {
                state.push_error(
                    condition
                        .location()
                        .into_extended(loc)
                        .fail("Missing node after ':' in ternary operator".to_owned()),
                );
                Id::NotFound
            }
            Self { condition, success, failure: Some((_, failure)) } => {
                let loc_cond = condition.location();
                let loc_succ = success.location();
                let loc_fail = failure.location();
                match (
                    condition.push_in(bbs, state),
                    success.push_in(bbs, state),
                    failure.push_in(bbs, state),
                ) {
                    res @ ((_, _, None) | (_, None, _) | (None, _, _)) => {
                        if res.0.is_none() {
                            state.stat_not_expr(loc_cond, "ternary condition");
                        }
                        if res.1.is_none() {
                            state.stat_not_expr(loc_succ, "ternary success");
                        }
                        if res.2.is_none() {
                            state.stat_not_expr(loc_fail, "ternary failure");
                        }
                        Id::NotFound
                    }
                    (Some(Id::NotFound), _, _)
                    | (_, Some(Id::NotFound), _)
                    | (_, _, Some(Id::NotFound)) => Id::NotFound,
                    (
                        Some(Id::Found(node_c, ty_c)),
                        Some(Id::Found(node_s, ty_s)),
                        Some(Id::Found(node_f, ty_f)),
                    ) => {
                        let ty = state.store_errors(ty_c.apply_ternary(&ty_s, &ty_f));
                        Id::Found(
                            state.push_element(Value::Ternary(node_c, node_s, node_f), ty.clone()),
                            ty,
                        )
                    }
                }
            }
        }
    }
}
