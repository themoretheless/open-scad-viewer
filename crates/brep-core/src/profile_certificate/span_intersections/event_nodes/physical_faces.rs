//! Physical trim faces with original-source implicit world trace geometry.
use super::*;
use nurbs_core::surface::Surface;
use std::{collections::BTreeMap, sync::Arc};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPhysicalVertex {
    pub chart_group: usize,
    pub uv_aliases: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPhysicalTrace {
    pub carrier: usize,
    pub source: usize,
    pub cuts: [OriginalCarrierCut; 2],
    /// Direction of this UV trace use relative to increasing original parameter.
    pub reversed: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPhysicalEdgeUse {
    pub face: usize,
    pub patch: usize,
    pub boundary: usize,
    pub coedge: usize,
    pub reversed: bool,
    pub halfedge: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPhysicalEdge {
    pub vertices: [usize; 2],
    pub traces: Vec<OriginalPhysicalTrace>,
    pub uses: Vec<OriginalPhysicalEdgeUse>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPhysicalFacePatch {
    pub region: usize,
    pub outer: usize,
    pub holes: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalPhysicalFace {
    pub patches: Vec<usize>,
    pub boundary_cycles: Vec<usize>,
    pub euler_characteristic: i64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Geometry {
    vertices: Vec<OriginalPhysicalVertex>,
    edges: Vec<OriginalPhysicalEdge>,
    patches: Vec<OriginalPhysicalFacePatch>,
    faces: Vec<OriginalPhysicalFace>,
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalPhysicalFaces {
    embedding: Arc<OwnedOriginalGlobalChartEmbedding>,
    geometry: Geometry,
}
fn derive(embedding: &OwnedOriginalGlobalChartEmbedding) -> Result<Geometry> {
    embedding.reverify_source()?;
    let topology = embedding.pairs().atlas().regularity().coverage().topology();
    let dictionary = topology.seams().dictionary();
    let regions = dictionary.regions();
    let chart = dictionary.chart();
    let owner = chart.graph().owner();
    let mut vertex_ids = BTreeMap::new();
    let mut vertices = Vec::new();
    for link in topology.vertex_links() {
        if vertex_ids.insert(link.vertex, vertices.len()).is_some() {
            return Err(refused("Physical trim repeats an original quotient vertex"));
        }
        vertices.push(OriginalPhysicalVertex {
            chart_group: link.vertex,
            uv_aliases: chart.groups()[link.vertex].clone(),
        });
    }
    let mut edges = Vec::new();
    for edge in topology.edges() {
        let vertices = [
            *vertex_ids
                .get(&edge.vertices[0])
                .ok_or_else(|| refused("Physical edge start has no actual quotient vertex"))?,
            *vertex_ids
                .get(&edge.vertices[1])
                .ok_or_else(|| refused("Physical edge end has no actual quotient vertex"))?,
        ];
        let traces = edge
            .halfedges
            .iter()
            .map(|h| {
                let carrier = h / 2;
                let c = &owner.carriers()[carrier];
                OriginalPhysicalTrace {
                    carrier,
                    source: c.source,
                    cuts: c.cuts.clone(),
                    reversed: h % 2 != 0,
                }
            })
            .collect();
        edges.push(OriginalPhysicalEdge {
            vertices,
            traces,
            uses: Vec::new(),
        });
    }
    let patches = regions
        .regions()
        .iter()
        .enumerate()
        .map(|(region, r)| OriginalPhysicalFacePatch {
            region,
            outer: r.outer,
            holes: r.holes.clone(),
        })
        .collect::<Vec<_>>();
    let mut patch_faces = vec![None; patches.len()];
    let mut faces = Vec::new();
    for (face, component) in topology.components().iter().enumerate() {
        for &patch in component {
            if patch >= patches.len() || patch_faces[patch].replace(face).is_some() {
                return Err(refused(
                    "Physical patch has invalid or repeated face ownership",
                ));
            }
        }
        faces.push(OriginalPhysicalFace {
            patches: component.clone(),
            boundary_cycles: Vec::new(),
            euler_characteristic: topology.euler_characteristics()[face],
        });
    }
    if patch_faces.iter().any(Option::is_none) {
        return Err(refused("Physical trim omits an actual material patch"));
    }
    let mut boundary_patch = vec![None; topology.loops().len()];
    let mut halfedge_faces = BTreeMap::new();
    for (patch, p) in patches.iter().enumerate() {
        let face = patch_faces[patch].unwrap();
        for boundary in std::iter::once(p.outer).chain(p.holes.iter().copied()) {
            if boundary_patch[boundary].replace(patch).is_some() {
                return Err(refused("Physical trim loop belongs to multiple patches"));
            }
            for (coedge, c) in topology.loops()[boundary].iter().enumerate() {
                if halfedge_faces.insert(c.halfedge, face).is_some() {
                    return Err(refused("Physical trim repeats an original geometry use"));
                }
                edges[c.edge].uses.push(OriginalPhysicalEdgeUse {
                    face,
                    patch,
                    boundary,
                    coedge,
                    reversed: c.reversed,
                    halfedge: c.halfedge,
                });
            }
        }
    }
    if boundary_patch.iter().any(Option::is_none)
        || edges
            .iter()
            .any(|e| e.uses.len() != e.traces.len() || e.uses.is_empty() || e.uses.len() > 2)
    {
        return Err(refused("Physical edge/loop use census is incomplete"));
    }
    for edge in &edges {
        let trace_ids = edge
            .traces
            .iter()
            .map(|t| 2 * t.carrier + usize::from(t.reversed))
            .collect::<std::collections::BTreeSet<_>>();
        let use_ids = edge
            .uses
            .iter()
            .map(|u| u.halfedge)
            .collect::<std::collections::BTreeSet<_>>();
        if trace_ids != use_ids || trace_ids.len() != edge.traces.len() {
            return Err(refused(
                "Physical edge use does not retain its actual original parameter trace",
            ));
        }
    }
    for (cycle, boundary) in topology.boundaries().iter().enumerate() {
        let face = *halfedge_faces
            .get(&boundary[0])
            .ok_or_else(|| refused("Physical boundary has no actual face use"))?;
        if boundary
            .iter()
            .any(|h| halfedge_faces.get(h) != Some(&face))
        {
            return Err(refused("Physical boundary crosses unrelated source faces"));
        }
        faces[face].boundary_cycles.push(cycle);
    }
    Ok(Geometry {
        vertices,
        edges,
        patches,
        faces,
    })
}
impl OwnedOriginalPhysicalFaces {
    pub fn embedding(&self) -> &Arc<OwnedOriginalGlobalChartEmbedding> {
        &self.embedding
    }
    pub fn surface(&self) -> &Surface {
        self.embedding
            .pairs()
            .atlas()
            .regularity()
            .coverage()
            .topology()
            .seams()
            .surface()
    }
    /// Each edge world trace is EXACTLY S(original_pcurve(t)) on its owned
    /// algebraic cuts. These original curves are never rebuilt or fitted.
    pub fn sources(&self) -> &[Curve] {
        self.embedding
            .pairs()
            .atlas()
            .regularity()
            .coverage()
            .topology()
            .seams()
            .dictionary()
            .chart()
            .graph()
            .owner()
            .atlas()
            .sources()
    }
    pub fn vertices(&self) -> &[OriginalPhysicalVertex] {
        &self.geometry.vertices
    }
    /// Uses keep their own original parameter clocks; alternate traces share
    /// a proved world image, not an assumed linear parameter correspondence.
    pub fn edges(&self) -> &[OriginalPhysicalEdge] {
        &self.geometry.edges
    }
    pub fn patches(&self) -> &[OriginalPhysicalFacePatch] {
        &self.geometry.patches
    }
    pub fn faces(&self) -> &[OriginalPhysicalFace] {
        &self.geometry.faces
    }
    pub fn loops(&self) -> &[Vec<OriginalQuotientCoedge>] {
        self.embedding
            .pairs()
            .atlas()
            .regularity()
            .coverage()
            .topology()
            .loops()
    }
    pub fn boundary_cycles(&self) -> &[Vec<usize>] {
        self.embedding
            .pairs()
            .atlas()
            .regularity()
            .coverage()
            .topology()
            .boundaries()
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(&self.embedding)? != self.geometry {
            return Err(refused(
                "Physical original trim face geometry or ownership changed",
            ));
        }
        Ok(())
    }
}
/// Publish physical face geometry only after original closed material regions,
/// chart coverage, actual surface/seam identity, nonzero source normals,
/// manifold quotient and COMPLETE global chart pair proof. World geometry is
/// exact source-function composition with algebraic cuts, not fitted 3D poles.
/// Legacy floating Curve/Model conversion, STEP and tessellation are separate
/// consumers and gain no unproved approximation authority from this owner.
pub fn own_original_physical_faces(
    embedding: Arc<OwnedOriginalGlobalChartEmbedding>,
) -> Result<OwnedOriginalPhysicalFaces> {
    let geometry = derive(&embedding)?;
    Ok(OwnedOriginalPhysicalFaces {
        embedding,
        geometry,
    })
}
#[derive(Clone, Copy, Debug)]
pub struct OriginalPhysicalFaceLimits {
    pub boundary: OriginalBoundaryAssemblyLimits,
    pub containment: ParameterOrderLimits,
    pub vertices: OriginalChartVertexLimits,
    pub coverage: ParameterOrderLimits,
    pub regularity_cells: usize,
    pub atlas: OriginalInjectiveAtlasLimits,
    pub pairs: OriginalAtlasPairLimits,
}
impl Default for OriginalPhysicalFaceLimits {
    fn default() -> Self {
        Self {
            boundary: Default::default(),
            containment: Default::default(),
            vertices: Default::default(),
            coverage: Default::default(),
            regularity_cells: 4096,
            atlas: Default::default(),
            pairs: Default::default(),
        }
    }
}
pub fn assemble_original_physical_faces(
    surface: &Surface,
    loops: &[Vec<Curve>],
    coefficients: &[i32],
    wrapped: [bool; 2],
    width: f64,
    limits: OriginalPhysicalFaceLimits,
) -> Result<OwnedOriginalPhysicalFaces> {
    surface.validate()?;
    let chart = [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ];
    let regions = Arc::new(assemble_original_filled_regions(
        loops,
        coefficients,
        width,
        limits.boundary,
        limits.containment,
    )?);
    let vertices = Arc::new(own_original_chart_vertices(
        regions.owner().cycles().graph().clone(),
        chart,
        wrapped,
        limits.vertices,
    )?);
    let dictionary = Arc::new(own_original_seam_dictionary(
        regions,
        vertices,
        limits.coverage,
    )?);
    let seams = Arc::new(bind_original_seams_to_surface(dictionary, surface)?);
    let topology = Arc::new(own_original_periodic_topology(seams)?);
    let coverage = Arc::new(own_original_trim_chart_coverage(topology, limits.coverage)?);
    let regularity = Arc::new(own_original_regular_trim_surface(
        coverage,
        limits.regularity_cells,
    )?);
    let atlas = Arc::new(own_original_injective_chart_atlas(
        regularity,
        limits.atlas,
    )?);
    let pairs = Arc::new(own_original_atlas_pairs(atlas, limits.pairs)?);
    own_original_physical_faces(Arc::new(own_original_global_chart_embedding(pairs)?))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_geometry_replay_rejects_source_traces_uses_faces_and_vertex_aliases() {
        let v = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
        let wire = v
            .windows(2)
            .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
            .collect();
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            weights: vec![vec![1.; 2]; 2],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            periodic_u: false,
            periodic_v: false,
        };
        let f = assemble_original_physical_faces(
            &surface,
            &[wire],
            &[1],
            [false, false],
            1e-12,
            Default::default(),
        )
        .unwrap();
        f.reverify_source().unwrap();
        let mut bad = f.clone();
        bad.geometry.vertices[0].uv_aliases.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = f.clone();
        bad.geometry.edges[0].traces[0].source = 99;
        assert!(bad.reverify_source().is_err());
        let mut bad = f.clone();
        bad.geometry.edges[0].uses[0].reversed = !bad.geometry.edges[0].uses[0].reversed;
        assert!(bad.reverify_source().is_err());
        let mut bad = f.clone();
        bad.geometry.faces[0].patches.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = f;
        bad.geometry.faces[0].boundary_cycles.clear();
        assert!(bad.reverify_source().is_err());
    }
}
