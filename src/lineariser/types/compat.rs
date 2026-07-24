use crate::Res;
use crate::errors::api::{ErrorLocation, Located};
use crate::lineariser::types::Type;
use crate::parser::api::{BasicDataType, Modifiers, Qualifiers, UnaryOperator};

impl Type {
    /// Returns the output of a type when passed to a unary operator.
    pub fn apply_unary(self, op: &Located<UnaryOperator>) -> Res<Self> {
        let loc = op.as_location();
        match op.as_value() {
            UnaryOperator::AddressOf => self.drop_const().indirection(true, loc),
            UnaryOperator::Indirection => self.drop_const().indirection(false, loc),
            UnaryOperator::LogicalNot => Res::ok(Self::from_base(BasicDataType::Bool.into())),
            UnaryOperator::Minus => self.drop_const().drop_unsigned(loc),
            UnaryOperator::Plus
            | UnaryOperator::PostfixDecrement
            | UnaryOperator::PostfixIncrement
            | UnaryOperator::PrefixDecrement
            | UnaryOperator::PrefixIncrement => Res::ok(self.drop_const()),
            UnaryOperator::BitwiseNot => Res::ok(self.drop_const()).and_then(|ty| {
                if ty.indirections.len() > 1 {
                    Res::ok(ty).add_err(
                        loc.suggest("Taking bitwise not of pointer is confusing".to_owned()),
                    )
                } else {
                    Res::ok(ty)
                }
            }),
        }
    }

    /// Drops the const qualifier, if present.
    fn drop_const(mut self) -> Self {
        self.indirections
            .last_mut()
            .expect(">=1")
            .retain(|dec| *dec != Qualifiers::Const.into());
        self
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
}
