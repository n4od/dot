//! Port of `src/bot/math.ts`. Kept formula-for-formula so the frames match.

pub const TAU: f64 = std::f64::consts::PI * 2.0;

/// JS `clamp(v, lo = 0, hi = 1)`.
pub fn clamp(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

pub fn ease_out_cubic(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(3)
}

pub fn ease_in_out_cubic(t: f64) -> f64 {
    if t < 0.5 {
        4.0 * t.powi(3)
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

pub fn ease_out_quint(t: f64) -> f64 {
    1.0 - (1.0 - t).powi(5)
}

/// Periodic 1D noise, seamless over `period`.
pub fn loop_noise(t: f64, period: f64, seed: f64) -> f64 {
    let p = (t / period) * TAU;
    0.55 * (p + seed).sin()
        + 0.3 * (2.0 * p + seed * 1.7 + 1.1).sin()
        + 0.15 * (3.0 * p + seed * 2.3 + 2.4).sin()
}

/// Short round, matching JS `Math.round` (half toward +Infinity) rather than
/// Rust's `round` (half away from zero).
pub fn r2(v: f64) -> f64 {
    ((v * 100.0) + 0.5).floor() / 100.0
}

/// JS `Math.round` for the places where an exact half must go up.
pub fn js_round(v: f64) -> f64 {
    (v + 0.5).floor()
}
