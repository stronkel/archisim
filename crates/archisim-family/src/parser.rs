//! Reads recipe text into a [`FamilyDef`].

use std::collections::BTreeMap;

use crate::ast::*;
use crate::error::{syntax, FamilyError};
use crate::lexer::{lines, Line, Tok};
use crate::Value;

/// Names the program provides to every wall-hosted family: the side the
/// leaves open to (`"interior"` or `"exterior"`) and the handing
/// (`"left"` or `"right"`, UNE-EN 12519). A recipe cannot declare them.
pub const BUILTINS: [&str; 2] = ["opens_to", "hand"];

/// Names the program also provides to plan symbols of wall-hosted
/// families, in the family's axes: where the wall's exterior and interior
/// faces are (`y`, metres) and the sill height above the level.
pub const PLAN_BUILTINS: [&str; 3] = ["wall_ext", "wall_int", "sill_height"];

/// Reads a recipe. Every `include` must have been expanded first (see
/// [`expand_includes`]).
pub fn parse(source: &str) -> Result<FamilyDef, FamilyError> {
    let tree = lines(source)?;
    let mut def = FamilyDef {
        name: String::new(),
        category: String::new(),
        host: Host::None,
        version: 1,
        description: String::new(),
        params: Vec::new(),
        prelude: Vec::new(),
        components: BTreeMap::new(),
        body: Vec::new(),
        plans: Vec::new(),
        outputs: Vec::new(),
        ifc: Vec::new(),
    };
    let mut seen_family = false;
    let mut seen_body = false;
    for line in &tree {
        let mut c = Cursor::new(line);
        let head = c.ident()?;
        match head.as_str() {
            "family" => {
                if seen_family {
                    return Err(syntax(line.no, "only one «family» line is allowed"));
                }
                seen_family = true;
                def.name = c.string()?;
                c.end()?;
                header(&mut def, &line.children)?;
            }
            "include" => {
                return Err(syntax(
                    line.no,
                    "«include» must be expanded before reading the recipe",
                ))
            }
            "type_params" | "instance_params" => {
                c.end()?;
                let scope = if head == "type_params" {
                    ParamScope::Type
                } else {
                    ParamScope::Instance
                };
                params(&mut def, scope, None, &line.children)?;
            }
            "let" | "require" => def.prelude.push(statement(line)?),
            "component" => {
                let name = c.ident()?;
                c.sym("(")?;
                let mut args = Vec::new();
                if !c.eat_sym(")") {
                    loop {
                        let a = c.ident()?;
                        if args.contains(&a) {
                            return Err(syntax(line.no, format!("argument «{a}» is repeated")));
                        }
                        args.push(a);
                        if c.eat_sym(")") {
                            break;
                        }
                        c.sym(",")?;
                    }
                }
                c.end()?;
                if def.components.contains_key(&name) {
                    return Err(syntax(line.no, format!("component «{name}» is repeated")));
                }
                let body = statements(&line.children)?;
                def.components.insert(
                    name.clone(),
                    Component {
                        name,
                        args,
                        body,
                        line: line.no,
                    },
                );
            }
            "plan" => {
                // `plan 1:50`
                let one = c.number()?;
                c.sym(":")?;
                let scale = c.number()?;
                c.end()?;
                if one != 1.0 || scale < 1.0 {
                    return Err(syntax(line.no, "write the scale as «plan 1:50»"));
                }
                if def.plans.iter().any(|p| p.scale == scale) {
                    return Err(syntax(
                        line.no,
                        format!("there is already a plan 1:{scale}"),
                    ));
                }
                let body = statements(&line.children)?;
                if let Some(bad) = first_where(&body, &|s| matches!(s, Stmt::Box { .. })) {
                    return Err(syntax(bad, "a plan symbol draws lines, not boxes"));
                }
                def.plans.push(PlanSymbol {
                    scale,
                    body,
                    line: line.no,
                });
            }
            "body" => {
                c.end()?;
                if seen_body {
                    return Err(syntax(line.no, "only one «body» block is allowed"));
                }
                seen_body = true;
                def.body = statements(&line.children)?;
                if let Some(bad) = first_where(&def.body, &Stmt::is_plan_mark) {
                    return Err(syntax(bad, "lines, outlines and arcs go in a «plan» block"));
                }
            }
            "outputs" | "ifc" => {
                c.end()?;
                let list = assignments(&line.children)?;
                if head == "outputs" {
                    def.outputs.extend(list);
                } else {
                    def.ifc.extend(list);
                }
            }
            other => return Err(syntax(line.no, format!("unknown section «{other}»"))),
        }
    }
    if !seen_family {
        return Err(syntax(1, "the recipe must start with «family \"Name\"»"));
    }
    if def.category.is_empty() {
        return Err(syntax(1, "the family needs a «category» line"));
    }
    check_names(&def)?;
    Ok(def)
}

/// Replaces every top-level `include "name"` line with the text the
/// resolver gives for that name, so that a family stored in a project
/// carries its shared pieces with it.
pub fn expand_includes(
    source: &str,
    resolve: impl Fn(&str) -> Option<String>,
) -> Result<String, FamilyError> {
    let mut out = String::with_capacity(source.len());
    for (i, raw) in source.lines().enumerate() {
        let trimmed = raw.trim();
        if raw.starts_with("include") {
            let toks = crate::lexer::tokenize(trimmed, i + 1)?;
            let name = match toks.as_slice() {
                [Tok::Ident(kw), Tok::Str(name)] if kw == "include" => name.clone(),
                _ => return Err(syntax(i + 1, "write «include \"name\"»")),
            };
            let text = resolve(&name)
                .ok_or_else(|| syntax(i + 1, format!("no shared pieces called «{name}»")))?;
            if text.lines().any(|l| l.starts_with("include")) {
                return Err(syntax(i + 1, "included pieces cannot include others"));
            }
            out.push_str(&format!("# --- include \"{name}\" ---\n"));
            out.push_str(text.trim_end());
            out.push_str(&format!("\n# --- end of \"{name}\" ---\n"));
        } else {
            out.push_str(raw);
            out.push('\n');
        }
    }
    Ok(out)
}

fn header(def: &mut FamilyDef, children: &[Line]) -> Result<(), FamilyError> {
    for line in children {
        let mut c = Cursor::new(line);
        match c.ident()?.as_str() {
            "category" => def.category = c.ident()?,
            "host" => {
                def.host = match c.ident()?.as_str() {
                    "wall" => Host::Wall,
                    "none" => Host::None,
                    other => {
                        return Err(syntax(
                            line.no,
                            format!("unknown host «{other}» (use wall or none)"),
                        ))
                    }
                }
            }
            "version" => {
                let n = c.number()?;
                if n < 1.0 || n.fract() != 0.0 || n > f64::from(u32::MAX) {
                    return Err(syntax(line.no, "the version must be a whole number from 1"));
                }
                def.version = n as u32;
            }
            "description" => def.description = c.string()?,
            other => return Err(syntax(line.no, format!("unknown family setting «{other}»"))),
        }
        c.end()?;
        no_children(line)?;
    }
    Ok(())
}

fn params(
    def: &mut FamilyDef,
    scope: ParamScope,
    section: Option<&str>,
    children: &[Line],
) -> Result<(), FamilyError> {
    for line in children {
        let mut c = Cursor::new(line);
        if c.peek_ident("section") {
            c.ident()?;
            let label = c.string()?;
            c.end()?;
            if section.is_some() {
                return Err(syntax(line.no, "sections cannot be nested"));
            }
            params(def, scope, Some(&label), &line.children)?;
            continue;
        }
        no_children(line)?;
        let name = c.ident()?;
        if def.params.iter().any(|p| p.name == name) {
            return Err(syntax(line.no, format!("parameter «{name}» is repeated")));
        }
        if BUILTINS.contains(&name.as_str()) || PLAN_BUILTINS.contains(&name.as_str()) {
            return Err(syntax(
                line.no,
                format!("«{name}» is provided by the program and cannot be declared"),
            ));
        }
        let kind = match c.ident()?.as_str() {
            "length" => ParamKind::Length,
            "angle" => ParamKind::Angle,
            "number" => ParamKind::Number,
            "integer" => ParamKind::Integer,
            "bool" => ParamKind::Bool,
            "material" => ParamKind::Material,
            "text" => ParamKind::Text,
            "choice" => {
                c.sym("(")?;
                let mut options = vec![c.string()?];
                while c.eat_sym(",") {
                    options.push(c.string()?);
                }
                c.sym(")")?;
                ParamKind::Choice(options)
            }
            other => return Err(syntax(line.no, format!("unknown parameter kind «{other}»"))),
        };
        c.sym("=")?;
        let default = c.literal()?;
        let mut param = Param {
            label: name.clone(),
            name,
            scope,
            kind,
            default,
            range: None,
            section: section.map(str::to_string),
            visible_if: None,
            line: line.no,
        };
        while !c.at_end() {
            match c.ident()?.as_str() {
                "range" => {
                    let lo = c.signed_number()?;
                    c.sym("..")?;
                    let hi = c.signed_number()?;
                    if hi < lo {
                        return Err(syntax(line.no, "the range goes from low to high"));
                    }
                    param.range = Some((lo, hi));
                }
                "label" => param.label = c.string()?,
                "visible_if" => param.visible_if = Some(c.expr()?),
                other => {
                    return Err(syntax(
                        line.no,
                        format!("unknown parameter setting «{other}»"),
                    ))
                }
            }
        }
        check_default(&param)?;
        def.params.push(param);
    }
    Ok(())
}

/// The default must be of the parameter's kind and inside its range.
fn check_default(p: &Param) -> Result<(), FamilyError> {
    let bad = |what: &str| {
        Err(syntax(
            p.line,
            format!("the default of «{}» must be {what}", p.name),
        ))
    };
    match (&p.kind, &p.default) {
        (k, Value::Number(n)) if k.is_numeric() => {
            if *k == ParamKind::Integer && n.fract() != 0.0 {
                return bad("a whole number");
            }
            if let Some((lo, hi)) = p.range {
                if *n < lo || *n > hi {
                    return bad("inside its range");
                }
            }
        }
        (k, _) if k.is_numeric() => return bad("a number"),
        (ParamKind::Bool, Value::Bool(_)) => {}
        (ParamKind::Bool, _) => return bad("yes or no"),
        (ParamKind::Material, Value::Text(_)) => {}
        (ParamKind::Material, _) => return bad("a material name in quotes"),
        (ParamKind::Text, Value::Text(_)) => {}
        (ParamKind::Text, _) => return bad("a text in quotes"),
        (ParamKind::Choice(options), Value::Text(t)) => {
            if !options.contains(t) {
                return bad("one of its options");
            }
        }
        (ParamKind::Choice(_), _) => return bad("one of its options, in quotes"),
        _ => return bad("valid"),
    }
    if p.range.is_some() && !p.kind.is_numeric() {
        return Err(syntax(p.line, "only numbers can have a range"));
    }
    Ok(())
}

fn assignments(children: &[Line]) -> Result<Vec<Output>, FamilyError> {
    let mut out: Vec<Output> = Vec::new();
    for line in children {
        no_children(line)?;
        let mut c = Cursor::new(line);
        let name = c.ident()?;
        c.sym("=")?;
        let expr = c.expr()?;
        c.end()?;
        if out.iter().any(|o| o.name == name) {
            return Err(syntax(line.no, format!("«{name}» is repeated")));
        }
        out.push(Output {
            name,
            expr,
            line: line.no,
        });
    }
    Ok(out)
}

fn statements(lines: &[Line]) -> Result<Vec<Stmt>, FamilyError> {
    let mut out: Vec<Stmt> = Vec::new();
    for line in lines {
        let mut c = Cursor::new(line);
        if c.peek_ident("else") {
            c.ident()?;
            // `else` or `else if <condition>`, attached to the last `if`.
            let nested = if c.peek_ident("if") {
                c.ident()?;
                let cond = c.expr()?;
                c.end()?;
                Some(Stmt::If {
                    cond,
                    then: statements(&line.children)?,
                    otherwise: Vec::new(),
                    line: line.no,
                })
            } else {
                c.end()?;
                None
            };
            let Some(Stmt::If { otherwise, .. }) = out.last_mut() else {
                return Err(syntax(line.no, "«else» without an «if» before it"));
            };
            let slot = open_else(otherwise)
                .ok_or_else(|| syntax(line.no, "this «if» already has an «else»"))?;
            *slot = match nested {
                Some(stmt) => vec![stmt],
                None => {
                    let body = statements(&line.children)?;
                    if body.is_empty() {
                        return Err(syntax(line.no, "«else» needs an indented block"));
                    }
                    body
                }
            };
            continue;
        }
        out.push(statement(line)?);
    }
    Ok(out)
}

/// The still-empty `else` at the end of an `if … else if …` chain.
fn open_else(otherwise: &mut Vec<Stmt>) -> Option<&mut Vec<Stmt>> {
    if otherwise.is_empty() {
        return Some(otherwise);
    }
    match otherwise.as_mut_slice() {
        [Stmt::If {
            otherwise: inner, ..
        }] => open_else(inner),
        _ => None,
    }
}

fn statement(line: &Line) -> Result<Stmt, FamilyError> {
    let mut c = Cursor::new(line);
    let no = line.no;
    let kw = c.ident()?;
    let block_kw = matches!(kw.as_str(), "repeat" | "if");
    if !block_kw {
        no_children(line)?;
    }
    let stmt = match kw.as_str() {
        "box" => {
            let min = c.triple()?;
            c.keyword("to")?;
            let max = c.triple()?;
            c.keyword("material")?;
            let material = c.expr()?;
            Stmt::Box {
                min,
                max,
                material,
                line: no,
            }
        }
        "use" => {
            let name = c.ident()?;
            c.sym("(")?;
            let mut args = Vec::new();
            if !c.eat_sym(")") {
                loop {
                    args.push(c.expr()?);
                    if c.eat_sym(")") {
                        break;
                    }
                    c.sym(",")?;
                }
            }
            Stmt::Use {
                name,
                args,
                line: no,
            }
        }
        "repeat" => {
            let var = c.ident()?;
            c.keyword("in")?;
            let from = c.expr()?;
            c.sym("..")?;
            let to = c.expr()?;
            Stmt::Repeat {
                var,
                from,
                to,
                body: statements(&line.children)?,
                line: no,
            }
        }
        "if" => {
            let cond = c.expr()?;
            let then = statements(&line.children)?;
            if then.is_empty() {
                return Err(syntax(no, "«if» needs an indented block"));
            }
            Stmt::If {
                cond,
                then,
                otherwise: Vec::new(),
                line: no,
            }
        }
        "let" => {
            let name = c.ident()?;
            c.sym("=")?;
            Stmt::Let {
                name,
                expr: c.expr()?,
                line: no,
            }
        }
        "line" => {
            let from = c.pair()?;
            c.keyword("to")?;
            let to = c.pair()?;
            let pen = c.pen()?;
            let dashed = c.peek_ident("dashed");
            if dashed {
                c.ident()?;
            }
            Stmt::Line {
                from,
                to,
                pen,
                dashed,
                line: no,
            }
        }
        "polygon" => {
            let mut points = vec![c.pair()?];
            while c.peek_sym("(") {
                points.push(c.pair()?);
            }
            if points.len() < 3 {
                return Err(syntax(no, "an outline needs at least three points"));
            }
            let pen = c.pen()?;
            let filled = c.peek_ident("filled");
            if filled {
                c.ident()?;
            }
            Stmt::Polygon {
                points,
                pen,
                filled,
                line: no,
            }
        }
        "arc" => {
            c.keyword("center")?;
            let center = c.pair()?;
            c.keyword("from")?;
            let from = c.pair()?;
            c.keyword("to")?;
            let to = c.pair()?;
            let pen = c.pen()?;
            Stmt::Arc {
                center,
                from,
                to,
                pen,
                line: no,
            }
        }
        "require" => {
            let cond = c.expr()?;
            c.keyword("else")?;
            Stmt::Require(Require {
                cond,
                message: c.string()?,
                line: no,
            })
        }
        other => return Err(syntax(no, format!("unknown instruction «{other}»"))),
    };
    if let Stmt::Repeat { body, .. } = &stmt {
        if body.is_empty() {
            return Err(syntax(no, "«repeat» needs an indented block"));
        }
    }
    c.end()?;
    Ok(stmt)
}

/// The line of the first statement (nested ones included) that matches.
fn first_where(stmts: &[Stmt], pred: &dyn Fn(&Stmt) -> bool) -> Option<usize> {
    for s in stmts {
        if pred(s) {
            return Some(stmt_line(s));
        }
        let inner = match s {
            Stmt::Repeat { body, .. } => first_where(body, pred),
            Stmt::If {
                then, otherwise, ..
            } => first_where(then, pred).or_else(|| first_where(otherwise, pred)),
            _ => None,
        };
        if inner.is_some() {
            return inner;
        }
    }
    None
}

pub(crate) fn stmt_line(s: &Stmt) -> usize {
    match s {
        Stmt::Box { line, .. }
        | Stmt::Use { line, .. }
        | Stmt::Repeat { line, .. }
        | Stmt::If { line, .. }
        | Stmt::Let { line, .. }
        | Stmt::Line { line, .. }
        | Stmt::Polygon { line, .. }
        | Stmt::Arc { line, .. } => *line,
        Stmt::Require(r) => r.line,
    }
}

fn no_children(line: &Line) -> Result<(), FamilyError> {
    match line.children.first() {
        Some(child) => Err(syntax(child.no, "unexpected indentation")),
        None => Ok(()),
    }
}

/// Every `use` names an existing piece with the right number of arguments,
/// and pieces do not use themselves.
fn check_names(def: &FamilyDef) -> Result<(), FamilyError> {
    fn walk(def: &FamilyDef, stmts: &[Stmt], stack: &mut Vec<String>) -> Result<(), FamilyError> {
        for s in stmts {
            match s {
                Stmt::Use { name, args, line } => {
                    let Some(comp) = def.components.get(name) else {
                        return Err(syntax(*line, format!("no piece called «{name}»")));
                    };
                    if comp.args.len() != args.len() {
                        return Err(syntax(
                            *line,
                            format!(
                                "«{name}» takes {} values, not {}",
                                comp.args.len(),
                                args.len()
                            ),
                        ));
                    }
                    if stack.contains(name) {
                        return Err(syntax(*line, format!("«{name}» uses itself")));
                    }
                    stack.push(name.clone());
                    walk(def, &comp.body, stack)?;
                    stack.pop();
                }
                Stmt::Repeat { body, .. } => walk(def, body, stack)?,
                Stmt::If {
                    then, otherwise, ..
                } => {
                    walk(def, then, stack)?;
                    walk(def, otherwise, stack)?;
                }
                Stmt::Box { .. }
                | Stmt::Let { .. }
                | Stmt::Require(_)
                | Stmt::Line { .. }
                | Stmt::Polygon { .. }
                | Stmt::Arc { .. } => {}
            }
        }
        Ok(())
    }
    walk(def, &def.body, &mut Vec::new())?;
    for p in &def.plans {
        walk(def, &p.body, &mut Vec::new())?;
    }
    for comp in def.components.values() {
        walk(def, &comp.body, &mut vec![comp.name.clone()])?;
    }
    Ok(())
}

/// Reads the tokens of one line.
struct Cursor<'a> {
    toks: &'a [Tok],
    pos: usize,
    no: usize,
}

impl<'a> Cursor<'a> {
    fn new(line: &'a Line) -> Self {
        Self {
            toks: &line.toks,
            pos: 0,
            no: line.no,
        }
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn at_end(&self) -> bool {
        self.pos >= self.toks.len()
    }

    fn err<T>(&self, expected: &str) -> Result<T, FamilyError> {
        let found = self
            .peek()
            .map_or("the end of the line".to_string(), Tok::describe);
        Err(syntax(
            self.no,
            format!("expected {expected}, found {found}"),
        ))
    }

    fn end(&self) -> Result<(), FamilyError> {
        if self.at_end() {
            Ok(())
        } else {
            self.err("the end of the line")
        }
    }

    fn ident(&mut self) -> Result<String, FamilyError> {
        match self.peek() {
            Some(Tok::Ident(s)) => {
                let s = s.clone();
                self.pos += 1;
                Ok(s)
            }
            _ => self.err("a name"),
        }
    }

    fn peek_ident(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(Tok::Ident(s)) if s == kw)
    }

    fn keyword(&mut self, kw: &str) -> Result<(), FamilyError> {
        if self.peek_ident(kw) {
            self.pos += 1;
            Ok(())
        } else {
            self.err(&format!("«{kw}»"))
        }
    }

    fn string(&mut self) -> Result<String, FamilyError> {
        match self.peek() {
            Some(Tok::Str(s)) => {
                let s = s.clone();
                self.pos += 1;
                Ok(s)
            }
            _ => self.err("a text in quotes"),
        }
    }

    fn number(&mut self) -> Result<f64, FamilyError> {
        match self.peek() {
            Some(Tok::Num(n)) => {
                let n = *n;
                self.pos += 1;
                Ok(n)
            }
            _ => self.err("a number"),
        }
    }

    fn signed_number(&mut self) -> Result<f64, FamilyError> {
        if self.eat_sym("-") {
            Ok(-self.number()?)
        } else {
            self.number()
        }
    }

    fn eat_sym(&mut self, sym: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Sym(s)) if *s == sym) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn sym(&mut self, sym: &str) -> Result<(), FamilyError> {
        if self.eat_sym(sym) {
            Ok(())
        } else {
            self.err(&format!("«{sym}»"))
        }
    }

    /// A constant: number, text, or yes/no.
    fn literal(&mut self) -> Result<Value, FamilyError> {
        match self.peek() {
            Some(Tok::Str(_)) => Ok(Value::Text(self.string()?)),
            Some(Tok::Ident(s)) if is_bool(s).is_some() => {
                let b = is_bool(s).expect("checked");
                self.pos += 1;
                Ok(Value::Bool(b))
            }
            _ => Ok(Value::Number(self.signed_number()?)),
        }
    }

    fn peek_sym(&self, sym: &str) -> bool {
        matches!(self.peek(), Some(Tok::Sym(s)) if *s == sym)
    }

    fn pair(&mut self) -> Result<[Expr; 2], FamilyError> {
        self.sym("(")?;
        let x = self.expr()?;
        self.sym(",")?;
        let y = self.expr()?;
        self.sym(")")?;
        Ok([x, y])
    }

    /// `pen cut|frame|view|thin|hatch`.
    fn pen(&mut self) -> Result<Pen, FamilyError> {
        self.keyword("pen")?;
        let no = self.no;
        match self.ident()?.as_str() {
            "cut" => Ok(Pen::Cut),
            "frame" => Ok(Pen::Frame),
            "view" => Ok(Pen::View),
            "thin" => Ok(Pen::Thin),
            "hatch" => Ok(Pen::Hatch),
            other => Err(syntax(
                no,
                format!("unknown pen «{other}» (cut, frame, view, thin or hatch)"),
            )),
        }
    }

    fn triple(&mut self) -> Result<[Expr; 3], FamilyError> {
        self.sym("(")?;
        let x = self.expr()?;
        self.sym(",")?;
        let y = self.expr()?;
        self.sym(",")?;
        let z = self.expr()?;
        self.sym(")")?;
        Ok([x, y, z])
    }

    // ---- Expressions: if > or > and > not > comparison > + - > * / > unary

    fn expr(&mut self) -> Result<Expr, FamilyError> {
        if self.peek_ident("if") {
            self.pos += 1;
            let cond = self.expr()?;
            self.keyword("then")?;
            let a = self.expr()?;
            self.keyword("else")?;
            let b = self.expr()?;
            return Ok(Expr::If(Box::new(cond), Box::new(a), Box::new(b)));
        }
        self.or()
    }

    fn or(&mut self) -> Result<Expr, FamilyError> {
        let mut e = self.and()?;
        while self.peek_ident("or") {
            self.pos += 1;
            e = Expr::Bin(BinOp::Or, Box::new(e), Box::new(self.and()?));
        }
        Ok(e)
    }

    fn and(&mut self) -> Result<Expr, FamilyError> {
        let mut e = self.not()?;
        while self.peek_ident("and") {
            self.pos += 1;
            e = Expr::Bin(BinOp::And, Box::new(e), Box::new(self.not()?));
        }
        Ok(e)
    }

    fn not(&mut self) -> Result<Expr, FamilyError> {
        if self.peek_ident("not") {
            self.pos += 1;
            return Ok(Expr::Not(Box::new(self.not()?)));
        }
        self.comparison()
    }

    fn comparison(&mut self) -> Result<Expr, FamilyError> {
        let e = self.sum()?;
        let op = match self.peek() {
            Some(Tok::Sym("==")) => BinOp::Eq,
            Some(Tok::Sym("!=")) => BinOp::Ne,
            Some(Tok::Sym("<")) => BinOp::Lt,
            Some(Tok::Sym("<=")) => BinOp::Le,
            Some(Tok::Sym(">")) => BinOp::Gt,
            Some(Tok::Sym(">=")) => BinOp::Ge,
            _ => return Ok(e),
        };
        self.pos += 1;
        Ok(Expr::Bin(op, Box::new(e), Box::new(self.sum()?)))
    }

    fn sum(&mut self) -> Result<Expr, FamilyError> {
        let mut e = self.product()?;
        loop {
            let op = if self.eat_sym("+") {
                BinOp::Add
            } else if self.eat_sym("-") {
                BinOp::Sub
            } else {
                return Ok(e);
            };
            e = Expr::Bin(op, Box::new(e), Box::new(self.product()?));
        }
    }

    fn product(&mut self) -> Result<Expr, FamilyError> {
        let mut e = self.unary()?;
        loop {
            let op = if self.eat_sym("*") {
                BinOp::Mul
            } else if self.eat_sym("/") {
                BinOp::Div
            } else {
                return Ok(e);
            };
            e = Expr::Bin(op, Box::new(e), Box::new(self.unary()?));
        }
    }

    fn unary(&mut self) -> Result<Expr, FamilyError> {
        if self.eat_sym("-") {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        self.atom()
    }

    fn atom(&mut self) -> Result<Expr, FamilyError> {
        match self.peek().cloned() {
            Some(Tok::Num(n)) => {
                self.pos += 1;
                Ok(Expr::Num(n))
            }
            Some(Tok::Str(s)) => {
                self.pos += 1;
                Ok(Expr::Str(s))
            }
            Some(Tok::Sym("(")) => {
                self.pos += 1;
                let e = self.expr()?;
                self.sym(")")?;
                Ok(e)
            }
            Some(Tok::Ident(name)) => {
                self.pos += 1;
                if let Some(b) = is_bool(&name) {
                    return Ok(Expr::Bool(b));
                }
                if is_reserved(&name) {
                    return Err(syntax(self.no, format!("«{name}» cannot be used here")));
                }
                if self.eat_sym("(") {
                    let mut args = Vec::new();
                    if !self.eat_sym(")") {
                        loop {
                            args.push(self.expr()?);
                            if self.eat_sym(")") {
                                break;
                            }
                            self.sym(",")?;
                        }
                    }
                    return Ok(Expr::Call(name, args));
                }
                Ok(Expr::Var(name))
            }
            _ => self.err("a value"),
        }
    }
}

fn is_bool(s: &str) -> Option<bool> {
    match s {
        "yes" | "true" => Some(true),
        "no" | "false" => Some(false),
        _ => None,
    }
}

fn is_reserved(s: &str) -> bool {
    matches!(
        s,
        "if" | "then" | "else" | "and" | "or" | "not" | "to" | "material" | "in"
    )
}
