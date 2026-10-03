//! Complete original-face coverage for minimum aligned interior material chords.
//! Curved same-face pairs retain a zero lower bound until a complete intrinsic
//! exclusion is available. Sampling supplies an upper witness, never coverage.
use crate::{Error, Model, Result, material_chord, material_wall, shell_distance};
use cad_predicates::Sign;
use nurbs_core::surface::Surface;

pub struct Limits {
    pub wall: material_wall::Limits,
    pub pairs: usize,
    pub plane_controls: usize,
    pub normal_spans: usize,
}
pub struct Pair {
    pub faces: [usize; 2],
    pub lower_bound_mm: Option<f64>,
    pub reason: &'static str,
}
pub struct Report {
    pub candidate: material_chord::NormalReport,
    pub pairs: Vec<Pair>,
    pub total_pairs: usize,
    pub enumeration_complete: bool,
    pub lower_bound_mm: f64,
    pub interval_mm: Option<[f64; 2]>,
    pub converged: bool,
    pub reason: &'static str,
    pub plane_controls: usize,
    pub normal_spans: usize,
    pub cells: usize,
    pub domain_cells: usize,
}
type Plane = [[f64; 3]; 3];
fn plane(surface: &Surface, remaining: usize) -> (Option<Plane>, usize) {
    let count = surface.control_points.iter().map(Vec::len).sum::<usize>();
    if count > remaining {
        return (None, 0);
    }
    let points = surface.control_points.iter().flatten().collect::<Vec<_>>();
    let a = points[0];
    let Some(b) = points.iter().copied().find(|p| *p != a) else {
        return (None, count);
    };
    let Some(c) = points.iter().copied().find(|p| {
        [[0, 1], [0, 2], [1, 2]].into_iter().any(|axes| {
            crate::shared_boundary::orient(&[a, b, p], Some(axes))
                .is_some_and(|sign| sign != Sign::Zero)
        })
    }) else {
        return (None, count);
    };
    if !points
        .iter()
        .all(|p| crate::shared_boundary::orient(&[a, b, c, p], None) == Some(Sign::Zero))
    {
        return (None, count);
    }
    (
        Some([
            a.as_slice().try_into().unwrap(),
            b.as_slice().try_into().unwrap(),
            c.as_slice().try_into().unwrap(),
        ]),
        count,
    )
}
fn same_plane(a: Plane, b: Plane) -> bool {
    b.iter().all(|p| {
        crate::shared_boundary::orient(&[&a[0], &a[1], &a[2], p], None) == Some(Sign::Zero)
    })
}
fn domain(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
// If a line is within alpha of each unoriented endpoint normal, the normals
// must be within 2*alpha. Use an outward upper bound, not a rounded threshold.
fn necessary_normal_sine(sine_squared: f64) -> Result<Option<f64>> {
    if sine_squared >= 0.5 {
        return Ok(None);
    }
    let upper = ((4. * sine_squared).next_up() * (1. - sine_squared).next_up()).next_up();
    Ok((upper < 1.).then_some(upper))
}
/// Enumerates distinct and same-face pairs of every original face. A pair with
/// no refined proof still contributes zero. Missing pairs also force zero and
/// prohibit convergence. No adjacency, trimming hole or body is skipped.
pub fn inspect(
    model: &Model,
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance_mm: f64,
    tolerance_uv: f64,
    max_sine_squared: f64,
    limits: Limits,
) -> Result<Report> {
    model.validate()?;
    if !tolerance_mm.is_finite()
        || tolerance_mm <= 0.
        || !max_sine_squared.is_finite()
        || !(0. ..1.).contains(&max_sine_squared)
        || !(1..=100000).contains(&limits.pairs)
        || !(1..=1000000).contains(&limits.plane_controls)
        || !(1..=100000).contains(&limits.normal_spans)
        || !(1..=1000000).contains(&limits.wall.distance_cells)
        || !(1..=8000000).contains(&limits.wall.distance_domain_cells)
    {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Whole-wall coverage requires finite tolerances and bounded positive work",
        ));
    }
    let candidate = material_chord::inspect_with_normals(
        model,
        origin,
        direction,
        tolerance_uv,
        limits.wall.material,
        max_sine_squared,
        limits.wall.normal_spans,
    )?;
    let count = model.faces.len();
    let total_pairs = count
        .checked_mul(count + 1)
        .and_then(|n| n.checked_div(2))
        .ok_or_else(|| Error::new("BREP_RESOURCE_LIMIT", "Whole-wall face pair count overflow"))?;
    let mut used_controls = 0;
    let planes = model
        .faces
        .iter()
        .map(|f| {
            let (p, used) = plane(&f.surface, limits.plane_controls - used_controls);
            used_controls += used;
            p
        })
        .collect::<Vec<_>>();
    let necessary = necessary_normal_sine(max_sine_squared)?;
    let mut report = Report {
        candidate,
        pairs: Vec::with_capacity(total_pairs.min(limits.pairs)),
        total_pairs,
        enumeration_complete: false,
        lower_bound_mm: 0.,
        interval_mm: None,
        converged: false,
        reason: "candidate-unproven",
        plane_controls: used_controls,
        normal_spans: 0,
        cells: 0,
        domain_cells: 0,
    };
    let mut lower = f64::INFINITY;
    'pairs: for a in 0..count {
        for b in a..count {
            if report.pairs.len() == limits.pairs {
                break 'pairs;
            }
            let mut pair = Pair {
                faces: [a, b],
                lower_bound_mm: Some(0.),
                reason: "distance-work-limit",
            };
            if a == b {
                if planes[a].is_some() {
                    pair.lower_bound_mm = None;
                    pair.reason = "planar-self-excluded";
                } else {
                    pair.reason = "self-pair-unresolved";
                }
            } else if matches!((planes[a],planes[b]),(Some(x),Some(y)) if same_plane(x,y)) {
                pair.lower_bound_mm = None;
                pair.reason = "coplanar-chord-excluded";
            } else {
                let remaining = limits.normal_spans - report.normal_spans;
                if let Some(threshold) = necessary.filter(|_| remaining > 0) {
                    let normal = nurbs_core::normal_alignment::inspect_pair(
                        [&model.faces[a].surface, &model.faces[b].surface],
                        [
                            domain(&model.faces[a].surface),
                            domain(&model.faces[b].surface),
                        ],
                        threshold,
                        remaining,
                    )?;
                    report.normal_spans += normal.spans;
                    if normal.aligned == Some(false) {
                        pair.lower_bound_mm = None;
                        pair.reason = "endpoint-normal-excluded";
                    }
                }
                if pair.lower_bound_mm.is_some() {
                    let cells = limits.wall.distance_cells - report.cells;
                    let domains = limits.wall.distance_domain_cells - report.domain_cells;
                    if cells > 0 && domains > 0 {
                        let d = shell_distance::distance_between_face_sets(
                            model,
                            &[a],
                            model,
                            &[b],
                            tolerance_mm,
                            tolerance_uv,
                            cells,
                            domains,
                        )?;
                        report.cells += d.cells;
                        report.domain_cells += d.domain_cells;
                        pair.lower_bound_mm = Some(d.lower_bound_mm);
                        pair.reason = "complete-face-distance-bound";
                    }
                }
            }
            if let Some(value) = pair.lower_bound_mm {
                lower = lower.min(value);
            }
            report.pairs.push(pair);
        }
    }
    report.enumeration_complete = report.pairs.len() == total_pairs;
    if !report.enumeration_complete {
        lower = 0.;
    }
    report.lower_bound_mm = if lower.is_finite() { lower } else { 0. };
    report.reason = if !report.candidate.chord.proven {
        "material-chord-unproven"
    } else if report.candidate.aligned != Some(true) {
        "candidate-normal-unproven"
    } else {
        "whole-wall-bounds"
    };
    if report.reason == "whole-wall-bounds" {
        let upper = report.candidate.chord.length_interval_mm.unwrap()[1];
        if !lower.is_finite() || report.lower_bound_mm > upper {
            return Err(Error::new(
                "BREP_INVALID_INPUT",
                "Whole-wall coverage contradicts its material witness",
            ));
        }
        report.interval_mm = Some([report.lower_bound_mm, upper]);
        report.converged = report.enumeration_complete
            && (upper - report.lower_bound_mm).next_up() <= tolerance_mm;
        if !report.enumeration_complete {
            report.reason = "face-pair-limit";
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> Limits {
        Limits {
            wall: material_wall::Limits {
                material: crate::material_segment::tests::limits(),
                distance_cells: 10000,
                distance_domain_cells: 1000000,
                normal_spans: 100,
            },
            pairs: 10000,
            plane_controls: 10000,
            normal_spans: 10000,
        }
    }
    #[test]
    fn all_box_faces_and_diagonal_pairs_qualify_the_true_smallest_width() {
        let model = crate::cuboid([0.; 3], [10., 20., 30.]).unwrap();
        let before = format!("{model:?}");
        let r = inspect(
            &model,
            [-2., 10., 15.],
            [14., 0., 0.],
            1e-5,
            1e-7,
            1e-6,
            limits(),
        )
        .unwrap();
        assert_eq!(r.total_pairs, 21);
        assert_eq!(r.pairs.len(), 21);
        assert_eq!(
            r.pairs
                .iter()
                .filter(|p| p.reason == "planar-self-excluded")
                .count(),
            6
        );
        assert!(r.enumeration_complete && r.converged);
        let d = r.interval_mm.unwrap();
        assert!(d[0] <= 10. && d[1] >= 10. && d[1] - d[0] <= 1e-5);
        assert_eq!(format!("{model:?}"), before);
    }
    #[test]
    fn missing_pairs_and_plane_work_never_become_whole_wall_success() {
        let model = crate::cuboid([0.; 3], [10., 20., 30.]).unwrap();
        let mut limited = limits();
        limited.pairs = 1;
        let r = inspect(
            &model,
            [-2., 10., 15.],
            [14., 0., 0.],
            100.,
            1e-7,
            1e-6,
            limited,
        )
        .unwrap();
        assert!(!r.enumeration_complete && !r.converged && r.lower_bound_mm == 0.);
        assert_eq!(r.reason, "face-pair-limit");
        let mut limited = limits();
        limited.plane_controls = 1;
        let r = inspect(
            &model,
            [-2., 10., 15.],
            [14., 0., 0.],
            1e-5,
            1e-7,
            1e-6,
            limited,
        )
        .unwrap();
        assert!(r.enumeration_complete && !r.converged && r.lower_bound_mm == 0.);
        assert_eq!(
            r.pairs
                .iter()
                .filter(|p| p.reason == "self-pair-unresolved")
                .count(),
            6
        );
    }
    #[test]
    fn a_long_chord_does_not_replace_the_global_minimum() {
        let model = crate::cuboid([0.; 3], [10., 20., 30.]).unwrap();
        let r = inspect(
            &model,
            [5., -2., 15.],
            [0., 24., 0.],
            1e-5,
            1e-7,
            1e-6,
            limits(),
        )
        .unwrap();
        let d = r.interval_mm.unwrap();
        assert!(r.enumeration_complete && !r.converged);
        assert!(d[0] > 9.99 && d[0] <= 10. && d[1] >= 20. && d[1] < 20.01);
    }
    #[test]
    fn all_open_enclosure_walls_and_floor_contribute_to_the_minimum() {
        let model = crate::operations::boolean(
            &crate::cuboid([0.; 3], [40., 30., 20.]).unwrap(),
            &crate::cuboid([1.4, 1.4, 2.], [38.6, 28.6, 22.]).unwrap(),
            "difference",
        )
        .unwrap();
        let r = inspect(
            &model,
            [-2., 15., 10.],
            [3.5, 0., 0.],
            1e-5,
            1e-7,
            1e-6,
            limits(),
        )
        .unwrap();
        assert_eq!(
            r.total_pairs,
            model.faces.len() * (model.faces.len() + 1) / 2
        );
        let d = r.interval_mm.unwrap();
        assert!(r.enumeration_complete && r.converged);
        assert!(d[0] <= 1.4 && d[1] >= 1.4 && d[1] - d[0] <= 1e-5);
    }
    #[test]
    fn warped_controls_and_unfinished_plane_checks_cannot_exclude_a_self_pair() {
        let model = crate::cuboid([0.; 3], [10., 20., 30.]).unwrap();
        let source = &model.faces[0].surface;
        assert!(plane(source, 100).0.is_some());
        assert!(plane(source, 1).0.is_none());
        let axis = (0..3)
            .find(|&k| {
                source
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[k] == source.control_points[0][0][k])
            })
            .unwrap();
        let mut warped = source.clone();
        warped.control_points[1][1][axis] += 1e-12;
        assert!(plane(&warped, 100).0.is_none());
        let mut rational = source.clone();
        rational.weights[0][0] *= 0.25;
        assert!(plane(&rational, 100).0.is_some());
    }
    #[test]
    fn curved_self_pairs_remain_visible_even_with_a_valid_radial_witness() {
        let model = crate::cylinder(5., 6.).unwrap();
        let r = inspect(
            &model,
            [-7., -7., 3.],
            [14., 14., 0.],
            1e-5,
            1e-7,
            1e-6,
            limits(),
        )
        .unwrap();
        assert!(r.enumeration_complete);
        assert!(
            r.pairs
                .iter()
                .any(|p| p.faces[0] == p.faces[1] && p.reason == "self-pair-unresolved")
        );
        assert_eq!(r.lower_bound_mm, 0.);
        assert!(!r.converged);
        assert!(r.candidate.chord.proven && r.candidate.aligned == Some(true));
        assert!(r.interval_mm.unwrap()[1] >= 10.);
    }
}
