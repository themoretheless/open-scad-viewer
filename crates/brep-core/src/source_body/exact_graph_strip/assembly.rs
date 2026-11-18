//! Assemble a strip from certified transition panels and their shared edges.
use super::*;

/// Per-panel curvature fields and internal G2 joins of consecutive panels.
fn certify_panels(
    panels: &[Patch],
    budget: &mut Budget,
) -> Result<(Vec<CurvatureReport>, Vec<JoinReport>)> {
    let mut fields = Vec::new();
    for p in panels {
        require(
            p.contact_jets() == [true; 2],
            "Source contact two-jets are unproven",
        )?;
        let field = p.inspect_curvature(MAX_CURVATURE_PER_MM, CURVATURE_SAMPLES, budget)?;
        require(
            field.decision == CurvatureDecision::Within,
            "Exact panel field does not pass 10/mm curvature",
        )?;
        fields.push(field);
    }
    let mut joins = Vec::new();
    for pair in panels.windows(2) {
        let join =
            pair[0].inspect_join_g2(&pair[1], G2_POSITION_TOLERANCE, G2_NORMAL_TOLERANCE, budget)?;
        require(
            join.within && join.position_exact,
            "Exact transition internal G2 join is unproven",
        )?;
        joins.push(join);
    }
    Ok((fields, joins))
}
type StripVertices = Vec<brep_topology::Vertex<[Q; 3]>>;
type StripEdges = Vec<brep_topology::Edge<Edge>>;
/// Station vertices (two per station) and station edges, in panel order.
fn station_edges(panels: &[Patch], budget: &mut Budget) -> Result<(StripVertices, StripEdges)> {
    let mut stations = Vec::new();
    if let Some(first) = panels.first() {
        stations.push(first.boundary_definition(Boundary::UMin, budget)?);
    }
    for p in panels {
        stations.push(p.boundary_definition(Boundary::UMax, budget)?);
    }
    let mut vertices = Vec::new();
    let mut edges = Vec::new();
    for station in &stations {
        let (Some(start), Some(end)) = (station.points.first(), station.points.last()) else {
            return Err(Error::new(
                "BREP_EXACT_GRAPH_STRIP",
                "Transition station has no control points",
            ));
        };
        let first = vertices.len();
        vertices.push(brep_topology::Vertex {
            point: start.clone(),
        });
        vertices.push(brep_topology::Vertex { point: end.clone() });
        edges.push(brep_topology::Edge {
            degenerate: false,
            vertices: [first, first + 1],
            curve: Edge::original(station.points.clone(), edges.len()),
        });
    }
    Ok((vertices, edges))
}
/// Appends panel `i`'s bottom/top edges and its four-coedge outer loop.
fn panel_loop(
    i: usize,
    p: &Patch,
    edges: &mut StripEdges,
    budget: &mut Budget,
) -> Result<brep_topology::Loop<ParameterLine>> {
    let bottom = p.boundary_definition(Boundary::VMin, budget)?;
    let top = p.boundary_definition(Boundary::VMax, budget)?;
    let left = p.boundary_definition(Boundary::UMin, budget)?;
    let right = p.boundary_definition(Boundary::UMax, budget)?;
    let bottom_id = edges.len();
    edges.push(brep_topology::Edge {
        degenerate: false,
        vertices: [2 * i, 2 * i + 2],
        curve: Edge::original(bottom.points.clone(), edges.len()),
    });
    let top_id = edges.len();
    edges.push(brep_topology::Edge {
        degenerate: false,
        vertices: [2 * i + 1, 2 * i + 3],
        curve: Edge::original(top.points.clone(), edges.len()),
    });
    Ok(brep_topology::Loop {
        coedges: vec![
            brep_topology::Coedge::new(bottom_id, false, line(&bottom, false)),
            brep_topology::Coedge::new(i + 1, false, line(&right, false)),
            brep_topology::Coedge::new(top_id, true, line(&top, true)),
            brep_topology::Coedge::new(i, true, line(&left, true)),
        ],
    })
}
/// Fresh source-contact, internal G2, whole-panel 10/mm curvature and exact
/// canonical-boundary checks before publishing an immutable OPEN strip owner.
/// Sharing uses construction identity, never nearest-position edge welding.
pub fn assemble(panels: Vec<Patch>, tolerance_mm: f64, budget: &mut Budget) -> Result<Strip> {
    require(
        !panels.is_empty() && panels.len() <= MAX_PANELS,
        "Choose 1..64 exact transition panels",
    )?;
    require(
        tolerance_mm.is_finite() && TOLERANCE_MM.contains(&tolerance_mm),
        "Choose a supported B-rep tolerance",
    )?;
    let (fields, joins) = certify_panels(&panels, budget)?;
    let n = panels.len();
    let (vertices, mut edges) = station_edges(&panels, budget)?;
    let mut loops = Vec::new();
    let mut faces = Vec::new();
    for (i, p) in panels.into_iter().enumerate() {
        loops.push(panel_loop(i, &p, &mut edges, budget)?);
        faces.push(brep_topology::Face {
            surface: Arc::new(p),
            outer: i,
            holes: vec![],
        });
    }
    let topology = Topology {
        vertices,
        edges,
        loops,
        faces,
        shells: vec![brep_topology::Shell {
            closed: false,
            faces: (0..n)
                .map(|face| brep_topology::FaceUse {
                    face,
                    reversed: false,
                })
                .collect(),
        }],
        bodies: vec![],
        tolerance_mm,
    };
    let geometry = inspect_geometry(&topology, budget)?;
    require(
        geometry.open_edges.len() == 2 * n + 2,
        "Unexpected open transition boundary",
    )?;
    Ok(Strip {
        topology,
        joins,
        fields,
        geometry,
    })
}
