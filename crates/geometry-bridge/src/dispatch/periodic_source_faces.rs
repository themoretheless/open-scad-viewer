use crate::*;
use brep_core::profile_certificate::{
    OriginalPeriodicChartCycleEdgeKind, OriginalPeriodicSourceUse, PeriodicSewnFaceLimits,
    sew_periodic_source_face,
};
use nurbs_core::curve::Curve;

pub(super) fn sew(v: &Value) -> Result<Value> {
    let surface = field(v, "surface")?;
    let loops: Vec<Vec<Curve>> = field(v, "loops")?;
    let wrapped = match v["wrapped_axes"].as_str() {
        Some("u") => [true, false],
        Some("v") => [false, true],
        Some("uv") => [true, true],
        _ => {
            return Err(Error::new(
                "BREP_PERIODIC_FACE_INPUT",
                "Choose u, v or uv wrapped axes",
            ));
        }
    };
    let max_work: usize = field(v, "max_work")?;
    let max_edges: usize = field(v, "max_edges")?;
    let mut curves = Vec::new();
    let contours: Vec<Vec<_>> = loops
        .into_iter()
        .map(|wire| {
            wire.into_iter()
                .map(|curve| {
                    let source = curves.len();
                    curves.push(curve);
                    OriginalPeriodicSourceUse {
                        source,
                        reversed: false,
                    }
                })
                .collect()
        })
        .collect();
    let owned = sew_periodic_source_face(
        &surface,
        &curves,
        &contours,
        wrapped,
        field(v, "base_uv")?,
        field(v, "base_winding")?,
        field(v, "probe_radius_uv")?,
        max_work,
        PeriodicSewnFaceLimits {
            max_edges,
            max_seam_comparisons: max_work,
            ..Default::default()
        },
    )?;
    let vertices: Vec<_> = owned
        .vertices()
        .iter()
        .map(|vertex| {
            json!({
                "fans":vertex.fans, "chartVertices":vertex.chart_vertices
            })
        })
        .collect();
    let edges: Vec<_> = owned
        .edges()
        .iter()
        .map(|edge| {
            let original = match edge.original {
                OriginalPeriodicChartCycleEdgeKind::NativeShore { shore, interval } => {
                    json!({"kind":"native_shore","shore":shore,"interval":interval})
                }
                OriginalPeriodicChartCycleEdgeKind::SourceTrace(trace) => {
                    json!({"kind":"original_trace","trace":trace})
                }
            };
            json!({"vertices":edge.vertices,"chartEdges":edge.chart_edges,"original":original})
        })
        .collect();
    let coedges: Vec<_> = owned.coedges().iter().map(|edge| json!({
        "patch":edge.patch,"edge":edge.edge,"chartEdge":edge.chart_edge,"reversed":edge.reversed
    })).collect();
    let faces: Vec<_> = owned
        .faces()
        .iter()
        .map(|face| {
            json!({
                "patches":face.patches,"boundaryCycles":face.boundary_cycles,
                "eulerCharacteristic":face.euler_characteristic,"genus":face.genus
            })
        })
        .collect();
    Ok(json!({
        "vertices":vertices,"edges":edges,"coedges":coedges,"faces":faces,
        "seams":owned.seams(),"sourceBound":true,
        "previewRole":"support_surface","solidBodyAuthored":false
    }))
}
