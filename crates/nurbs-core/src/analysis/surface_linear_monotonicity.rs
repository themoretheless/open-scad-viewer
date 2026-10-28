//! Whole-chart injectivity under one fixed exact integer linear projection.
//! Sampling may propose a projection elsewhere, but never certifies this result.
use crate::{
    check, distance_bounds::Interval as I, surface::Surface, surface_measure::jets, Result,
};
mod bernstein_minors;
mod bernstein_symmetric;
mod polynomial_projection;

#[derive(Clone, Debug)]
pub struct ProfileParameterization {
    pub weights: [f64; 2],
    pub derivative_lower: f64,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub certified: bool,
    pub cells: usize,
    pub projection: [[i8; 3]; 2],
    pub row_weights: [f64; 2],
    pub profile_parameterization: Option<ProfileParameterization>,
    pub reason: Option<&'static str>,
}
pub fn inspect(s: &Surface, projection: [[i8; 3]; 2], max_cells: usize) -> Result<Report> {
    inspect_with_depth(s, projection, max_cells, 12)
}
/// Linear projection commutes with the rational basis when every projected
/// Euclidean pole is exactly representable. Never substitute rounded poles.
fn exact_projected_surface(s: &Surface, projection: [[i8; 3]; 2]) -> Option<Surface> {
    use crate::exact_curve_segments::{exact_add, exact_mul};
    let mut out = s.clone();
    for (source, target) in s
        .control_points
        .iter()
        .flatten()
        .zip(out.control_points.iter_mut().flatten())
    {
        for row in 0..2 {
            let mut value = 0.;
            for k in 0..3 {
                value = exact_add(value, exact_mul(source[k], projection[row][k] as f64)?)?;
            }
            target[row] = value;
        }
        target[2] = 0.;
    }
    Some(out)
}
/// With weights constant along V, the first control row is an independent
/// polynomial station curve. Exact pole subtraction gives S=P(V)+R(U,V),
/// hence Su=Ru and Sv=Pv+Rv. Never replace a pole by a rounded difference.
fn exact_station_split(s: &Surface) -> Option<(Surface, Surface)> {
    if s.weights.iter().any(|row| row.iter().any(|w| *w != row[0])) {
        return None;
    }
    let mut relative = s.clone();
    let mut position = s.clone();
    for u in 0..s.control_points.len() {
        for v in 0..s.control_points[0].len() {
            for k in 0..3 {
                relative.control_points[u][v][k] = crate::exact_curve_segments::exact_add(
                    s.control_points[u][v][k],
                    -s.control_points[0][v][k],
                )?;
            }
            position.control_points[u][v] = s.control_points[0][v].clone();
            position.weights[u][v] = 1.;
        }
    }
    Some((relative, position))
}
fn inspect_with_depth(
    s: &Surface,
    projection: [[i8; 3]; 2],
    max_cells: usize,
    max_depth: usize,
) -> Result<Report> {
    let first = inspect_metric(s, projection, [1., 1.], max_cells, max_depth)?;
    if first.certified || first.cells == max_cells {
        return Ok(first);
    }
    let mut second = inspect_weighted(s, projection, max_cells - first.cells, max_depth)?;
    second.cells += first.cells;
    Ok(second)
}
fn inspect_weighted(
    s: &Surface,
    projection: [[i8; 3]; 2],
    max_cells: usize,
    max_depth: usize,
) -> Result<Report> {
    let u = 0.5 * s.knots_u[s.degree_u] + 0.5 * s.knots_u[s.control_points.len()];
    let v = 0.5 * s.knots_v[s.degree_v] + 0.5 * s.knots_v[s.control_points[0].len()];
    let Some((du, dv)) = s.evaluate(u, v)?.first_derivatives() else {
        return inspect_metric(s, projection, [1., 1.], 0, max_depth);
    };
    let diagonal = [
        (0..3).map(|k| projection[0][k] as f64 * du[k]).sum::<f64>(),
        (0..3).map(|k| projection[1][k] as f64 * dv[k]).sum::<f64>(),
    ];
    let scale = (diagonal[0] / diagonal[1]).abs();
    if !scale.is_finite() || scale <= 0. || scale == 1. {
        return inspect_metric(s, projection, [1., 1.], 0, max_depth);
    }
    // A sampled diagonal only proposes the fixed metric. Every chart cell
    // still requires the complete outward SPD proof; failures charge its budget.
    inspect_metric(s, projection, [scale, 1.], max_cells, max_depth)
}

/// Fixed common metric, strictly positive symmetric Jacobian only. This
/// contract can glue continuous charts across C0 seams; a P-matrix result on
/// either chart alone is insufficient for that conclusion.
pub fn inspect_strong_metric(s:&Surface,projection:[[i8;3];2],row_weights:[f64;2],max_cells:usize)->Result<Report> {
    inspect_metric_impl(s,projection,row_weights,[[1.,1.];2],max_cells,12,true)
}
fn inspect_metric(
    s: &Surface,
    projection: [[i8; 3]; 2],
    row_weights: [f64; 2],
    max_cells: usize,
    max_depth: usize,
) -> Result<Report> {
    inspect_metric_impl(s,projection,row_weights,[[1.,1.];2],max_cells,max_depth,false)
}
/// Strict common-metric proof after a constant affine change of source
/// parameter units. Rates enclose the exact constant source derivatives with
/// respect to the common parameters; original knots and poles stay unchanged.
pub fn inspect_strong_reparameterized(s:&Surface,projection:[[i8;3];2],row_weights:[f64;2],rates:[[f64;2];2],max_cells:usize)->Result<Report> {
    inspect_metric_impl(s,projection,row_weights,rates,max_cells,12,true)
}
fn inspect_metric_impl(s:&Surface,projection:[[i8;3];2],row_weights:[f64;2],rates:[[f64;2];2],max_cells:usize,max_depth:usize,strong_only:bool)->Result<Report> {
    s.validate()?;
    let columns=[I::new(rates[0][0],rates[0][1])?,I::new(rates[1][0],rates[1][1])?];
    check(columns.iter().all(|r|r.lo>0.||r.hi<0.),"Parameter rates must exclude zero")?;
    check(
        max_cells <= 100000
            && s.control_points.iter().flatten().all(|p| p.len() == 3)
            && row_weights.iter().all(|v| v.is_finite() && *v > 0.)
            && projection.iter().flatten().all(|v| (-64..=64).contains(v)),
        "Invalid linear projection or cell budget",
    )?;
    let independent = (0..3).any(|k| {
        projection[0][k] as i16 * projection[1][(k + 1) % 3] as i16
            != projection[1][k] as i16 * projection[0][(k + 1) % 3] as i16
    });
    check(independent, "Linear projection rows must be independent")?;
    let projected_surface = exact_projected_surface(s, projection);
    let working = projected_surface.as_ref().unwrap_or(s);
    let station_split = exact_station_split(working).map(|(r,p)|(r,p,1))
        .or_else(|| {
            let transposed=crate::surface_edit::transpose(working).ok()?;
            let (r,p)=exact_station_split(&transposed)?;
            Some((crate::surface_edit::transpose(&r).ok()?,crate::surface_edit::transpose(&p).ok()?,0))
        });
    let mut working_span=jets::FirstStableSpanCache::new(working);
    let mut split_spans=station_split.as_ref().map(|(relative,position,_)|
        (jets::FirstStableSpanCache::new(relative),jets::FirstStableSpanCache::new(position)));
    let mut out = Report {
        certified: false,
        cells: 0,
        projection,
        row_weights,
        profile_parameterization: None,
        reason: Some("linear-monotone-projection-unproved"),
    };
    let degrees = [s.degree_u, s.degree_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    if s.periodic_u
        || s.periodic_v
        || (0..2).any(|k| {
            knots[k]
                .iter()
                .filter(|v| **v > knots[k][degrees[k]] && **v < knots[k][counts[k]])
                .any(|v| knots[k].iter().filter(|x| *x == v).count() > degrees[k])
        })
    {
        out.reason = Some("continuous-rectangle-domain-unproved");
        return Ok(out);
    }
    // A closed coefficient hull proves one fixed positive symmetric metric
    // over the WHOLE rectangle, including continuous knot joins. Along every
    // segment, integral(delta^T J delta)>0 gives global injectivity. This
    // optional polynomial bound grants no success on arithmetic failure.
    if max_cells>0 && matches!(polynomial_projection::certify(s,projection,row_weights,rates),Ok(true)) {
        out.cells=1;out.certified=true;out.reason=None;return Ok(out);
    }
    // Gale--Nikaido, Math. Ann. 159 (1965), Theorem 4: a C1 map whose
    // Jacobian is a P-matrix everywhere on a rectangle is globally injective.
    // https://fig.if.usp.br/~marchett/fluidos/jacobian-matrix-global-univalence-mapping_gale-nikaido.pdf
    // A C0 knot join is excluded from this alternative. Strong monotonicity
    // below remains valid for the existing piecewise smooth continuous case.
    let continuously_differentiable = !strong_only && (0..2).all(|k| {
        knots[k]
            .iter()
            .filter(|v| **v > knots[k][degrees[k]] && **v < knots[k][counts[k]])
            .all(|v| knots[k].iter().filter(|x| *x == v).count() < degrees[k])
    });
    let mut pending = Vec::new();
    for u in degrees[0]..counts[0] {
        for v in degrees[1]..counts[1] {
            if knots[0][u] < knots[0][u + 1] && knots[1][v] < knots[1][v + 1] {
                if pending.len() == max_cells {
                    out.reason = Some("cell-budget-exhausted");
                    return Ok(out);
                }
                pending.push((
                    [u, v],
                    [
                        [knots[0][u], knots[0][u + 1]],
                        [knots[1][v], knots[1][v + 1]],
                    ],
                    0usize,
                ));
            }
        }
    }
    while let Some((span, domain, depth)) = pending.pop() {
        if out.cells == max_cells {
            out.reason = Some("cell-budget-exhausted");
            return Ok(out);
        }
        out.cells += 1;
        let result = (|| -> Result<i8> {
            // Differentiate the original homogeneous span before restricting
            // its value image. Query-cell differences amplify interval widths
            // on small translated rational walls and waste the shared budget.
            let derivatives = if let Some((relative,position))=&mut split_spans {
                let r=relative.calculate(span,domain)?;
                let p=position.calculate(span,domain)?;
                let mut derivatives=[r[1],r[2]];
                let axis=station_split.as_ref().unwrap().2;
                for k in 0..3 {derivatives[axis][k]=derivatives[axis][k].add(p[axis+1][k])?;}
                derivatives
            } else {
                let j=working_span.calculate(span,domain)?;
                [j[1],j[2]]
            };
            let mut projected = [[I::point(0.); 2]; 2];
            for row in 0..2 {
                for axis in 0..2 {
                    let width = I::point(knots[axis][span[axis] + 1])
                        .sub(I::point(knots[axis][span[axis]]))?;
                    if projected_surface.is_some() {
                        let derivative = derivatives[axis][row];
                        projected[row][axis] = derivative.div(width)?;
                        continue;
                    }
                    for k in 0..3 {
                        if projection[row][k] == 0 {
                            continue;
                        }
                        let derivative = derivatives[axis][k];
                        projected[row][axis] = projected[row][axis].add(
                            derivative
                                .div(width)?
                                .mul(I::point(projection[row][k] as f64))?,
                        )?;
                    }
                }
            }
            // Restrict-then-differentiate and differentiate-then-restrict are
            // independent outward enclosures of the same Jacobian. Neither
            // dominates for every retained chart. Intersect them rather than
            // losing the tighter enclosure on corrected miter walls.
            let restricted = jets::calculate_first(working,span,domain)?;
            for row in 0..2 {
                for axis in 0..2 {
                    let width = I::point(domain[axis][1]).sub(I::point(domain[axis][0]))?;
                    let derivative = if axis == 0 {
                        restricted[1]
                    } else {
                        restricted[2]
                    };
                    let other = if projected_surface.is_some() {
                        derivative[row].div(width)?
                    } else {
                        let mut value = I::point(0.);
                        for k in 0..3 {
                            if projection[row][k] != 0 {
                                value = value.add(
                                    derivative[k]
                                        .div(width)?
                                        .mul(I::point(projection[row][k] as f64))?,
                                )?;
                            }
                        }
                        value
                    };
                    projected[row][axis] = I::new(
                        projected[row][axis].lo.max(other.lo),
                        projected[row][axis].hi.min(other.hi),
                    )?;
                    projected[row][axis] = projected[row][axis].mul(I::point(row_weights[row]))?;
                    if rates[axis]!=[1.,1.] {projected[row][axis]=projected[row][axis].mul(columns[axis])?;}
                }
            }
            let [[uu, uv], [vu, vv]] = projected;
            if uu.hi <= 0. || vv.hi <= 0. {
                return Ok(-1);
            }
            let off = uv.add(vu)?.mul(I::point(0.5))?;
            let radius = I::point(off.lo.abs().max(off.hi.abs()));
            let diagonal = uu.lo > 0. && vv.lo > 0.;
            let strong_monotonicity = diagonal && uu.mul(vv)?.sub(radius.mul(radius)?)?.lo > 0.;
            let p_matrix =
                continuously_differentiable && diagonal && uu.mul(vv)?.sub(uv.mul(vu)?)?.lo > 0.;
            Ok(i8::from(strong_monotonicity || p_matrix))
        })();
        // Preserve the cheap outward-jet proofs and rigorous negative exits.
        // Correlated polynomial minors resolve only an otherwise unknown cell,
        // including numerical enclosure failures, under the same counted cell.
        let result = if !matches!(&result, Ok(1) | Ok(-1))
            && ((strong_only && bernstein_symmetric::certify(s,span,domain,projection,row_weights,rates).unwrap_or(false))
                || (continuously_differentiable && bernstein_minors::certify(s,span,domain,projection).unwrap_or(false)))
        {
            Ok(1)
        } else {
            result
        };
        match result {
            Ok(1) => (),
            Ok(0) if depth < max_depth => {
                let axis = depth % 2;
                let middle = domain[axis][0] * 0.5 + domain[axis][1] * 0.5;
                if !(domain[axis][0] < middle && middle < domain[axis][1]) {
                    out.reason = Some("parameter-subdivision-unresolved");
                    return Ok(out);
                }
                let mut left = domain;
                let mut right = domain;
                left[axis][1] = middle;
                right[axis][0] = middle;
                pending.push((span, right, depth + 1));
                pending.push((span, left, depth + 1));
            }
            Ok(0) => {
                out.reason = Some("subdivision-depth-exhausted");
                return Ok(out);
            }
            Err(_) => {
                out.reason = Some("numerical-enclosure-unresolved");
                return Ok(out);
            }
            _ => return Ok(out),
        }
    }
    out.certified = true;
    out.reason = None;
    Ok(out)
}
/// Exact comparisons of retained corner controls only propose a projection.
/// The full outward proof remains mandatory; no corner sample certifies a chart.
fn inspect_candidate_original(s: &Surface, max_cells: usize) -> Result<Report> {
    s.validate()?;
    if let Some((polynomial, parameterization)) = polynomial_profile_image(s)? {
        let mut report = inspect_candidate_projected(&polynomial, max_cells)?;
        report.profile_parameterization = Some(parameterization);
        return Ok(report);
    }
    inspect_candidate_projected(s, max_cells)
}
/// Exact positive profile reparameterization of a retained quintic image.
pub fn polynomial_profile_image(s: &Surface) -> Result<Option<(Surface, ProfileParameterization)>> {
    s.validate()?;
    // Constant longitudinal weights on a linear-profile quintic wall change
    // only its profile traversal. Prove the identical polynomial image under
    // the exact positive rational bijection, without modifying the source.
    if s.degree_u == 1
        && s.degree_v == 5
        && s.knots_u == [0., 0., 1., 1.]
        && s.control_points.len() == 2
        && !s.periodic_u
        && s.weights.iter().all(|row| row.iter().all(|w| *w == row[0]))
    {
        let weights = [s.weights[0][0], s.weights[1][0]];
        if weights != [1., 1.] && weights.iter().all(|w| (1e-100..=1e100).contains(w)) {
            let lower = I::point(weights[0].min(weights[1]))
                .div(I::point(weights[0].max(weights[1])))?
                .lo;
            if lower > 0. {
                let mut polynomial = s.clone();
                polynomial.weights.iter_mut().for_each(|row| row.fill(1.));
                return Ok(Some((
                    polynomial,
                    ProfileParameterization {
                        weights,
                        derivative_lower: lower,
                    },
                )));
            }
        }
    }
    Ok(None)
}
impl ProfileParameterization {
    pub fn inverse_interval(&self, lambda: [f64; 2]) -> Result<[f64; 2]> {
        Self {
            weights: [self.weights[1], self.weights[0]],
            derivative_lower: self.derivative_lower,
        }
        .map_interval(lambda)
        .map(|mapped| mapped.0)
    }

    pub fn map_interval(&self, u: [f64; 2]) -> Result<([f64; 2], [f64; 2])> {
        check(
            u[0] >= 0.
                && u[1] <= 1.
                && u[0] <= u[1]
                && self.weights.iter().all(|w| (1e-100..=1e100).contains(w)),
            "Invalid rational profile interval",
        )?;
        let x = I::new(u[0], u[1])?;
        let a = I::point(self.weights[0]);
        let b = I::point(self.weights[1]);
        let denominator = a.mul(I::point(1.).sub(x)?)?.add(b.mul(x)?)?.intersect(
            self.weights[0].min(self.weights[1]),
            self.weights[0].max(self.weights[1]),
        )?;
        let mapped = b.mul(x)?.div(denominator)?.intersect(0., 1.)?;
        let derivative = a.mul(b)?.div(denominator.mul(denominator)?)?;
        Ok(([mapped.lo, mapped.hi], [derivative.lo, derivative.hi]))
    }
}
fn inspect_candidate_projected(s: &Surface, max_cells: usize) -> Result<Report> {
    // Preserve the legacy unit-metric candidates and their shared budget first.
    // New metric proposals cannot consume work needed by existing certificates.
    // New degree-five linear-profile walls benefit from the metric during each
    // candidate attempt. Legacy source charts retain their original ordering.
    let quintic = s.degree_u == 1 && s.degree_v == 5;
    let first = inspect_candidate_proposals(s, max_cells, quintic)?;
    if first.certified || first.cells == max_cells {
        return Ok(first);
    }
    let independent = (0..3).any(|k| {
        first.projection[0][k] as i16 * first.projection[1][(k + 1) % 3] as i16
            != first.projection[1][k] as i16 * first.projection[0][(k + 1) % 3] as i16
    });
    if !independent {
        return Ok(first);
    }
    let mut second = inspect_weighted(s, first.projection, max_cells - first.cells, 14)?;
    second.cells += first.cells;
    Ok(second)
}
fn inspect_candidate_proposals(s: &Surface, max_cells: usize, weighted: bool) -> Result<Report> {
    s.validate()?;
    check(
        max_cells <= 100000 && s.control_points.iter().flatten().all(|p| p.len() == 3),
        "Invalid linear injectivity budget/dimension",
    )?;
    let a = &s.control_points[0][0];
    let ends = [
        &s.control_points[s.control_points.len() - 1][0],
        &s.control_points[0][s.control_points[0].len() - 1],
    ];
    let mut projection: [[i8; 3]; 2] = std::array::from_fn(|row| {
        std::array::from_fn(|k| {
            if ends[row][k] > a[k] {
                1
            } else if ends[row][k] < a[k] {
                -1
            } else {
                0
            }
        })
    });
    // An axis-aligned station chord can dominate the symmetric Jacobian if
    // included in both rows. Propose the transverse row without that axis.
    // This changes only the candidate; the same complete proof is required.
    let station_axes = (0..3)
        .filter(|&k| projection[1][k] != 0)
        .collect::<Vec<_>>();
    if station_axes.len() == 1 {
        projection[0][station_axes[0]] = 0;
    }
    let independent = (0..3).any(|k| {
        projection[0][k] as i16 * projection[1][(k + 1) % 3] as i16
            != projection[1][k] as i16 * projection[0][(k + 1) % 3] as i16
    });
    // A cheap dual-map attempt avoids spending the corner-map allowance on
    // thin spatial charts. Sampling only chooses the map; the whole-domain
    // interval proof and every attempted cell remain mandatory and charged.
    let mut preliminary_cells = 0;
    if let Some(candidate) = if weighted {None} else {midpoint_projection(s)?} {
        if candidate != projection {
            // A rational ruled station chart often needs more than a handful
            // of cells for its thin profile direction. Avoid redoing the same
            // dual-map search after two exhausted preliminary proposals.
            let ruled_station = s.degree_u > 1
                && s.degree_v == 1
                && s.control_points[0].len() == 2
                && s.weights.iter().all(|row| row.iter().all(|w| *w == row[0]));
            let allowance = if ruled_station { 512 } else { 16 };
            let preliminary = inspect_with_depth(s, candidate, max_cells.min(allowance), 14)?;
            if preliminary.certified || preliminary.cells == max_cells {
                return Ok(preliminary);
            }
            preliminary_cells = preliminary.cells;
        }
    }
    let remaining = max_cells - preliminary_cells;
    let mut first = if !independent {
        Report {
            certified: false,
            cells: 0,
            projection,
            row_weights: [1., 1.],
            profile_parameterization: None,
            reason: Some("linear-projection-proposal-unproved"),
        }
    } else {
        inspect_proposal(s, projection, if weighted {remaining} else {remaining.min(16)}, 12, weighted)?
    };
    first.cells += preliminary_cells;
    if first.certified || first.cells == max_cells {
        return Ok(first);
    }
    // Longitudinal motion can overwhelm a corner-sign proposal on a thin
    // spatial wall. Try the midpoint dual map with a bounded portion first;
    // its sampled construction is only a proposal, never positive evidence.
    if let Some(candidate) = midpoint_projection(s)? {
        if candidate != projection {
            let mut hinted =
                inspect_with_depth(s, candidate, (max_cells - first.cells).min(512), 14)?;
            hinted.cells += first.cells;
            if hinted.certified || hinted.cells == max_cells {
                return Ok(hinted);
            }
            first.cells = hinted.cells;
        }
    }
    // Preserve the original candidate as a full-budget fallback; a short
    // preliminary search is not treated as a negative geometry certificate.
    if independent && first.cells < max_cells {
        let mut retry = inspect(s, projection, max_cells - first.cells)?;
        retry.cells += first.cells;
        if retry.certified || retry.cells == max_cells {
            return Ok(retry);
        }
        first = retry;
    }
    // A rounded transverse direction only proposes another fixed integer map.
    // Every cell is proved again, with the remaining shared work budget.
    let chord: [f64; 3] = std::array::from_fn(|k| ends[1][k] - a[k]);
    let length2: f64 = chord.iter().map(|x| x * x).sum();
    let dot: f64 = (0..3).map(|k| projection[0][k] as f64 * chord[k]).sum();
    let transverse: [f64; 3] =
        std::array::from_fn(|k| projection[0][k] as f64 - dot / length2 * chord[k]);
    let scale = transverse.iter().map(|x| x.abs()).fold(0., f64::max);
    if !length2.is_finite() || length2 <= 0. || !scale.is_finite() || scale <= 0. {
        return Ok(first);
    }
    let station_scale = chord.iter().map(|x| x.abs()).fold(0., f64::max);
    let alternative = [
        transverse.map(|x| (2. * x / scale).round() as i8),
        chord.map(|x| (2. * x / station_scale).round() as i8),
    ];
    let independent = (0..3).any(|k| {
        alternative[0][k] as i16 * alternative[1][(k + 1) % 3] as i16
            != alternative[1][k] as i16 * alternative[0][(k + 1) % 3] as i16
    });
    if !independent || alternative == projection {
        return Ok(first);
    }
    let mut second = inspect_proposal(s, alternative, max_cells - first.cells, 12, weighted)?;
    second.cells += first.cells;
    if second.certified || second.cells == max_cells {
        return Ok(second);
    }
    let Some(candidate) = midpoint_projection(s)? else {
        return Ok(second);
    };
    if candidate == alternative || candidate == projection {
        return Ok(second);
    }
    let mut third = inspect_proposal(s, candidate, max_cells - second.cells, 14, weighted)?;
    third.cells += second.cells;
    Ok(third)
}
fn midpoint_projection(s: &Surface) -> Result<Option<[[i8; 3]; 2]>> {
    // Midpoint tangents propose dual directions. Every actual acceptance still
    // requires one fixed integer map's whole-domain outward interval proof.
    let u = 0.5 * s.knots_u[s.degree_u] + 0.5 * s.knots_u[s.control_points.len()];
    let v = 0.5 * s.knots_v[s.degree_v] + 0.5 * s.knots_v[s.control_points[0].len()];
    let Some((du, dv)) = s.evaluate(u, v)?.first_derivatives() else {
        return Ok(None);
    };
    let dot = |a: [f64; 3], b: [f64; 3]| (0..3).map(|k| a[k] * b[k]).sum::<f64>();
    let uu = dot(du, du);
    let uv = dot(du, dv);
    let vv = dot(dv, dv);
    let dual: [[f64; 3]; 2] = [
        std::array::from_fn(|k| vv * du[k] - uv * dv[k]),
        std::array::from_fn(|k| uu * dv[k] - uv * du[k]),
    ];
    let mut candidate = [[0_i8; 3]; 2];
    for row in 0..2 {
        let scale = dual[row].iter().map(|x| x.abs()).fold(0., f64::max);
        if !scale.is_finite() || scale <= 0. {
            return Ok(None);
        }
        candidate[row] = dual[row].map(|x| (64. * x / scale).round() as i8);
    }
    let independent = (0..3).any(|k| {
        candidate[0][k] as i16 * candidate[1][(k + 1) % 3] as i16
            != candidate[1][k] as i16 * candidate[0][(k + 1) % 3] as i16
    });
    Ok(independent.then_some(candidate))
}
fn inspect_proposal(
    s: &Surface,
    projection: [[i8; 3]; 2],
    max_cells: usize,
    depth: usize,
    weighted: bool,
) -> Result<Report> {
    if weighted {
        let first = inspect_weighted(s, projection, max_cells, depth)?;
        if first.certified || first.cells == max_cells {
            return Ok(first);
        }
        let mut second = inspect_metric(s, projection, [1., 1.], max_cells - first.cells, depth)?;
        second.cells += first.cells;
        Ok(second)
    } else {
        inspect_metric(s, projection, [1., 1.], max_cells, depth)
    }
}

pub fn inspect_candidate_with_hint(
    s: &Surface,
    hint: Option<[[i8; 3]; 2]>,
    max_cells: usize,
) -> Result<Report> {
    let Some(projection) = hint else {
        return inspect_candidate(s, max_cells);
    };
    let first = inspect(s, projection, max_cells)?;
    if first.certified || first.cells == max_cells {
        return Ok(first);
    }
    let mut fallback = inspect_candidate(s, max_cells - first.cells)?;
    fallback.cells += first.cells;
    Ok(fallback)
}

#[cfg(test)]
#[path = "surface_linear_monotonicity/tests.rs"]
mod retained_route_tests;

pub fn inspect_candidate(s: &Surface, max_cells: usize) -> Result<Report> {
    // Preserve every already successful original proposal and its work/report.
    // A finer integer dual map is only a proposal after the original cover
    // failed; acceptance still proves the same full-domain outward inequality.
    let mut report = inspect_candidate_original(s, max_cells)?;
    for resolution in [32., 64.] {
        if report.certified || report.cells == max_cells { break; }
        let Some(projection) = midpoint_projection_at_resolution(s, resolution)? else { break; };
        if projection == report.projection { continue; }
        let mut next = inspect_with_depth(s, projection, max_cells - report.cells, 14)?;
        next.cells += report.cells;
        report = next;
    }
    Ok(report)
}
fn midpoint_projection_at_resolution(s: &Surface, resolution: f64) -> Result<Option<[[i8; 3]; 2]>> {
    // Midpoint tangents propose dual directions. Every actual acceptance still
    // requires one fixed integer map's whole-domain outward interval proof.
    let u = 0.5 * s.knots_u[s.degree_u] + 0.5 * s.knots_u[s.control_points.len()];
    let v = 0.5 * s.knots_v[s.degree_v] + 0.5 * s.knots_v[s.control_points[0].len()];
    let Some((du, dv)) = s.evaluate(u, v)?.first_derivatives() else {
        return Ok(None);
    };
    let dot = |a: [f64; 3], b: [f64; 3]| (0..3).map(|k| a[k] * b[k]).sum::<f64>();
    let uu = dot(du, du);
    let uv = dot(du, dv);
    let vv = dot(dv, dv);
    let dual: [[f64; 3]; 2] = [
        std::array::from_fn(|k| vv * du[k] - uv * dv[k]),
        std::array::from_fn(|k| uu * dv[k] - uv * du[k]),
    ];
    let mut candidate = [[0_i8; 3]; 2];
    for row in 0..2 {
        let scale = dual[row].iter().map(|x| x.abs()).fold(0., f64::max);
        if !scale.is_finite() || scale <= 0. {
            return Ok(None);
        }
        candidate[row] = dual[row].map(|x| (resolution * x / scale).round() as i8);
    }
    let independent = (0..3).any(|k| {
        candidate[0][k] as i16 * candidate[1][(k + 1) % 3] as i16
            != candidate[1][k] as i16 * candidate[0][(k + 1) % 3] as i16
    });
    Ok(independent.then_some(candidate))
}
