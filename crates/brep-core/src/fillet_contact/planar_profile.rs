//! Explicit planar G2 profile proposal; all contact/G2/curvature receipts are fresh.
use nurbs_core::{Error, Result, surface::Surface};
fn fail(s: &str) -> Error {
    Error::new("BREP_PLANAR_G2_PROFILE", s)
}
pub(super) enum BoundedKind {Cubic,Matched,Quartic}
pub(super) fn refit_bounded(
    s: &Surface,
    supports: [&Surface; 2],
    radius: f64,
    maximum: f64,
    tolerance: f64,
    kind: BoundedKind,
) -> Result<Surface> {
    for p in supports {
        p.validate()?;
    }
    s.validate()?;
    if supports.iter().any(|p| p.control_points[0][0].len() != 3)
        || s.control_points[0][0].len() != 3
    {
        return Err(fail("Bounded profile requires XYZ supports and skin"));
    }
    let plane = |p: &Surface| {
        (0..3).find_map(|k| {
            let value = p.control_points[0][0][k];
            p.control_points
                .iter()
                .flatten()
                .all(|x| x.len() == 3 && x[k] == value)
                .then_some((k, value))
        })
    };
    let [Some(a), Some(b)] = supports.map(plane) else {
        return Err(fail("Bounded profile requires literal coordinate planes"));
    };
    if a.0 == b.0 || s.control_points[0][0].len() != 3 {
        return Err(fail("Need distinct XYZ coordinate planes"));
    }
    let p0 = &s.control_points[0][0];
    let p1 = &s.control_points[s.control_points.len() - 1][0];
    if p0[a.0] != a.1 || p1[b.0] != b.1 {
        return Err(fail("Original profile endpoints leave support planes"));
    }
    let signs = [(p1[a.0] - a.1).signum(), (p0[b.0] - b.1).signum()];
    if signs.contains(&0.) {
        return Err(fail("Degenerate bounded profile footprint"));
    }
    let proposal =
        nurbs_core::construct::curves::bounded_corner::propose(radius, maximum, tolerance)?;
    let c = match kind {
        BoundedKind::Matched=>proposal.composite_matched_g2()?,
        BoundedKind::Cubic=>proposal.composite_cubic()?,
        BoundedKind::Quartic=>proposal.quartic_g2()?,
    };
    let free = 3 - a.0 - b.0;
    let axial = [
        s.control_points[0][0][free],
        s.control_points[0].last().unwrap()[free],
    ];
    if axial[0] == axial[1] {
        return Err(fail("Zero bounded profile extrusion span"));
    }
    let points = c
        .control_points
        .iter()
        .map(|p| {
            (0..2)
                .map(|j| {
                    let mut q = vec![0.; 3];
                    q[a.0] = a.1 + signs[0] * p[0];
                    q[b.0] = b.1 + signs[1] * p[1];
                    q[free] = axial[j];
                    q
                })
                .collect()
        })
        .collect();
    let lo = s.knots_v[s.degree_v];
    let hi = s.knots_v[s.control_points[0].len()];
    let out = Surface {
        degree_u: c.degree,
        degree_v: 1,
        knots_u: c.knots,
        knots_v: vec![lo, lo, hi, hi],
        control_points: points,
        weights: c.weights.iter().map(|w| vec![*w; 2]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    out.validate()?;
    Ok(out)
}
pub(super) fn refit(s: &Surface, supports: [&Surface; 2], fraction: f64) -> Result<Surface> {
    if !fraction.is_finite() || !(0.1..=0.3).contains(&fraction) {
        return Err(fail("Handle fraction must be within 0.1..0.3"));
    }
    for p in supports {
        p.validate()?;
    }
    s.validate()?;
    if supports.iter().any(|p| p.control_points[0][0].len() != 3)
        || s.control_points[0][0].len() != 3
    {
        return Err(fail("Planar G2 profile requires 3D supports and skin"));
    }
    let planes = supports.map(|p| {
        (0..3).find_map(|k| {
            let x = p.control_points[0][0][k];
            p.control_points
                .iter()
                .flatten()
                .all(|q| q.len() == 3 && q[k] == x)
                .then_some((k, x))
        })
    });
    let [Some(a), Some(b)] = planes else {
        return Err(fail("Both supports must be literal coordinate planes"));
    };
    if a.0 == b.0
        || s.degree_u != 5
        || s.control_points.len() != 6
        || s.periodic_u
        || s.periodic_v
        || s.weights.iter().flatten().any(|w| *w <= 0.)
    {
        return Err(fail(
            "Need a nonperiodic six-row quintic skin between distinct coordinate planes",
        ));
    }
    let mut out = s.clone();
    for j in 0..s.control_points[0].len() {
        let p0 = &s.control_points[0][j];
        let p5 = &s.control_points[5][j];
        if p0[a.0] != a.1 || p5[b.0] != b.1 {
            return Err(fail(
                "Endpoint poles do not belong to the literal support planes",
            ));
        }
        let chord = (0..3).map(|k| (p5[k] - p0[k]).powi(2)).sum::<f64>().sqrt();
        for (end, near, second) in [(0, 1, 2), (5, 4, 3)] {
            let tangent = std::array::from_fn::<_, 3, _>(|k| {
                s.control_points[near][j][k] - s.control_points[end][j][k]
            });
            let len = tangent.iter().map(|x| x * x).sum::<f64>().sqrt();
            if !len.is_finite() || len <= 0. || !chord.is_finite() || chord <= 0. {
                return Err(fail("Degenerate planar profile tangent"));
            }
            for k in 0..3 {
                let delta = tangent[k] * (fraction * chord / len);
                out.control_points[near][j][k] = s.control_points[end][j][k] + delta;
                out.control_points[second][j][k] = s.control_points[end][j][k] + 2. * delta;
            }
            let (axis, value) = if end == 0 { a } else { b };
            if tangent[axis] != 0. {
                return Err(fail("Original endpoint tangent leaves its support plane"));
            }
            out.control_points[near][j][axis] = value;
            out.control_points[second][j][axis] = value;
        }
    }
    // Literal perpendicular coordinate planes and a constant-radius factory
    // define an extrusion. Author that exact proposal instead of retaining
    // natural-loft station noise. Fresh contact/radius/G2 checks still decide.
    let free = 3 - a.0 - b.0;
    let end = s.control_points[0].len() - 1;
    let axial = [s.control_points[0][0][free], s.control_points[0][end][free]];
    if axial[0] == axial[1] {
        return Err(fail("Zero planar extrusion span"));
    }
    let low = s.knots_v[s.degree_v];
    let high = s.knots_v[s.control_points[0].len()];
    out.control_points = (0..6)
        .map(|i| {
            (0..2)
                .map(|j| {
                    let mut p = out.control_points[i][0].clone();
                    p[free] = axial[j];
                    p
                })
                .collect()
        })
        .collect();
    out.weights = vec![vec![1.; 2]; 6];
    out.degree_v = 1;
    out.knots_v = vec![low, low, high, high];
    out.validate()?;
    Ok(out)
}
