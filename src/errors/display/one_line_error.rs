use crate::errors::display::{display_prefix, display_snippet, display_squiggles};

/// Data for an error that holds in one line
#[derive(Debug)]
pub struct OneLineError<'disp> {
    /// Line of code in which the error occurred
    pub code_line: &'disp str,
    /// Column of the start of the error
    pub col1: u32,
    /// Column of the start of the error
    pub col2: u32,
    /// Column of the start of the error
    pub col3: u32,
    /// Level of the error to be displayed
    pub err_lvl: &'disp str,
    /// File of the error
    pub file_name: &'disp str,
    /// Length of the error on the line
    pub len1: u32,
    /// Length of the error on the line
    pub len2: u32,
    /// Length of the error on the line
    pub len3: u32,
    /// Line of the error
    pub line: u32,
}

impl OneLineError<'_> {
    /// Displays the error, with prefix, snippet and squiggles
    pub fn disp(&self, buf: &mut String, msg: &str) -> bool {
        display_prefix(buf, self.file_name, self.line, self.col1, msg, self.err_lvl)
            && display_snippet(buf, self.line, self.code_line)
            && display_squiggles(buf, self)
    }

    /// Displays the error, with prefix, snippet and squiggles.
    ///
    /// Will display only snippet and squibles if message is None.
    pub fn disp_opt(&self, buf: &mut String, msg_opt: Option<&str>) -> bool {
        msg_opt.is_none_or(|msg| {
            display_prefix(buf, self.file_name, self.line, self.col1, msg, self.err_lvl)
        }) && display_snippet(buf, self.line, self.code_line)
            && display_squiggles(buf, self)
    }
}
