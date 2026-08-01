use alloc::collections::BTreeMap;
use core::fmt::Display;

use crate::Res;
use crate::errors::api::{ErrorLocation, Located};
use crate::lineariser::types::decorators::{
    FunctionAttribute, IndirectionDecorator, TypeDecorator
};
use crate::lineariser::types::name::TypeToken;
use crate::lineariser::types::state::TypeParsingState;
use crate::parser::api::{
    Attribute, AttributeKeyword, Modifiers, SpecialAttributes, UserDefinedTypes
};

impl TypeParsingState {
    /// Adds an attribute to the current type parsing state.
    #[expect(clippy::min_ident_chars, reason = "scoped shorthand")]
    pub(super) fn add_attribute(&mut self, attr: &Located<Attribute>) -> Res<()> {
        #[cfg(feature = "debug")]
        crate::lgp!("Add attr {attr} to {self:?}");
        use AttributeKeyword as K;
        use SpecialAttributes as S;
        let loc = attr.as_location();
        match attr.as_value() {
            Attribute::Indirection => self.add_indirection(loc),
            Attribute::User(name) => self.add_type(loc.wrap(TypeToken::TypeDef(name.to_owned()))),
            Attribute::Keyword(kwd) => match kwd {
                K::Modifiers(Modifiers::Long) => self.add_long(loc),
                K::Modifiers(dec) => self.add_ty_dec(loc.wrap(*dec)),
                K::BasicDataType(base) => self.add_type(loc.wrap(TypeToken::BasicDataType(*base))),
                K::Qualifiers(dec) => self.add_indirection_dec(loc.wrap(*dec)),
                K::Storage(dec) => self.add_ty_dec(loc.wrap(*dec)),
                K::UserDefinedTypes(usr_def) => self.add_usr_def(loc.wrap(*usr_def)),
                K::SpecialAttributes(special) => match special {
                    S::Atomic => self.add_ty_dec(loc.wrap(TypeDecorator::Atomic)),
                    S::Inline => self.add_fn_attr(loc.wrap(FunctionAttribute::Inline)),
                    S::Noreturn => self.add_fn_attr(loc.wrap(FunctionAttribute::NoReturn)),
                    S::Restrict =>
                        self.add_indirection_dec(loc.wrap(IndirectionDecorator::Restrict)),
                    S::Alignas | S::Generic | S::Typeof | S::TypeofUnqual => Res::ok(())
                        .add_err(loc.fail(format!("`{special}` keyword not yet supported"))),
                },
            },
        }
    }

    /// Adds a function-only attribute to the current type parsing state.
    pub(super) fn add_fn_attr(&mut self, attr: Located<FunctionAttribute>) -> Res<()> {
        already_pushed(
            attr.as_location(),
            self.fn_attr.insert(*attr.as_value(), attr.as_location()),
        )
        .add_err_cond(
            self.ind_attrs.len() > 1,
            attr.as_location()
                .fail("found function attribute after indirection".to_owned()),
        )
    }

    /// Adds an indirection
    pub(super) fn add_indirection(&mut self, loc: ErrorLocation) -> Res<()> {
        self.ind_attrs.push(BTreeMap::new());
        Res::ok(()).add_err_cond(
            self.base.is_none(),
            loc.fail("Missing type name before pointer".to_owned()),
        )
    }

    /// Adds an indirection decorator to the current type parsing state.
    pub(super) fn add_indirection_dec(
        &mut self,
        dec: Located<impl Into<IndirectionDecorator> + Display + Copy>,
    ) -> Res<()> {
        already_pushed(
            dec.as_location(),
            self.ind_attrs
                .last_mut()
                .expect("never empty")
                .insert(dec.drop_location().into(), dec.as_location()),
        )
    }

    /// Adds a base type decorator to the current type parsing state.
    pub(super) fn add_long(&mut self, loc: ErrorLocation) -> Res<()> {
        if let Some(other_longs) = self.base_attrs.get(&Modifiers::LongLong.into()) {
            Res::ok(()).add_err(
                other_longs
                    .into_two_tokens(loc)
                    .warn("Found 3 `long` modifiers, max is 2.".to_owned()),
            )
        } else if let Some(first_long) = self.base_attrs.remove(&Modifiers::Long.into()) {
            self.base_attrs
                .insert(Modifiers::LongLong.into(), loc.into_two_tokens(first_long));
            Res::ok(())
        } else {
            self.base_attrs.insert(Modifiers::Long.into(), loc);
            Res::ok(())
        }
    }

    /// Adds a base type decorator to the current type parsing state.
    pub(super) fn add_ty_dec(
        &mut self,
        dec: Located<impl Into<TypeDecorator> + Display + Copy>,
    ) -> Res<()> {
        already_pushed(
            dec.as_location(),
            self.base_attrs
                .insert(dec.drop_location().into(), dec.as_location()),
        )
        .add_err_cond(
            self.ind_attrs.len() > 1,
            dec.as_location()
                .fail("found type attribute after indirection".to_owned()),
        )
    }

    /// Adds the base type to the parsing state.
    pub(super) fn add_type(&mut self, base: Located<TypeToken>) -> Res<()> {
        if let Some(old) = &self.base {
            let loc = old.as_location().into_two_tokens(base.as_location());
            self.base = Some(base);
            Res::ok(()).add_err(loc.fail("found 2 type names".to_owned()))
        } else {
            self.base = Some(base);
            Res::ok(())
        }
    }

    /// Adds a user defined type attribute, like `struct`.
    pub(super) fn add_usr_def(&mut self, usr_def: Located<UserDefinedTypes>) -> Res<()> {
        if let Some(old) = &self.usr_def {
            Res::ok(()).add_err(
                usr_def
                    .as_location()
                    .into_two_tokens(old.as_location())
                    .fail("found 2 user defined type attributes".to_owned()),
            )
        } else {
            self.usr_def = Some(usr_def);
            Res::ok(())
        }
    }
}

/// Move
fn already_pushed(new_loc: ErrorLocation, old: Option<ErrorLocation>) -> Res<()> {
    old.map_or_else(
        || Res::ok(()),
        |old_loc| {
            Res::ok(()).add_err(
                new_loc
                    .into_two_tokens(old_loc)
                    .warn("Attribute was provided twice".to_owned()),
            )
        },
    )
}
