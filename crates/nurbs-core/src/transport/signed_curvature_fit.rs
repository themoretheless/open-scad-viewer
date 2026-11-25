use super::*;

pub(super) fn build(v:&Value,op:&str)->Result<Value>{
    let tolerance=field(v,"tolerance_mm")?;
    if op=="curve_from_curvature_samples" {
        let stations:Vec<f64>=field(v,"stations_mm")?;
        let curvatures:Vec<f64>=field(v,"curvatures_per_mm")?;
        check(stations.len()==curvatures.len(),"Every curvature station must have one signed curvature")?;
        let pairs:Vec<_>=stations.into_iter().zip(curvatures).collect();
        let [x,y]:[f64;2]=field(v,"origin")?;
        let r=crate::curve_reconstruction::reconstruct_from_curvature(&pairs,[x,y,0.],field(v,"start_angle_rad")?,tolerance)?;
        return Ok(json!({"curve":r.curve,"sites":r.sites,"tangentAnglesRad":r.tangent_angles,"lengthMm":r.length,
            "quadratureErrorEstimateMm":r.quadrature_error_estimate,"maxSiteDeviationMm":r.fitted_site_max_deviation,
            "maxCurvatureResidualPerMm":r.max_residual_curvature,"undefinedCurvatureSites":r.undefined_curvature_sites,
            "fitConverged":r.fit_converged,"panels":r.integration_panels,"continuousErrorCertified":false}));
    }
    let r=crate::biarc::fit_biares_report(&field(v,"curve")?,tolerance,field(v,"max_arcs")?)?;
    let i:usize=field(v,"piece")?;
    let curve=r.as_nurbs.get(i).ok_or_else(||input("Unknown arc piece"))?;
    let arcs=r.arcs.iter().map(|a|json!({"start":a.start,"joint":a.joint,"end":a.end,
        "radiiMm":a.radii.map(|r|r.is_finite().then_some(r)),"sampledSagittaMm":a.sagitta_error})).collect::<Vec<_>>();
    Ok(json!({"curve":curve,"pieces":r.as_nurbs.len(),"arcs":arcs,"maxSampledErrorMm":r.max_error,"continuousErrorCertified":false}))
}
