//! Module to convert a list of [`Token`] into an [`Ast`].

use alloc::vec::IntoIter;

use super::keyword::handle_keyword;
use super::literal::Literal;
use super::modifiers::push::Push as _;
use super::state::ParsingState;
use super::symbols::api::BracedBlock;
use super::symbols::handle_symbol;
use super::tree::api::Ast;
use super::variable::Variable;
use crate::errors::api::{ErrorLocation, Res};
use crate::lexer::api::{Token, TokenValue};

/// Indicates whether the current block should continue parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseAction {
    /// Continue parsing the current block.
    Continue,
    /// Stop parsing the current block (a closing delimiter was found).
    Stop,
}

/// Pushes a [`Literal`] into the [`Ast`]
fn handle_literal(current: &mut Ast, lit: Ast, location: ErrorLocation) -> Res<ParseAction> {
    current
        .push_block_as_leaf(lit)
        .map_err(|err| location.crash(err))?;
    Res::ok(ParseAction::Continue)
}

/// Function to parse one node, and by recursivity, one block. At the end of the
/// block, this function stops and is recalled from [`parse`].
pub fn parse_block(
    tokens: &mut IntoIter<Token>,
    p_state: &mut ParsingState,
    current: &mut Ast,
) -> Res<()> {
    let mut errors = vec![];
    while let Some(token) = tokens.next() {
        #[cfg(feature = "debug")]
        crate::lgp!("\x1b[36m{:20} on {current}\x1b[0m", format!("{token}"),);
        let (value, location) = token.into_value_location();
        let res = match value {
            TokenValue::Char(ch) =>
                handle_literal(current, Ast::Leaf(location.wrap(Literal::Char(ch))), location),
            TokenValue::Ident(val) =>
                handle_literal(current, Ast::Variable(Variable::from(location.wrap(val))), location),
            TokenValue::Number(nb) =>
                handle_literal(current, Ast::Leaf(location.wrap(Literal::Number(nb))), location),
            TokenValue::Str(val) =>
                handle_literal(current, Ast::Leaf(location.wrap(Literal::Str(val))), location),
            TokenValue::Symbol(symbol) => handle_symbol(symbol, current, p_state, tokens, location),
            TokenValue::Keyword(keyword) => handle_keyword(keyword, current, p_state, location),
        };
        let has_failures = res.has_failures();
        let action = res.store_errors(&mut |err| errors.push(err));
        if has_failures || action != Some(ParseAction::Continue) {
            break;
        }
    }
    Res::from(((), errors))
}

/// Parses a list of tokens into an Abstract Syntax Tree.
///
/// This function manages the blocks with successive calls and checks.
#[must_use]
pub fn parse(tokens: Vec<Token>) -> Res<BracedBlock> {
    let mut tokens_iter = tokens.into_iter();
    let mut ast = Ast::BracedBlock(BracedBlock::default());
    let mut p_state = ParsingState::default();
    let res = parse_block(&mut tokens_iter, &mut p_state, &mut ast);
    let Ast::BracedBlock(bb) = ast else {
        unreachable!("Braced block can't become another node.")
    };
    if !res.has_failures() && p_state.has_opening_blocks() {
        res.extend_errs(p_state.mismatched_error()).map(|()| bb)
    } else {
        res.map(|()| bb)
    }
}
