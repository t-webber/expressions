//! Module to define the symbol-handling-state

use core::fmt;

use crate::errors::api::LocationPointer;
use crate::lexer::api::Symbol;
use crate::lexer::types::api::LexingData;

/// A default impossible character, used to not have to use options.
const NULL: char = '\0';

/// Current state of the symbols.
///
/// Operators have a maximum length of 3, so this struct contains the last 3 (or
/// less) symbols found. We trying to push a new char, it will check the biggest
/// succession of these chars to make an operator and make it a token. This
/// makes space for the `char` that is to be pushed.
#[derive(PartialEq, Eq)]
pub struct SymbolState {
    /// Oldest char that was pushed, if not equal to [`NULL`].
    first: char,
    /// Second oldest
    second: char,
    /// Newest char that was pushed, if not equal to [`NULL`].
    third: char,
}

impl SymbolState {
    /// Removes last pushed `char` of the state
    pub const fn clear_last(&mut self) {
        if self.third != NULL {
            self.third = NULL;
        } else if self.second != NULL {
            self.second = NULL;
        } else {
            self.first = NULL;
        }
    }

    /// Handler for digraphs and trigraphs.
    fn handle_digraphs_trigraphs(&mut self) -> Option<(String, usize)> {
        let symbols = (self.first, self.second, self.third);
        let (graph, is_trigraph) = match symbols {
            ('?', '?', '=') => (Some('#'), true),
            ('?', '?', '/') => (Some('\\'), true),
            ('?', '?', '\'') => (Some('^'), true),
            ('?', '?', '(') => (Some('['), true),
            ('?', '?', ')') => (Some(']'), true),
            ('?', '?', '!') => (Some('|'), true),
            ('?', '?', '<') => (Some('{'), true),
            ('?', '?', '>') => (Some('}'), true),
            ('?', '?', '-') => (Some('~'), true),
            ('<', ':', _) => (Some('['), false),
            (':', '>', _) => (Some(']'), false),
            ('<', '%', _) => (Some('{'), false),
            ('%', '>', _) => (Some('}'), false),
            ('%', ':', _) => {
                return Some((
                    "Found invalid character '#', found by replacing digraph '%:'.".to_owned(),
                    2,
                ));
            }
            _ => (None, false),
        };
        if let Some(symbol) = graph {
            if is_trigraph {
                let msg = format!(
                    "use of trigraphs: replace '{}{}{}' by '{symbol}'.",
                    self.first, self.second, self.third
                );
                self.first = NULL;
                self.second = NULL;
                self.third = NULL;
                return Some((msg, 3));
            }
            self.first = symbol;
            self.second = self.third;
            self.third = NULL;
        }
        None
    }

    /// Checks if the state contains a value or not.
    pub const fn is_empty(&self) -> bool {
        self.first == NULL && self.second == NULL && self.third == NULL
    }

    /// Checks if the state is a valid trigraph or not.
    pub const fn is_trigraph_prefix(&self) -> bool {
        matches!((self.first, self.second, self.third), ('?', '?', NULL) | (_, '?', '?'))
    }

    /// Returns the last element of the state by copying it: it is not removed
    /// from the state.
    pub fn last(&self) -> char {
        debug_assert!(self.first != NULL, "initialised with one");
        if self.second == NULL {
            self.first
        } else if self.third == NULL {
            self.second
        } else {
            self.third
        }
    }

    /// Returns the number of [`Symbol`] in [`SymbolState`]
    pub fn len(&self) -> usize {
        debug_assert!(self.first != NULL, "initialised with one");
        if self.third == NULL {
            if self.second == NULL {
                return 1;
            }
            return 2;
        }
        3
    }

    /// Pushes a `char` into the state.
    ///
    /// # Returns
    ///
    /// This function may return a [`Symbol`] (and its `size`) if space was
    /// needed.
    pub fn push(
        &mut self,
        value: char,
        lex_data: &mut LexingData,
        location: &LocationPointer,
    ) -> Option<(usize, Symbol)> {
        let op = if self.third == NULL {
            None
        } else {
            self.try_to_operator(lex_data, location)
        };
        if self.first == NULL {
            self.first = value;
        } else if self.second == NULL {
            self.second = value;
        } else if self.third == NULL {
            self.third = value;
        } else {
            unreachable!("symbols full but try_to_operator returned none");
        }
        op
    }

    /// Forces the state to make space.
    ///
    /// # Returns
    ///
    /// This function returns the [`Symbol`] that was cleared from the state and
    /// that needs to be pushed.
    ///
    /// This functions returns `None` if and only if the state was empty.
    pub fn try_to_operator(
        &mut self,
        lex_data: &mut LexingData,
        location: &LocationPointer,
    ) -> Option<(usize, Symbol)> {
        debug_assert!(!self.is_empty(), "initialised with one");
        let initial_len = self.len();
        if let Some((msg, len)) = self.handle_digraphs_trigraphs() {
            let new_location = location.to_past(len, initial_len);
            lex_data.push_err(new_location.fail(msg));
        }
        let result = match (self.first, self.second, self.third) {
            ('<', '<', '=') => Some((3, Symbol::ShiftLeftAssign)),
            ('>', '>', '=') => Some((3, Symbol::ShiftRightAssign)),
            ('-', '>', _) => Some((2, Symbol::Arrow)),
            ('+', '+', _) => Some((2, Symbol::Increment)),
            ('-', '-', _) => Some((2, Symbol::Decrement)),
            ('<', '<', _) => Some((2, Symbol::ShiftLeft)),
            ('>', '>', _) => Some((2, Symbol::ShiftRight)),
            ('&', '&', _) => Some((2, Symbol::LogicalAnd)),
            ('|', '|', _) => Some((2, Symbol::LogicalOr)),
            ('<', '=', _) => Some((2, Symbol::Le)),
            ('>', '=', _) => Some((2, Symbol::Ge)),
            ('=', '=', _) => Some((2, Symbol::Equal)),
            ('!', '=', _) => Some((2, Symbol::Different)),
            ('+', '=', _) => Some((2, Symbol::AddAssign)),
            ('-', '=', _) => Some((2, Symbol::SubAssign)),
            ('*', '=', _) => Some((2, Symbol::MulAssign)),
            ('/', '=', _) => Some((2, Symbol::DivAssign)),
            ('%', '=', _) => Some((2, Symbol::ModAssign)),
            ('&', '=', _) => Some((2, Symbol::AndAssign)),
            ('|', '=', _) => Some((2, Symbol::OrAssign)),
            ('^', '=', _) => Some((2, Symbol::XorAssign)),
            ('#', '#', _) => Some((2, Symbol::Concat)),
            ('+', _, _) => Some((1, Symbol::Plus)),
            ('-', _, _) => Some((1, Symbol::Minus)),
            ('(', _, _) => Some((1, Symbol::ParenthesisOpen)),
            (')', _, _) => Some((1, Symbol::ParenthesisClose)),
            ('[', _, _) => Some((1, Symbol::BracketOpen)),
            (']', _, _) => Some((1, Symbol::BracketClose)),
            ('.', _, _) => Some((1, Symbol::Dot)),
            ('{', _, _) => Some((1, Symbol::BraceOpen)),
            ('}', _, _) => Some((1, Symbol::BraceClose)),
            ('~', _, _) => Some((1, Symbol::BitwiseNot)),
            ('!', _, _) => Some((1, Symbol::LogicalNot)),
            ('*', _, _) => Some((1, Symbol::Star)),
            ('&', _, _) => Some((1, Symbol::Ampersand)),
            ('%', _, _) => Some((1, Symbol::Modulo)),
            ('/', _, _) => Some((1, Symbol::Divide)),
            ('>', _, _) => Some((1, Symbol::Gt)),
            ('<', _, _) => Some((1, Symbol::Lt)),
            ('=', _, _) => Some((1, Symbol::Assign)),
            ('|', _, _) => Some((1, Symbol::BitwiseOr)),
            ('^', _, _) => Some((1, Symbol::BitwiseXor)),
            (',', _, _) => Some((1, Symbol::Comma)),
            ('?', _, _) => Some((1, Symbol::Interrogation)),
            (':', _, _) => Some((1, Symbol::Colon)),
            (';', _, _) => Some((1, Symbol::SemiColon)),
            ('#', _, _) => Some((1, Symbol::Hash)),
            (NULL, NULL, NULL) => None,
            pairs => unreachable!(
                "unsupported character were filtered before, and this is unsupported: {pairs:?}"
            ),
        };

        if let Some((nb_consumed, _)) = &result {
            match *nb_consumed {
                1 => {
                    self.first = self.second;
                    self.second = self.third;
                    self.third = NULL;
                }
                2 => {
                    self.first = self.third;
                    self.second = NULL;
                    self.third = NULL;
                }
                3 => {
                    self.first = NULL;
                    self.second = NULL;
                    self.third = NULL;
                }
                _ => unreachable!("`nb_consumed` must be 0, 1, 2 or 3"),
            }
        }
        result
    }
}

impl From<char> for SymbolState {
    fn from(value: char) -> Self {
        Self { first: value, second: NULL, third: NULL }
    }
}

impl fmt::Debug for SymbolState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SymbolState({:?},{:?},{:?})", self.first, self.second, self.third)
    }
}
