//! Why a recipe could not be read or evaluated.

use std::fmt;

/// An error in a recipe or in the values given to it.
///
/// Messages are in English; ArchiSim translates them for the user, using
/// the parameter labels the recipe gives in Spanish.
#[derive(Debug, Clone, PartialEq)]
pub enum FamilyError {
    /// The text does not follow the language.
    Syntax { line: usize, message: String },
    /// A parameter has no value.
    MissingValue { param: String },
    /// A value was given for a parameter the family does not have.
    UnknownParam { param: String },
    /// A value of the wrong kind (a text for a length…).
    WrongKind {
        param: String,
        label: String,
        expected: &'static str,
    },
    /// A number outside the parameter's range.
    OutOfRange {
        param: String,
        label: String,
        value: f64,
        min: f64,
        max: f64,
    },
    /// A fractional number for an integer parameter.
    NotInteger {
        param: String,
        label: String,
        value: f64,
    },
    /// A choice that is not one of the parameter's options.
    UnknownOption {
        param: String,
        label: String,
        value: String,
    },
    /// A `require` rule of the recipe is not met; the message is the
    /// recipe's own (in Spanish).
    Requirement { line: usize, message: String },
    /// A piece would have a negative size: the values leave no room for it.
    NegativeSize { line: usize },
    /// A formula failed (division by zero, unknown name, wrong kind…).
    Eval { line: usize, message: String },
    /// The recipe would generate too much geometry or never finish.
    TooComplex { line: usize },
}

impl fmt::Display for FamilyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FamilyError::Syntax { line, message } => write!(f, "line {line}: {message}"),
            FamilyError::MissingValue { param } => write!(f, "parameter {param} has no value"),
            FamilyError::UnknownParam { param } => {
                write!(f, "the family has no parameter {param}")
            }
            FamilyError::WrongKind {
                param, expected, ..
            } => write!(f, "parameter {param} must be a {expected}"),
            FamilyError::OutOfRange {
                param,
                value,
                min,
                max,
                ..
            } => write!(f, "{param} = {value} is outside its range {min}..{max}"),
            FamilyError::NotInteger { param, value, .. } => {
                write!(f, "{param} = {value} must be a whole number")
            }
            FamilyError::UnknownOption { param, value, .. } => {
                write!(f, "\"{value}\" is not an option of {param}")
            }
            FamilyError::Requirement { line, message } => write!(f, "line {line}: {message}"),
            FamilyError::NegativeSize { line } => {
                write!(f, "line {line}: a piece would have a negative size")
            }
            FamilyError::Eval { line, message } => write!(f, "line {line}: {message}"),
            FamilyError::TooComplex { line } => {
                write!(f, "line {line}: the recipe generates too much geometry")
            }
        }
    }
}

impl std::error::Error for FamilyError {}

pub(crate) fn syntax(line: usize, message: impl Into<String>) -> FamilyError {
    FamilyError::Syntax {
        line,
        message: message.into(),
    }
}
