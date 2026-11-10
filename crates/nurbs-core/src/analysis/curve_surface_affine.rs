//! Exact formal agreement with a subinterval of an unchanged canonical curve.
//! Chart membership of the actual pcurve restriction remains a separate proof.
use crate::{
    curve::Curve, curve_surface_agreement::AlgebraicCompositionIdentity, surface::Surface, Error,
    Result,
};
use cad_predicates::{AuthoredScalar, Limits, PredicateContext, SourceArena, ToleranceContext};
/// `range` gives two exact normalized numerator/denominator pairs. Both
/// endpoints must be in [0,1] and distinct; either traversal direction is valid.
pub fn verify_algebraic(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    range: [[f64; 2]; 2],
    max_work: u64,
) -> Result<Option<AlgebraicCompositionIdentity>> {
    c.validate()?;
    p.validate()?;
    s.validate()?;
    if c.control_points[0].len() != 3
        || p.control_points[0].len() != 2
        || range.iter().flatten().any(|v| !v.is_finite())
        || max_work > cad_predicates::MAX_WORK
    {
        return Err(Error::new(
            "NURBS_INVALID_INPUT",
            "Invalid original affine composition input",
        ));
    }
    let bezier = |k: &[f64], d: usize, n: usize| {
        n == d + 1 && k[..=d].iter().all(|&x| x == k[d]) && k[n..].iter().all(|&x| x == k[n])
    };
    if c.periodic
        || p.periodic
        || s.periodic_u
        || s.periodic_v
        || c.degree > 32
        || p.degree > 8
        || s.degree_u > 8
        || s.degree_v > 8
        || !bezier(&s.knots_u, s.degree_u, s.control_points.len())
        || !bezier(&s.knots_v, s.degree_v, s.control_points[0].len())
    {
        return Ok(None);
    }
    // Non-Bezier carriers are partitioned by the native exact predicate.
    // No floating-point candidate or rounded split endpoint is authoritative.
    let p_piecewise=!bezier(&p.knots,p.degree,p.control_points.len());
    let piecewise=p_piecewise || !bezier(&c.knots,c.degree,c.control_points.len());
    let mut values = Vec::new();
    for (points, weights) in [
        (&c.control_points, &c.weights),
        (&p.control_points, &p.weights),
    ] {
        for (point, w) in points.iter().zip(weights) {
            values.extend(point.iter().copied());
            values.push(*w);
        }
    }
    for (row, weights) in s.control_points.iter().zip(&s.weights) {
        for (point, w) in row.iter().zip(weights) {
            values.extend(point.iter().copied());
            values.push(*w);
        }
    }
    values.extend([
        s.knots_u[s.degree_u],
        s.knots_u[s.control_points.len()],
        s.knots_v[s.degree_v],
        s.knots_v[s.control_points[0].len()],
    ]);
    values.extend(range.iter().flatten().copied());
    if piecewise {values.extend(c.knots.iter().copied());}
    if p_piecewise {values.extend(p.knots.iter().copied());}
    let source = SourceArena::authored(
        "original-affine-composition",
        1,
        values
            .into_iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| Error::new("NURBS_INVALID_INPUT", "Invalid affine composition source"))?;
    let mut index = 0;
    let mut leaf = || {
        let r = source.leaf(index).unwrap();
        index += 1;
        r
    };
    let cc: Vec<_> = c
        .control_points
        .iter()
        .map(|_| std::array::from_fn(|_| leaf()))
        .collect();
    let pp: Vec<_> = p
        .control_points
        .iter()
        .map(|_| std::array::from_fn(|_| leaf()))
        .collect();
    let ss: Vec<Vec<_>> = s
        .control_points
        .iter()
        .map(|row| {
            row.iter()
                .map(|_| std::array::from_fn(|_| leaf()))
                .collect()
        })
        .collect();
    let domain = std::array::from_fn(|_| std::array::from_fn(|_| leaf()));
    let range = std::array::from_fn(|_| std::array::from_fn(|_| leaf()));
    let knots=if piecewise {(0..c.knots.len()).map(|_|leaf()).collect::<Vec<_>>()}else{Vec::new()};
    let pknots=if p_piecewise {(0..p.knots.len()).map(|_|leaf()).collect::<Vec<_>>()}else{Vec::new()};
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work,
            ..Limits::default()
        },
        None,
    );
    let r = if p_piecewise {
        cad_predicates::rational_bspline_pair_composition_piecewise_identity(&mut ctx,&cc,&knots,c.degree,&pp,&pknots,p.degree,&ss,domain,range)
    }else if piecewise {
        cad_predicates::rational_bspline_composition_piecewise_identity(&mut ctx,&cc,&knots,c.degree,&pp,&ss,domain,range)
    }else{cad_predicates::rational_bezier_composition_affine_identity(
        &mut ctx, &cc, &pp, &ss, domain, range,
    )}
    .map_err(|_| {
        Error::new(
            "NURBS_INVALID_INPUT",
            "Invalid affine composition identity request",
        )
    })?;
    Ok(Some(AlgebraicCompositionIdentity {
        outcome: r.outcome,
        work_used: r.work_used,
        context: r.context,
    }))
}
/// Exact rational linear reparameterization of an original Bezier carrier.
/// Positive map weights and distinct range endpoints are required. Piecewise
/// curves return None until their projective common partition is supported.
pub fn verify_projective_algebraic(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    range: [[f64; 2]; 2],
    map_weights: [f64; 2],
    max_work: u64,
) -> Result<Option<AlgebraicCompositionIdentity>> {
    c.validate()?;
    p.validate()?;
    s.validate()?;
    if c.control_points[0].len() != 3
        || p.control_points[0].len() != 2
        || range.iter().flatten().any(|v| !v.is_finite())
        || map_weights.iter().any(|v| !v.is_finite())
        || max_work > cad_predicates::MAX_WORK
    {
        return Err(Error::new(
            "NURBS_INVALID_INPUT",
            "Invalid original projective composition input",
        ));
    }
    let bezier = |k: &[f64], d: usize, n: usize| {
        n == d + 1 && k[..=d].iter().all(|&x| x == k[d]) && k[n..].iter().all(|&x| x == k[n])
    };
    if c.periodic
        || p.periodic
        || s.periodic_u
        || s.periodic_v
        || c.degree > 32
        || p.degree > 8
        || s.degree_u > 8
        || s.degree_v > 8
        || !bezier(&c.knots, c.degree, c.control_points.len())
        || !bezier(&p.knots, p.degree, p.control_points.len())
        || !bezier(&s.knots_u, s.degree_u, s.control_points.len())
        || !bezier(&s.knots_v, s.degree_v, s.control_points[0].len())
    {
        return Ok(None);
    }
    let mut values = Vec::new();
    for (points, weights) in [
        (&c.control_points, &c.weights),
        (&p.control_points, &p.weights),
    ] {
        for (point, w) in points.iter().zip(weights) {
            values.extend(point.iter().copied());
            values.push(*w);
        }
    }
    for (row, weights) in s.control_points.iter().zip(&s.weights) {
        for (point, w) in row.iter().zip(weights) {
            values.extend(point.iter().copied());
            values.push(*w);
        }
    }
    values.extend([
        s.knots_u[s.degree_u],
        s.knots_u[s.control_points.len()],
        s.knots_v[s.degree_v],
        s.knots_v[s.control_points[0].len()],
    ]);
    values.extend(range.iter().flatten().copied());
    values.extend(map_weights);
    let source = SourceArena::authored(
        "original-projective-composition",
        1,
        values
            .into_iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| Error::new("NURBS_INVALID_INPUT", "Invalid projective composition source"))?;
    let mut index = 0;
    let mut leaf = || {
        let r = source.leaf(index).unwrap();
        index += 1;
        r
    };
    let cc: Vec<_> = c
        .control_points
        .iter()
        .map(|_| std::array::from_fn(|_| leaf()))
        .collect();
    let pp: Vec<_> = p
        .control_points
        .iter()
        .map(|_| std::array::from_fn(|_| leaf()))
        .collect();
    let ss: Vec<Vec<_>> = s
        .control_points
        .iter()
        .map(|row| {
            row.iter()
                .map(|_| std::array::from_fn(|_| leaf()))
                .collect()
        })
        .collect();
    let domain = std::array::from_fn(|_| std::array::from_fn(|_| leaf()));
    let range = std::array::from_fn(|_| std::array::from_fn(|_| leaf()));
    let map_weights = std::array::from_fn(|_| leaf());
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work,
            ..Limits::default()
        },
        None,
    );
    let r = cad_predicates::rational_bezier_composition_projective_identity(
        &mut ctx, &cc, &pp, &ss, domain, range, map_weights,
    )
    .map_err(|_| {
        Error::new(
            "NURBS_INVALID_INPUT",
            "Invalid projective composition identity request",
        )
    })?;
    Ok(Some(AlgebraicCompositionIdentity {
        outcome: r.outcome,
        work_used: r.work_used,
        context: r.context,
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    use cad_predicates::BezierIdentity;
    fn plane(fixed: bool) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..2)
                .map(|u| {
                    (0..2)
                        .map(|v| {
                            if fixed {
                                vec![0.609375, u as f64, v as f64]
                            } else {
                                vec![u as f64, v as f64, 0.]
                            }
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn projective_original_sources_preserve_domains_and_budget() {
        let c=Curve{degree:2,knots:vec![-5.,-5.,-5.,3.,3.,3.],
            control_points:vec![vec![0.,0.,0.],vec![0.5,0.,0.],vec![1.,1.,0.]],weights:vec![1.;3],periodic:false};
        let mut p=Curve{degree:2,knots:vec![10.,10.,10.,18.,18.,18.],
            control_points:vec![vec![0.,0.],vec![0.5,0.],vec![1.,1.]],weights:vec![1.,2.,4.],periodic:false};
        let s=plane(false);let range=[[0.,1.],[1.,1.]];
        let report=verify_projective_algebraic(&c,&p,&s,range,[1.,2.],1000000).unwrap().unwrap();
        assert_eq!(report.outcome,BezierIdentity::Equal);
        assert_eq!(verify_algebraic(&c,&p,&s,range,1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
        for budget in [0,report.work_used-1] {
            let r=verify_projective_algebraic(&c,&p,&s,range,[1.,2.],budget).unwrap().unwrap();
            assert_eq!(r.outcome,BezierIdentity::Indeterminate(cad_predicates::Reason::ResourceLimit));
            assert!(r.work_used<=budget);
        }
        p.control_points.reverse();p.weights.reverse();
        assert_eq!(verify_projective_algebraic(&c,&p,&s,[range[1],range[0]],[2.,1.],1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
        p.control_points[1][1]+=0.125;
        assert_eq!(verify_projective_algebraic(&c,&p,&s,[range[1],range[0]],[2.,1.],1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
        assert!(verify_projective_algebraic(&c,&p,&s,range,[f64::NAN,1.],1000000).is_err());
    }
    #[test]
    fn piecewise_uv_and_canonical_curves_use_exact_common_partition() {
        let s=plane(false);
        // Independent authored domains preserve the normalized parameter map.
        // Rational and polynomial controls exercise both homogeneous paths.
        for rational in [false,true] {
            let (coordinates,weights)=if rational {
                (vec![0.,0.,1.,1.],vec![1.,0.75,0.75,1.])
            }else{(vec![0.,0.25,0.75,1.],vec![1.;4])};
            let c=Curve{degree:2,knots:vec![-5.,-5.,-5.,-1.,3.,3.,3.],
                control_points:coordinates.iter().map(|x|vec![*x,2.*x,0.]).collect(),weights:weights.clone(),periodic:false};
            let mut p=Curve{degree:2,knots:vec![10.,10.,10.,14.,18.,18.,18.],
                control_points:coordinates.iter().map(|x|vec![*x,2.*x]).collect(),weights,periodic:false};
            let range=[[0.,1.],[1.,1.]];
            let r=verify_algebraic(&c,&p,&s,range,1000000).unwrap().unwrap();
            assert_eq!(r.outcome,BezierIdentity::Equal);
            let limited=verify_algebraic(&c,&p,&s,range,r.work_used-1).unwrap().unwrap();
            assert_eq!(limited.outcome,BezierIdentity::Indeterminate(cad_predicates::Reason::ResourceLimit));
            p.control_points.reverse();p.weights.reverse();
            assert_eq!(verify_algebraic(&c,&p,&s,[range[1],range[0]],1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
            p.control_points[0][1]+=0.125;
            assert_eq!(verify_algebraic(&c,&p,&s,[range[1],range[0]],1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
        }
        let c=Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![0.,0.,0.],vec![1.,2.,0.]],weights:vec![1.;2],periodic:false};
        let p=Curve{degree:1,knots:vec![10.,10.,12.,16.,18.,18.],
            control_points:vec![vec![0.,0.],vec![0.25,0.5],vec![0.75,1.5],vec![1.,2.]],weights:vec![1.;4],periodic:false};
        assert_eq!(verify_algebraic(&c,&p,&s,[[0.,1.],[1.,1.]],1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
        let c=Curve{degree:1,knots:vec![-5.,-5.,-1.,3.,3.],
            control_points:vec![vec![0.,0.,0.],vec![0.5,1.,0.],vec![1.,2.,0.]],weights:vec![1.;3],periodic:false};
        // Canonical knot at 1/2 and UV knots at 1/4 and 3/4 require
        // four common intervals. Neither source is rounded or resampled.
        assert_eq!(verify_algebraic(&c,&p,&s,[[0.,1.],[1.,1.]],1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
        let mut partial=p.clone();
        partial.control_points=vec![vec![0.25,0.5],vec![0.375,0.75],vec![0.625,1.25],vec![0.75,1.5]];
        let range=[[1.,4.],[3.,4.]];
        let full=verify_algebraic(&c,&partial,&s,range,1000000).unwrap().unwrap();
        assert_eq!(full.outcome,BezierIdentity::Equal);
        assert_eq!(verify_algebraic(&c,&partial,&s,range,0).unwrap().unwrap().outcome,
            BezierIdentity::Indeterminate(cad_predicates::Reason::ResourceLimit));
        partial.control_points.reverse();
        assert_eq!(verify_algebraic(&c,&partial,&s,[range[1],range[0]],1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
        // A changed final UV span must not inherit earlier span equality.
        partial.control_points[3][1]+=0.125;
        assert_eq!(verify_algebraic(&c,&partial,&s,[range[1],range[0]],1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
        let mut wrong_knots=p.clone();wrong_knots.knots[2]=13.;
        assert_eq!(verify_algebraic(&c,&wrong_knots,&s,[[0.,1.],[1.,1.]],1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
    }
    #[test]
    fn partial_piecewise_ranges_keep_exact_source_parameters_and_late_exhaustion() {
        let s=plane(false);
        for rational in [false,true] {
            let (points,weights,source_points,source_weights)=if rational {
                (vec![vec![1.,0.,0.],vec![1.,0.,0.],vec![1.,286.,0.],vec![1.,429.,0.]],vec![1.,0.75,0.75,1.],
                 vec![vec![1.,33.],vec![1.,117.],vec![1.,297.]],vec![0.8125,0.6875,0.8125])
            }else{
                (vec![vec![0.,0.,0.],vec![0.5,0.5,0.],vec![1.5,0.5,0.],vec![2.,0.,0.]],vec![1.;4],
                 vec![vec![0.5,0.375],vec![1.,0.625],vec![1.5,0.375]],vec![1.;3])
            };
            let c=Curve{degree:2,knots:vec![-5.,-5.,-5.,-1.,3.,3.,3.],control_points:points,weights,periodic:false};
            let mut p=Curve{degree:2,knots:vec![10.,10.,10.,18.,18.,18.],control_points:source_points,weights:source_weights,periodic:false};
            let range=[[1.,4.],[3.,4.]];
            let report=verify_algebraic(&c,&p,&s,range,1000000).unwrap().unwrap();
            assert_eq!(report.outcome,BezierIdentity::Equal);
            let budget=report.work_used-1;
            let limited=verify_algebraic(&c,&p,&s,range,budget).unwrap().unwrap();
            assert_eq!(limited.outcome,BezierIdentity::Indeterminate(cad_predicates::Reason::ResourceLimit));
            assert!(limited.work_used<=budget);
            p.control_points.reverse();p.weights.reverse();
            assert_eq!(verify_algebraic(&c,&p,&s,[range[1],range[0]],1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
            let mut changed=c.clone();changed.control_points[3][1]+=0.125;
            assert_eq!(verify_algebraic(&changed,&p,&s,[range[1],range[0]],1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
        }
    }
    #[test]
    fn piecewise_affine_composition_covers_every_rational_knot_span() {
        let s=plane(false);
        for rational in [false,true] {
            let (points,weights,source_points,source_weights)=if rational {
                (vec![vec![1.,0.,0.],vec![1.,0.,0.],vec![1.,2.,0.],vec![1.,3.,0.]],vec![1.,0.75,0.75,1.],
                 vec![vec![1.,0.],vec![1.,0.],vec![1.,3.]],vec![1.,0.5,1.])
            }else{
                (vec![vec![0.,0.,0.],vec![0.5,0.5,0.],vec![1.5,0.5,0.],vec![2.,0.,0.]],vec![1.;4],
                 vec![vec![0.,0.],vec![1.,1.],vec![2.,0.]],vec![1.;3])
            };
            let c=Curve{degree:2,knots:vec![0.,0.,0.,0.5,1.,1.,1.],control_points:points,weights,periodic:false};
            let mut p=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:source_points,weights:source_weights,periodic:false};
            let range=[[0.,1.],[1.,1.]];
            assert_eq!(verify_algebraic(&c,&p,&s,range,1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
            p.control_points.reverse();p.weights.reverse();
            assert_eq!(verify_algebraic(&c,&p,&s,[range[1],range[0]],1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
            let mut changed=c.clone();changed.control_points[3][1]+=0.125;
            assert_eq!(verify_algebraic(&changed,&p,&s,[range[1],range[0]],1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
            assert!(matches!(verify_algebraic(&c,&p,&s,range,0).unwrap().unwrap().outcome,BezierIdentity::Indeterminate(_)));
        }
    }
    #[test]
    fn spline_span_pullback_is_exact_and_refuses_cross_span_or_changed_weights() {
        let s=plane(false);
        for rational in [false,true] {
            let (c,mut p)=if rational {
                let points=vec![vec![1.,0.,0.],vec![1.,0.,0.],vec![1.,0.,0.],vec![1.,1.,0.],vec![0.,1.,0.],vec![0.,1.,0.],vec![0.,1.,0.]];
                (Curve{degree:2,knots:vec![-1.,-1.,-1.,0.,0.,1.,1.,2.,2.,2.],control_points:points,weights:vec![1.,1.,1.,0.5,1.,1.,1.],periodic:false},
                 Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![1.,0.],vec![1.,1.],vec![0.,1.]],weights:vec![1.,0.5,1.],periodic:false})
            }else{
                (Curve{degree:2,knots:vec![0.,0.,0.,1.,2.,3.,3.,3.],control_points:vec![vec![0.,0.,0.],vec![1.,1.,0.],vec![2.,0.,0.],vec![3.,1.,0.],vec![4.,0.,0.]],weights:vec![1.;5],periodic:false},
                 Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![1.5,0.5],vec![2.,0.],vec![2.5,0.5]],weights:vec![1.;3],periodic:false})
            };
            let original=c.clone();let range=[[1.,3.],[2.,3.]];
            for crossing in [
                [[1f64.next_down(),3.],[2.,3.]],
                [[1.,3.],[2f64.next_up(),3.]],
            ] {
                let refused=verify_algebraic(&c,&p,&s,crossing,1000000).unwrap().unwrap();
                assert_ne!(refused.outcome,BezierIdentity::Equal,"An exact fraction outside the original span cannot be clipped");
            }
            let r=verify_algebraic(&c,&p,&s,range,1000000).unwrap().unwrap();
            assert_eq!(r.outcome,BezierIdentity::Equal);assert!(r.work_used<=1000000);
            p.control_points.reverse();p.weights.reverse();
            assert_eq!(verify_algebraic(&c,&p,&s,[range[1],range[0]],1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
            p.weights[1]+=0.125;
            assert_eq!(verify_algebraic(&c,&p,&s,[range[1],range[0]],1000000).unwrap().unwrap().outcome,BezierIdentity::Different);
            assert_ne!(verify_algebraic(&c,&p,&s,[[0.,1.],[1.,1.]],1000000).unwrap().unwrap().outcome,BezierIdentity::Equal);
            assert!(matches!(verify_algebraic(&c,&p,&s,range,0).unwrap().unwrap().outcome,BezierIdentity::Indeterminate(_)));
            assert_eq!(c,original);
        }
    }
    #[test]
    fn closing_line_coverage_uses_exact_fraction_endpoints_and_preserves_definitions() {
        let c = Curve {
            degree: 1,
            knots: vec![-5., -5., 3., 3.],
            control_points: vec![vec![0.609375, -0.25, 0.], vec![0.609375, 1.25, 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let p = Curve {
            degree: 1,
            knots: vec![10., 10., 18., 18.],
            control_points: vec![vec![0., 0.], vec![0.859375, 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let s = plane(true);
        let range = [[1., 6.], [71., 96.]];
        assert_eq!(
            verify_algebraic(&c, &p, &s, range, 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Equal
        );
        assert_eq!(
            crate::curve_surface_agreement::verify_exact_algebraic(&c, &p, &s, false, 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Different
        );
        let mut reverse = p.clone();
        reverse.control_points.reverse();
        assert_eq!(
            verify_algebraic(&c, &reverse, &s, [range[1], range[0]], 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Equal
        );
        assert_eq!(
            verify_algebraic(&c, &p, &s, [[1., 6.], [71. + 1e-12, 96.]], 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Different
        );
        for invalid in [
            [[1., 0.], [71., 96.]],
            [[1., 6.], [1., 6.]],
            [[1., 6.], [97., 96.]],
        ] {
            assert!(matches!(
                verify_algebraic(&c, &p, &s, invalid, 1_000_000)
                    .unwrap()
                    .unwrap()
                    .outcome,
                BezierIdentity::Indeterminate(_)
            ));
        }
        assert!(matches!(
            verify_algebraic(&c, &p, &s, range, 0)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Indeterminate(_)
        ));
    }
    #[test]
    fn curved_and_weighted_canonical_subintervals_have_exact_formal_composition() {
        let s = plane(false);
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let p = Curve {
            degree: 2,
            knots: c.knots.clone(),
            control_points: vec![
                vec![0.9375, 0.4375],
                vec![0.8125, 0.8125],
                vec![0.4375, 0.9375],
            ],
            weights: vec![1.; 3],
            periodic: false,
        };
        assert_eq!(
            verify_algebraic(&c, &p, &s, [[1., 4.], [3., 4.]], 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Equal
        );
        let c = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1., 2.],
            periodic: false,
        };
        let p = Curve {
            degree: 1,
            knots: c.knots.clone(),
            control_points: vec![vec![0.5, 0.], vec![1., 0.]],
            weights: vec![4., 6.],
            periodic: false,
        };
        assert_eq!(
            verify_algebraic(&c, &p, &s, [[1., 3.], [1., 1.]], 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Equal
        );
        let mut wrong = p.clone();
        wrong.weights[1] = 6. + 1e-12;
        assert_eq!(
            verify_algebraic(&c, &wrong, &s, [[1., 3.], [1., 1.]], 1_000_000)
                .unwrap()
                .unwrap()
                .outcome,
            BezierIdentity::Different
        );
    }
}
