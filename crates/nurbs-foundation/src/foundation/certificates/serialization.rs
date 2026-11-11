//! Compatibility encoding for native curve and surface certificates.
use super::*;
use value_codec::{Serialize, Value, json};
pub fn certify_curve(curve: &Curve, tolerance: Option<ToleranceContext>) -> Result<Value> {
    Ok(certify_curve_report(curve, tolerance)?.to_value())
}
pub fn certify_surface(surface: &Surface, tolerance: Option<ToleranceContext>) -> Result<Value> {
    Ok(certify_surface_report(surface, tolerance)?.to_value())
}
impl Serialize for CurveRegularity {
    fn to_value(&self) -> Value {
        match self {
            Self::Constant => json!({"classification":"singular","reason":"constant-span"}),
            Self::Subdivision { depth, children } => {
                json!({"classification":"certified_regular","method":"recursive-Bernstein-component-separation","subdivisionDepth":depth,"children":children})
            }
            Self::Bounds {
                classification,
                derivative_numerator_bounds,
            } => {
                json!({"classification":match classification {CurveRegularityClass::Regular=>"certified_regular",CurveRegularityClass::Singular=>"singular",CurveRegularityClass::Unresolved=>"unresolved"},"derivativeNumeratorBounds":derivative_numerator_bounds})
            }
        }
    }
}
impl Serialize for CurveSpanCertificate {
    fn to_value(&self) -> Value {
        json!({"domain":self.domain,"min":self.min,"max":self.max,"denominatorLower":self.denominator[0],"denominatorUpper":self.denominator[1],"regularity":self.regularity})
    }
}
impl Serialize for CurveCertificate {
    fn to_value(&self) -> Value {
        json!({"version":"nurbs-foundation/1","kind":"curve","rounding":"binary64-nextafter-outward","basis":"positive-rational-convex-hull","periodic":self.periodic,"period":self.period,"spans":self.spans,"evidence":super::super::tolerance_evidence(&self.tolerance)})
    }
}
impl Serialize for SurfaceNormalRegularity {
    fn to_value(&self) -> Value {
        let classification = match self.classification {
            SurfaceRegularityClass::Singular => "singular",
            SurfaceRegularityClass::SingularOrUnresolved => "singular_or_unresolved",
            SurfaceRegularityClass::PlanarRegular => "certified_planar_regular",
            SurfaceRegularityClass::Regular => "certified_regular",
            SurfaceRegularityClass::Unresolved => "unresolved",
        };
        let mut result = json!({"classification":classification,"normalNumeratorBounds":self.normal_numerator_bounds});
        match self.classification {
            SurfaceRegularityClass::Singular => {
                result["method"] = json!("homogeneous-normal-Bernstein-exact-zero")
            }
            SurfaceRegularityClass::SingularOrUnresolved => {
                result["reason"] = json!("corner-normal-unavailable")
            }
            SurfaceRegularityClass::Regular | SurfaceRegularityClass::PlanarRegular => {
                result["method"] = json!("homogeneous-normal-Bernstein-component-separation")
            }
            SurfaceRegularityClass::Unresolved => {}
        }
        if let Some(normals) = &self.corner_normals {
            result["cornerNormals"] = json!(normals);
        }
        result
    }
}
impl Serialize for SurfaceCellCertificate {
    fn to_value(&self) -> Value {
        json!({"domainU":self.domain_u,"domainV":self.domain_v,"min":self.min,"max":self.max,"denominatorLower":self.denominator[0],"denominatorUpper":self.denominator[1],"normalRegularity":self.normal_regularity})
    }
}
impl Serialize for SingularRegion {
    fn to_value(&self) -> Value {
        let mut result = json!({"parameterBox":self.parameter_box});
        if let Some(bounds) = &self.normal_numerator_bounds {
            result["normalNumeratorBounds"] = json!(bounds);
        }
        if let Some(point) = self.point {
            result["point"] = json!(point);
        }
        if let Some(class) = self.classification {
            result["classification"] = json!(match class {
                NormalBoxClass::SingularPatch => "singular_patch",
                NormalBoxClass::IsolatedSingularPoint => "isolated_singular_point",
                NormalBoxClass::Regular => "certified_regular",
                NormalBoxClass::Unresolved => "unresolved",
                NormalBoxClass::ResourceExhausted => "resource_exhausted",
                NormalBoxClass::TrimFailed => "trim_failed",
            });
        }
        result
    }
}
impl Serialize for SingularityLocalization {
    fn to_value(&self) -> Value {
        json!({"method":"recursive-sub-knot-cell-normal-cone","complete":self.complete(),"resourceLimit":MAX_CERTIFICATE_CELLS,"workCells":self.work_cells,"isolatedSingularPoints":self.isolated_points,"singularCurves":self.singular_curves,"regularComplement":self.regular_complement,"unresolved":self.unresolved})
    }
}
impl Serialize for SurfaceCertificate {
    fn to_value(&self) -> Value {
        json!({"version":"nurbs-foundation/1","kind":"surface","rounding":"binary64-nextafter-outward","basis":"positive-rational-convex-hull","periodicU":self.periodic_u,"periodicV":self.periodic_v,"periodU":self.period_u,"periodV":self.period_v,"cells":self.cells,"singularityLocalization":self.singularity_localization,"evidence":super::super::tolerance_evidence(&self.tolerance)})
    }
}
