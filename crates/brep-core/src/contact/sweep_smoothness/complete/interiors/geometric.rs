//! Exact knot-line transport through a partition of the original tensor chart.
use crate::{Result, Surface};
use nurbs_core::{
    continuity::{
        ProjectiveSeam, inspect_projective_seam_collection_with_rational_scale_proposals,
    },
    editing::{
        exact_periodic_surface::open_exact_periodic_surface_axis,
        exact_surface_knot_insertion::tile_tensor_surface,
    },
    surface::Axis,
};
use std::borrow::Cow;

fn cuts(knots: &[f64], degree: usize, count: usize, order: usize) -> Option<Vec<f64>> {
    let [lo, hi] = [knots[degree], knots[count]];
    if knots[..=degree].iter().any(|&k| k != lo) || knots[count..].iter().any(|&k| k != hi) {
        return None;
    }
    let mut cuts = Vec::new();
    for block in knots
        .chunk_by(|a, b| a == b)
        .filter(|k| lo < k[0] && k[0] < hi)
    {
        if block.len() > degree {
            return None;
        }
        if degree < block.len() + order {
            cuts.push(block[0]);
        }
    }
    (cuts.len() <= 254).then_some(cuts)
}

pub(super) struct Report {
    pub(super) certified: bool,
    pub(super) cells: usize,
    pub(super) work: u64,
    pub(super) seams: usize,
}

pub(super) fn inspect(surface: &Surface, order: usize, cells: usize, work: u64) -> Result<Report> {
    let mut report = Report {
        certified: false,
        cells: 0,
        work: 0,
        seams: 0,
    };
    if work == 0 || cells == 0 {
        return Ok(report);
    }
    let wraps = [surface.periodic_u, surface.periodic_v];
    let mut surface = Cow::Borrowed(surface);
    for (axis, wraps) in [Axis::U, Axis::V].into_iter().zip(wraps) {
        if !wraps {
            continue;
        }
        let remaining = (work - report.work).min(cad_predicates::MAX_WORK);
        if remaining == 0 {
            return Ok(report);
        }
        let opened = open_exact_periodic_surface_axis(&surface, axis, remaining)?;
        report.work += opened.work_used;
        let Some(chart) = opened.surface else {
            return Ok(report);
        };
        surface = Cow::Owned(chart);
    }
    let (Some(u), Some(v)) = (
        cuts(
            &surface.knots_u,
            surface.degree_u,
            surface.control_points.len(),
            order,
        ),
        cuts(
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
            order,
        ),
    ) else {
        return Ok(report);
    };
    let [columns, rows] = [u.len() + 1, v.len() + 1];
    let count = rows * columns;
    let seam_count =
        rows * (u.len() + usize::from(wraps[0])) + columns * (v.len() + usize::from(wraps[1]));
    if seam_count == 0 || count > cells || count > 4096 || seam_count > 4096 {
        return Ok(report);
    }
    report.cells = count;
    let patches = if count == 1 {
        vec![surface.into_owned()]
    } else {
        let remaining = (work - report.work).min(cad_predicates::MAX_WORK);
        if remaining == 0 {
            return Ok(report);
        }
        let tiled = tile_tensor_surface(&surface, &u, &v, remaining)?;
        report.work += tiled.work_used;
        let Some(patches) = tiled.patches else {
            return Ok(report);
        };
        patches
    };

    let mut seams = Vec::with_capacity(seam_count);
    let mut join = |a, b, ab: &str, bb: &str| {
        seams.push(ProjectiveSeam {
            patches: [a, b],
            boundaries: [ab.into(), bb.into()],
            order,
            normal_scale: 1.,
        });
    };
    for row in 0..rows {
        for column in 0..columns {
            let index = row * columns + column;
            if column + 1 < columns {
                join(index, index + 1, "uMax", "uMin");
            } else if wraps[0] {
                join(index, row * columns, "uMax", "uMin");
            }
            if row + 1 < rows {
                join(index, index + columns, "vMax", "vMin");
            } else if wraps[1] {
                join(index, column, "vMax", "vMin");
            }
        }
    }
    let proof = inspect_projective_seam_collection_with_rational_scale_proposals(
        &patches,
        &seams,
        work - report.work,
    )?;
    report.work += proof.exact_work;
    report.certified = proof.certified;
    if proof.certified {
        report.seams = seams.len();
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saddle() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 0.25, 1., 1.],
            knots_v: vec![0., 0., 0.5, 1., 1.],
            control_points: [0., 1., 3.]
                .into_iter()
                .map(|x| {
                    [0., 2., 3.]
                        .into_iter()
                        .map(|y| vec![x, y, x * y / 8.])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn crossing_internal_knot_lines_keep_g2_with_exact_nondyadic_speed_ratios() {
        let source = saddle();
        let mut model = crate::cuboid([0.; 3], [3.; 3]).unwrap();
        model.faces[0].surface = source.clone();
        let raw =
            nurbs_core::analysis::surface_continuity::qualify(&source, 2, 1000, 1_000_000).unwrap();
        assert!(raw.certificate.is_none());
        let report = super::super::inspect(&model, 2, 1000, 1_000_000).unwrap();
        assert!(report.certified, "face={:?}", report.unresolved_face);
        assert_eq!(report.geometric_seams, 4);
        assert_eq!(model.faces[0].surface, source);

        for (cells, work) in [
            (report.cells - 1, report.exact_work),
            (report.cells, report.exact_work - 1),
        ] {
            let short = super::super::inspect(&model, 2, cells, work).unwrap();
            assert!(!short.certified);
            assert_eq!(short.unresolved_face, Some(0));
            assert!(short.cells <= cells && short.exact_work <= work);
        }
        model.faces[0].surface.control_points[1][1][2] += 1. / 16.;
        let kink = super::super::inspect(&model, 1, 1000, 1_000_000).unwrap();
        assert!(!kink.certified);
        assert_eq!(kink.geometric_seams, 0);
        assert_eq!(kink.unresolved_face, Some(0));
    }
    fn periodic_chart(torus: bool) -> Surface {
        let ring = [
            [1., -1.],
            [1., 0.],
            [1., 1.],
            [0., 1.],
            [-1., 1.],
            [-1., 0.],
            [-1., -1.],
            [0., -1.],
            [1., -1.],
            [1., 0.],
        ];
        let weights = [0.5, 1., 0.5, 1., 0.5, 1., 0.5, 1., 0.5, 1.];
        let periodic_knots = vec![-1., -1., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 5.];
        let (control_points, weights_v) = if torus {
            let rows = ring
                .iter()
                .map(|&[x, y]| {
                    ring.iter()
                        .map(|&[c, s]| vec![x * (4. + c), y * (4. + c), s])
                        .collect()
                })
                .collect();
            (rows, weights.to_vec())
        } else {
            let rows = ring
                .iter()
                .map(|&[x, y]| vec![vec![x, y, 0.], vec![x, y, 2.]])
                .collect();
            (rows, vec![1., 1.])
        };
        Surface {
            degree_u: 2,
            degree_v: if torus { 2 } else { 1 },
            knots_u: periodic_knots.clone(),
            knots_v: if torus {
                periodic_knots
            } else {
                vec![0., 0., 1., 1.]
            },
            control_points,
            weights: weights
                .iter()
                .map(|&u| weights_v.iter().map(|v| u * v).collect())
                .collect(),
            periodic_u: true,
            periodic_v: torus,
        }
    }

    #[test]
    fn periodic_original_face_covers_internal_knots_and_every_wrap_join() {
        for (torus, expected) in [(false, 4), (true, 32)] {
            let source = periodic_chart(torus);
            let proof = inspect(&source, 2, 1000, 16_000_000).unwrap();
            assert!(proof.certified, "torus={torus} work={}", proof.work);
            assert_eq!(proof.seams, expected);
            for (cells, work) in [(proof.cells - 1, proof.work), (proof.cells, proof.work - 1)] {
                let short = inspect(&source, 2, cells, work).unwrap();
                assert!(!short.certified);
                assert_eq!(short.seams, 0);
                assert!(short.cells <= cells && short.work <= work);
            }
            let mut model = crate::cuboid([0.; 3], [3.; 3]).unwrap();
            model.faces[0].surface = source;
            let report = super::super::inspect(&model, 2, 1000, 16_000_000).unwrap();
            assert!(report.certified);
            assert_eq!(report.geometric_seams, expected);
        }
    }

    #[test]
    fn periodic_flag_cannot_hide_a_kink_across_the_opened_period() {
        let mut source = periodic_chart(false);
        for row in [0, 8] {
            for point in &mut source.control_points[row] {
                point[0] += 0.125;
            }
        }
        let report = inspect(&source, 1, 1000, 16_000_000).unwrap();
        assert!(!report.certified);
        assert_eq!(report.seams, 0);
        assert!(report.work > 0);
    }
}
