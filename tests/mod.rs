//! Different unit tests to test the whole compilation chain.
//!
//! It is scoped as such to allow sharing code between test files.

#![allow(
    clippy::tests_outside_test_module,
    clippy::print_stderr,
    clippy::unwrap_used,
    reason = "test"
)]

extern crate alloc;

mod lineariser;
mod parser;
mod runner;
