//! Module to store the errors while still returning a result.
//!
//! This crate implements the [`Res`] struct and its methods.

extern crate alloc;
use alloc::vec;
use core::{convert, ops};
use std::process::exit;

use super::compile::CompileError;
use super::display::display_errors;
use crate::errors::compile::CompileErrorList;

/// [`Result`] alias for [`CompileError`]
pub type CompileRes<T> = Result<T, CompileError>;

/// Struct to store the errors, whilst still having the desired value.
///
/// This struct is meant as a [`Result`], but were it is possible to
/// have a value and some errors at the same time. It is for example the case
/// for warnings and suggestions that must be stored, and at the
/// same time, the compiler continues to work. Please refer to
/// [`Res::as_displayed_errors`] to get a pretty stringified version of these
/// errors.
// TODO: instead of T: default, Res = Fatal(errors) | NonFatal(T, warnings)
#[derive(Debug)]
pub struct Res<T> {
    /// The errors that occurred
    errors: CompileErrorList,
    /// The desired result
    result: Option<T>,
}

impl<T> Res<T> {
    /// Adds an error to a current [`Res`]
    pub(crate) fn add_err(mut self, error: CompileError) -> Self {
        self.errors.0.push(error);
        self
    }

    /// Adds an error to a current [`Res`] if the condition is true.
    pub(crate) fn add_err_cond(mut self, cond: bool, error: CompileError) -> Self {
        if cond {
            self.errors.0.push(error);
        }
        self
    }

    /// Adds an error to a current [`Res`]
    pub(crate) fn add_err_opt(mut self, error: Option<CompileError>) -> Self {
        if let Some(err) = error {
            self.errors.0.push(err);
        }
        self
    }

    /// Adds an error to a current [`Res`]
    pub(crate) fn add_errs(mut self, errors: Vec<CompileError>) -> Self {
        self.errors.0.extend(errors);
        self
    }

    /// Applies a function to the result, if it has a result.
    pub fn and_then<U, F: FnOnce(T) -> Res<U>>(mut self, func: F) -> Res<U> {
        if let Some(old) = self.result {
            let res = func(old);
            self.errors.0.extend(res.errors.0);
            Res { errors: self.errors, result: res.result }
        } else {
            Res { errors: self.errors, result: None }
        }
    }

    /// Returns all the errors in a user-readable format.
    ///
    /// # Returns
    ///
    /// A [`String`] containing all the errors, displayed in a user-readable
    /// format, with a clickable location, and an explanation message.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::fs;
    ///
    /// use c_parser::lex;
    ///
    /// let content = "int m@in() { }";
    /// let filename = "filename.c";
    /// let res = lex(content, 0);
    /// let (_, errors) = res.as_displayed_errors(&[(0, filename, content)]);
    /// let expected = "filename.c:1:6: error: Character '@' not supported.
    ///     1 | int m@in() { }
    ///              ^
    /// ";
    ///
    /// assert!(errors == expected, "!{errors}!\n!{expected}!");
    /// ```
    ///
    /// # Panics
    ///
    /// If there are too many errors, a buffer overflow occurs
    pub fn as_displayed_errors(self, files: &[(u32, &str, &str)]) -> (Option<T>, String) {
        (
            self.result,
            display_errors(&self.errors, files).expect("Buffer overflow, failed to fetch errors"),
        )
    }

    /// Checks if the ``errors`` field is empty
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(c_parser::Res::ok(0).errors_empty() == true);
    /// ```
    ///
    /// ```ignore
    /// assert!(Res::from_errs(vec![]).errors_empty() == true);
    /// ```
    pub const fn errors_empty(&self) -> bool {
        self.errors.0.is_empty()
    }

    /// Adds errors to a current [`Res`]
    pub(crate) fn extend_errs(mut self, errors: Vec<CompileError>) -> Self {
        self.errors.0.extend(errors);
        self
    }

    /// Returns a [`Res`] with an error but no value.
    #[must_use]
    pub fn from_err(err: CompileError) -> Self {
        Self { errors: vec![err].into(), result: None }
    }

    /// Checks if the [`Res`] contains critical failures.
    pub(crate) fn has_failures(&self) -> bool {
        self.errors.0.iter().any(CompileError::is_failure)
    }

    /// Applies a function to the result, if it has a result.
    pub fn map<U, F: FnOnce(T) -> U>(self, func: F) -> Res<U> {
        Res { errors: self.errors, result: self.result.map(func) }
    }

    /// Returns a [`Res`] with a value and no errors.
    pub fn ok(value: T) -> Self {
        Self { errors: vec![].into(), result: Some(value) }
    }

    /// Stores the errors with a function and returns the value
    pub(crate) fn store_errors<F: FnMut(CompileError)>(self, store: &mut F) -> Option<T> {
        for err in self.errors.0 {
            store(err);
        }
        self.result
    }

    /// Prints all the errors to the user.
    ///
    /// # Returns
    ///
    /// The value of the [`Res`] if there aren't any errors of level `Failure`.
    ///
    /// # Panics
    ///
    /// If there is at least one error of level `Failure`.
    #[coverage(off)]
    #[expect(clippy::print_stderr, reason = "goal of function")]
    pub fn unwrap_or_display(self, files: &[(u32, &str, &str)]) -> Option<T> {
        let has_failures = self.has_failures();
        let (result, display) = self.as_displayed_errors(files);
        eprint!("{display}");
        if has_failures {
            exit(1);
        }
        result
    }
}

impl<T> ops::FromResidual<Result<convert::Infallible, CompileError>> for Res<T> {
    fn from_residual(residual: Result<convert::Infallible, CompileError>) -> Self {
        match residual {
            Ok(_) => unreachable!(/* By definition of Infallible */),
            Err(err) => Self::from_err(err),
        }
    }
}

impl<T, F> From<(T, F)> for Res<T>
where
    CompileErrorList: From<F>,
{
    fn from(value: (T, F)) -> Self {
        Self { errors: value.1.into(), result: Some(value.0) }
    }
}
