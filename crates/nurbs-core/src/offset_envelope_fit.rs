//! Finite rational envelope proposals with full-rectangle error qualification.
//! Samples create a candidate only. Interval jets and an enclosed anchor prove
//! a pointwise error bound by the mean-value theorem; every leaf is retained.
use crate::{
    Result, check, distance_bounds::Interval as I, offset_contact_tangent, offset_envelope,
    surface::Surface, surface_contact::Verdict, surface_offset,
};
#[derive(Debug)]
pub struct Cell {
    pub domain: [[f64; 2]; 2],
    pub error_upper_mm: Option<f64>,
    pub anchor_error_mm: Option<[f64; 2]>,
    pub envelope_regular: bool,
    pub candidate_regular: bool,
    pub admitted: bool,
}
pub struct Report {
    pub cells: Vec<Cell>,
    pub visited: usize,
    pub envelope_queries: usize,
    pub approximation_proven: bool,
    pub reason: &'static str,
}
fn intervals(x: [[f64; 2]; 3]) -> Result<[I; 3]> {
    Ok([
        I::new(x[0][0], x[0][1])?,
        I::new(x[1][0], x[1][1])?,
        I::new(x[2][0], x[2][1])?,
    ])
}
fn cross(a: [I; 3], b: [I; 3]) -> Result<[I; 3]> {
    let mut r = [I::point(0.); 3];
    for k in 0..3 {
        r[k] = a[(k + 1) % 3]
            .mul(b[(k + 2) % 3])?
            .sub(a[(k + 2) % 3].mul(b[(k + 1) % 3])?)?;
    }
    Ok(r)
}
fn validate(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    axis: usize,
    drive: [f64; 2],
    free: [f64; 2],
    second: [[f64; 2]; 2],
    spans: usize,
) -> Result<()> {
    check(axis < 2, "Envelope fitting requires source axis 0 or 1")?;
    check(
        drive[0] < drive[1],
        "Envelope fitting needs a positive driving interval",
    )?;
    check(
        distances.iter().all(|d| d.is_finite())
            && distances[0] != 0.
            && distances[0].abs() == distances[1].abs(),
        "Envelope fitting requires equal nonzero absolute radii",
    )?;
    let mut first = [free; 2];
    first[axis] = drive;
    crate::normal_alignment::validate_rectangle(surfaces[0], first, spans)?;
    crate::normal_alignment::validate_rectangle(surfaces[1], second, spans)
}
/// Candidate U is the original driving source parameter; candidate V is the
/// rational arc parameter. Matching natural domains forbid a silent re-map.
/// A fit certificate is pointwise in this correspondence, stronger than an
/// unordered sample or one-sided closest-point comparison. It is not a trim,
/// G1 tolerance, embedding or final B-rep topology certificate.
pub fn certify(
    candidate: &Surface,
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    axis: usize,
    drive: [f64; 2],
    free: [f64; 2],
    second: [[f64; 2]; 2],
    spans: usize,
    tolerance: f64,
    max_cells: usize,
) -> Result<Report> {
    validate(surfaces, distances, axis, drive, free, second, spans)?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Choose a positive finite envelope fit tolerance",
    )?;
    check(
        max_cells > 0 && max_cells <= 1_000_000,
        "Envelope fit cell budget must be between 1 and 1000000",
    )?;
    candidate.validate()?;
    let natural = [
        [
            candidate.knots_u[candidate.degree_u],
            candidate.knots_u[candidate.control_points.len()],
        ],
        [
            candidate.knots_v[candidate.degree_v],
            candidate.knots_v[candidate.control_points[0].len()],
        ],
    ];
    check(
        natural == [drive, [0., 1.]],
        "Candidate domains must match the driving interval and arc [0,1]",
    )?;
    let smooth = offset_contact_tangent::differentiable(candidate, natural, 1);
    let mut out = Report {
        cells: vec![],
        visited: 0,
        envelope_queries: 0,
        approximation_proven: false,
        reason: "fit-work-limit",
    };
    let mut pending = vec![natural];
    let mut mismatch = false;
    while let Some(domain) = pending.pop() {
        out.visited += 1;
        let mut cell = Cell {
            domain,
            error_upper_mm: None,
            anchor_error_mm: None,
            envelope_regular: false,
            candidate_regular: false,
            admitted: false,
        };
        let mut scores = [domain[0][1] - domain[0][0], domain[1][1] - domain[1][0]];
        let envelope = offset_envelope::certify_arc(
            surfaces, distances, axis, domain[0], free, second, spans, domain[1], 1,
        )?;
        out.envelope_queries += 1;
        if smooth {
            if let Some(e) = envelope.cells.first() {
                cell.envelope_regular = e.regular;
                let jets = surface_offset::jacobian_bounds(candidate, domain, 0., spans)?;
                if let Some(jets) = jets.derivatives {
                    let cu = intervals(jets[0])?;
                    let cv = intervals(jets[1])?;
                    cell.candidate_regular = offset_contact_tangent::speed(cross(cu, cv)?)?.lo > 0.;
                    let eu = intervals(e.center_derivative)?;
                    let ev = intervals(e.arc_derivative)?;
                    let mut du = [I::point(0.); 3];
                    let mut dv = du;
                    for k in 0..3 {
                        du[k] = cu[k].sub(eu[k])?;
                        dv[k] = cv[k].sub(ev[k])?;
                    }
                    scores = [
                        offset_contact_tangent::speed(du)?.hi * scores[0],
                        offset_contact_tangent::speed(dv)?.hi * scores[1],
                    ];
                    let mut tm = domain[0][0] + (domain[0][1] - domain[0][0]) * 0.5;
                    // A one-ULP positive band encloses the exact anchor root.
                    // The anchor is its lower endpoint, not a floating solver sample.
                    if tm.next_up() > domain[0][1] {
                        tm = domain[0][0];
                    }
                    let sm = domain[1][0] + (domain[1][1] - domain[1][0]) * 0.5;
                    let anchor = offset_envelope::certify_arc(
                        surfaces,
                        distances,
                        axis,
                        [tm, tm.next_up()],
                        free,
                        second,
                        spans,
                        [sm, sm],
                        1,
                    )?;
                    out.envelope_queries += 1;
                    if let Some(a) = anchor.cells.first() {
                        let cb = crate::surface_distance::rectangle_bounds(
                            candidate,
                            [[tm, tm], [sm, sm]],
                        )?;
                        let ab = intervals(a.image)?;
                        let mut residual = [I::point(0.); 3];
                        for k in 0..3 {
                            residual[k] = I::new(cb[k][0], cb[k][1])?.sub(ab[k])?;
                        }
                        let error = offset_contact_tangent::speed(residual)?;
                        cell.anchor_error_mm = Some([error.lo, error.hi]);
                        mismatch |= error.lo > tolerance;
                        let dt = I::new(domain[0][0], domain[0][1])?.sub(I::point(tm))?;
                        let ds = I::new(domain[1][0], domain[1][1])?.sub(I::point(sm))?;
                        for k in 0..3 {
                            residual[k] = residual[k].add(du[k].mul(dt)?)?.add(dv[k].mul(ds)?)?;
                        }
                        let upper = offset_contact_tangent::speed(residual)?.hi;
                        cell.error_upper_mm = Some(upper);
                        cell.admitted =
                            upper <= tolerance && cell.envelope_regular && cell.candidate_regular;
                    }
                }
            }
        }
        let split_axis = if scores[0] >= scores[1] { 0 } else { 1 };
        let mid = domain[split_axis][0] + (domain[split_axis][1] - domain[split_axis][0]) * 0.5;
        let disproven = cell.anchor_error_mm.is_some_and(|x| x[0] > tolerance);
        if !cell.admitted
            && !disproven
            && smooth
            && mid > domain[split_axis][0]
            && mid < domain[split_axis][1]
            && out.visited + pending.len() + 2 <= max_cells
        {
            let mut left = domain;
            let mut right = domain;
            left[split_axis][1] = mid;
            right[split_axis][0] = mid;
            pending.push(right);
            pending.push(left);
        } else {
            out.cells.push(cell);
        }
    }
    out.approximation_proven = out.cells.iter().all(|c| c.admitted);
    out.reason = if out.approximation_proven {
        "pointwise-fit-within-tolerance"
    } else if mismatch {
        "candidate-mismatch"
    } else if !smooth {
        "candidate-continuity-unproven"
    } else {
        "fit-work-limit"
    };
    Ok(out)
}
/// Two enclosed contact sections propose a finite degree-(1,2) rational patch.
/// Midpoints and floating normals are proposal data only; `certify` must admit
/// the entire patch before it can become any qualified geometry.
pub fn propose(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    axis: usize,
    drive: [f64; 2],
    free: [f64; 2],
    second: [[f64; 2]; 2],
    spans: usize,
) -> Result<Option<Surface>> {
    validate(surfaces, distances, axis, drive, free, second, spans)?;
    let r = distances[0].abs();
    let mut controls = vec![];
    let mut weights = vec![];
    for t in drive {
        let Verdict::Witness(w) = surface_offset::certify_contact_section(
            surfaces, distances, axis, t, free, second, spans,
        )?
        else {
            return Ok(None);
        };
        let midpoint = |x: [f64; 2]| x[0] + (x[1] - x[0]) * 0.5;
        let center = w.point.map(midpoint);
        let uv = [w.first_uv.map(midpoint), w.second_uv.map(midpoint)];
        let mut dirs = [[0.; 3]; 2];
        for side in 0..2 {
            let e = surface_offset::evaluate(surfaces[side], uv[side], distances[side])?;
            dirs[side] = e.source_unit_normal.map(|n| -distances[side].signum() * n);
        }
        let d = 1. + dirs[0].iter().zip(dirs[1]).map(|(a, b)| a * b).sum::<f64>();
        if !d.is_finite() || d <= 0. {
            return Ok(None);
        };
        let mut row = vec![vec![0.; 3]; 3];
        for k in 0..3 {
            row[0][k] = center[k] + r * dirs[0][k];
            row[1][k] = center[k] + r * (dirs[0][k] + dirs[1][k]) / d;
            row[2][k] = center[k] + r * dirs[1][k];
        }
        controls.push(row);
        weights.push(vec![1., (d / 2.).sqrt(), 1.]);
    }
    let candidate = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![drive[0], drive[0], drive[1], drive[1]],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: controls,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    candidate.validate()?;
    Ok(Some(candidate))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn pair() -> [Surface; 2] {
        let a = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let mut b = a.clone();
        for row in &mut b.control_points {
            for p in row {
                let z = p[1];
                p[1] = 0.5;
                p[2] = z;
            }
        }
        [a, b]
    }
    fn query(
        candidate: &Surface,
        a: &Surface,
        b: &Surface,
        tolerance: f64,
        cells: usize,
    ) -> Report {
        certify(
            candidate,
            [a, b],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
            tolerance,
            cells,
        )
        .unwrap()
    }
    #[test]
    fn rational_patch_is_qualified_over_the_full_parameter_rectangle() {
        let [a, b] = pair();
        let c = propose(
            [&a, &b],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
        )
        .unwrap()
        .unwrap();
        let r = query(&c, &a, &b, 1e-4, 2047);
        assert!(r.approximation_proven, "{} {:?}", r.reason, r.cells);
        assert!(r.visited <= 2047);
        assert!(r.envelope_queries <= 2 * r.visited);
        let area: f64 = r
            .cells
            .iter()
            .map(|c| (c.domain[0][1] - c.domain[0][0]) * (c.domain[1][1] - c.domain[1][0]))
            .sum();
        assert!((area - 0.04).abs() < 1e-12);
        for cell in &r.cells {
            assert!(cell.error_upper_mm.unwrap() <= 1e-4);
            assert!(cell.candidate_regular && cell.envelope_regular);
        }
    }
    #[test]
    fn curved_rational_contact_produces_a_finite_patch_with_uniform_error_in_rotated_frames() {
        let a = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![[3., 0.], [3., 3.], [0., 3.]]
                .into_iter()
                .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 5.]])
                .collect(),
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.]
                .into_iter()
                .map(|w| vec![w; 2])
                .collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let [_, mut b] = pair();
        for row in &mut b.control_points {
            for p in row {
                let y = 4. * p[0];
                let z = 5. * p[2];
                *p = vec![2. - z, y, z];
            }
        }
        let x = 2. + 0.2 * 2_f64.sqrt() - 0.5;
        let y = (3.2_f64.powi(2) - x * x).sqrt();
        let mut u = 0.5;
        for _ in 0..8 {
            let e = surface_offset::evaluate(&a, [u, 0.1], 0.2).unwrap();
            u -= (e.point[0] - x) / e.du[0];
        }
        let bu = y / 4.;
        let bv = (0.5 - 0.2 / 2_f64.sqrt()) / 5.;
        let drive = [0.0999, 0.1001];
        let free = [u - 0.005, u + 0.005];
        let second = [[bu - 0.005, bu + 0.005], [bv - 0.005, bv + 0.005]];
        for rotated in [false, true] {
            let mut aa = a.clone();
            let mut bb = b.clone();
            if rotated {
                for s in [&mut aa, &mut bb] {
                    for row in &mut s.control_points {
                        for p in row {
                            *p = vec![p[2] + 17., p[0] - 9., p[1] + 23.];
                        }
                    }
                }
            }
            let c = propose([&aa, &bb], [0.2, 0.2], 1, drive, free, second, 2)
                .unwrap()
                .unwrap();
            let r = certify(
                &c,
                [&aa, &bb],
                [0.2, 0.2],
                1,
                drive,
                free,
                second,
                2,
                1e-3,
                2047,
            )
            .unwrap();
            assert!(
                r.approximation_proven,
                "{rotated}: {} {:?}",
                r.reason, r.cells
            );
            for cell in &r.cells {
                assert!(cell.error_upper_mm.unwrap() <= 1e-3);
            }
            // Independent sphere-radius checks on the final authored NURBS,
            // using the analytic center for this regression fixture.
            for t in [drive[0], 0.1, drive[1]] {
                for s in [0., 0.25, 0.5, 0.75, 1.] {
                    let p = c.evaluate(t, s).unwrap().point;
                    let x = 2. + 0.2 * 2_f64.sqrt() - 5. * t;
                    let y = (3.2_f64.powi(2) - x * x).sqrt();
                    let center = if rotated {
                        [5. * t + 17., x - 9., y + 23.]
                    } else {
                        [x, y, 5. * t]
                    };
                    let radius = p
                        .iter()
                        .zip(center)
                        .map(|(p, c)| (p - c).powi(2))
                        .sum::<f64>()
                        .sqrt();
                    assert!((radius - 0.2).abs() <= 1e-3);
                }
            }
        }
    }
    #[test]
    fn candidate_domains_and_continuity_are_part_of_admission() {
        let [a, b] = pair();
        let mut c = propose(
            [&a, &b],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
        )
        .unwrap()
        .unwrap();
        let original = c.clone();
        c.periodic_u = true;
        c.control_points.push(c.control_points[0].clone());
        c.weights.push(c.weights[0].clone());
        c.knots_u = vec![0.33, 0.35, 0.37, 0.39, 0.41];
        let r = query(&c, &a, &b, 1e-4, 3);
        assert!(!r.approximation_proven);
        assert_eq!(r.reason, "candidate-continuity-unproven");
        c = original;
        c.knots_u = vec![0., 0., 1., 1.];
        assert!(
            certify(
                &c,
                [&a, &b],
                [0.2, 0.2],
                0,
                [0.35, 0.39],
                [0.25, 0.35],
                [[0.30, 0.44], [0.15, 0.25]],
                2,
                1e-4,
                3
            )
            .is_err()
        );
    }
    #[test]
    fn modified_patch_and_work_stop_cannot_gain_qualification() {
        let [a, b] = pair();
        let mut c = propose(
            [&a, &b],
            [0.2, 0.2],
            0,
            [0.35, 0.39],
            [0.25, 0.35],
            [[0.30, 0.44], [0.15, 0.25]],
            2,
        )
        .unwrap()
        .unwrap();
        let stopped = query(&c, &a, &b, 1e-4, 1);
        assert!(!stopped.approximation_proven);
        assert_eq!(stopped.cells.len(), 1);
        assert_eq!(stopped.cells[0].domain, [[0.35, 0.39], [0., 1.]]);
        for row in &mut c.control_points {
            for p in row {
                p[2] += 0.01;
            }
        }
        let mismatch = query(&c, &a, &b, 1e-4, 2047);
        assert!(!mismatch.approximation_proven);
        assert_eq!(mismatch.reason, "candidate-mismatch");
        assert!(
            mismatch
                .cells
                .iter()
                .any(|c| c.anchor_error_mm.unwrap()[0] > 1e-4)
        );
    }
}
