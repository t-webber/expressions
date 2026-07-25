use core::fmt::Write as _;
use std::collections::HashMap;

use crate::errors::api::{CompileError, ErrorLocation};
use crate::errors::compile::CompileErrorList;
use crate::errors::display::code_line::CodeLine;
use crate::errors::display::one_line_error::OneLineError;
use crate::errors::display::{display_prefix, safe_decrement};
use crate::errors::error_location::{Pos, Span};
use crate::utils::{u32_to_usize, usize_to_u32};

/// Display one error
///
/// This is wrapper for [`display_error`]. Please refer to its documentation.
fn display_error(
    buf: &mut String,
    error: &CompileError,
    file_contents: &HashMap<u32, (&str, &str)>,
) -> bool {
    let (location, msg, err_lvl) = error.as_values();
    match location {
        ErrorLocation::Token(file, span) => CodeLine::new(file_contents, file, span.pos.line)
            .err1(span.pos.col, span.len, span.pos.line, &err_lvl)
            .disp(buf, msg),
        ErrorLocation::Block(file, start, end) => {
            let start_code_line = CodeLine::new(file_contents, file, start.line);
            writeln!(buf).is_ok()
                && display_prefix(buf, start_code_line.0, start.line, start.col, msg, &err_lvl)
                && start_code_line
                    .err1(
                        start.col,
                        usize_to_u32(
                            start_code_line
                                .1
                                .len()
                                .checked_sub(u32_to_usize(safe_decrement(start.col)))
                                .expect("col <= len"),
                        ),
                        start.line,
                        &err_lvl,
                    )
                    .disp(buf, "Multi-line error occurred. Starts here...")
                && CodeLine::new(file_contents, file, end.line)
                    .err1(end.col, 1, end.line, &err_lvl)
                    .disp(buf, "...and ends here.")
                && writeln!(buf).is_ok()
        }
        ErrorLocation::None => unreachable!("never built"),
        ErrorLocation::TwoTokens(
            file,
            Span { pos: Pos { line: line1, col: col1 }, len: len1 },
            Span { pos: Pos { line: line2, col: col2 }, len: len2 },
        ) =>
            if line1 == line2 {
                CodeLine::new(file_contents, file, line1)
                    .err2(col1, len1, col2, len2, line1, &err_lvl)
                    .disp(buf, msg)
            } else {
                CodeLine::new(file_contents, file, line1)
                    .err1(col1, len1, line1, &err_lvl)
                    .disp(buf, msg)
                    && CodeLine::new(file_contents, file, line2)
                        .err1(col2, len2, line2, &err_lvl)
                        .disp_opt(buf, None)
            },
        ErrorLocation::ThreeTokens(
            file,
            Span { pos: Pos { line: line1, col: col1 }, len: len1 },
            Span { pos: Pos { line: line2, col: col2 }, len: len2 },
            Span { pos: Pos { line: line3, col: col3 }, len: len3 },
        ) =>
            if line1 == line2 && line2 == line3 {
                {
                    let this = &CodeLine::new(file_contents, file, line1);
                    OneLineError {
                        col1,
                        col2,
                        col3,
                        code_line: this.1,
                        err_lvl: &err_lvl,
                        file_name: this.0,
                        len1,
                        len2,
                        len3,
                        line: line1,
                    }
                }
                .disp(buf, msg)
            } else if line1 == line2 {
                CodeLine::new(file_contents, file, line1)
                    .err2(col1, len1, col2, len2, line1, &err_lvl)
                    .disp(buf, msg)
                    && CodeLine::new(file_contents, file, line3)
                        .err1(col3, len3, line3, &err_lvl)
                        .disp_opt(buf, None)
            } else if line2 == line3 {
                CodeLine::new(file_contents, file, line1)
                    .err1(col1, len1, line1, &err_lvl)
                    .disp(buf, msg)
                    && CodeLine::new(file_contents, file, line2)
                        .err2(col2, len2, col3, len3, line2, &err_lvl)
                        .disp_opt(buf, None)
            } else {
                CodeLine::new(file_contents, file, line1)
                    .err1(col1, len1, line1, &err_lvl)
                    .disp(buf, msg)
                    && CodeLine::new(file_contents, file, line2)
                        .err1(col2, len2, line2, &err_lvl)
                        .disp_opt(buf, None)
                    && CodeLine::new(file_contents, file, line3)
                        .err1(col3, len3, line3, &err_lvl)
                        .disp_opt(buf, None)
            },
    }
}

/// Transforms [`CompileError`] into a human-readable string
///
/// See [`Res::as_displayed_errors`](crate::errors::result::Res::as_displayed_errors)
/// for extra information and examples.
///
/// # Errors
///
/// Returns an error when the writing on the string buffer fails.
#[coverage(off)]
pub fn display_errors(
    errors: &CompileErrorList,
    files: &[(u32, &str, &str)],
) -> Result<String, ()> {
    let mut file_contents: HashMap<u32, (&str, &str)> = HashMap::new();
    let mut buf = String::new();
    for (id, filename, content) in files {
        file_contents.insert(*id, (filename, content));
    }
    for error in &errors.0 {
        if !display_error(&mut buf, error, &file_contents) {
            return Err(());
        }
    }
    Ok(buf)
}
