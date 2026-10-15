//! Compatible retained Bezier profiles with exact shared adaptive-degree station jets.
//! This only constructs a bounded candidate; regularity/material stay separate.
use super::*;
use cad_predicates::{
    AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
};

fn zero_difference(
    a: [usize; 3],
    b: [usize; 3],
    coefficients: [usize; 3],
    source: &SourceArena,
    ctx: &mut PredicateContext<'_>,
) -> bool {
    let refs = |indices: [usize; 3]| indices.map(|i| source.leaf(i).unwrap());
    cad_predicates::direction_dot3d(ctx, refs(a), refs(b), refs([6, 6, 6]), refs(coefficients))
        .is_ok_and(|proof| proof.outcome == Outcome::Sign(Sign::Zero))
}
fn shared_jets(left: &Surface, right: &Surface, work: &mut u64, max_work: u64) -> bool {
    if left.weights != right.weights || left.control_points.len() != right.control_points.len() {
        return false;
    }
    for (a, b) in left.control_points.iter().zip(&right.control_points) {
        let last = a.len() - 1;
        if a[last] != b[0] {
            return false;
        }
        for k in 0..3 {
            let values = [
                a[last - 2][k],
                a[last - 1][k],
                a[last][k],
                b[0][k],
                b[1][k],
                b[2][k],
                0.,
                1.,
                -1.,
                -2.,
            ]
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .to_vec();
            let Ok(source) = SourceArena::authored("general-station-shared-jets", 1, values) else {
                return false;
            };
            let tolerance = ToleranceContext::default_valid();
            let mut ctx = PredicateContext::new(
                &source,
                &tolerance,
                Limits {
                    max_work: max_work - *work,
                    ..Limits::default()
                },
                None,
            );
            // Equal first and second differences, including nonzero shared
            // curvature. Positive V-constant weights give equal homogeneous
            // jets; equal degree/domain factors cancel on the two sides.
            let valid = zero_difference([1, 3, 6], [2, 4, 6], [7, 8, 6], &source, &mut ctx)
                && zero_difference([0, 1, 6], [5, 4, 6], [7, 9, 6], &source, &mut ctx);
            *work += ctx.work_used();
            if !valid {
                return false;
            }
        }
    }
    true
}

pub(super) fn construct(
    sections: &[Vec<Vec<Curve>>],
    sharp: &[usize],
    closed: bool,
    quantum: f64,
    tolerance: f64,
    max_work: u64,
) -> Result<SmoothStationWalls> {
    let n = sections.len();
    let loops = sections[0].len();
    let mut out = SmoothStationWalls {
        sides: None,
        wall_displacement_upper: None,
        work: 0,
        reason: "work-limit",
    };
    let span_limit = (MAX_FACES - if closed { 0 } else { 2 }) / (n - 1);
    let mut prepared = Vec::with_capacity(n);
    for station in sections {
        if station.len() != loops {
            out.reason = "incompatible-section-basis";
            return Ok(out);
        }
        let mut row = Vec::with_capacity(loops);
        let mut station_spans = 0;
        for (l, wire) in station.iter().enumerate() {
            if wire.len() > span_limit - station_spans {
                out.reason = "face-limit";
                return Ok(out);
            }
            if wire.len() != sections[0][l].len() {
                out.reason = "incompatible-section-basis";
                return Ok(out);
            }
            let mut spans = Vec::new();
            for (c, first) in wire.iter().zip(&sections[0][l]) {
                c.validate()?;
                if !charge(&mut out.work, max_work) {
                    return Ok(out);
                }
                if c.degree != first.degree
                    || c.knots != first.knots
                    || c.weights != first.weights
                    || c.periodic != first.periodic
                    || c.control_points.iter().any(|p| p.len() != 3)
                {
                    out.reason = "incompatible-section-basis";
                    return Ok(out);
                }
                spans.extend(retained_bezier_pieces(c)?);
                if spans.len() > span_limit - station_spans {
                    out.reason = "face-limit";
                    return Ok(out);
                }
            }
            if spans.is_empty()
                || spans.len() > MAX_FACES
                || spans
                    .iter()
                    .any(|p| p.control_points.len() != p.degree + 1 || p.control_points.len() > 33)
            {
                out.reason = "unsupported-profile";
                return Ok(out);
            }
            station_spans += spans.len();
            row.push(spans);
        }
        prepared.push(row);
    }
    let count = prepared[0].iter().map(Vec::len).sum::<usize>();
    if count * (n - 1) + if closed { 0 } else { 2 } > MAX_FACES {
        out.reason = "face-limit";
        return Ok(out);
    }
    for station in &prepared[1..] {
        for (a, b) in prepared[0].iter().zip(station) {
            if a.len() != b.len()
                || a.iter().zip(b).any(|(a, b)| {
                    a.degree != b.degree || a.knots != b.knots || a.weights != b.weights
                })
            {
                out.reason = "incompatible-section-basis";
                return Ok(out);
            }
        }
    }
    let is_sharp = |i: usize| {
        sharp.contains(&i)
            || closed && (i == 0 || i == n - 1) && (sharp.contains(&0) || sharp.contains(&(n - 1)))
    };
    'degree: for degree in [5usize, 13, 25] {
        out.wall_displacement_upper = None;
        out.reason = "work-limit";
        let mut sides = vec![Vec::new(); loops];
        let mut upper = 0_f64;
        for layer in 0..n - 1 {
            for l in 0..loops {
                for q in 0..prepared[0][l].len() {
                    let a = &prepared[layer][l][q];
                    let b = &prepared[layer + 1][l][q];
                    let mut poles = vec![vec![vec![0.; 3]; degree + 1]; a.control_points.len()];
                    for i in 0..a.control_points.len() {
                        for k in 0..3 {
                            if !charge(&mut out.work, max_work) {
                                return Ok(out);
                            }
                            let tangent = |before: usize, after: usize, den: f64| {
                                let units = ((prepared[after][l][q].control_points[i][k]
                                    - prepared[before][l][q].control_points[i][k])
                                    / den
                                    / quantum)
                                    .round();
                                (units.is_finite() && units.abs() < 2_f64.powi(48))
                                    .then_some(units * quantum)
                            };
                            let ta = if (!closed && layer == 0) || is_sharp(layer) {
                                tangent(layer, layer + 1, degree as f64)
                            } else {
                                tangent(
                                    if layer == 0 { n - 2 } else { layer - 1 },
                                    layer + 1,
                                    (2 * degree) as f64,
                                )
                            };
                            let tb = if (!closed && layer + 1 == n - 1) || is_sharp(layer + 1) {
                                tangent(layer, layer + 1, degree as f64)
                            } else {
                                tangent(
                                    layer,
                                    if layer + 1 == n - 1 { 1 } else { layer + 2 },
                                    (2 * degree) as f64,
                                )
                            };
                            let (Some(ta), Some(tb)) = (ta, tb) else {
                                out.reason = "station-tangent-range";
                                return Ok(out);
                            };
                            let p = a.control_points[i][k];
                            let r = b.control_points[i][k];
                            let paired_end = closed || layer + 1 != n - 1;
                            let (left_near, left_far) = if paired_end && !is_sharp(layer + 1) {
                                if !charge(&mut out.work, max_work) {
                                    return Ok(out);
                                }
                                // Derive the left jet from the same rounded
                                // right controls used by the next patch. This
                                // avoids independent p +/- t rounding across
                                // binary exponent boundaries. Exact predicates
                                // below still decide whether both jets agree.
                                let right_near = r + tb;
                                let right_far = r + 2. * tb;
                                let near = 2. * r - right_near;
                                (near, (2. * near - 2. * right_near) + right_far)
                            } else {
                                (r - tb, r - 2. * tb)
                            };
                            for j in 0..=degree {
                                let value = match j {
                                    0 => p,
                                    1 => p + ta,
                                    2 => p + 2. * ta,
                                    j if j == degree - 2 => left_far,
                                    j if j == degree - 1 => left_near,
                                    j if j == degree => r,
                                    _ => p + (r - p) * (j as f64 / degree as f64),
                                };
                                if !value.is_finite() {
                                    out.reason = "control-lattice-range";
                                    return Ok(out);
                                }
                                poles[i][j][k] = value;
                            }
                        }
                    }
                    for i in 0..poles.len() {
                        for j in 0..=degree {
                            let mut squared = Interval::point(0.);
                            for k in 0..3 {
                                if !charge(&mut out.work, max_work) {
                                    return Ok(out);
                                }
                                let t = Interval::point(j as f64)
                                    .div(Interval::point(degree as f64))?;
                                let original = Interval::point(a.control_points[i][k])
                                    .mul(Interval::point(1.).sub(t)?)?
                                    .add(Interval::point(b.control_points[i][k]).mul(t)?)?;
                                let delta = Interval::point(poles[i][j][k]).sub(original)?;
                                let radius = delta.lo.abs().max(delta.hi.abs());
                                squared = squared
                                    .add(Interval::point(radius).mul(Interval::point(radius))?)?;
                            }
                            upper = upper.max(squared.hi.max(0.).sqrt().next_up());
                        }
                    }
                    let surface = Surface {
                        degree_u: a.degree,
                        degree_v: degree,
                        knots_u: a.knots.clone(),
                        knots_v: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
                        control_points: poles,
                        weights: a.weights.iter().map(|&w| vec![w; degree + 1]).collect(),
                        periodic_u: false,
                        periodic_v: false,
                    };
                    if surface.validate().is_err() {
                        out.reason = "control-lattice-range";
                        return Ok(out);
                    }
                    sides[l].push(surface);
                }
            }
        }
        // This outward norm enclosure has a positive rounding floor at zero.
        // Preserve the original complete refusal without spending extra work.
        if tolerance == 0. && upper > 0. {
            out.wall_displacement_upper = Some(upper);
            out.reason = "displacement-budget";
            return Ok(out);
        }
        if upper > tolerance {
            let Some(tighter) = error_bound::tighten(&sides, tolerance, &mut out.work, max_work)?
            else {
                out.wall_displacement_upper = None;
                out.reason = "work-limit";
                return Ok(out);
            };
            upper = tighter;
        }
        out.wall_displacement_upper = Some(upper);
        if !upper.is_finite() || upper > tolerance {
            out.reason = "displacement-budget";
            continue;
        }
        for l in 0..loops {
            let spans = prepared[0][l].len();
            for station in 1..n - 1 {
                if is_sharp(station) {
                    continue;
                }
                for q in 0..spans {
                    if !shared_jets(
                        &sides[l][(station - 1) * spans + q],
                        &sides[l][station * spans + q],
                        &mut out.work,
                        max_work,
                    ) {
                        out.reason = "shared-station-jets-unproved";
                        // A rounded low-degree proposal can lose exact jets at
                        // an exponent boundary. Higher degrees use smaller
                        // offsets; recheck them under the remaining allowance.
                        continue 'degree;
                    }
                }
            }
            if closed && !is_sharp(0) {
                for q in 0..spans {
                    if !shared_jets(
                        &sides[l][(n - 2) * spans + q],
                        &sides[l][q],
                        &mut out.work,
                        max_work,
                    ) {
                        out.reason = "shared-station-jets-unproved";
                        continue 'degree;
                    }
                }
            }
        }
        out.sides = Some(sides);
        out.reason = if degree == 5 {
            "bounded-general-quintic-station-jets"
        } else {
            "bounded-general-higher-degree-station-jets"
        };
        return Ok(out);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_nonzero_second_jet_is_exact_and_curvature_mutation_is_refused() {
        // Authored binary64 controls at the observed exponent-boundary case.
        // The shared second difference is nonzero; G2 does not require zero curvature.
        let l2=f64::from_bits(0x3ff1f3b645a1c28c);
        let l1=f64::from_bits(0x3ff84189374bc28e);
        let station=f64::from_bits(0x3ffe8f5c28f5c28f);
        let r1=f64::from_bits(0x40026e978d4fe148);
        let r2=f64::from_bits(0x400595810624e148);
        let surface=|y:[f64;6]| Surface {
            degree_u:1,degree_v:5,knots_u:vec![0.,0.,1.,1.],
            knots_v:[vec![0.;6],vec![1.;6]].concat(),
            control_points:[0.,1.].into_iter().map(|x|y.iter().map(|&v|vec![x,v,0.]).collect()).collect(),
            weights:vec![vec![1.;6];2],periodic_u:false,periodic_v:false,
        };
        let left=surface([0.3,0.7,1.,l2,l1,station]);
        let right=surface([station,r1,r2,3.1,3.5,3.9]);
        let mut work=0;
        assert!(shared_jets(&left,&right,&mut work,10000));
        assert!(work>0&&work<=10000);
        let mut changed=right.clone();
        changed.control_points[0][2][1]=changed.control_points[0][2][1].next_up();
        let mut changed_work=0;
        assert!(!shared_jets(&left,&changed,&mut changed_work,10000),
            "Equal position/first jet must not conceal a changed second jet");
        let mut zero_work=0;
        assert!(!shared_jets(&left,&right,&mut zero_work,0));
        assert_eq!(zero_work,0);
    }
}
