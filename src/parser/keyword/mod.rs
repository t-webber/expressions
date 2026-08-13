//! Module to handle keywords, convert them to operators and push them into the
//! [`Ast`].

pub mod attributes;
pub mod control_flow;
pub mod functions;
pub mod sort;

use control_flow::pushable::PushableKeyword;
use sort::{Context, KeywordParsing, PushInNode as _};

use super::parse_content::ParseAction;
use super::state::ParsingState;
use crate::errors::api::{ErrorLocation, Res};
use crate::lexer::api::Keyword;
use crate::parser::symbols::api::BracedBlock;
use crate::parser::tree::Ast;
use crate::parser::tree::api::AstPushContext;

/// Main handler to push a keyword into an [`Ast`].
///
/// This function deals also the recursion calls.
pub fn handle_keyword(
    keyword: Keyword,
    current: &mut Ast,
    p_state: &ParsingState,
    keyword_location: ErrorLocation,
) -> Res<ParseAction> {
    let ctx = if p_state.is_in_switch() {
        Context::Switch
    } else {
        Context::from(&*current)
    };
    let parsed_keyword: KeywordParsing =
        KeywordParsing::try_from((keyword, ctx)).map_err(|msg| keyword_location.crash(msg))?;
    let ast_push_ctx = match parsed_keyword {
        KeywordParsing::Attr(_) => AstPushContext::UserVariable,
        KeywordParsing::Pushable(PushableKeyword::Else) => AstPushContext::Else,
        KeywordParsing::CtrlFlow(_) | KeywordParsing::False | KeywordParsing::True =>
            AstPushContext::Literal,
        KeywordParsing::Func(_) | KeywordParsing::Null => AstPushContext::None,
    };
    let located_keyword = keyword_location.wrap(parsed_keyword);
    if current.can_push_leaf_with_ctx(ast_push_ctx) {
        located_keyword
            .push_in_node(current)
            .map_err(|msg| keyword_location.crash(msg))?;
    } else if let Ast::BracedBlock(BracedBlock { elts, full: false, .. }) = current {
        match elts.last_mut() {
            Some(last) if last.is_empty() => {
                located_keyword
                    .push_in_node(last)
                    .map_err(|msg| keyword_location.crash(msg))?;
            }
            Some(Ast::BracedBlock(_) | Ast::ControlFlow(_)) | None => {
                let mut new = Ast::Empty;
                located_keyword
                    .push_in_node(&mut new)
                    .map_err(|msg| keyword_location.crash(msg))?;
                elts.push(new);
            }
            Some(_) => {
                return keyword_location
                    .crash("Invalid keyword in current context. Perhaps a missing ';'".to_owned())
                    .into_res();
            }
        }
    } else {
        unreachable!("trying to push {:?} in {current}", located_keyword.into_inner().0)
    }
    Res::ok(ParseAction::Continue)
}
