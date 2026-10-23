//! Full-parameter contact position and tangent-plane angular qualification.
//! Opposite normals describe the same plane. Shell orientation, radius and
//! global G1 branch/topology admission are separate from this report.
use crate::{
    Result, check, curve::Curve, curve_surface_agreement as lift, distance_bounds::Interval as I,
    normal_alignment, surface::Surface,
};
pub struct Cell {
    pub interval: [f64; 2],
    pub normals: Option<normal_alignment::PairReport>,
}
pub struct Report {
    pub positions: Vec<lift::Report>,
    pub cells: Vec<Cell>,
    pub visited: usize,
    pub spans: usize,
    pub positions_proven: bool,
    pub tangent_planes_proven: bool,
}
fn uv_box(p: &Curve, t: [f64; 2], reversed: bool, hull: [[f64; 2]; 2]) -> Result<[[f64; 2]; 2]> {
    let d = p.domain();
    let fraction = if reversed {
        I::point(1.).sub(I::new(t[0], t[1])?)?
    } else {
        I::new(t[0], t[1])?
    };
    let parameter = I::point(d[0]).add(I::point(d[1]).sub(I::point(d[0]))?.mul(fraction)?)?;
    let bounds = lift::curve_bounds(p, I::new(parameter.lo.max(d[0]), parameter.hi.min(d[1]))?)?;
    // The positive-weight B-spline lies inside the original control hull.
    // Intersecting with that independent enclosure removes outward rounding
    // beyond an exact boundary without assuming a near-boundary UV snap.
    Ok(std::array::from_fn(|i| {
        [bounds[i].lo.max(hull[i][0]), bounds[i].hi.min(hull[i][1])]
    }))
}
pub fn certify(
    world: &Curve,
    surfaces: [&Surface; 2],
    pcurves: [&Curve; 2],
    reversed: [bool; 2],
    tolerance_mm: f64,
    max_sine_squared: f64,
    agreement_cells: usize,
    normal_cells: usize,
    normal_spans: usize,
) -> Result<Report> {
    world.validate()?;
    check(
        world.control_points[0].len() == 3,
        "Contact normal qualification needs a 3D world curve",
    )?;
    for side in 0..2 {
        surfaces[side].validate()?;
        pcurves[side].validate()?;
        check(
            !pcurves[side].periodic && pcurves[side].control_points[0].len() == 2,
            "Contact normal qualification needs nonperiodic 2D pcurves",
        )?;
    }
    check(
        tolerance_mm.is_finite()
            && tolerance_mm > 0.
            && max_sine_squared.is_finite()
            && (0. ..1.).contains(&max_sine_squared)
            && [agreement_cells, normal_cells, normal_spans]
                .iter()
                .all(|n| (1..=100000).contains(n)),
        "Contact qualification needs finite tolerances and bounded positive work",
    )?;
    let mut out = Report {
        positions: vec![],
        cells: vec![],
        visited: 0,
        spans: 0,
        positions_proven: true,
        tangent_planes_proven: false,
    };
    let mut used = 0;
    for side in 0..2 {
        if used == agreement_cells {
            out.positions_proven = false;
            break;
        }
        let r = lift::verify(
            world,
            pcurves[side],
            surfaces[side],
            reversed[side],
            tolerance_mm,
            agreement_cells - used,
        )?;
        used += r.cells;
        out.positions_proven &= r.status == lift::Status::WithinTolerance;
        out.positions.push(r);
    }
    let mut hulls = [[[0.; 2]; 2]; 2];
    let mut inside = true;
    for side in 0..2 {
        let s = surfaces[side];
        let domain = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        for axis in 0..2 {
            hulls[side][axis] = [
                pcurves[side]
                    .control_points
                    .iter()
                    .map(|p| p[axis])
                    .fold(f64::INFINITY, f64::min),
                pcurves[side]
                    .control_points
                    .iter()
                    .map(|p| p[axis])
                    .fold(f64::NEG_INFINITY, f64::max),
            ];
            inside &=
                hulls[side][axis][0] >= domain[axis][0] && hulls[side][axis][1] <= domain[axis][1];
        }
    }
    if !inside || !out.positions_proven {
        out.cells.push(Cell {
            interval: [0., 1.],
            normals: None,
        });
        return Ok(out);
    }
    let mut pending = vec![([0., 1.], 0usize)];
    while let Some((t, depth)) = pending.pop() {
        if out.visited == normal_cells || out.spans == normal_spans {
            out.cells.push(Cell {
                interval: t,
                normals: None,
            });
            continue;
        }
        out.visited += 1;
        let domains = [
            uv_box(pcurves[0], t, reversed[0], hulls[0])?,
            uv_box(pcurves[1], t, reversed[1], hulls[1])?,
        ];
        let r = normal_alignment::inspect_pair(
            surfaces,
            domains,
            max_sine_squared,
            normal_spans - out.spans,
        )?;
        out.spans += r.spans;
        let middle = t[0] * 0.5 + t[1] * 0.5;
        if r.aligned.is_some()
            || depth == 32
            || middle <= t[0]
            || middle >= t[1]
            || normal_cells - out.visited < pending.len() + 2
            || out.spans == normal_spans
        {
            out.cells.push(Cell {
                interval: t,
                normals: Some(r),
            });
        } else {
            pending.push(([middle, t[1]], depth + 1));
            pending.push(([t[0], middle], depth + 1));
        }
    }
    out.tangent_planes_proven = out
        .cells
        .iter()
        .all(|c| c.normals.as_ref().is_some_and(|r| r.aligned == Some(true)));
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn line(points: Vec<Vec<f64>>, d: [f64; 2]) -> Curve {
        Curve {
            degree: 1,
            knots: vec![d[0], d[0], d[1], d[1]],
            control_points: points,
            weights: vec![1.; 2],
            periodic: false,
        }
    }
    fn plane() -> Surface {
        Surface {
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
        }
    }
    #[test]
    fn rational_transition_contacts_qualify_and_normal_work_stop_stays_unresolved() {
        let patch = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![2., 2., 8., 8.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: [0., 1.]
                .iter()
                .map(|&z| vec![vec![1., 0., z], vec![1., 1., z], vec![0., 1., z]])
                .collect(),
            weights: vec![vec![1., 0.5f64.sqrt(), 1.]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        for side in [0usize, 1] {
            let mut source = plane();
            for (i, row) in source.control_points.iter_mut().enumerate() {
                for (j, p) in row.iter_mut().enumerate() {
                    *p = if side == 0 {
                        vec![1., j as f64, i as f64]
                    } else {
                        vec![j as f64, 1., i as f64]
                    };
                }
            }
            let world = line(
                if side == 0 {
                    vec![vec![1., 0., 0.], vec![1., 0., 1.]]
                } else {
                    vec![vec![0., 1., 0.], vec![0., 1., 1.]]
                },
                [2., 8.],
            );
            let pc = line(vec![vec![2., side as f64], vec![8., side as f64]], [2., 8.]);
            let uv = line(vec![vec![0., 0.], vec![1., 0.]], [10., 14.]);
            let r = certify(
                &world,
                [&patch, &source],
                [&pc, &uv],
                [false, false],
                1e-7,
                1e-10,
                10000,
                1000,
                10000,
            )
            .unwrap();
            assert!(r.positions_proven && r.tangent_planes_proven);
            let stop = certify(
                &world,
                [&patch, &source],
                [&pc, &uv],
                [false, false],
                1e-7,
                1e-10,
                10000,
                1000,
                1,
            )
            .unwrap();
            assert!(stop.positions_proven && !stop.tangent_planes_proven);
            assert_eq!(stop.spans, 1);
            assert_eq!(stop.cells[0].interval, [0., 1.]);
        }
    }
    #[test]
    fn coincident_end_normals_cannot_hide_an_interior_tangent_defect() {
        let a = plane();
        let mut b = a.clone();
        b.degree_u = 2;
        b.knots_u = vec![0., 0., 0., 1., 1., 1.];
        b.control_points = vec![
            a.control_points[0].clone(),
            vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.1]],
            a.control_points[1].clone(),
        ];
        b.weights = vec![vec![1.; 2]; 3];
        let w = line(vec![vec![0., 0., 0.], vec![1., 0., 0.]], [2., 8.]);
        let p = line(vec![vec![0., 0.], vec![1., 0.]], [10., 14.]);
        let r = certify(
            &w,
            [&a, &b],
            [&p, &p],
            [false, false],
            1e-7,
            1e-6,
            10000,
            1000,
            10000,
        )
        .unwrap();
        assert!(r.positions_proven);
        assert!(!r.tangent_planes_proven);
        assert!(
            r.cells
                .iter()
                .any(|c| c.normals.as_ref().is_some_and(|r| r.aligned == Some(false)))
        );
    }
    #[test]
    fn exact_boundary_and_reversed_pcurve_qualify_without_uv_snapping() {
        let a = plane();
        let w = line(vec![vec![0., 0., 0.], vec![1., 0., 0.]], [2., 8.]);
        let p = line(vec![vec![0., 0.], vec![1., 0.]], [10., 14.]);
        let mut q = p.clone();
        q.control_points.reverse();
        let r = certify(
            &w,
            [&a, &a],
            [&p, &q],
            [false, true],
            1e-7,
            1e-12,
            10000,
            1000,
            10000,
        )
        .unwrap();
        assert!(r.positions_proven && r.tangent_planes_proven);
        assert_eq!(r.cells.first().unwrap().interval[0], 0.);
        assert_eq!(r.cells.last().unwrap().interval[1], 1.);
        let r = certify(&w, [&a, &a], [&p, &q], [false, true], 1e-7, 1e-12, 1, 1, 1).unwrap();
        assert!(!r.positions_proven && !r.tangent_planes_proven);
        assert!(r.cells[0].normals.is_none());
    }
}
