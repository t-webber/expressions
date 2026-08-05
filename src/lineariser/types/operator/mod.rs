/// Type after binary operator.
mod binary;
use crate::Res;
use crate::errors::api::{ErrorLocation, Located};
use crate::lineariser::types::Type;
use crate::lineariser::types::name::{TypeName, TypeToken};
use crate::parser::api::{BasicDataType, Modifiers, Qualifiers, UnaryOperator};

/// Indicator of whether a type is real, imaginary, or complex.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Domain {
    /// Has both imaginary and real parts.
    Complex,
    /// No real part.
    Imaginary,
    /// No imaginary part.
    Real,
}

impl Domain {
    /// Determines the domain of an artithmetic operation on types of the given
    /// domains.
    fn or(self, other: Self) -> Self {
        if self == other { self } else { Self::Complex }
    }

    /// Adds the necessary qualifier for the domain of the result type.
    fn wrap(self, ty: Type) -> Type {
        match self {
            Self::Imaginary => ty.add_modifier(Modifiers::Imaginary),
            Self::Complex => ty.add_modifier(Modifiers::Complex),
            Self::Real => ty,
        }
    }
}

impl From<&Type> for Domain {
    fn from(value: &Type) -> Self {
        if value.base_decorations.contains(&Modifiers::Complex.into()) {
            Self::Complex
        } else if value
            .base_decorations
            .contains(&Modifiers::Imaginary.into())
        {
            Self::Imaginary
        } else {
            Self::Real
        }
    }
}

impl Type {
    /// Adds a modifier to a type.
    fn add_modifier(mut self, modifier: Modifiers) -> Self {
        if !self.base_decorations.contains(&modifier.into()) {
            self.base_decorations.push(modifier.into());
        }
        self
    }

    /// Adds a modifier to a type.
    fn add_qualifier(mut self, qualifier: Qualifiers) -> Self {
        let last = self.indirections.last_mut().expect(">=1");
        if !last.contains(&qualifier.into()) {
            last.push(qualifier.into());
        }
        self
    }

    /// Returns the output of a type when passed to a binary operator.
    #[expect(clippy::unused_self, unused_variables, reason = "todo")]
    pub fn apply_ternary(self, success: &Self, failure: &Self) -> Res<Self> {
        // TODO: implement
        Res::ok(Self::empty())
    }

    /// Returns the output of a type when passed to a unary operator.
    pub fn apply_unary(self, op: &Located<UnaryOperator>) -> Res<Self> {
        let loc = op.as_location();
        match op.as_value() {
            UnaryOperator::AddressOf => self.indirection(true, loc),
            UnaryOperator::Indirection => self.indirection(false, loc),
            UnaryOperator::LogicalNot => Res::ok(Self::from_base(BasicDataType::Bool.into())),
            UnaryOperator::Minus => self.drop_unsigned(loc),
            UnaryOperator::Plus
            | UnaryOperator::PostfixDecrement
            | UnaryOperator::PostfixIncrement
            | UnaryOperator::PrefixDecrement
            | UnaryOperator::PrefixIncrement => Res::ok(self),
            UnaryOperator::BitwiseNot => Res::ok(self).and_then(|ty| {
                if ty.indirections.len() > 1 {
                    Res::ok(ty).add_err(
                        loc.suggest("Taking bitwise not of pointer is confusing".to_owned()),
                    )
                } else {
                    Res::ok(ty)
                }
            }),
        }
        .map(|ty| ty.add_qualifier(Qualifiers::Const))
    }

    /// Drops the unsigned modifier, if present.
    fn drop_unsigned(mut self, loc: ErrorLocation) -> Res<Self> {
        let len = self.base_decorations.len();
        self.base_decorations
            .retain(|dec| *dec != Modifiers::Unsigned.into());
        if len == self.base_decorations.len() {
            Res::ok(self)
        } else {
            Res::ok(self).add_err(loc.warn("Converts unsigned to signed which is lossy".to_owned()))
        }
    }

    /// Adds or removes an indirection.
    fn indirection(mut self, add: bool, loc: ErrorLocation) -> Res<Self> {
        if add {
            self.indirections.push(vec![]);
        } else if self.indirections.len() == 1 {
            return Res::ok(self)
                .add_err(loc.fail("Trying to dereference a non-pointer expression".to_owned()));
        } else {
            self.indirections.pop();
        }
        Res::ok(self)
    }

    /// Returns the size of the integer as the integer part, and priority as the
    /// decimal part for integer ranking.
    ///
    /// Calling this function on a non integer type is undefined behaviour.
    // TODO: long short, long char, short char
    fn integer_size(&self, base: BasicDataType) -> f32 {
        // TODO: support bigint, fits perfectly with the floats
        if self.base_decorations.contains(&Modifiers::LongLong.into()) {
            64.2
        } else if self.base_decorations.contains(&Modifiers::Long.into()) {
            64.1
        } else if self.base_decorations.contains(&Modifiers::Short.into()) {
            16.1
        } else if matches!(base, BasicDataType::Char | BasicDataType::Bool) {
            8.1
        } else {
            debug_assert_eq!(base, BasicDataType::Int, "invariant and big int todo");
            32.1
        }
    }

    /// Promote to `int` if it is an integer smaller than 32 bits.
    fn promote_integer(self) -> Self {
        if let TypeName::TypeToken(TypeToken::BasicDataType(base)) = self.base
            && base.is_integer()
            && self.integer_size(base) < 32.0
        {
            Self::from_base(TypeName::TypeToken(TypeToken::BasicDataType(BasicDataType::Int)))
        } else {
            self
        }
    }
}
