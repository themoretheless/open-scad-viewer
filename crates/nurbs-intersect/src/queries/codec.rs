use super::*;
impl value_codec::Serialize for Plane {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"normal":self.normal,"offset":self.offset})
    }
}
impl<'de> value_codec::Deserialize<'de> for Plane {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        Ok(Self {
            normal: value_codec::from_value(value["normal"].clone())?,
            offset: value_codec::from_value(value["offset"].clone())?,
        })
    }
}
impl value_codec::Serialize for Options {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"distanceTolerance":self.distance_tolerance,"parameterTolerance":self.parameter_tolerance,
            "maxDepth":self.max_depth,"maxBoxes":self.max_boxes})
    }
}
impl<'de> value_codec::Deserialize<'de> for Options {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        if !value.is_object() {
            return Err(value_codec::error("Intersection options must be an object"));
        }
        let mut options = Self::default();
        if let Some(v) = value.get("distanceTolerance") {
            options.distance_tolerance = value_codec::from_value(v.clone())?;
        }
        if let Some(v) = value.get("parameterTolerance") {
            options.parameter_tolerance = value_codec::from_value(v.clone())?;
        }
        if let Some(v) = value.get("maxDepth") {
            options.max_depth = value_codec::from_value(v.clone())?;
        }
        if let Some(v) = value.get("maxBoxes") {
            options.max_boxes = value_codec::from_value(v.clone())?;
        }
        Ok(options)
    }
}
impl value_codec::Serialize for Unresolved {
    fn to_value(&self) -> value_codec::Value {
        let reason = match self.reason {
            UnresolvedReason::BudgetExceeded => "budget_exceeded",
            UnresolvedReason::TangencyOrMultipleRoot => "tangency_or_multiple_root",
            UnresolvedReason::NearCoincidence => "near_coincidence",
            UnresolvedReason::BoundaryCrossing => "boundary_crossing",
            UnresolvedReason::UnsupportedSurface => "unsupported_surface",
            UnresolvedReason::CoincidentTrim => "coincident_trim",
        };
        value_codec::json!({"parameterBox":self.parameter_box,"reason":reason})
    }
}
impl<T: value_codec::Serialize> value_codec::Serialize for Report<T> {
    fn to_value(&self) -> value_codec::Value {
        let coverage = match self.coverage {
            Coverage::Complete => "complete",
            Coverage::NumericallyResolved => "numerically_resolved",
            Coverage::Incomplete => "incomplete",
        };
        let evidence = match self.coverage {
            Coverage::Complete => "analytic_coverage_certified",
            Coverage::NumericallyResolved => "numerical_uncertified",
            Coverage::Incomplete => "incomplete",
        };
        value_codec::json!({"components":self.components,"unresolved":self.unresolved,"boxesVisited":self.boxes_visited,
            "bernsteinExcluded":self.bernstein_excluded,"coverage":coverage,"permitsTopologyChange":false,
            "evidence":evidence})
    }
}
impl value_codec::Serialize for CurvePoint {
    fn to_value(&self) -> value_codec::Value {
        let contact = match self.contact {
            Contact::Transverse => "transverse",
            Contact::Boundary => "boundary",
        };
        value_codec::json!({"parameter":self.parameter,"parameterInterval":self.parameter_interval,"point":self.point,
            "planeResidual":self.plane_residual,"contact":contact})
    }
}
impl value_codec::Serialize for CurvePlaneComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Point(point) => value_codec::json!({"kind":"point","curve":point}),
            Self::Overlap {
                parameter_interval,
                control_residual,
            } => {
                value_codec::json!({"kind":"overlap","parameterInterval":parameter_interval,"controlResidual":control_residual})
            }
        }
    }
}
impl value_codec::Serialize for SurfacePoint {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"uv":self.uv,"point":self.point,"planeResidual":self.plane_residual})
    }
}
impl value_codec::Serialize for SurfaceTrace {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::RuledU {
                surface,
                plane,
                v_interval,
            } => {
                value_codec::json!({"kind":"ruled_u","surface":surface,"plane":plane,"vInterval":v_interval})
            }

            Self::Line {
                surface,
                plane,
                start,
                end,
            } => {
                value_codec::json!({"kind":"line","surface":surface,"plane":plane,"start":start,"end":end})
            }
            Self::Ruled {
                surface,
                plane,
                u_interval,
            } => {
                value_codec::json!({"kind":"ruled","surface":surface,"plane":plane,"uInterval":u_interval})
            }
        }
    }
}
impl<'de> value_codec::Deserialize<'de> for SurfaceTrace {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let surface = value_codec::from_value(value["surface"].clone())?;
        let plane = value_codec::from_value(value["plane"].clone())?;
        match value["kind"].as_str() {
            Some("line") => Ok(Self::Line {
                surface,
                plane,
                start: value_codec::from_value(value["start"].clone())?,
                end: value_codec::from_value(value["end"].clone())?,
            }),
            Some("ruled_u") => Ok(Self::RuledU {
                surface,
                plane,
                v_interval: value_codec::from_value(value["vInterval"].clone())?,
            }),
            Some("ruled") => Ok(Self::Ruled {
                surface,
                plane,
                u_interval: value_codec::from_value(value["uInterval"].clone())?,
            }),
            _ => Err(value_codec::error(
                "Unknown surface intersection trace kind",
            )),
        }
    }
}
impl value_codec::Serialize for SurfacePlaneComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Curve {
                trace,
                parameter_box,
                samples,
                max_sample_residual,
            } => {
                value_codec::json!({"kind":"curve","trace":trace,"parameterBox":parameter_box,"samples":samples,"maxSampleResidual":max_sample_residual})
            }
            Self::Point(point) => value_codec::json!({"kind":"point","surface":point}),
            Self::Overlap {
                parameter_box,
                control_residual,
            } => {
                value_codec::json!({"kind":"overlap","parameterBox":parameter_box,"controlResidual":control_residual})
            }
        }
    }
}
impl value_codec::Serialize for SurfaceSurfaceComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Overlap {
                first_boundary,
                second_boundary,
                points,
                max_sample_residual,
            } => {
                value_codec::json!({"kind":"overlap","firstBoundary":first_boundary,"secondBoundary":second_boundary,"points":points,"maxSampleResidual":max_sample_residual})
            }
            Self::Point {
                first,
                second,
                residual,
            } => {
                value_codec::json!({"kind":"point","first":first,"second":second,"residual":residual})
            }
            Self::Curve {
                first,
                second,
                max_sample_residual,
            } => {
                value_codec::json!({"kind":"curve","first":first,"second":second,"maxSampleResidual":max_sample_residual})
            }
        }
    }
}
impl value_codec::Serialize for CurveSegmentComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Point {
                curve,
                segment_parameter,
                line_residual,
            } => {
                value_codec::json!({"kind":"point","curve":curve,"segmentParameter":segment_parameter,"lineResidual":line_residual})
            }
            Self::Overlap { curve_interval } => {
                value_codec::json!({"kind":"overlap","curveInterval":curve_interval})
            }
        }
    }
}
impl value_codec::Serialize for CurveCurveComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Point {
                first,
                first_interval,
                second,
                second_interval,
                point,
                residual,
                contact,
            } => {
                let contact = match contact {
                    Contact::Transverse => "transverse",
                    Contact::Boundary => "boundary",
                };
                value_codec::json!({"kind":"point","first":first,"firstInterval":first_interval,
                    "second":second,"secondInterval":second_interval,"point":point,
                    "residual":residual,"contact":contact})
            }
            Self::Overlap {
                first_interval,
                second_interval,
                reversed,
                max_control_residual,
            } => {
                value_codec::json!({"kind":"overlap","firstInterval":first_interval,
                    "secondInterval":second_interval,"reversed":reversed,
                    "maxControlResidual":max_control_residual})
            }
        }
    }
}
impl value_codec::Serialize for CurveSurfaceComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Point {
                curve,
                uv,
                surface_residual,
            } => {
                value_codec::json!({"kind":"point","curve":curve,"uv":uv,"surfaceResidual":surface_residual})
            }
            Self::Overlap { curve_interval } => {
                value_codec::json!({"kind":"overlap","curveInterval":curve_interval})
            }
        }
    }
}

impl value_codec::Serialize for CurveRuledSurfaceComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Point {
                t,
                t_interval,
                uv,
                uv_box,
                point,
                residual,
                contact,
            } => {
                let contact = match contact {
                    Contact::Transverse => "transverse",
                    Contact::Boundary => "boundary",
                };
                value_codec::json!({"kind":"point","t":t,"tInterval":t_interval,
                    "uv":uv,"uvBox":uv_box,"point":point,
                    "residual":residual,"contact":contact})
            }
            Self::Overlap {
                curve_interval,
                uv_start,
                uv_end,
                max_control_residual,
                seam_wrap,
                correspondence,
            } => {
                value_codec::json!({"kind":"overlap","curveInterval":curve_interval,
                    "uvStart":uv_start,"uvEnd":uv_end,
                    "maxControlResidual":max_control_residual,
                    "seamWrap":seam_wrap,"correspondence":correspondence})
            }
        }
    }
}
impl value_codec::Serialize for OverlapCorrespondence {
    fn to_value(&self) -> value_codec::Value {
        let kind = match self.kind {
            OverlapCorrespondenceKind::MobiusV => "mobius_v",
            OverlapCorrespondenceKind::AffineU => "affine_u",
        };
        value_codec::json!({"kind":kind,"samples":self.samples})
    }
}
