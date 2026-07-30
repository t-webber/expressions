use crate::Res;
use crate::errors::api::{ErrorLocation, Located};
use crate::lineariser::types::Type;
use crate::lineariser::types::name::{TypeName, TypeToken};
use crate::parser::api::{BasicDataType, BinaryOperator, Modifiers, Qualifiers, UnaryOperator};

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
    pub fn apply_binary(&self, op: &Located<BinaryOperator>, other: &Self) -> Res<Self> {
        if let TypeName::TypeToken(TypeToken::BasicDataType(base_s)) = self.base
            && let TypeName::TypeToken(TypeToken::BasicDataType(base_o)) = other.base
            && *op.as_value() == BinaryOperator::Add
        {
            Self::apply_binary_add_basic_types(self, base_s, other, base_o, op.as_location())
                .map(|ty| ty.add_qualifier(Qualifiers::Const))
        } else {
            // TODO: implement this
            Res::ok(Self::empty())
        }
    }

    /// Returns the output of a type when passed to an add operation on 2 basic
    /// data types.
    fn apply_binary_add_basic_types(
        &self,
        self_base: BasicDataType,
        other: &Self,
        other_base: BasicDataType,
        loc: ErrorLocation,
    ) -> Res<Self> {
        #[allow(
            clippy::allow_attributes,
            clippy::min_ident_chars,
            clippy::useless_attribute,
            reason = "scoped shorthand"
        )]
        use BasicDataType as B;

        // pointers
        if let Some((ptr, non, non_base)) = if self.indirections.len() > 1 {
            Some((self, other, other_base))
        } else if other.indirections.len() > 1 {
            Some((other, self, self_base))
        } else {
            None
        } {
            return if non_base.is_integer()
                && Domain::from(non) == Domain::Real
                && non.indirections.len() == 1
            {
                // PERF: unnecessary clone
                Res::ok(ptr.to_owned())
            } else {
                // PERF: unnecessary clone
                Res::ok(ptr.to_owned()).add_err(
                    loc.fail("Pointer arithmetic only valid with real integers".to_owned()),
                )
            };
        }

        // void
        if self_base == B::Void || other_base == B::Void {
            return Res::ok(Self::empty())
                .add_err(loc.fail("Can't use operator on void argument".to_owned()));
        }

        // decimal
        match (self_base, other_base) {
            (B::Decimal128, non) | (non, B::Decimal128) if non.is_decimal() =>
                return Res::ok(Self::from_base(B::Decimal128.into())),
            (B::Decimal64, non) | (non, B::Decimal64) if non.is_decimal() =>
                return Res::ok(Self::from_base(B::Decimal64.into())),
            (B::Decimal32, non) | (non, B::Decimal32) if non.is_decimal() =>
                return Res::ok(Self::from_base(B::Decimal32.into())),
            _ if self_base.is_decimal() || other_base.is_decimal() =>
                return Res::ok(Self::empty())
                    .add_err(loc.fail("Mixed use of decimal and non-decimal arguments".to_owned())),
            _ => (),
        }

        // floats
        let complex = Domain::from(self).or(Domain::from(other));
        let ok = |ty| Res::ok(complex.wrap(ty));
        let okbase = |ty: BasicDataType| ok(Self::from_base(ty.into()));
        let okmod = |ty: BasicDataType, modifier: Modifiers| {
            ok(Self::from_base(ty.into()).add_modifier(modifier))
        };

        if self_base == B::Double && self.base_decorations.contains(&Modifiers::Long.into())
            || other_base == B::Double && other.base_decorations.contains(&Modifiers::Long.into())
        {
            return okmod(B::Double, Modifiers::Long);
        }
        match (self_base, other_base) {
            (B::Double, _) | (_, B::Double) => return okbase(B::Double),
            (B::Float, _) | (_, B::Float) => return okbase(B::Float),
            _ => (),
        }

        // integers

        let self_unsigned = self.base_decorations.contains(&Modifiers::Unsigned.into());
        let get_long = |ty: &Self| {
            if ty.base_decorations.contains(&Modifiers::LongLong.into()) {
                Some(Modifiers::LongLong)
            } else if ty.base_decorations.contains(&Modifiers::Long.into()) {
                Some(Modifiers::Long)
            } else {
                None
            }
        };
        let self_long = get_long(self);
        let other_unsigned = other.base_decorations.contains(&Modifiers::Unsigned.into());
        let other_long = get_long(other);

        let okint = |base: BasicDataType, unsigned: bool, modifier: Option<Modifiers>| {
            let mut ty = Self::from_base(base.into()).add_qualifier(Qualifiers::Const);
            if unsigned {
                ty = ty.add_modifier(Modifiers::Unsigned);
            }
            if let Some(some) = modifier {
                ty = ty.add_modifier(some);
            }
            ok(ty)
        };

        let self_size = self.integer_size(self_base);
        let other_size = other.integer_size(other_base);
        if self_size.max(other_size) < 32.0 {
            okbase(B::Int)
        } else if self_size > other_size {
            okint(self_base, self_unsigned, self_long)
        } else if other_size > self_size {
            okint(other_base, other_unsigned, other_long)
        } else {
            okint(self_base, self_unsigned || other_unsigned, self_long)
        }
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
}
