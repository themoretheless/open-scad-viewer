//! Contact width quotient jets from original laws. The anchor interval is an
//! explicit premise. This does not establish anchor ownership or rail contact.
use super::*;

/// Original contact control jets with explicit q/anchor premises. Continuous
/// normal-plane compatibility, retained stations and caps remain separate.
pub fn certify_contact_control_trajectory(
    path:&Curve,guide:&Curve,scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    q:[[f64;2];3],anchor:[f64;2],traversal:[f64;2],max_cells:usize,
)->Result<TrajectoryReport>{
    let batch=certify_contact_control_trajectories(path,guide,scale,twist,affine,&[q],anchor,traversal,max_cells)?;
    Ok(TrajectoryReport {traversal,status:batch.status,cells:batch.cells,jet:batch.jets.map(|mut v|v.remove(0)),single_span:batch.single_span,reason:batch.reason})
}
/// Original fitted-control jets, sharing every source restriction once.
pub fn certify_contact_control_trajectories(
    path:&Curve,guide:&Curve,scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    qs:&[[[f64;2];3]],anchor:[f64;2],traversal:[f64;2],max_cells:usize,
)->Result<TrajectoriesReport>{
    twist.validate()?;
    check(
        twist.control_points.iter().all(|p| p[0] == 0.),
        "Contact control requires zero twist",
    )?;
    let fit = certify_contact_fit(path, guide, scale, affine, anchor, traversal, max_cells)?;
    if fit.status != Status::Certified {
        return Ok(TrajectoriesReport {
            traversal,
            status: Status::Unresolved,
            cells: fit.cells,
            jets: None,
            single_span: false,
            reason: Some("contact-fit-enclosure-unresolved"),
        });
    }
    let mut frame = certify_path_guide(path, guide, twist, traversal, max_cells - fit.cells)?;
    frame.cells += fit.cells;
    frame.single_span &= fit.single_span;
    let decode = |v: [f64; 2]| I::new(v[0], v[1]);
    super::trajectory::control_trajectories_with_fit(
        path,
        scale,
        affine,
        qs,
        traversal,
        max_cells,
        frame,
        Some([
            decode(fit.value.unwrap())?,
            decode(fit.first.unwrap())?,
            decode(fit.second.unwrap())?,
        ]),
    )
}

pub fn certify_contact_fit(
    path: &Curve,
    guide: &Curve,
    scale: &Curve,
    affine: Option<(&Curve, &Curve)>,
    anchor: [f64; 2],
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<GuideWidthReport> {
    let anchor = I::new(anchor[0], anchor[1])?;
    check(anchor.lo > 0., "Contact anchor width must be positive")?;
    super::super::validate_law(scale, true)?;
    let width = certify_guide_width(path, guide, traversal, max_cells)?;
    let mut out = GuideWidthReport {
        status: Status::Unresolved,
        cells: width.cells,
        single_span: false,
        value: None,
        first: None,
        second: None,
    };
    if width.status != Status::Certified {
        return Ok(out);
    }
    let uniform = super::super::scalar_certificate::certify_traversal(
        scale,
        traversal,
        max_cells - out.cells,
    )?;
    out.cells += uniform.cells;
    if uniform.status != Status::Certified {
        return Ok(out);
    }
    let uniform_single_span = uniform.single_span;
    let [a, b] = scale.domain();
    let span = I::point(b).sub(I::point(a))?;
    let interval = |x: [f64; 2]| I::new(x[0], x[1]);
    let uniform = [
        interval(uniform.value.unwrap())?,
        interval(uniform.first.unwrap())?.mul(span)?,
        interval(uniform.second.unwrap())?.mul(span.mul(span)?)?,
    ];
    let zero = I::point(0.);
    let mut axis = [I::point(1.), zero, zero];
    let mut center = [zero; 3];
    let mut single_span = width.single_span && uniform_single_span;
    if let Some((axes, centers)) = affine {
        let ar =
            vector_certificate::certify_traversal(axes, traversal, max_cells - out.cells, true)?;
        out.cells += ar.cells;
        if ar.status != Status::Certified {
            return Ok(out);
        }
        let cr = vector_certificate::certify_traversal(
            centers,
            traversal,
            max_cells - out.cells,
            false,
        )?;
        out.cells += cr.cells;
        if cr.status != Status::Certified {
            return Ok(out);
        }
        let aj = jet(axes, &ar)?;
        let cj = jet(centers, &cr)?;
        axis = [aj.v[0], aj.d[0], aj.dd[0]];
        center = [cj.v[0], cj.d[0], cj.dd[0]];
        single_span &= ar.single_span && cr.single_span;
    }
    let mut denominator = super::trajectory::product(uniform, axis)?;
    for k in 0..3 {
        denominator[k] = denominator[k].mul(anchor)?.add(center[k])?;
    }
    if denominator[0].lo <= 0. {
        return Ok(out);
    }
    let value = interval(width.value.unwrap())?.div(denominator[0])?;
    let first = interval(width.first.unwrap())?
        .sub(value.mul(denominator[1])?)?
        .div(denominator[0])?;
    let second = interval(width.second.unwrap())?
        .sub(value.mul(denominator[2])?)?
        .sub(first.mul(denominator[1])?.mul(I::point(2.))?)?
        .div(denominator[0])?;
    out.status = Status::Certified;
    out.single_span = single_span;
    out.value = Some([value.lo, value.hi]);
    out.first = Some([first.lo, first.hi]);
    out.second = Some([second.lo, second.hi]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_contact_quotient_jets_enclose_joint_laws_and_refuse_partial_work() {
        let line = |a, b, domain: [f64; 2]| {
            let mut c = crate::primitives::line(a, b).unwrap();
            c.knots = vec![domain[0], domain[0], domain[1], domain[1]];
            c
        };
        let path = line([0.; 3], [0., 0., 10.], [2., 5.]);
        let guide = line([2., 0., 0.], [2., 0., 10.], [17., 19.]);
        let scale = line([1., 0., 0.], [2., 0., 0.], [23., 29.]);
        let axes = line([1.; 3], [2., 1., 1.], [31., 41.]);
        let center = line([0.; 3], [0.5, 0., 0.], [43., 47.]);
        let report = certify_contact_fit(
            &path,
            &guide,
            &scale,
            Some((&axes, &center)),
            [2., 2.],
            [0.25, 0.5],
            100,
        )
        .unwrap();
        assert_eq!(report.status, Status::Certified);
        assert!(report.single_span);
        for t in [0.25_f64, 0.375, 0.5] {
            let denominator = 2. * (1. + t).powi(2) + 0.5 * t;
            let d = 4. * (1. + t) + 0.5;
            let values = [
                2. / denominator,
                -2. * d / denominator.powi(2),
                4. * d * d / denominator.powi(3) - 8. / denominator.powi(2),
            ];
            for (bound, expected) in [
                report.value.unwrap(),
                report.first.unwrap(),
                report.second.unwrap(),
            ]
            .into_iter()
            .zip(values)
            {
                assert!(
                    bound[0] <= expected && expected <= bound[1],
                    "{bound:?} excludes {expected}"
                );
            }
        }
        let short = certify_contact_fit(
            &path,
            &guide,
            &scale,
            Some((&axes, &center)),
            [2., 2.],
            [0.25, 0.5],
            report.cells - 1,
        )
        .unwrap();
        assert_eq!(short.status, Status::Unresolved);
        assert!(short.value.is_none() && short.first.is_none() && short.second.is_none());
        let mut bad_center = center.clone();
        bad_center.control_points = vec![vec![-10., 0., 0.]; 2];
        let bad = certify_contact_fit(
            &path,
            &guide,
            &scale,
            Some((&axes, &bad_center)),
            [2., 2.],
            [0.25, 0.5],
            100,
        )
        .unwrap();
        assert_eq!(bad.status, Status::Unresolved);
        assert!(bad.value.is_none());
    }

    #[test]
    fn contact_control_composes_transverse_fit_only_and_shares_all_work() {
        let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
        let guide = crate::primitives::line([2., 0., 0.], [2., 0., 10.]).unwrap();
        let scale = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        let axes = crate::primitives::line([1.; 3], [2., 1., 1.]).unwrap();
        let center = crate::primitives::line([0.; 3], [0.5, 0., 0.]).unwrap();
        let mut twist = scale.clone();
        twist.control_points = vec![vec![0.; 3]; 2];
        let q = [[1., 1.], [3., 3.], [4., 4.]];
        let report = certify_contact_control_trajectory(
            &path,
            &guide,
            &scale,
            &twist,
            Some((&axes, &center)),
            q,
            [2., 2.],
            [0.25, 0.5],
            100,
        )
        .unwrap();
        assert_eq!(report.status, Status::Certified);
        assert!(report.single_span);
        let jet = report.jet.unwrap();
        for t in [0.25_f64, 0.375, 0.5] {
            let n = (1. + t).powi(2) + 0.5 * t;
            let d = 2. * (1. + t).powi(2) + 0.5 * t;
            let nd = 2. * (1. + t) + 0.5;
            let dd = 4. * (1. + t) + 0.5;
            let numerator = nd * d - n * dd;
            let expected = [
                [2. * n / d, 3. * (1. + t), 4. + 14. * t],
                [2. * numerator / d.powi(2), 3., 14.],
                [
                    2. * ((2. * d - 4. * n) / d.powi(2) - 2. * numerator * dd / d.powi(3)),
                    0.,
                    0.,
                ],
            ];
            for (bounds, values) in [jet.value, jet.first, jet.second].into_iter().zip(expected) {
                for k in 0..3 {
                    assert!(
                        bounds[k][0] <= values[k] && values[k] <= bounds[k][1],
                        "{bounds:?} excludes {values:?}"
                    );
                }
            }
        }
        let short = certify_contact_control_trajectory(
            &path,
            &guide,
            &scale,
            &twist,
            Some((&axes, &center)),
            q,
            [2., 2.],
            [0.25, 0.5],
            report.cells - 1,
        )
        .unwrap();
        assert_eq!(short.status, Status::Unresolved);
        assert!(short.jet.is_none() && short.cells <= report.cells - 1);
    }
}


#[test]
fn shared_control_jets_preserve_original_analytic_transport_and_whole_batch_refusal() {
    let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let guide=crate::primitives::line([2.,0.,0.],[2.,0.,10.]).unwrap();
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=Curve {degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![0.;3];2],weights:vec![1.;2],periodic:false};
    let qs=[[[1.,1.],[0.,0.],[0.,0.]],[[2.,2.],[3.,3.],[4.,4.]]];
    for contact in [false,true] {
        let batch=if contact {certify_contact_control_trajectories(&path,&guide,&scale,&twist,None,&qs,[2.,2.],[0.25,0.5],1000)} else {certify_path_guide_control_trajectories(&path,&guide,&scale,&twist,None,&qs,[0.25,0.5],1000)}.unwrap();
        assert_eq!(batch.status,Status::Certified);
        let jets=batch.jets.as_ref().unwrap();
        let mut separate_cells=0;
        for (i,&q) in qs.iter().enumerate() {
            let single=if contact {certify_contact_control_trajectory(&path,&guide,&scale,&twist,None,q,[2.,2.],[0.25,0.5],1000)} else {certify_path_guide_control_trajectory(&path,&guide,&scale,&twist,None,q,[0.25,0.5],1000)}.unwrap();
            let jet=single.jet.unwrap();
            assert_eq!(jets[i].value,jet.value);
            assert_eq!(jets[i].first,jet.first);
            assert_eq!(jets[i].second,jet.second);
            assert_eq!(batch.cells,single.cells);
            separate_cells+=single.cells;
            for t in [0.25,0.375,0.5] {
                let value=[if contact {q[0][0]} else {(1.+t)*q[0][0]},(1.+t)*q[1][0],10.*t+(1.+t)*q[2][0]];
                let first=[if contact {0.} else {q[0][0]},q[1][0],10.+q[2][0]];
                for (bounds,expected) in [(jets[i].value,value),(jets[i].first,first),(jets[i].second,[0.;3])] {
                    for k in 0..3 {assert!(bounds[k][0]<=expected[k]&&expected[k]<=bounds[k][1]);}
                }
            }
        }
        assert!(batch.cells<separate_cells);
        let short=if contact {certify_contact_control_trajectories(&path,&guide,&scale,&twist,None,&qs,[2.,2.],[0.25,0.5],batch.cells-1)} else {certify_path_guide_control_trajectories(&path,&guide,&scale,&twist,None,&qs,[0.25,0.5],batch.cells-1)}.unwrap();
        assert_eq!(short.status,Status::Unresolved);
        assert!(short.jets.is_none());
    }
}
