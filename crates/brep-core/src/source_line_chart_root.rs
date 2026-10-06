//! Exact affine chart identity plus a fresh unique-root union query.
//! The caller separately proves both original main curves share a world carrier.
use crate::{source_boundary_fragment::Role, source_contact_point::SourcePoint};
use nurbs_core::{Error, Result, curve::Curve, interval_eval::Interval as I};
pub fn same(
    points: [(&SourcePoint, Role); 2],
    reversed: [bool; 2],
    max_work: u64,
) -> Result<(bool, u64, usize)> {
    let curves = points.map(|(p, role)| {
        if role == Role::Boundary {
            (p.boundary(), p.contact(), 0)
        } else {
            (p.contact(), p.boundary(), 1)
        }
    });
    let bezier = |c: &Curve| {
        let d = c.domain();
        c.control_points.len() == c.degree + 1
            && c.degree >= 1
            && c.degree <= 32
            && c.knots[..=c.degree].iter().all(|&k| k == d[0])
            && c.knots[c.control_points.len()..].iter().all(|&k| k == d[1])
    };
    if max_work == 0
        || curves
            .iter()
            .any(|(main, other, _)| main.degree != 1 || !bezier(main) || !bezier(other))
        || curves[0].1.degree != curves[1].1.degree
    {
        return Ok((false, 0, 0));
    }
    use cad_predicates::{
        AuthoredScalar, Limits, ParameterIdentity, PredicateContext, SourceArena, ToleranceContext,
    };
    let mut values = Vec::new();
    for (main, _, _) in curves {
        for (p, w) in main.control_points.iter().zip(&main.weights) {
            values.extend([p[0], p[1], *w]);
        }
    }
    for (_, other, _) in curves {
        for (p, w) in other.control_points.iter().zip(&other.weights) {
            values.extend([p[0], p[1], *w]);
        }
    }
    let arena = SourceArena::authored(
        "source-line-chart-root",
        1,
        values
            .iter()
            .map(|v: &f64| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| {
        Error::new(
            "BREP_SOURCE_SHARED_EDGE",
            "Invalid original line chart source",
        )
    })?;
    let main = std::array::from_fn(|side| {
        std::array::from_fn(|i| {
            std::array::from_fn(|axis| arena.leaf(6 * side + 3 * i + axis).unwrap())
        })
    });
    let count = curves[0].1.control_points.len();
    let cutters: [Vec<_>; 2] = std::array::from_fn(|side| {
        (0..count)
            .map(|i| {
                std::array::from_fn(|axis| arena.leaf(12 + 3 * (side * count + i) + axis).unwrap())
            })
            .collect()
    });
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: max_work.min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let proof = cad_predicates::line_chart_cutter_identity(
        &mut ctx,
        main,
        [&cutters[0], &cutters[1]],
        reversed,
    )
    .map_err(|_| {
        Error::new(
            "BREP_SOURCE_SHARED_EDGE",
            "Invalid original chart identity request",
        )
    })?;
    if proof.outcome != ParameterIdentity::Equal {
        return Ok((false, proof.work_used, 0));
    }
    let normalize = |range: [f64; 2], domain: [f64; 2]| -> Result<I> {
        I::new(range[0], range[1])?
            .sub(I::point(domain[0]))?
            .div(I::point(domain[1]).sub(I::point(domain[0]))?)?
            .intersect(0., 1.)
    };
    let mut ranges = [[[0.; 2]; 2]; 2];
    for side in 0..2 {
        for axis in 0..2 {
            let index = if axis == 0 {
                curves[side].2
            } else {
                1 - curves[side].2
            };
            let domain = if axis == 0 {
                curves[side].0.domain()
            } else {
                curves[side].1.domain()
            };
            let mut q = normalize(points[side].0.selector()[index], domain)?;
            if axis == 0 && reversed[side] {
                q = I::point(1.).sub(q)?;
            }
            if axis == 0 && reversed[0] {
                q = I::point(1.).sub(q)?;
            }
            let target = if axis == 0 {
                curves[0].0.domain()
            } else {
                curves[0].1.domain()
            };
            let t = I::point(target[0])
                .add(q.mul(I::point(target[1]).sub(I::point(target[0]))?)?)?
                .intersect(target[0], target[1])?;
            ranges[side][axis] = [t.lo, t.hi];
        }
    }
    let union = std::array::from_fn(|axis| {
        [
            ranges[0][axis][0].min(ranges[1][axis][0]),
            ranges[0][axis][1].max(ranges[1][axis][1]),
        ]
    });
    let unique = nurbs_core::uv_curve_crossings::certify_box(curves[0].0, curves[0].1, union)?
        .state
        == nurbs_core::uv_curve_crossings::State::Unique;
    Ok((unique, proof.work_used, 1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nurbs_core::surface::Surface;
    #[test]
    fn identical_equations_do_not_merge_different_selected_roots() {
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            periodic_u: false,
            periodic_v: false,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![-1., -1., 1., 1.],
            control_points: vec![
                vec![vec![0., -1., 0.], vec![0., 1., 0.]],
                vec![vec![1., -1., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
        };
        let main = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let cutter = Curve {
            degree: 2,
            periodic: false,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.1875], vec![0.5, -0.3125], vec![1., 0.1875]],
            weights: vec![1.; 3],
        };
        let point = |range| {
            let report =
                crate::source_contact_point::qualify(&surface, &main, &cutter, [range; 2], 100)
                    .unwrap();
            report.point.expect(report.reason)
        };
        let a = point([0.24, 0.26]);
        let b = point([0.74, 0.76]);
        let same_a = point([0.249, 0.251]);
        let different = same(
            [(&a, Role::Boundary), (&b, Role::Boundary)],
            [false, false],
            100000,
        )
        .unwrap();
        assert!(!different.0 && different.1 > 0 && different.2 == 1);
        assert!(
            same(
                [(&a, Role::Boundary), (&same_a, Role::Boundary)],
                [false, false],
                100000
            )
            .unwrap()
            .0
        );
        assert!(
            !same(
                [(&a, Role::Boundary), (&same_a, Role::Boundary)],
                [false, false],
                1
            )
            .unwrap()
            .0
        );

        let mut reflected_main = main.clone();
        reflected_main.control_points.reverse();
        reflected_main.knots = vec![2., 2., 4., 4.];
        let mut reflected_cutter = cutter.clone();
        for p in &mut reflected_cutter.control_points {
            p[1] = -p[1];
        }
        reflected_cutter.knots = vec![10., 10., 10., 20., 20., 20.];
        let report = crate::source_contact_point::qualify(
            &surface,
            &reflected_cutter,
            &reflected_main,
            [[12.4, 12.6], [3.48, 3.52]],
            100,
        )
        .unwrap();
        let contact = report.point.expect(report.reason);
        assert!(
            same(
                [(&a, Role::Boundary), (&contact, Role::Contact)],
                [false, true],
                100000
            )
            .unwrap()
            .0
        );
    }
}
