//! Module that implements the functions that lex raw strings.
//!
//! See [`lex`] for more information.

use super::state::api::{CommentState, EscapeState, LexingState as LS, SymbolState, end_current};
use super::types::api::{LexingData, Token};
use crate::errors::api::{LocationPointer, Res};

/// Function to manage one character.
///
/// This function updates the [`LS`] automaton, and executes the right
/// handlers.
fn lex_char(
    ch: char,
    location: &LocationPointer,
    lex_data: &mut LexingData,
    lex_state: &mut LS,
    escape_state: &mut Option<EscapeState>,
    eol: bool,
) {
    #[cfg(feature = "debug")]
    crate::lgp!(notab: "{ch} {escape_state:?} {lex_state:?}");
    match (ch, lex_state, escape_state) {
        (_, LS::StartOfLine, _) if ch.is_whitespace() => (),

        /* Inside comment */
        ('/', state @ LS::Comment(CommentState::Star), _) => {
            *state = LS::Comment(CommentState::False);
        }
        ('*', state @ LS::Comment(CommentState::True), _) => {
            *state = LS::Comment(CommentState::Star);
        }
        (_, LS::Comment(CommentState::True), _) => (),
        (_, state @ LS::Comment(CommentState::Star), _) => {
            *state = LS::Comment(CommentState::True);
        }

        /* Escaped character */
        (_, state @ (LS::Str(_) | LS::Char(None)), escape @ Some(_)) => {
            if let Some((escaped, additional)) = escape
                .as_mut()
                .expect("see match above")
                .push_one_char_in_escape(ch, lex_data, location)
            {
                match state {
                    LS::Str(str) => str.0.push(escaped),
                    LS::Char(inner @ None) => *inner = Some(escaped),
                    LS::Char(_)
                    | LS::Comment(_)
                    | LS::Ident(_)
                    | LS::StartOfLine
                    | LS::Symbols(_)
                    | LS::Unset => unreachable!(),
                }
                *escape = None;
                if let Some(inner) = additional {
                    lex_char(inner, location, lex_data, state, escape, eol);
                }
            }
        }

        /* Create comment */
        ('*', state, _) if state.symbol_and_last_is('/') => {
            if let LS::Symbols(symbol) = state {
                symbol.clear_last();
            }
            end_current(state, lex_data, location);
            *state = LS::Comment(CommentState::True);
        }

        /* Escape character */
        ('\\', LS::Char(None) | LS::Str(_), escape) => *escape = Some(EscapeState::Single),
        ('\\', _, escape) if eol => *escape = Some(EscapeState::Single),
        ('\\', _, _) => {
            lex_data.push_err(
                location
                    .fail("Escape characters are only authorised in strings or chars.".to_owned()),
            );
        }

        /* Static strings and chars */
        // open/close
        ('\'', LS::Symbols(symbol_state), _) if symbol_state.is_trigraph_prefix() => {
            if let Some((size, symbol)) = symbol_state.push(ch, lex_data, location) {
                lex_data.push_token(Token::from_symbol(symbol, size, location));
            }
        }
        ('\'', state @ LS::Char(_), _) => end_current(state, lex_data, location),
        ('\'', state, _) if !matches!(state, LS::Str(_)) => {
            end_current(state, lex_data, location);
            *state = LS::Char(None);
        }
        ('\"', state @ LS::Str(_), _) => {
            end_current(state, lex_data, location);
        }
        ('\"', state, _) if !matches!(state, LS::Char(_)) => {
            end_current(state, lex_data, location);
            *state = LS::Str((String::new(), location.to_owned()));
        }
        // middle
        (_, LS::Char(Some(_)), _) =>
            lex_data.push_err(location.fail("A char must contain only one character.".to_owned())),
        (_, state @ LS::Char(None), _) => *state = LS::Char(Some(ch)),
        (_, LS::Str((val, _)), _) => val.push(ch),

        /* Operator symbols */
        ('/', state, _) if state.symbol_and_last_is('/') => {
            if let LS::Symbols(symbol) = state {
                symbol.clear_last();
            }
            end_current(state, lex_data, location);
            lex_data.set_end_line();
        }
        ('.', LS::Ident(ident), _) if !ident.contains('.') && ident.is_number() => {
            ident.push('.');
        }
        ('+' | '-', LS::Ident(ident), _) if !ident.contains(['-', '+']) && ident.last_is_exp() =>
            ident.push(ch),

        (_, state, _) if is_symbol(ch) => lex_char_symbol(state, ch, lex_data, location),

        /* Whitespace: end of everyone */
        (_, state, _) if ch.is_whitespace() => {
            end_current(state, lex_data, location);
        }

        /* Identifier */
        (_, state, _) if ch.is_alphanumeric() || matches!(ch, '_') =>
            lex_char_ident(state, lex_data, location, ch),
        (_, _, _) => {
            lex_data.push_err(location.fail(format!("Character '{ch}' not supported.")));
        }
    }
}

/// Function to manage one character, if it is an ident-valid character.
///
/// This function updates the [`LS`] automaton, and executes the right
/// handlers.
fn lex_char_ident(state: &mut LS, lex_data: &mut LexingData, location: &LocationPointer, ch: char) {
    if let LS::Symbols(symbol) = state
        && matches!(symbol.last(), '.')
        && ch.is_ascii_digit()
    {
        symbol.clear_last();
        end_current(state, lex_data, location);
        state.new_ident_str(format!("0.{ch}"));
    } else if let LS::Ident(val) = state {
        val.push(ch);
    } else {
        end_current(state, lex_data, location);
        state.new_ident(ch);
    }
}

/// Function to manage one character, if it is a symbol character.
///
/// This function updates the [`LS`] automaton, and executes the right
/// handlers.
fn lex_char_symbol(
    state: &mut LS,
    ch: char,
    lex_data: &mut LexingData,
    location: &LocationPointer,
) {
    if let LS::Symbols(symbol_state) = state {
        if let Some((size, symbol)) = symbol_state.push(ch, lex_data, location) {
            lex_data.push_token(Token::from_symbol(symbol, size, location));
        }
    } else {
        end_current(state, lex_data, location);
        *state = LS::Symbols(SymbolState::from(ch));
    }
}

/// Function that lexes a whole source file.
///
/// This function creates the automaton and the data to be modified by the other
/// functions. Every character is parsed one by one, and the state is modified
/// accordingly. When the state changes, the buffers of the state are empty into
/// the data.
#[must_use]
pub fn lex(content: &str, file_id: u32) -> Res<Vec<Token>> {
    let mut location = LocationPointer::start_file(file_id);
    let mut lex_data = LexingData::default();
    let mut lex_state = LS::default();
    let mut escape_state = None;

    for line in content.lines() {
        location.incr_line(&mut |err| lex_data.push_err(err));
        lex_line(line, &mut location, &mut lex_data, &mut lex_state, &mut escape_state);
    }
    end_current(&mut lex_state, &mut lex_data, &location);

    lex_data.into_res()
}

/// Function that lexes one line.
///
/// It stops at the first erroneous character, or at the end of the line if
/// everything was ok.
fn lex_line(
    line: &str,
    location: &mut LocationPointer,
    lex_data: &mut LexingData,
    lex_state: &mut LS,
    escape_state: &mut Option<EscapeState>,
) {
    lex_data.newline();
    let trimmed = line.trim_end();
    if trimmed.is_empty() {
        end_current(lex_state, lex_data, location);
        return;
    }
    let last = trimmed.len().checked_sub(1).expect("trimmed is not empty");
    for (idx, ch) in trimmed.chars().enumerate() {
        location.incr_col(&mut |err| lex_data.push_err(err));
        lex_char(ch, location, lex_data, lex_state, escape_state, idx == last);
        if lex_data.is_end_line() {
            break;
        }
    }
    location.incr_col(&mut |err| lex_data.push_err(err));
    if matches!(escape_state, Some(EscapeState::Single)) {
        *escape_state = None;
        if line.ends_with(char::is_whitespace) {
            lex_data.push_err(location.suggest(
                "Found whitespace after '\\' at EOL. Please remove the space.".to_owned(),
            ));
        }
    } else {
        end_current(lex_state, lex_data, location);
        if !matches!(*lex_state, LS::Comment(CommentState::True)) {
            *lex_state = LS::default();
        }
    }
}

/// Returns `true` iff `ch` is a symbol.
#[must_use]
#[rustfmt::skip]
const fn is_symbol(ch: char) -> bool {
    matches!(
        ch,
        '(' | ')' | '[' | ']' | '{' | '}' | '~' | '!' | '*' | '&' | '%' | '/'
            | '>' | '<' | '=' | '|' | '^' | ',' | '?' | ':' | ';' | '.' | '+'
            | '-' | '#'
    )
}
