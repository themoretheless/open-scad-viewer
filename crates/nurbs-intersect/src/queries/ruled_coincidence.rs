use super::*;
/// Homogeneous control net of a ruled patch, indexed [u][v] with two V rows.
pub(crate) type HomogeneousGrid = Vec<Vec<[f64; 4]>>;
pub(crate) fn homogeneous_grid(surface: &Surface) -> HomogeneousGrid {
    surface
        .control_points
        .iter()
        .zip(&surface.weights)
        .map(|(points, weights)| {
            points
                .iter()
                .zip(weights)
                .map(|(p, w)| [p[0] * w, p[1] * w, p[2] * w, *w])
                .collect()
        })
        .collect()
}
/// de Casteljau split of both V rows at the U midpoint of a Bezier patch.
pub(crate) fn split_grid_u(grid: &HomogeneousGrid) -> (HomogeneousGrid, HomogeneousGrid) {
    let columns: Vec<Vec<[f64; 4]>> = (0..grid[0].len())
        .map(|j| grid.iter().map(|row| row[j]).collect())
        .collect();
    let mut left: HomogeneousGrid = vec![Vec::new(); grid.len()];
    let mut right: HomogeneousGrid = vec![Vec::new(); grid.len()];
    for (j, column) in columns.iter().enumerate() {
        let (l, r) = split_homogeneous(column);
        for (i, row) in left.iter_mut().enumerate() {
            row.push(l[i]);
        }
        for (i, row) in right.iter_mut().enumerate() {
            row.push(r[i]);
        }
        let _ = j;
    }
    (left, right)
}
pub(crate) fn grid_ranges(grid: &HomogeneousGrid) -> [[f64; 2]; 3] {
    hull_ranges(&grid.iter().flatten().copied().collect::<Vec<_>>())
}
pub(crate) fn grids_excluded(a: &[[f64; 4]], b: &HomogeneousGrid) -> bool {
    let ra = hull_ranges(a);
    let rb = grid_ranges(b);
    (0..3).any(|axis| ra[axis][1] < rb[axis][0] || rb[axis][1] < ra[axis][0])
}
/// Seam state of the U direction of a canonical ruled-in-V surface. Closure
/// is geometric: the two seam ruling curves (the first and last homogeneous
/// control rows) must coincide as rational curves, i.e. be exactly
/// proportional with a positive ratio. Merging is never by tolerance: only a
/// bitwise-exact proportionality closes the seam; a near-proportional seam
/// (relative mismatch within 1e-6) stays duplicate and is reported as
/// explicit near_coincidence bands at both domain ends instead of a guess.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum RuledSeam {
    Open,
    Closed,
    Uncertain,
}
pub(crate) fn ruled_seam_state(surface: &Surface) -> RuledSeam {
    let grid = homogeneous_grid(surface);
    let (first, last) = (&grid[0], &grid[grid.len() - 1]);
    let anchor = (0..4)
        .max_by(|&a, &b| first[0][a].abs().total_cmp(&first[0][b].abs()))
        .unwrap();
    if !first[0][anchor].is_finite() || first[0][anchor].abs() <= 0. {
        return RuledSeam::Open;
    }
    let ratio = last[0][anchor] / first[0][anchor];
    if !ratio.is_finite() || ratio <= 0. {
        return RuledSeam::Open;
    }
    let scale = first
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0., f64::max)
        .max(1.);
    let mismatch = |row: &[[f64; 4]], other: &[[f64; 4]]| {
        row.iter().zip(other).fold(0_f64, |m, (a, b)| {
            m.max((0..4).fold(0_f64, |m2, k| m2.max((b[k] - ratio * a[k]).abs())))
        })
    };
    let delta = mismatch(first, last);
    if delta == 0. {
        RuledSeam::Closed
    } else if delta <= 1e-6 * scale {
        RuledSeam::Uncertain
    } else {
        RuledSeam::Open
    }
}

/// Curve-on-surface admission for one (curve span, surface U-span) pair in the
/// canonical ruled-in-V orientation. Two algebraically exact families are
/// admitted: the piece is collinear with a ruling (constant U, Möbius-lifted
/// V endpoints), or its homogeneous polygon is a proportional blend of the two
/// boundary polygons (constant V). Anything ambiguous is an explicit
/// unresolved region; non-coincident pairs return false for point isolation.
#[allow(clippy::too_many_arguments)]
pub(crate) fn ruled_coincidence(
    piece: &Curve,
    patch: &Surface,
    ta: [f64; 2],
    ua: [f64; 2],
    vd: [f64; 2],
    seam_u: Option<[f64; 2]>,
    options: Options,
    report: &mut Report<CurveRuledSurfaceComponent>,
) -> Result<bool> {
    let box6 = || vec![ta[0], ta[1], ua[0], ua[1], vd[0], vd[1]];
    let b0 = patch.iso(nurbs_core::surface::Axis::V, vd[0])?;
    let b1 = patch.iso(nurbs_core::surface::Axis::V, vd[1])?;
    let a0 = point3(piece.control_points.first().unwrap());
    let a1 = point3(piece.control_points.last().unwrap());
    let chord = sub(a1, a0);
    let length = chord[0].hypot(chord[1]).hypot(chord[2]);
    let mut boundary_on_line = false;
    if length > 0. {
        let dir = chord.map(|x| x / length);
        let off_line = |p: [f64; 3]| {
            let v = cross(dir, sub(p, a0));
            v[0].hypot(v[1]).hypot(v[2])
        };
        let control_residual = piece
            .control_points
            .iter()
            .map(|c| off_line(point3(c)))
            .fold(0., f64::max);
        if control_residual <= options.distance_tolerance {
            // A straight piece lies on the ruled surface only as a ruling:
            // find U where both boundary points sit on the support line.
            // Candidates are certified plane roots of B0 near the line;
            // halo bands touching a certified root are absorbed, bands that
            // provably stay off the second plane are dropped, and anything
            // else keeps the coincidence ambiguous instead of guessed.
            let axis = (0..3)
                .min_by(|a, b| dir[*a].abs().total_cmp(&dir[*b].abs()))
                .unwrap();
            let mut basis = [0.; 3];
            basis[axis] = 1.;
            let n = cross(dir, basis);
            let plane1 = Plane {
                normal: n,
                offset: dot(n, a0),
            }
            .normalized()?;
            let n = cross(dir, plane1.normal);
            let plane2 = Plane {
                normal: n,
                offset: dot(n, a0),
            }
            .normalized()?;
            if report.boxes_visited >= options.max_boxes {
                report.unresolved(box6(), UnresolvedReason::BudgetExceeded);
                return Ok(true);
            }
            let roots = curve_plane(
                &b0,
                plane1,
                Options {
                    max_boxes: options.max_boxes - report.boxes_visited,
                    ..options
                },
            )?;
            report.boxes_visited += roots.boxes_visited;
            report.bernstein_excluded += roots.bernstein_excluded;
            let mut line_points = Vec::new();
            let mut certified: Vec<[f64; 2]> = Vec::new();
            let mut bands: Vec<[f64; 2]> = Vec::new();
            let mut joint_bands: Vec<[f64; 2]> = Vec::new();
            for component in roots.components {
                match component {
                    CurvePlaneComponent::Point(point) => {
                        certified.push(point.parameter_interval);
                        if plane2.distance(point.point).abs() <= options.distance_tolerance {
                            line_points.push(point.parameter);
                        }
                    }
                    // B0 lies in the first plane over this interval: isolate
                    // its second-plane crossings through one more shared-budget
                    // query instead of assuming a line coincidence.
                    CurvePlaneComponent::Overlap {
                        parameter_interval, ..
                    } => {
                        if report.boxes_visited >= options.max_boxes {
                            report.unresolved(box6(), UnresolvedReason::BudgetExceeded);
                            return Ok(true);
                        }
                        let piece0 = b0.trim(parameter_interval[0], parameter_interval[1])?;
                        let second = curve_plane(
                            &piece0,
                            plane2,
                            Options {
                                max_boxes: options.max_boxes - report.boxes_visited,
                                ..options
                            },
                        )?;
                        report.boxes_visited += second.boxes_visited;
                        report.bernstein_excluded += second.bernstein_excluded;
                        for nested in second.components {
                            match nested {
                                CurvePlaneComponent::Point(point) => {
                                    certified.push(point.parameter_interval);
                                    line_points.push(point.parameter);
                                }
                                CurvePlaneComponent::Overlap { .. } => boundary_on_line = true,
                            }
                        }
                        for pending in second.unresolved {
                            joint_bands.push([pending.parameter_box[0], pending.parameter_box[1]]);
                        }
                    }
                }
            }
            for pending in roots.unresolved {
                bands.push([pending.parameter_box[0], pending.parameter_box[1]]);
            }
            let mut ambiguous = false;
            for band in bands {
                // A band provably staying off the second plane cannot hide a
                // ruling candidate; outward-rounded Bernstein bounds decide.
                let clear = if band[0] == band[1] {
                    let p = point3(&b0.evaluate(band[0])?.point);
                    plane2.distance(p).abs() > options.distance_tolerance
                } else {
                    let piece = b0.trim(band[0], band[1])?;
                    let values = coefficients(&piece, plane2);
                    values
                        .iter()
                        .all(|c| c.bound.lo > options.distance_tolerance)
                        || values
                            .iter()
                            .all(|c| c.bound.hi < -options.distance_tolerance)
                };
                if clear {
                    continue;
                }
                joint_bands.push(band);
            }
            // Remaining bands touching a certified root are its rounding halo;
            // isolated ones keep the coincidence ambiguous, never guessed.
            for band in joint_bands {
                if certified
                    .iter()
                    .any(|iv| iv[0] <= band[1] && band[0] <= iv[1])
                {
                    continue;
                }
                ambiguous = true;
            }
            // A line point of B0 is a ruling candidate only if the other
            // boundary point at the same U also sits on the support line.
            let mut candidates = Vec::new();
            for u in line_points {
                if off_line(point3(&b1.evaluate(u)?.point)) <= options.distance_tolerance {
                    candidates.push(u);
                }
            }
            candidates.dedup_by(|a, b| *a == *b);
            // An exactly closed seam makes the u_min and u_max rulings the
            // same curve: fold the u_max candidate into the canonical u_min
            // representative (exact parameter correspondence only).
            if let Some([s0, s1]) = seam_u
                && candidates.contains(&s0)
                && candidates.contains(&s1)
            {
                candidates.retain(|&u| u != s1);
            }
            if candidates.len() > 1 {
                report.unresolved(box6(), UnresolvedReason::NearCoincidence);
                return Ok(true);
            }
            if candidates.is_empty() && ambiguous {
                report.unresolved(box6(), UnresolvedReason::NearCoincidence);
                return Ok(true);
            }
            if let [u] = candidates[..] {
                if ambiguous {
                    report.unresolved(box6(), UnresolvedReason::NearCoincidence);
                    return Ok(true);
                }
                let r0 = point3(&b0.evaluate(u)?.point);
                let r1 = point3(&b1.evaluate(u)?.point);
                let ruling = sub(r1, r0);
                let dd = dot(ruling, ruling);
                let w0 = curve_weight_at(&b0, u)?;
                let w1 = curve_weight_at(&b1, u)?;
                if !dd.is_finite()
                    || dd <= 0.
                    || !w0.is_finite()
                    || w0 <= 0.
                    || !w1.is_finite()
                    || w1 <= 0.
                {
                    report.unresolved(box6(), UnresolvedReason::NearCoincidence);
                    return Ok(true);
                }
                // Möbius lift of a Cartesian line fraction s to ruling V:
                // s = λw1/((1-λ)w0+λw1) inverts to λ = s·w0/(w1-s(w1-w0)).
                let lift = |p: [f64; 3]| {
                    let s = dot(sub(p, r0), ruling) / dd;
                    let denominator = w1 - s * (w1 - w0);
                    if !denominator.is_finite() || denominator <= 0. {
                        return None;
                    }
                    let lambda = s * w0 / denominator;
                    (lambda.is_finite()).then(|| vd[0] + lambda * (vd[1] - vd[0]))
                };
                let mut lifts = Vec::new();
                for control in &piece.control_points {
                    let Some(v) = lift(point3(control)) else {
                        report.unresolved(box6(), UnresolvedReason::NearCoincidence);
                        return Ok(true);
                    };
                    lifts.push(v);
                }
                // The piece image lies inside its control polygon, so the whole
                // span stays on the surface exactly when every lift does.
                let vtol = options.parameter_tolerance;
                if lifts.iter().any(|v| *v < vd[0] - vtol || *v > vd[1] + vtol) {
                    report.unresolved(box6(), UnresolvedReason::CoincidentTrim);
                    return Ok(true);
                }
                let clamp = |v: f64| v.max(vd[0]).min(vd[1]);
                let uv_start = [u, clamp(lifts[0])];
                let uv_end = [u, clamp(*lifts.last().unwrap())];
                let mut max_residual = control_residual;
                for (t, uv) in [(ta[0], uv_start), (ta[1], uv_end)] {
                    let pc = point3(&piece.evaluate(t)?.point);
                    let ps = patch.evaluate(uv[0], uv[1])?.point;
                    let residual = distance(pc, ps);
                    if residual > options.distance_tolerance {
                        report.unresolved(box6(), UnresolvedReason::NearCoincidence);
                        return Ok(true);
                    }
                    max_residual = max_residual.max(residual);
                }
                // The Möbius t->v correspondence along the ruling, sampled at
                // the interval fractions 0, 1/2, 1: three samples determine
                // the cross-ratio map exactly.
                let mut samples = [[0.; 3]; 3];
                let mut sampled = true;
                for (i, s) in samples.iter_mut().enumerate() {
                    let t = ta[0] + (ta[1] - ta[0]) * (i as f64) * 0.5;
                    let p = point3(&piece.evaluate(t)?.point);
                    match lift(p) {
                        Some(v) => *s = [t, u, clamp(v)],
                        None => {
                            sampled = false;
                            break;
                        }
                    }
                }
                let correspondence = sampled.then_some(OverlapCorrespondence {
                    kind: OverlapCorrespondenceKind::MobiusV,
                    samples,
                });
                push_cs_overlap(
                    ta,
                    uv_start,
                    uv_end,
                    max_residual,
                    false,
                    correspondence,
                    report,
                );
                return Ok(true);
            }
        }
    }
    // Iso-V coincidence: the elevated homogeneous polygon must be a positive
    // proportional blend (1-λ)H0 + λH1 of the two boundary polygons.
    let degree = piece.degree.max(patch.degree_u);
    if degree > 25 {
        return Ok(false);
    }
    let ec = homogeneous4(&piece.elevate(degree)?);
    let e0 = homogeneous4(&b0.elevate(degree)?);
    let e1 = homogeneous4(&b1.elevate(degree)?);
    let weight_scale = ec.iter().map(|h| h[3].abs()).fold(0., f64::max);
    for reversed in [false, true] {
        let at = |row: &[[f64; 4]], i: usize| row[if reversed { degree - i } else { i }];
        let i0 = ec
            .iter()
            .enumerate()
            .max_by(|a, b| a.1[3].abs().total_cmp(&b.1[3].abs()))
            .map(|(i, _)| i)
            .unwrap();
        let (a, b, c) = (at(&e0, i0), at(&e1, i0), ec[i0]);
        let d = std::array::from_fn::<_, 4, _>(|k| b[k] - a[k]);
        // α·H0 + β·(H1-H0) = Hc is linear in (α, β); solve from the dominant
        // polygon row by 2x2 normal equations, then verify every row.
        let (aa, ad, dd) = (dot4(a, a), dot4(a, d), dot4(d, d));
        let determinant = aa * dd - ad * ad;
        if !determinant.is_finite() || determinant <= 0. {
            continue;
        }
        let alpha = (dd * dot4(a, c) - ad * dot4(d, c)) / determinant;
        let beta = (aa * dot4(d, c) - ad * dot4(a, c)) / determinant;
        if !alpha.is_finite() || alpha <= 0. || !beta.is_finite() {
            continue;
        }
        let lambda = beta / alpha;
        if !(-1e-9..=1. + 1e-9).contains(&lambda) {
            continue;
        }
        let mut weight_residual = 0_f64;
        let mut control_residual = 0_f64;
        let mut valid = true;
        for (i, c) in ec.iter().enumerate() {
            let (a, b) = (at(&e0, i), at(&e1, i));
            let h = std::array::from_fn::<_, 4, _>(|k| alpha * a[k] + beta * (b[k] - a[k]));
            if !h[3].is_finite() || h[3] <= 0. {
                valid = false;
                break;
            }
            weight_residual = weight_residual.max((h[3] - c[3]).abs() / weight_scale);
            let ph = std::array::from_fn(|k| h[k] / h[3]);
            let pc = std::array::from_fn(|k| c[k] / c[3]);
            control_residual = control_residual.max(distance(ph, pc));
        }
        if !valid || weight_residual > 1e-9 || control_residual > options.distance_tolerance {
            continue;
        }
        let v = vd[0] + lambda.clamp(0., 1.) * (vd[1] - vd[0]);
        let (uv_start, uv_end) = if reversed {
            ([ua[1], v], [ua[0], v])
        } else {
            ([ua[0], v], [ua[1], v])
        };
        let mut endpoint_residual = 0_f64;
        for (t, uv) in [(ta[0], uv_start), (ta[1], uv_end)] {
            let pc = point3(&piece.evaluate(t)?.point);
            let ps = patch.evaluate(uv[0], uv[1])?.point;
            let residual = distance(pc, ps);
            if residual > options.distance_tolerance {
                report.unresolved(box6(), UnresolvedReason::NearCoincidence);
                return Ok(true);
            }
            endpoint_residual = endpoint_residual.max(residual);
        }
        push_cs_overlap(
            ta,
            uv_start,
            uv_end,
            control_residual.max(endpoint_residual),
            false,
            // Affine t->u correspondence at constant v, sampled at the
            // interval fractions 0, 1/2, 1.
            Some(OverlapCorrespondence {
                kind: OverlapCorrespondenceKind::AffineU,
                samples: std::array::from_fn(|i| {
                    let f = (i as f64) * 0.5;
                    [
                        ta[0] + (ta[1] - ta[0]) * f,
                        uv_start[0] + (uv_end[0] - uv_start[0]) * f,
                        v,
                    ]
                }),
            }),
            report,
        );
        return Ok(true);
    }
    if boundary_on_line {
        report.unresolved(box6(), UnresolvedReason::CoincidentTrim);
        return Ok(true);
    }
    Ok(false)
}
