//! Original planar 2-jet collars, with owned collapsed endpoint coverage.
use super::*;
fn axis_sign(j: &jets::Jets, axis: usize) -> Result<Option<i8>> {
    let (a, b) = ((axis + 1) % 3, (axis + 2) % 3);
    let determinant = j[1][0][a]
        .mul(j[0][1][b])?
        .sub(j[1][0][b].mul(j[0][1][a])?)?;
    Ok(if determinant.lo > 0. {
        Some(1)
    } else if determinant.hi < 0. {
        Some(-1)
    } else {
        None
    })
}
/// Literal constant heights in the actual first three rows prove zero second
/// fundamental form. Original Jacobians must retain one sign over all seam
/// interiors; owned native pole neighborhoods cover collapsed endpoints.
pub fn inspect_planar_boundary(
    curve: &Curve,
    pc: [&Curve; 2],
    s: [&Surface; 2],
    height: usize,
    poles: [Option<&FlatPoleCertificate<'_>>; 2],
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
) -> Result<ExtrusionBoundaryReport> {
    check(
        height < 3
            && tolerances.iter().all(|x| x.is_finite() && *x >= 0.)
            && tolerances[1] <= 2.
            && max_cells <= 100000
            && max_exact <= 1000000,
        "Invalid original planar seam limits",
    )?;
    for surface in s {
        surface.validate()?;
        check(
            !surface.periodic_u && !surface.periodic_v,
            "Planar seam needs original nonperiodic charts",
        )?;
    }
    for p in pc {
        p.validate()?;
        check(
            (1..=16).contains(&p.degree) && p.control_points[0].len() == 2 && !p.periodic,
            "Planar seam needs bounded original pcurves",
        )?;
    }
    check(
        pc[1].degree == 1 && pc[1].control_points.len() == 2,
        "Candidate planar boundary needs an affine U clock",
    )?;
    let a = s[0];
    let b = s[1];
    let (p, q, n, m) = (
        b.degree_u,
        b.degree_v,
        b.control_points.len(),
        b.control_points[0].len(),
    );
    check(
        (2..=16).contains(&p)
            && (2..=16).contains(&q)
            && m == q + 1
            && b.knots_v[..=q].iter().all(|&v| v == b.knots_v[q])
            && b.knots_v[m..].iter().all(|&v| v == b.knots_v[m])
            && pc[1].weights[0] == pc[1].weights[1],
        "Planar candidate needs original Bernstein V and affine U clock",
    )?;
    check(
        b.knots_u[..=p].iter().all(|&u| u == b.knots_u[p])
            && b.knots_u[n..].iter().all(|&u| u == b.knots_u[n]),
        "Planar candidate U endpoints must be clamped",
    )?;
    let source_height = a.control_points[0][0][height];
    check(
        a.control_points
            .iter()
            .flatten()
            .all(|p| p[height] == source_height),
        "Original source is not literally planar",
    )?;
    check(
        a.control_points.len() == a.degree_u + 1
            && a.control_points[0].len() == a.degree_v + 1
            && a.knots_u[..=a.degree_u]
                .iter()
                .all(|&u| u == a.knots_u[a.degree_u])
            && a.knots_u[a.degree_u + 1..]
                .iter()
                .all(|&u| u == a.knots_u[a.degree_u + 1])
            && a.knots_v[..=a.degree_v]
                .iter()
                .all(|&v| v == a.knots_v[a.degree_v])
            && a.knots_v[a.degree_v + 1..]
                .iter()
                .all(|&v| v == a.knots_v[a.degree_v + 1]),
        "Planar source needs one original Bernstein chart",
    )?;
    let positions = verify_positions(
        curve,
        pc,
        s,
        [false; 2],
        tolerances[0],
        max_cells / 4,
        max_exact,
    )?;
    let mut work = positions.cells;
    let mut sign_a = None;
    let query = std::array::from_fn(|k| {
        [
            pc[0]
                .control_points
                .iter()
                .map(|p| p[k])
                .fold(f64::INFINITY, f64::min),
            pc[0]
                .control_points
                .iter()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max),
        ]
    });
    if work < max_cells {
        work += 1;
        let j =
            jets::calculate_partial_stable_up_to(a, [a.degree_u, a.degree_v], query, [None; 2], 1)?;
        sign_a = axis_sign(&j, height)?;
    }
    let v = pc[1].control_points[0][1];
    check(
        pc[1].control_points[1][1] == v && (v == b.knots_v[q] || v == b.knots_v[m]),
        "Planar pcurve must own an actual V boundary",
    )?;
    let end = v == b.knots_v[m];
    let row = if end { q } else { 0 };
    let u0 = pc[1].control_points[0][0];
    let u1 = pc[1].control_points[1][0];
    check(
        u0 != u1 && u0.min(u1) >= b.knots_u[p] && u0.max(u1) <= b.knots_u[n],
        "Planar pcurve leaves original U domain",
    )?;
    let mut cells = Vec::new();
    let mut candidate_sign = None;
    for span in p..n {
        let original = [b.knots_u[span], b.knots_u[span + 1]];
        if original[0] >= original[1] || original[1] <= u0.min(u1) || original[0] >= u0.max(u1) {
            continue;
        }
        check(
            original[0] >= u0.min(u1)
                && original[1] <= u0.max(u1)
                && (original[0] == b.knots_u[p]
                    || b.knots_u.iter().filter(|&&u| u == original[0]).count() == p)
                && (original[1] == b.knots_u[n]
                    || b.knots_u.iter().filter(|&&u| u == original[1]).count() == p),
            "Planar seam needs whole literal original Bernstein spans",
        )?;
        let height_value = b.control_points[span - p][row][height];
        for i in span - p..=span {
            for offset in 0..3 {
                let j = if end { row - offset } else { offset };
                check(
                    b.control_points[i][j][height] == height_value,
                    "Candidate collar has a nonplanar original second jet",
                )?;
            }
        }
        let mut range = original;
        for e in [false, true] {
            let global = if e { span == n - 1 } else { span == p };
            if !global {
                continue;
            }
            let index = if e { n - 1 } else { 0 };
            let collapsed = b.control_points[index]
                .iter()
                .all(|x| x == &b.control_points[index][0]);
            if !collapsed {
                continue;
            }
            let cert = poles[usize::from(e)]
                .ok_or_else(|| crate::input("Planar collar endpoint needs owned pole quotient"))?;
            check(
                std::ptr::eq(cert.source(), b)
                    && cert.is_end() == e
                    && cert.pole()[height] == height_value,
                "Planar collar pole source/plane differs",
            )?;
            let normal = cert.normal();
            check(
                normal[height].abs() == 1. && (0..3).all(|k| k == height || normal[k] == 0.),
                "Pole normal differs from actual planar collar",
            )?;
            let sign = normal[height] as i8;
            check(
                candidate_sign.is_none_or(|s| s == sign),
                "Planar collar orientation differs at owned poles",
            )?;
            candidate_sign = Some(sign);
            let t = cert.band_normalized();
            let width = I::point(original[1]).sub(I::point(original[0]))?;
            if e {
                range[1] = I::point(original[1]).sub(width.mul(I::point(t))?)?.hi;
            } else {
                range[0] = I::point(original[0]).add(width.mul(I::point(t))?)?.lo;
            }
            // Round toward the pole: numerical chart coverage overlaps the
            // exact normalized certified band instead of leaving a gap.
            range[0] = range[0].max(original[0]);
            range[1] = range[1].min(original[1]);
        }
        if range[0] < range[1] {
            cells.push((span, range, None));
        }
    }
    check(!cells.is_empty(), "No original planar boundary spans")?;
    let mut pending = cells;
    let mut unresolved = 0;
    while let Some((span, u, _)) = pending.pop() {
        if work == max_cells {
            unresolved += 1 + pending.len();
            break;
        }
        work += 1;
        let j = jets::calculate_partial_stable_up_to(b, [span, q], [u, [v, v]], [None; 2], 1)?;
        if let Some(sign) = axis_sign(&j, height)? {
            check(
                candidate_sign.is_none_or(|s| s == sign),
                "Planar original Jacobian changes orientation",
            )?;
            candidate_sign = Some(sign);
            continue;
        }
        let mid = u[0] * 0.5 + u[1] * 0.5;
        if !(u[0] < mid && mid < u[1]) {
            unresolved += 1;
            continue;
        }
        pending.push((span, [u[0], mid], None::<()>));
        pending.push((span, [mid, u[1]], None));
    }
    let proven =
        positions.proven && sign_a.is_some() && candidate_sign.is_some() && unresolved == 0;
    Ok(ExtrusionBoundaryReport {
        positions,
        decision: if proven {
            Decision::WithinTolerance
        } else {
            Decision::Unresolved
        },
        orientation: if sign_a == candidate_sign {
            Orientation::Same
        } else {
            Orientation::Opposite
        },
        normal: [0., if proven { 0. } else { f64::INFINITY }],
        tensor: [0., if proven { 0. } else { f64::INFINITY }],
        regularity_proven: sign_a.is_some() && candidate_sign.is_some() && unresolved == 0,
        cells: work,
        pending: unresolved + usize::from(sign_a.is_none()),
    })
}
