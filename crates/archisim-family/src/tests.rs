use super::*;

const WINDOW: &str = r#"
family "Ventana de prueba"   # a comment
  category window
  host wall
  version 3
  description "Para las pruebas"

type_params
  section "Dimensiones"
    width   length = 1.20 range 0.40..3.00 label "Hueco de obra: ancho"
    height  length = 1.20 range 0.40..3.00 label "Hueco de obra: alto"
  section "Hojas"
    leaves  integer = 2 range 1..3 label "Nº de hojas"
    shutter bool = no label "Persiana"
    box_h   length = 0.25 visible_if shutter label "Altura del capialzado"
    finish  choice("lacado", "anodizado") = "lacado"
  frame_w   length = 0.06
  frame_mat material = "Aluminio"
  glass     material = "Vidrio"

instance_params
  note text = ""

let f = frame_w
let x0 = -width / 2
require 2 * f < width else "El marco no deja sitio para el vidrio"

component block(xa, xb, depth, za, zb, mat)
  box (xa, -depth / 2, za) to (xb, depth / 2, zb) material mat

body
  use block(x0, x0 + f, 0.07, 0, height, frame_mat)
  let clear = (width - 2 * f - (leaves - 1) * f) / leaves
  repeat i in 0..leaves
    let a = x0 + f + i * (clear + f)
    use block(a, a + clear, min(0.024, 0.07), f, height - f, glass)
  if shutter
    use block(x0, -x0, 0.07, height, height + box_h, frame_mat)
  else if finish == "anodizado"
    use block(x0, x0 + 0.01, 0.01, 0, 0.01, frame_mat)
  else
    use block(x0, x0 + 0.02, 0.01, 0, 0.01, frame_mat)

outputs
  opening_width = width
  operation = if leaves == 1 then "single" else "multiple"

ifc
  predefined_type = "WINDOW"
"#;

fn values(pairs: &[(&str, Value)]) -> Values {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

fn window_values() -> Values {
    let def = parse(WINDOW).unwrap();
    let mut v = def.defaults(ParamScope::Type);
    v.insert("frame_mat".into(), Value::Material(7));
    v.insert("glass".into(), Value::Material(8));
    v
}

fn instance() -> Values {
    values(&[("note", Value::Text(String::new()))])
}

#[test]
fn reads_the_header_and_parameters() {
    let def = parse(WINDOW).unwrap();
    assert_eq!(def.name, "Ventana de prueba");
    assert_eq!(def.category, "window");
    assert_eq!(def.host, Host::Wall);
    assert_eq!(def.version, 3);
    assert_eq!(def.description, "Para las pruebas");
    assert_eq!(def.folder, "");
    let with_folder = WINDOW.replace("  version 3\n", "  version 3\n  folder \"Practicables\"\n");
    assert_eq!(parse(&with_folder).unwrap().folder, "Practicables");
    let names: Vec<_> = def.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "width",
            "height",
            "leaves",
            "shutter",
            "box_h",
            "finish",
            "frame_w",
            "frame_mat",
            "glass",
            "note"
        ]
    );
    let w = def.param("width").unwrap();
    assert_eq!(w.kind, ParamKind::Length);
    assert_eq!(w.range, Some((0.4, 3.0)));
    assert_eq!(w.label, "Hueco de obra: ancho");
    assert_eq!(w.section.as_deref(), Some("Dimensiones"));
    assert_eq!(w.scope, ParamScope::Type);
    assert_eq!(def.param("frame_w").unwrap().section, None);
    assert_eq!(def.param("note").unwrap().scope, ParamScope::Instance);
    assert_eq!(
        def.param("finish").unwrap().kind,
        ParamKind::Choice(vec!["lacado".into(), "anodizado".into()])
    );
    assert_eq!(
        def.param("frame_mat").unwrap().default,
        Value::Text("Aluminio".into())
    );
    assert_eq!(def.components.len(), 1);
}

#[test]
fn evaluates_formulas_repeats_and_pieces() {
    let def = parse(WINDOW).unwrap();
    let ev = def
        .evaluate(&window_values(), &instance(), &Values::new())
        .unwrap();
    // Jamb, two panes, and the "lacado" marker.
    assert_eq!(ev.parts.len(), 4);
    let jamb = &ev.parts[0];
    assert_eq!(jamb.material, 7);
    assert_eq!(jamb.min, [-0.6, -0.035, 0.0]);
    assert_eq!(jamb.max, [-0.6 + 0.06, 0.035, 1.2]);
    // The same arithmetic, in the same order, as a hand-written program.
    let (w, f, n) = (1.2_f64, 0.06_f64, 2.0_f64);
    let clear = (w - 2.0 * f - (n - 1.0) * f) / n;
    let x0 = -w / 2.0;
    for i in 0..2 {
        let pane = &ev.parts[1 + i];
        let a = x0 + f + i as f64 * (clear + f);
        assert_eq!(pane.material, 8);
        assert_eq!(pane.min, [a, -0.012, f]);
        assert_eq!(pane.max, [a + clear, 0.012, w - f]);
    }
    assert_eq!(ev.parts[3].max[0], x0 + 0.02);
    assert_eq!(ev.outputs["opening_width"], Value::Number(1.2));
    assert_eq!(ev.outputs["operation"], Value::Text("multiple".into()));
    assert_eq!(ev.ifc["predefined_type"], Value::Text("WINDOW".into()));
}

#[test]
fn conditions_choose_their_branch() {
    let def = parse(WINDOW).unwrap();
    let mut v = window_values();
    v.insert("shutter".into(), Value::Bool(true));
    v.insert("leaves".into(), Value::Number(1.0));
    let ev = def.evaluate(&v, &instance(), &Values::new()).unwrap();
    assert_eq!(ev.parts.len(), 3);
    assert_eq!(ev.parts[2].min[2], 1.2);
    assert!((ev.parts[2].max[2] - 1.45).abs() < 1e-12);
    assert_eq!(ev.outputs["operation"], Value::Text("single".into()));

    v.insert("shutter".into(), Value::Bool(false));
    v.insert("finish".into(), Value::Text("anodizado".into()));
    let ev = def.evaluate(&v, &instance(), &Values::new()).unwrap();
    assert!((ev.parts.last().unwrap().max[0] - (-0.6 + 0.01)).abs() < 1e-12);
}

#[test]
fn values_are_checked() {
    let def = parse(WINDOW).unwrap();
    let eval = |v: &Values| def.evaluate(v, &instance(), &Values::new());

    let mut v = window_values();
    v.insert("width".into(), Value::Number(0.3));
    assert!(matches!(
        eval(&v),
        Err(FamilyError::OutOfRange { ref label, min, max, .. })
            if label == "Hueco de obra: ancho" && min == 0.4 && max == 3.0
    ));

    let mut v = window_values();
    v.insert("leaves".into(), Value::Number(1.5));
    assert!(matches!(eval(&v), Err(FamilyError::NotInteger { .. })));

    let mut v = window_values();
    v.insert("finish".into(), Value::Text("cromado".into()));
    assert!(matches!(eval(&v), Err(FamilyError::UnknownOption { .. })));

    let mut v = window_values();
    v.insert("glass".into(), Value::Text("Vidrio".into()));
    assert!(matches!(
        eval(&v),
        Err(FamilyError::WrongKind {
            expected: "material",
            ..
        })
    ));

    let mut v = window_values();
    v.remove("height");
    assert_eq!(
        eval(&v),
        Err(FamilyError::MissingValue {
            param: "height".into()
        })
    );

    let mut v = window_values();
    v.insert("colour".into(), Value::Number(1.0));
    assert!(matches!(eval(&v), Err(FamilyError::UnknownParam { .. })));

    // An instance value given as a type value is unknown to the type.
    let mut v = window_values();
    v.insert("note".into(), Value::Text("x".into()));
    assert!(matches!(eval(&v), Err(FamilyError::UnknownParam { .. })));
}

#[test]
fn requirements_and_negative_pieces_are_rejected() {
    let def = parse(WINDOW).unwrap();
    let mut v = window_values();
    v.insert("frame_w".into(), Value::Number(0.7));
    assert_eq!(
        def.evaluate(&v, &instance(), &Values::new()),
        Err(FamilyError::Requirement {
            line: 26,
            message: "El marco no deja sitio para el vidrio".into()
        })
    );
    // A frame that leaves no room for three panes: the glass would have a
    // negative width.
    let mut v = window_values();
    v.insert("frame_w".into(), Value::Number(0.2));
    v.insert("width".into(), Value::Number(0.6));
    v.insert("leaves".into(), Value::Number(3.0));
    assert!(matches!(
        def.evaluate(&v, &instance(), &Values::new()),
        Err(FamilyError::NegativeSize { .. })
    ));
}

#[test]
fn visibility_follows_other_parameters() {
    let def = parse(WINDOW).unwrap();
    let mut v = window_values();
    assert!(!def.is_visible("box_h", &v));
    assert!(def.is_visible("width", &v));
    v.insert("shutter".into(), Value::Bool(true));
    assert!(def.is_visible("box_h", &v));
    assert!(!def.is_visible("nothing", &v));
}

#[test]
fn builtins_are_provided_by_the_caller() {
    let src = r#"
family "Hoja"
  category door
  host wall
type_params
  m material = "Madera"
body
  if opens_to == "exterior" and hand == "left"
    box (0, 0, 0) to (1, 1, 1) material m
"#;
    let def = parse(src).unwrap();
    let t = values(&[("m", Value::Material(1))]);
    let b = |o: &str, h: &str| {
        values(&[
            ("opens_to", Value::Text(o.into())),
            ("hand", Value::Text(h.into())),
        ])
    };
    let n = |o, h| {
        def.evaluate(&t, &Values::new(), &b(o, h))
            .unwrap()
            .parts
            .len()
    };
    assert_eq!(n("exterior", "left"), 1);
    assert_eq!(n("interior", "left"), 0);
    assert_eq!(n("exterior", "right"), 0);
    // Without them, the name is unknown.
    assert!(matches!(
        def.evaluate(&t, &Values::new(), &Values::new()),
        Err(FamilyError::Eval { line: 8, .. })
    ));
    // And a recipe cannot declare them.
    let bad = "family \"x\"\n  category door\ntype_params\n  hand text = \"left\"\n";
    assert!(matches!(
        parse(bad),
        Err(FamilyError::Syntax { line: 4, .. })
    ));
}

#[test]
fn includes_are_expanded_into_the_recipe() {
    let src = "family \"Con piezas\"\n  category door\ninclude \"piezas\"\nbody\n  use unit(m)\ntype_params\n  m material = \"x\"\n";
    let pieces = "component unit(mat)\n  box (0, 0, 0) to (1, 1, 1) material mat\n";
    let expanded =
        expand_includes(src, |name| (name == "piezas").then(|| pieces.to_string())).unwrap();
    assert!(expanded.contains("component unit(mat)"));
    assert!(!expanded.lines().any(|l| l.starts_with("include")));
    let def = parse(&expanded).unwrap();
    let ev = def
        .evaluate(
            &values(&[("m", Value::Material(3))]),
            &Values::new(),
            &Values::new(),
        )
        .unwrap();
    assert_eq!(ev.parts[0].max, [1.0, 1.0, 1.0]);
    // Unknown pieces and unexpanded includes are reported.
    assert!(expand_includes(src, |_| None).is_err());
    assert!(matches!(
        parse(src),
        Err(FamilyError::Syntax { line: 3, .. })
    ));
}

#[test]
fn syntax_errors_say_where() {
    let cases: &[(&str, usize)] = &[
        ("category door", 1),
        ("family \"x\"\n", 1),
        (
            "family \"x\"\n  category door\nbody\n  box (0, 0) to (1, 1, 1) material m\n",
            4,
        ),
        ("family \"x\"\n  category door\nbody\n  frobnicate\n", 4),
        ("family \"x\"\n  category door\nbody\n  use nothing()\n", 4),
        (
            "family \"x\"\n  category door\ntype_params\n  w length = 5 range 0..1\n",
            4,
        ),
        (
            "family \"x\"\n  category door\ntype_params\n  w length = \"a\"\n",
            4,
        ),
        (
            "family \"x\"\n  category door\ntype_params\n  w length = 1\n  w length = 2\n",
            5,
        ),
        (
            "family \"x\"\n  category door\nbody\n  else\n    let a = 1\n",
            4,
        ),
        ("family \"x\"\n  category door\nbody\n\tlet a = 1\n", 4),
        (
            "family \"x\"\n  category door\nbody\n    let a = 1\n  let b = \"x\n",
            5,
        ),
        (
            "family \"x\"\n  category door\ncomponent a()\n  use a()\nbody\n  use a()\n",
            4,
        ),
        ("family \"x\"\n  category door\nbody\n  let a = (1 + 2\n", 4),
        ("family \"x\"\n  category door\nbody\n  if yes\n", 4),
        ("family \"x\"\n  category door\n    host wall\n", 3),
    ];
    for (src, line) in cases {
        match parse(src) {
            Err(FamilyError::Syntax { line: l, .. }) => assert_eq!(l, *line, "{src}"),
            other => panic!("{src}: {other:?}"),
        }
    }
}

#[test]
fn runtime_errors_are_reported() {
    let run = |body: &str| {
        let src = format!("family \"x\"\n  category door\nbody\n{body}");
        parse(&src)
            .unwrap()
            .evaluate(&Values::new(), &Values::new(), &Values::new())
    };
    assert!(matches!(
        run("  let a = 1 / 0\n"),
        Err(FamilyError::Eval { line: 4, .. })
    ));
    assert!(matches!(
        run("  let a = sqrt(-1)\n"),
        Err(FamilyError::Eval { .. })
    ));
    assert!(matches!(
        run("  let a = 1 + \"x\"\n"),
        Err(FamilyError::Eval { .. })
    ));
    assert!(matches!(
        run("  let a = 1 == \"x\"\n"),
        Err(FamilyError::Eval { .. })
    ));
    assert!(matches!(
        run("  if 1\n    let a = 1\n"),
        Err(FamilyError::Eval { .. })
    ));
    assert!(matches!(
        run("  let a = 1\n  let a = 2\n"),
        Err(FamilyError::Eval { line: 5, .. })
    ));
    assert!(matches!(
        run("  box (0, 0, 0) to (1, 1, 1) material 3\n"),
        Err(FamilyError::Eval { .. })
    ));
    assert!(matches!(
        run("  repeat i in 0..1000000\n    let a = i\n"),
        Err(FamilyError::TooComplex { .. })
    ));
    assert!(matches!(
        run("  repeat i in 0..0.5\n    let a = i\n"),
        Err(FamilyError::Eval { .. })
    ));
    // Nested repeats cannot run forever either.
    assert!(matches!(
        run("  repeat i in 0..1000\n    repeat j in 0..1000\n      repeat k in 0..1000\n        let a = 1\n"),
        Err(FamilyError::TooComplex { .. })
    ));
    assert!(run("  let a = if 2 > 1 and not no then max(1, 4, 2) else 0\n").is_ok());
    assert!(run("  let a = \"SIDE\" + \"HUNG\"\n  require a == \"SIDEHUNG\" else \"x\"\n").is_ok());
}

#[test]
fn a_piece_sees_only_its_arguments() {
    let src = "family \"x\"\n  category door\ntype_params\n  w length = 1\ncomponent p()\n  let a = w\nbody\n  use p()\n";
    let def = parse(src).unwrap();
    let r = def.evaluate(
        &values(&[("w", Value::Number(1.0))]),
        &Values::new(),
        &Values::new(),
    );
    assert!(matches!(r, Err(FamilyError::Eval { line: 6, .. })));
}

#[test]
fn numbers_and_ranges_are_read_correctly() {
    let toks = lexer::tokenize("0.40..3.00 1..2 -0.5", 1).unwrap();
    use lexer::Tok;
    assert_eq!(
        toks,
        [
            Tok::Num(0.4),
            Tok::Sym(".."),
            Tok::Num(3.0),
            Tok::Num(1.0),
            Tok::Sym(".."),
            Tok::Num(2.0),
            Tok::Sym("-"),
            Tok::Num(0.5)
        ]
    );
    // Unary minus binds tighter than division, as in ordinary arithmetic.
    let src = "family \"x\"\n  category door\noutputs\n  a = -1 / 4 * 2 - 3\n  b = 2 - 3 - 4\n";
    let ev = parse(src)
        .unwrap()
        .evaluate(&Values::new(), &Values::new(), &Values::new())
        .unwrap();
    assert_eq!(ev.outputs["a"], Value::Number(-1.0 / 4.0 * 2.0 - 3.0));
    assert_eq!(ev.outputs["b"], Value::Number(-5.0));
}

const SYMBOL: &str = r#"
family "Hueco con símbolo"
  category window
  host wall
type_params
  width length = 1.0
  m material = "x"
component jamb(x, d)
  polygon (x, -d) (x + 0.05, -d) (x + 0.05, d) (x, d) pen frame filled
body
  box (0, 0, 0) to (1, 1, 1) material m
plan 1:50
  use jamb(-width / 2, 0.035)
  line (-width / 2, 0) to (width / 2, 0) pen thin
  if sill_height > 0
    line (-width / 2, wall_ext) to (width / 2, wall_ext) pen view dashed
  arc center (0, 0) from (0.5, 0) to (0, 0.5) pen thin
plan 1:100
  line (-width / 2, 0) to (width / 2, 0) pen cut
"#;

fn plan_builtins(sill: f64) -> Values {
    values(&[
        ("opens_to", Value::Text("interior".into())),
        ("hand", Value::Text("left".into())),
        ("wall_ext", Value::Number(-0.15)),
        ("wall_int", Value::Number(0.15)),
        ("sill_height", Value::Number(sill)),
    ])
}

#[test]
fn plan_symbols_are_chosen_by_the_nearest_scale() {
    let def = parse(SYMBOL).unwrap();
    assert_eq!(def.plans.len(), 2);
    assert_eq!(def.plan_for(50.0).unwrap().scale, 50.0);
    assert_eq!(def.plan_for(20.0).unwrap().scale, 50.0);
    assert_eq!(def.plan_for(75.0).unwrap().scale, 100.0);
    assert_eq!(def.plan_for(200.0).unwrap().scale, 100.0);
    // Without plan symbols nothing is drawn.
    let bare = parse("family \"x\"\n  category door\n").unwrap();
    assert!(bare.plan_for(50.0).is_none());
    assert_eq!(
        bare.evaluate_plan(&Values::new(), &Values::new(), &Values::new(), 50.0),
        Ok(Vec::new())
    );
}

#[test]
fn plan_symbols_draw_lines_outlines_and_arcs() {
    let def = parse(SYMBOL).unwrap();
    let t = values(&[("width", Value::Number(1.0)), ("m", Value::Material(1))]);
    let marks = def
        .evaluate_plan(&t, &Values::new(), &plan_builtins(0.9), 50.0)
        .unwrap();
    assert_eq!(marks.len(), 4);
    assert_eq!(
        marks[0],
        Mark::Polygon {
            points: vec![
                [-0.5, -0.035],
                [-0.45, -0.035],
                [-0.45, 0.035],
                [-0.5, 0.035]
            ],
            pen: Pen::Frame,
            style: LineStyle::Continuous,
            filled: true
        }
    );
    assert_eq!(
        marks[2],
        Mark::Line {
            from: [-0.5, -0.15],
            to: [0.5, -0.15],
            pen: Pen::View,
            style: LineStyle::Dashed
        }
    );
    assert!(matches!(marks[3], Mark::Arc { pen: Pen::Thin, .. }));
    // The condition on the sill.
    let marks = def
        .evaluate_plan(&t, &Values::new(), &plan_builtins(0.0), 50.0)
        .unwrap();
    assert_eq!(marks.len(), 3);
    // At 1:100 the simple symbol.
    let marks = def
        .evaluate_plan(&t, &Values::new(), &plan_builtins(0.0), 100.0)
        .unwrap();
    assert!(matches!(
        marks.as_slice(),
        [Mark::Line { pen: Pen::Cut, .. }]
    ));
    // The 3D body is unaffected by the plan blocks.
    let ev = def
        .evaluate(&t, &Values::new(), &plan_builtins(0.0))
        .unwrap();
    assert_eq!(ev.parts.len(), 1);
}

#[test]
fn plan_and_body_statements_stay_in_their_place() {
    let head = "family \"x\"\n  category door\ntype_params\n  m material = \"x\"\n";
    // A box in a plan, a line in the body: refused when reading.
    let e = parse(&format!(
        "{head}plan 1:50\n  box (0, 0, 0) to (1, 1, 1) material m\n"
    ));
    assert!(matches!(e, Err(FamilyError::Syntax { line: 6, .. })));
    let e = parse(&format!(
        "{head}body\n  if yes\n    line (0, 0) to (1, 0) pen thin\n"
    ));
    assert!(matches!(e, Err(FamilyError::Syntax { line: 7, .. })));
    // Through a piece, when it runs.
    let src =
        format!("{head}component mark()\n  line (0, 0) to (1, 0) pen thin\nbody\n  use mark()\n");
    let def = parse(&src).unwrap();
    let t = values(&[("m", Value::Material(1))]);
    assert!(matches!(
        def.evaluate(&t, &Values::new(), &Values::new()),
        Err(FamilyError::Eval { line: 6, .. })
    ));
    // Badly written marks.
    for (body, line) in [
        ("  line (0, 0) to (1, 0) pen gruesa\n", 6),
        ("  line (0, 0) to (1, 0)\n", 6),
        ("  polygon (0, 0) (1, 0) pen thin\n", 6),
        ("  arc (0, 0) from (1, 0) to (0, 1) pen thin\n", 6),
    ] {
        let e = parse(&format!("{head}plan 1:50\n{body}"));
        assert!(
            matches!(e, Err(FamilyError::Syntax { line: l, .. }) if l == line),
            "{body}: {e:?}"
        );
    }
    for bad in ["plan 50\n", "plan 2:50\n", "plan 1:50\nplan 1:50\n"] {
        assert!(parse(&format!("{head}{bad}")).is_err(), "{bad}");
    }
    // The plan names the program provides cannot be declared.
    let e = parse("family \"x\"\n  category door\ntype_params\n  sill_height length = 1\n");
    assert!(matches!(e, Err(FamilyError::Syntax { line: 4, .. })));
}

#[test]
fn line_styles_come_from_names_and_parameters() {
    let src = r#"
family "Estilos"
  category window
  host wall
type_params
  swing linestyle = "dashdot" label "Línea del giro"
plan 1:50
  line (0, 0) to (1, 0) pen thin
  line (0, 0) to (1, 0) pen thin dotted
  arc center (0, 0) from (1, 0) to (0, 1) pen thin style swing
  polygon (0, 0) (1, 0) (1, 1) pen frame style "longdash" filled
"#;
    let def = parse(src).unwrap();
    assert_eq!(def.param("swing").unwrap().kind, ParamKind::LineStyle);
    let t = values(&[("swing", Value::Text("dashdot".into()))]);
    let marks = def
        .evaluate_plan(&t, &Values::new(), &plan_builtins(0.0), 50.0)
        .unwrap();
    let styles: Vec<LineStyle> = marks
        .iter()
        .map(|m| match m {
            Mark::Line { style, .. } | Mark::Polygon { style, .. } | Mark::Arc { style, .. } => {
                *style
            }
        })
        .collect();
    assert_eq!(
        styles,
        [
            LineStyle::Continuous,
            LineStyle::Dotted,
            LineStyle::DashDot,
            LineStyle::LongDash
        ]
    );
    assert!(matches!(&marks[3], Mark::Polygon { filled: true, .. }));
    // A value that is not a style, in a parameter or written in a mark.
    let bad = values(&[("swing", Value::Text("ondulada".into()))]);
    assert!(matches!(
        def.evaluate_plan(&bad, &Values::new(), &plan_builtins(0.0), 50.0),
        Err(FamilyError::UnknownOption { .. })
    ));
    let e = parse("family \"x\"\n  category door\ntype_params\n  s linestyle = \"wavy\"\n");
    assert!(matches!(e, Err(FamilyError::Syntax { line: 4, .. })));
    let e = parse("family \"x\"\n  category door\nplan 1:50\n  line (0, 0) to (1, 0) pen thin style \"wavy\"\n")
        .unwrap()
        .evaluate_plan(&Values::new(), &Values::new(), &plan_builtins(0.0), 50.0);
    assert!(matches!(e, Err(FamilyError::Eval { line: 4, .. })));
    // Every style has a Spanish name and a pattern; only continuous is solid.
    for s in LineStyle::ALL {
        assert_eq!(LineStyle::from_code(s.code()), Some(s));
        assert!(!s.label().is_empty());
        assert_eq!(s.pattern_mm().is_empty(), s == LineStyle::Continuous);
    }
}

/// A library file keeps types with the recipe: written and read back the
/// same, and taken off again for the project's copy.
#[test]
fn catalogue_round_trip() {
    use crate::{catalog_text, parse, without_catalog, Catalog, CatalogMaterial, CatalogType};
    let recipe = "family \"Mesa\"\n  category furniture\n  host none\n\ntype_params\n  width length = 1.60 label \"Largo\"\n  top material = \"Roble\" label \"Tablero\"\n  round bool = no label \"Redonda\"\n\nbody\n  box (0, 0, 0) to (width, 0.9, 0.75) material top\n";
    let cat = Catalog {
        materials: vec![CatalogMaterial {
            name: "Roble \"natural\"".into(),
            category: "wood".into(),
            color: [180, 140, 90],
        }],
        types: vec![CatalogType {
            name: "Mesa 180".into(),
            mark: "M-01".into(),
            guid: "2K$S4$YmH0mRk6amryew4Z".into(),
            description: "Comedor\nseis plazas".into(),
            values: vec![
                ("width".into(), Value::Number(1.8)),
                ("top".into(), Value::Text("Roble \"natural\"".into())),
                ("round".into(), Value::Bool(true)),
                ("tiny".into(), Value::Number(0.000_001)),
                ("negative".into(), Value::Number(-2.5)),
            ],
        }],
    };
    let file = format!("{recipe}\n{}", catalog_text(&cat));
    let def = parse(&file).unwrap();
    assert_eq!(def.catalog, cat);
    // A comment does not start a catalogue; the project's copy has none.
    let copy = without_catalog(&file);
    assert!(!copy.contains("catalog"));
    assert_eq!(parse(&copy).unwrap().catalog, Catalog::default());
    assert_eq!(parse(&copy).unwrap().params, def.params);
    assert!(parse(&format!("{recipe}\ncatalog\n  shelf \"x\"\n")).is_err());
}

#[test]
fn elevations_draw_opening_lines_in_x_and_z() {
    let src = "family \"x\"\n  category window\n  host wall\n\ntype_params\n  width length = 1.0\n  height length = 1.2\n\nelevation\n  line (-width / 2, 0) to (width / 2, height / 2) pen thin\n  if hand == \"left\"\n    line (-width / 2, height) to (width / 2, height / 2) pen thin dashed\n";
    let def = parse(src).unwrap();
    let t = values(&[
        ("width", Value::Number(1.0)),
        ("height", Value::Number(1.2)),
    ]);
    let mut b = Values::new();
    b.insert("hand".into(), Value::Text("left".into()));
    let marks = def.evaluate_elevation(&t, &Values::new(), &b).unwrap();
    assert_eq!(marks.len(), 2);
    assert_eq!(
        marks[0],
        Mark::Line {
            from: [-0.5, 0.0],
            to: [0.5, 0.6],
            pen: Pen::Thin,
            style: LineStyle::Continuous
        }
    );
    // A box is not allowed in an elevation, nor two elevations.
    assert!(parse(&format!("{src}  box (0, 0, 0) to (1, 1, 1) material 1\n")).is_err());
    assert!(parse(&format!(
        "{src}\nelevation\n  line (0, 0) to (1, 1) pen thin\n"
    ))
    .is_err());
    // Without one, nothing is drawn.
    let bare = parse("family \"y\"\n  category door\n").unwrap();
    assert_eq!(
        bare.evaluate_elevation(&Values::new(), &Values::new(), &Values::new()),
        Ok(Vec::new())
    );
}
