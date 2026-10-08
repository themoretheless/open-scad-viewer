//! Sufficient whole-chart exclusion of chords aligned with endpoint normals.
//! Original rational derivative bounds cover every nonempty knot rectangle.
use crate::{check, distance_bounds::Interval as I, normal_alignment, surface::Surface, Result};

pub struct Certificate<'a> {
    surface: &'a Surface,
    domain: [[f64; 2]; 2],
    max_sine_squared: f64,
    cosine_squared_upper: f64,
}
impl<'a> Certificate<'a> {
    pub fn surface(&self) -> &'a Surface {
        self.surface
    }
    pub fn domain(&self) -> [[f64; 2]; 2] {
        self.domain
    }
    pub fn max_sine_squared(&self) -> f64 {
        self.max_sine_squared
    }
    pub fn cosine_squared_upper(&self) -> f64 {
        self.cosine_squared_upper
    }
}
pub struct Report<'a> {
    pub certificate: Option<Certificate<'a>>,
    pub spans: usize,
    pub complete: bool,
    pub cosine_squared_upper: Option<f64>,
    pub reason: &'static str,
}
fn separated_square(x: I) -> Result<Option<I>> {
    let lower = if x.lo > 0. {
        x.lo
    } else if x.hi < 0. {
        -x.hi
    } else {
        return Ok(None);
    };
    let upper = x.lo.abs().max(x.hi.abs());
    Ok(Some(I::new(
        (lower * lower).next_down().max(0.),
        (upper * upper).next_up(),
    )?))
}
fn cosine_upper(j: [[I; 2]; 3]) -> Result<f64> {
    let mut variation = I::point(0.);
    for row in j {
        for x in row {
            let width = I::point(x.hi).sub(I::point(x.lo))?;
            variation = variation.add(width.mul(width)?)?;
        }
    }
    let mut best = 1_f64;
    let mut gram_determinant = I::point(0.);
    for [a, b] in [[0, 1], [0, 2], [1, 2]] {
        let det = j[a][0].mul(j[b][1])?.sub(j[a][1].mul(j[b][0])?)?;
        let Some(det_squared) = separated_square(det)? else {
            continue;
        };
        if det_squared.lo <= 0. {
            continue;
        }
        gram_determinant = gram_determinant.add(det_squared)?;
        let mut norm = I::point(0.);
        for axis in [a, b] {
            for direction in 0..2 {
                let upper = j[axis][direction].lo.abs().max(j[axis][direction].hi.abs());
                norm = norm.add(I::point(upper).mul(I::point(upper))?)?;
            }
        }
        let ratio = variation.mul(norm)?.div(det_squared)?;
        best = best.min(ratio.hi);
    }
    // For every enclosed 3x2 mean differential M, Cauchy--Binet gives
    // det(M^T M) = sum of its three squared coordinate minors. Hence
    // sigma_min(M)^2 >= det(M^T M) / trace(M^T M). Summing separated
    // minor bounds preserves all original differentials and can strengthen
    // the single-coordinate lower bound above.
    if gram_determinant.lo > 0. {
        let mut trace = I::point(0.);
        for row in j {
            for x in row {
                let upper = x.lo.abs().max(x.hi.abs());
                trace = trace.add(I::point(upper).mul(I::point(upper))?)?;
            }
        }
        best = best.min(variation.mul(trace)?.div(gram_determinant)?.hi);
    }
    Ok(best)
}
/// For chord=M z, |n.chord|² <= ||n^T M||² |z|² while
/// |chord|² >= sigma_min(M)² |z|². Cauchy--Binet bounds sigma_min
/// from all separated minors. No endpoint normal is inferred from samples.
fn endpoint_matrix_cosine(j: [[I; 2]; 3], n: [I; 3]) -> Result<f64> {
    let mut determinant = I::point(0.);
    for [a, b] in [[0, 1], [0, 2], [1, 2]] {
        let d = j[a][0].mul(j[b][1])?.sub(j[a][1].mul(j[b][0])?)?;
        if let Some(x) = separated_square(d)? {
            determinant = determinant.add(x)?;
        }
    }
    let mut trace = I::point(0.);
    let mut normal_squared = I::point(0.);
    for k in 0..3 {
        if let Some(x) = separated_square(n[k])? {
            normal_squared = normal_squared.add(x)?;
        }
        for a in 0..2 {
            let upper = j[k][a].lo.abs().max(j[k][a].hi.abs());
            trace = trace.add(I::point(upper).mul(I::point(upper))?)?;
        }
    }
    if normal_squared.lo <= 0. {
        return Ok(1.);
    }
    // Gershgorin also bounds the largest eigenvalue of the 2x2 Gram
    // matrix. Nearly orthogonal columns need not pay the full trace bound.
    let mut diagonal = [I::point(0.); 2];
    let mut diagonal_lower = [I::point(0.); 2];
    let mut off_diagonal = I::point(0.);
    for row in j {
        for a in 0..2 {
            if let Some(lower) = separated_square(row[a])? {
                diagonal_lower[a] = diagonal_lower[a].add(lower)?;
            }
            let upper = row[a].lo.abs().max(row[a].hi.abs());
            diagonal[a] = diagonal[a].add(I::point(upper).mul(I::point(upper))?)?;
        }
        off_diagonal = off_diagonal.add(row[0].mul(row[1])?)?;
    }
    let off_upper = off_diagonal.lo.abs().max(off_diagonal.hi.abs());
    let gershgorin = diagonal[0].hi.max(diagonal[1].hi);
    let mut largest = I::point(gershgorin).add(I::point(off_upper))?.hi.min(trace.hi);
    // For each enclosed Gram matrix, increasing its diagonals and replacing
    // its off-diagonal by an absolute upper bound increases lambda_max.
    // Evaluate that dominating 2x2 matrix with outward arithmetic.
    let difference = I::point(diagonal[0].hi).sub(I::point(diagonal[1].hi))?;
    let difference_upper = difference.lo.abs().max(difference.hi.abs());
    let discriminant = I::point(difference_upper).mul(I::point(difference_upper))?
        .add(I::point(off_upper).mul(I::point(off_upper))?.mul(I::point(4.))?)?;
    let root_upper = discriminant.hi.sqrt().next_up();
    let eigenvalue_upper = I::point(diagonal[0].hi).add(I::point(diagonal[1].hi))?
        .add(I::point(root_upper))?.mul(I::point(0.5))?.hi;
    largest = largest.min(eigenvalue_upper);
    // lambda_min increases with either diagonal and decreases with |b|.
    // Thus lower diagonal norms and the absolute off-diagonal upper bound
    // yield a second independent lower bound for every enclosed matrix.
    let lower_diagonals = diagonal_lower.map(|x| x.lo.max(0.));
    let difference = I::point(lower_diagonals[0]).sub(I::point(lower_diagonals[1]))?;
    let difference_upper = difference.lo.abs().max(difference.hi.abs());
    let discriminant = I::point(difference_upper).mul(I::point(difference_upper))?
        .add(I::point(off_upper).mul(I::point(off_upper))?.mul(I::point(4.))?)?;
    let root_upper = discriminant.hi.sqrt().next_up();
    let direct_lower = I::point(lower_diagonals[0]).add(I::point(lower_diagonals[1]))?
        .sub(I::point(root_upper))?.mul(I::point(0.5))?.lo.max(0.);
    let minor_lower = if determinant.lo > 0. && largest > 0. {
        determinant.div(I::point(largest))?.lo.max(0.)
    } else { 0. };
    let singular_lower = direct_lower.max(minor_lower);
    if singular_lower <= 0. { return Ok(1.); }
    let mut numerator = I::point(0.);
    for a in 0..2 {
        let mut dot = I::point(0.);
        for k in 0..3 {
            dot = dot.add(n[k].mul(j[k][a])?)?;
        }
        let upper = dot.lo.abs().max(dot.hi.abs());
        numerator = numerator.add(I::point(upper).mul(I::point(upper))?)?;
    }
    Ok(numerator
        .div(normal_squared.mul(I::point(singular_lower))?)?
        .hi
        .min(1.))
}
/// Pair-specific exclusion, not an intrinsic certificate for the convex union.
/// Each attempted matrix charges its executed net restrictions; local paths
/// reserve six, other paths five. Endpoint normal restrictions are reused.
#[derive(Default)]
pub(crate) struct PairExclusion {
    pub excluded: bool,
    pub spans: usize,
    pub coordinate_path_excluded: bool,
    pub coordinate_path_attempted: bool,
    pub coordinate_path_available: bool,
    pub local_path_excluded: bool,
    pub local_path_margin: Option<f64>,
    pub local_path_attempted: bool,
    pub local_path_available: bool,
}
#[cfg(test)]
pub(crate) fn rational_pole_pair_exclusion(
    s: &Surface,
    pair: [[[f64; 2]; 2]; 2],
    differential: &crate::rational_pole_differential::Certificate<'_>,
    normals: &crate::factored_surface_normals::Certificate<'_>,
    sine: f64,
    max_spans: usize,
) -> Result<(bool, usize)> {
    let r = rational_pole_pair_report(s, pair, differential, normals, sine, max_spans)?;
    Ok((r.excluded, r.spans))
}
pub(crate) fn rational_pole_pair_report(
    s: &Surface,
    pair: [[[f64; 2]; 2]; 2],
    differential: &crate::rational_pole_differential::Certificate<'_>,
    normals: &crate::factored_surface_normals::Certificate<'_>,
    sine: f64,
    max_spans: usize,
) -> Result<PairExclusion> {
    rational_pole_pair_report_cached(s, pair, differential, normals, sine, max_spans, None)
}
pub(crate) fn rational_pole_pair_report_cached(
    s: &Surface,
    pair: [[[f64; 2]; 2]; 2],
    differential: &crate::rational_pole_differential::Certificate<'_>,
    normals: &crate::factored_surface_normals::Certificate<'_>,
    sine: f64,
    max_spans: usize,
    cached: Option<&crate::factored_surface_normals::PairBounds<'_>>,
) -> Result<PairExclusion> {
    check(
        std::ptr::eq(s, differential.surface())
            && std::ptr::eq(s, normals.surface())
            && differential.end() == normals.end(),
        "Use original pole facts for this pair",
    )?;
    check(
        sine.is_finite() && (0. ..1.).contains(&sine),
        "Valid pole pair sine threshold",
    )?;
    let components = cached.map(|cache| cache.components(normals, pair)).transpose()?;
    if max_spans < 5 {
        return Ok(PairExclusion::default());
    }
    let union = std::array::from_fn(|a| {
        [
            pair[0][a][0].min(pair[1][a][0]),
            pair[0][a][1].max(pair[1][a][1]),
        ]
    });
    let threshold = I::point(1.).sub(I::point(sine))?.lo;
    let mut used = 0;
    let mut attempted = false;
    let mut available = false;
    let mut local_attempted = false;
    let mut local_available = false;
    // Scoped to this invocation: source, pole orientation and rectangles have
    // already been validated and remain immutable throughout all three modes.
    // Charge a restriction only when its endpoint bound is first computed.
    let mut endpoint_bounds = components.map(|values| values.map(Some)).unwrap_or([None; 2]);
    // Start with the tightest independently confined chart; retain wider
    // alternatives when the endpoint path cannot be confined there.
    // Small budgets retain the original broad-chart route, which may be
    // the only confined path when there is no budget for alternatives.
    let modes = if max_spans >= 25 { [0, 4, 3, 1, 2] } else { [0, 1, 3, 4, 2] };
    for mode in modes {
        let local_path = matches!(mode, 1 | 3 | 4);
        let cost = if local_path { 6 } else { 5 };
        let coordinate_path = mode == 0;
        if max_spans - used < cost {
            break;
        }
        if coordinate_path {
            attempted = true;
        }
        if local_path {
            local_attempted = true;
        }
        let (matrix, spent) = match mode {
            0 => differential.coordinate_path_report(union)?,
            1 => differential.local_coordinate_path_report(union, 3.)?,
            3 => differential.local_coordinate_path_report(union, 1.)?,
            4 => differential.local_coordinate_path_report(union, 0.25)?,
            _ => differential.bounds_report(union)?,
        };
        used += spent;
        if local_path && matrix.is_some() {
            local_available = true;
        }
        if coordinate_path && matrix.is_some() {
            available = true;
        }
        let Some(mut j) = matrix else {
            continue;
        };
        for a in 0..2 {
            let scale = j
                .iter()
                .map(|r| r[a].lo.abs().max(r[a].hi.abs()))
                .fold(0_f64, f64::max);
            if scale >= f64::MIN_POSITIVE && scale.is_finite() {
                for row in &mut j {
                    row[a] = row[a].div(I::point(scale))?;
                }
            }
        }
        for (index, endpoint) in pair.into_iter().enumerate() {
            let b = if let Some(bounds) = endpoint_bounds[index] {
                bounds
            } else {
                if used == max_spans {
                    break;
                }
                let bounds = normals.bounds(endpoint)?;
                used += 1;
                endpoint_bounds[index] = Some(bounds);
                bounds
            };
            let scale = b.iter().flatten().map(|x| x.abs()).fold(0_f64, f64::max);
            if scale < f64::MIN_POSITIVE {
                continue;
            }
            let mut n = [I::point(0.); 3];
            for k in 0..3 {
                n[k] = I::new(b[k][0], b[k][1])?.div(I::point(scale))?;
            }
            if endpoint_matrix_cosine(j, n)? < threshold {
                return Ok(PairExclusion {
                    excluded: true,
                    spans: used,
                    coordinate_path_excluded: coordinate_path,
                    coordinate_path_attempted: attempted,
                    coordinate_path_available: available,
                    local_path_excluded: local_path,
                    local_path_margin: match mode { 1 => Some(3.), 3 => Some(1.),
                        4 => Some(0.25), _ => None },
                    local_path_attempted: local_attempted,
                    local_path_available: local_available,
                });
            }
        }
    }
    Ok(PairExclusion {
        excluded: false,
        spans: used,
        coordinate_path_excluded: false,
        coordinate_path_attempted: attempted,
        coordinate_path_available: available,
        local_path_excluded: false,
        local_path_margin: None,
        local_path_attempted: local_attempted,
        local_path_available: local_available,
    })
}
fn balanced_cosine_upper(j: [[I; 2]; 3]) -> Result<f64> {
    let mut best = cosine_upper(j)?;
    // J D is the same original differential in rescaled parameter coordinates:
    // chord = M D (D^-1 delta), and n.(J D)=0. The intrinsic inequality
    // therefore applies unchanged to any fixed positive diagonal D.
    let scales: [f64; 2] = std::array::from_fn(|a| {
        1. / j
            .iter()
            .map(|row| row[a].lo.abs().max(row[a].hi.abs()))
            .fold(0_f64, f64::max)
    });
    if scales.iter().all(|s| s.is_finite() && *s > 0.) {
        let mut balanced = j;
        let mut finite = true;
        for k in 0..3 {
            for a in 0..2 {
                match j[k][a].mul(I::point(scales[a])) {
                    Ok(x) => balanced[k][a] = x,
                    Err(_) => finite = false,
                }
            }
        }
        if finite {
            if let Ok(bound) = cosine_upper(balanced) {
                best = best.min(bound);
            }
        }
    }
    Ok(best)
}
/// If J ranges in H, the chord is M*delta with M in H by integration along
/// the straight parameter segment. A uniformly nonsingular coordinate minor
/// gives |chord| >= |det(M)|/||M||F * |delta|. At either endpoint n.J=0, so
/// |n.chord| <= |n| * ||J-J(endpoint)||F * |delta|. The squared ratio below
/// bounds cosine to every endpoint normal. This suffices to exclude aligned
/// chords, including arbitrarily close pairs, without dividing by UV distance.
pub fn qualify<'a>(s: &'a Surface, max_sine_squared: f64, max_spans: usize) -> Result<Report<'a>> {
    s.validate()?;
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    qualify_rectangle(s, domain, max_sine_squared, max_spans)
}
/// Excludes all aligned self chords whose endpoints lie in this original
/// parameter rectangle. A local certificate does not cover the full chart.
pub fn qualify_rectangle<'a>(
    s: &'a Surface,
    domain: [[f64; 2]; 2],
    max_sine_squared: f64,
    max_spans: usize,
) -> Result<Report<'a>> {
    check(
        max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared),
        "Self chord exclusion requires squared sine in [0,1)",
    )?;
    s.validate()?;
    check(
        domain.iter().all(|a| a[0] < a[1]),
        "Self chord rectangle requires positive widths",
    )?;
    normal_alignment::validate_rectangle(s, domain, max_spans)?;
    let mut out = Report {
        certificate: None,
        spans: 0,
        complete: false,
        cosine_squared_upper: None,
        reason: "self-chord-unproven",
    };
    let degrees = [s.degree_u, s.degree_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    // Integration across a knot needs a continuous chart. Repeated internal
    // knots beyond degree permit jumps; never infer continuity from samples.
    for axis in 0..2 {
        if degrees[axis] == 0 {
            out.reason = "self-chord-continuity-unproven";
            return Ok(out);
        }
        let mut i = degrees[axis] + 1;
        while i < counts[axis] {
            let knot = knots[axis][i];
            let mut end = i + 1;
            while end < knots[axis].len() && knots[axis][end] == knot {
                end += 1
            }
            if knot > domain[axis][0] && knot < domain[axis][1] && end - i > degrees[axis] {
                out.reason = "self-chord-continuity-unproven";
                return Ok(out);
            }
            i = end;
        }
    }
    let mut hull = [[[f64::INFINITY, f64::NEG_INFINITY]; 2]; 3];
    for u in degrees[0]..counts[0] {
        for v in degrees[1]..counts[1] {
            if knots[0][u] == knots[0][u + 1] || knots[1][v] == knots[1][v + 1] {
                continue;
            }
            let section = [
                [
                    knots[0][u].max(domain[0][0]),
                    knots[0][u + 1].min(domain[0][1]),
                ],
                [
                    knots[1][v].max(domain[1][0]),
                    knots[1][v + 1].min(domain[1][1]),
                ],
            ];
            if section.iter().any(|d| d[0] > d[1]) {
                continue;
            }
            if out.spans == max_spans {
                out.reason = "self-chord-span-limit";
                return Ok(out);
            }
            let j = normal_alignment::jacobian_on(s, [u, v], section)?;
            out.spans += 1;
            for k in 0..3 {
                for a in 0..2 {
                    hull[k][a][0] = hull[k][a][0].min(j[k][a].lo);
                    hull[k][a][1] = hull[k][a][1].max(j[k][a].hi);
                }
            }
        }
    }
    out.complete = true;
    let mut j = [[I::point(0.); 2]; 3];
    for k in 0..3 {
        for a in 0..2 {
            j[k][a] = I::new(hull[k][a][0], hull[k][a][1])?;
        }
    }
    let best = balanced_cosine_upper(j)?;
    out.cosine_squared_upper = Some(best);
    let threshold = I::point(1.).sub(I::point(max_sine_squared))?;
    if best < threshold.lo {
        out.certificate = Some(Certificate {
            surface: s,
            domain,
            max_sine_squared,
            cosine_squared_upper: best,
        });
        out.reason = "self-chord-normal-excluded";
    }
    Ok(out)
}
/// Excludes nonzero geometric chords using S_v=t² B on an exactly verified
/// collapsed Bezier rail. Along a straight UV segment, averaged S_u and the
/// positive t²-weighted average B lie in the same bounded differential hull.
/// Thus chord=M*(delta_u,delta_v integral(t²)); the ordinary singular-value
/// and endpoint-normal variation argument applies to this weighted vector.
/// If that vector is zero, the geometric chord is zero and is not a wall chord.
pub fn qualify_polar_rectangle<'a>(
    s: &'a Surface,
    domain: [[f64; 2]; 2],
    end: bool,
    max_sine_squared: f64,
    max_spans: usize,
) -> Result<Report<'a>> {
    s.validate()?;
    check(
        max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared),
        "Polar chord exclusion requires squared sine in [0,1)",
    )?;
    normal_alignment::validate_rectangle(s, domain, max_spans)?;
    let mut out = Report {
        certificate: None,
        spans: 0,
        complete: false,
        cosine_squared_upper: None,
        reason: "polar-chord-work-limit",
    };
    if max_spans < 3 {
        return Ok(out);
    }
    let Some(j) = crate::polar_surface_differential::bounds(s, end, domain)? else {
        out.reason = "polar-chord-factor-unproven";
        return Ok(out);
    };
    out.spans = 3;
    out.complete = true;
    let best = cosine_upper(j)?;
    out.cosine_squared_upper = Some(best);
    out.reason = "polar-chord-unproven";
    if best < I::point(1.).sub(I::point(max_sine_squared))?.lo {
        out.certificate = Some(Certificate {
            surface: s,
            domain,
            max_sine_squared,
            cosine_squared_upper: best,
        });
        out.reason = "polar-chord-normal-excluded";
    }
    Ok(out)
}
/// Uses the exact source-owned rational decomposition. Preparation work is
/// recorded separately; each query restricts five bounded Bernstein nets.
pub fn qualify_rational_polar_rectangle<'a>(
    s: &'a Surface,
    domain: [[f64; 2]; 2],
    proof: &crate::rational_pole_differential::Certificate<'a>,
    max_sine_squared: f64,
    max_spans: usize,
) -> Result<Report<'a>> {
    check(
        std::ptr::eq(s, proof.surface()),
        "Use the same original rational pole surface",
    )?;
    check(
        max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared),
        "Rational chord requires squared sine in [0,1)",
    )?;
    normal_alignment::validate_rectangle(s, domain, max_spans)?;
    let mut out = Report {
        certificate: None,
        spans: 0,
        complete: false,
        cosine_squared_upper: None,
        reason: "rational-polar-chord-work-limit",
    };
    if max_spans < 5 {
        return Ok(out);
    }
    out.spans = 5;
    let Some(j) = proof.bounds(domain)? else {
        out.reason = "rational-polar-longitudinal-unproven";
        return Ok(out);
    };
    let best = balanced_cosine_upper(j)?;
    out.complete = true;
    out.cosine_squared_upper = Some(best);
    out.reason = "rational-polar-chord-unproven";
    if best < I::point(1.).sub(I::point(max_sine_squared))?.lo {
        out.certificate = Some(Certificate {
            surface: s,
            domain,
            max_sine_squared,
            cosine_squared_upper: best,
        });
        out.reason = "rational-polar-chord-normal-excluded";
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn direct_gram_lower_bound_covers_interval_matrix_chords_and_aligned_case() {
        let j = [[I::new(0.9,1.1).unwrap(),I::point(0.)],
            [I::point(0.),I::new(0.9,1.1).unwrap()], [I::point(0.);2]];
        let n = [I::point(0.3),I::point(0.),I::point(1.)];
        let upper = endpoint_matrix_cosine(j,n).unwrap();
        assert!(upper < 0.13); // Minor / largest-eigenvalue alone exceeds .18.
        for a in [0.9,1.,1.1] { for b in [0.9,1.,1.1] {
            for x in [-1.,0.,1.] { for y in [-1.,0.,1.] {
                if x==0. && y==0. { continue; }
                let cosine = (0.3*a*x)*(0.3*a*x)/(1.09*((a*x)*(a*x)+(b*y)*(b*y)));
                assert!(cosine <= upper);
            }}
        }}
        assert_eq!(endpoint_matrix_cosine(j,[I::point(1.),I::point(0.),I::point(0.)])
            .unwrap(),1.);
    }
    #[test]
    fn unequal_gram_diagonals_enclose_sheared_independent_chords() {
        let j = [[I::point(1.), I::point(0.1)],
            [I::point(0.), I::point(0.5)], [I::point(0.), I::point(0.1)]];
        let n = [I::point(0.), I::point(0.4), I::point(1.)];
        let upper = endpoint_matrix_cosine(j, n).unwrap();
        assert!(upper < 0.31); // Gershgorin alone exceeds .328.
        for x in [-2., -1., -0.1, 0., 1., 2.] {
            for y in [-2., -1., 0., 1., 2.] {
                if x == 0. && y == 0. { continue; }
                let length_squared = (x+0.1*y)*(x+0.1*y)+0.26*y*y;
                let cosine_squared = (0.3*y)*(0.3*y)/(1.16*length_squared);
                assert!(cosine_squared <= upper);
            }
        }
    }
    #[test]
    fn endpoint_gram_eigenvalue_bound_encloses_independent_chord_directions() {
        let j = [[I::point(1.), I::point(0.)],
            [I::point(0.), I::point(1.)], [I::point(0.), I::point(0.2)]];
        let n = [I::point(0.), I::point(0.4), I::point(1.)];
        let upper = endpoint_matrix_cosine(j, n).unwrap();
        assert!(upper < 0.32); // The trace-only estimate exceeds 0.60.
        for x in [-2., -1., 0., 1., 2.] {
            for y in [-2., -1., 0., 1., 2.] {
                if x == 0. && y == 0. { continue; }
                let length_squared = x*x + 1.04*y*y;
                let cosine_squared = (0.6*y)*(0.6*y)/(1.16*length_squared);
                assert!(cosine_squared <= upper);
            }
        }
    }
    #[test]
    fn endpoint_normal_matrix_preserves_tangent_variation_and_refuses_aligned_chords() {
        let zero = I::point(0.);
        let wide = I::new(1., 10.).unwrap();
        let j = [[wide, zero], [zero, wide], [zero, zero]];
        assert_eq!(cosine_upper(j).unwrap(), 1.);
        assert!(endpoint_matrix_cosine(j, [zero, zero, I::point(1.)]).unwrap() < 1e-20);
        let exact = [[I::point(1.), zero], [zero, I::point(1.)], [zero, zero]];
        assert_eq!(
            endpoint_matrix_cosine(exact, [I::point(1.), zero, zero]).unwrap(),
            1.
        );
        assert_eq!(endpoint_matrix_cosine(exact, [zero; 3]).unwrap(), 1.);
        assert_eq!(
            endpoint_matrix_cosine([[zero; 2]; 3], [zero, zero, I::point(1.)]).unwrap(),
            1.
        );
    }
    #[test]
    fn combined_minors_strengthen_oblique_differential_lower_bound() {
        let low = I::new(-0.1, 0.1).unwrap();
        let high = I::new(0.9, 1.1).unwrap();
        let j = [[high, low], [high, high], [low, high]];
        // Independently computed single-minor best is at least
        // 0.24 * 2.44 / 0.8^2 = 0.915. All three minors jointly
        // give an upper cosine bound below 0.8 for the same enclosure.
        assert!(cosine_upper(j).unwrap() < 0.8);
        assert!(0.24 * 2.44 / 0.8_f64.powi(2) > 0.9);
    }
    #[test]
    fn parameter_units_preserve_curved_chord_bounds_and_rejections() {
        let mut old_bound_refused = false;
        for height in [0.01, 2.] {
            let original = graph(height);
            for scales in [
                [1048576., 0.00000095367431640625],
                [0.00000095367431640625, 1048576.],
            ] {
                let mut s = original.clone();
                for x in &mut s.knots_u {
                    *x *= scales[0];
                }
                for x in &mut s.knots_v {
                    *x *= scales[1];
                }
                if height == 0.01 {
                    let derivative = normal_alignment::jacobian_on(
                        &s,
                        [s.degree_u, s.degree_v],
                        [[0., scales[0]], [0., scales[1]]],
                    )
                    .unwrap();
                    old_bound_refused |= cosine_upper(derivative).unwrap() >= 1.;
                }
                let r = qualify(&s, 1e-6, 1).unwrap();
                if height == 2. {
                    assert!(r.certificate.is_none());
                    continue;
                }
                let proof = r
                    .certificate
                    .expect("Affine parameter units must not hide an intrinsic exclusion");
                assert!(std::ptr::eq(proof.surface(), &s));
                for a in 0..16 {
                    for b in 0..16 {
                        if a == b {
                            continue;
                        }
                        let u = a as f64 / 15.;
                        let v = b as f64 / 15.;
                        let dx = v - u;
                        let dz = 0.01 * (v * v - u * u);
                        let nx = -0.02 * u;
                        let cosine2 =
                            (nx * dx + dz).powi(2) / ((nx * nx + 1.) * (dx * dx + dz * dz));
                        assert!(cosine2 <= proof.cosine_squared_upper());
                    }
                }
            }
        }
        assert!(
            old_bound_refused,
            "Regression must exercise a previously inconclusive parameter scaling"
        );
    }
    fn graph(height: f64) -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..2)
                        .map(|v| vec![i as f64 / 2., v as f64, if i == 2 { height } else { 0. }])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn whole_curved_graph_excludes_aligned_chords_and_retains_source_identity() {
        let s = graph(0.01);
        let r = qualify(&s, 1e-6, 1).unwrap();
        assert!(r.complete);
        assert_eq!(r.spans, 1);
        let c = r.certificate.expect("small curvature full chart proof");
        assert!(std::ptr::eq(c.surface(), &s));
        assert_eq!(c.max_sine_squared(), 1e-6);
        assert!(c.cosine_squared_upper() < 0.001);
        let strong = graph(2.);
        assert!(qualify(&strong, 1e-6, 1).unwrap().certificate.is_none());
        let mut collapsed = s.clone();
        for row in &mut collapsed.control_points {
            for p in row {
                p[1] = 0.
            }
        }
        assert!(qualify(&collapsed, 1e-6, 1).unwrap().certificate.is_none());
        assert!(qualify(&s, 1., 1).is_err());
        assert!(qualify(&s, 1e-6, 0).is_err());
    }
    #[test]
    fn missing_knot_span_or_discontinuous_chart_never_excludes() {
        let mut s = graph(0.01);
        s.degree_u = 1;
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        let r = qualify(&s, 1e-6, 1).unwrap();
        assert!(!r.complete);
        assert_eq!(r.spans, 1);
        assert!(r.certificate.is_none());
        assert!(qualify(&s, 1e-6, 2).unwrap().certificate.is_some());
        s.control_points.insert(2, s.control_points[1].clone());
        s.weights.insert(2, vec![1.; 2]);
        s.knots_u = vec![0., 0., 0.5, 0.5, 1., 1.];
        assert!(qualify(&s, 1e-6, 2).is_err());
    }
    #[test]
    fn independent_parabola_chords_and_reversed_coordinates_stay_inside_bound() {
        let mut s = graph(0.01);
        for sign in [1., -1.] {
            for row in &mut s.control_points {
                for p in row {
                    p[0] = sign * p[0].abs();
                }
            }
            let proof = qualify(&s, 1e-6, 1).unwrap().certificate.unwrap();
            for a in 0..16 {
                for b in 0..16 {
                    if a == b {
                        continue;
                    }
                    let u = a as f64 / 15.;
                    let v = b as f64 / 15.;
                    // Independent analytic S(u,t)=(sign*u,t,0.01*u^2).
                    let dx = sign * (v - u);
                    let dz = 0.01 * (v * v - u * u);
                    let nx = -sign * 0.02 * u;
                    let cosine2 = (nx * dx + dz).powi(2) / ((nx * nx + 1.) * (dx * dx + dz * dz));
                    assert!(cosine2 <= proof.cosine_squared_upper());
                }
            }
        }
    }
    #[test]
    fn original_rational_weights_have_continuous_exclusion_with_analytic_oracle() {
        let mut s = graph(0.01);
        let w = 1.001;
        s.weights[2] = vec![w; 2];
        let proof = qualify(&s, 1e-6, 1)
            .unwrap()
            .certificate
            .expect("mild rational graph");
        let point = |u: f64| {
            let d = 1. + (w - 1.) * u * u;
            [(u + (w - 1.) * u * u) / d, w * 0.01 * u * u / d]
        };
        for a in 0..16 {
            for b in 0..16 {
                if a == b {
                    continue;
                }
                let u = a as f64 / 15.;
                let v = b as f64 / 15.;
                let d = 1. + (w - 1.) * u * u;
                let dxdu = ((1. + 2. * (w - 1.) * u) * d
                    - (u + (w - 1.) * u * u) * 2. * (w - 1.) * u)
                    / (d * d);
                let dzdu = 2. * w * 0.01 * u / (d * d);
                let pa = point(u);
                let pb = point(v);
                let dx = pb[0] - pa[0];
                let dz = pb[1] - pa[1];
                let cosine2 = (-dzdu * dx + dxdu * dz).powi(2)
                    / ((dxdu * dxdu + dzdu * dzdu) * (dx * dx + dz * dz));
                assert!(cosine2 <= proof.cosine_squared_upper());
            }
        }
    }
    #[test]
    fn local_certificate_retains_its_rectangle_and_does_not_prove_full_chart() {
        let s = graph(2.);
        let domain = [[0.2, 0.21], [0.3, 0.8]];
        assert!(qualify(&s, 1e-6, 1).unwrap().certificate.is_none());
        let c = qualify_rectangle(&s, domain, 1e-6, 1)
            .unwrap()
            .certificate
            .unwrap();
        assert_eq!(c.domain(), domain);
        assert!(std::ptr::eq(c.surface(), &s));
        assert!(c.cosine_squared_upper() < 0.01);
        assert!(qualify_rectangle(&s, [[0.2, 0.2], [0., 1.]], 1e-6, 1).is_err());
        assert!(qualify_rectangle(&s, [[-0.1, 0.2], [0., 1.]], 1e-6, 1).is_err());
    }
    #[test]
    fn local_rectangle_spends_only_intersecting_original_spans() {
        let mut s = graph(0.01);
        s.degree_u = 1;
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        let domain = [[0.6, 0.8], [0., 1.]];
        let r = qualify_rectangle(&s, domain, 1e-6, 1).unwrap();
        assert!(r.complete);
        assert_eq!(r.spans, 1);
        assert!(r.certificate.is_some());
        let r = qualify_rectangle(&s, [[0.4, 0.6], [0., 1.]], 1e-6, 1).unwrap();
        assert!(!r.complete);
        assert!(r.certificate.is_none());
    }
}
