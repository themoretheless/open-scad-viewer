//! Optional wire-format implementations, separate from native algorithms.
use super::*;

impl value_codec::Serialize for Mesh {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "positions".into(),
            value_codec::Serialize::to_value(&self.positions),
        );
        object.insert(
            "indices".into(),
            value_codec::Serialize::to_value(&self.indices),
        );
        if self.uv.is_some() {
            object.insert("uv".into(), value_codec::Serialize::to_value(&self.uv));
        }
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for Mesh {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let positions: Vec<f64> = value_codec::Deserialize::from_value(
            object
                .remove("positions")
                .ok_or_else(|| value_codec::error("Missing field positions"))?,
        )?;
        let indices: Vec<usize> = value_codec::Deserialize::from_value(
            object
                .remove("indices")
                .ok_or_else(|| value_codec::error("Missing field indices"))?,
        )?;
        let uv: Option<Vec<f64>> = if let Some(v) = object.remove("uv") {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        Ok(Self {
            positions,
            indices,
            uv,
        })
    }
}

impl value_codec::Serialize for Seams {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert("u".into(), value_codec::Serialize::to_value(&self.u));
        object.insert("v".into(), value_codec::Serialize::to_value(&self.v));
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for Seams {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let u: bool = value_codec::Deserialize::from_value(
            object
                .remove("u")
                .ok_or_else(|| value_codec::error("Missing field u"))?,
        )?;
        let v: bool = value_codec::Deserialize::from_value(
            object
                .remove("v")
                .ok_or_else(|| value_codec::error("Missing field v"))?,
        )?;
        Ok(Self { u, v })
    }
}

impl value_codec::Serialize for Construction {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::TriangleMesh => value_codec::Value::String("triangle_mesh".into()),
            Self::Boolean => value_codec::Value::String("boolean".into()),
            Self::SampledSurface => value_codec::Value::String("sampled_surface".into()),
            Self::FixedVectorThickening => {
                value_codec::Value::String("fixed_vector_thickening".into())
            }
        }
    }
}

impl<'de> value_codec::Deserialize<'de> for Construction {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        match value.as_str().unwrap_or("") {
            "triangle_mesh" => Ok(Self::TriangleMesh),
            "boolean" => Ok(Self::Boolean),
            "sampled_surface" => Ok(Self::SampledSurface),
            "fixed_vector_thickening" => Ok(Self::FixedVectorThickening),
            _ => Err(value_codec::error("Unknown enum variant")),
        }
    }
}

impl value_codec::Serialize for Report {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        if self.boolean.is_some() {
            object.insert(
                "boolean".into(),
                value_codec::Serialize::to_value(&self.boolean),
            );
        }
        object.insert(
            "triangleCount".into(),
            value_codec::Serialize::to_value(&self.triangle_count),
        );
        object.insert(
            "vertexCount".into(),
            value_codec::Serialize::to_value(&self.vertex_count),
        );
        object.insert(
            "boundaryEdges".into(),
            value_codec::Serialize::to_value(&self.boundary_edges),
        );
        object.insert(
            "nonManifoldEdges".into(),
            value_codec::Serialize::to_value(&self.non_manifold_edges),
        );
        object.insert(
            "orientationConflicts".into(),
            value_codec::Serialize::to_value(&self.orientation_conflicts),
        );
        object.insert(
            "degenerateTriangles".into(),
            value_codec::Serialize::to_value(&self.degenerate_triangles),
        );
        object.insert(
            "closed".into(),
            value_codec::Serialize::to_value(&self.closed),
        );
        object.insert(
            "signedVolumeMm3".into(),
            value_codec::Serialize::to_value(&self.signed_volume_mm3),
        );
        object.insert(
            "errorBoundCertified".into(),
            value_codec::Serialize::to_value(&self.error_bound_certified),
        );
        object.insert(
            "selfIntersectionStatus".into(),
            value_codec::Serialize::to_value(&self.self_intersection_status),
        );
        object.insert(
            "construction".into(),
            value_codec::Serialize::to_value(&self.construction),
        );
        if self.uv_area.is_some() {
            object.insert(
                "uvArea".into(),
                value_codec::Serialize::to_value(&self.uv_area),
            );
        }
        if self.sampled_deviation_mm.is_some() {
            object.insert(
                "sampledDeviationMm".into(),
                value_codec::Serialize::to_value(&self.sampled_deviation_mm),
            );
        }
        if self.parameter_seams_welded.is_some() {
            object.insert(
                "parameterSeamsWelded".into(),
                value_codec::Serialize::to_value(&self.parameter_seams_welded),
            );
        }
        if self.collapsed_boundary_count.is_some() {
            object.insert(
                "collapsedBoundaryCount".into(),
                value_codec::Serialize::to_value(&self.collapsed_boundary_count),
            );
        }
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for Report {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let boolean: Option<solid::boolean::BooleanReport> =
            if let Some(v) = object.remove("boolean") {
                value_codec::Deserialize::from_value(v)?
            } else {
                Default::default()
            };
        let triangle_count: usize = value_codec::Deserialize::from_value(
            object
                .remove("triangleCount")
                .ok_or_else(|| value_codec::error("Missing field triangleCount"))?,
        )?;
        let vertex_count: usize = value_codec::Deserialize::from_value(
            object
                .remove("vertexCount")
                .ok_or_else(|| value_codec::error("Missing field vertexCount"))?,
        )?;
        let boundary_edges: usize = value_codec::Deserialize::from_value(
            object
                .remove("boundaryEdges")
                .ok_or_else(|| value_codec::error("Missing field boundaryEdges"))?,
        )?;
        let non_manifold_edges: usize = value_codec::Deserialize::from_value(
            object
                .remove("nonManifoldEdges")
                .ok_or_else(|| value_codec::error("Missing field nonManifoldEdges"))?,
        )?;
        let orientation_conflicts: usize = value_codec::Deserialize::from_value(
            object
                .remove("orientationConflicts")
                .ok_or_else(|| value_codec::error("Missing field orientationConflicts"))?,
        )?;
        let degenerate_triangles: usize = value_codec::Deserialize::from_value(
            object
                .remove("degenerateTriangles")
                .ok_or_else(|| value_codec::error("Missing field degenerateTriangles"))?,
        )?;
        let closed: bool = value_codec::Deserialize::from_value(
            object
                .remove("closed")
                .ok_or_else(|| value_codec::error("Missing field closed"))?,
        )?;
        let signed_volume_mm3: f64 = value_codec::Deserialize::from_value(
            object
                .remove("signedVolumeMm3")
                .ok_or_else(|| value_codec::error("Missing field signedVolumeMm3"))?,
        )?;
        let error_bound_certified: bool = value_codec::Deserialize::from_value(
            object
                .remove("errorBoundCertified")
                .ok_or_else(|| value_codec::error("Missing field errorBoundCertified"))?,
        )?;
        let self_intersection_status: String = value_codec::Deserialize::from_value(
            object
                .remove("selfIntersectionStatus")
                .ok_or_else(|| value_codec::error("Missing field selfIntersectionStatus"))?,
        )?;
        let construction: Construction = value_codec::Deserialize::from_value(
            object
                .remove("construction")
                .ok_or_else(|| value_codec::error("Missing field construction"))?,
        )?;
        let uv_area: Option<f64> = if let Some(v) = object.remove("uvArea") {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        let sampled_deviation_mm: Option<f64> = if let Some(v) = object.remove("sampledDeviationMm")
        {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        let parameter_seams_welded: Option<Seams> =
            if let Some(v) = object.remove("parameterSeamsWelded") {
                value_codec::Deserialize::from_value(v)?
            } else {
                Default::default()
            };
        let collapsed_boundary_count: Option<usize> =
            if let Some(v) = object.remove("collapsedBoundaryCount") {
                value_codec::Deserialize::from_value(v)?
            } else {
                Default::default()
            };
        Ok(Self {
            boolean,
            triangle_count,
            vertex_count,
            boundary_edges,
            non_manifold_edges,
            orientation_conflicts,
            degenerate_triangles,
            closed,
            signed_volume_mm3,
            error_bound_certified,
            self_intersection_status,
            construction,
            uv_area,
            sampled_deviation_mm,
            parameter_seams_welded,
            collapsed_boundary_count,
        })
    }
}

impl value_codec::Serialize for BuiltMesh {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        if let value_codec::Value::Object(fields) = value_codec::Serialize::to_value(&self.mesh) {
            object.extend(fields);
        }
        object.insert(
            "report".into(),
            value_codec::Serialize::to_value(&self.report),
        );
        value_codec::Value::Object(object)
    }
}

impl<'de> value_codec::Deserialize<'de> for BuiltMesh {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let mesh: Mesh = value_codec::Deserialize::from_value(value.clone())?;
        let report: Report = value_codec::Deserialize::from_value(
            object
                .remove("report")
                .ok_or_else(|| value_codec::error("Missing field report"))?,
        )?;
        Ok(Self { mesh, report })
    }
}
