//! Compatibility encoding for native point and cloud fits.
use super::*;
use value_codec::{Serialize, Value, json};
pub fn fit_curve_points(
    points: Vec<Vec<f64>>,
    control_count: usize,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(fit_curve_points_report(points, control_count, tolerance)?.to_value())
}
pub fn interpolate_surface_grid(
    points: Vec<Vec<[f64; 3]>>,
    fitting: bool,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(interpolate_surface_grid_report(points, fitting, tolerance)?.to_value())
}
pub fn fit_curve_cloud_certified(
    points: Vec<Vec<f64>>,
    control_count: usize,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(fit_curve_cloud_certified_report(points, control_count, tolerance)?.to_value())
}
pub fn fit_surface_cloud_certified(
    points: Vec<[f64; 3]>,
    controls_u: usize,
    controls_v: usize,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(fit_surface_cloud_certified_report(points, controls_u, controls_v, tolerance)?.to_value())
}
impl<C: Serialize> Serialize for CurveFitResult<C> {
    fn to_value(&self) -> Value {
        json!({"curve":self.curve,"certificate":self.certificate})
    }
}
impl<C: Serialize> Serialize for SurfaceFitResult<C> {
    fn to_value(&self) -> Value {
        json!({"surface":self.surface,"certificate":self.certificate})
    }
}
impl Serialize for CurveFitCertificate {
    fn to_value(&self) -> Value {
        json!({"version":"nurbs-foundation/3","classification":"approximate_fit",
 "fittedToExactPromotion":false,"dataSiteErrorUpper":self.data_site_error_upper,"siteCount":self.site_count,
 "controlCount":self.control_count,"degree":self.degree,"conditioning":{"method":"normal-equations-pivoted-elimination","rank":self.control_count},
 "resource":{"maxSites":4096,"maxControls":26},"evidence":super::super::tolerance_evidence(&self.tolerance)})
    }
}
impl Serialize for GridFitCertificate {
    fn to_value(&self) -> Value {
        json!({"version":"nurbs-foundation/3","classification":if self.fitting{"approximate_fit"}else{"interpolation"},
 "fittedToExactPromotion":false,"dataSiteErrorUpper":0.,"rank":self.rank,
 "conditioning":{"method":"tensor-degree-one-cardinal","conditionUpper":1.},"resource":{"maxControlsPerAxis":32},
 "evidence":super::super::tolerance_evidence(&self.tolerance)})
    }
}
fn cloud_evidence(e: &CloudFitEvidence) -> Value {
    json!({"version":"nurbs-foundation/4","classification":"approximate_cloud_fit","fittedToExactPromotion":false,
 "dataSiteErrorUpper":e.data_site_error_upper,"hausdorffErrorUpper":e.hausdorff_error_upper,"siteCount":e.site_count,
 "conditioning":{"method":"robust-pivoted-normal-equations","rank":e.rank,"pivotLower":e.pivot_lower},
 "evidence":super::super::tolerance_evidence(&e.tolerance)})
}
impl Serialize for CloudCurveCertificate {
    fn to_value(&self) -> Value {
        let mut result = cloud_evidence(&self.evidence);
        result["controlCount"] = json!(self.control_count);
        result["requestedControls"] = json!(self.requested_controls);
        result["admittedAssumptions"] = json!([
            "chordal-parameter-delta-net-of-fitted-domain",
            "residual-Lipschitz-from-control-polygon-speed"
        ]);
        result["resource"] = json!({"maxSites":4096,"maxControls":26});
        result
    }
}
impl Serialize for CloudSurfaceCertificate {
    fn to_value(&self) -> Value {
        let mut result = cloud_evidence(&self.evidence);
        result["controlsU"] = json!(self.controls_u);
        result["controlsV"] = json!(self.controls_v);
        result["admittedAssumptions"] = json!([
            "PCA-plane-parameter-domain",
            "delta-net-density-from-sqrt-N-samples",
            "control-hull-Lipschitz-remainder"
        ]);
        result["resource"] = json!({"maxSites":4096,"maxControlsPerAxis":8});
        result
    }
}
