//! Example of use of the crate, to lex and parse a file.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::panic,
    clippy::expect_used,
    clippy::unwrap_in_result,
    reason = "it's a cli"
)]

use std::{env, fs};

use c_parser::{display_tokens, lex, linearise, parse};

/// Parses the argvs to print nice errors on misuse, and returns the filename
/// otherwise.
fn parse_args() -> Option<String> {
    let mut args = env::args();
    let prog_name = args.next().expect("arg0 always exists");
    let Some(filename) = args.next() else {
        eprintln!("Missing argument, usage: {prog_name} <filename>");
        return None;
    };
    if args.next().is_some() {
        eprintln!("Too many argument, usage: {prog_name} <filename>");
        return None;
    }
    Some(filename)
}

fn main() {
    let Some(filename) = parse_args() else { return };
    let content =
        fs::read_to_string(&filename).unwrap_or_else(|_| panic!("Failed to read {filename}"));
    let files = [(0, filename.as_str(), content.as_str())];
    let tokens = lex(&content, 0)
        .unwrap_or_display(files.as_slice())
        .expect("no tokens found");
    println!("LEX: {}", display_tokens(&tokens));
    let ast = parse(tokens)
        .unwrap_or_display(files.as_slice())
        .expect("no ast found");
    println!("AST: {ast}");
    let ssa = linearise(ast)
        .unwrap_or_display(files.as_slice())
        .expect("no ssa");
    println!("{}", ssa.display());
}
