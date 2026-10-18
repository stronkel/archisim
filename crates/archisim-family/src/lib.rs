//! ArchiSim family recipes (`.asfam`).
//!
//! A family is a readable text file that describes a parametric building
//! element (a door, a window, a piece of furniture): its parameters, the
//! formulas between them, its 3D body built from simple pieces, the values
//! the rest of the program reads (rough opening, frame depth…) and its IFC
//! data. It plays the role of a Revit `.rfa` family or an ArchiCAD GDL
//! object, with a language of its own.
//!
//! This crate knows nothing about the building model: it parses recipes and
//! evaluates them for given parameter values. Materials are opaque handles
//! ([`Value::Material`]) chosen by the caller.
//!
//! ```text
//! family "Puerta corredera"
//!   category door
//!   host wall
//!   version 1
//!
//! type_params
//!   section "Dimensiones"
//!     width  length = 0.80 range 0.40..3.00 label "Hueco de obra: ancho"
//!
//! body
//!   box (-width / 2, -0.02, 0) to (width / 2, 0.02, 2.0) material leaf
//! ```
//!
//! See `docs/FAMILIAS.md` for the full language.

mod ast;
mod error;
mod eval;
mod lexer;
mod parser;

use std::collections::BTreeMap;
use std::fmt;

pub use ast::{
    BinOp, Component, Expr, FamilyDef, Host, Param, ParamKind, ParamScope, Require, Stmt,
};
pub use error::FamilyError;
pub use eval::{Evaluation, Part, MAX_PARTS, MAX_REPEAT};
pub use parser::{expand_includes, parse};

/// A parameter or expression value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Lengths (metres), angles (radians), numbers and integers.
    Number(f64),
    Bool(bool),
    /// Texts and the chosen option of a choice parameter.
    Text(String),
    /// A material, as an opaque handle chosen by the caller (ArchiSim uses
    /// its material ids).
    Material(u64),
}

impl Value {
    /// The kind of value, as written in error messages.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "number",
            Value::Bool(_) => "yes/no",
            Value::Text(_) => "text",
            Value::Material(_) => "material",
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_material(&self) -> Option<u64> {
        match self {
            Value::Material(m) => Some(*m),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{n}"),
            Value::Bool(true) => f.write_str("yes"),
            Value::Bool(false) => f.write_str("no"),
            Value::Text(t) => write!(f, "\"{t}\""),
            Value::Material(m) => write!(f, "material #{m}"),
        }
    }
}

/// Parameter values by parameter name.
pub type Values = BTreeMap<String, Value>;

#[cfg(test)]
mod tests;
