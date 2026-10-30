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
    /// For a piece of an opening's shape (`outline`), its outline in
    /// elevation: a convex polygon in `x` and `z` (counter-clockwise, x to
    /// the right, z up) running across from `min[1]` to `max[1]`; `min`
    /// and `max` are then its box. `None` for a box.
    pub profile: Option<Vec<[f64; 2]>>,
}

/// The `tint` of a [`Mark::Polygon`] filled in the colour of the lines (a
/// solid silhouette), rather than in a material's.
pub const INK_TINT: u64 = u64::MAX;

/// One mark of a plan symbol, in the family's own axes (metres, `x` and
/// `y` as for [`Part`]).
#[derive(Debug, Clone, PartialEq)]
pub enum Mark {
    Line {
        from: [f64; 2],
        to: [f64; 2],
        pen: Pen,
        style: LineStyle,
    },
    /// Closed outline; `filled` hides what is drawn under it (white), or
    /// with `tint` in that material's colour, `opacity` from 0 (clear) to
    /// 1 (solid).
    Polygon {
        points: Vec<[f64; 2]>,
        pen: Pen,
        style: LineStyle,
        filled: bool,
        tint: Option<u64>,
        opacity: f64,
    },
    /// Arc around `center` from `from` to `to`, the short way; the radius
    /// is the distance from the centre to `from`.
    Arc {
        center: [f64; 2],
        from: [f64; 2],
        to: [f64; 2],
        pen: Pen,
        style: LineStyle,
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
        base.entry("seed".into()).or_insert(Value::Number(0.5));
        let mut run = Run::new(self, Mode::Plan);
        let mut env = vec![base];
        run.exec(&self.prelude, &mut env)?;
        env.push(HashMap::new());
        run.exec(&symbol.body, &mut env)?;
        Ok(run.marks)
    }

    /// Draws the `plan presentation` symbol (for views with a presentation
    /// style); `None` if the recipe has none. `builtins` as for
    /// [`FamilyDef::evaluate_plan`], with `seed` (0 to 1) to make each
    /// element a little different.
    pub fn evaluate_presentation(
        &self,
        type_values: &Values,
        instance_values: &Values,
        builtins: &Values,
    ) -> Option<Result<Vec<Mark>, FamilyError>> {
        let symbol = self.presentation.as_ref()?;
        Some((|| {
            self.check_values(ParamScope::Type, type_values)?;
            self.check_values(ParamScope::Instance, instance_values)?;
            let mut base: HashMap<String, Value> = HashMap::new();
            for (k, v) in builtins.iter().chain(type_values).chain(instance_values) {
                base.insert(k.clone(), v.clone());
            }
            base.entry("seed".into()).or_insert(Value::Number(0.5));
            let mut run = Run::new(self, Mode::Plan);
            let mut env = vec![base];
            run.exec(&self.prelude, &mut env)?;
            env.push(HashMap::new());
            run.exec(&symbol.body, &mut env)?;
            Ok(run.marks)
        })())
    }

    /// Draws the `elevation` marks (opening lines…), in the family's `x`
    /// and `z` as the two coordinates of each mark; empty if the recipe has
    /// none.
    pub fn evaluate_elevation(
        &self,
        type_values: &Values,
        instance_values: &Values,
        builtins: &Values,
    ) -> Result<Vec<Mark>, FamilyError> {
        let Some(symbol) = &self.elevation else {
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

    /// The type values after a stretch by the handles: `values` (the
    /// type's, the new measurement among them; missing ones take their
    /// default) with the `stretch` parameters worked out again from them,
    /// rounded if whole and kept in their range. Only the parameters that
    /// change are returned.
    pub fn evaluate_stretch(&self, values: &Values) -> Result<Values, FamilyError> {
        if self.stretch.is_empty() {
            return Ok(Values::new());
        }
        let mut base: HashMap<String, Value> = self
            .defaults(ParamScope::Type)
            .into_iter()
            .chain(self.defaults(ParamScope::Instance))
            .collect();
        base.extend(values.iter().map(|(k, v)| (k.clone(), v.clone())));
        let before = base.clone();
        let mut run = Run::new(self, Mode::Body);
        let mut env = vec![base];
        let out = run.assign(&self.stretch, &mut env)?;
        let mut changed = Values::new();
        for (name, v) in out {
            let (Some(p), Some(mut n)) = (self.param(&name), v.as_number()) else {
                continue;
            };
            if p.kind == ParamKind::Integer {
                n = n.round();
            }
            if let Some((lo, hi)) = p.range {
                n = n.clamp(lo, hi);
            }
            let now = before.get(&name).and_then(Value::as_number);
            if now.is_none_or(|x| (x - n).abs() > 1e-9) {
                changed.insert(name, Value::Number(n));
            }
        }
        Ok(changed)
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
        (ParamKind::LineStyle, Value::Text(t)) => {
            if LineStyle::from_code(t).is_some() {
                Ok(())
            } else {
                Err(FamilyError::UnknownOption {
                    param: p.name.clone(),
                    label: p.label.clone(),
                    value: t.clone(),
                })
            }
        }
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

    fn line_style(
        &mut self,
        e: &Expr,
        env: &[HashMap<String, Value>],
        line: usize,
    ) -> Result<LineStyle, FamilyError> {
        match self.eval(e, env, line)? {
            Value::Text(t) => LineStyle::from_code(&t)
                .ok_or_else(|| eval_err(line, format!("«{t}» is not a line style"))),
            other => Err(eval_err(
                line,
                format!("a line style is a name, not a {}", other.kind_name()),
            )),
        }
    }

    /// The material and opacity of a tinted outline (opacity 1 without
    /// one).
    fn tint(
        &mut self,
        tint: &Option<Expr>,
        opacity: &Option<Expr>,
        env: &[HashMap<String, Value>],
        line: usize,
    ) -> Result<(Option<u64>, f64), FamilyError> {
        let material = match tint {
            None => None,
            Some(e) => match self.eval(e, env, line)? {
                Value::Material(m) => Some(m),
                other => {
                    return Err(eval_err(
                        line,
                        format!("«tint» takes a material, not a {}", other.kind_name()),
                    ))
                }
            },
        };
        let opacity = match opacity {
            None => 1.0,
            Some(e) => self.number(e, env, line)?,
        };
        if !(0.0..=1.0).contains(&opacity) {
            return Err(eval_err(line, "the opacity goes from 0 to 1"));
        }
        Ok((material, opacity))
    }

    /// The `seed` the program gives each element (0,5 without one).
    fn seed(&self, env: &[HashMap<String, Value>]) -> f64 {
        env.iter()
            .rev()
            .find_map(|f| match f.get("seed") {
                Some(Value::Number(n)) => Some(*n),
                _ => None,
            })
            .unwrap_or(0.5)
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
                    style,
                    line,
                } => {
                    self.tick(*line)?;
                    let mark = Mark::Line {
                        from: self.point(from, env, *line)?,
                        to: self.point(to, env, *line)?,
                        pen: *pen,
                        style: self.line_style(style, env, *line)?,
                    };
                    self.push_mark(mark, *line)?;
                }
                Stmt::Polygon {
                    points,
                    pen,
                    style,
                    filled,
                    tint,
                    opacity,
                    line,
                } => {
                    self.tick(*line)?;
                    let mut pts = Vec::with_capacity(points.len());
                    for p in points {
                        pts.push(self.point(p, env, *line)?);
                    }
                    let (tint, opacity) = self.tint(tint, opacity, env, *line)?;
                    let mark = Mark::Polygon {
                        points: pts,
                        pen: *pen,
                        style: self.line_style(style, env, *line)?,
                        filled: *filled,
                        tint,
                        opacity,
                    };
                    self.push_mark(mark, *line)?;
                }
                Stmt::Cloud {
                    center,
                    radius,
                    lobes,
                    jitter,
                    pen,
                    style,
                    filled,
                    tint,
                    opacity,
                    line,
                } => {
                    self.tick(*line)?;
                    let c = self.point(center, env, *line)?;
                    let r = self.number(radius, env, *line)?;
                    let n = self.number(lobes, env, *line)?.round();
                    let j = self.number(jitter, env, *line)?;
                    if r.is_nan()
                        || r <= 0.0
                        || !(3.0..=64.0).contains(&n)
                        || !(0.0..=0.5).contains(&j)
                    {
                        return Err(eval_err(
                            *line,
                            "a cloud needs a radius, 3 to 64 lobes and a jitter from 0 to 0,5",
                        ));
                    }
                    let seed = self.seed(env);
                    let (tint, opacity) = self.tint(tint, opacity, env, *line)?;
                    let mark = Mark::Polygon {
                        points: cloud(c, r, n as usize, j, seed),
                        pen: *pen,
                        style: self.line_style(style, env, *line)?,
                        filled: *filled,
                        tint,
                        opacity,
                    };
                    self.push_mark(mark, *line)?;
                }
                Stmt::Arc {
                    center,
                    from,
                    to,
                    pen,
                    style,
                    line,
                } => {
                    self.tick(*line)?;
                    let mark = Mark::Arc {
                        center: self.point(center, env, *line)?,
                        from: self.point(from, env, *line)?,
                        to: self.point(to, env, *line)?,
                        pen: *pen,
                        style: self.line_style(style, env, *line)?,
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
                        profile: None,
                    });
                }
                Stmt::Outline {
                    shape,
                    from,
                    to,
                    clip,
                    depth,
                    material,
                    line,
                } => {
                    self.tick(*line)?;
                    if self.mode != Mode::Body {
                        return Err(eval_err(*line, "a plan symbol draws lines, not pieces"));
                    }
                    let kind = match self.eval(&shape[0], env, *line)? {
                        Value::Text(t) => crate::ShapeKind::from_name(&t)
                            .ok_or_else(|| eval_err(*line, format!("«{t}» is not a shape")))?,
                        other => {
                            return Err(eval_err(
                                *line,
                                format!("a shape is a name, not a {}", other.kind_name()),
                            ))
                        }
                    };
                    let w = self.number(&shape[1], env, *line)?;
                    let h = self.number(&shape[2], env, *line)?;
                    let rise = self.number(&shape[3], env, *line)?;
                    if w <= 0.0 || h <= 0.0 {
                        return Err(FamilyError::NegativeSize { line: *line });
                    }
                    let a = self.number(from, env, *line)?;
                    let b = match to {
                        Some(e) => Some(self.number(e, env, *line)?),
                        None => None,
                    };
                    let bounds = match clip.as_deref() {
                        Some((lo, hi)) => {
                            Some((self.point(lo, env, *line)?, self.point(hi, env, *line)?))
                        }
                        None => None,
                    };
                    let (y0, y1) = (
                        self.number(&depth[0], env, *line)?,
                        self.number(&depth[1], env, *line)?,
                    );
                    if y1 < y0 - 1e-9 {
                        return Err(FamilyError::NegativeSize { line: *line });
                    }
                    let material = match self.eval(material, env, *line)? {
                        Value::Material(m) => m,
                        other => {
                            return Err(eval_err(
                                *line,
                                format!("a piece needs a material, not a {}", other.kind_name()),
                            ))
                        }
                    };
                    let outline = crate::OpeningShape::new(kind, rise).polygon(w, h);
                    let pieces = match b {
                        Some(b) => crate::shape::band(&outline, a.min(b), a.max(b)),
                        None => vec![crate::shape::inset(&outline, a)],
                    };
                    for piece in pieces {
                        let piece = match bounds {
                            Some((lo, hi)) => crate::shape::clip(&piece, lo, hi),
                            None => piece,
                        };
                        if piece.len() < 3 || crate::shape::area(&piece) < 1e-9 {
                            continue;
                        }
                        if self.parts.len() >= MAX_PARTS {
                            return Err(FamilyError::TooComplex { line: *line });
                        }
                        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
                        for q in &piece {
                            lo[0] = lo[0].min(q[0]);
                            hi[0] = hi[0].max(q[0]);
                            lo[2] = lo[2].min(q[1]);
                            hi[2] = hi[2].max(q[1]);
                        }
                        lo[1] = y0;
                        hi[1] = y1.max(y0);
                        self.parts.push(Part {
                            material,
                            min: lo,
                            max: hi,
                            line: *line,
                            profile: Some(piece),
                        });
                    }
                }
                Stmt::Prism {
                    points,
                    depth,
                    material,
                    line,
                } => {
                    self.tick(*line)?;
                    if self.mode != Mode::Body {
                        return Err(eval_err(*line, "a plan symbol draws lines, not pieces"));
                    }
                    let mut piece = Vec::with_capacity(points.len());
                    for q in points {
                        piece.push(self.point(q, env, *line)?);
                    }
                    let (y0, y1) = (
                        self.number(&depth[0], env, *line)?,
                        self.number(&depth[1], env, *line)?,
                    );
                    if y1 < y0 - 1e-9 {
                        return Err(FamilyError::NegativeSize { line: *line });
                    }
                    let material = match self.eval(material, env, *line)? {
                        Value::Material(m) => m,
                        other => {
                            return Err(eval_err(
                                *line,
                                format!("a piece needs a material, not a {}", other.kind_name()),
                            ))
                        }
                    };
                    // Counter-clockwise in elevation, as the shapes are.
                    let area = crate::shape::area(&piece);
                    if area.abs() < 1e-9 {
                        continue;
                    }
                    if area < 0.0 {
                        piece.reverse();
                    }
                    if self.parts.len() >= MAX_PARTS {
                        return Err(FamilyError::TooComplex { line: *line });
                    }
                    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
                    for q in &piece {
                        lo[0] = lo[0].min(q[0]);
                        hi[0] = hi[0].max(q[0]);
                        lo[2] = lo[2].min(q[1]);
                        hi[2] = hi[2].max(q[1]);
                    }
                    lo[1] = y0;
                    hi[1] = y1.max(y0);
                    self.parts.push(Part {
                        material,
                        min: lo,
                        max: hi,
                        line: *line,
                        profile: Some(piece),
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
                    // The same number in [0, 1) for the same value: to make
                    // each tree a little different, with `seed`.
                    "rand" => noise(one(&nums)?),
                    other => return Err(eval_err(line, format!("unknown function «{other}»"))),
                };
                finite(r)
            }
        }
    }
}

/// A number in [0, 1) that is always the same for the same `x`.
pub(crate) fn noise(x: f64) -> f64 {
    let mut h = x.to_bits() ^ 0x9E37_79B9_7F4A_7C15;
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    h ^= h >> 33;
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// The outline of a cloud: `lobes` bulges round `c`, each of its own size
/// and width (by `seed`), the outline going in and out by about `jitter`
/// of the radius. Counter-clockwise.
pub(crate) fn cloud(c: [f64; 2], r: f64, lobes: usize, jitter: f64, seed: f64) -> Vec<[f64; 2]> {
    use std::f64::consts::TAU;
    let k = |i: usize, w: f64| noise(seed * 7919.0 + i as f64 * 13.0 + w);
    // Where each lobe starts, unevenly round the circle.
    let mut starts: Vec<f64> = (0..lobes)
        .map(|i| (i as f64 + 0.35 * (k(i, 0.1) - 0.5)) / lobes as f64 * TAU)
        .collect();
    let turn = k(0, 0.7) * TAU;
    for a in &mut starts {
        *a += turn;
    }
    let mut out = Vec::new();
    for i in 0..lobes {
        let a0 = starts[i];
        let a1 = if i + 1 < lobes {
            starts[i + 1]
        } else {
            starts[0] + TAU
        };
        // The lobe bulges out from the inner radius by its own amount.
        let inner = r * (1.0 - jitter * (0.6 + 0.8 * k(i, 0.3)));
        let bulge = r * jitter * (0.8 + 0.9 * k(i, 0.5));
        let steps = 6;
        for s in 0..steps {
            let t = s as f64 / steps as f64;
            let a = a0 + (a1 - a0) * t;
            let rr = inner + bulge * (std::f64::consts::PI * t).sin();
            out.push([c[0] + rr * a.cos(), c[1] + rr * a.sin()]);
        }
    }
    out
}
