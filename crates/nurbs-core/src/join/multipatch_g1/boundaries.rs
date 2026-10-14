use super::*;

/// Boundary curve of a surface (free axis of the boundary, as stored).
pub(super) fn boundary_curve(s: &Surface, b: Boundary) -> Curve {
    match b {
        Boundary::UMin => Curve {
            degree: s.degree_v,
            knots: s.knots_v.clone(),
            control_points: s.control_points[0].clone(),
            weights: s.weights[0].clone(),
            periodic: s.periodic_v,
        },
        Boundary::UMax => Curve {
            degree: s.degree_v,
            knots: s.knots_v.clone(),
            control_points: s.control_points.last().unwrap().clone(),
            weights: s.weights.last().unwrap().clone(),
            periodic: s.periodic_v,
        },
        Boundary::VMin => Curve {
            degree: s.degree_u,
            knots: s.knots_u.clone(),
            control_points: s.control_points.iter().map(|r| r[0].clone()).collect(),
            weights: s.weights.iter().map(|r| r[0]).collect(),
            periodic: s.periodic_u,
        },
        Boundary::VMax => Curve {
            degree: s.degree_u,
            knots: s.knots_u.clone(),
            control_points: s
                .control_points
                .iter()
                .map(|r| r.last().unwrap().clone())
                .collect(),
            weights: s.weights.iter().map(|r| *r.last().unwrap()).collect(),
            periodic: s.periodic_u,
        },
    }
}

/// Whether the vertex sits at the start (false) or end (true) of the
/// boundary's stored parameter.
pub(super) fn vertex_at_end(corner: Corner, b: Boundary) -> bool {
    let (su, sv) = corner.signs();
    match b {
        Boundary::UMin | Boundary::UMax => sv < 0.,
        Boundary::VMin | Boundary::VMax => su < 0.,
    }
}

pub(super) fn incoming(patch: &VertexPatch) -> Boundary {
    let [b0, b1] = patch.corner.boundaries();
    if same_boundary(patch.outgoing, b0) {
        b1
    } else {
        b0
    }
}

/// Free axis of a boundary: true when the boundary parameter runs along U.
pub(super) fn free_axis_u(b: Boundary) -> bool {
    matches!(b, Boundary::VMin | Boundary::VMax)
}

/// Clamped-end check for a boundary (multiplicity degree + 1 at both ends of
/// the free axis is not required — only at the boundary itself the cross axis
/// must be clamped). Returns true when the CROSS axis is clamped at the end
/// that defines `b`.
pub(super) fn clamped_at(s: &Surface, b: Boundary) -> bool {
    let (knots, degree, count) = match b {
        Boundary::UMin | Boundary::UMax => (&s.knots_u, s.degree_u, s.control_points.len()),
        Boundary::VMin | Boundary::VMax => (&s.knots_v, s.degree_v, s.control_points[0].len()),
    };
    let start = knots[..=degree].iter().all(|&k| k == knots[0]);
    let end = knots[count..].iter().all(|&k| k == knots[count]);
    match b {
        Boundary::UMin | Boundary::VMin => start,
        Boundary::UMax | Boundary::VMax => end,
    }
}

/// Affine-rescale both knot vectors of `axis` of every surface so the active
/// domain becomes [0, 1].
pub(super) fn normalized_axis(s: &Surface, axis_u: bool) -> Result<Surface> {
    let mut out = s.clone();
    let (knots, degree, count) = if axis_u {
        (&mut out.knots_u, out.degree_u, out.control_points.len())
    } else {
        (
            &mut out.knots_v,
            out.degree_v,
            out.control_points[0].len(),
        )
    };
    let a = knots[degree];
    let b = knots[count];
    check(b > a, "Vertex fan patches need a nonzero axis domain")?;
    if a != 0. || b != 1. {
        for k in knots.iter_mut() {
            *k = (*k - a) / (b - a);
        }
    }
    out.validate()?;
    Ok(out)
}

/// Multiplicity-limited union of interior knots of two seam curves after
/// elevation to `degree`; returns the insertion list for `curve`.
pub(super) fn insertions(curve: &Curve, other: &Curve, degree: usize) -> Result<Vec<f64>> {
    let mut target: Vec<(f64, usize)> = Vec::new();
    for c in [curve, other] {
        let [a, b] = c.domain();
        let mut values: Vec<f64> = c
            .knots
            .iter()
            .copied()
            .filter(|k| *k > a && *k < b)
            .collect();
        values.dedup();
        for v in values {
            let mult = c.knots.iter().filter(|&&k| k == v).count().min(degree);
            match target.iter_mut().find(|(x, _)| *x == v) {
                Some((_, m)) => *m = (*m).max(mult),
                None => target.push((v, mult)),
            }
        }
    }
    target.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = Vec::new();
    for (v, m) in target {
        let have = curve.knots.iter().filter(|&&k| k == v).count().min(degree);
        out.extend(std::iter::repeat_n(v, m.saturating_sub(have)));
    }
    Ok(out)
}

/// Harmonize degree and knots along every seam of the fan (exact elevation
/// and union refinement applied to whole surface axes).
pub(super) fn harmonize(fan: &[VertexPatch]) -> Result<Vec<Surface>> {
    let n = fan.len();
    // Normalize every axis that carries a fan boundary to the [0, 1] domain.
    let mut surfaces: Vec<Surface> = fan
        .iter()
        .map(|p| {
            let mut s = p.surface.clone();
            if free_axis_u(p.outgoing) || free_axis_u(incoming(p)) {
                s = normalized_axis(&s, true)?;
            }
            if !free_axis_u(p.outgoing) || !free_axis_u(incoming(p)) {
                s = normalized_axis(&s, false)?;
            }
            Ok(s)
        })
        .collect::<Result<_>>()?;
    // Pairwise per seam: both sides elevate to the common degree, then refine
    // to the multiplicity-limited union of interior knots.
    for seam in 0..n {
        let next = (seam + 1) % n;
        let b_a = fan[seam].outgoing;
        let b_b = incoming(&fan[next]);
        let curve_a = boundary_curve(&surfaces[seam], b_a);
        let curve_b = boundary_curve(&surfaces[next], b_b);
        let degree = curve_a.degree.max(curve_b.degree);
        let elevated_a = curve_a.elevate(degree)?;
        let elevated_b = curve_b.elevate(degree)?;
        let ins_a = insertions(&elevated_a, &elevated_b, degree)?;
        let ins_b = insertions(&elevated_b, &elevated_a, degree)?;
        let axis_a = if free_axis_u(b_a) {
            crate::surface::Axis::U
        } else {
            crate::surface::Axis::V
        };
        let axis_b = if free_axis_u(b_b) {
            crate::surface::Axis::U
        } else {
            crate::surface::Axis::V
        };
        let (sa, sb) = if seam < next {
            let (before, after) = surfaces.split_at_mut(next);
            (&mut before[seam], &mut after[0])
        } else {
            let (before, after) = surfaces.split_at_mut(seam);
            (&mut after[0], &mut before[next])
        };
        *sa = sa.edit_axis(axis_a, |c| c.elevate(degree)?.refine(&ins_a))?;
        *sb = sb.edit_axis(axis_b, |c| c.elevate(degree)?.refine(&ins_b))?;
    }
    Ok(surfaces)
}
