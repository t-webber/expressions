//! Module to handle symbols, convert them to operators and push them into the
//! [`Ast`].

#[expect(clippy::inline_modules, reason = "clearer api")]
pub mod api {
    //! Api module to choose what functions to export.

    #![allow(clippy::pub_use, reason = "expose simple API")]

    pub use super::blocks::braced_blocks::BracedBlock;
    pub use super::blocks::default::{FunctionCall, ListInitialiser};
    pub use super::blocks::parens::{Cast, ParensBlock};
}

mod blocks;
mod handlers;
mod sort_symbols;

use alloc::vec::IntoIter;

use blocks::recursion::blocks_handler;
use sort_symbols::handle_one_symbol;

use super::parse_content::ParseAction;
use super::state::ParsingState;
use super::tree::api::Ast;
use crate::errors::api::{ErrorLocation, Res};
use crate::lexer::api::{Symbol, Token};

/// Main handler to push a symbol into an [`Ast`].
///
/// This function deals also the recursion calls.
pub fn handle_symbol(
    symbol: Symbol,
    current: &mut Ast,
    p_state: &mut ParsingState,
    tokens: &mut IntoIter<Token>,
    location: ErrorLocation,
) -> Res<ParseAction> {
    match handle_one_symbol(location.wrap(symbol), current) {
        Err(err) => location.crash(err).into_res(),
        Ok(Some(block_state)) => blocks_handler(current, tokens, p_state, location, &block_state),
        Ok(None) => Res::ok(ParseAction::Continue),
    }
}
