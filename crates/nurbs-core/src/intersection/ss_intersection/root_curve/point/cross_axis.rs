//! Exact cross-axis identity from original equations, not overlapping images.
use super::*;
fn polynomial(point:&OriginalSsRootPoint,limits:SsAlgebraicClockLimits)->Result<Vec<Q>> {
    match &point.clock {
        Clock::Binary(t)=>Ok(vec![-Q::from_float(*t).unwrap(),Q::one()]),
        Clock::Algebraic {point,..}=>{point.clock().enclose(0,limits)?;Ok(point.clock().polynomial().to_vec())},
    }
}
fn directed(a:&OriginalSsRootPoint,b:&OriginalSsRootPoint,clock:SsAlgebraicClockLimits,geometry:OriginalSsCurveLimits)->Result<Option<bool>> {
    let Carrier::Deflated(f)=&a.curve.carrier else {return Ok(None)};
    let ideal=f.source().original().original();
    let fa=frames::frame(&a.curve)?;
    let fb=frames::frame(&b.curve)?;
    let k=a.curve.free_axis().index();
    let j=b.curve.free_axis().index();
    check(fb[j][1]>Q::zero(),"Cross-axis target clock frame must increase")?;
    let pa=polynomial(a,clock)?;
    let pb=polynomial(b,clock)?;
    let scale=&fa[j][1]/&fb[j][1];
    let shift=(&fa[j][0]-&fb[j][0])/&fb[j][1];
    let (zero,_)=super::super::super::algebraic::prove_clock_consequence(
        ideal,(&pa,k),(&pb,j,scale,shift),ExactSsGroebnerLimits {algebra:geometry.projection,..Default::default()})?;
    if !zero {return Ok(Some(false));}
    let bounds=port::membership::bind(a,&b.curve,[0;4],geometry)?;
    let selected=match &b.clock {
        Clock::Binary(t)=>{let q=Q::from_float(*t).unwrap();[q.clone(),q]},
        Clock::Algebraic {point,..}=>point.clock().isolating_interval().clone(),
    };
    // A linear polynomial has exactly one root globally; otherwise the whole
    // native target-clock enclosure must lie in the owned unique-root isolator.
    Ok(Some(pb.len()==2 || (selected[0]<=bounds[0] && bounds[1]<=selected[1])))
}
pub(super) fn prove(a:&OriginalSsRootPoint,b:&OriginalSsRootPoint,clock:SsAlgebraicClockLimits,geometry:OriginalSsCurveLimits)->Result<bool> {
    if directed(a,b,clock,geometry)?==Some(true) {return Ok(true);}
    Ok(directed(b,a,clock,geometry)?.unwrap_or(false))
}

pub(super) fn reparameterize(point:&OriginalSsRootPoint,target:Arc<OriginalSsRootCurve>,clock:SsAlgebraicClockLimits,geometry:OriginalSsCurveLimits)->Result<OriginalSsRootPoint> {
    let Carrier::Deflated(f)=&point.curve.carrier else {
        return Err(crate::input("SS clock projection requires an original deflated span graph"));
    };
    // Native source membership first supplies a conservative target-clock box.
    let bounds=port::membership::bind(point,&target,[0;4],geometry)?;
    let fa=frames::frame(&point.curve)?;
    let fb=frames::frame(&target)?;
    let k=point.curve.free_axis().index();
    let j=target.free_axis().index();
    check(fa[j][1]>Q::zero() && fb[j][1]>Q::zero(),"Projected SS frames must increase")?;
    let scale=&fb[j][1]/&fa[j][1];
    let shift=(&fb[j][0]-&fa[j][0])/&fa[j][1];
    let source=polynomial(point,clock)?;
    let (coefficients,_)=super::super::super::algebraic::project_original_ss_clock(
        f.source().original().original(),(&source,k),(j,scale,shift),
        ExactSsGroebnerLimits {algebra:geometry.projection,..Default::default()})?;
    let selected=Arc::new(QualifiedSsAlgebraicClock::qualify(coefficients,bounds,clock)?);
    OriginalSsRootPoint::from_algebraic(OriginalSsAlgebraicPoint::new(target,selected)?,
        SsAlgebraicPointLimits {clock,geometry,..Default::default()})
}
