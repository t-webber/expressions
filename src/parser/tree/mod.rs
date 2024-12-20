use core::fmt;
use core::mem;
pub mod binary;
pub mod unary;
use binary::Binary;
use unary::Unary;

use crate::lexer::api::types::Number;

pub trait Operator: fmt::Debug {
    fn precedence(&self) -> u32;
    fn associativity(&self) -> Associativity;
}

pub trait AddArgument: Into<Node> {
    fn add_argument(&mut self, arg: Node) -> bool;
}

pub trait TakeOperator<T: AddArgument> {
    fn take_operator(self) -> T;
}

#[derive(Debug, PartialEq, Eq)]
pub enum Associativity {
    LeftToRight,
    RightToLeft,
}

#[derive(Debug, PartialEq)]
pub struct CompoundLiteral {
    args: Vec<Node>,
    operator: CompoundLiteralOperator,
    type_: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CompoundLiteralOperator;

impl Operator for CompoundLiteralOperator {
    fn associativity(&self) -> Associativity {
        Associativity::LeftToRight
    }

    fn precedence(&self) -> u32 {
        1
    }
}

#[derive(Debug, PartialEq)]
pub struct FunctionCall {
    name: String,
    operator: FunctionOperator,
    args: Vec<Node>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FunctionOperator;

impl Operator for FunctionOperator {
    fn associativity(&self) -> Associativity {
        Associativity::LeftToRight
    }

    fn precedence(&self) -> u32 {
        1
    }
}

#[derive(Debug, PartialEq, Default)]
pub enum Literal {
    #[default]
    Empty,
    String(String),
    Variable(String),
    Char(char),
    Str(String),
    Number(Number),
}

#[derive(Debug, Default, PartialEq)]
pub enum Node {
    //Todo blocks are only authorised at higher levels
    #[default]
    Empty,
    Binary(Binary),
    CompoundLiteral(CompoundLiteral),
    FunctionCall(FunctionCall),
    Leaf(Literal),
    Ternary(Ternary),
    Unary(Unary),
    Vec(Vec<Node>),
    Block(Vec<Node>),
}

impl Node {
    /// This functions returns Err if two many arguments were provided,
    /// like in the expression: `a+b c`.
    pub fn push_node_as_leaf(&mut self, node: Self) -> Result<(), &'static str> {
        match self {
            Self::Empty => *self = node,
            // push in Option<Box<Node>>
            Self::Binary(
                Binary {
                    arg_l: last @ None, ..
                }
                | Binary {
                    arg_l: Some(_),
                    arg_r: last @ None,
                    ..
                },
            )
            | Self::Ternary(
                Ternary {
                    condition: last @ None,
                    ..
                }
                | Ternary {
                    success: last @ None,
                    ..
                }
                | Ternary {
                    failure: last @ None,
                    ..
                },
            )
            | Self::Unary(Unary {
                arg: last @ None, ..
            }) => *last = Some(Box::new(node)),
            // push in Vec<Node>
            Self::Block(vec) => vec.push(node),
            // todo
            Self::Vec(_) | Self::FunctionCall(_) | Self::CompoundLiteral(_) => todo!(),
            // Errors
            Self::Leaf(_) => {
                return Err("Found 2 consecutive litteral without a logical relation.")
            }
            Self::Unary(_) => {
                return Err("Found 2 arguments for a unary operator. Did you forget an operator?")
            }
            Self::Binary(_) => {
                return Err("Found 3 arguments for a binary operator. Did you forget an operator?")
            }
            Self::Ternary(_) => {
                return Err(
                    "Found 4 arguments for the ternary operator. Did you forget an operator?",
                )
            }
        };
        Ok(())
    }

    pub fn take_last_leaf(&mut self) -> Option<Literal> {
        match self {
            node @ Self::Leaf(_) => {
                if let Self::Leaf(leaf) = mem::replace(node, Self::Empty) {
                    Some(leaf)
                } else {
                    panic!("never happens because old is leaf...")
                }
            }
            Self::Binary(Binary {
                arg_r: Some(child), ..
            })
            | Self::Ternary(
                Ternary {
                    failure: Some(child),
                    ..
                }
                | Ternary {
                    success: Some(child),
                    ..
                }
                | Ternary {
                    condition: Some(child),
                    ..
                },
            )
            | Self::Unary(Unary {
                arg: Some(child), ..
            }) => child.take_last_leaf(),
            Self::Block(vec) => vec.last_mut().and_then(Self::take_last_leaf),
            // todo
            Self::Vec(_) | Self::FunctionCall(_) | Self::CompoundLiteral(_) => todo!(),
            // Errors
            Self::Empty | Self::Binary(_) | Self::Ternary(_) | Self::Unary(_) => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == Self::Empty
    }

    pub fn push_op<U: AddArgument, T: Operator + TakeOperator<U> + Into<Self>>(
        &mut self,
        operator: T,
    ) -> Result<(), &'static str> {
        //TODO: this doesn't work for cast, sizeof and alignof
        match operator.associativity() {
            Associativity::LeftToRight => match self.take_last_leaf() {
                None => {
                    // This is error is never printed, because the only left-to-right operators are postfix increments, and those are catched.
                    return Err(
                        "Found left-to-right unary operator, without having a leaf before.",
                    );
                }
                Some(leaf) => {
                    let mut new_leaf = operator.take_operator();
                    new_leaf.add_argument(Self::Leaf(leaf));
                    self.push_node_as_leaf(new_leaf.into())?;
                }
            },
            Associativity::RightToLeft => {
                if self.push_node_as_leaf(operator.into()).is_err() {
                    // Example: `int c = a+b!;`
                    return Err(
                        "Found right-to-left unary operator, within a context not waiting for leaf.",
                    );
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct Ternary {
    pub(super) operator: TernaryOperator,
    pub(super) condition: Option<Box<Node>>,
    pub(super) success: Option<Box<Node>>,
    pub(super) failure: Option<Box<Node>>,
}

impl From<Ternary> for Node {
    fn from(val: Ternary) -> Self {
        Self::Ternary(val)
    }
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct TernaryOperator;

impl Operator for TernaryOperator {
    fn associativity(&self) -> Associativity {
        Associativity::RightToLeft
    }

    fn precedence(&self) -> u32 {
        13
    }
}
