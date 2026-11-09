//! Replays the generated golden scripts and compares the Rust frames to the
//! TypeScript ones. See `tools/golden/dump.ts` for how the contract is built.

use dot_lib::engine::{BotEngine, ExprId, Look, ShapeId, StateId};
use serde::Deserialize;

#[derive(Deserialize)]
struct Golden {
    scale: f64,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    init: Init,
    ops: Vec<Op>,
    t: f64,
    frame: Frame,
}

#[derive(Deserialize)]
struct Init {
    state: String,
    shape: Option<String>,
    expr: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
enum Op {
    #[serde(rename = "state")]
    State { id: String, at: f64 },
    #[serde(rename = "shape")]
    Shape { shape: Option<String>, at: f64 },
    #[serde(rename = "expr")]
    Expr { expr: Option<String>, at: f64 },
    #[serde(rename = "look")]
    Look {
        look: Option<LookDto>,
        at: f64,
        morph: Option<f64>,
    },
}

#[derive(Deserialize)]
struct LookDto {
    yaw: f64,
    pitch: f64,
    mix: f64,
    spin: f64,
    wander: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Frame {
    body_path: String,
    body_alpha: f64,
    eyes: Vec<EyeDto>,
    dots: Vec<DotDto>,
    dots_behind: bool,
    arcs: Vec<ArcDto>,
    notif: Option<CircDto>,
    notch: Option<CircDto>,
}

#[derive(Deserialize)]
struct EyeDto {
    d: String,
    matrix: String,
    alpha: f64,
}

#[derive(Deserialize)]
struct DotDto {
    x: f64,
    y: f64,
    r: f64,
    opacity: f64,
    color: Option<String>,
    depth: Option<f64>,
    /// Non-circular glyph path (the tear of the italic "!").
    d: Option<String>,
    rot: Option<f64>,
}

#[derive(Deserialize)]
struct ArcDto {
    id: String,
    front: String,
    back: String,
    width: f64,
    opacity: f64,
    grad: GradDto,
}

#[derive(Deserialize)]
struct GradDto {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    stops: Vec<String>,
}

#[derive(Deserialize)]
struct CircDto {
    x: f64,
    y: f64,
    r: f64,
}

/* ------------------------------------------------------------ comparison */

#[derive(PartialEq)]
enum Tok {
    Num(f64),
    Sep(char),
}

/// How many number-bearing strings differ byte-for-byte but agree numerically
/// (formatting/rounding boundaries only). Reported at the end of the run.
static RELAXED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn tokenize(s: &str) -> Vec<Tok> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() || c == '-' || c == '+' || c == '.' {
            let mut j = i + 1;
            while j < chars.len()
                && (chars[j].is_ascii_digit()
                    || chars[j] == '.'
                    || chars[j] == '-'
                    || chars[j] == '+'
                    || chars[j] == 'e'
                    || chars[j] == 'E')
            {
                j += 1;
            }
            let sub: String = chars[i..j].iter().collect();
            if let Ok(v) = sub.parse::<f64>() {
                out.push(Tok::Num(v));
                i = j;
                continue;
            }
        }
        out.push(Tok::Sep(c));
        i += 1;
    }
    out
}

fn nums_close(got: f64, want: f64, abs: f64, rel: f64) -> bool {
    let d = (got - want).abs();
    d <= abs + rel * want.abs()
}

/// Strings that embed numbers (paths, matrices): same command skeleton, then
/// numbers within 1e-3 (formatting/rounding boundaries differ).
fn num_str_eq(got: &str, want: &str) -> bool {
    let a = tokenize(got);
    let b = tokenize(want);
    if a.len() != b.len() {
        return false;
    }
    for (x, y) in a.iter().zip(b.iter()) {
        match (x, y) {
            (Tok::Sep(cx), Tok::Sep(cy)) if cx == cy => {}
            (Tok::Num(nx), Tok::Num(ny)) => {
                if !nums_close(*nx, *ny, 1e-3, 1e-3) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    if got != want {
        RELAXED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    true
}

fn cmp_f64(out: &mut Vec<String>, name: &str, got: f64, want: f64) {
    if !nums_close(got, want, 1e-6, 1e-6) {
        out.push(format!("{name}: got {got} want {want}"));
    }
}

fn cmp_str(out: &mut Vec<String>, name: &str, got: &str, want: &str) {
    if got != want {
        out.push(format!("{name}: got {got:?} want {want:?}"));
    }
}

fn cmp_num_str(out: &mut Vec<String>, name: &str, got: &str, want: &str) {
    if !num_str_eq(got, want) {
        out.push(format!("{name}: got {got:?} want {want:?}"));
    }
}

fn cmp_opt_num_str(out: &mut Vec<String>, name: &str, got: &Option<String>, want: &Option<String>) {
    match (got, want) {
        (None, None) => {}
        (Some(g), Some(w)) => cmp_num_str(out, name, g, w),
        _ => out.push(format!("{name}: got {got:?} want {want:?}")),
    }
}

fn cmp_circ(
    out: &mut Vec<String>,
    name: &str,
    got: Option<&dot_lib::engine::NotifFrame>,
    want: &Option<CircDto>,
) {
    match (got, want) {
        (None, None) => {}
        (Some(g), Some(w)) => {
            cmp_f64(out, &format!("{name}.x"), g.x, w.x);
            cmp_f64(out, &format!("{name}.y"), g.y, w.y);
            cmp_f64(out, &format!("{name}.r"), g.r, w.r);
        }
        _ => out.push(format!("{name}: presence differs")),
    }
}

/* ---------------------------------------------------------------- replay */

fn replay(case: &Case, scale: f64) -> dot_lib::engine::BotFrame {
    let initial = StateId::from_id(&case.init.state).expect("state id");
    let shape = case
        .init
        .shape
        .as_deref()
        .map(|s| ShapeId::from_id(s).expect("shape id"));
    let expr = case
        .init
        .expr
        .as_deref()
        .map(|s| ExprId::from_id(s).expect("expr id"));
    let mut engine = BotEngine::new(scale, initial, shape, expr);
    engine.reset(initial, 0.0);
    for op in &case.ops {
        match op {
            Op::State { id, at } => {
                engine.set_state(StateId::from_id(id).expect("state id"), *at);
            }
            Op::Shape { shape, at } => {
                let shape = shape
                    .as_deref()
                    .map(|s| ShapeId::from_id(s).expect("shape id"));
                engine.set_shape(shape, *at);
            }
            Op::Expr { expr, at } => {
                let expr = expr
                    .as_deref()
                    .map(|s| ExprId::from_id(s).expect("expr id"));
                engine.set_expression(expr, *at);
            }
            Op::Look { look, at, morph } => {
                engine.set_look(
                    look.as_ref().map(|l| Look {
                        yaw: l.yaw,
                        pitch: l.pitch,
                        mix: l.mix,
                        spin: l.spin,
                        wander: l.wander,
                    }),
                    *at,
                    morph.unwrap_or(BotEngine::LOOK_MORPH),
                );
            }
        }
    }
    engine.sample(case.t)
}

fn check(case: &Case, got: &dot_lib::engine::BotFrame) -> Vec<String> {
    let mut out = Vec::new();
    let want = &case.frame;

    cmp_num_str(&mut out, "bodyPath", &got.body_path, &want.body_path);
    cmp_f64(&mut out, "bodyAlpha", got.body_alpha, want.body_alpha);

    if got.eyes.len() != want.eyes.len() {
        out.push(format!(
            "eyes.len: got {} want {}",
            got.eyes.len(),
            want.eyes.len()
        ));
    } else {
        for (i, (g, w)) in got.eyes.iter().zip(want.eyes.iter()).enumerate() {
            cmp_num_str(&mut out, &format!("eyes[{i}].d"), &g.d, &w.d);
            cmp_num_str(&mut out, &format!("eyes[{i}].matrix"), &g.matrix, &w.matrix);
            cmp_f64(&mut out, &format!("eyes[{i}].alpha"), g.alpha, w.alpha);
        }
    }

    if got.dots.len() != want.dots.len() {
        out.push(format!(
            "dots.len: got {} want {}",
            got.dots.len(),
            want.dots.len()
        ));
    } else {
        for (i, (g, w)) in got.dots.iter().zip(want.dots.iter()).enumerate() {
            let p = format!("dots[{i}]");
            cmp_f64(&mut out, &format!("{p}.x"), g.x, w.x);
            cmp_f64(&mut out, &format!("{p}.y"), g.y, w.y);
            cmp_f64(&mut out, &format!("{p}.r"), g.r, w.r);
            cmp_f64(&mut out, &format!("{p}.opacity"), g.opacity, w.opacity);
            cmp_str(
                &mut out,
                &format!("{p}.color"),
                &format!("{:?}", g.color),
                &format!("{:?}", w.color),
            );
            match (g.depth, w.depth) {
                (None, None) => {}
                (Some(a), Some(b)) => cmp_f64(&mut out, &format!("{p}.depth"), a, b),
                _ => out.push(format!("{p}.depth presence differs")),
            }
            cmp_opt_num_str(&mut out, &format!("{p}.d"), &g.d, &w.d);
            match (g.rot, w.rot) {
                (None, None) => {}
                (Some(a), Some(b)) => cmp_f64(&mut out, &format!("{p}.rot"), a, b),
                _ => out.push(format!("{p}.rot presence differs")),
            }
        }
    }

    if got.dots_behind != want.dots_behind {
        out.push(format!(
            "dotsBehind: got {} want {}",
            got.dots_behind, want.dots_behind
        ));
    }

    if got.arcs.len() != want.arcs.len() {
        out.push(format!(
            "arcs.len: got {} want {}",
            got.arcs.len(),
            want.arcs.len()
        ));
    } else {
        for (i, (g, w)) in got.arcs.iter().zip(want.arcs.iter()).enumerate() {
            let p = format!("arcs[{i}]");
            cmp_str(&mut out, &format!("{p}.id"), &g.id, &w.id);
            cmp_num_str(&mut out, &format!("{p}.front"), &g.front, &w.front);
            cmp_num_str(&mut out, &format!("{p}.back"), &g.back, &w.back);
            cmp_f64(&mut out, &format!("{p}.width"), g.width, w.width);
            cmp_f64(&mut out, &format!("{p}.opacity"), g.opacity, w.opacity);
            cmp_f64(&mut out, &format!("{p}.grad.x1"), g.grad.x1, w.grad.x1);
            cmp_f64(&mut out, &format!("{p}.grad.y1"), g.grad.y1, w.grad.y1);
            cmp_f64(&mut out, &format!("{p}.grad.x2"), g.grad.x2, w.grad.x2);
            cmp_f64(&mut out, &format!("{p}.grad.y2"), g.grad.y2, w.grad.y2);
            if g.grad.stops != w.grad.stops {
                out.push(format!(
                    "{p}.grad.stops: got {:?} want {:?}",
                    g.grad.stops, w.grad.stops
                ));
            }
        }
    }

    cmp_circ(&mut out, "notif", got.notif.as_ref(), &want.notif);
    cmp_circ(&mut out, "notch", got.notch.as_ref(), &want.notch);

    out
}

#[test]
fn golden_frames_replay_exactly() {
    let golden: Golden = serde_json::from_str(include_str!("engine-golden.json"))
        .expect("golden contract must parse");
    assert!(!golden.cases.is_empty());

    let mut failures: Vec<String> = Vec::new();
    let mut failed_cases = 0usize;

    for case in &golden.cases {
        let frame = replay(case, golden.scale);
        let problems = check(case, &frame);
        if !problems.is_empty() {
            failed_cases += 1;
            for problem in problems.iter().take(6) {
                failures.push(format!("{}: {}", case.name, problem));
            }
        }
    }

    if !failures.is_empty() {
        for line in failures.iter().take(40) {
            eprintln!("{line}");
        }
        panic!(
            "{} of {} cases failed ({} mismatches shown max)",
            failed_cases,
            golden.cases.len(),
            failures.len()
        );
    }

    eprintln!(
        "all {} parity cases pass ({} number-bearing strings matched only under 1e-3 tolerance)",
        golden.cases.len(),
        RELAXED.load(std::sync::atomic::Ordering::Relaxed)
    );
}
