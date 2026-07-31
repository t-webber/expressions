//! Public functions to interactive with the lineariser state.

extern crate alloc;
use alloc::collections::btree_map::BTreeMap;

use crate::Res;
use crate::errors::api::{CompileError, ErrorLocation};
use crate::lineariser::state::LState;
use crate::lineariser::symbol::{ElementBuilder, FunctionBuilder, Symbol};

impl LState {
    /// Decrements the depth: exits a block.
    pub fn decrement_depth(&mut self) {
        for (name, element) in self
            .declarations
            .pop()
            .expect("can't decrement without first incrementing")
        {
            self.symbols.push(element.with_name(name));
        }
    }

    /// Returns the ID of a declaration by name, if found.
    pub fn find_declaration(&self, fname: &str) -> Option<&ElementBuilder> {
        for table in self.declarations.iter().rev() {
            if let Some(symbol) = table.get(fname) {
                return Some(symbol);
            }
        }
        None
    }

    /// Returns the ID of a function by name, if found.
    pub fn find_function(&self, fname: &str) -> Option<&FunctionBuilder> {
        self.functions.get(fname)
    }

    /// Increments the depth: enters a block.
    pub fn increment_depth(&mut self) {
        self.declarations.push(BTreeMap::new());
    }

    /// Creates the state to parse the global scope.
    pub fn init(&mut self) {
        self.declarations.push(BTreeMap::new());
    }

    /// Returns the inner [`Ssa`](crate::lineariser::Ssa).
    #[expect(clippy::unwrap_used, reason = "checked")]
    pub fn into_symbol_list(mut self) -> Res<Vec<Symbol>> {
        debug_assert!(self.declarations.len() == 1, "unclosed block");
        self.declarations
            .into_iter()
            .next()
            .unwrap()
            .into_iter()
            .for_each(|(name, builder)| self.symbols.push(builder.with_name(name)));
        self.literals
            .into_iter()
            .for_each(|(value, lit)| self.symbols.push(lit.with_value(value)));
        self.functions
            .into_iter()
            .for_each(|(name, func)| self.symbols.push(func.with_name(name)));
        Res::from((self.symbols, self.errors))
    }

    /// Adds an error to the state.
    pub fn push_error(&mut self, err: CompileError) {
        self.errors.push(err);
    }

    /// Adds a _statement not expression_ error on the given location.
    pub fn stat_not_expr(&mut self, loc: ErrorLocation, scope: &str) {
        self.push_error(loc.fail(format!("Expected expression in {scope}, got statement")));
    }

    /// Stores the errors of a result and returns the inner value.
    ///
    /// # Panics
    ///
    /// If the res doesn't contain any value.
    pub fn store_errors<T>(&mut self, res: Res<T>) -> T {
        res.store_errors(&mut |err| self.push_error(err))
            .expect("invariant")
    }
}
