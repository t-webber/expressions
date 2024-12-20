use super::state::ParsingState;
use super::tree::binary::BinaryOperator;
use super::tree::unary::UnaryOperator;
use super::tree::Node;
use crate::as_error;
use crate::errors::compile::CompileError;
use crate::errors::location::Location;
use crate::lexer::api::tokens_types::{Symbol, Token};
use crate::parser::parse_block;
use crate::parser::tree::{Ternary, TernaryOperator};
use core::mem;
extern crate alloc;
use alloc::vec::IntoIter;

fn safe_decr(counter: &mut usize) -> Result<(), &'static str> {
    *counter = counter.checked_sub(1).ok_or("Mismactched closing brace")?;
    Ok(())
}

fn handle_colon(current: &mut Node, p_state: &mut ParsingState) -> Result<(), &'static str> {
    if let Node::Ternary(Ternary {
        condition,
        success,
        failure,
        ..
    }) = current
    {
        if condition.is_none() || success.is_none() || p_state.ternary == 0 {
            return Err("Found empty success block. Succession of '?' and ':' without expression is not allowed.");
        }
        *failure = Some(Box::new(Node::Empty));
        p_state.ternary -= 1;
        Ok(())
    } else {
        Err("Unexpected symbol ':'. Found outside of goto and ternary operator context.")
    }
}

fn handle_one_symbol(
    symbol: &Symbol,
    current: &mut Node,
    p_state: &mut ParsingState,
) -> Result<bool, &'static str> {
    use BinaryOperator as BOp;
    #[allow(clippy::enum_glob_use)]
    use Symbol::*;
    use UnaryOperator as UOp;
    match symbol {
        // mirror unary
        BitwiseNot => current.push_op(UOp::BitwiseNot)?,
        LogicalNot => current.push_op(UOp::LogicalNot)?,
        // mirror binary
        Assign => current.push_op(BOp::Assign)?,
        BitwiseOr => current.push_op(BOp::BitwiseOr)?,
        BitwiseXor => current.push_op(BOp::BitwiseXor)?,
        Divide => current.push_op(BOp::Divide)?,
        Gt => current.push_op(BOp::Gt)?,
        Lt => current.push_op(BOp::Lt)?,
        Modulo => current.push_op(BOp::Modulo)?,
        AddAssign => current.push_op(BOp::AddAssign)?,
        AndAssign => current.push_op(BOp::AndAssign)?,
        Different => current.push_op(BOp::Different)?,
        DivAssign => current.push_op(BOp::DivAssign)?,
        Equal => current.push_op(BOp::Equal)?,
        Ge => current.push_op(BOp::Ge)?,
        Le => current.push_op(BOp::Le)?,
        LogicalAnd => current.push_op(BOp::LogicalAnd)?,
        LogicalOr => current.push_op(BOp::LogicalOr)?,
        ModAssign => current.push_op(BOp::ModAssign)?,
        MulAssign => current.push_op(BOp::MulAssign)?,
        OrAssign => current.push_op(BOp::OrAssign)?,
        LeftShift => current.push_op(BOp::LeftShift)?,
        RightShift => current.push_op(BOp::RightShift)?,
        SubAssign => current.push_op(BOp::SubAssign)?,
        XorAssign => current.push_op(BOp::XorAssign)?,
        LeftShiftAssign => current.push_op(BOp::LeftShiftAssign)?,
        RightShiftAssign => current.push_op(BOp::RightShiftAssign)?,
        // unique non mirrors
        Ampercent => current.push_op(UOp::AddressOf)?,
        Arrow => current.push_op(BOp::StructEnumMemberPointerAccess)?,
        Dot => current.push_op(BinaryOperator::StructEnumMemberAccess)?,
        // postfix has smaller precedence than prefix
        Increment => current
            .push_op(UOp::PostfixIncrement)
            .unwrap_or(current.push_op(UOp::PrefixIncrement)?), // Operator is left to right, so, if an error occurs, current isn't modified
        Decrement => current
            .push_op(UOp::PostfixDecrement)
            .unwrap_or(current.push_op(UOp::PrefixDecrement)?), // Operator is left to right, so, if an error occurs, current isn't modified
        // binary and unary operators //TODO: not sure this is good, may not work on extreme cases
        Minus => current
            .push_op(BOp::Subtract)
            .unwrap_or(current.push_op(UOp::Minus)?),
        Plus => current
            .push_op(BOp::Add)
            .unwrap_or(current.push_op(UOp::Plus)?),
        Star => current
            .push_op(BOp::Multiply)
            .unwrap_or(current.push_op(UOp::Indirection)?),
        // ternary (only ternary because trigraphs are ignored, and colon is sorted in main function in mod.rs)
        Interrogation => {
            let old_node = mem::take(current);
            *current = Node::Ternary(Ternary {
                operator: TernaryOperator,
                condition: Some(Box::from(old_node)),
                success: Some(Box::from(Node::Empty)),
                failure: None,
            });
            p_state.ternary += 1;
        }
        Colon => handle_colon(current, p_state)?,
        //
        SemiColon => return Ok(false),
        Comma => todo!(),
        // parenthesis
        BraceOpen => p_state.braces += 1,
        BraceClose => {
            safe_decr(&mut p_state.braces)?;
            return Ok(false);
        }
        BracketOpen => p_state.brackets += 1,
        BracketClose => {
            safe_decr(&mut p_state.brackets)?;
            return Ok(false);
        }
        ParenthesisOpen => p_state.parenthesis += 1,
        ParenthesisClose => {
            safe_decr(&mut p_state.parenthesis)?;
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn handle_symbol(
    symbol: &Symbol,
    current: &mut Node,
    p_state: &mut ParsingState,
    tokens: &mut IntoIter<Token>,
    location: Location,
) -> Result<(), CompileError> {
    if handle_one_symbol(symbol, current, p_state).map_err(|err| as_error!(location, "{err}"))? {
        parse_block(tokens, p_state, current)
    } else {
        Ok(())
    }
}
