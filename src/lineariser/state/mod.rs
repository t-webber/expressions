//! Walks the [`Ast`](crate::parser::api::Ast) and converts it to the
//! [`Ssa`](super::ssa::Ssa).

mod api;
mod helpers;
mod push;

extern crate alloc;
use alloc::collections::BTreeMap;
use std::collections::HashMap;

use crate::errors::api::CompileError;
use crate::lineariser::symbol::{ElementBuilder, FunctionBuilder, LiteralBuilder, Symbol};
use crate::parser::api::Literal;

/// Linearising State used to convert the parsed
/// [`Ast`](crate::parser::api::Ast) into a [`Ssa`](super::ssa::Ssa).
#[derive(Default, Debug)]
#[expect(clippy::field_scoped_visibility_modifiers, reason = "bad lint")]
pub struct LState {
    /// Array of length `depth` containing the variables declared in this scope.
    pub(super) declarations: Vec<BTreeMap<String, ElementBuilder>>,
    /// Errors that occurred while linearising the Ast.
    pub(super) errors: Vec<CompileError>,
    /// Declared functions.
    pub(super) functions: BTreeMap<String, FunctionBuilder>,
    /// Literals to put in rodata.
    pub(super) literals: HashMap<Literal, LiteralBuilder>,
    /// Unique id of the next symbol to be declared.
    pub(super) next_symbol_id: usize,
    /// The actual values of the built symbols, ready to be handed over to the
    /// Ssa.
    pub(super) symbols: Vec<Symbol>,
}
