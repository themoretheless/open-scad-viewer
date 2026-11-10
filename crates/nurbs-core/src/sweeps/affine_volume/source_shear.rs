//! Exact paired source/presentation change, never a fitted replacement carrier.
//! Q'=S Q, o'=S o, A'=A S^-1, hence T+A'(Q'-o') == T+A(Q-o).
use super::*;
use crate::numerics::compensated::two_sum;
#[derive(Clone, Debug)]
pub struct SourceShearReport {
    pub base: usize,
    pub dependent: usize,
    pub scale: f64,
}
enum OwnedMotion {
    Polynomial(Vec<[Pose; 6]>),
    Rational(Vec<RationalPiece>),
}
pub(super) struct Derived {
    pub carrier: Surface,
    pub origin: [f64; 3],
    motion: OwnedMotion,
    pub report: SourceShearReport,
}
impl Derived {
    pub fn motion(&self) -> Motion<'_> {
        match &self.motion {
            OwnedMotion::Polynomial(p) => Motion::Quintic(p),
            OwnedMotion::Rational(p) => Motion::Rational(p),
        }
    }
}
fn subtract(a: f64, b: f64) -> Option<f64> {
    let (v, e) = two_sum(a, -b);
    (v.is_finite() && e == 0.).then_some(v)
}
fn multiply(a: f64, b: f64) -> Option<f64> {
    if a == 0. || b == 0. {
        return Some(0.);
    }
    if a == 1. {
        return Some(b);
    }
    if b == 1. {
        return Some(a);
    }
    let v = a * b;
    (v.is_finite()
        && a.abs() >= 1e-100
        && b.abs() >= 1e-100
        && v.abs() >= 1e-100
        && a.mul_add(b, -v) == 0.)
        .then_some(v)
}
/// Every failed exact comparison spends caller work. The width heuristic only
/// selects proposals; invertibility of S is exact (det S=1), not inferred from
/// that heuristic. All original time pieces/weights/knots are retained.
pub(super) fn prepare(
    carrier: &Surface,
    origin: [f64; 3],
    pieces: Motion<'_>,
    work: &mut u64,
    max: u64,
) -> std::result::Result<Option<Derived>, ()> {
    macro_rules! charge {
        ($n:expr) => {
            if max - *work < $n {
                return Err(());
            }
            *work += $n;
        };
    }
    let first = pieces.controls(0)[0];
    for base in 0..3 {
        for dependent in 0..3 {
            if base == dependent {
                continue;
            }
            for axis in 0..3 {
                charge!(1);
                if first[base][axis] == 0. {
                    continue;
                }
                let scale = first[dependent][axis] / first[base][axis];
                if !scale.is_finite() || scale == 0. || scale.abs() > 1e9 {
                    continue;
                }
                let mut derived = carrier.clone();
                let mut valid = true;
                let mut old_bounds = [f64::INFINITY, f64::NEG_INFINITY];
                let mut new_bounds = old_bounds;
                for (before, after) in carrier
                    .control_points
                    .iter()
                    .flatten()
                    .zip(derived.control_points.iter_mut().flatten())
                {
                    charge!(3);
                    let Some(value) =
                        multiply(scale, before[dependent]).and_then(|v| subtract(before[base], -v))
                    else {
                        valid = false;
                        break;
                    };
                    if value.abs() > 1e9 {
                        valid = false;
                        break;
                    }
                    after[base] = value;
                    old_bounds[0] = old_bounds[0].min(before[base]);
                    old_bounds[1] = old_bounds[1].max(before[base]);
                    new_bounds[0] = new_bounds[0].min(value);
                    new_bounds[1] = new_bounds[1].max(value);
                }
                if !valid || new_bounds[1] - new_bounds[0] >= (old_bounds[1] - old_bounds[0]) * 0.5
                {
                    continue;
                }
                charge!(3);
                let Some(value) =
                    multiply(scale, origin[dependent]).and_then(|v| subtract(origin[base], -v))
                else {
                    continue;
                };
                if value.abs() > 1e9 {
                    continue;
                }
                let mut new_origin = origin;
                new_origin[base] = value;
                let mut rows = Vec::new();
                'pieces: for i in 0..pieces.len() {
                    let mut controls = Vec::new();
                    for original in pieces.controls(i) {
                        let mut pose = *original;
                        for k in 0..3 {
                            charge!(3);
                            let Some(value) = multiply(scale, original[base][k])
                                .and_then(|v| subtract(original[dependent][k], v))
                            else {
                                valid = false;
                                break 'pieces;
                            };
                            if value.abs() > 1e9 {
                                valid = false;
                                break 'pieces;
                            }
                            pose[dependent][k] = value;
                        }
                        controls.push(pose);
                    }
                    rows.push(controls);
                }
                if !valid {
                    continue;
                }
                let motion = match pieces {
                    Motion::Quintic(_) => OwnedMotion::Polynomial(
                        rows.into_iter().map(|p| p.try_into().unwrap()).collect(),
                    ),
                    Motion::Rational(original) => OwnedMotion::Rational(
                        rows.into_iter()
                            .zip(original)
                            .map(|(controls, p)| RationalPiece {
                                controls,
                                weights: p.weights.clone(),
                            })
                            .collect(),
                    ),
                };
                return Ok(Some(Derived {
                    carrier: derived,
                    origin: new_origin,
                    motion,
                    report: SourceShearReport {
                        base,
                        dependent,
                        scale,
                    },
                }));
            }
        }
    }
    Ok(None)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paired_shear_arithmetic_refuses_rounded_residual_and_gradual_underflow() {
        assert_eq!(multiply(8., 0.125), Some(1.));
        assert_eq!(multiply(f64::from_bits(1), f64::from_bits(1)), None);
        assert_eq!(subtract(1., f64::from_bits(1)), None);
        assert_eq!(subtract(8.125, 8.), Some(0.125));
    }
}
