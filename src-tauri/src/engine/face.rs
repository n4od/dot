//! Port of `src/bot/face.ts`.

use super::math::{clamp, loop_noise};
use super::{catalogue, EyePose};

pub const EYE_SPLIT: f64 = 15.46;
pub const EYE_W: f64 = 0.186;
pub const EYE_H: f64 = 0.412;

#[derive(Clone, Copy, Debug, serde::Deserialize)]
pub struct HeadGaze {
    pub yaw: f64,
    pub pitch: f64,
    pub roll: f64,
}

pub const REST_GAZE: HeadGaze = HeadGaze {
    yaw: 28.49,
    pitch: 28.62,
    roll: -13.0,
};

fn deg(d: f64) -> f64 {
    (d * std::f64::consts::PI) / 180.0
}

type Vec3 = [f64; 3];

/// Rotate two vectors of an orthonormal frame inside their common plane.
fn spin(u: Vec3, v: Vec3, angle: f64) -> (Vec3, Vec3) {
    let c = angle.cos();
    let s = angle.sin();
    (
        [
            u[0] * c + v[0] * s,
            u[1] * c + v[1] * s,
            u[2] * c + v[2] * s,
        ],
        [
            v[0] * c - u[0] * s,
            v[1] * c - u[1] * s,
            v[2] * c - u[2] * s,
        ],
    )
}

/// Head frame, then the two eyes. Index 0 is the inner eye.
pub fn eye_poses(gaze: &HeadGaze, scale: f64, split: f64) -> [EyePose; 2] {
    let mut f: Vec3 = [0.0, 0.0, 1.0];
    let mut right: Vec3 = [1.0, 0.0, 0.0];
    let mut down: Vec3 = [0.0, 1.0, 0.0];

    let (nf, nr) = spin(f, right, deg(gaze.yaw));
    f = nf;
    right = nr;
    let (nd, nf2) = spin(down, f, deg(gaze.pitch));
    down = nd;
    f = nf2;
    let (nr2, nd2) = spin(right, down, deg(gaze.roll));
    right = nr2;
    down = nd2;

    let build = |side: f64| -> EyePose {
        let (ef, er) = spin(f, right, deg(split * side));
        EyePose {
            x: ef[0] * scale,
            y: ef[1] * scale,
            a: er[0],
            b: er[1],
            c: down[0],
            d: down[1],
            depth: ef[2],
        }
    };

    [build(-1.0), build(1.0)]
}

#[derive(Clone, Copy, Debug)]
pub struct Liveliness {
    pub d_yaw: f64,
    pub d_pitch: f64,
    pub d_roll: f64,
    pub lid: f64,
    pub drift_x: f64,
    pub drift_y: f64,
    pub breath: f64,
}

/// Measured: 1 to 2 frames at 10 fps.
const BLINK_DUR: f64 = 0.18;

fn blink_lid(t: f64) -> f64 {
    let blinks = &catalogue().blinks;
    for &start in blinks {
        if t < start {
            break;
        }
        let k = (t - start) / BLINK_DUR;
        if (0.0..=1.0).contains(&k) {
            return if k < 0.45 {
                1.0 - k / 0.45
            } else {
                (k - 0.45) / 0.55
            };
        }
    }
    1.0
}

pub fn liveliness(t: f64, wander: f64, blink: bool, float: bool) -> Liveliness {
    Liveliness {
        d_yaw: (loop_noise(t, 11.3, 0.4) * 5.5 + loop_noise(t, 3.7, 2.1) * 1.6) * wander,
        d_pitch: (loop_noise(t, 9.1, 1.3) * 4.2 + loop_noise(t, 4.3, 0.7) * 1.3) * wander,
        d_roll: loop_noise(t, 13.7, 3.2) * 2.2 * wander,
        lid: if blink { blink_lid(t) } else { 1.0 },
        drift_x: if float {
            loop_noise(t, 7.9, 1.9) * 0.006
        } else {
            0.0
        },
        drift_y: if float {
            loop_noise(t, 5.3, 0.3) * 0.007
        } else {
            0.0
        },
        breath: if float {
            1.0 + ((t / 3.4) * std::f64::consts::PI * 2.0).sin() * 0.005
        } else {
            1.0
        },
    }
}

/// Blink is a vertical squash in screen space, applied after the tangent matrix.
pub fn blink_scale(lid: f64) -> f64 {
    0.06 + 0.94 * clamp(lid)
}
