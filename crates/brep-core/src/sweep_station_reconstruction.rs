//! Atomic retained station reconstruction. Boundary provenance and global
//! admission are separate; this operation owns source/cap correspondence.
use crate::{Model, Result, analytic, invalid};
use nurbs_core::curve::Curve;

pub struct Reconstruction {
    pub model: Option<Model>,
    pub candidate: analytic::SmoothStationWalls,
}

pub fn preserve_caps(source: &Model, candidate: &Model, closed: bool) -> Result<()> {
    source.validate()?;
    candidate.validate()?;
    if closed {
        return Ok(());
    }
    if source.faces.len() < 2 || source.faces.len() != candidate.faces.len() {
        return Err(invalid("Station smoothing changed cap ownership"));
    }
    for face in source.faces.len() - 2..source.faces.len() {
        if source.faces[face] != candidate.faces[face] {
            return Err(invalid("Station smoothing changed a filled cap"));
        }
        let cap = &source.faces[face];
        for &index in std::iter::once(&cap.outer).chain(cap.holes.iter()) {
            if source.loops[index] != candidate.loops[index] {
                return Err(invalid("Station smoothing changed cap trims"));
            }
            for coedge in &source.loops[index].coedges {
                let edge = &source.edges[coedge.edge];
                if *edge != candidate.edges[coedge.edge] {
                    return Err(invalid("Station smoothing changed a cap boundary edge"));
                }
                for &vertex in &edge.vertices {
                    if source.vertices[vertex] != candidate.vertices[vertex] {
                        return Err(invalid("Station smoothing changed a cap boundary vertex"));
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn reconstruct(
    source: &Model,
    sections: &[Vec<Vec<Curve>>],
    sharp: &[usize],
    closed: bool,
    quantum: f64,
    tolerance: f64,
    max_work: u64,
) -> Result<Reconstruction> {
    source.validate()?;
    let baseline = if closed {
        crate::periodic_section_loft(sections)?
    } else {
        crate::rational_section_loft(sections)?
    };
    if baseline != *source {
        return Err(invalid(
            "Station smoothing sections do not reproduce the certified source body",
        ));
    }
    let candidate =
        analytic::smooth_station_walls(sections, sharp, closed, quantum, tolerance, max_work)?;
    let model = match (&candidate.sides, candidate.wall_displacement_upper) {
        (Some(sides), Some(_)) => {
            let model = analytic::section_loft_surfaces(sections, sides, closed)?;
            preserve_caps(source, &model, closed)?;
            Some(model)
        }
        _ => None,
    };
    Ok(Reconstruction { model, candidate })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sections() -> Vec<Vec<Vec<Curve>>> {
        let signs = [
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
        (0..3)
            .map(|i| {
                vec![vec![Curve {
                    degree: 2,
                    knots: vec![0., 0., 0., 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1., 1., 1.],
                    control_points: signs
                        .iter()
                        .map(|&(x, y)| vec![x + if i == 1 { 1. } else { 0. }, y, i as f64 * 5.])
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
            })
            .collect()
    }
    #[test]
    fn reconstruction_owns_source_and_caps_and_refuses_partial_geometry() {
        let sections = sections();
        let source = crate::rational_section_loft(&sections).unwrap();
        let report = reconstruct(&source, &sections, &[], false, 0.125, 10., 10000).unwrap();
        let model = report.model.unwrap();
        preserve_caps(&source, &model, false).unwrap();
        let denied = reconstruct(&source, &sections, &[], false, 0.125, 0., 10000).unwrap();
        assert!(denied.model.is_none());
        let denied = reconstruct(&source, &sections, &[], false, 0.125, 10., 1).unwrap();
        assert!(denied.model.is_none());
        let mut altered = sections.clone();
        altered[1][0][0].control_points[0][0] += 0.125;
        assert!(reconstruct(&source, &altered, &[], false, 0.125, 10., 10000).is_err());
        let mut changed = model.clone();
        let cap = changed.faces.len() - 1;
        changed.faces[cap].surface.control_points[0][0][0] += 0.125;
        assert!(preserve_caps(&source, &changed, false).is_err());
    }
    #[test]
    fn periodic_polynomial_profiles_preserve_caps_and_actual_g2() {
        let section = |x: f64, z: f64| {
            vec![vec![Curve {
                degree: 2,
                knots: (0..9).map(|i| i as f64).collect(),
                control_points: [(1., 0.), (0., 1.), (-1., 0.), (0., -1.), (1., 0.), (0., 1.)]
                    .iter()
                    .map(|&(a, b)| vec![x + a, b, z])
                    .collect(),
                weights: vec![1.; 6],
                periodic: true,
            }]]
        };
        let sections = vec![section(0., 0.), section(1., 5.), section(0., 10.)];
        let original = sections.clone();
        let source = crate::rational_section_loft(&sections).unwrap();
        let result = reconstruct(&source, &sections, &[], false, 0.125, 10., 10000).unwrap();
        let model = result.model.unwrap();
        preserve_caps(&source, &model, false).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        let smoothness =
            crate::sweep_smoothness::inspect_profile(&model, &caps, 2_000_000).unwrap();
        assert!(smoothness.profile.g2_certified);
        assert!(smoothness.station.g2_certified);
        assert_eq!(sections, original);
        assert!(result.candidate.wall_displacement_upper.unwrap() > 0.);
        // Odd lattice coordinates produce exact half-grid Bezier poles.
        let mut small = sections.clone();
        for station in &mut small {
            let center_x = station[0][0].control_points[0][0] - 1.;
            for pole in &mut station[0][0].control_points {
                pole[0] = center_x + (pole[0] - center_x) * 0.125;
                pole[1] *= 0.125;
            }
        }
        let small_source = crate::rational_section_loft(&small).unwrap();
        assert!(
            reconstruct(&small_source, &small, &[], false, 0.125, 10., 10000)
                .unwrap()
                .model
                .is_some()
        );
        let work = result.candidate.work;
        let short = reconstruct(&source, &sections, &[], false, 0.125, 10., work - 1).unwrap();
        assert!(short.model.is_none());
        assert_eq!(short.candidate.work, work - 1);
        assert!(
            reconstruct(&source, &sections, &[], false, 0.125, 0., 10000)
                .unwrap()
                .model
                .is_none()
        );
        let sharp = reconstruct(&source, &sections, &[1], false, 0.125, 10., 10000)
            .unwrap()
            .model
            .unwrap();
        let sharp_report =
            crate::sweep_smoothness::inspect_profile(&sharp, &caps, 2_000_000).unwrap();
        assert!(!sharp_report.station.g1_certified);
    }
    #[test]
    fn periodic_polynomial_closed_station_seam_is_audited() {
        let section = |x: f64, y: f64| {
            vec![vec![Curve {
                degree: 2,
                knots: (0..9).map(|i| i as f64).collect(),
                control_points: [(1., 0.), (0., 1.), (-1., 0.), (0., -1.), (1., 0.), (0., 1.)]
                    .iter()
                    .map(|&(a, b)| vec![x, y + a, b])
                    .collect(),
                weights: vec![1.; 6],
                periodic: true,
            }]]
        };
        let sections = vec![
            section(0., 0.),
            section(10., 0.),
            section(10., 10.),
            section(0., 10.),
            section(0., 0.),
        ];
        let source = crate::periodic_section_loft(&sections).unwrap();
        let result = reconstruct(&source, &sections, &[], true, 0.125, 10., 10000).unwrap();
        let model = result.model.unwrap();
        let report = crate::sweep_smoothness::inspect_axis(
            &model,
            &[],
            crate::sweep_smoothness::Axis::Station,
            2_000_000,
        )
        .unwrap();
        assert!(report.g2_certified);
        assert_eq!(report.edge_ids.len(), 16);
        let sharp = reconstruct(&source, &sections, &[0], true, 0.125, 10., 10000)
            .unwrap()
            .model
            .unwrap();
        let sharp_report = crate::sweep_smoothness::inspect_axis(
            &sharp,
            &[],
            crate::sweep_smoothness::Axis::Station,
            2_000_000,
        )
        .unwrap();
        assert!(!sharp_report.g1_certified);
        let mut rational = sections.clone();
        for section in &mut rational {
            section[0][0].weights[1] = 0.5;
            section[0][0].weights[5] = 0.5;
        }
        let rational_source = crate::periodic_section_loft(&rational).unwrap();
        let denied =
            reconstruct(&rational_source, &rational, &[], true, 0.125, 10., 10000).unwrap();
        assert!(denied.model.is_none());
        assert_eq!(denied.candidate.reason, "shared-station-jets-unproved");
    }
    #[test]
    fn general_multisegment_hollow_profiles_preserve_caps_and_station_g2() {
        let ring = |r: f64, reverse: bool, x: f64, z: f64| {
            let mut points = vec![[-r, -r], [r, -r], [r, r], [-r, r], [-r, -r]];
            if reverse {
                points.reverse();
            }
            points
                .windows(2)
                .map(|p| Curve {
                    degree: 1,
                    knots: vec![0., 0., 1., 1.],
                    control_points: p.iter().map(|v| vec![v[0] + x, v[1], z]).collect(),
                    weights: vec![1.; 2],
                    periodic: false,
                })
                .collect::<Vec<_>>()
        };
        let sections = [(0., 0.), (0.5, 5.), (0., 10.)]
            .iter()
            .map(|&(x, z)| vec![ring(2., false, x, z), ring(0.5, true, x, z)])
            .collect::<Vec<_>>();
        let original = sections.clone();
        let source = crate::rational_section_loft(&sections).unwrap();
        let result = reconstruct(&source, &sections, &[], false, 0.125, 1., 100000).unwrap();
        assert_eq!(
            result.candidate.reason,
            "bounded-general-quintic-station-jets"
        );
        let model = result.model.unwrap();
        preserve_caps(&source, &model, false).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        assert!(caps.iter().all(|&f| model.faces[f].holes.len() == 1));
        let report = crate::sweep_smoothness::inspect_profile(&model, &caps, 2_000_000).unwrap();
        assert!(report.station.g2_certified);
        assert!(!report.profile.g1_certified, "Polygon corners must stay C0");
        assert_eq!(sections, original);
        assert!(result.candidate.wall_displacement_upper.unwrap() > 0.);
        let short = reconstruct(
            &source,
            &sections,
            &[],
            false,
            0.125,
            1.,
            result.candidate.work - 1,
        )
        .unwrap();
        assert!(short.model.is_none());
        assert!(short.candidate.work <= result.candidate.work - 1);
        let zero = reconstruct(&source, &sections, &[], false, 0.125, 1., 0).unwrap();
        assert!(zero.model.is_none());
        let tight = reconstruct(&source, &sections, &[], false, 0.125, 0., 100000).unwrap();
        assert!(tight.model.is_none());
        let sharp = reconstruct(&source, &sections, &[1], false, 0.125, 1., 100000)
            .unwrap()
            .model
            .unwrap();
        assert!(
            !crate::sweep_smoothness::inspect_profile(&sharp, &caps, 2_000_000)
                .unwrap()
                .station
                .g1_certified
        );
        let mut different_weights = sections.clone();
        different_weights[1][0][0].weights[1] = 2.;
        assert!(crate::rational_section_loft(&different_weights).is_err());
        let denied = crate::analytic::smooth_station_walls(
            &different_weights,
            &[],
            false,
            0.125,
            1.,
            100000,
        )
        .unwrap();
        assert!(denied.sides.is_none());
        assert_eq!(denied.reason, "incompatible-section-basis");
    }

    #[test]
    fn general_periodic_nonuniform_rational_profile_has_actual_station_g2() {
        let profile = |z: f64| Curve {
            degree: 2,
            knots: (0..9).map(|i| i as f64).collect(),
            control_points: vec![
                vec![1., 0., z],
                vec![0., 1., z],
                vec![-1., 0., z],
                vec![0., -1., z],
                vec![1., 0., z],
                vec![0., 1., z],
            ],
            weights: vec![1., 0.5, 1., 1., 1., 0.5],
            periodic: true,
        };
        let sections = [0., 5., 10.]
            .into_iter()
            .map(|z| vec![vec![profile(z)]])
            .collect::<Vec<_>>();
        let source = crate::rational_section_loft(&sections).unwrap();
        let result = reconstruct(&source, &sections, &[], false, 0.125, 1., 100000).unwrap();
        let model = result.model.unwrap();
        preserve_caps(&source, &model, false).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        assert!(
            crate::sweep_smoothness::inspect_profile(&model, &caps, 2_000_000)
                .unwrap()
                .station
                .g2_certified
        );
        assert_eq!(
            result.candidate.reason,
            "bounded-general-quintic-station-jets"
        );
        let short = reconstruct(
            &source,
            &sections,
            &[],
            false,
            0.125,
            1.,
            result.candidate.work - 1,
        )
        .unwrap();
        assert!(short.model.is_none());
    }

    #[test]
    fn rational_profile_smoothness_requires_actual_retained_jets() {
        let profile = |radius: f64, z: f64| Curve {
            degree: 2,
            knots: (0..9).map(|i| i as f64).collect(),
            control_points: [
                [radius, 0., z],
                [0., radius, z],
                [-radius, 0., z],
                [0., -radius, z],
                [radius, 0., z],
                [0., radius, z],
            ]
            .map(Vec::from)
            .to_vec(),
            weights: vec![1., 0.5, 1., 1., 1., 0.5],
            periodic: true,
        };
        for radius in [1., 3.] {
            let sections = [0., 5., 10.]
                .into_iter()
                .map(|z| vec![vec![profile(radius, z)]])
                .collect::<Vec<_>>();
            let source = crate::rational_section_loft(&sections).unwrap();
            let report = crate::sweep_smoothness::inspect_profile(
                &source,
                &[source.faces.len() - 2, source.faces.len() - 1],
                2_000_000,
            )
            .unwrap();
            assert_eq!(report.profile.g1_certified, radius == 3., "radius={radius}");
        }
    }
}
