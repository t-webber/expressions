/// Add some new item to the type parsing state.
mod add;
/// Helper to handle complex modifiers.
mod complex;

extern crate alloc;
use alloc::collections::BTreeMap;
use core::mem::take;

use complex::FoundComplex;

use crate::Res;
use crate::errors::api::{ErrorLocation, Located};
use crate::lineariser::types::decorators::{
    FunctionAttribute, IndirectionDecorator, TypeDecorator
};
use crate::lineariser::types::name::{TypeName, TypeToken};
use crate::lineariser::types::{ReturnType, Type};
use crate::parser::api::{Attribute, BasicDataType, Modifiers, UserDefinedTypes};
use crate::utils::{display, repr_vec};

/// Automaton to parse the list of attributes into a structure type.
#[derive(Debug)]
pub struct TypeParsingState {
    /// Base type, like `int` or the `name` in `struct name`
    base: Option<Located<TypeToken>>,
    /// Attributes that only apply on the base type, not on a pointer, like
    /// `short` or `signed`.
    base_attrs: BTreeMap<TypeDecorator, ErrorLocation>,
    /// Attributes that only apply on function  declarations, like `inline` and
    /// `_NoReturn`.
    fn_attr: BTreeMap<FunctionAttribute, ErrorLocation>,
    /// Attributes that can be applied on a pointers, like
    /// `const` or `volatile`.
    ind_attrs: Vec<BTreeMap<IndirectionDecorator, ErrorLocation>>,
    /// Keywords for user defined types, like `struct` or `enum`
    usr_def: Option<Located<UserDefinedTypes>>,
}

display!(TypeParsingState, self, f, {
    let Self { base, base_attrs, usr_def, fn_attr, ind_attrs } = self;
    repr_vec(fn_attr.keys(), " ").fmt(f)?;
    repr_vec(base_attrs.keys(), " ").fmt(f)?;
    if let Some(usr_def_ty) = usr_def {
        usr_def_ty.fmt(f)?;
    }
    repr_vec(base, " ").fmt(f)?;
    ind_attrs
        .iter()
        .map(|ind| repr_vec(ind.keys(), " "))
        .collect::<Vec<_>>()
        .join(" * ")
        .fmt(f)
});

impl Default for TypeParsingState {
    fn default() -> Self {
        Self {
            base: None,
            base_attrs: BTreeMap::new(),
            usr_def: None,
            fn_attr: BTreeMap::new(),
            ind_attrs: vec![BTreeMap::new()],
        }
    }
}

impl TypeParsingState {
    /// Returns a complex number attribute if any.
    fn find_complex(&self) -> Option<FoundComplex> {
        self.base_attrs.get(&Modifiers::Complex.into()).map_or_else(
            || {
                self.base_attrs
                    .get(&Modifiers::Imaginary.into())
                    .map(|imm| FoundComplex::Imaginary(*imm))
            },
            |complex| Some(FoundComplex::Complex(*complex)),
        )
    }

    /// Returns the return type represented by the current parsing state.
    pub fn into_return_type(mut self, loc: ErrorLocation) -> Res<ReturnType> {
        self.take_name(loc).map(|base| ReturnType {
            attrs: self.fn_attr.into_keys().collect(),
            ty: Type {
                base,
                base_decorations: self.base_attrs.into_keys().collect(),
                indirections: self
                    .ind_attrs
                    .into_iter()
                    .map(|dec| dec.into_keys().collect())
                    .collect(),
            },
        })
    }

    /// Returns the type represented by the current parsing state.
    pub fn into_type(mut self, loc: ErrorLocation) -> Res<Type> {
        self.take_name(loc)
            .map(|base| Type {
                base,
                base_decorations: self.base_attrs.into_keys().collect(),
                indirections: self
                    .ind_attrs
                    .into_iter()
                    .map(|dec| dec.into_keys().collect())
                    .collect(),
            })
            .add_errs(
                self.fn_attr
                    .values()
                    .map(|fn_loc| {
                        fn_loc.fail("Found function-only attribute in an object type".to_owned())
                    })
                    .collect(),
            )
    }

    /// Builds a [`TypeParsingState`] for a list of attributes.
    pub fn parse_from(attrs: &[Located<Attribute>]) -> Res<Self> {
        let mut state = Self::default();
        let mut errors = vec![];
        for attr in attrs {
            state
                .add_attribute(attr)
                .store_errors(&mut |err| errors.push(err));
        }
        Res::from((state, errors))
    }

    /// Takes the name and returns it.
    fn take_name(&mut self, full_ty_loc: ErrorLocation) -> Res<TypeName> {
        take(&mut self.base).map_or_else(
            || {
                Res::ok(TypeName::from(BasicDataType::Int))
                    .add_err(full_ty_loc.fail("Missing variable name or type name".to_owned()))
            },
            |base| {
                Res::ok(base)
                    .and_then(|name| {
                        let err = match (self.find_complex(), name.as_value()) {
                            (Some(complex), TypeToken::BasicDataType(basic))
                                if basic.is_decimal() =>
                                Some(
                                    complex
                                        .loc()
                                        .into_two_tokens(name.as_location())
                                        .fail("Decimal can't be complex, only real".to_owned()),
                                ),
                            (None, _) | (Some(_), TypeToken::BasicDataType(_)) => None,
                            (Some(complex), _) if self.usr_def.is_some() => Some(
                                complex
                                    .loc()
                                    .into_two_tokens(name.as_location())
                                    .fail("User-defined type can't be complex".to_owned()),
                            ),
                            (Some(_), TypeToken::TypeDef(_)) => todo!(),
                        };
                        Res::ok(name).add_err_opt(err)
                    })
                    .and_then(|name| {
                        let name_loc = name.as_location();
                        name.drop_location().with(take(&mut self.usr_def), name_loc)
                    })
            },
        )
    }
}
