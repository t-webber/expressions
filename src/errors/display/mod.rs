//! Module to display the errors
//!
//! Implements the [`display_errors`] function that converts the
//! [`CompileError`](crate::errors::api::CompileError) to a user-readable error
//! string.

/// Represents the string segment of a line of the input source code.
///
/// It is references by the error location and the error should be within that
/// line of code.
mod code_line;
/// Displays the error(s).
mod display_error;
/// Error that is associated to a line of code.
mod one_line_error;

use core::fmt::Write as _;

pub(super) use crate::errors::display::display_error::display_errors;
use crate::errors::display::one_line_error::OneLineError;
use crate::utils::u32_to_usize;

/// Wrapper for [`writeln!`] to customise error handling
macro_rules! writeln_bool {
    ($buf:ident $(,$arg:expr)*) => {
        writeln!($buf, $($arg,)*).is_ok()
    };
}

/// Display the prefix of the error
///
/// The prefix of a displayed error is the location, followed by the error type
/// and level, followed by the message. After the
/// prefix, the only pieces remaining are items such as
///
/// - a snippet of the line of code in which the error occurred;
/// - squiggles underneath the line of code;
/// - some information about the end of the error for errors on multiple lines.
fn display_prefix(
    buf: &mut String,
    file_name: &str,
    line: u32,
    col: u32,
    msg: &str,
    err_lvl: &str,
) -> bool {
    writeln_bool!(buf, "{file_name}:{line}:{col}: {err_lvl}: {msg}")
}

/// Display the code snippet of the error
///
/// This displays the line of code whence the error originates.
fn display_snippet(buf: &mut String, line: u32, code_line: &str) -> bool {
    writeln_bool!(buf, "{line:5} | {code_line}")
}

/// Display the squiggles under the code snippet to add visual prop
fn display_squiggles(buf: &mut String, err: &OneLineError<'_>) -> bool {
    display_squiggle_segment(buf, safe_decrement(err.col1).saturating_add(8), err.len1, 0, 0)
        && (err.col1 == err.col2
            || display_squiggle_segment(buf, err.col2, err.len2, err.col1, err.len1)
                && (err.col2 == err.col3
                    || display_squiggle_segment(buf, err.col3, err.len3, err.col2, err.len3)))
        && writeln_bool!(buf, "")
}

/// Display the squiggles under one continuous segment.
fn display_squiggle_segment(
    buf: &mut String,
    col: u32,
    len: u32,
    prev_col: u32,
    prev_len: u32,
) -> bool {
    let between = " ".repeat(u32_to_usize(col.saturating_sub(prev_col).saturating_sub(prev_len)));
    let second = "~".repeat(u32_to_usize(safe_decrement(len)));
    write!(buf, "{between}^{second}").is_ok()
}

/// Decrements a value of 1
const fn safe_decrement(val: u32) -> u32 {
    val.checked_sub(1)
        .expect("line, col, len are initialised at 1, then incremented")
}
