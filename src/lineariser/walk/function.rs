//! Walks a function call, declaration or definition, updating state and
//! creating symbols and basic blocks.

use crate::errors::api::Located;
use crate::lineariser::basic_block::{BasicBlocks, Id};
use crate::lineariser::state::LState;
use crate::lineariser::symbol::Value;
use crate::lineariser::types::{ReturnType, Type};
use crate::parser::api::{Ast, BracedBlock, FunctionCall, VariableName, VariableValue};

impl FunctionCall {
    /// Pushes some content into the [`BasicBlocks`].
    pub fn push_in(self, bbs: &mut BasicBlocks, state: &mut LState) -> Option<Id> {
        #[cfg(feature = "debug")]
        crate::lgp!(notab: "Pushing fn {self}");
        let Self { mut arguments, function_body, variable, .. } = self;
        match variable.into_value() {
            VariableValue::AttributeVariable(attr) => {
                let loc = attr.location();
                let (var, attrs) = attr.into_single_variable();
                if let Some(name) = var {
                    declare_function(
                        name,
                        arguments,
                        state.store_errors(ReturnType::from_attributes(&attrs)),
                        function_body,
                        state,
                    );
                } else {
                    state.push_error(
                        loc.fail("Found illegal comma in function declaration".to_owned()),
                    );
                }
                None
            }
            VariableValue::VariableName(loc, VariableName::UserDefined(name))
                if function_body.is_some() =>
            {
                state.push_error(loc.fail(format!("Missing return type for function {name}")));
                declare_function(
                    loc.wrap(name),
                    arguments,
                    ReturnType::empty(),
                    function_body,
                    state,
                );
                None
            }
            VariableValue::VariableName(loc, VariableName::Keyword(kwd))
                if function_body.is_some() =>
            {
                state.push_error(loc.fail(format!(
                    "Attempt to declare function with an invalid name, `{kwd}` is a keyword"
                )));
                None
            }
            VariableValue::VariableName(varloc, VariableName::UserDefined(name)) => {
                if let Some(func) = state.find_function(&name) {
                    let ret = func.ret.clone();
                    let fid = func.id;
                    let mut args = vec![];
                    let mut has_errors = false;
                    for arg in arguments {
                        let argloc = arg.location();
                        match arg.push_in(bbs, state) {
                            Some(Id::Found(id, _)) => args.push(id),
                            Some(Id::NotFound) => has_errors = true,
                            None => {
                                state.stat_not_expr(argloc, "function argument");
                                has_errors = true;
                            }
                        }
                    }
                    if has_errors {
                        Some(Id::NotFound)
                    } else {
                        let ty = ret.into_type();
                        Some(Id::Found(state.push_element(Value::Call(fid, args), ty.clone()), ty))
                    }
                } else {
                    state.push_error(varloc.fail(format!("Call of undeclared function {name}")));
                    Some(Id::NotFound)
                }
            }
            VariableValue::VariableName(loc, VariableName::Keyword(kwd)) => {
                if arguments.len() > 1 {
                    state.push_error(loc.fail(format!(
                        "Too many arguments in call to `{kwd}`: expected 1, got {}",
                        arguments.len()
                    )));
                    return None;
                }
                let Some(_) = arguments.pop() else {
                    state.push_error(
                        loc.fail(format!("Missing argument in call to `{kwd}`: expected 1, got 0")),
                    );
                    return None;
                };
                state.push_error(loc.fail(format!("Function keuword {kwd} is not yet supported")));
                Some(Id::NotFound)
            }
        }
    }
}

/// Declares a function with the given signature.
fn declare_function(
    name: Located<String>,
    arguments: Vec<Ast>,
    ret: ReturnType,
    body: Option<BracedBlock>,
    state: &mut LState,
) {
    #[cfg(feature = "debug")]
    crate::lgp!(notab: "Declaring fn {}", name.as_value());
    let mut args = vec![];
    for ast in arguments {
        if let Ast::Variable(arg_var) = ast {
            match arg_var.into_value() {
                VariableValue::AttributeVariable(arg_attr) => {
                    let loc = arg_attr.location();
                    let (arg, attrs) = arg_attr.into_single_variable();
                    let ty = state.store_errors(Type::from_attributes(&attrs));
                    if let Some(arg_name) = arg {
                        args.push((arg_name, ty));
                    } else {
                        state.push_error(loc.fail("Missing argument name".to_owned()));
                        args.push((loc.wrap(String::new()), ty));
                    }
                }
                VariableValue::VariableName(loc, arg_name) => {
                    state.push_error(loc.fail("Missing argument type".to_owned()));
                    match arg_name {
                        VariableName::Keyword(_) => {
                            state.push_error(
                                loc.fail("Invalid argument name, shadows keyword.".to_owned()),
                            );
                            args.push((loc.wrap(String::new()), Type::empty()));
                        }
                        VariableName::UserDefined(vname) =>
                            args.push((loc.wrap(vname), Type::empty())),
                    }
                }
            }
        } else {
            let loc = ast.location();
            state.push_error(loc.fail("Expected argument declaration".to_owned()));
            args.push((loc.wrap(String::new()), Type::empty()));
        }
    }
    state.push_function(name, args, ret, body);
}
