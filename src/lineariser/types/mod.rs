//! Module to define and handle type coherence and storing.

/// Type decorators, like `const`, `short` or `thread_local`.
mod decorators;
/// Base of the type, like `struct A`, `custom` or `int`.
mod name;
/// Computes the new type after an operation.
mod operator;
/// Parsing state to read one by one the attributes and build a [`Type`] or
/// [`ReturnType`].
mod state;

use crate::errors::api::Located;
use crate::lineariser::types::decorators::{
    FunctionAttribute, IndirectionDecorator, TypeDecorator
};
use crate::lineariser::types::name::{TypeName, TypeToken};
use crate::lineariser::types::state::TypeParsingState;
use crate::parser::api::{Attribute, Literal, Modifiers, Qualifiers};
use crate::utils::{display, repr_vec};
use crate::{EMPTY, Number, Res};

/// Helper macro to create a type attribute.
macro_rules! lity {
    ($base:ident $($base_decorations:ident)*) => {
        Self {
            base: TypeName::from($crate::parser::api::BasicDataType::$base),
            base_decorations: vec![$(Modifiers::$base_decorations.into()),*],
            indirections: vec![vec![Qualifiers::Const.into()]],
        }
    };
    (star: $base:ident) => {
        Self {
            base: TypeName::from($crate::parser::api::BasicDataType::$base),
            base_decorations: vec![],
            indirections: vec![vec![Qualifiers::Const.into()], vec![Qualifiers::Const.into()]],
        }
    }
}

/// Return type of a function.
///
/// It adds attributes on top of [`Type`] for attributes like `inline` and
/// `noreturn`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnType {
    /// Function-specific attributes.
    attrs: Vec<FunctionAttribute>,
    /// Underlying type.
    ty: Type,
}

display!(
    ReturnType,
    self,
    f,
    write!(
        f,
        "{}{}{}",
        repr_vec(&self.attrs, " "),
        if self.attrs.is_empty() { "" } else { " " },
        self.ty
    )
);

impl ReturnType {
    /// Returns a place holder return type for function defines but wrongly.
    pub fn empty() -> Self {
        Self { ty: Type::empty(), attrs: vec![] }
    }

    /// Builds a [`ReturnType`] for a list of attributes.
    pub fn from_attributes(attrs: &[Located<Attribute>]) -> Res<Self> {
        TypeParsingState::parse_from(attrs).and_then(|state| {
            state.into_return_type(
                attrs
                    .first()
                    .expect("invariant")
                    .as_location()
                    .into_extended(attrs.last().expect("invariant").as_location()),
            )
        })
    }

    /// Returns the type of the variable returned by such a function.
    pub fn into_type(self) -> Type {
        self.ty
    }
}

/// Representation of a type.
///
/// It separates decorations from the actual type name.
///
/// - The "type name" is the 1 word undecorated attribute of the type obtained
///   after removing all indirections.
/// - An "indirection" to a type is simply the type of a pointer to the former
///   type.
/// - A "decorator" is any kind of qualifier, modifier, or additional
///   information on the type, such as `short`, `const`, `volatile` or `auto`.
///
/// Some decorators can only be applied on the base type name, whereas others
/// can be applied to each level of indirection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Type {
    /// Base type of the data, once all pointers are accessed.
    base: TypeName,
    /// Decorations on the base type name that can't be applied to indirections.
    ///
    /// # Examples
    ///
    /// `short`, `static`, etc.
    base_decorations: Vec<TypeDecorator>,
    /// Decorations on the base type name.
    ///
    /// The first element also applied to the base type name. Thus, the number
    /// of indirection levels is the length of the vector plus one.
    indirections: Vec<Vec<IndirectionDecorator>>,
}

impl Type {
    /// Returns a place holder return type for function defines but wrongly.
    pub fn empty() -> Self {
        Self {
            base: TypeName::TypeToken(TypeToken::TypeDef(String::new())),
            base_decorations: vec![],
            indirections: vec![vec![]],
        }
    }

    /// Builds a [`Type`] for a list of attributes.
    pub fn from_attributes(attrs: &[Located<Attribute>]) -> Res<Self> {
        TypeParsingState::parse_from(attrs).and_then(|state| {
            state.into_type(
                attrs
                    .first()
                    .expect("invariant")
                    .as_location()
                    .into_extended(attrs.last().expect("invariant").as_location()),
            )
        })
    }

    /// Creates a type from the given base.
    fn from_base(base: TypeName) -> Self {
        Self { base, base_decorations: vec![], indirections: vec![vec![]] }
    }

    /// Builds and returns the type of a literal.
    pub fn from_lit(lit: &Literal) -> Self {
        match lit {
            Literal::Char(_) => lity!(Char),
            Literal::ConstantBool(_) => lity!(Bool),
            Literal::Null => lity!(star: Void),
            Literal::Str(_) => lity!(star: Char),
            Literal::Number(Number::Int(_)) => lity!(Int),
            Literal::Number(Number::Long(_)) => lity!(Int Long),
            Literal::Number(Number::LongLong(_)) => lity!(Int LongLong),
            Literal::Number(Number::Float(_)) => lity!(Float),
            Literal::Number(Number::Double(_)) => lity!(Double),
            Literal::Number(Number::LongDouble(_)) => lity!(Double Long),
            Literal::Number(Number::UInt(_)) => lity!(Int Unsigned),
            Literal::Number(Number::ULong(_)) => lity!(Int Unsigned Long),
            Literal::Number(Number::ULongLong(_)) => lity!(Int Unsigned LongLong),
        }
    }
}

display!(Type, self, f, {
    if *self == Self::empty() {
        return EMPTY.fmt(f);
    }
    let first = self.indirections.first().expect("always has a first");
    let mut prev = if first.is_empty() {
        false
    } else {
        repr_vec(first, " ").fmt(f)?;
        true
    };
    prev = if self.base_decorations.is_empty() {
        prev
    } else {
        if prev {
            " ".fmt(f)?;
        }
        repr_vec(&self.base_decorations, " ").fmt(f)?;
        true
    };
    if prev {
        " ".fmt(f)?;
    }
    self.base.fmt(f)?;
    for ind in self.indirections.iter().skip(1) {
        " *".fmt(f)?;
        if !ind.is_empty() {
            " ".fmt(f)?;
            repr_vec(ind, " ").fmt(f)?;
        }
    }
    Ok(())
});
