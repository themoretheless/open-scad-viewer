//! Bounded quintic candidates with shared binary-lattice station jets.
//! No regularity, embedding or Solid claim is issued by this construction.
use super::*;
use nurbs_core::interval_eval::Interval;
pub struct SmoothStationWalls {
    pub sides: Option<Vec<Vec<Surface>>>,
    pub wall_displacement_upper: Option<f64>,
    pub work: u64,
    pub reason: &'static str,
}
type Gen = [[f64; 3]; 3];
const SIGNS: [(f64, f64); 9] = [
    (1., 0.),
    (1., 1.),
    (0., 1.),
    (-1., 1.),
    (-1., 0.),
    (-1., -1.),
    (0., -1.),
    (1., -1.),
    (1., 0.),
];
fn charge(work: &mut u64, max: u64) -> bool {
    if *work == max {
        return false;
    }
    *work += 1;
    true
}
fn tangent(a: &Gen, b: &Gen, den: f64) -> Gen {
    std::array::from_fn(|i| std::array::from_fn(|k| ((b[i][k] - a[i][k]) / den).round()))
}
fn shifted(p: &Gen, t: &Gen, factor: f64) -> Gen {
    std::array::from_fn(|i| std::array::from_fn(|k| p[i][k] + factor * t[i][k]))
}

pub fn smooth_station_walls(
    sections: &[Vec<Vec<Curve>>],
    sharp: &[usize],
    closed: bool,
    quantum: f64,
    tolerance: f64,
    max_work: u64,
) -> Result<SmoothStationWalls> {
    if !(2..=129).contains(&sections.len())
        || sections[0].is_empty()
        || sections[0].len() > 16
        || !quantum.is_finite()
        || quantum <= 0.
        || !tolerance.is_finite()
        || tolerance < 0.
        || max_work > 1_000_000
    {
        return Err(err("Invalid smooth-station construction request"));
    }
    let bits = quantum.to_bits();
    if bits & ((1u64 << 52) - 1) != 0 || (bits >> 52) & 2047 == 0 {
        return Err(err("Smooth-station quantum must be a normal power of two"));
    }
    let n = sections.len();
    let loops = sections[0].len();
    let mut report = SmoothStationWalls {
        sides: None,
        wall_displacement_upper: None,
        work: 0,
        reason: "work-limit",
    };
    if loops * 4 * (n - 1) + if closed { 0 } else { 2 } > MAX_FACES {
        report.reason = "face-limit";
        return Ok(report);
    }
    if sharp.iter().any(|i| *i >= n)
        || sharp
            .iter()
            .enumerate()
            .any(|(i, x)| sharp[..i].contains(x))
    {
        return Err(err("Invalid sharp station indices"));
    }
    if closed && (n < 4 || sections.first() != sections.last()) {
        report.reason = "periodic-source-mismatch";
        return Ok(report);
    }
    let mut generators = Vec::<Vec<Gen>>::new();
    for station in sections {
        if station.len() != loops {
            report.reason = "incompatible-section-basis";
            return Ok(report);
        }
        let mut row = Vec::new();
        for (l, wire) in station.iter().enumerate() {
            if wire.len() != 1 {
                report.reason = "unsupported-profile";
                return Ok(report);
            }
            let c = &wire[0];
            c.validate()?;
            if c.degree != 2
                || c.control_points.len() != 9
                || c.control_points.iter().any(|p| p.len() != 3)
                || c.weights.len() != 9
                || c.weights
                    .iter()
                    .enumerate()
                    .any(|(i, w)| *w <= 0. || *w != c.weights[i % 2])
                || c.periodic
                || c.control_points[0] != c.control_points[8]
            {
                report.reason = "unsupported-profile";
                return Ok(report);
            }
            let first = &sections[0][l][0];
            if c.knots != first.knots || c.weights != first.weights {
                report.reason = "incompatible-section-basis";
                return Ok(report);
            }
            let prepared = retained_bezier_pieces(c)?;
            if prepared.len() != 4
                || prepared.iter().enumerate().any(|(q, p)| {
                    p.degree != 2
                        || p.control_points != c.control_points[2 * q..2 * q + 3]
                        || p.weights != c.weights[2 * q..2 * q + 3]
                })
            {
                report.reason = "noncanonical-source-decomposition";
                return Ok(report);
            }
            let mut g = [[0.; 3]; 3];
            for k in 0..3 {
                let p0 = c.control_points[0][k] / quantum;
                let p4 = c.control_points[4][k] / quantum;
                g[0][k] = (p0 + p4) * 0.5;
                g[1][k] = p0 - g[0][k];
                g[2][k] = c.control_points[2][k] / quantum - g[0][k];
            }
            if g.iter()
                .flatten()
                .any(|x| !x.is_finite() || x.fract() != 0. || x.abs() >= 2_f64.powi(48))
            {
                report.reason = "generator-lattice-range";
                return Ok(report);
            }
            for (i, (a, b)) in SIGNS.iter().copied().enumerate() {
                for k in 0..3 {
                    if !charge(&mut report.work, max_work) {
                        return Ok(report);
                    }
                    let integer = g[0][k] + a * g[1][k] + b * g[2][k];
                    if integer * quantum != c.control_points[i][k]
                        || c.control_points[i][k] / quantum != integer
                    {
                        report.reason = "source-generator-identity";
                        return Ok(report);
                    }
                }
            }
            row.push(g);
        }
        generators.push(row);
    }
    let is_sharp = |i: usize| {
        sharp.contains(&i)
            || (closed
                && (i == 0 || i == n - 1)
                && (sharp.contains(&0) || sharp.contains(&(n - 1))))
    };
    let mut sides = vec![Vec::new(); loops];
    let mut upper = 0_f64;
    for layer in 0..n - 1 {
        for l in 0..loops {
            let a = &generators[layer][l];
            let b = &generators[layer + 1][l];
            let ta = if (!closed && layer == 0) || is_sharp(layer) {
                tangent(a, b, 5.)
            } else {
                let prev = if layer == 0 { n - 2 } else { layer - 1 };
                tangent(&generators[prev][l], b, 10.)
            };
            let tb = if (!closed && layer + 1 == n - 1) || is_sharp(layer + 1) {
                tangent(a, b, 5.)
            } else {
                let next = if layer + 1 == n - 1 { 1 } else { layer + 2 };
                tangent(a, &generators[next][l], 10.)
            };
            let controls = [
                *a,
                shifted(a, &ta, 1.),
                shifted(a, &ta, 2.),
                shifted(b, &tb, -2.),
                shifted(b, &tb, -1.),
                *b,
            ];
            let mut poles = vec![vec![vec![0.; 3]; 6]; 9];
            for (j, g) in controls.iter().enumerate() {
                for (i, (u, v)) in SIGNS.iter().copied().enumerate() {
                    let mut squared = Interval::point(0.);
                    for k in 0..3 {
                        if !charge(&mut report.work, max_work) {
                            return Ok(report);
                        }
                        let integer = g[0][k] + u * g[1][k] + v * g[2][k];
                        let point = integer * quantum;
                        if !point.is_finite()
                            || integer.abs() >= 2_f64.powi(51)
                            || point / quantum != integer
                        {
                            report.reason = "control-lattice-range";
                            return Ok(report);
                        }
                        poles[i][j][k] = point;
                        let t = Interval::point(j as f64).div(Interval::point(5.))?;
                        let original = Interval::point(sections[layer][l][0].control_points[i][k])
                            .mul(Interval::point(1.).sub(t)?)?
                            .add(
                                Interval::point(sections[layer + 1][l][0].control_points[i][k])
                                    .mul(t)?,
                            )?;
                        let delta = Interval::point(point).sub(original)?;
                        let radius = delta.lo.abs().max(delta.hi.abs());
                        squared =
                            squared.add(Interval::point(radius).mul(Interval::point(radius))?)?;
                    }
                    upper = upper.max(squared.hi.max(0.).sqrt().next_up());
                }
            }
            for q in 0..4 {
                sides[l].push(Surface {
                    degree_u: 2,
                    degree_v: 5,
                    knots_u: vec![0., 0., 0., 1., 1., 1.],
                    knots_v: vec![0., 0., 0., 0., 0., 0., 1., 1., 1., 1., 1., 1.],
                    control_points: poles[2 * q..2 * q + 3].to_vec(),
                    weights: sections[layer][l][0].weights[2 * q..2 * q + 3]
                        .iter()
                        .map(|w| vec![*w; 6])
                        .collect(),
                    periodic_u: false,
                    periodic_v: false,
                });
            }
        }
    }
    report.wall_displacement_upper = Some(upper);
    if upper > tolerance {
        report.reason = "displacement-budget";
        return Ok(report);
    }
    report.sides = Some(sides);
    report.reason = "bounded-shared-quintic-station-jets";
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn section(x: f64, z: f64) -> Vec<Vec<Curve>> {
        vec![vec![Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.],
            control_points: SIGNS
                .iter()
                .map(|(a, b)| vec![x + a, b.to_owned(), z])
                .collect(),
            weights: (0..9)
                .map(|i| {
                    if i % 2 == 0 {
                        1.
                    } else {
                        std::f64::consts::FRAC_1_SQRT_2
                    }
                })
                .collect(),
            periodic: false,
        }]]
    }
    #[test]
    fn shared_quintic_jets_and_atomic_limits() {
        let sections = vec![section(0., 0.), section(1., 5.), section(0., 10.)];
        let report = smooth_station_walls(&sections, &[], false, 0.125, 10., 10000).unwrap();
        let sides = report.sides.unwrap();
        assert_eq!(sides[0].len(), 8);
        assert!(report.wall_displacement_upper.unwrap() > 0.);
        for q in 0..4 {
            let a = &sides[0][q];
            let b = &sides[0][4 + q];
            a.validate().unwrap();
            b.validate().unwrap();
            for i in 0..3 {
                for k in 0..3 {
                    let p = &a.control_points[i];
                    let r = &b.control_points[i];
                    assert_eq!(p[5][k], r[0][k]);
                    assert_eq!(p[5][k] - p[4][k], r[1][k] - r[0][k]);
                    assert_eq!(p[5][k] - 2. * p[4][k] + p[3][k], 0.);
                    assert_eq!(r[2][k] - 2. * r[1][k] + r[0][k], 0.);
                }
            }
        }
        let model = section_loft_surfaces(&sections, &sides, false).unwrap();
        model.validate().unwrap();
        for faces in [[0, 1], [2, 3], [4, 5], [6, 7]] {
            assert!(
                crate::shared_boundary::inspect_opposite_pair(&model, faces)
                    .unwrap()
                    .is_some()
            );
        }
        let limits = crate::volume_validity::Limits {
            boundary: crate::boundary_embedding::Limits {
                exact_work: 1_000_000,
                trim_pairs: 10_000,
                trim_cells: 100_000,
                trim_domain_cells: 1_000_000,
                spans: 1000,
                contacts: crate::face_contacts::Limits {
                    pairs: 10_000,
                    cells: 100_000,
                    domain_cells: 1_000_000,
                    cells_per_pair: 1000,
                    domain_cells_per_pair: 10_000,
                },
            },
            nesting_pairs: 1000,
            nesting_cells: 100_000,
            nesting_domain_cells: 1_000_000,
            orientation_cells: 100_000,
            orientation_domain_cells: 1_000_000,
            orientation_spans: 100,
        };
        let proof = crate::volume_validity::inspect_sweep(
            &model,
            1e-8,
            limits,
            100_000,
            &[8, 9],
            crate::sweep_cap_contacts::Budgets {
                max_walls: 1024,
                max_exact_work: 1_000_000,
                max_chart_cells: 1000,
                max_trim_pairs: 100_000,
                max_trim_cells: 100_000,
                max_trim_domain_cells: 1_000_000,
            },
        )
        .unwrap();
        assert!(proof.boundary.proven);
        assert!(proof.proven);
        assert_eq!(proof.orientations[0].outward, Some(true));
        let denied = smooth_station_walls(&sections, &[], false, 0.125, 0., 10000).unwrap();
        assert!(denied.sides.is_none());
        assert_eq!(denied.reason, "displacement-budget");
        let denied = smooth_station_walls(&sections, &[], false, 0.125, 10., 82).unwrap();
        assert!(denied.sides.is_none());
        assert_eq!(denied.work, 82);
        assert_eq!(denied.reason, "work-limit");
        let sharp = smooth_station_walls(&sections, &[1], false, 0.125, 10., 10000)
            .unwrap()
            .sides
            .unwrap();
        assert_ne!(
            sharp[0][0].control_points[0][5][0] - sharp[0][0].control_points[0][4][0],
            sharp[0][4].control_points[0][1][0] - sharp[0][4].control_points[0][0][0]
        );
    }
    #[test]
    fn closed_station_jets_include_the_wrap() {
        let sections = vec![
            section(0., 0.),
            section(2., 0.),
            section(2., 2.),
            section(0., 0.),
        ];
        let sides = smooth_station_walls(&sections, &[], true, 0.125, 10., 10000)
            .unwrap()
            .sides
            .unwrap();
        let sharp_first = smooth_station_walls(&sections, &[0], true, 0.125, 10., 10000).unwrap();
        let sharp_last = smooth_station_walls(&sections, &[3], true, 0.125, 10., 10000).unwrap();
        assert_eq!(sharp_first.sides, sharp_last.sides);
        for q in 0..4 {
            for i in 0..3 {
                for k in 0..3 {
                    let a = &sides[0][8 + q].control_points[i];
                    let b = &sides[0][q].control_points[i];
                    assert_eq!(a[5][k], b[0][k]);
                    assert_eq!(a[5][k] - a[4][k], b[1][k] - b[0][k]);
                }
            }
        }
        let mut incompatible = sections;
        incompatible[1][0][0].weights[1] = 0.5;
        assert!(
            smooth_station_walls(&incompatible, &[], true, 0.125, 10., 10000)
                .unwrap()
                .sides
                .is_none()
        );
    }
}
