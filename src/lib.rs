#![doc = include_str!("../docs/README.md")]
#![feature(
    is_ascii_octdigit,
    f128,
    pattern,
    try_trait_v2,
    coverage_attribute,
    stmt_expr_attributes,
    macro_metavar_expr_concat,
    try_trait_v2_residual
)]

extern crate alloc;

mod errors;
mod lexer;
mod lineariser;
mod parser;
mod utils;

#[expect(
    clippy::useless_attribute,
    clippy::pub_use,
    reason = "re-export for better API"
)]
pub use crate::errors::api::Res;
#[expect(
    clippy::useless_attribute,
    clippy::pub_use,
    reason = "re-export for better API"
)]
pub use crate::lexer::api::{Number, Token, TokenValue, display_tokens, lex};
#[expect(
    clippy::useless_attribute,
    clippy::pub_use,
    reason = "re-export for better API"
)]
pub use crate::lineariser::linearise;
#[expect(
    clippy::useless_attribute,
    clippy::pub_use,
    reason = "re-export for better API"
)]
pub use crate::parser::api::{Ast, BracedBlock, parse};

/// String to represent an empty node when displaying the AST in a
/// human-readable way.
const EMPTY: &str = "\u{2205} ";
