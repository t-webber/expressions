//! A couple of utility functions to ease the code

#![expect(clippy::arbitrary_source_item_ordering, reason = "macro usage")]

/// Macro to derive `fmt::Display` without needing to write all the boiler plate
///
/// # Example
///
/// ```ignore
/// #![feature(coverage_attribute)]
/// enum Token {
///     Symbol(char),
///     String(String),
/// }
///
/// c_parser::display!(
///     Token,
///     self,
///     f,
///     match self {
///         Self::Symbol(ch) => ch.fmt(f),
///         Self::String(value) => write!(f, "\"{value}\""),
///     }
/// );
/// ```
macro_rules! display {
    ($t:ty, $self:ident, $f:ident, $code:expr) => {
        #[coverage(off)]
        impl core::fmt::Display for $t {
            fn fmt(&$self, $f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                $code
            }
        }
    };
}

/// Helper to implement the [`Ord`] and [`PartialOrd`] traits
macro_rules! ord {
    ($ty:ty, $self:ident, $other:ident, $expr:expr) => {
        impl Ord for $ty {
            fn cmp(&$self, $other: &Self) -> Ordering {
                $expr
            }
        }
        impl PartialOrd for $ty {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
    };
}

/// Implements the `From` trait for a variant of an enum whose name is the name
/// of the underlying type.
macro_rules! from {
    ($from:ident $to:ty) => {
        impl From<$from> for $to {
            fn from(value: $from) -> Self {
                Self::$from(value)
            }
        }
    };
}

/// Equivalent of vec!, but for hashset.
macro_rules! bset {
    () => {
        #[expect(clippy::std_instead_of_alloc, reason="convenience for macro usage")]
        {
            std::collections::BTreeSet::new()
        }
    };
    ($($set:expr),*) => {
        {let mut set = bset!(); $(set.insert($set);)* set}
    };
}

use core::fmt;

use crate::EMPTY;

/// Displays the fullness, with `..` if the content is still pushable
pub const fn repr_fullness(full: bool) -> &'static str {
    if full { "" } else { ".." }
}

/// Displays an option with the [`EMPTY`] string.
#[expect(
    clippy::ref_option,
    reason = "convenient usage (always used on &Option and removes need for .as_ref)"
)]
pub fn repr_option<T: fmt::Display>(opt: &Option<T>) -> String {
    opt.as_ref().map_or_else(|| EMPTY.to_owned(), T::to_string)
}
/// Displays an option with the [`EMPTY`] string.
pub fn repr_option_vec<T: fmt::Display>(vec: &[Option<T>], sep: &str) -> String {
    vec.iter().map(repr_option).collect::<Vec<_>>().join(sep)
}

/// Displays a vector with the [`EMPTY`] string.
pub fn repr_vec<'item, T: fmt::Display + 'item, I: IntoIterator<Item = &'item T>>(
    vec: I,
    sep: &str,
) -> String {
    vec.into_iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(sep)
}

/// Struct to track if object are used or not.
pub struct SingleUse<T> {
    /// Whether the value is used or not.
    used: bool,
    /// Value held.
    value: T,
}

impl<T: Copy> SingleUse<T> {
    /// Returns the value, marking it as used.
    pub const fn as_value(&mut self) -> T {
        self.used = true;
        self.value
    }
    /// Creates a new [`SingleUse`].
    pub const fn from(value: T) -> Self {
        Self { value, used: false }
    }
    /// Returns the value only if unused.
    pub const fn try_into_value(self) -> Option<T> {
        if self.used { None } else { Some(self.value) }
    }
}

/// Safely converts u32 to usize.
#[expect(clippy::as_conversions, reason = "usize is 32 or 64")]
pub const fn u32_to_usize(val: u32) -> usize {
    val as usize
}

/// Safely converts usize to u32.
///
/// # Panics
///
/// on overflow.
pub fn usize_to_u32(val: usize) -> u32 {
    u32::try_from(val).expect("File too big, please refactor or split in multiple files.")
}

pub(crate) use bset;
pub(crate) use display;
pub(crate) use from;
pub(crate) use ord;
