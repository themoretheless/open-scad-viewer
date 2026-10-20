//! Exact point equality on unchanged original rational UV curves.
use crate::{curve::Curve, Error, Result};
use cad_predicates::{AuthoredScalar, Limits, PredicateContext, SourceArena, ToleranceContext};
pub fn verify(
    curves: [&Curve; 2],
    parameters: [f64; 2],
    max_work: u64,
) -> Result<Option<cad_predicates::BezierIdentityDecision>> {
    if parameters.iter().any(|t| !t.is_finite()) || max_work > cad_predicates::MAX_WORK {
        return Err(Error::new(
            "NURBS_INVALID_INPUT",
            "Choose finite point parameters and bounded exact work",
        ));
    }
    for c in curves {
        c.validate()?;
        if c.control_points[0].len() != 2 {
            return Err(Error::new(
                "NURBS_INVALID_INPUT",
                "Choose original UV curves",
            ));
        }
        if c.periodic
            || c.degree == 0
            || c.degree > 32
        {
            return Ok(None);
        }
    }
    let piecewise=curves.iter().any(|c| {
        let n=c.control_points.len();
        n!=c.degree+1 || !c.knots[..=c.degree].iter().all(|&t|t==c.knots[c.degree])
            || !c.knots[n..].iter().all(|&t|t==c.knots[n])
    });
    let mut raw = Vec::new();
    for c in curves {
        for (p, w) in c.control_points.iter().zip(&c.weights) {
            raw.extend([p[0], p[1], *w]);
        }
    }
    for i in 0..2 {
        let d = curves[i].domain();
        raw.extend([parameters[i], d[0], d[1]]);
    }
    if piecewise {for c in curves {raw.extend(c.knots.iter().copied());}}
    let source = SourceArena::authored(
        "original-uv-point-equality",
        1,
        raw.into_iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|_| Error::new("NURBS_INVALID_INPUT", "Invalid exact point inputs"))?;
    let mut cursor = 0;
    let mut poles = Vec::new();
    for c in curves {
        poles.push(
            (0..c.control_points.len())
                .map(|_| {
                    std::array::from_fn(|_| {
                        let r = source.leaf(cursor).unwrap();
                        cursor += 1;
                        r
                    })
                })
                .collect::<Vec<_>>(),
        );
    }
    let parameters = std::array::from_fn(|_| {
        std::array::from_fn(|_| {
            let r = source.leaf(cursor).unwrap();
            cursor += 1;
            r
        })
    });
    let knots:Vec<Vec<_>>=if piecewise {curves.iter().map(|c|(0..c.knots.len()).map(|_|{
        let r=source.leaf(cursor).unwrap();cursor+=1;r
    }).collect()).collect()}else{Vec::new()};
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
    let result=if piecewise {
        cad_predicates::rational_bspline_uv_point_identity(&mut ctx,[&poles[0],&poles[1]],
            [&knots[0],&knots[1]],[curves[0].degree,curves[1].degree],[parameters[0][0],parameters[1][0]])
    }else{cad_predicates::rational_bezier_uv_point_identity(&mut ctx, [&poles[0], &poles[1]], parameters)};
    result
        .map(Some)
        .map_err(|_| {
            Error::new(
                "NURBS_INVALID_INPUT",
                "Invalid exact point identity request",
            )
        })
}

/// Verify rational parameter candidates on unchanged single Bezier UV curves.
pub fn verify_fraction(curves:[&Curve;2],parameters:[[f64;2];2],max_work:u64)
 ->Result<Option<cad_predicates::BezierIdentityDecision>> {
    if max_work>cad_predicates::MAX_WORK || parameters.iter().any(|p|p.iter().any(|t|!t.is_finite()) || p[1]<=0.) {
        return Err(Error::new("NURBS_INVALID_INPUT","Finite rational parameters and positive denominators required"));
    }
    for c in curves {
        c.validate()?;
        if c.control_points[0].len()!=2 {return Err(Error::new("NURBS_INVALID_INPUT","Original UV curves required"));}
        let n=c.control_points.len();let d=c.domain();
        if c.periodic || c.degree==0 || c.degree>32 || n!=c.degree+1
            || !c.knots[..n].iter().all(|&t|t==d[0]) || !c.knots[n..].iter().all(|&t|t==d[1]) {return Ok(None);}
    }
    let mut raw=Vec::new();
    for c in curves {for (p,w) in c.control_points.iter().zip(&c.weights) {raw.extend([p[0],p[1],*w]);}}
    for i in 0..2 {let d=curves[i].domain();raw.extend([parameters[i][0],parameters[i][1],d[0],d[1]]);}
    let arena=SourceArena::authored("original-uv-fraction-point-equality",1,raw.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect())
        .map_err(|_|Error::new("NURBS_INVALID_INPUT","Invalid rational original source"))?;
    let mut cursor=0;
    let poles:Vec<Vec<_>>=curves.iter().map(|c|(0..c.control_points.len()).map(|_|std::array::from_fn(|_|{let r=arena.leaf(cursor).unwrap();cursor+=1;r})).collect()).collect();
    let parameters=std::array::from_fn(|_|std::array::from_fn(|_|{let r=arena.leaf(cursor).unwrap();cursor+=1;r}));
    let tolerance=ToleranceContext::default_valid();
    let mut ctx=PredicateContext::new(&arena,&tolerance,Limits{max_work,..Limits::default()},None);
    cad_predicates::rational_bezier_uv_point_fraction_identity(&mut ctx,[&poles[0],&poles[1]],parameters)
        .map(Some).map_err(|_|Error::new("NURBS_INVALID_INPUT","Invalid rational point equality request"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn multispan_rational_root_candidates_keep_exact_authored_parameters() {
        let a=Curve{degree:2,knots:vec![0.,0.,0.,0.5,1.,1.,1.],
            control_points:vec![vec![0.,0.],vec![1.,0.],vec![1.,2.],vec![0.,3.]],
            weights:vec![1.,0.5,0.5,1.],periodic:false};
        let mut b=a.clone();b.knots=vec![10.,10.,10.,14.,18.,18.,18.];
        for (first,second) in [(0.25,12.),(0.5,14.),(0.75,16.)] {
            let report=verify([&a,&b],[first,second],1000000).unwrap().unwrap();
            assert_eq!(report.outcome,cad_predicates::BezierIdentity::Equal);
            assert_eq!(verify([&a,&b],[first,second],report.work_used-1).unwrap().unwrap().outcome,
                cad_predicates::BezierIdentity::Indeterminate(cad_predicates::Reason::ResourceLimit));
        }
        b.control_points.reverse();b.weights.reverse();
        assert_eq!(verify([&a,&b],[0.25,16.],1000000).unwrap().unwrap().outcome,cad_predicates::BezierIdentity::Equal);
        let mut changed=b.clone();changed.control_points[2][1]+=0.125;
        assert_eq!(verify([&a,&changed],[0.25,16.],1000000).unwrap().unwrap().outcome,cad_predicates::BezierIdentity::Different);
        assert!(matches!(verify([&a,&b],[-0.001,16.],1000000).unwrap().unwrap().outcome,
            cad_predicates::BezierIdentity::Indeterminate(_)));
        assert_eq!(verify([&a,&b],[0.25,16.],0).unwrap().unwrap().outcome,
            cad_predicates::BezierIdentity::Indeterminate(cad_predicates::Reason::ResourceLimit));
        let split=Curve{degree:1,knots:vec![0.,0.,0.5,0.5,1.,1.],
            control_points:vec![vec![0.,0.],vec![0.25,0.],vec![0.75,0.],vec![1.,0.]],weights:vec![1.;4],periodic:false};
        assert!(verify([&split,&split],[0.5,0.5],1000000).is_err(),
            "Disconnected source curves need separate nodes before root identity");
    }
    #[test]
    fn original_rational_points_match_without_cartesian_division() {
        let a = Curve {
            degree: 1,
            knots: vec![2., 2., 4., 4.],
            control_points: vec![vec![0., 0.], vec![1., 0.]],
            weights: vec![1., 2.],
            periodic: false,
        };
        let b = Curve {
            degree: 1,
            knots: vec![10., 10., 18., 18.],
            control_points: vec![vec![1., 0.], vec![0., 0.]],
            weights: vec![2., 1.],
            periodic: false,
        };
        assert_eq!(
            verify([&a, &b], [3., 14.], cad_predicates::MAX_WORK)
                .unwrap()
                .unwrap()
                .outcome,
            cad_predicates::BezierIdentity::Equal
        );
        let mut changed = b.clone();
        changed.control_points[0][0] += 1e-12;
        assert_eq!(
            verify([&a, &changed], [3., 14.], cad_predicates::MAX_WORK)
                .unwrap()
                .unwrap()
                .outcome,
            cad_predicates::BezierIdentity::Different
        );
        assert!(matches!(
            verify([&a, &b], [3., 14.], 1).unwrap().unwrap().outcome,
            cad_predicates::BezierIdentity::Indeterminate(_)
        ));
        assert!(matches!(
            verify([&a, &b], [1., 14.], cad_predicates::MAX_WORK)
                .unwrap()
                .unwrap()
                .outcome,
            cad_predicates::BezierIdentity::Indeterminate(_)
        ));
        let multi = Curve::from_polyline(vec![vec![0., 0.], vec![0.5, 0.], vec![1., 0.]]).unwrap();
        assert_eq!(verify([&a, &multi], [3., 0.5], cad_predicates::MAX_WORK)
            .unwrap().unwrap().outcome,cad_predicates::BezierIdentity::Different);
        assert!(verify([&a, &b], [f64::NAN, 14.], cad_predicates::MAX_WORK).is_err());
        assert_eq!(a.knots, vec![2., 2., 4., 4.]);
    }
}
