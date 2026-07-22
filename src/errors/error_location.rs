//! Module to store the location and length of the error.
//!
//! This crate implements the [`ErrorLocation`] struct and its methods.

use core::cmp::Ordering;
use core::mem::take;

use super::compile::{CompileError, ErrorLevel};
use crate::errors::api::Located;
use crate::utils::ord;

/// Position in the source file to point to a specific location in the code when
/// displaying an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pos {
    /// Column of the source file (starting at 0)
    pub col: u32,
    /// Line of the source file (starting at 0)
    pub line: u32,
}

ord!(
    Pos,
    self,
    other,
    match self.line.cmp(&other.line) {
        Ordering::Equal => self.col.cmp(&other.col),
        ord @ (Ordering::Less | Ordering::Greater) => ord,
    }
);

/// Position in the source file to point to a specific part of the code when
/// displaying an error.
///
/// A span is a continuous succession of characters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    /// Length of the handled token (starts at pos and last len chars)
    pub len: u32,
    /// Cf. [`Pos`]
    pub pos: Pos,
}

ord!(Span, self, other, self.pos.cmp(&other.pos));

/// Wrapper around the file number, for type safety.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileId(pub u32);

/// Type to pinpoint a precise character in the C source file.
///
/// The locations are computed by the lexer that reads the C source file. Then,
/// the locations are stored inside the tokens to keep them for the rest of the
/// compiler.
///
/// # Note
///
/// In order to respect the click links from terminals, the line and column of
/// a file start at 1 and not 0.
#[derive(Clone, Default, Copy, Debug)]
pub enum ErrorLocation {
    /// Location a block of the source file
    ///
    /// # Fields
    ///
    /// file name, start line, start column, end line, end column
    Block(FileId, Pos, Pos),
    /// Never built, useful for taking
    #[default]
    None,
    /// Put squiggles for 3 tokens, but not between.
    ThreeTokens(FileId, Span, Span, Span),
    /// Location a token of the source file
    Token(FileId, Span),
    /// Put squiggles for 2 tokens, but not between.
    TwoTokens(FileId, Span, Span),
}

impl ErrorLocation {
    /// Returns the filename of the current [`ErrorLocation`]
    fn as_filename(self) -> FileId {
        match self {
            Self::None => unreachable!("never built"),
            Self::Block(file, ..)
            | Self::TwoTokens(file, ..)
            | Self::ThreeTokens(file, ..)
            | Self::Token(file, ..) => file,
        }
    }

    /// Returns the start and end of the [`ErrorLocation`]
    ///
    /// # Panics
    ///
    /// If called on an error location that cannot be extended.
    #[expect(clippy::arithmetic_side_effects, reason = "in range of tokens")]
    #[expect(clippy::panic, reason = "todo")]
    fn as_pos(self) -> (Pos, Pos) {
        match self {
            Self::Block(_, start, end) => (start, end),
            Self::Token(_, Span { pos: Pos { line, col }, len }) =>
                (Pos { col, line }, Pos { line, col: col + len }),
            Self::TwoTokens(..) | Self::ThreeTokens(..) => panic!("can not be extended"),
            Self::None => unreachable!("never built"),
        }
    }

    /// Extends a current [`ErrorLocation`] by changing the end of the location.
    pub fn extend(&mut self, other: Self) {
        *self = take(self).into_extended(other);
    }

    /// Returns a [`ErrorLocation`] that is the combination of the span covered
    /// by the 2 inputs.
    ///
    /// # Panics
    ///
    /// If the second provided [`ErrorLocation`] is before or overlaps the
    /// first.
    pub fn into_extended(self, other: Self) -> Self {
        debug_assert_eq!(
            self.as_filename(),
            other.as_filename(),
            "can't merge 2 locations from different files"
        );
        let first = self.as_pos();
        let second = other.as_pos();
        let (min, max) = (first.min(second), first.max(second));
        let file = self.as_filename();
        if min.0.line == max.1.line {
            Self::Token(file, Span { pos: min.0, len: max.1.col.saturating_sub(min.0.col) })
        } else {
            Self::Block(file, min.0, max.1)
        }
    }

    /// Makes an error location out of 2 tokens.
    ///
    /// # Panics
    ///
    /// If one of the given error locations isn't a token.
    #[expect(clippy::panic, reason = "todo")]
    pub fn into_two_tokens(self, other: Self) -> Self {
        match (self, other) {
            (Self::Token(file1, span1), Self::Token(file2, span2)) if file1 == file2 =>
                if span1 <= span2 {
                    Self::TwoTokens(file1, span1, span2)
                } else {
                    Self::TwoTokens(file1, span2, span1)
                },
            (Self::TwoTokens(file1, span1, span2), Self::Token(file3, span3))
            | (Self::Token(file3, span3), Self::TwoTokens(file1, span1, span2))
                if file1 == file3 =>
            {
                let mut arr = [span1, span2, span3];
                arr.sort();
                Self::ThreeTokens(file1, arr[0], arr[1], arr[2])
            }
            _ => panic!("invariant"),
        }
    }

    /// Creates a new [`ErrorLocation`] of type char at the given position
    pub const fn new_char(file: u32, line: u32, col: u32) -> Self {
        Self::Token(FileId(file), Span { pos: Pos { col, line }, len: 1 })
    }

    /// Adds a value to the error location to make a [`Located`].
    pub fn wrap<T>(self, value: T) -> Located<T> {
        Located::from((value, self))
    }
}

impl ErrorLocation {
    /// Creates a [`CompileError`] of level [`ErrorLevel::Crash`].
    pub fn crash(self, msg: String) -> CompileError {
        CompileError::from((self, msg, ErrorLevel::Crash))
    }

    /// Creates a [`CompileError`] of level [`ErrorLevel::Fault`].
    pub fn fail(self, msg: String) -> CompileError {
        CompileError::from((self, msg, ErrorLevel::Fault))
    }

    /// Creates a [`CompileError`] of level [`ErrorLevel::Suggestion`].
    pub fn suggest(self, msg: String) -> CompileError {
        CompileError::from((self, msg, ErrorLevel::Suggestion))
    }

    /// Creates a [`CompileError`] of level [`ErrorLevel::Warning`].
    pub fn warn(self, msg: String) -> CompileError {
        CompileError::from((self, msg, ErrorLevel::Warning))
    }
}
