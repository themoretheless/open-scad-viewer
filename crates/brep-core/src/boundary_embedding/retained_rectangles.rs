//! Exact proof-only restrictions of linear rational tensor directions.
//! The retained model, clocks, pcurves, edges and identities are never edited.
use super::*;
use nurbs_core::surface::Surface;
#[derive(Clone, Debug)]
pub struct RectangleRestriction {
    pub face: usize,
    pub axis: usize,
    pub original: [f64; 2],
    pub retained: [f64; 2],
    pub checked_coordinates: usize,
}
pub struct RetainedRectangleReport {
    pub proven: bool,
    /// Face indices refer to the original model; only proof supports differ.
    pub proof: Report,
    pub restrictions: Vec<RectangleRestriction>,
    pub coefficient_checks: usize,
}
fn sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bv = s - a;
    let av = s - bv;
    (s, (a - av) + (b - bv))
}
fn range(x: f64) -> bool {
    x.is_finite() && (x == 0. || (1e-100..=1e100).contains(&x.abs()))
}
fn lerp(a: f64, b: f64, t: f64) -> Option<f64> {
    if !range(a) || !range(b) || !range(t) {
        return None;
    }
    if t == 0. {
        return Some(a);
    }
    if t == 1. {
        return Some(b);
    }
    let (d, e) = sum(b, -a);
    if e != 0. || !range(d) {
        return None;
    }
    let p = d * t;
    if !p.is_finite() || d.mul_add(t, -p) != 0. {
        return None;
    }
    let (s, e) = sum(a, p);
    (s.is_finite() && e == 0.).then_some(s)
}
fn restrict(
    s: &Surface,
    axis: usize,
    r: [f64; 2],
    checks: &mut usize,
    max: usize,
) -> Option<Surface> {
    let degree = if axis == 0 { s.degree_u } else { s.degree_v };
    let n = if axis == 0 {
        s.control_points.len()
    } else {
        s.control_points[0].len()
    };
    let knots = if axis == 0 { &s.knots_u } else { &s.knots_v };
    if degree != 1
        || n != 2
        || knots != &[0., 0., 1., 1.]
        || s.periodic_u
        || s.periodic_v
        || !(0. <= r[0] && r[0] < r[1] && r[1] <= 1.)
        || s.weights
            .iter()
            .flatten()
            .any(|w| !w.is_finite() || *w <= 0.)
    {
        return None;
    }
    let free = if axis == 0 {
        s.control_points[0].len()
    } else {
        s.control_points.len()
    };
    let mut out = s.clone();
    for j in 0..free {
        let (a, b, wa, wb) = if axis == 0 {
            (
                &s.control_points[0][j],
                &s.control_points[1][j],
                s.weights[0][j],
                s.weights[1][j],
            )
        } else {
            (
                &s.control_points[j][0],
                &s.control_points[j][1],
                s.weights[j][0],
                s.weights[j][1],
            )
        };
        if wa != wb || a.len() != 3 || b.len() != 3 {
            return None;
        }
        for (i, t) in r.into_iter().enumerate() {
            for k in 0..3 {
                if *checks >= max {
                    return None;
                }
                *checks += 1;
                let p = lerp(a[k], b[k], t)?;
                if axis == 0 {
                    out.control_points[i][j][k] = p;
                } else {
                    out.control_points[j][i][k] = p;
                }
            }
        }
    }
    if axis == 0 {
        out.knots_u = vec![r[0], r[0], r[1], r[1]];
    } else {
        out.knots_v = vec![r[0], r[0], r[1], r[1]];
    }
    Some(out)
}
/// Qualify the actual retained face sets using exact proof-only subcharts.
/// Only a literal rectangular trim in a clamped unit linear tensor direction
/// with equal cross-direction weights is eligible. Every changed pole is an
/// exactly representable affine interpolation (guarded TwoSum/FMA checks).
/// The positive rational denominator is independent of that linear direction,
/// so the restricted surface equals the original on the complete rectangle.
/// Pcurves/3D curves/topology are unchanged, and the complete embedding audit is
/// rerun on this equivalent model. Unsupported or rounded restrictions do not
/// obtain a receipt; the unchanged-support audit remains free to refuse.
pub fn inspect_retained_rectangles(
    model: &Model,
    tolerance_uv: f64,
    limits: Limits,
    max_linear_cells: usize,
    max_coefficients: usize,
) -> Result<RetainedRectangleReport> {
    model.validate()?;
    if !(1..=1_000_000).contains(&max_coefficients) {
        return Err(Error::new(
            "BREP_RETAINED_RECTANGLE_BUDGET",
            "Need bounded exact coefficient checks",
        ));
    }
    let mut candidate = model.clone();
    let mut restrictions = Vec::new();
    let mut checks = 0;
    for face in 0..model.faces.len() {
        let Some(rect) =
            face_domain::FaceDomain::new(model, face, tolerance_uv)?.literal_rectangle()
        else {
            continue;
        };
        for axis in 0..2 {
            if rect[axis] == [0., 1.] {
                continue;
            }
            let before = checks;
            if let Some(surface) = restrict(
                &candidate.faces[face].surface,
                axis,
                rect[axis],
                &mut checks,
                max_coefficients,
            ) {
                candidate.faces[face].surface = surface;
                restrictions.push(RectangleRestriction {
                    face,
                    axis,
                    original: [0., 1.],
                    retained: rect[axis],
                    checked_coordinates: checks - before,
                });
            }
        }
    }
    let proof = super::inspect_with_linear(&candidate, tolerance_uv, limits, max_linear_cells)?;
    Ok(RetainedRectangleReport {
        proven: proof.proven,
        proof,
        restrictions,
        coefficient_checks: checks,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_affine_coordinates_only() {
        assert_eq!(lerp(0.5, 4.5, 0.25), Some(1.5));
        assert!(lerp(1., f64::from_bits(1f64.to_bits() + 1), 0.25).is_none());
        assert!(lerp(1e-200, 2e-200, 0.25).is_none());
    }
    #[test]
    fn rational_weight_or_work_changes_do_not_inherit_restriction() {
        let m = crate::cuboid([0.; 3], [4.; 3]).unwrap();
        let s = &m.faces[0].surface;
        let mut work = 0;
        assert!(restrict(s, 0, [0.25, 1.], &mut work, 1000).is_some());
        let mut different = s.clone();
        different.weights[1][0] = 2.;
        assert!(restrict(&different, 0, [0.25, 1.], &mut work, 1000).is_none());
        assert!(restrict(s, 0, [0.25, 1.], &mut 0, 0).is_none());
    }
}
