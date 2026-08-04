//! Module to define the [`Keyword`] type.

use crate::utils::display;

/// Defines the keyword type and its methods
macro_rules! impl_keywords {
    ($($pascal:ident $str:expr,)* : $($uname:ident $ustr:expr,)*) => {

        /// Keywords of the language
        ///
        /// See [CppReference](https://en.cppreference.com/w/c/keyword) for the list of C keywords.
        #[derive(Debug)]
        pub enum Keyword {
            $($pascal,)*
        }

        impl Keyword {
            /// Tries to make a keyword from a literal.
            pub fn from_value_or_res(value: &str) -> TryKeyword {
                match value {
                    $($str => TryKeyword::Success(Self::$pascal),)*
                    $($ustr => TryKeyword::Deprecated(Self::$uname),)*
                    _ => TryKeyword::Failure,
                }
            }
        }

        display!(Keyword, self, f,
                match self {
                    $(Self::$pascal => $str.fmt(f),)*
                }
    );
    }

}

impl_keywords!(
    Alignas "alignas",
    Alignof "alignof",
    Auto "auto",
    Bool "bool",
    Break "break",
    Case "case",
    Char "char",
    Const "const",
    Constexpr "constexpr",
    Continue "continue",
    Default "default",
    Do "do",
    Double "double",
    Else "else",
    Enum "enum",
    Extern "extern",
    False "false",
    Float "float",
    For "for",
    Goto "goto",
    If "if",
    Inline "inline",
    Int "int",
    Long "long",
    Nullptr "nullptr",
    Register "register",
    Restrict "restrict",
    Return "return",
    Short "short",
    Signed "signed",
    Sizeof "sizeof",
    Static "static",
    StaticAssert "static_assert",
    Struct "struct",
    Switch "switch",
    ThreadLocal "thread_local",
    True "true",
    Typedef "typedef",
    Typeof "typeof",
    TypeofUnqual "typeof_unqual",
    Union "union",
    Unsigned "unsigned",
    Void "void",
    Volatile "volatile",
    While "while",
    Atomic "_Atomic",
    BigInt "_BigInt",
    Complex "_Complex",
    Decimal128 "_Decimal128",
    Decimal32 "_Decimal32",
    Decimal64 "_Decimal64",
    Generic "_Generic",
    Imaginary "_Imaginary",
    Noreturn "_Noreturn",
    :
    Alignas "_Alignas",
    Alignof "_Alignof",
    Bool "_Bool",
    StaticAssert "_Static_assert",
    ThreadLocal "_Thread_local",
);

/// Enum to store the keyword and specify if it is deprecated or not.
///
/// # Note
///
/// For the moment, deprecated means deprecated for C23.
#[derive(Debug)]
pub enum TryKeyword {
    /// Is a keyword, but deprecated for C23
    Deprecated(Keyword),
    /// Not a keyword
    Failure,
    /// Valid keyword
    Success(Keyword),
}
