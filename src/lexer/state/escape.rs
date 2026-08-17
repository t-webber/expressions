//! Module to define the state and handlers for escaped characters and
//! sequences.

use crate::errors::api::LocationPointer;
use crate::lexer::types::api::{EscapeSequence, LexingData};

/// Used to store the current escape state and the escape sequence values if
/// needed.
#[derive(Debug)]
pub enum EscapeState {
    /// Reading an escape sequence.
    Sequence(EscapeSequence),
    /// Escape opened and found 1 character after it.
    Single,
}

impl EscapeState {
    /// Parses the token following the escape character. It determines whether
    /// it is a escape sequence (in which case waiting for the next
    /// characters is necessary) or a one character escape (in which case
    /// this function returns the appropriate character).
    ///
    /// Returns the character obtained from the single character escape if it
    /// not a sequence, else it initialised the sequence and returns `None`.
    fn handle_first_escaped_char(
        &mut self,
        ch: char,
        lex_data: &mut LexingData,
        location: &LocationPointer,
    ) -> Option<char> {
        match ch {
            'a' => return Some('\u{0007}'),  // alert (beep, bell)
            'b' => return Some('\u{0008}'),  // backspace
            't' => return Some('\u{0009}'),  // horizontal tab
            'n' => return Some('\u{000A}'),  // newline (line feed)
            'v' => return Some('\u{000B}'),  // vertical tab
            'f' => return Some('\u{000C}'),  // formfeed page break
            'r' => return Some('\u{000D}'),  // carriage return
            'e' => return Some('\u{001B}'),  // escape character
            '"' => return Some('\u{0022}'),  // double quotation mark
            '\'' => return Some('\u{0027}'), // apostrophe or single quotation mark
            '?' => return Some('\u{003F}'),  // question mark (used to avoid trigraphs)
            '\\' => return Some('\u{005C}'), // backslash
            'u' | 'U' => *self = EscapeSequence::new_unicode(ch == 'u').into(),
            'x' => *self = EscapeSequence::new_hex().into(),
            '0'..='9' => *self = EscapeSequence::new_octal(ch).into(),
            _ => {
                lex_data.push_err(location.to_past(2, 1).warn(format!(
                "Escape ignored. Escaping character '{ch}' has no effect. Please remove the '\\'.",
            )));
                return Some(ch);
            }
        }
        None
    }

    /// Handle character in a escape context.
    ///
    /// This function pushes the characters in escaped sequences or characters,
    /// and pushes the character resulting of the escaped sequence into the
    /// string or char.
    ///
    /// # Escaped characters
    ///
    /// Single characters after the `\` (e.g., `\n`). If the character doesn't
    /// mean anything escaped, it is returned as raw with a warning (e.g.,
    /// `\o`).
    ///
    /// # Escape sequences
    ///
    /// - 4-digit hexadecimal code: `\uXXXX`
    ///     - The `\u` prefix must be followed by 4 hexdigits.
    /// - 8-digit hexadecimal code: `\UXXXXXXXX`
    ///     - The `\U` prefix must be followed by 8 hexdigits.
    /// - up-to-3 digit octal code: `\XXX`
    ///     - The `\` prefix can be followed by up to 3 octdigits, and can start
    ///       with 0 (`\0`, `\2`, `\07`, etc.)
    /// - hexadecimal modular code: `\xXX`
    ///     - The `\x` can be followed by any number of digits but only the last
    ///       2 hexdigits will be kept (i.e., modulo 0xff). For instance, `\x3f`
    ///       will produce a '?' and so will `\x23f`.
    #[must_use]
    pub fn push_one_char_in_escape(
        &mut self,
        ch: char,
        lex_data: &mut LexingData,
        location: &LocationPointer,
    ) -> Option<(char, Option<char>)> {
        match self {
            Self::Sequence(escape_sequence) => escape_sequence
                .push_char(ch, location, lex_data)
                .map(|(escaped_nb, additional)| {
                    let escaped = char::try_from(escaped_nb).unwrap_or_else(|_| {
                        let len = escape_sequence.len();
                        #[expect(clippy::arithmetic_side_effects, reason = "len >= 1")]
                        let err_location = location.to_past(len, len - 1);
                        lex_data.push_err(err_location.fail(format!(
                            "escaped sequence expands to {escaped_nb} which is not a valid char."
                        )));
                        '0'
                    });
                    (escaped, additional)
                }),
            Self::Single => self
                .handle_first_escaped_char(ch, lex_data, location)
                .map(|escaped| (escaped, None)),
        }
    }
}

impl From<EscapeSequence> for EscapeState {
    fn from(value: EscapeSequence) -> Self {
        Self::Sequence(value)
    }
}
