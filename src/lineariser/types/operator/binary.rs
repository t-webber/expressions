use crate::Res;
use crate::errors::api::{ErrorLocation, Located};
use crate::lineariser::types::Type;
use crate::lineariser::types::name::{TypeName, TypeToken};
use crate::lineariser::types::operator::Domain;
use crate::parser::api::{BasicDataType, BinaryOperator, Modifiers, Qualifiers};

impl Type {
    /// Returns the output of a type when passed to a binary operator.
    pub fn apply_binary(&self, op: &Located<BinaryOperator>, other: &Self) -> Res<Self> {
        use BinaryOperator as Bo;
        match op.as_value() {
            Bo::ArraySubscript => todo!(),
            Bo::Add
            | Bo::Subtract
            | Bo::Multiply
            | Bo::Divide
            | Bo::Modulo
            | Bo::BitwiseAnd
            | Bo::BitwiseXor
            | Bo::BitwiseOr =>
                if let TypeName::TypeToken(TypeToken::BasicDataType(base_s)) = self.base
                    && let TypeName::TypeToken(TypeToken::BasicDataType(base_o)) = other.base
                {
                    Self::apply_binary_add_basic_types(
                        self,
                        base_s,
                        other,
                        base_o,
                        op.as_location(),
                    )
                } else {
                    todo!()
                },
            Bo::StructEnumMemberAccess | Bo::StructEnumMemberPointerAccess | Bo::Comma =>
                Res::ok(other.clone()),
            Bo::ShiftRight | Bo::ShiftLeft => Res::ok(self.clone().promote_integer()),
            Bo::Lt
            | Bo::Le
            | Bo::Gt
            | Bo::Ge
            | Bo::Equal
            | Bo::Different
            | Bo::LogicalAnd
            | Bo::LogicalOr => Res::ok(Self::from_base(TypeName::TypeToken(
                TypeToken::BasicDataType(BasicDataType::Bool),
            ))),
            Bo::AddAssign
            | Bo::SubAssign
            | Bo::MulAssign
            | Bo::DivAssign
            | Bo::AndAssign
            | Bo::XorAssign
            | Bo::OrAssign
            | Bo::ModAssign
            | Bo::Assign
            | Bo::ShiftLeftAssign
            | Bo::ShiftRightAssign => Res::ok(self.clone()),
        }
        .map(|ty| ty.add_qualifier(Qualifiers::Const))
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
}
