//! Walks the [`Ast`](crate::parser::api::Ast) and converts it to the
//! [`Ssa`](super::ssa::Ssa).

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::collections::btree_map::Entry;
use std::collections::{HashMap, HashSet};

use crate::Res;
use crate::errors::api::{CompileError, ErrorLocation, Located};
use crate::lineariser::basic_block::BasicBlocks;
use crate::lineariser::symbol::{ElementBuilder, FunctionBuilder, LiteralBuilder, Symbol, Value};
use crate::lineariser::types::{ReturnType, Type};
use crate::parser::api::{BracedBlock, Literal};
use crate::utils::SingleUse;

/// Linearising State used to convert the parsed
/// [`Ast`](crate::parser::api::Ast) into a [`Ssa`](super::ssa::Ssa).
#[derive(Default, Debug)]
pub struct LState {
    /// Array of length `depth` containing the variables declared in this scope.
    declarations: Vec<BTreeMap<String, ElementBuilder>>,
    /// Errors that occurred while linearising the Ast.
    errors: Vec<CompileError>,
    /// Declared functions.
    functions: BTreeMap<String, FunctionBuilder>,
    /// Literals to put in rodata.
    literals: HashMap<Literal, LiteralBuilder>,
    /// Unique id of the next symbol to be declared.
    next_symbol_id: usize,
    /// The actual values of the built symbols, ready to be handed over to the
    /// Ssa.
    symbols: Vec<Symbol>,
}

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

    /// Increment the id and return the one that can be used.
    ///
    /// This function ensures that every id is unique.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "todo: fail when no more ids available"
    )]
    const fn get_and_bump_symbol_id(&mut self) -> SingleUse<usize> {
        let old = self.next_symbol_id;
        self.next_symbol_id += 1;
        SingleUse::from(old)
    }

    /// Increments the depth: enters a block.
    pub fn increment_depth(&mut self) {
        self.declarations.push(BTreeMap::new());
    }

    /// Creates the state to parse the global scope.
    pub fn init(&mut self) {
        self.declarations.push(BTreeMap::new());
    }

    /// Returns the inner [`Ssa`](super::ssa::Ssa).
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

    /// Creates a variable [`Symbol`].
    pub fn push_declaration(&mut self, name: Located<String>, ty: &Type, value: Value) -> usize {
        let (name_v, loc) = name.into_inner();
        if self.functions.contains_key(&name_v) {
            self.errors
                .push(loc.fail(format!("Variable declaration shadows function {name_v}")));
        }
        let mut id = self.get_and_bump_symbol_id();
        let last = self.declarations.last_mut().expect("depth>=1");
        match last.entry(name_v.clone()) {
            Entry::Vacant(vacant) => {
                let symbol = ElementBuilder {
                    metadata: LiteralBuilder { id: id.as_value(), ty: ty.to_owned() },
                    value,
                };
                vacant.insert(symbol);
                id.as_value()
            }
            Entry::Occupied(mut occupied) => {
                let old_symbol = occupied.get_mut();
                match old_symbol {
                    ElementBuilder { metadata, .. } if *ty != metadata.ty => self.errors.push(
                        loc.crash(format!("Redeclaration of {name_v} with a different type")),
                    ),
                    ElementBuilder { value: old_val @ Value::DeclaredOnly, .. } => *old_val = value,
                    ElementBuilder { .. } =>
                        if !matches!(value, Value::DeclaredOnly) {
                            self.errors
                                .push(loc.crash(format!("Redefinition of variable {name_v}")));
                        },
                }
                let symbol_id = old_symbol.metadata.id;
                self.reset_symbol_id(id);
                symbol_id
            }
        }
    }

    /// Push an element into the Ssa.
    pub fn push_element(&mut self, value: Value, ty: Type) -> usize {
        let id = self.get_and_bump_symbol_id().as_value();
        self.symbols.push(Symbol::Element {
            name: None,
            value: ElementBuilder { value, metadata: LiteralBuilder { id, ty } },
        });
        id
    }

    /// Adds an error to the state.
    pub fn push_error(&mut self, err: CompileError) {
        self.errors.push(err);
    }

    /// Creates a function [`Symbol`].
    ///
    /// # Note
    ///
    /// The function is pushed into the function tables before the body being
    /// linearised to ensure recursion calls don't trigger a 'call to
    /// undeclared function'.
    pub fn push_function(
        &mut self,
        name: Located<String>,
        args: Vec<(Located<String>, Type)>,
        ret: ReturnType,
        maybe_fn_body: Option<BracedBlock>,
    ) {
        let (name_v, loc) = name.into_inner();
        if self.declarations.len() > 1 {
            self.errors
                .push(loc.fail("Non top-level functions is a GCC extension.".to_owned()));
        }
        self.increment_depth();

        if self.find_declaration(&name_v).is_some() {
            self.errors
                .push(loc.warn(format!("Function declaration shadows variable {name_v}")));
        }

        let mut symbol_args = vec![];
        let mut names = HashSet::new();
        for arg in args {
            let dup = !names.insert(arg.0.as_value().to_owned());
            if !arg.0.as_value().is_empty() {
                if dup {
                    self.errors.push(
                        arg.0
                            .as_location()
                            .fail("Multiple arguments have the same name".to_owned()),
                    );
                } else if self.find_declaration(arg.0.as_value()).is_some() {
                    self.errors.push(
                        arg.0
                            .as_location()
                            .warn("Function argument shadows global variable".to_owned()),
                    );
                }
            }
            let id = if arg.0.as_value().is_empty() {
                self.push_literal(Literal::Null)
            } else {
                self.push_declaration(arg.0.clone(), &arg.1, Value::DeclaredOnly)
            };
            symbol_args.push((id, arg.1));
        }

        let mut id = self.get_and_bump_symbol_id();
        match self.functions.entry(name_v.clone()) {
            Entry::Vacant(vacant) => {
                vacant.insert(FunctionBuilder {
                    args: symbol_args,
                    body: None,
                    ret,
                    id: id.as_value(),
                });
            }
            Entry::Occupied(mut occupied) => {
                let old_symbol = occupied.get_mut();
                match old_symbol {
                    FunctionBuilder { args: old_args, ret: old_ret, .. }
                        if symbol_args.len() != old_args.len()
                            || symbol_args
                                .iter()
                                .zip(old_args.iter())
                                .any(|((_, new_ty), (_, old_ty))| new_ty != old_ty)
                            || ret != *old_ret =>
                        self.errors.push(loc.crash(format!(
                            "Redeclaration of function {name_v} with a different signature"
                        ))),
                    FunctionBuilder { body: Some(_), .. } =>
                        if maybe_fn_body.is_some() {
                            self.errors
                                .push(loc.crash(format!("Redefinition of function {name_v}")));
                        },
                    FunctionBuilder { body: None, .. } => (),
                }
            }
        }

        self.reset_symbol_id(id);

        if let Some(body) = maybe_fn_body {
            self.increment_depth();
            self.functions
                .get_mut(&name_v)
                .expect("just populated")
                .body = Some(BasicBlocks::from_braced_block(body, self));
            self.decrement_depth();
        }

        let scope = self.declarations.last_mut().expect("never empty");
        #[expect(clippy::iter_over_hash_type, reason = "order doesn't matter")]
        for arg_name in names {
            if !arg_name.is_empty() {
                let ok = scope.remove(&arg_name);
                debug_assert!(ok.is_some(), "was declared in this scope");
            }
        }
        debug_assert!(
            self.declarations
                .last_mut()
                .expect("never empty")
                .is_empty(),
            "created on purpose"
        );

        self.decrement_depth();
    }

    /// Creates a new symbol for a literal value.
    pub fn push_literal(&mut self, literal: Literal) -> usize {
        if let Some(sym) = self.literals.get(&literal) {
            return sym.id;
        }
        let id = self.get_and_bump_symbol_id().as_value();
        let ty = Type::from_lit(&literal);
        self.literals.insert(literal, LiteralBuilder { id, ty });
        id
    }

    /// Resets the symbol id to the given value.
    const fn reset_symbol_id(&mut self, value: SingleUse<usize>) {
        if let Some(old) = value.try_into_value() {
            self.next_symbol_id = old;
        }
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
