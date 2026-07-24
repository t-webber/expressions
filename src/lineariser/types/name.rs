use crate::Res;
use crate::errors::api::{ErrorLocation, Located};
use crate::parser::api::{BasicDataType, UserDefinedTypes};
use crate::utils::{display, from};

/// The type name is in only 1 token (a basic data type or a typedefed name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeToken {
    /// The type is a builtin, like `int` or `char`.
    BasicDataType(BasicDataType),
    /// The type is user-defined with a typedef.
    TypeDef(String),
}

impl TypeToken {
    /// Adds a user defined type attribute to the type name.
    pub fn with(
        self,
        usr_def_attr: Option<Located<UserDefinedTypes>>,
        name_loc: ErrorLocation,
    ) -> Res<TypeName> {
        let Some(usr_def) = usr_def_attr else {
            return Res::ok(TypeName::TypeToken(self));
        };
        match self {
            Self::TypeDef(name) => Res::ok(match usr_def.as_value() {
                UserDefinedTypes::Struct => TypeName::Struct(name),
                UserDefinedTypes::Union => TypeName::Union(name),
                UserDefinedTypes::Enum => TypeName::Enum(name),
            }),
            Self::BasicDataType(ty) => Res::ok(TypeName::TypeToken(self)).add_err(
                usr_def
                    .as_location()
                    .into_two_tokens(name_loc)
                    .fail(format!("Can't apply `{}` to builtin type `{ty}`", usr_def.as_value())),
            ),
        }
    }
}

display!(
    TypeToken,
    self,
    f,
    match self {
        Self::BasicDataType(ty) => ty.fmt(f),
        Self::TypeDef(ty) => ty.fmt(f),
    }
);

from!(BasicDataType TypeToken);

/// Actual name of the type segment, stripped of modifiers, qualifiers and what
/// not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeName {
    /// The type is user-defined with a enum.
    Enum(String),
    /// The type is user-defined with a struct.
    Struct(String),
    /// The type name is in only 1 token (a basic data type or a typedefed
    /// name).
    TypeToken(TypeToken),
    /// The type is user-defined with a union.
    Union(String),
}

display!(
    TypeName,
    self,
    f,
    match self {
        Self::TypeToken(ty) => ty.fmt(f),
        Self::Struct(name) => write!(f, "struct {name}"),
        Self::Union(name) => write!(f, "union {name}"),
        Self::Enum(name) => write!(f, "enum {name}"),
    }
);

from!(TypeToken TypeName);
from!(BasicDataType TypeToken TypeName);
