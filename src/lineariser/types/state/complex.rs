use crate::errors::api::ErrorLocation;
use crate::utils::display;

/// Represent a modifier that indicates a complex type.
pub enum FoundComplex {
    /// Cf [`Modifiers::Complex`](crate::parser::api::Modifiers::Complex).
    Complex(ErrorLocation),
    /// Cf [`Modifiers::Imaginary`](crate::parser::api::Modifiers::Imaginary).
    Imaginary(ErrorLocation),
}

impl FoundComplex {
    /// Returns the location of the modifier represented.
    pub const fn loc(&self) -> ErrorLocation {
        match self {
            Self::Complex(loc) | Self::Imaginary(loc) => *loc,
        }
    }
}

display!(
    FoundComplex,
    self,
    f,
    match self {
        Self::Complex(_) => "complex",
        Self::Imaginary(_) => "imaginary",
    }
    .fmt(f)
);
