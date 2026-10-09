//! Tessellation and interchange preserve topological face identity.
use super::*;
use std::collections::BTreeMap;

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
    let mut positions = Vec::<f64>::new();
    let mut cells = BTreeMap::<[i64; 3], Vec<usize>>::new();
    let mut remap = vec![];
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
fn finish(
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
fn finish_indexed(
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
enum FreeformTessNote {
    None,
    SampledUncertified,
    BernsteinCertified,
}

fn finish_indexed_mode(
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
fn is_affine_plane(surface: &Surface, tolerance: f64) -> bool {
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
struct BoundarySample {
    /// Authored edges this sample lies on: its own coedge's edge, and for
    /// the coedge's start vertex also the previous coedge's edge.
    edges: [usize; 2],
    uv: [f64; 2],
    position: usize,
}
/// Geometric positions belong to authored vertices and edges, never to a
/// tolerance bucket. Two nearby disconnected shells keep disjoint indices.
struct EdgeSamplingRegistry {
    mesh: Mesh,
    edges: Vec<Vec<usize>>,
}
impl EdgeSamplingRegistry {
    fn new(model: &brep_core::Model, segments: usize) -> Result<Self> {
        let mut schedule = vec![1; model.edges.len()];
        for face in &model.faces {
            let curved = !is_affine_plane(&face.surface, model.tolerance_mm);
            for &wire in std::iter::once(&face.outer).chain(&face.holes) {
                for coedge in &model.loops[wire].coedges {
                    if curved || model.edges[coedge.edge].curve.degree > 1 {
                        schedule[coedge.edge] = segments;
                    }
                }
            }
        }
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
    fn divisions(&self, coedge: &brep_core::Coedge) -> usize {
        self.edges[coedge.edge].len() - 1
    }
    fn position(&self, coedge: &brep_core::Coedge, index: usize) -> Result<usize> {
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
    fn sample_wire(
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
    fn verify_boundary_uses(&self, model: &brep_core::Model, face_ids: &[usize]) -> Result<()> {
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
    fn append(
        &mut self,
        face: &brep_core::Face,
        use_: &brep_core::FaceUse,
        built: FaceMesh,
        face_ids: &mut Vec<usize>,
    ) -> Result<()> {
        let mut remap = Vec::with_capacity(built.uv.len());
        for (uv, shared) in built.uv.iter().zip(built.shared) {
            remap.push(if let Some(position) = shared {
                position
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
struct FaceMesh {
    uv: Vec<[f64; 2]>,
    shared: Vec<Option<usize>>,
    triangles: Vec<[usize; 3]>,
}
fn uv_key(point: [f64; 2]) -> [u64; 2] {
    point.map(|x| if x == 0. { 0 } else { x.to_bits() })
}
fn triangulate_boundary(
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

fn nurbs_with_freeform_note(
    model: &brep_core::Model,
    segments: usize,
    freeform_note: FreeformTessNote,
) -> Result<Tessellation> {
    model.validate()?;
    if !(1..=32).contains(&segments) {
        return Err(input("B-rep tessellation segments must be 1..32"));
    }
    let mut registry = EdgeSamplingRegistry::new(model, segments)?;
    let mut face_ids = Vec::new();
    let mut closed_triangles = Vec::new();
    for shell in &model.shells {
        for use_ in &shell.faces {
            let face = &model.faces[use_.face];
            let outer = registry.sample_wire(model, face, face.outer)?;
            let holes = face
                .holes
                .iter()
                .map(|&wire| registry.sample_wire(model, face, wire))
                .collect::<Result<Vec<_>>>()?;
            let built = if is_affine_plane(&face.surface, model.tolerance_mm) {
                triangulate_boundary(&outer, &holes)?
            } else if let Some(rectangular) = rectangular_face(model, face, segments, &registry)? {
                rectangular
            } else if let Some(quarter) = quarter_disk_face(model, face, segments, &registry)? {
                quarter
            } else {
                clamp_interior_to_trims(
                    model,
                    face,
                    refine_trimmed_face(face, triangulate_boundary(&outer, &holes)?, segments)
                        .map_err(|error| {
                            input(format!("Face {} refinement: {}", use_.face, error.message))
                        })?,
                )?
            };
            registry.append(face, use_, built, &mut face_ids)?;
            closed_triangles.resize(face_ids.len(), shell.closed);
        }
    }
    registry.verify_boundary_uses(model, &face_ids)?;
    let topology_face_ids = face_ids
        .iter()
        .map(|&face| model.1.faces[face].to_string())
        .collect();
    let freeform_faces = model
        .faces
        .iter()
        .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1);
    let note = if freeform_faces {
        freeform_note
    } else {
        FreeformTessNote::None
    };
    let mut result = finish_indexed_mode(
        registry.mesh,
        face_ids,
        Some(topology_face_ids),
        !model.shells.is_empty() && model.shells.iter().all(|s| s.closed),
        note,
    )?;
    result.closed_triangles = Some(closed_triangles);
    if let Some(certificate) = &mut result.certificate {
        certificate.shell_count = model.shells.len();
    }
    Ok(result)
}

/// Certified finite tessellation for independently recognized analytic shells.
/// Shared-edge registry incidence supplies no-crack/orientation proof; curved
/// deviation is selected from closed-form sphere/cone/torus/cylinder bounds.
pub fn certified_nurbs(
    model: &brep_core::Model,
    chord_tolerance_mm: f64,
    max_triangles: usize,
) -> Result<CertifiedTessellation> {
    use brep_core::predicate_evidence::{PredicateEvidence, compose_predicate_evidence};
    if !(chord_tolerance_mm.is_finite() && chord_tolerance_mm > 0.)
        || !(12..=20_000).contains(&max_triangles)
    {
        return Err(nurbs_core::Error::new(
            "BREP_TESSELLATION_OPTIONS_INVALID",
            "Chord tolerance must be positive and triangle budget 12..20000",
        ));
    }
    model.validate()?;
    let audit = brep_core::solid_audit::audit_solid(model)?;
    // Classify before adaptive search so unsupported geometry is a typed
    // refusal, distinct from a proved family exhausting the finite budget.
    brep_core::analysis::certified_tessellation_deviation(model, 1)?;
    let segments = (1..=32)
        .find(|segments| {
            brep_core::analysis::certified_tessellation_deviation(model, *segments)
                .is_ok_and(|deviation| deviation <= chord_tolerance_mm)
        })
        .ok_or_else(|| nurbs_core::Error::new(
            "BREP_TESSELLATION_BUDGET_EXHAUSTED",
            "Requested two-sided analytic deviation needs more than 32 subdivisions per patch, or a shell is outside the certified finite matrix",
        ))?;
    let deviation = brep_core::analysis::certified_tessellation_deviation(model, segments)?;
    let tessellation = nurbs(model, segments)?;
    if tessellation.built.mesh.indices.len() / 3 > max_triangles {
        return Err(nurbs_core::Error::new(
            "BREP_TESSELLATION_BUDGET_EXHAUSTED",
            "Certified tessellation exceeded its triangle budget",
        ));
    }
    let certificate = tessellation.certificate.as_ref().ok_or_else(|| {
        nurbs_core::Error::new(
            "BREP_CERTIFIED_TESSELLATION_REFUSED",
            "Tessellation did not publish incidence coverage",
        )
    })?;
    let naming_complete = model.persistent_naming_complete()
        && model.1.faces.len() == model.faces.len()
        && model.1.edges.len() == model.edges.len();
    if !certificate.complete
        || !certificate.closed_shells
        || !tessellation.built.report.closed
        || !naming_complete
    {
        return Err(nurbs_core::Error::new(
            "BREP_CERTIFIED_TESSELLATION_REFUSED",
            "Coverage, audit, ChangeSet, or naming evidence is incomplete",
        ));
    }
    let context = model.tolerance_context()?;
    let evidence = compose_predicate_evidence(
        &context,
        [
            // The analytic sagitta is recorded separately as the two-sided
            // tessellation enclosure. Predicate evidence binds exact authored
            // edge correspondence to the model tolerance context.
            PredicateEvidence::positional(&context, 0., chord_tolerance_mm)?,
            PredicateEvidence::topology_preservation(
                &context,
                "shared-edge identity, orientation, and no T-junction coverage",
                true,
            )?,
        ],
    )?;
    Ok(CertifiedTessellation {
        capability: CERTIFIED_TESSELLATION_CAPABILITY,
        context: context.spec_identity(),
        surface_to_mesh_deviation_mm: deviation,
        mesh_to_surface_deviation_mm: deviation,
        audit,
        evidence,
        change_set: model.1.change_set.clone(),
        naming_complete,
        max_triangles,
        subdivisions_per_patch: segments,
        tessellation,
    })
}

/// Certified freeform tessellation finite cell: equal-weight clamped Bezier
/// shells with Bernstein second-difference deviation (capability /1).
pub fn certified_freeform_nurbs(
    model: &brep_core::Model,
    chord_tolerance_mm: f64,
    max_triangles: usize,
) -> Result<CertifiedTessellation> {
    use brep_core::predicate_evidence::{PredicateEvidence, compose_predicate_evidence};
    if !(chord_tolerance_mm.is_finite() && chord_tolerance_mm > 0.)
        || !(12..=20_000).contains(&max_triangles)
    {
        return Err(nurbs_core::Error::new(
            "BREP_TESSELLATION_OPTIONS_INVALID",
            "Chord tolerance must be positive and triangle budget 12..20000",
        ));
    }
    model.validate()?;
    let audit = brep_core::solid_audit::audit_solid(model)?;
    brep_core::analysis::certified_freeform_tessellation_deviation(model, 1)?;
    let segments = (1..=32)
        .find(|segments| {
            brep_core::analysis::certified_freeform_tessellation_deviation(model, *segments)
                .is_ok_and(|deviation| deviation <= chord_tolerance_mm)
        })
        .ok_or_else(|| {
            nurbs_core::Error::new(
                "BREP_TESSELLATION_BUDGET_EXHAUSTED",
                "Requested freeform Bernstein deviation needs more than 32 subdivisions per patch",
            )
        })?;
    let deviation =
        brep_core::analysis::certified_freeform_tessellation_deviation(model, segments)?;
    let tessellation =
        nurbs_with_freeform_note(model, segments, FreeformTessNote::BernsteinCertified)?;
    if tessellation.built.mesh.indices.len() / 3 > max_triangles {
        return Err(nurbs_core::Error::new(
            "BREP_TESSELLATION_BUDGET_EXHAUSTED",
            "Certified freeform tessellation exceeded its triangle budget",
        ));
    }
    let certificate = tessellation.certificate.as_ref().ok_or_else(|| {
        nurbs_core::Error::new(
            "BREP_CERTIFIED_TESSELLATION_REFUSED",
            "Tessellation did not publish incidence coverage",
        )
    })?;
    let naming_complete = model.persistent_naming_complete()
        && model.1.faces.len() == model.faces.len()
        && model.1.edges.len() == model.edges.len();
    if !certificate.complete
        || !certificate.closed_shells
        || !tessellation.built.report.closed
        || !naming_complete
    {
        return Err(nurbs_core::Error::new(
            "BREP_CERTIFIED_TESSELLATION_REFUSED",
            "Coverage, audit, ChangeSet, or naming evidence is incomplete",
        ));
    }
    let context = model.tolerance_context()?;
    let evidence = compose_predicate_evidence(
        &context,
        [
            PredicateEvidence::positional(&context, 0., chord_tolerance_mm)?,
            PredicateEvidence::topology_preservation(
                &context,
                "shared-edge identity, orientation, and Bernstein freeform deviation",
                true,
            )?,
        ],
    )?;
    Ok(CertifiedTessellation {
        capability: FREEFORM_TESSELLATION_CAPABILITY,
        context: context.spec_identity(),
        surface_to_mesh_deviation_mm: deviation,
        mesh_to_surface_deviation_mm: deviation,
        audit,
        evidence,
        change_set: model.1.change_set.clone(),
        naming_complete,
        max_triangles,
        subdivisions_per_patch: segments,
        tessellation,
    })
}
fn rectangular_face(
    model: &brep_core::Model,
    face: &brep_core::Face,
    _segments: usize,
    registry: &EdgeSamplingRegistry,
) -> Result<Option<FaceMesh>> {
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
    if !rectangular {
        return Ok(None);
    }
    let horizontal = |v: f64| {
        coedges
            .iter()
            .filter(|coedge| {
                let points = &coedge.pcurve.control_points;
                points[0][1] == v && points[1][1] == v && !model.edges[coedge.edge].degenerate
            })
            .map(|coedge| registry.divisions(coedge))
            .sum::<usize>()
    };
    let vertical = |u: f64| {
        coedges
            .iter()
            .filter(|coedge| {
                let points = &coedge.pcurve.control_points;
                points[0][0] == u && points[1][0] == u && !model.edges[coedge.edge].degenerate
            })
            .map(|coedge| registry.divisions(coedge))
            .sum::<usize>()
    };
    let segments_u = horizontal(min_v).max(horizontal(max_v));
    let segments_v = vertical(min_u).max(vertical(max_u));
    if segments_u == 0 || segments_v == 0 {
        return Err(input("Rectangular patch has no ordinary boundary schedule"));
    }
    let poles: Vec<_> = coedges
        .iter()
        .filter(|c| model.edges[c.edge].degenerate)
        .map(|c| model.edges[c.edge].vertices[0])
        .collect();
    let mut boundary = BTreeMap::<(usize, usize), usize>::new();
    let mut pole_rows = BTreeMap::<usize, usize>::new();
    for coedge in coedges {
        let divisions = registry.divisions(coedge);
        let domain = coedge.pcurve.domain();
        for sample in 0..=divisions {
            let t = sample as f64 / divisions as f64;
            let uv = coedge
                .pcurve
                .evaluate(domain[0] + t * (domain[1] - domain[0]))?
                .point;
            let key = (
                ((uv[0] - min_u) / (max_u - min_u) * segments_u as f64).round() as usize,
                ((uv[1] - min_v) / (max_v - min_v) * segments_v as f64).round() as usize,
            );
            let position = registry.position(coedge, sample)?;
            boundary.insert(key, position);
            if model.edges[coedge.edge].degenerate {
                pole_rows.insert(key.1, position);
            }
        }
    }
    let mut out = FaceMesh {
        uv: vec![],
        shared: vec![],
        triangles: vec![],
    };
    for y in 0..=segments_v {
        for x in 0..=segments_u {
            let shared = boundary
                .get(&(x, y))
                .copied()
                .or_else(|| pole_rows.get(&y).copied());
            out.uv.push([
                min_u + (max_u - min_u) * x as f64 / segments_u as f64,
                min_v + (max_v - min_v) * y as f64 / segments_v as f64,
            ]);
            out.shared.push(shared);
        }
    }
    for y in 0..segments_v {
        for x in 0..segments_u {
            let a = y * (segments_u + 1) + x;
            let b = a + 1;
            let d = a + segments_u + 1;
            let c = d + 1;
            for triangle in [[a, b, c], [a, c, d]] {
                let repeated = (0..3).find_map(|i| {
                    let first = out.shared[triangle[i]];
                    first.filter(|_| first == out.shared[triangle[(i + 1) % 3]])
                });
                if let Some(vertex) = repeated {
                    if !poles.contains(&vertex) {
                        return Err(input(
                            "Sampling resolution collapses a nondegenerate boundary",
                        ));
                    }
                    continue;
                }
                out.triangles.push(triangle);
            }
        }
    }
    Ok(Some(out))
}
fn quarter_disk_face(
    model: &brep_core::Model,
    face: &brep_core::Face,
    segments: usize,
    registry: &EdgeSamplingRegistry,
) -> Result<Option<FaceMesh>> {
    let coedges = &model.loops[face.outer].coedges;
    if !face.holes.is_empty() || coedges.len() != 3 {
        return Ok(None);
    }
    let a = &coedges[0].pcurve;
    let arc = &coedges[1].pcurve;
    let b = &coedges[2].pcurve;
    let bezier = |c: &Curve| {
        let [low, high] = c.domain();
        c.knots[..=c.degree].iter().all(|k| *k == low)
            && c.knots[c.control_points.len()..].iter().all(|k| *k == high)
    };
    let straight = |c: &Curve, points: &[Vec<f64>]| {
        bezier(c) && c.degree == 1 && c.control_points == points && c.weights[0] == c.weights[1]
    };
    let quarter = straight(a, &[vec![0., 0.], vec![1., 0.]])
        && straight(b, &[vec![0., 1.], vec![0., 0.]])
        && bezier(arc)
        && arc.degree == 2
        && arc.control_points == vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]]
        // The sphere's exact binary rational traversal has a different
        // parameter speed from the symmetric quarter arc. Both trace the
        // same quarter circle; evaluate the authored curve below so the
        // interior grid retains its shared edge sample schedule.
        && (arc.weights == vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.]
            || arc.weights == vec![1., 1., 2.]);
    if !quarter {
        return Ok(None);
    }
    if coedges.iter().any(|c| registry.divisions(c) != segments) {
        return Err(input("Quarter-disk edge schedules disagree"));
    }
    let mut out = FaceMesh {
        uv: vec![[0., 0.]],
        shared: vec![Some(registry.position(&coedges[0], 0)?)],
        triangles: vec![],
    };
    let [low, high] = arc.domain();
    for row in 1..=segments {
        for column in 0..=segments {
            let p = arc
                .evaluate(low + (high - low) * column as f64 / segments as f64)?
                .point;
            let radius = row as f64 / segments as f64;
            out.uv.push([p[0] * radius, p[1] * radius]);
            out.shared.push(if row == segments {
                Some(registry.position(&coedges[1], column)?)
            } else if column == 0 {
                Some(registry.position(&coedges[0], row)?)
            } else if column == segments {
                Some(registry.position(&coedges[2], segments - row)?)
            } else {
                None
            });
        }
    }
    for j in 0..segments {
        out.triangles.push([0, 1 + j, 2 + j]);
    }
    for row in 1..segments {
        for j in 0..segments {
            let a = 1 + (row - 1) * (segments + 1) + j;
            let b = a + 1;
            let d = a + segments + 1;
            let c = d + 1;
            out.triangles.extend([[a, d, c], [a, c, b]]);
        }
    }
    Ok(Some(out))
}
/// Split only interior UV edges; all boundary positions retain their registry ownership.
fn refine_trimmed_face(
    face: &brep_core::Face,
    mesh: FaceMesh,
    segments: usize,
) -> Result<FaceMesh> {
    let surface = &face.surface;
    let mut uv = mesh.uv;
    let mut shared = mesh.shared;
    let mut triangles = mesh.triangles;
    let width = surface.knots_u[surface.control_points.len()] - surface.knots_u[surface.degree_u];
    let height =
        surface.knots_v[surface.control_points[0].len()] - surface.knots_v[surface.degree_v];
    let key = |a: usize, b: usize| if a < b { [a, b] } else { [b, a] };
    for _ in 0..24 {
        let mut incidence = BTreeMap::<[usize; 2], usize>::new();
        for t in &triangles {
            for i in 0..3 {
                *incidence.entry(key(t[i], t[(i + 1) % 3])).or_default() += 1;
            }
        }
        let mut splits = BTreeMap::<[usize; 2], usize>::new();
        for (edge, count) in incidence {
            let [a, b] = edge;
            let length =
                ((uv[a][0] - uv[b][0]) / width).powi(2) + ((uv[a][1] - uv[b][1]) / height).powi(2);
            if count == 2 && length > 2. / (segments * segments) as f64 {
                splits.insert(edge, uv.len());
                uv.push([(uv[a][0] + uv[b][0]) / 2., (uv[a][1] + uv[b][1]) / 2.]);
                shared.push(None);
            }
        }
        if splits.is_empty() {
            return Ok(FaceMesh {
                uv,
                shared,
                triangles,
            });
        }
        let mut refined = Vec::new();
        for mut t in triangles {
            let count = (0..3)
                .filter(|&i| splits.contains_key(&key(t[i], t[(i + 1) % 3])))
                .count();
            if count == 0 {
                refined.push(t);
                continue;
            }
            if count == 3 {
                let [a, b, c] = t;
                let ab = splits[&key(a, b)];
                let bc = splits[&key(b, c)];
                let ca = splits[&key(c, a)];
                refined.extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
            } else {
                // Rotate so the one marked edge is AB, or the two are AB and BC.
                while !(splits.contains_key(&key(t[0], t[1]))
                    && (count == 1 || splits.contains_key(&key(t[1], t[2]))))
                {
                    t.rotate_left(1);
                }
                let [a, b, c] = t;
                let ab = splits[&key(a, b)];
                if count == 1 {
                    refined.extend([[a, ab, c], [ab, b, c]]);
                } else {
                    let bc = splits[&key(b, c)];
                    refined.extend([[ab, b, bc], [a, ab, c], [ab, bc, c]]);
                }
            }
        }
        if refined.len() > 20000 {
            return Err(input(format!(
                "B-rep tessellation exceeds 20000 triangles during interior refinement: {} triangles at segments {}",
                refined.len(),
                segments
            )));
        }
        triangles = refined;
    }
    Err(input(
        "B-rep interior tessellation exceeded its refinement budget",
    ))
}
/// Interior refinement points are midpoints of the boundary triangulation,
/// so on the concave side of a curved trim they can fall into the sliver
/// between the chord and the exact pcurve, and evaluate off the face. Test
/// every interior point against a dense sampling of the exact trim loops and
/// pull the outliers onto that boundary.
fn clamp_interior_to_trims(
    model: &brep_core::Model,
    face: &brep_core::Face,
    mut mesh: FaceMesh,
) -> Result<FaceMesh> {
    const DENSE: usize = 48;
    let mut loops: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut curved = false;
    for &wire in std::iter::once(&face.outer).chain(&face.holes) {
        let mut polyline = Vec::new();
        for coedge in &model.loops[wire].coedges {
            let curve = &coedge.pcurve;
            curved |= curve.degree > 1;
            let [a, b] = curve.domain();
            for i in 0..DENSE {
                let t = a + (b - a) * i as f64 / DENSE as f64;
                let p = curve.evaluate(t)?.point;
                polyline.push([p[0], p[1]]);
            }
        }
        loops.push(polyline);
    }
    if !curved {
        return Ok(mesh);
    }
    let inside = |p: [f64; 2]| {
        let mut crossings = 0usize;
        for polyline in &loops {
            for i in 0..polyline.len() {
                let a = polyline[i];
                let b = polyline[(i + 1) % polyline.len()];
                if (a[1] > p[1]) != (b[1] > p[1]) {
                    let x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0]);
                    if x > p[0] {
                        crossings += 1;
                    }
                }
            }
        }
        crossings % 2 == 1
    };
    for (index, point) in mesh.uv.iter_mut().enumerate() {
        if mesh.shared[index].is_some() || inside(*point) {
            continue;
        }
        let mut best: Option<(f64, [f64; 2])> = None;
        for polyline in &loops {
            for i in 0..polyline.len() {
                let a = polyline[i];
                let b = polyline[(i + 1) % polyline.len()];
                let d = [b[0] - a[0], b[1] - a[1]];
                let len2 = d[0] * d[0] + d[1] * d[1];
                let t = if len2 > 0. {
                    (((point[0] - a[0]) * d[0] + (point[1] - a[1]) * d[1]) / len2).clamp(0., 1.)
                } else {
                    0.
                };
                let q = [a[0] + t * d[0], a[1] + t * d[1]];
                let dist = (q[0] - point[0]).powi(2) + (q[1] - point[1]).powi(2);
                if best.map(|(bd, _)| dist < bd).unwrap_or(true) {
                    best = Some((dist, q));
                }
            }
        }
        if let Some((_, q)) = best {
            // Land just inside the boundary rather than on it, so the moved
            // sample never coincides with a boundary sample (which would fold
            // a triangle or break the shared seam samples).
            let away = [point[0] - q[0], point[1] - q[1]];
            let len = away[0].hypot(away[1]);
            let nudged = if len > 0. {
                [q[0] - away[0] / len * 1e-4, q[1] - away[1] / len * 1e-4]
            } else {
                q
            };
            *point = if inside(nudged) { nudged } else { q };
        }
    }
    Ok(mesh)
}

pub fn polygons(model: &polygon_core::solid::brep::Model) -> Result<Tessellation> {
    let t = polygon_core::solid::brep::tessellate(model)?;
    finish(
        t.mesh,
        t.face_ids,
        None,
        model.tolerance_mm,
        model.shells.iter().all(|s| s.closed),
    )
}

#[cfg(test)]
#[path = "tests/brep_registry.rs"]
mod registry_tests;
