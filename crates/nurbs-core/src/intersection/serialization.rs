//! Host encoding of native curve intersection evidence.
use super::*;
use crate::resource;
use value_codec::{Value, json};

impl value_codec::Serialize for ContactClass {
    fn to_value(&self) -> Value {
        let label = match self {
            Self::Transverse => "transverse",
            Self::OddTangency => "odd_tangency",
            Self::EvenTangency => "even_tangency",
            Self::HigherOrderContact => "higher_order_contact",
            Self::UnresolvedConditioning => "unresolved_conditioning",
            Self::Coincident => "coincident",
            Self::PoleOrSingular => "pole_or_singular",
            Self::Boundary => "boundary",
            Self::NearCoincidence => "near_coincidence",
        };
        value_codec::Serialize::to_value(label)
    }
}

impl value_codec::Serialize for CurveSurfaceComponent {
    fn to_value(&self) -> Value {
        match self {
            Self::Point(p) => json!({"kind":"point","t":p.t,"tInterval":p.t_interval,
                "uv":p.uv,"uvBox":p.uv_box,"point":p.point,"residual":p.residual,
                "contactClass":p.contact,"multiplicity":p.multiplicity,"orientation":1,
                "seamWrap":0,"curveWrap":p.curve_wrap,"geometryEnclosure":p.geometry_enclosure,
                "parameterBox":p.parameter_box,"coedgeTrim":null}),
            Self::Overlap(p) => json!({"kind":"overlap","curveInterval":p.curve_interval,
                "uvStart":p.uv_start,"uvEnd":p.uv_end,"contactClass":"coincident",
                "multiplicity":null,"orientation":1,"seamWrap":0,"curveWrap":p.curve_wrap,
                "geometryEnclosure":p.geometry_enclosure,"coedgeTrim":p.coedge_trim,
                "correspondence":{"kind":"affine_uv","samples":p.samples}}),
        }
    }
}
impl value_codec::Serialize for UnresolvedCurveSurface {
    fn to_value(&self) -> Value {
        let parameter_box = match self.parameter_box {
            CurveSurfaceParameterBox::Curve(bounds) => value_codec::Serialize::to_value(&bounds),
            CurveSurfaceParameterBox::CurveSurface(bounds) => {
                value_codec::Serialize::to_value(&bounds)
            }
        };
        let reason = match self.reason {
            UnresolvedReason::ConditioningBoundary => "conditioning_boundary",
            UnresolvedReason::ResourceBoundary => "resource_boundary",
        };
        json!({"parameterBox":parameter_box,"reason":reason})
    }
}
impl value_codec::Serialize for CurveSurfaceIntersection {
    fn to_value(&self) -> Value {
        json!({"version":VERSION,"kind":"curve_surface",
            "coverage":{"method":"Bernstein-hull-exclusion-with-Krawczyk-or-plane-Bernstein",
                "complete":self.certified && self.unresolved.is_empty(),
                "searchComplete":self.unresolved.is_empty(),"certified":self.certified,"boxesVisited":self.boxes_visited,
                "bernsteinExcluded":self.bernstein_excluded,"krawczykIsolated":self.krawczyk_isolated,
                "resourceLimit":MAX_BOXES},"components":self.components,"unresolved":self.unresolved,
            "rounding":"uncertified-binary64","evidence":tolerance_evidence(&self.tolerance)})
    }
}
pub fn intersect_curve_surface(
    curve: &Curve,
    surface: &Surface,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    Ok(value_codec::Serialize::to_value(
        &intersect_curve_surface_report(curve, surface, tolerance)?,
    ))
}

impl value_codec::Serialize for UnresolvedCurveIntersection {
    fn to_value(&self) -> Value {
        let reason = match self.reason {
            UnresolvedReason::ConditioningBoundary => "conditioning_boundary",
            UnresolvedReason::ResourceBoundary => "resource_boundary",
        };
        let mut value = json!({"parameterBox":self.parameter_box,"reason":reason});
        if let Some(classification) = self.classification {
            value["classification"] = json!(classification);
        }
        value
    }
}

pub(super) fn encode_cc_report(
    report: CurveCurveReport,
    tolerance: &ToleranceContext,
    complete: bool,
) -> Value {
    let components: Vec<Value> = report
        .components
        .into_iter()
        .map(|c| {
            if c.kind == CurveCurveComponentKind::Overlap {
                json!({
                    "kind":"overlap",
                    "firstInterval":c.first_interval,
                    "secondInterval":c.second_interval,
                    "reversed":c.reversed,
                    "contactClass":c.contact,
                    "multiplicity":null,
                    "orientation":c.orientation,
                    "firstWrap":c.first_wrap,
                    "secondWrap":c.second_wrap,
                    "geometryEnclosure":c.enclosure,
                    "coedgeTrim":c.coedge_trim,
                    "maxControlResidual":c.residual
                })
            } else {
                json!({
                    "kind":"point",
                    "first":c.first,
                    "second":c.second,
                    "firstInterval":c.first_interval,
                    "secondInterval":c.second_interval,
                    "point":c.point,
                    "residual":c.residual,
                    "contactClass":c.contact,
                    "multiplicity":c.multiplicity,
                    "orientation":c.orientation,
                    "firstWrap":c.first_wrap,
                    "secondWrap":c.second_wrap,
                    "geometryEnclosure":c.enclosure,
                    "parameterBox":[c.first_interval[0],c.first_interval[1],c.second_interval[0],c.second_interval[1]]
                })
            }
        })
        .collect();
    json!({
        "version":VERSION,
        "kind":"curve_curve",
        "coverage":{
            "method":"Bernstein-hull-exclusion-with-Krawczyk-isolation",
            "complete":complete && report.unresolved.is_empty(),
            "boxesVisited":report.boxes_visited,
            "bernsteinExcluded":report.bernstein_excluded,
            "krawczykIsolated":report.krawczyk_isolated,
            "resourceLimit":MAX_BOXES
        },
        "components":components,
        "unresolved":report.unresolved,
        "rounding":"binary64-nextafter-outward",
        "evidence":tolerance_evidence(tolerance)
    })
}

pub fn intersect_curve_curve(
    first: &Curve,
    second: &Curve,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    let native = intersect_curve_curve_report(first, second, tolerance)?;
    let complete = native.report.unresolved.is_empty();
    Ok(encode_cc_report(native.report, &native.tolerance, complete))
}

pub(crate) fn tolerance_evidence(context: &ToleranceContext) -> Value {
    let spatial = context.spatial_bounds();
    json!({
        "toleranceIdentity": context.spec_identity(),
        "linearAbsoluteMm": spatial.absolute_mm,
        "linearRelative": spatial.relative,
        "parametricFloor": context.parametric_bounds().floor,
        "maxEntityErrorMm": context.entity_error_bounds().maximum_mm
    })
}

/// Resource-bound probe used by adversarial corpus generators.
pub fn resource_boundary_probe(degree: usize, controls: usize) -> Result<Value> {
    if degree == 0 || degree > MAX_DEGREE {
        return Err(resource("Degree outside admitted 1..25"));
    }
    if controls > MAX_CONTROLS {
        return Err(resource("Controls exceed 256"));
    }
    Ok(
        json!({"version":VERSION,"admitted":true,"maxDegree":MAX_DEGREE,"maxControls":MAX_CONTROLS,"maxBoxes":MAX_BOXES,"maxSpans":MAX_SPANS}),
    )
}
