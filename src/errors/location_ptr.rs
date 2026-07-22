//! Pointer that is incremented and follows the lexer through the different
//! characters of the input file. It is converted to [`ErrorLocation`] when a
//! token is found.

use super::compile::{CompileError, ErrorLevel};
use crate::errors::api::ErrorLocation;
use crate::errors::error_location::{FileId, Pos, Span};
use crate::utils::usize_to_u32;

/// Structure used to lex the items and move the pointer forward.
#[derive(Debug, Clone, Copy)]
pub struct LocationPointer {
    /// Abscissa of the location
    col: u32,
    /// File of the location
    file: u32,
    /// Ordinate of the location
    line: u32,
}

impl LocationPointer {
    /// Increments column of location by 1
    ///
    /// This is used by lexer when parsing every character of the C file.
    pub(crate) fn incr_col<F: FnOnce(CompileError)>(&mut self, store: F) {
        match self.col.checked_add(1) {
            Some(col) => self.col = col,
            None => store(
                self.warn(format!("More than ({}) chars in this line, please refactor.", u32::MAX)),
            ),
        }
    }

    /// Increments line of location by 1
    ///
    /// This is used by lexer when parsing every line of the C file.
    pub(crate) fn incr_line<F: FnOnce(CompileError)>(&mut self, store: F) {
        self.col = 0;
        match self.line.checked_add(1) {
            Some(line) => self.line = line,
            None => store(
                self.warn(format!("More than ({}) lines in this file, please refactor.", u32::MAX)),
            ),
        }
    }

    /// Converts the [`LocationPointer`] to an [`ErrorLocation::Block`]
    pub(crate) const fn into_block(self, other: &Self) -> ErrorLocation {
        if self.line == other.line {
            ErrorLocation::Token(
                FileId(self.file),
                Span {
                    pos: Pos { line: self.line, col: self.col },
                    len: other.col.saturating_sub(self.col).saturating_add(1),
                },
            )
        } else {
            ErrorLocation::Block(
                FileId(self.file),
                Pos { line: self.line, col: self.col },
                Pos { line: other.line, col: other.col },
            )
        }
    }

    /// Creates a new file with the given file index.
    pub(crate) const fn start_file(file: u32) -> Self {
        Self { col: 0, file, line: 0 }
    }

    /// Moves the location back a few character on the current line
    ///
    /// If the offset is too big, the column is set to minimal (1) without any
    /// warnings or errors.
    pub(crate) fn to_past(self, len: usize, offset: usize) -> ErrorLocation {
        ErrorLocation::Token(
            FileId(self.file),
            Span {
                pos: Pos {
                    line: self.line,
                    col: self
                        .col
                        .checked_sub(usize_to_u32(offset))
                        .expect("never happens"),
                },
                len: usize_to_u32(len),
            },
        )
    }
}

impl LocationPointer {
    /// Creates a [`CompileError`] of level [`ErrorLevel::Fault`].
    pub fn fail(self, msg: String) -> CompileError {
        CompileError::from((
            ErrorLocation::new_char(self.file, self.line, self.col),
            msg,
            ErrorLevel::Fault,
        ))
    }

    /// Creates a [`CompileError`] of level [`ErrorLevel::Suggestion`].
    pub fn suggest(self, msg: String) -> CompileError {
        CompileError::from((
            ErrorLocation::new_char(self.file, self.line, self.col),
            msg,
            ErrorLevel::Suggestion,
        ))
    }

    /// Creates a [`CompileError`] of level [`ErrorLevel::Warning`].
    pub fn warn(self, msg: String) -> CompileError {
        CompileError::from((
            ErrorLocation::new_char(self.file, self.line, self.col),
            msg,
            ErrorLevel::Warning,
        ))
    }
}

#[cfg(test)]
mod test {
    use crate::errors::api::LocationPointer;

    #[test]
    fn too_many_cols() {
        let mut location = LocationPointer { col: u32::MAX, file: 0, line: 0 };
        let mut has_err = false;
        location.incr_col(|err| {
            let got = err.as_values().1;
            has_err =
                got == format!("More than ({}) chars in this line, please refactor.", u32::MAX);
        });
        assert!(has_err);
    }

    #[test]
    fn too_many_lines() {
        let mut location = LocationPointer { col: 0, file: 0, line: u32::MAX };
        let mut has_err = false;
        location.incr_line(|err| {
            let got = err.as_values().1;
            has_err =
                got == format!("More than ({}) lines in this file, please refactor.", u32::MAX);
        });
        assert!(has_err);
    }
}
