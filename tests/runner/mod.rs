//! Defines the logic to test and bless code.

#![allow(clippy::non_ascii_literal, reason = "visual alignment")]
#![allow(clippy::panic, reason = "test")]

pub mod macros;
pub mod run;
pub mod store;

use std::env::var;
use std::sync::{LazyLock, Mutex};

use crate::runner::run::TestScope;
use crate::runner::store::Tests;

macro_rules! declare {
        ($($pascal:ident, $str:expr)*) => {
            $(const $pascal: &str = $str;)*
        };
    }

declare!(
CONTENTS, " contents "
EXPECTED, " expected "
COMPUTED, " computed "
__DIFF__, "── diff ──"
________, "──────────"
_TOKENS_, "─ tokens ─"
_PARSED_, "── tree ──"
_LINEAR_, "── ssa ───"
//
SIDE, "\x1b[33m───────────────"
C0, "\x1b[0m"
EOL, "\x1b[38;5;240m¤\x1b[0m\n"
CC, "\x1b[32m"
CE, "\x1b[31m"
);

static FS_VALUES: LazyLock<Mutex<Tests>> = LazyLock::new(|| Mutex::from(Tests::load()));

pub fn test(module_name: &str, test_name: &str, content: &str, scope: TestScope) {
    let computed = scope.run(content).replace("..", "");
    eprint!("{SIDE}{COMPUTED}{SIDE}{C0}\n{}{EOL}", computed.replace('\n', EOL));

    let key = format!("{module_name}::{test_name}");

    let pin = var("CARGO_PIN").unwrap_or_default();
    if pin == "pin" {
        let mut lock = FS_VALUES.lock().unwrap();
        lock.set(key, &computed);
        lock.store();
        drop(lock);
        return;
    }

    let expected = if pin == "unpin" {
        let mut lock = FS_VALUES.lock().unwrap();
        lock.remove(&key);
        lock.store();
        drop(lock);
        None
    } else if !pin.is_empty() {
        panic!(
            "\x1b[31mInvalid value for CARGO_PIN environenment variable: {pin}. Please stick to using `cargo test`, `cargo pin` or `cargo unpin`.\x1b[0m"
        )
    } else {
        let binding = FS_VALUES.lock().unwrap();
        let expected = binding.get(&key).map(str::to_owned);
        drop(binding);
        expected
    }.map(|inner| inner.replace("..", ""));

    let Some(exp) = expected else {
        let msg = "\x1b[31mNo expected output provided, use `cargo pin` to use current computed output as expected test output.\x1b[0m";

        if cfg!(feature = "no_test_fail") {
            eprintln!("{msg}");
            return;
        }
        panic!("{msg}");
    };

    eprintln!("{SIDE}{EXPECTED}{SIDE}{C0}");
    eprint!("{}{EOL}", exp.replace('\n', EOL));
    if exp == computed {
        return;
    }

    eprintln!("{SIDE}{__DIFF__}{SIDE}{C0}");
    let mut e_lines = exp.lines();
    let mut c_lines = computed.lines();

    loop {
        match (e_lines.next(), c_lines.next()) {
            (Some(el), Some(cl)) if el == cl => eprint!(" │{C0}{el}{EOL}"),
            (None, None) =>
                if cfg!(feature = "no_test_fail") {
                    return;
                } else {
                    panic!()
                },
            (Some(el), None) => eprint!("{CE}e│{el}{EOL}"),
            (None, Some(cl)) => eprint!("{CC}c│{cl}{EOL}"),
            (Some(el), Some(cl)) => {
                let mut e_in = el.chars();
                let mut c_in = cl.chars();
                let mut e_out = String::new();
                let mut c_out = String::new();
                loop {
                    match (e_in.next(), c_in.next()) {
                        (None, None) => break,
                        (None, Some(ch)) => {
                            c_out.push_str(CC);
                            c_out.push(ch);
                            break;
                        }
                        (Some(ch), None) => {
                            e_out.push_str(CE);
                            e_out.push(ch);
                            break;
                        }
                        (Some(e_ch), Some(c_ch)) => {
                            let eq = e_ch == c_ch;
                            e_out.push_str(if eq { C0 } else { CE });
                            e_out.push(if e_ch == ' ' && !eq { '⍽' } else { e_ch });
                            c_out.push_str(if eq { C0 } else { CC });
                            c_out.push(if c_ch == ' ' && !eq { '⍽' } else { c_ch });
                        }
                    }
                }
                c_out.extend(c_in);
                e_out.extend(e_in);
                eprint!("{CC}c│{c_out}{EOL}");
                eprint!("{CE}e│{e_out}{EOL}");
            }
        }
    }
}
