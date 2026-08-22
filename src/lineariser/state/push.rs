//! Public functions to add symbols in the lineariser state.

use alloc::collections::btree_map::Entry;
use std::collections::HashSet;

use crate::BracedBlock;
use crate::errors::api::Located;
use crate::lineariser::basic_block::BasicBlocks;
use crate::lineariser::state::LState;
use crate::lineariser::symbol::{ElementBuilder, FunctionBuilder, LiteralBuilder, Symbol, Value};
use crate::lineariser::types::{ReturnType, Type};
use crate::parser::api::{Attribute, Literal};

impl LState {
    /// Creates a variable [`Symbol`].
    pub fn push_declaration(&mut self, name: Located<String>, ty: &Type, value: Value) -> usize {
        let (name_v, loc) = name.into_inner();
        if self.functions.contains_key(&name_v) {
            self.push_error(loc.fail(format!("Variable declaration shadows function {name_v}")));
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
            self.push_error(loc.fail("Non top-level functions is a GCC extension.".to_owned()));
        }
        self.increment_depth();

        if self.find_declaration(&name_v).is_some() {
            self.push_error(loc.warn(format!("Function declaration shadows variable {name_v}")));
        }

        let (symbol_args, names) = self.push_function_arguments(args);

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
                        self.push_error(loc.crash(format!(
                            "Redeclaration of function {name_v} with a different signature"
                        ))),
                    FunctionBuilder { body: Some(_), .. } =>
                        if maybe_fn_body.is_some() {
                            self.push_error(
                                loc.crash(format!("Redefinition of function {name_v}")),
                            );
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

    /// Creates variables for the arguments of the function in order to be
    /// referenced.
    fn push_function_arguments(
        &mut self,
        args: Vec<(Located<String>, Type)>,
    ) -> (Vec<(usize, Type)>, HashSet<String>) {
        let mut symbol_args = vec![];
        let mut names = HashSet::new();
        for arg in args {
            let dup = !names.insert(arg.0.as_value().to_owned());
            if !arg.0.as_value().is_empty() {
                if dup {
                    self.push_error(
                        arg.0
                            .as_location()
                            .fail("Multiple arguments have the same name".to_owned()),
                    );
                } else if self.find_declaration(arg.0.as_value()).is_some() {
                    self.push_error(
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
        (symbol_args, names)
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

    /// Creates a new type alias definition (`typedef`).
    pub fn push_typedef(&mut self, name: Located<String>, attrs: &[Located<Attribute>]) {
        let str = name.as_value();
        if self.find_declaration(str).is_some()
            || self.find_function(str).is_some()
            || self.typedefs.contains_key(str)
        {
            self.push_error(name.as_location().fail(format!(
                "Can't use '{str}' as type name as it is already declared as a variable",
            )));
            return;
        }
        let ty = self.store_errors(Type::from_attributes(attrs));
        self.typedefs.insert(name.drop_location(), ty);
    }
}
