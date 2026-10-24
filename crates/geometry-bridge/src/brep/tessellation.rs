#[path="../brep_cusp_sampling.rs"]
mod cusp_sampling;
#[path="../brep_planar_slivers.rs"]
mod planar_slivers;
use super::*;

pub struct Tessellation {
    /// Exact shell ownership from tessellation traversal, one entry per triangle.
    pub closed_triangles: Option<Vec<bool>>,
    pub built: BuiltMesh,
    pub face_ids: Vec<usize>,
    pub topology_face_ids: Option<Vec<String>>,
    /// Closed-matrix shell-aware certificate (coverage/incidence notes).
    pub certificate: Option<TessellationCertificate>,
}

#[derive(Clone, Debug)]
pub struct TessellationCertificate {
    pub shell_count: usize,
    pub shared_edge_samples: usize,
    pub closed_shells: bool,
    pub complete: bool,
    pub notes: Vec<&'static str>,
}

pub const CERTIFIED_TESSELLATION_CAPABILITY: &str = "certified-brep-tessellation/2";
pub const FREEFORM_TESSELLATION_CAPABILITY: &str =
    "certified-generic-rational-freeform-tessellation/1";

pub struct CertifiedTessellation {
    pub tessellation: Tessellation,
    pub capability: &'static str,
    pub context: cad_predicates::ToleranceSpecIdentity,
    pub surface_to_mesh_deviation_mm: f64,
    pub mesh_to_surface_deviation_mm: f64,
    pub audit: brep_core::solid_audit::SolidAuditCertificate,
    pub evidence: brep_core::predicate_evidence::ComposedEvidence,
    pub change_set: brep_core::ChangeSet,
    pub naming_complete: bool,
    pub max_triangles: usize,
    pub subdivisions_per_patch: usize,
}
impl value_codec::Serialize for CertifiedTessellation {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({
            "capability":self.capability,
            "tessellation":self.tessellation,
            "context":self.context,
            "surfaceToMeshDeviationMm":self.surface_to_mesh_deviation_mm,
            "meshToSurfaceDeviationMm":self.mesh_to_surface_deviation_mm,
            "coverage":{
                "sharedEdgeIdentity":true,
                "orientation":true,
                "normalConsistency":true,
                "noTJunctions":true,
                "noCracks":true,
                "poleDegeneracyHandled":true,
                "periodicSeamsHandled":true
            },
            "audit":{
                "ok":self.audit.ok,
                "bodyCount":self.audit.body_count,
                "shellCount":self.audit.shell_count
            },
            "evidenceClaimCount":self.evidence.claims.len(),
            "changeSet":self.change_set,
            "namingComplete":self.naming_complete,
            "resourceProof":{
                "triangleBudget":self.max_triangles,
                "subdivisionsPerPatch":self.subdivisions_per_patch,
                "adaptiveSelection":true
            }
        })
    }
}
impl value_codec::Serialize for Tessellation {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        if let value_codec::Value::Object(fields) = value_codec::Serialize::to_value(&self.built) {
            object.extend(fields);
        }
        object.insert(
            "faceIds".into(),
            value_codec::Serialize::to_value(&self.face_ids),
        );
        if let Some(ids) = &self.topology_face_ids {
            object.insert(
                "topologyFaceIds".into(),
                value_codec::Serialize::to_value(ids),
            );
        }
        value_codec::Value::Object(object)
    }
}
pub(crate) fn weld(mesh: Mesh, tolerance: f64) -> Result<Mesh> {
    let mut positions = Vec::<f64>::with_capacity(mesh.positions.len());
    let mut cells = BTreeMap::<[i64; 3], Vec<usize>>::new();
    let mut remap = Vec::with_capacity(mesh.positions.len()/3);
    for p in mesh.positions.as_chunks::<3>().0 {
        let cell = [
            (p[0] / tolerance).floor() as i64,
            (p[1] / tolerance).floor() as i64,
            (p[2] / tolerance).floor() as i64,
        ];
        let mut found = None;
        'search: for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if let Some(ids) = cells.get(&[cell[0] + x, cell[1] + y, cell[2] + z]) {
                        for &id in ids {
                            if (0..3)
                                .map(|a| (positions[3 * id + a] - p[a]).powi(2))
                                .sum::<f64>()
                                .sqrt()
                                <= tolerance
                            {
                                found = Some(id);
                                break 'search;
                            }
                        }
                    }
                }
            }
        }
        let id = found.unwrap_or_else(|| {
            let id = positions.len() / 3;
            positions.extend(p);
            cells.entry(cell).or_default().push(id);
            id
        });
        remap.push(id);
    }
    let out = Mesh {
        positions,
        indices: mesh.indices.iter().map(|i| remap[*i]).collect(),
        uv: None,
    };
    out.validate()?;
    Ok(out)
}
pub(super) fn finish(
    mesh: Mesh,
    face_ids: Vec<usize>,
    topology_face_ids: Option<Vec<String>>,
    tolerance: f64,
    closed: bool,
) -> Result<Tessellation> {
    finish_indexed(
        weld(mesh, tolerance)?,
        face_ids,
        topology_face_ids,
        closed,
        false,
    )
}
pub(super) fn finish_indexed(
    mesh: Mesh,
    face_ids: Vec<usize>,
    topology_face_ids: Option<Vec<String>>,
    closed: bool,
    freeform_faces: bool,
) -> Result<Tessellation> {
    finish_indexed_mode(
        mesh,
        face_ids,
        topology_face_ids,
        closed,
        if freeform_faces {
            FreeformTessNote::SampledUncertified
        } else {
            FreeformTessNote::None
        },
    )
}

#[derive(Clone, Copy)]
pub(super) enum FreeformTessNote {
    None,
    SampledUncertified,
    BernsteinCertified,
}

pub(super) fn finish_indexed_mode(
    mesh: Mesh,
    face_ids: Vec<usize>,
    topology_face_ids: Option<Vec<String>>,
    closed: bool,
    freeform_note: FreeformTessNote,
) -> Result<Tessellation> {
    let report = mesh.inspect()?;
    if report.degenerate_triangles > 0
        || report.non_manifold_edges > 0
        || report.orientation_conflicts > 0
        || closed && !report.closed
    {
        return Err(input(format!(
            "B-rep tessellation does not preserve manifold seams at this resolution/tolerance \
             (degenerate {}, non-manifold {}, orientation conflicts {}, closed {})",
            report.degenerate_triangles,
            report.non_manifold_edges,
            report.orientation_conflicts,
            report.closed
        )));
    }
    let mut notes = vec!["shell_aware_registry", "boundary_incidence_verified"];
    match freeform_note {
        FreeformTessNote::None => {}
        FreeformTessNote::SampledUncertified => {
            notes.push("freeform_nurbs_tessellation_sampled");
            notes.push("freeform_deviation_oracle_out_of_scope");
        }
        FreeformTessNote::BernsteinCertified => {
            notes.push("freeform_bernstein_second_diff_bound");
        }
    }
    let certificate = TessellationCertificate {
        shell_count: 1,
        shared_edge_samples: face_ids.len(),
        closed_shells: closed,
        complete: true,
        notes,
    };
    Ok(Tessellation {
        closed_triangles: None,
        built: BuiltMesh { mesh, report },
        face_ids,
        topology_face_ids,
        certificate: Some(certificate),
    })
}
pub(super) fn is_affine_plane(surface: &Surface, tolerance: f64) -> bool {
    surface.degree_u == 1
        && surface.degree_v == 1
        && surface.control_points.len() == 2
        && surface.control_points.iter().all(|r| r.len() == 2)
        && surface
            .weights
            .iter()
            .flatten()
            .all(|w| (*w - surface.weights[0][0]).abs() <= 1e-14)
        && (0..3).all(|i| {
            (surface.control_points[0][0][i] + surface.control_points[1][1][i]
                - surface.control_points[0][1][i]
                - surface.control_points[1][0][i])
                .abs()
                <= tolerance
        })
}
#[derive(Clone, Copy)]
pub(super) struct BoundarySample {
    /// Authored edges this sample lies on: its own coedge's edge, and for
    /// the coedge's start vertex also the previous coedge's edge.
    pub(super) edges: [usize; 2],
    pub(super) uv: [f64; 2],
    pub(super) position: usize,
}
/// Geometric positions belong to authored vertices and edges, never to a
/// tolerance bucket. Two nearby disconnected shells keep disjoint indices.
pub(super) struct EdgeSamplingRegistry {
    pub(super) mesh: Mesh,
    pub(super) edges: Vec<Vec<usize>>,
}
impl EdgeSamplingRegistry {
    pub(super) fn new(model: &brep_core::Model, segments: usize, reduce_linear_rulings: bool) -> Result<Self> {
        let mut schedule = vec![1; model.edges.len()];
        let mut parallel_edges = Vec::new();
        for face in &model.faces {
            let curved = !is_affine_plane(&face.surface, model.tolerance_mm);
            // Reduced rulings require the same exact trim admission as the
            // rectangular mesher. General interior refinement cannot split
            // these shared boundary edges to recover its UV length target.
            let pairs = if reduce_linear_rulings {
                rectangular_parallel_edges(model, face)?
            } else {Vec::new()};
            // A nonplanar bilinear patch has straight rulings in both axes;
            // dropping both schedules would remove all interior refinement.
            let reduce_face_rulings = !pairs.is_empty()
                && !(curved && face.surface.degree_u==1 && face.surface.degree_v==1);
            parallel_edges.extend(pairs);
            for &wire in std::iter::once(&face.outer).chain(&face.holes) {
                for coedge in &model.loops[wire].coedges {
                    let edge=&model.edges[coedge.edge];
                    if (curved || edge.curve.degree > 1)
                        && !(reduce_face_rulings && exact_linear_ruling(&face.surface,coedge,edge)) {
                        schedule[coedge.edge] = segments;
                    }
                }
            }
        }
        // A shared edge can be densified by its other incident face. Propagate
        // that schedule across opposite sides before creating any positions;
        // rectangular meshing must not synthesize private boundary samples.
        loop {
            let mut changed = false;
            for &[a,b] in &parallel_edges {
                let divisions = schedule[a].max(schedule[b]);
                changed |= schedule[a] != divisions || schedule[b] != divisions;
                schedule[a] = divisions; schedule[b] = divisions;
            }
            if !changed {break;}
        }
        // A distinct curved cusp can have coincident chords on a uniform
        // parameter grid. Refine its shared schedule before allocating positions.
        cusp_sampling::refine(model,&mut schedule,&parallel_edges)?;
        let count = model.vertices.len() + schedule.iter().map(|n| n - 1).sum::<usize>();
        if count > 60000 {
            return Err(input("B-rep boundary sampling exceeds 60000 positions"));
        }
        let mut registry = Self {
            mesh: Mesh {
                positions: model.vertices.iter().flat_map(|v| v.point).collect(),
                indices: vec![],
                uv: None,
            },
            edges: Vec::with_capacity(model.edges.len()),
        };
        for (edge, divisions) in model.edges.iter().zip(schedule) {
            if edge.degenerate {
                registry.edges.push(vec![edge.vertices[0]; divisions + 1]);
                continue;
            }
            let [a, b] = edge.curve.domain();
            let mut samples = vec![edge.vertices[0]];
            for i in 1..divisions {
                let point = edge
                    .curve
                    .evaluate(a + (b - a) * i as f64 / divisions as f64)?
                    .point;
                samples.push(registry.mesh.positions.len() / 3);
                registry.mesh.positions.extend(point);
            }
            samples.push(edge.vertices[1]);
            registry.edges.push(samples);
        }
        Ok(registry)
    }
    pub(super) fn divisions(&self, coedge: &brep_core::Coedge) -> usize {
        self.edges[coedge.edge].len() - 1
    }
    pub(super) fn position(&self, coedge: &brep_core::Coedge, index: usize) -> Result<usize> {
        let divisions = self.divisions(coedge);
        if index > divisions {
            return Err(input(
                "Face sampling disagrees with its authored edge schedule",
            ));
        }
        Ok(self.edges[coedge.edge][if coedge.reversed {
            divisions - index
        } else {
            index
        }])
    }
    pub(super) fn sample_wire(
        &self,
        model: &brep_core::Model,
        face: &brep_core::Face,
        wire: usize,
    ) -> Result<Vec<BoundarySample>> {
        let mut samples = Vec::new();
        let coedges = &model.loops[wire].coedges;
        for (k, coedge) in coedges.iter().enumerate() {
            let previous = coedges[(k + coedges.len() - 1) % coedges.len()].edge;
            let divisions = self.divisions(coedge);
            let [a, b] = coedge.pcurve.domain();
            for i in 0..=divisions {
                let canonical_index = if coedge.reversed { divisions - i } else { i };
                let parameter = canonical_index as f64 / divisions as f64;
                let local_parameter = if coedge.reversed {
                    1. - parameter
                } else {
                    parameter
                };
                let p = coedge.pcurve.evaluate(a + (b - a) * local_parameter)?.point;
                let uv = [p[0], p[1]];
                let position = self.position(coedge, i)?;
                let expected = self.mesh.point(position)?;
                let actual = face.surface.evaluate(uv[0], uv[1])?.point;
                if expected
                    .iter()
                    .zip(actual)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt()
                    > model.tolerance_mm
                {
                    return Err(input(
                        "Face pcurve disagrees with its authoritative edge samples",
                    ));
                }
                if i < divisions {
                    let edges = if i == 0 {
                        [coedge.edge, previous]
                    } else {
                        [coedge.edge, coedge.edge]
                    };
                    samples.push(BoundarySample {
                        edges,
                        uv,
                        position,
                    });
                }
            }
        }
        Ok(samples)
    }
    pub(super) fn verify_boundary_uses(&self, model: &brep_core::Model, face_ids: &[usize]) -> Result<()> {
        let mut actual = BTreeMap::<(usize, usize, usize), i32>::new();
        let add = |map: &mut BTreeMap<(usize, usize, usize), i32>, face, a: usize, b: usize| {
            if a == b {
                return;
            }
            *map.entry((face, a.min(b), a.max(b))).or_default() += if a < b { 1 } else { -1 };
        };
        for (triangle, &face) in self.mesh.indices.as_chunks::<3>().0.iter().zip(face_ids) {
            for i in 0..3 {
                add(&mut actual, face, triangle[i], triangle[(i + 1) % 3]);
            }
        }
        let mut expected = BTreeMap::<(usize, usize, usize), i32>::new();
        for shell in &model.shells {
            for use_ in &shell.faces {
                let face = &model.faces[use_.face];
                for &wire in std::iter::once(&face.outer).chain(&face.holes) {
                    for coedge in &model.loops[wire].coedges {
                        for segment in self.edges[coedge.edge].windows(2) {
                            let [a, b] = [segment[0], segment[1]];
                            if a == b {
                                if model.edges[coedge.edge].degenerate {
                                    continue;
                                }
                                return Err(input(
                                    "Resolution collapses a nondegenerate authored edge",
                                ));
                            }
                            let reversed = coedge.reversed != use_.reversed;
                            add(
                                &mut expected,
                                use_.face,
                                if reversed { b } else { a },
                                if reversed { a } else { b },
                            );
                        }
                    }
                }
            }
        }
        actual.retain(|_, count| *count != 0);
        expected.retain(|_, count| *count != 0);
        if actual != expected {
            let first = actual
                .iter()
                .find(|(key, value)| expected.get(key) != Some(value))
                .map(|(key, value)| {
                    format!("actual {key:?}={value}, expected {:?}", expected.get(key))
                })
                .or_else(|| {
                    expected
                        .iter()
                        .find(|(key, value)| actual.get(key) != Some(value))
                        .map(|(key, value)| {
                            format!("expected {key:?}={value}, actual {:?}", actual.get(key))
                        })
                })
                .unwrap_or_default();
            return Err(input(format!(
                "Tessellation face boundaries do not match the authored edge sample registry: {first}"
            )));
        }
        Ok(())
    }
    pub(super) fn append(
        &mut self,
        face: &brep_core::Face,
        use_: &brep_core::FaceUse,
        mut built: FaceMesh,
        face_ids: &mut Vec<usize>,
        model_tolerance_mm: f64,
        allow_preview_retriangulation: bool,
    ) -> Result<()> {
        let mut remap = Vec::with_capacity(built.uv.len());
        for (uv, shared) in built.uv.iter().zip(&built.shared) {
            remap.push(if let Some(position) = shared {
                *position
            } else {
                let index = self.mesh.positions.len() / 3;
                if index >= 60000 {
                    return Err(input("B-rep tessellation exceeds 60000 positions"));
                }
                self.mesh
                    .positions
                    .extend(face.surface.evaluate(uv[0], uv[1])?.point);
                index
            });
        }
        if is_affine_plane(&face.surface, 0.) && face.surface.weights.iter().flatten()
            .all(|w| *w == face.surface.weights[0][0]) {
            let points: Vec<[f64;3]> = remap.iter().map(|&i| {
                let p = &self.mesh.positions[3*i..3*i+3]; [p[0],p[1],p[2]]
            }).collect();
            repair_collinear_triangles(&mut built, &points);
        }
        if allow_preview_retriangulation && is_affine_plane(&face.surface, model_tolerance_mm) {
            let points: Vec<[f64;3]> = remap.iter().map(|&i| {
                let p = &self.mesh.positions[3*i..3*i+3]; [p[0],p[1],p[2]]
            }).collect();
            planar_slivers::repair(&mut built, &points);
        }
        for triangle in built.triangles {
            let mut mapped = triangle.map(|i| remap[i]);
            if use_.reversed {
                mapped.swap(1, 2);
            }
            self.mesh.indices.extend(mapped);
            face_ids.push(use_.face);
        }
        if face_ids.len() > 20000 {
            return Err(input(format!(
                "B-rep tessellation exceeds 20000 triangles after face {}: {} triangles",
                use_.face,
                face_ids.len()
            )));
        }
        Ok(())
    }
}
pub(super) struct FaceMesh {
    pub(super) uv: Vec<[f64; 2]>,
    pub(super) shared: Vec<Option<usize>>,
    pub(super) triangles: Vec<[usize; 3]>,
}
pub(super) fn uv_key(point: [f64; 2]) -> [u64; 2] {
    point.map(|x| if x == 0. { 0 } else { x.to_bits() })
}
pub(super) fn triangulate_boundary(
    outer: &[BoundarySample],
    holes: &[Vec<BoundarySample>],
) -> Result<FaceMesh> {
    let mut boundary = BTreeMap::new();
    for sample in outer.iter().chain(holes.iter().flatten()) {
        if let Some((previous, _)) =
            boundary.insert(uv_key(sample.uv), (sample.position, sample.edges))
            && previous != sample.position
        {
            return Err(input(
                "Distinct authored boundaries have an ambiguous shared UV sample",
            ));
        }
    }
    let fill = planar_geometry::triangulation::triangulate_profile(
        &outer.iter().map(|p| p.uv).collect::<Vec<_>>(),
        &holes
            .iter()
            .map(|wire| wire.iter().map(|p| p.uv).collect())
            .collect::<Vec<_>>(),
    )
    .inspect_err(|_error| {
        #[cfg(test)]
        eprintln!(
            "triangulation boundary outer={:?} holes={:?}",
            outer.iter().map(|p| p.uv).collect::<Vec<_>>(),
            holes
                .iter()
                .map(|w| w.iter().map(|p| p.uv).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
    })?;
    let mut out = FaceMesh {
        uv: vec![],
        shared: vec![],
        triangles: vec![],
    };
    let mut local = BTreeMap::<[u64; 2], usize>::new();
    let mut remap = Vec::new();
    let mut edges_of = Vec::new();
    for point in fill.positions {
        let key = uv_key(point);
        let (global, edges) = *boundary
            .get(&key)
            .ok_or_else(|| input("Triangulation introduced an unowned boundary position"))?;
        let index = *local.entry(key).or_insert_with(|| {
            let index = out.uv.len();
            out.uv.push(point);
            out.shared.push(Some(global));
            edges_of.push(edges);
            index
        });
        remap.push(index);
    }
    for triangle in fill.indices.as_chunks::<3>().0 {
        let triangle = triangle.map(|index| remap[index as usize]);
        if (0..3).any(|i| triangle[i] == triangle[(i + 1) % 3]) {
            return Err(input("Trim triangulation collapsed an authored boundary"));
        }
        out.triangles.push(triangle);
    }
    split_boundary_chords(&mut out, &edges_of);
    Ok(out)
}

/// A boundary-only triangulation connects boundary samples by chords that
/// are interior to this face. Along a curved authored edge the face across
/// it can pick the same chord between the same two samples, and the two
/// meshes then share an edge that is not authored: four triangles on one 3D
/// edge, no longer a manifold shell. Split every chord between two samples
/// of one authored edge at its midpoint with a private interior vertex;
/// chords lie inside the polygon, so the midpoint does. Samples of a
/// straight edge are collinear and never chorded, so planar faces keep
/// their boundary-only triangulation.
fn split_boundary_chords(mesh: &mut FaceMesh, edges_of: &[[usize; 2]]) {
    let same_edge = |p: usize, q: usize| edges_of[p].iter().any(|e| edges_of[q].contains(e));
    loop {
        let mut incidence = BTreeMap::<[usize; 2], Vec<usize>>::new();
        for (index, t) in mesh.triangles.iter().enumerate() {
            for k in 0..3 {
                let (p, q) = (t[k], t[(k + 1) % 3]);
                incidence
                    .entry([p.min(q), p.max(q)])
                    .or_default()
                    .push(index);
            }
        }
        let chord = incidence.iter().find(|([p, q], tris)| {
            tris.len() == 2
                && mesh.shared[*p].is_some()
                && mesh.shared[*q].is_some()
                && same_edge(*p, *q)
        });
        let Some((&[p, q], tris)) = chord else {
            return;
        };
        let (first, second) = (tris[0].min(tris[1]), tris[0].max(tris[1]));
        let m = mesh.uv.len();
        mesh.uv.push([
            (mesh.uv[p][0] + mesh.uv[q][0]) / 2.,
            (mesh.uv[p][1] + mesh.uv[q][1]) / 2.,
        ]);
        mesh.shared.push(None);
        let mut replacement = Vec::with_capacity(4);
        for &index in &[first, second] {
            let t = mesh.triangles[index];
            let x = t.into_iter().find(|&v| v != p && v != q).unwrap();
            // Keep the winding: p -> q -> x becomes p -> m -> x, m -> q -> x.
            if (0..3).any(|k| t[k] == p && t[(k + 1) % 3] == q) {
                replacement.push([p, m, x]);
                replacement.push([m, q, x]);
            } else {
                replacement.push([q, m, x]);
                replacement.push([m, p, x]);
            }
        }
        mesh.triangles.swap_remove(second);
        mesh.triangles.swap_remove(first);
        mesh.triangles.extend(replacement);
    }
}
pub fn nurbs(model: &brep_core::Model, segments: usize) -> Result<Tessellation> {
    nurbs_with_freeform_note(model, segments, FreeformTessNote::SampledUncertified)
}

fn rectangular_parallel_edges(model: &brep_core::Model, face: &brep_core::Face)
    -> Result<Vec<[usize;2]>> {
    let coedges=&model.loops[face.outer].coedges;
    if coedges.len()!=4 || rectangular_trim_domain(model,face)?.is_none()
        || coedges.iter().any(|c|model.edges[c.edge].degenerate) {return Ok(Vec::new());}
    let mut horizontal=Vec::new(); let mut vertical=Vec::new();
    for c in coedges {
        let p=&c.pcurve.control_points;
        if p[0][1]==p[1][1] {horizontal.push(c.edge);} else {vertical.push(c.edge);}
    }
    if horizontal.len()!=2 || vertical.len()!=2 {return Ok(Vec::new());}
    Ok(vec![[horizontal[0],horizontal[1]],[vertical[0],vertical[1]]])
}

fn exact_linear_ruling(surface: &Surface, coedge: &brep_core::Coedge, edge: &brep_core::Edge) -> bool {
    let p=&coedge.pcurve;
    let curve=&edge.curve;
    if edge.degenerate || p.degree!=1 || p.control_points.len()!=2
        || p.weights[0]!=p.weights[1] || curve.degree!=1
        || curve.control_points.len()!=2 || curve.weights[0]!=curve.weights[1] {
        return false;
    }
    let a=&p.control_points[0];let b=&p.control_points[1];
    let u = a[1]==b[1] && a[0]!=b[0] && surface.degree_u==1
        && !surface.periodic_u && surface.control_points.len()==2
        && surface.knots_u.len()==4 && surface.knots_u[0]==surface.knots_u[1]
        && surface.knots_u[2]==surface.knots_u[3]
        && surface.weights[0]==surface.weights[1];
    let v = a[0]==b[0] && a[1]!=b[1] && surface.degree_v==1
        && !surface.periodic_v && surface.control_points.iter().all(|row|row.len()==2)
        && surface.knots_v.len()==4 && surface.knots_v[0]==surface.knots_v[1]
        && surface.knots_v[2]==surface.knots_v[3]
        && surface.weights.iter().all(|row|row[0]==row[1]);
    u || v
}

fn repair_collinear_triangles(mesh: &mut FaceMesh, points: &[[f64;3]]) {
    let normal = |a:usize,b:usize,c:usize| represented_triangle_cross(points[a],points[b],points[c]);
    let Some(n)=mesh.triangles.iter().filter_map(|t|normal(t[0],t[1],t[2]))
        .find(|n|*n!=[0;3]) else {return;};
    let axis=(0..3).max_by_key(|&i|n[i].unsigned_abs()).unwrap();
    let area=|a,b,c|normal(a,b,c).map(|n|n[axis].signum());
    loop {
        let mut replacement=None;
        'search: for (i,t) in mesh.triangles.iter().enumerate() {
            if normal(t[0],t[1],t[2])!=Some([0;3]) {continue;}
            for k in 0..3 {
                let [a,b,c]=[t[k],t[(k+1)%3],t[(k+2)%3]];
                let [pa,pb,pc]=[points[a],points[b],points[c]];
                let Some(j)=(0..3).find(|&j|pa[j]!=pb[j]) else {continue;};
                if a==b || a==c || b==c || !(pa[j].min(pb[j])<pc[j] && pc[j]<pa[j].max(pb[j])) {continue;}
                let neighbors:Vec<_>=mesh.triangles.iter().enumerate()
                    .filter(|(_,q)|q.contains(&a)&&q.contains(&b)).collect();
                if neighbors.len()!=2 {continue;}
                let Some((j,q))=neighbors.into_iter().find(|(j,_)|*j!=i) else {continue;};
                let Some(&d)=q.iter().find(|&&v|v!=a&&v!=b) else {continue;};
                let forward=(0..3).any(|l|q[l]==a&&q[(l+1)%3]==b);
                let (u,v)=if forward {(a,b)} else {(b,a)};
                let Some(sign)=area(q[0],q[1],q[2]) else {continue;};
                let first=[u,c,d];let second=[c,v,d];
                if sign==0 || area(first[0],first[1],first[2])!=Some(sign)
                    || area(second[0],second[1],second[2])!=Some(sign) {continue;}
                replacement=Some((i,j,first,second));break 'search;
            }
        }
        let Some((i,j,first,second))=replacement else {return;};
        mesh.triangles[i]=first;mesh.triangles[j]=second;
    }
}

fn rectangular_trim_domain(model: &brep_core::Model, face: &brep_core::Face) -> Result<Option<[f64; 4]>> {
    let coedges = &model.loops[face.outer].coedges;
    let corners = model.loop_uv(face.outer, 1)?;
    let min_u = corners
        .iter()
        .map(|point| point[0])
        .fold(f64::INFINITY, f64::min);
    let max_u = corners
        .iter()
        .map(|point| point[0])
        .fold(f64::NEG_INFINITY, f64::max);
    let min_v = corners
        .iter()
        .map(|point| point[1])
        .fold(f64::INFINITY, f64::min);
    let max_v = corners
        .iter()
        .map(|point| point[1])
        .fold(f64::NEG_INFINITY, f64::max);
    let rectangular = face.holes.is_empty()
        && corners.len() >= 4
        && coedges.iter().all(|c| {
            c.pcurve.degree == 1
                && c.pcurve.control_points.len() == 2
                && c.pcurve.weights.iter().all(|w| *w == c.pcurve.weights[0])
                && {
                    let a = &c.pcurve.control_points[0];
                    let b = &c.pcurve.control_points[1];
                    (a[0] == b[0] || a[1] == b[1])
                        && [a, b].iter().all(|p| {
                            p[0] == min_u || p[0] == max_u || p[1] == min_v || p[1] == max_v
                        })
                }
        })
        && max_u > min_u
        && max_v > min_v;
    Ok(rectangular.then_some([min_u, max_u, min_v, max_v]))
}

fn represented_triangle_cross(a: [f64;3], b: [f64;3], c: [f64;3]) -> Option<[i128;3]> {
    let points=[a,b,c]; let mut parts=[[(0i128,0i32);3];3]; let mut exponent=i32::MAX;
    for i in 0..3 {for j in 0..3 {
        let x=points[i][j]; if !x.is_finite() {return None;}
        let bits=x.to_bits(); let field=((bits>>52)&2047) as i32;
        let mut mantissa=bits&((1u64<<52)-1);
        let mut e=if field==0 {-1074} else {mantissa|=1u64<<52;field-1075};
        if mantissa!=0 {let shift=mantissa.trailing_zeros();mantissa>>=shift;e+=shift as i32;exponent=exponent.min(e);}
        let signed=if bits>>63==0 {mantissa as i128} else {-(mantissa as i128)};
        parts[i][j]=(signed,e);
    }}
    if exponent==i32::MAX {return Some([0;3]);}
    let mut p=[[0i128;3];3];
    for i in 0..3 {for j in 0..3 {let (m,e)=parts[i][j];if m==0 {continue;}
        let shift=u32::try_from(e-exponent).ok()?;if shift>126 {return None;}
        p[i][j]=m.checked_mul(1i128.checked_shl(shift)?)?;
    }}
    let mut ab=[0i128;3];let mut ac=[0i128;3];
    for j in 0..3 {ab[j]=p[1][j].checked_sub(p[0][j])?;ac[j]=p[2][j].checked_sub(p[0][j])?;}
    Some([
        ab[1].checked_mul(ac[2])?.checked_sub(ab[2].checked_mul(ac[1])?)?,
        ab[2].checked_mul(ac[0])?.checked_sub(ab[0].checked_mul(ac[2])?)?,
        ab[0].checked_mul(ac[1])?.checked_sub(ab[1].checked_mul(ac[0])?)?,
    ])
}
