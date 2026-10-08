//! Original-law contact fit values, including point restrictions. No derivatives
//! or contact identities are inferred; anchor ownership is a separate premise.
use super::*;

#[derive(Clone, Debug)]
pub struct ContactFitValueReport {
    pub status: Status,
    pub cells: usize,
    pub value: Option<[f64; 2]>,
}

/// Original contact control value with explicit coordinate/anchor premises.
pub fn certify_contact_control_value(
    path:&Curve,guide:&Curve,scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    q:[[f64;2];3],anchor:[f64;2],traversal:[f64;2],max_cells:usize,
)->Result<ControlValueReport>{
    let batch=certify_contact_control_values(path,guide,scale,twist,affine,&[q],anchor,traversal,max_cells)?;
    Ok(ControlValueReport {status:batch.status,cells:batch.cells,value:batch.values.map(|v|v[0]),reason:batch.reason})
}
/// Shared source-law/frame/fit work; no partial control union on exhaustion.
pub fn certify_contact_control_values(
    path:&Curve,guide:&Curve,scale:&Curve,twist:&Curve,affine:Option<(&Curve,&Curve)>,
    qs:&[[[f64;2];3]],anchor:[f64;2],traversal:[f64;2],max_cells:usize,
)->Result<ControlValuesReport>{
    twist.validate()?;
    check(
        twist.control_points.iter().all(|p| p[0] == 0.),
        "Contact control requires zero twist",
    )?;
    let (fit,source_path,offset) = contact_fit_value_at_prepared(path,guide,scale,affine,anchor,traversal,traversal,traversal,true,max_cells)?;
    if fit.status != Status::Certified {
        return Ok(ControlValuesReport {
            status: Status::Unresolved,
            cells: fit.cells,
            values: None,
            reason: Some("contact-fit-value-unresolved"),
        });
    }
    let mut frame =
        super::guided_path_values::prepared_offset_frame_values(path,twist,traversal,traversal,offset.unwrap(),max_cells-fit.cells)?;
    frame.cells += fit.cells;
    let fit = fit.value.unwrap();
    super::trajectory::control_values_with_prepared_path(
        path,
        scale,
        affine,
        qs,
        traversal,
        max_cells,
        frame,
        Some(I::new(fit[0], fit[1])?),
        source_path,
    )
}

pub fn certify_contact_fit_value(
    path: &Curve,
    guide: &Curve,
    scale: &Curve,
    affine: Option<(&Curve, &Curve)>,
    anchor: [f64; 2],
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<ContactFitValueReport> {
    Ok(certify_contact_fit_value_with_path(path,guide,scale,affine,anchor,traversal,max_cells)?.0)
}
fn certify_contact_fit_value_with_path(
    path: &Curve,
    guide: &Curve,
    scale: &Curve,
    affine: Option<(&Curve, &Curve)>,
    anchor: [f64; 2],
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<(ContactFitValueReport,Option<[[f64;2];3]>)> {
    contact_fit_value_at(path,guide,scale,affine,anchor,traversal,traversal,traversal,true,max_cells)
}
/// Source path, source rail and authored laws can have independent parameters.
/// Width follows the constructor's norm(G-P), not its projected normal.
pub(super) fn contact_fit_value_at(
    path:&Curve,guide:&Curve,scale:&Curve,affine:Option<(&Curve,&Curve)>,anchor:[f64;2],
    path_traversal:[f64;2],guide_traversal:[f64;2],law_traversal:[f64;2],same_parameter:bool,max_cells:usize,
)->Result<(ContactFitValueReport,Option<[[f64;2];3]>)>{
    let (fit,position,_)=contact_fit_value_at_prepared(path,guide,scale,affine,anchor,path_traversal,guide_traversal,law_traversal,same_parameter,max_cells)?;
    Ok((fit,position))
}
pub(super) fn contact_fit_value_at_prepared(
    path:&Curve,guide:&Curve,scale:&Curve,affine:Option<(&Curve,&Curve)>,anchor:[f64;2],
    path_traversal:[f64;2],guide_traversal:[f64;2],law_traversal:[f64;2],same_parameter:bool,max_cells:usize,
)->Result<(ContactFitValueReport,Option<[[f64;2];3]>,Option<[[f64;2];3]>)>{
    let anchor = I::new(anchor[0], anchor[1])?;
    check(anchor.lo > 0., "Contact anchor width must be positive")?;
    super::super::validate_law(scale, true)?;
    let mut out = ContactFitValueReport {
        status: Status::Unresolved,
        cells: 0,
        value: None,
    };
    let p = vector_certificate::certify_values_traversal(path, path_traversal, max_cells, false)?;
    out.cells += p.cells;
    let Some(p) = p.value else {
        return Ok((out,None,None));
    };
    let paired=if same_parameter && path_traversal==guide_traversal && path_traversal[0]!=path_traversal[1] {
        super::paired_path_values::same_parameter_offset_values(path,guide,path_traversal,max_cells-out.cells)?
    }else{None};
    let offset=if let Some(paired)=paired {
        out.cells+=paired.cells;
        let Some(values)=paired.value else{return Ok((out,None,None))};
        decode(values)?
    }else{
    let g = vector_certificate::certify_values_traversal(
        guide,
        guide_traversal,
        max_cells - out.cells,
        false,
    )?;
    out.cells += g.cells;
    let Some(g) = g.value else {
        return Ok((out,None,None));
    };
    sub(decode(g)?, decode(p)?)?
    };
    let squared = square(offset[0])?
        .add(square(offset[1])?)?
        .add(square(offset[2])?)?;
    let width = I::new(
        squared.lo.max(0.).sqrt().next_down().max(0.),
        squared.hi.sqrt().next_up(),
    )?;
    if width.lo <= 0. {
        return Ok((out,None,None));
    }
    let charge = (scale.degree..scale.control_points.len())
        .filter(|&i| scale.knots[i] < scale.knots[i + 1])
        .count();
    if charge > max_cells - out.cells {
        return Ok((out,None,None));
    }
    let uniform = super::super::scalar_certificate::value_traversal(scale, law_traversal, charge)?;
    out.cells += charge;
    let Some(uniform) = uniform else {
        return Ok((out,None,None));
    };
    let mut axis = I::point(1.);
    let mut center = I::point(0.);
    if let Some((axes, centers)) = affine {
        let a = vector_certificate::certify_values_traversal(
            axes,
            law_traversal,
            max_cells - out.cells,
            true,
        )?;
        out.cells += a.cells;
        let Some(a) = a.value else {
            return Ok((out,None,None));
        };
        axis = I::new(a[0][0], a[0][1])?;
        let c = vector_certificate::certify_values_traversal(
            centers,
            law_traversal,
            max_cells - out.cells,
            false,
        )?;
        out.cells += c.cells;
        let Some(c) = c.value else {
            return Ok((out,None,None));
        };
        center = I::new(c[0][0], c[0][1])?;
    }
    let denominator = I::new(uniform[0], uniform[1])?
        .mul(axis)?
        .mul(anchor)?
        .add(center)?;
    if denominator.lo <= 0. {
        return Ok((out,None,None));
    }
    let value = width.div(denominator)?;
    out.status = Status::Certified;
    out.value = Some([value.lo, value.hi]);
    Ok((out,Some(p),Some(std::array::from_fn(|k|[offset[k].lo,offset[k].hi]))))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contact_fit_point_values_enclose_independent_joint_formula_tightly() {
        let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
        let guide = crate::primitives::line([2., 0., 0.], [2., 0., 10.]).unwrap();
        let mut scale = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        scale.knots = vec![7., 7., 9., 9.];
        let mut axes = crate::primitives::line([1.; 3], [2., 1., 1.]).unwrap();
        axes.knots = vec![17., 17., 19., 19.];
        let mut center = crate::primitives::line([0.; 3], [0.5, 0., 0.]).unwrap();
        center.knots = vec![23., 23., 29., 29.];
        for t in [0.0_f64, 0.13, 0.375, 0.5, 0.87, 1.] {
            let report = certify_contact_fit_value(
                &path,
                &guide,
                &scale,
                Some((&axes, &center)),
                [2., 2.],
                [t, t],
                100,
            )
            .unwrap();
            assert_eq!(report.status, Status::Certified);
            let bounds = report.value.unwrap();
            let expected = 2. / (2. * (1. + t).powi(2) + 0.5 * t);
            assert!(bounds[0] <= expected && expected <= bounds[1]);
            assert!(bounds[1] - bounds[0] < 1e-9);
            let mut twist = scale.clone();
            twist.control_points = vec![vec![0.; 3]; 2];
            let value = certify_contact_control_value(
                &path,
                &guide,
                &scale,
                &twist,
                Some((&axes, &center)),
                [[1., 1.], [3., 3.], [4., 4.]],
                [2., 2.],
                [t, t],
                200,
            )
            .unwrap();
            assert_eq!(value.status, Status::Certified);
            let xyz = value.value.unwrap();
            let ideal = [
                expected * ((1. + t).powi(2) + 0.5 * t),
                3. * (1. + t),
                4. + 14. * t,
            ];
            for k in 0..3 {
                assert!(xyz[k][0] <= ideal[k] && ideal[k] <= xyz[k][1]);
                assert!(xyz[k][1] - xyz[k][0] < 1e-9);
            }
            let limited = certify_contact_control_value(
                &path,
                &guide,
                &scale,
                &twist,
                Some((&axes, &center)),
                [[1., 1.], [3., 3.], [4., 4.]],
                [2., 2.],
                [t, t],
                value.cells - 1,
            )
            .unwrap();
            assert_eq!(limited.status, Status::Unresolved);
            assert!(limited.value.is_none());
            let short = certify_contact_fit_value(
                &path,
                &guide,
                &scale,
                Some((&axes, &center)),
                [2., 2.],
                [t, t],
                report.cells - 1,
            )
            .unwrap();
            assert_eq!(short.status, Status::Unresolved);
            assert!(short.value.is_none() && short.cells <= report.cells - 1);
        }
    }
}

#[test]
fn shared_control_values_preserve_each_original_enclosure_and_charge_sources_once() {
    let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let guide=crate::primitives::line([2.,0.,0.],[2.,0.,10.]).unwrap();
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=Curve {degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![0.;3];2],weights:vec![1.;2],periodic:false};
    let qs=[[[1.,1.],[0.,0.],[0.,0.]],[[2.,2.],[3.,3.],[4.,4.]]];
    for contact in [false,true] {
        let batch=if contact {certify_contact_control_values(&path,&guide,&scale,&twist,None,&qs,[2.,2.],[0.25,0.5],1000)} else {certify_path_guide_control_values(&path,&guide,&scale,&twist,None,&qs,[0.25,0.5],1000)}.unwrap();
        assert_eq!(batch.status,Status::Certified);
        let values=batch.values.as_ref().unwrap();
        let mut separate_cells=0;
        for (i,&q) in qs.iter().enumerate() {
            let single=if contact {certify_contact_control_value(&path,&guide,&scale,&twist,None,q,[2.,2.],[0.25,0.5],1000)} else {certify_path_guide_control_value(&path,&guide,&scale,&twist,None,q,[0.25,0.5],1000)}.unwrap();
            assert_eq!(values[i],single.value.unwrap());
            assert_eq!(batch.cells,single.cells);
            separate_cells+=single.cells;
            for t in [0.25,0.375,0.5] {
                let expected=[if contact {q[0][0]} else {(1.+t)*q[0][0]},(1.+t)*q[1][0],10.*t+(1.+t)*q[2][0]];
                for k in 0..3 {assert!(values[i][k][0]<=expected[k]&&expected[k]<=values[i][k][1]);}
            }
        }
        assert!(batch.cells<separate_cells);
        let short=if contact {certify_contact_control_values(&path,&guide,&scale,&twist,None,&qs,[2.,2.],[0.25,0.5],batch.cells-1)} else {certify_path_guide_control_values(&path,&guide,&scale,&twist,None,&qs,[0.25,0.5],batch.cells-1)}.unwrap();
        assert_eq!(short.status,Status::Unresolved);
        assert!(short.values.is_none());
    }
}


#[test]
fn cached_contact_path_matches_uncached_affine_transport() {
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=Curve {degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![0.;3];2],weights:vec![1.;2],periodic:false};
    let axes=crate::primitives::line([1.,2.,1.],[2.,1.,3.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.25,0.5,0.75]).unwrap();
    let affine=Some((&axes,&center));
    let qs=[[[1.,1.],[0.,0.],[0.,0.]],[[2.,2.],[3.,3.],[4.,4.]]];
    for offset in [0.,7.] {
        let path=crate::primitives::line([offset,0.,0.],[offset,0.,10.]).unwrap();
        let guide=crate::primitives::line([offset+2.,0.,0.],[offset+2.,0.,10.]).unwrap();
        for traversal in [[0.25,0.5],[0.5,0.5],[0.75,1.]] {
            let (fit,prepared)=certify_contact_fit_value_with_path(&path,&guide,&scale,affine,[2.,2.],traversal,1000).unwrap();
            assert_eq!(fit.status,Status::Certified);
            assert!(prepared.is_some());
            let mut frame=certify_path_guide_values(&path,&guide,&twist,traversal,1000-fit.cells).unwrap();
            frame.cells+=fit.cells;
            let fit_value=fit.value.unwrap();
            let uncached=super::trajectory::control_values_with_fit(&path,&scale,affine,&qs,traversal,1000,frame,Some(I::new(fit_value[0],fit_value[1]).unwrap())).unwrap();
            let cached=certify_contact_control_values(&path,&guide,&scale,&twist,affine,&qs,[2.,2.],traversal,1000).unwrap();
            assert_eq!(cached.status,Status::Certified);
            assert_eq!(cached.values,uncached.values);
            let source=vector_certificate::certify_values_traversal(&path,traversal,1000,false).unwrap();
            // Translation plus the prepared frame position and guide offset
            // each reuse an already-owned original value enclosure.
            assert_eq!(uncached.cells-cached.cells,3*source.cells);
            let exhausted=certify_contact_control_values(&path,&guide,&scale,&twist,affine,&qs,[2.,2.],traversal,cached.cells-1).unwrap();
            assert_eq!(exhausted.status,Status::Unresolved);
            assert!(exhausted.values.is_none());
        }
    }
}

#[test]
fn equal_interval_boxes_do_not_imply_shared_contact_parameter() {
    let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let guide=crate::primitives::line([1.,0.,0.],[1.,0.,10.]).unwrap();
    let scale=crate::sweeps::progressive_sweep::constant_vector_law([1.,0.,0.]).unwrap();
    let range=[0.25,0.5];
    let independent=contact_fit_value_at(&path,&guide,&scale,None,[1.,1.],range,range,range,false,1000).unwrap().0;
    let shared=contact_fit_value_at(&path,&guide,&scale,None,[1.,1.],range,range,range,true,1000).unwrap().0;
    assert_eq!(independent.status,Status::Certified);
    assert_eq!(shared.status,Status::Certified);
    let independently_reachable=2.5_f64.hypot(1.);
    assert!(independent.value.unwrap()[1]>=independently_reachable);
    assert!(shared.value.unwrap()[1]<1.+1e-10);
}
