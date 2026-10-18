//! Evaluates a recipe for given parameter values.

use std::collections::HashMap;

use crate::ast::*;
use crate::error::FamilyError;
use crate::{Value, Values};

/// Most pieces a family may generate.
pub const MAX_PARTS: usize = 20_000;
/// Most turns of one `repeat`.
pub const MAX_REPEAT: i64 = 1_000;
/// Most instructions run in one evaluation (guards against runaway
/// recipes).
const MAX_STEPS: usize = 2_000_000;
/// Deepest chain of pieces using pieces.
const MAX_DEPTH: usize = 32;

/// One box of the 3D body, in the family's own axes (metres): `x` along the
/// host wall, `y` across it towards the side the leaves open to, `z` up
/// from the sill. `min` ≤ `max` on every axis.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub material: u64,
    pub min: [f64; 3],
    pub max: [f64; 3],
    /// Recipe line that made it.
    pub line: usize,
}

/// One mark of a plan symbol, in the family's own axes (metres, `x` and
/// `y` as for [`Part`]).
#[derive(Debug, Clone, PartialEq)]
pub enum Mark {
    Line {
        from: [f64; 2],
        to: [f64; 2],
        pen: Pen,
        dashed: bool,
    },
    /// Closed outline; `filled` hides what is drawn under it (white).
    Polygon {
        points: Vec<[f64; 2]>,
        pen: Pen,
        filled: bool,
    },
    /// Arc around `center` from `from` to `to`, the short way; the radius
    /// is the distance from the centre to `from`.
    Arc {
        center: [f64; 2],
        from: [f64; 2],
        to: [f64; 2],
        pen: Pen,
    },
}

/// The result of evaluating a family.
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    /// The 3D body, in the order written.
    pub parts: Vec<Part>,
    /// The values of the `outputs` section.
    pub outputs: Values,
    /// The values of the `ifc` section.
    pub ifc: Values,
}

impl FamilyDef {
    /// The default value of every parameter of one scope. Materials come as
    /// their name ([`Value::Text`]) for the caller to resolve.
    pub fn defaults(&self, scope: ParamScope) -> Values {
        self.params_of(scope)
            .map(|p| (p.name.clone(), p.default.clone()))
            .collect()
    }

    /// Checks a complete set of values for one scope: every parameter has a
    /// value of its kind, inside its range, and nothing else is given.
    pub fn check_values(&self, scope: ParamScope, values: &Values) -> Result<(), FamilyError> {
        for name in values.keys() {
            if !self.params_of(scope).any(|p| &p.name == name) {
                return Err(FamilyError::UnknownParam {
                    param: name.clone(),
                });
            }
        }
        for p in self.params_of(scope) {
            let v = values
                .get(&p.name)
                .ok_or_else(|| FamilyError::MissingValue {
                    param: p.name.clone(),
                })?;
            check_value(p, v)?;
        }
        Ok(())
    }

    /// Evaluates the family: checks the values, runs the `require` rules,
    /// builds the 3D body and computes the outputs. `builtins` are the
    /// values the program provides (see [`crate::parser`]'s `BUILTINS`).
    pub fn evaluate(
        &self,
        type_values: &Values,
        instance_values: &Values,
        builtins: &Values,
    ) -> Result<Evaluation, FamilyError> {
        self.check_values(ParamScope::Type, type_values)?;
        self.check_values(ParamScope::Instance, instance_values)?;
        let mut base: HashMap<String, Value> = HashMap::new();
        for (k, v) in builtins.iter().chain(type_values).chain(instance_values) {
            base.insert(k.clone(), v.clone());
        }
        let mut run = Run::new(self, Mode::Body);
        let mut env = vec![base];
        run.exec(&self.prelude, &mut env)?;
        let outputs = run.assign(&self.outputs, &mut env)?;
        let ifc = run.assign(&self.ifc, &mut env)?;
        env.push(HashMap::new());
        run.exec(&self.body, &mut env)?;
        Ok(Evaluation {
            parts: run.parts,
            outputs,
            ifc,
        })
    }

    /// Draws the plan symbol for a scale (the nearest one written, see
    /// [`FamilyDef::plan_for`]). `builtins` are those of
    /// [`FamilyDef::evaluate`] plus, for wall-hosted families, the plan ones
    /// (see [`crate::PLAN_BUILTINS`]). A family without plan symbols draws
    /// nothing.
    pub fn evaluate_plan(
        &self,
        type_values: &Values,
        instance_values: &Values,
        builtins: &Values,
        scale: f64,
    ) -> Result<Vec<Mark>, FamilyError> {
        let Some(symbol) = self.plan_for(scale) else {
            return Ok(Vec::new());
        };
        self.check_values(ParamScope::Type, type_values)?;
        self.check_values(ParamScope::Instance, instance_values)?;
        let mut base: HashMap<String, Value> = HashMap::new();
        for (k, v) in builtins.iter().chain(type_values).chain(instance_values) {
            base.insert(k.clone(), v.clone());
        }
        let mut run = Run::new(self, Mode::Plan);
        let mut env = vec![base];
        run.exec(&self.prelude, &mut env)?;
        env.push(HashMap::new());
        run.exec(&symbol.body, &mut env)?;
        Ok(run.marks)
    }

    /// Whether a parameter is shown in the settings dialog for these values
    /// (its `visible_if`). Missing values take their default; a condition
    /// that cannot be evaluated shows the parameter.
    pub fn is_visible(&self, name: &str, values: &Values) -> bool {
        let Some(p) = self.param(name) else {
            return false;
        };
        let Some(cond) = &p.visible_if else {
            return true;
        };
        let mut base: HashMap<String, Value> = self
            .params
            .iter()
            .map(|p| (p.name.clone(), p.default.clone()))
            .collect();
        base.extend(values.iter().map(|(k, v)| (k.clone(), v.clone())));
        let mut run = Run::new(self, Mode::Body);
        !matches!(run.eval(cond, &[base], p.line), Ok(Value::Bool(false)))
    }
}

fn check_value(p: &Param, v: &Value) -> Result<(), FamilyError> {
    let wrong = || FamilyError::WrongKind {
        param: p.name.clone(),
        label: p.label.clone(),
        expected: p.kind.name(),
    };
    match (&p.kind, v) {
        (k, Value::Number(n)) if k.is_numeric() => {
            if !n.is_finite() {
                return Err(wrong());
            }
            if *k == ParamKind::Integer && n.fract() != 0.0 {
                return Err(FamilyError::NotInteger {
                    param: p.name.clone(),
                    label: p.label.clone(),
                    value: *n,
                });
            }
            if let Some((min, max)) = p.range {
                // A little tolerance for values typed in millimetres.
                if *n < min - 1e-9 || *n > max + 1e-9 {
                    return Err(FamilyError::OutOfRange {
                        param: p.name.clone(),
                        label: p.label.clone(),
                        value: *n,
                        min,
                        max,
                    });
                }
            }
            Ok(())
        }
        (ParamKind::Bool, Value::Bool(_))
        | (ParamKind::Material, Value::Material(_))
        | (ParamKind::Text, Value::Text(_)) => Ok(()),
        (ParamKind::Choice(options), Value::Text(t)) => {
            if options.contains(t) {
                Ok(())
            } else {
                Err(FamilyError::UnknownOption {
                    param: p.name.clone(),
                    label: p.label.clone(),
                    value: t.clone(),
                })
            }
        }
        _ => Err(wrong()),
    }
}

type Env = Vec<HashMap<String, Value>>;

/// What is being evaluated: the 3D body or a plan symbol.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Body,
    Plan,
}

struct Run<'a> {
    def: &'a FamilyDef,
    mode: Mode,
    parts: Vec<Part>,
    marks: Vec<Mark>,
    steps: usize,
    depth: usize,
}

fn eval_err(line: usize, message: impl Into<String>) -> FamilyError {
    FamilyError::Eval {
        line,
        message: message.into(),
    }
}

impl<'a> Run<'a> {
    fn new(def: &'a FamilyDef, mode: Mode) -> Self {
        Self {
            def,
            mode,
            parts: Vec::new(),
            marks: Vec::new(),
            steps: 0,
            depth: 0,
        }
    }

    fn point(
        &mut self,
        p: &[Expr; 2],
        env: &[HashMap<String, Value>],
        line: usize,
    ) -> Result<[f64; 2], FamilyError> {
        Ok([
            self.number(&p[0], env, line)?,
            self.number(&p[1], env, line)?,
        ])
    }

    fn push_mark(&mut self, mark: Mark, line: usize) -> Result<(), FamilyError> {
        if self.mode != Mode::Plan {
            return Err(eval_err(
                line,
                "lines, outlines and arcs go in a «plan» block, not in the body",
            ));
        }
        if self.marks.len() >= MAX_PARTS {
            return Err(FamilyError::TooComplex { line });
        }
        self.marks.push(mark);
        Ok(())
    }

    fn tick(&mut self, line: usize) -> Result<(), FamilyError> {
        self.steps += 1;
        if self.steps > MAX_STEPS {
            return Err(FamilyError::TooComplex { line });
        }
        Ok(())
    }

    fn assign(&mut self, list: &[Output], env: &mut Env) -> Result<Values, FamilyError> {
        let mut out = Values::new();
        for o in list {
            let v = self.eval(&o.expr, env, o.line)?;
            out.insert(o.name.clone(), v);
        }
        Ok(out)
    }

    fn exec(&mut self, stmts: &[Stmt], env: &mut Env) -> Result<(), FamilyError> {
        for s in stmts {
            match s {
                Stmt::Let { name, expr, line } => {
                    self.tick(*line)?;
                    let v = self.eval(expr, env, *line)?;
                    if env.iter().any(|f| f.contains_key(name)) {
                        return Err(eval_err(*line, format!("«{name}» already has a value")));
                    }
                    env.last_mut().expect("frame").insert(name.clone(), v);
                }
                Stmt::Require(r) => {
                    self.tick(r.line)?;
                    if !self.bool(&r.cond, env, r.line)? {
                        return Err(FamilyError::Requirement {
                            line: r.line,
                            message: r.message.clone(),
                        });
                    }
                }
                Stmt::Line {
                    from,
                    to,
                    pen,
                    dashed,
                    line,
                } => {
                    self.tick(*line)?;
                    let mark = Mark::Line {
                        from: self.point(from, env, *line)?,
                        to: self.point(to, env, *line)?,
                        pen: *pen,
                        dashed: *dashed,
                    };
                    self.push_mark(mark, *line)?;
                }
                Stmt::Polygon {
                    points,
                    pen,
                    filled,
                    line,
                } => {
                    self.tick(*line)?;
                    let mut pts = Vec::with_capacity(points.len());
                    for p in points {
                        pts.push(self.point(p, env, *line)?);
                    }
                    let mark = Mark::Polygon {
                        points: pts,
                        pen: *pen,
                        filled: *filled,
                    };
                    self.push_mark(mark, *line)?;
                }
                Stmt::Arc {
                    center,
                    from,
                    to,
                    pen,
                    line,
                } => {
                    self.tick(*line)?;
                    let mark = Mark::Arc {
                        center: self.point(center, env, *line)?,
                        from: self.point(from, env, *line)?,
                        to: self.point(to, env, *line)?,
                        pen: *pen,
                    };
                    self.push_mark(mark, *line)?;
                }
                Stmt::Box {
                    min,
                    max,
                    material,
                    line,
                } => {
                    self.tick(*line)?;
                    if self.mode != Mode::Body {
                        return Err(eval_err(*line, "a plan symbol draws lines, not boxes"));
                    }
                    let mut a = [0.0; 3];
                    let mut b = [0.0; 3];
                    for k in 0..3 {
                        a[k] = self.number(&min[k], env, *line)?;
                        b[k] = self.number(&max[k], env, *line)?;
                        if b[k] < a[k] - 1e-9 {
                            return Err(FamilyError::NegativeSize { line: *line });
                        }
                        b[k] = b[k].max(a[k]);
                    }
                    let material = match self.eval(material, env, *line)? {
                        Value::Material(m) => m,
                        other => {
                            return Err(eval_err(
                                *line,
                                format!("a box needs a material, not a {}", other.kind_name()),
                            ))
                        }
                    };
                    if self.parts.len() >= MAX_PARTS {
                        return Err(FamilyError::TooComplex { line: *line });
                    }
                    self.parts.push(Part {
                        material,
                        min: a,
                        max: b,
                        line: *line,
                    });
                }
                Stmt::Use { name, args, line } => {
                    self.tick(*line)?;
                    let comp = self
                        .def
                        .components
                        .get(name)
                        .ok_or_else(|| eval_err(*line, format!("no piece called «{name}»")))?;
                    let mut frame = HashMap::new();
                    for (arg, e) in comp.args.iter().zip(args) {
                        frame.insert(arg.clone(), self.eval(e, env, *line)?);
                    }
                    self.depth += 1;
                    if self.depth > MAX_DEPTH {
                        return Err(FamilyError::TooComplex { line: *line });
                    }
                    // A piece sees only its own arguments.
                    let mut inner = vec![frame];
                    let result = self.exec(&comp.body, &mut inner);
                    self.depth -= 1;
                    result?;
                }
                Stmt::Repeat {
                    var,
                    from,
                    to,
                    body,
                    line,
                } => {
                    let from = self.integer(from, env, *line)?;
                    let to = self.integer(to, env, *line)?;
                    if to - from > MAX_REPEAT {
                        return Err(FamilyError::TooComplex { line: *line });
                    }
                    for i in from..to {
                        self.tick(*line)?;
                        let mut frame = HashMap::new();
                        frame.insert(var.clone(), Value::Number(i as f64));
                        env.push(frame);
                        let result = self.exec(body, env);
                        env.pop();
                        result?;
                    }
                }
                Stmt::If {
                    cond,
                    then,
                    otherwise,
                    line,
                } => {
                    self.tick(*line)?;
                    let branch = if self.bool(cond, env, *line)? {
                        then
                    } else {
                        otherwise
                    };
                    env.push(HashMap::new());
                    let result = self.exec(branch, env);
                    env.pop();
                    result?;
                }
            }
        }
        Ok(())
    }

    fn number(
        &mut self,
        e: &Expr,
        env: &[HashMap<String, Value>],
        line: usize,
    ) -> Result<f64, FamilyError> {
        match self.eval(e, env, line)? {
            Value::Number(n) => Ok(n),
            other => Err(eval_err(
                line,
                format!("expected a number, found a {}", other.kind_name()),
            )),
        }
    }

    fn integer(
        &mut self,
        e: &Expr,
        env: &[HashMap<String, Value>],
        line: usize,
    ) -> Result<i64, FamilyError> {
        let n = self.number(e, env, line)?;
        if n.fract() != 0.0 || n.abs() > 1e12 {
            return Err(eval_err(line, format!("{n} is not a whole number")));
        }
        Ok(n as i64)
    }

    fn bool(
        &mut self,
        e: &Expr,
        env: &[HashMap<String, Value>],
        line: usize,
    ) -> Result<bool, FamilyError> {
        match self.eval(e, env, line)? {
            Value::Bool(b) => Ok(b),
            other => Err(eval_err(
                line,
                format!("expected yes or no, found a {}", other.kind_name()),
            )),
        }
    }

    fn eval(
        &mut self,
        e: &Expr,
        env: &[HashMap<String, Value>],
        line: usize,
    ) -> Result<Value, FamilyError> {
        let finite = |n: f64| {
            if n.is_finite() {
                Ok(Value::Number(n))
            } else {
                Err(eval_err(line, "the result is not a valid number"))
            }
        };
        match e {
            Expr::Num(n) => Ok(Value::Number(*n)),
            Expr::Str(s) => Ok(Value::Text(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Var(name) => env
                .iter()
                .rev()
                .find_map(|f| f.get(name))
                .cloned()
                .ok_or_else(|| eval_err(line, format!("unknown name «{name}»"))),
            Expr::Neg(a) => {
                let a = self.number(a, env, line)?;
                Ok(Value::Number(-a))
            }
            Expr::Not(a) => Ok(Value::Bool(!self.bool(a, env, line)?)),
            Expr::If(c, a, b) => {
                if self.bool(c, env, line)? {
                    self.eval(a, env, line)
                } else {
                    self.eval(b, env, line)
                }
            }
            Expr::Bin(op, a, b) => match op {
                BinOp::And => Ok(Value::Bool(
                    self.bool(a, env, line)? && self.bool(b, env, line)?,
                )),
                BinOp::Or => Ok(Value::Bool(
                    self.bool(a, env, line)? || self.bool(b, env, line)?,
                )),
                BinOp::Eq | BinOp::Ne => {
                    let (x, y) = (self.eval(a, env, line)?, self.eval(b, env, line)?);
                    if std::mem::discriminant(&x) != std::mem::discriminant(&y) {
                        return Err(eval_err(
                            line,
                            format!(
                                "cannot compare a {} with a {}",
                                x.kind_name(),
                                y.kind_name()
                            ),
                        ));
                    }
                    Ok(Value::Bool((x == y) == (*op == BinOp::Eq)))
                }
                BinOp::Add => {
                    // Numbers add up; texts join ("SIDEHUNG" + "LEFT").
                    match (self.eval(a, env, line)?, self.eval(b, env, line)?) {
                        (Value::Number(x), Value::Number(y)) => finite(x + y),
                        (Value::Text(x), Value::Text(y)) => Ok(Value::Text(x + &y)),
                        (x, y) => Err(eval_err(
                            line,
                            format!("cannot add a {} and a {}", x.kind_name(), y.kind_name()),
                        )),
                    }
                }
                _ => {
                    let x = self.number(a, env, line)?;
                    let y = self.number(b, env, line)?;
                    match op {
                        BinOp::Sub => finite(x - y),
                        BinOp::Mul => finite(x * y),
                        BinOp::Div => {
                            if y == 0.0 {
                                return Err(eval_err(line, "division by zero"));
                            }
                            finite(x / y)
                        }
                        BinOp::Lt => Ok(Value::Bool(x < y)),
                        BinOp::Le => Ok(Value::Bool(x <= y)),
                        BinOp::Gt => Ok(Value::Bool(x > y)),
                        BinOp::Ge => Ok(Value::Bool(x >= y)),
                        _ => unreachable!("handled above"),
                    }
                }
            },
            Expr::Call(name, args) => {
                let mut nums = Vec::with_capacity(args.len());
                for a in args {
                    nums.push(self.number(a, env, line)?);
                }
                let one = |nums: &[f64]| -> Result<f64, FamilyError> {
                    match nums {
                        [x] => Ok(*x),
                        _ => Err(eval_err(line, format!("«{name}» takes one value"))),
                    }
                };
                let r = match name.as_str() {
                    "min" | "max" => {
                        let Some(&first) = nums.first() else {
                            return Err(eval_err(line, format!("«{name}» needs values")));
                        };
                        nums.iter().fold(first, |acc, &x| {
                            if name == "min" {
                                acc.min(x)
                            } else {
                                acc.max(x)
                            }
                        })
                    }
                    "abs" => one(&nums)?.abs(),
                    "sqrt" => {
                        let x = one(&nums)?;
                        if x < 0.0 {
                            return Err(eval_err(line, "square root of a negative number"));
                        }
                        x.sqrt()
                    }
                    "floor" => one(&nums)?.floor(),
                    "ceil" => one(&nums)?.ceil(),
                    "round" => one(&nums)?.round(),
                    "sin" => one(&nums)?.sin(),
                    "cos" => one(&nums)?.cos(),
                    "tan" => one(&nums)?.tan(),
                    other => return Err(eval_err(line, format!("unknown function «{other}»"))),
                };
                finite(r)
            }
        }
    }
}
