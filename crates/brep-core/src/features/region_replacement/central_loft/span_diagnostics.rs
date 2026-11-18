//! Finite original-span diagnostics; never whole-seam G2 admission.
use super::*;
use nurbs_core::{
    interval_eval::{self, Interval as I},
    surface_shape_operator as shape,
};
fn point_shape(s: &Surface, pc: &Curve, t: f64) -> shape::Report {
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    assert!(
        pc.control_points
            .iter()
            .all(|p| (0..2).all(|k| domain[k][0] <= p[k] && p[k] <= domain[k][1]))
    );
    let d = pc.domain();
    let natural = I::point(d[0])
        .add(
            I::point(t)
                .mul(I::point(d[1]).sub(I::point(d[0])).unwrap())
                .unwrap(),
        )
        .unwrap();
    let uv = interval_eval::evaluate_interval(pc, natural.intersect(d[0], d[1]).unwrap()).unwrap();
    let query: [[f64; 2]; 2] = std::array::from_fn(|k| {
        let x = uv[k].intersect(domain[k][0], domain[k][1]).unwrap();
        [x.lo, x.hi]
    });
    let mut output: Option<shape::Report> = None;
    for u in s.degree_u..s.control_points.len() {
        for v in s.degree_v..s.control_points[0].len() {
            let full = [
                [s.knots_u[u], s.knots_u[u + 1]],
                [s.knots_v[v], s.knots_v[v + 1]],
            ];
            let clipped =
                std::array::from_fn(|k| [query[k][0].max(full[k][0]), query[k][1].min(full[k][1])]);
            if (0..2).any(|k| full[k][0] == full[k][1] || clipped[k][0] > clipped[k][1]) {
                continue;
            }
            let current = shape::span_rectangle_bounds(s, [u, v], clipped)
                .unwrap()
                .shape;
            assert_eq!(
                current.status,
                nurbs_core::surface_differential::Status::Available,
                "original spans={u},{v} uv={query:?}"
            );
            if let Some(out) = &mut output {
                let merge = |a: &mut [f64; 2], b: [f64; 2]| {
                    a[0] = a[0].min(b[0]);
                    a[1] = a[1].max(b[1]);
                };
                for k in 0..3 {
                    merge(
                        &mut out.normal.as_mut().unwrap()[k],
                        current.normal.unwrap()[k],
                    );
                    for j in 0..3 {
                        merge(
                            &mut out.tensor.as_mut().unwrap()[k][j],
                            current.tensor.unwrap()[k][j],
                        );
                    }
                }
            } else {
                output = Some(current);
            }
        }
    }
    output.unwrap()
}
fn difference(a: &[[f64; 2]], b: &[[f64; 2]], opposite: bool) -> [f64; 2] {
    let mut lower: f64 = 0.;
    let mut upper: f64 = 0.;
    for (a, b) in a.iter().zip(b) {
        let b = if opposite { [-b[1], -b[0]] } else { *b };
        let delta = I::new(a[0], a[1])
            .unwrap()
            .sub(I::new(b[0], b[1]).unwrap())
            .unwrap();
        lower = lower.max(if delta.lo > 0. {
            delta.lo
        } else if delta.hi < 0. {
            -delta.hi
        } else {
            0.
        });
        let radius = delta.lo.abs().max(delta.hi.abs());
        upper = (upper + (radius * radius).next_up()).next_up();
    }
    [lower, upper.sqrt().next_up()]
}
pub(super) fn inspect(candidate: &CentralLoftCandidate<'_, '_, '_>) {
    for i in 0..6 {
        let mut worst = [[[0_f64; 2]; 2]; 2];
        for t in [0.125, 0.375, 0.625, 0.875] {
            let a = point_shape(&candidate.originals[i].0, &candidate.originals[i].1, t);
            let b = point_shape(&candidate.surface, &candidate.pcurves[i], t);
            for sign in 0..2 {
                let normal = difference(&a.normal.unwrap(), &b.normal.unwrap(), sign == 1);
                let (ta, tb) = (a.tensor.unwrap(), b.tensor.unwrap());
                let curvature = difference(
                    &ta.into_iter().flatten().collect::<Vec<_>>(),
                    &tb.into_iter().flatten().collect::<Vec<_>>(),
                    sign == 1,
                );
                for (k, d) in [normal, curvature].into_iter().enumerate() {
                    for j in 0..2 {
                        worst[sign][k][j] = worst[sign][k][j].max(d[j]);
                    }
                }
            }
        }
        eprintln!("central original span samples {i}: coherent normal/tensor bounds={worst:?}");
        assert!(
            worst
                .iter()
                .any(|b| b[0][1] <= candidate.options.tolerances[1]
                    && b[1][1] <= candidate.options.tolerances[2]),
            "Finite point G2 diagnostics failed; no whole-seam admission follows from passing"
        );
    }
}
