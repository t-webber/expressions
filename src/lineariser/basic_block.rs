//! Defines the basic block logic, the elementary logic block of the
//! [`Ssa`](super::ssa::Ssa).

use crate::EMPTY;
use crate::lineariser::state::LState;
use crate::lineariser::types::Type;
use crate::parser::api::BracedBlock;
use crate::utils::display;

/// List of instructions that can exist in a basic block.
#[derive(Debug)]
pub enum Instruction {
    /// `return <expr>`
    Return(usize),
}

/// Id wrapper to avoid stopping when calling undeclared variables.
///
/// It prefers proceeding to try and find more errors.
#[derive(Debug)]
pub enum Id {
    /// Variable was found, and the payload is the unique id.
    Found(usize, Type),
    /// Variable not found.
    NotFound,
}

display!(
    Instruction,
    self,
    f,
    match self {
        Self::Return(lit) => write!(f, "return x{lit}"),
    }
);

/// List of basic blocks, that materialise a function body.
#[derive(Debug, Default)]
pub struct BasicBlocks(Vec<Vec<Instruction>>);

impl BasicBlocks {
    /// Adds a line to the last basic block
    pub fn add(&mut self, inst: Instruction) {
        if let Some(last) = self.0.last_mut() {
            last.push(inst);
        } else {
            self.0.push(vec![inst]);
        }
    }

    /// Creates a new basic block from the given braced block.
    pub fn from_braced_block(body: BracedBlock, state: &mut LState) -> Self {
        let mut this = Self(vec![]);
        for ast in body.elts {
            ast.push_in(&mut this, state);
        }
        this
    }
}

display!(
    BasicBlocks,
    self,
    f,
    if self.0.is_empty() {
        write!(f, " {EMPTY}")
    } else {
        for (id, bb) in self.0.iter().enumerate() {
            write!(f, "\n{:2}BB{id}:", "")?;
            for inst in bb {
                write!(f, "\n{:4}{inst}", "")?;
            }
        }
        Ok(())
    }
);
