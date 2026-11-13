//! Every face the engine can make, painted by the **shipped** native painter.
//!
//!   cargo run --manifest-path src-tauri/Cargo.toml --example faces-sheet
//!
//! Writes into `%TEMP%\dots-mouth\` and prints that directory: the eight
//! silhouettes at 512/180/96/42/24 px, the sixteen expressions, and the twelve
//! states that have a face — or deliberately none. It is how a change to the
//! mouth (or to any face element) is judged in pixels instead of prose, in both
//! contrast directions (`plat` ink, `peluche` cream).
//!
//! Nothing here feeds the app, and nothing asserts: this is the eye, not a gate.
//! The gate is `cargo test`'s parity suite, which these renders must agree with.

use dot_lib::engine::{BotEngine, ExprId, ShapeId, StateId};
use dot_lib::render;
use tiny_skia::Pixmap;

const SHAPES: [&str; 8] = [
    "cercle", "galet", "squircle", "capsule", "triangle", "hexagone", "nuage", "goutte",
];

const EXPRESSIONS: [&str; 16] = [
    "neutre",
    "attentif",
    "surpris",
    "excite",
    "heureux",
    "hilare",
    "colere",
    "triste",
    "effraye",
    "mefiant",
    "confus",
    "curieux",
    "fier",
    "timide",
    "blase",
    "somnolent",
];

/// Each state at a local time where it reads, including the ones that must show
/// no face at all.
const STATES: [(&str, f64); 12] = [
    ("idle", 0.7),
    ("wink", 0.6),
    ("wide", 0.8),
    ("think", 1.0),
    ("notify", 0.8),
    ("exclaim", 1.0),
    ("sleep", 1.0),
    ("egg", 0.6),
    ("hexagon", 0.6),
    ("play", 1.0),
    ("orbit", 1.2),
    ("burst", 2.34),
];

fn paint(
    frame: &dot_lib::engine::BotFrame,
    size: u32,
    file: std::path::PathBuf,
    style: &str,
    ink: &str,
) {
    let mut pix = Pixmap::new(size, size).expect("pixmap");
    render::paint_into(&mut pix, frame, ink, style);
    pix.save_png(file).expect("save png");
}

fn main() {
    let out = std::env::temp_dir().join("dots-mouth");
    std::fs::create_dir_all(&out).expect("out dir");

    // 512 crops in for inspection; 180 is the pet window; 96 is a tray-ish icon; 42 and
    // 24 are the expression thumbnails.
    let sizes = [512u32, 180, 96, 42, 24];
    // The two ends of the contrast range: light on dark, dark on light.
    let skins = [("plat", "#0a0a0c"), ("peluche", "#f1efe9")];

    for (style, ink) in skins {
        let cercle = ShapeId::from_id("cercle").expect("shape id");

        for name in SHAPES {
            let shape = ShapeId::from_id(name).expect("shape id");
            let engine = BotEngine::new(render::RAYON, StateId::Idle, Some(shape), None);
            let frame = engine.sample(0.7);
            for &size in &sizes {
                let file = out.join(format!("{style}-{name}-{size}.png"));
                paint(&frame, size, file, style, ink);
            }
        }

        for id in EXPRESSIONS {
            let expr = ExprId::from_id(id).expect("expr id");
            let engine = BotEngine::new(render::RAYON, StateId::Idle, Some(cercle), Some(expr));
            let frame = engine.sample(0.7);
            for &size in &[512u32, 180] {
                let file = out.join(format!("{style}-expr-{id}-{size}.png"));
                paint(&frame, size, file, style, ink);
            }
        }

        for (name, t) in STATES {
            let id = match name {
                "think" => StateId::Thinking,
                other => StateId::from_id(other).expect("state id"),
            };
            let engine = BotEngine::new(render::RAYON, id, Some(cercle), None);
            let frame = engine.sample(t);
            let file = out.join(format!("{style}-state-{name}-512.png"));
            paint(&frame, 512, file, style, ink);
        }
    }

    println!("{}", out.display());
}
