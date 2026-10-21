//! Explicit cubic wave graphs, not a sinusoidal approximation or a solid.
use crate::{Result, check, surface::Surface};

/// A C1 wave graph over an axis-aligned rectangle. Each half-period is the
/// cubic Hermite transition between +amplitude and -amplitude with zero slope
/// at both ends. x=origin.x+width*u, y=origin.y+length*v, u,v in [0,1].
/// No sampling, fitting, or trigonometric evaluation is used. Construction is
/// exact in real arithmetic; binary64 control rounding is not certified here.
/// Joining two periods preserves C1, not C2. Returns an open surface, no caps.
pub fn cubic(
    origin: [f64; 3],
    width: f64,
    length: f64,
    amplitude: f64,
    periods: usize,
) -> Result<Surface> {
    check(
        origin.iter().all(|x| x.is_finite())
            && width.is_finite()
            && width > 0.
            && length.is_finite()
            && length > 0.
            && amplitude.is_finite()
            && (1..=5).contains(&periods),
        "Cubic wave requires finite dimensions, positive width/length and 1..5 periods",
    )?;
    let pieces = periods * 2;
    let mut controls = Vec::with_capacity(3 * pieces + 1);
    for i in 0..=3 * pieces {
        let segment = (i / 3).min(pieces - 1);
        let local = i - segment * 3;
        let start = if segment % 2 == 0 {
            amplitude
        } else {
            -amplitude
        };
        let z = origin[2] + if local <= 1 { start } else { -start };
        let x = origin[0] + width * (i as f64 / (3 * pieces) as f64);
        controls.push(vec![vec![x, origin[1], z], vec![x, origin[1] + length, z]]);
    }
    check(
        controls
            .windows(2)
            .all(|rows| rows[0][0][0] < rows[1][0][0])
            && origin[1] + length > origin[1],
        "Wave dimensions collapse at coordinate precision",
    )?;
    let mut knots_u = vec![0.; 4];
    for i in 1..pieces {
        knots_u.extend([i as f64 / pieces as f64; 3]);
    }
    knots_u.extend([1.; 4]);
    let s = Surface {
        degree_u: 3,
        degree_v: 1,
        knots_u,
        knots_v: vec![0., 0., 1., 1.],
        weights: vec![vec![1.; 2]; controls.len()],
        control_points: controls,
        periodic_u: false,
        periodic_v: false,
    };
    s.validate()?;
    Ok(s)
}
