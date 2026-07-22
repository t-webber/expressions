use std::collections::HashMap;

use crate::errors::display::one_line_error::OneLineError;
use crate::errors::display::safe_decrement;
use crate::errors::error_location::FileId;

/// Precise line of code
pub struct CodeLine<'disp>(pub &'disp str, pub &'disp str);

impl<'disp> CodeLine<'disp> {
    /// Converts the code line to an error with only 1 location.
    pub const fn err1(
        &self,
        col: u32,
        len: u32,
        line: u32,
        err_lvl: &'disp str,
    ) -> OneLineError<'disp> {
        self.err2(col, len, col, len, line, err_lvl)
    }

    /// Converts the code line to an error with 2 locations.
    pub const fn err2(
        &self,
        col1: u32,
        len1: u32,
        col2: u32,
        len2: u32,
        line: u32,
        err_lvl: &'disp str,
    ) -> OneLineError<'disp> {
        OneLineError {
            col1,
            col2,
            col3: col2,
            code_line: self.1,
            err_lvl,
            file_name: self.0,
            len1,
            len2,
            len3: len2,
            line,
        }
    }

    /// Returns a precise line of code
    ///
    /// This takes as input the list of the content of all the files, the file
    /// wanted and the line number within this file and returns the line of code
    /// described by these two parameters.
    pub fn new(
        file_contents: &HashMap<u32, (&'disp str, &'disp str)>,
        file: FileId,
        line: u32,
    ) -> Self {
        let (name, content) = file_contents.get(&file.0).expect("file of error exists");
        Self(
            name,
            content
                .lines()
                .nth(usize::try_from(safe_decrement(line)).expect("never fails"))
                .expect("line of error exists"),
        )
    }
}
