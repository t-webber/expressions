//! Handler for block character

use alloc::vec::IntoIter;
use core::mem;

use super::braced_blocks::BracedBlock;
use super::default::ListInitialiser;
use super::parens::ParensBlock;
use crate::Res;
use crate::errors::api::ErrorLocation;
use crate::lexer::api::Token;
use crate::parser::keyword::control_flow::node::{
    switch_wanting_block, try_push_semicolon_control
};
use crate::parser::modifiers::functions::{CanMakeFnRes, MakeFunction as _};
use crate::parser::modifiers::list_initialiser::{
    apply_to_last_list_initialiser, can_push_list_initialiser
};
use crate::parser::modifiers::push::Push as _;
use crate::parser::operators::api::BinaryOperator;
use crate::parser::parse_content::{ParseAction, parse_block};
use crate::parser::state::{BlockType, ParsingState};
use crate::parser::tree::api::Ast;

/// State to indicate what needs to be done
#[derive(Debug)]
pub enum TodoBlock {
    /// `}`
    CloseBraceBlock,
    /// `]`
    CloseBracket,
    /// `)`
    CloseParens,
    /// `{`
    OpenBraceBlock,
    /// `[`
    OpenBracket,
    /// `(`
    OpenParens,
    /// `;`
    SemiColon,
}

/// Manages recursions calls and creates blocks
pub fn blocks_handler(
    current: &mut Ast,
    tokens: &mut IntoIter<Token>,
    p_state: &mut ParsingState,
    location: ErrorLocation,
    block_state: &TodoBlock,
) -> Res<ParseAction> {
    #[cfg(feature = "debug")]
    crate::lgp!("Handling block symbol {block_state:?}");
    match block_state {
        // semi-colon
        TodoBlock::SemiColon => {
            handle_semicolon(current);
            Res::ok(ParseAction::Continue)
        }
        // parenthesis
        TodoBlock::CloseParens => {
            p_state.push_closing_block(BlockType::Parenthesis, location);
            Res::ok(ParseAction::Stop)
        }
        TodoBlock::OpenParens => handle_parenthesis_open(current, p_state, tokens, location).map(|()| ParseAction::Continue),
        // bracket
        TodoBlock::CloseBracket => {
            p_state.push_closing_block(BlockType::Bracket, location);
            Res::ok(ParseAction::Stop)
        }
        TodoBlock::OpenBracket => {
            let mut bracket_node = Ast::Empty;
            p_state.push_ctrl_flow(false);
            let res = parse_block(tokens, p_state, &mut bracket_node);
            let has_failures = res.has_failures();
            if has_failures {
                res
            } else if p_state.pop_ctrl_flow().is_none() {
                res.add_err(BlockType::Bracket.mismatched_err_end(location))
            } else if let Some(start_location) = p_state.pop_and_compare_block(&BlockType::Bracket) {
                let op_location = start_location.into_extended(location);
                if let Err(err) = current.push_op(op_location.wrap(BinaryOperator::ArraySubscript)) {
                    res.add_err(location.crash(err))
                } else if let Err(err) = current.push_block_as_leaf(bracket_node) {
                    res.add_err(location.crash(err))
                }
                else {res}
            } else {
                res.add_err(BlockType::Bracket.mismatched_err_end(location))
            }.map(|()| ParseAction::Continue)
        }
        // brace
        TodoBlock::CloseBraceBlock
            if apply_to_last_list_initialiser(current, &|_, full, start| {*full = true; *start = start.into_extended(location); }).is_none() =>
        {
            p_state.push_closing_block(BlockType::Brace, location);
            Res::ok(ParseAction::Stop)
        }
        TodoBlock::OpenBraceBlock => match can_push_list_initialiser(current) {
            Err(op) => location.crash(format!(
                    "Found operator '{op}' applied on list initialiser '{{}}', but this is not allowed."
            ))
            .into_res(),
            Ok(true) => {
                current
                    .push_block_as_leaf(Ast::ListInitialiser(ListInitialiser { elts: vec![], full: false, location }))
                    .map_err(|err| location.crash(err))?;
                Res::ok(ParseAction::Continue)
            }
            Ok(false) => handle_brace_block_open(current, tokens, p_state, location).map(|()| ParseAction::Continue),
        },
        // others
        TodoBlock::CloseBraceBlock => Res::ok(ParseAction::Continue),
    }
}

/// Handler for `{`
///
/// Deals with recursion and merges the braced-blocks
fn handle_brace_block_open(
    current: &mut Ast,
    tokens: &mut IntoIter<Token>,
    p_state: &mut ParsingState,
    location: ErrorLocation,
) -> Res<()> {
    let mut brace_block = Ast::BracedBlock(BracedBlock { elts: vec![], full: false, location });
    p_state.push_ctrl_flow(switch_wanting_block(current));
    let res = parse_block(tokens, p_state, &mut brace_block);
    if res.has_failures() {
        return res;
    }
    if p_state.pop_ctrl_flow().is_some()
        && let Some(end_location) = p_state.pop_and_compare_block(&BlockType::Brace)
    {
        let Ast::BracedBlock(mut inner) = brace_block else {
            unreachable!("a block can't be changed to another node")
        };
        inner.location.extend(end_location);
        inner.full = true;
        if let Err(msg) = current.push_braced_block(inner) {
            return res.add_err(location.crash(msg));
        }
        res
    } else {
        res.add_err(BlockType::Brace.mismatched_err_end(location))
    }
}

/// Handler for `(`
///
/// Deals with recursion and pushes the function arguments if needed
fn handle_parenthesis_open(
    current: &mut Ast,
    p_state: &mut ParsingState,
    tokens: &mut IntoIter<Token>,
    location: ErrorLocation,
) -> Res<()> {
    match current.can_make_function() {
        CanMakeFnRes::CanMakeFn(variable_depth) =>
            make_function(current, p_state, tokens, location, variable_depth),
        CanMakeFnRes::None =>
            handle_non_function_parenthesis_open(current, p_state, tokens, location),
        CanMakeFnRes::TooDeep => Res::from_err(
            location.crash("Code to complex: AST to deep to fit depth in 32 bits.".to_owned()),
        ),
    }
}

/// Create a function for the found '('
///
/// Builds a function on a variable and adds its arguments.
fn make_function(
    current: &mut Ast,
    p_state: &mut ParsingState,
    tokens: &mut IntoIter<Token>,
    location: ErrorLocation,
    variable_depth: u32,
) -> Res<()> {
    let mut arguments_node = Ast::FunctionArgsBuild(vec![Ast::Empty], location, location);
    p_state.push_ctrl_flow(false);
    let mut res = parse_block(tokens, p_state, &mut arguments_node);
    let has_failures = res.has_failures();
    if has_failures {
        return res;
    }
    if p_state.pop_ctrl_flow().is_none() {
        return res.add_err(BlockType::Parenthesis.mismatched_err_end(location));
    }
    if let Some(closing_loc) = p_state.pop_and_compare_block(&BlockType::Parenthesis) {
        if let Ast::FunctionArgsBuild(vec, ..) = &mut arguments_node {
            let args = if let Some(last) = vec.last()
                && last.is_empty()
            {
                vec.pop();
                let args = mem::take(vec);
                if !args.is_empty() {
                    res = res.add_err(
                        arguments_node.location().suggest(
                            "Found extra comma in function argument list. Please remove the comma."
                                .to_owned(),
                        ),
                    );
                }
                args
            } else {
                mem::take(vec)
            };
            current.make_function(variable_depth, args, location.into_extended(closing_loc));
            res
        } else {
            unreachable!("a function args build cannot be dismissed as root");
        }
    } else {
        res.add_err(BlockType::Parenthesis.mismatched_err_end(location))
    }
}

/// Handles an opening '(', but when it can't be a function call.
fn handle_non_function_parenthesis_open(
    current: &mut Ast,
    p_state: &mut ParsingState,
    tokens: &mut IntoIter<Token>,
    location: ErrorLocation,
) -> Res<()> {
    let mut parenthesised_block = Ast::Empty;
    let res = parse_block(tokens, p_state, &mut parenthesised_block);
    let has_failures = res.has_failures();
    if has_failures {
        res
    } else {
        parenthesised_block.fill();
        if let Some(end_location) = p_state.pop_and_compare_block(&BlockType::Parenthesis) {
            if let Err(err) = current.push_block_as_leaf(ParensBlock::make_parens_ast(
                parenthesised_block,
                end_location.into_extended(location),
            )) {
                res.add_err(location.crash(err))
            } else {
                res
            }
        } else {
            res.add_err(BlockType::Parenthesis.mismatched_err_end(location))
        }
    }
}

/// Handler for `;`
///
/// Pushes a new empty node if needed.
fn handle_semicolon(current: &mut Ast) {
    if try_push_semicolon_control(current) {
        return;
    }
    if let Ast::BracedBlock(BracedBlock { elts, full, .. }) = current
        && !*full
    {
        if let Some(last) = elts.last_mut() {
            if !last.is_empty() {
                last.fill();
                elts.push(Ast::Empty);
            }
        } else {
            elts.push(Ast::Empty);
        }
    } else if !current.is_empty() {
        current.brace();
    }
}
