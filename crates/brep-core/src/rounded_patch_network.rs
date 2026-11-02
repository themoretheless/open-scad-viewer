//! Authored rectangular rational Bezier sewing. Public full-patch assembly
//! retains exact boundary ownership. The internal segmented-carrier path also
//! requires fresh whole-use tolerance lifts under its separate shared cell cap.
//! Endpoint coincidence only indexes candidates. Solid admission is independent.
use crate::{
    Body, Coedge, Edge, Error, Face, FaceUse, Loop, Model, Result, Shell, TopologyIds, Vertex,
};
use nurbs_core::{curve::Curve, curve_surface_agreement, surface::Surface};
use std::collections::{BTreeMap, BTreeSet};
fn refuse(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}
pub struct Assembly {
    pub model: Model,
    pub correspondence_work: u64,
    pub directly_owned_uses: usize,
    pub proved_uses: usize,
}
pub(crate) fn clamped_single(knots: &[f64], degree: usize, count: usize) -> bool {
    degree > 0
        && degree <= 32
        && count == degree + 1
        && knots.len() == 2 * count
        && knots[..count].iter().all(|&x| x == knots[degree])
        && knots[count..].iter().all(|&x| x == knots[count])
}
fn key(p: [f64; 3]) -> [u64; 3] {
    p.map(|x| if x == 0. { 0 } else { x.to_bits() })
}
fn endpoint(c: &Curve, end: bool) -> [f64; 3] {
    let p = if end {
        c.control_points.last().unwrap()
    } else {
        &c.control_points[0]
    };
    [p[0], p[1], p[2]]
}
/// Copy boundary poles and weights directly, without iso evaluation, fitting,
/// reverse-knot arithmetic, or endpoint healing. Full clamping is mandatory.
pub(crate) fn boundary(s: &Surface, side: usize) -> (Curve, Curve) {
    let nu = s.control_points.len();
    let nv = s.control_points[0].len();
    let u = [s.knots_u[s.degree_u], s.knots_u[nu]];
    let v = [s.knots_v[s.degree_v], s.knots_v[nv]];
    let (degree, knots, mut points, mut weights) = if side == 0 || side == 2 {
        let j = if side == 0 { 0 } else { nv - 1 };
        (
            s.degree_u,
            s.knots_u.clone(),
            s.control_points
                .iter()
                .map(|r| r[j].clone())
                .collect::<Vec<_>>(),
            s.weights.iter().map(|r| r[j]).collect::<Vec<_>>(),
        )
    } else {
        let i = if side == 1 { nu - 1 } else { 0 };
        (
            s.degree_v,
            s.knots_v.clone(),
            s.control_points[i].clone(),
            s.weights[i].clone(),
        )
    };
    if side >= 2 {
        points.reverse();
        weights.reverse();
    }
    let uv = [[u[0], v[0]], [u[1], v[0]], [u[1], v[1]], [u[0], v[1]]];
    (
        Curve {
            degree,
            knots,
            control_points: points,
            weights,
            periodic: false,
        },
        crate::line(uv[side].to_vec(), uv[(side + 1) % 4].to_vec()),
    )
}
pub(crate) fn same_owned_boundary(a: &Curve, b: &Curve, reverse: bool) -> bool {
    // Both are full single Bezier charts, so normalized traversal does not
    // depend on their independent authored knot-domain endpoints.
    a.degree == b.degree
        && a.control_points.len() == b.control_points.len()
        && a.control_points
            .iter()
            .zip(&a.weights)
            .enumerate()
            .all(|(i, (p, w))| {
                let j = if reverse {
                    b.control_points.len() - 1 - i
                } else {
                    i
                };
                p == &b.control_points[j] && *w == b.weights[j]
            })
}
/// Produce one combinatorially closed candidate. This does not certify G2,
/// injectivity, absence of intersections, nesting, or material orientation.
/// All source models and supplied surfaces remain immutable on every refusal.
pub fn assemble(
    surfaces: &[Surface],
    reversed: &[bool],
    tolerance: f64,
    max_work: u64,
    sources: &[&Model],
) -> Result<Assembly> {
    assemble_impl(
        surfaces, reversed, tolerance, max_work, sources, None, 0, &mut 0,
    )
}
/// Internal segmented-carrier path. Every prepared saved world/UV use gets a
/// fresh continuous lift check; no endpoint-only join authorizes a seam.
pub(crate) fn assemble_prepared(
    surfaces: &[Surface],
    reversed: &[bool],
    tolerance: f64,
    max_work: u64,
    boundaries: &[Vec<(Curve, Curve)>],
    max_cells: usize,
    cells: &mut usize,
) -> Result<Assembly> {
    assemble_impl(
        surfaces,
        reversed,
        tolerance,
        max_work,
        &[],
        Some(boundaries),
        max_cells,
        cells,
    )
}
#[allow(clippy::too_many_arguments)]
fn assemble_impl(
    surfaces: &[Surface],
    reversed: &[bool],
    tolerance: f64,
    max_work: u64,
    sources: &[&Model],
    prepared: Option<&[Vec<(Curve, Curve)>]>,
    max_cells: usize,
    cells: &mut usize,
) -> Result<Assembly> {
    if let Some(boundaries) = prepared {
        if boundaries.len() != surfaces.len()
            || boundaries.iter().any(|v| v.len() < 4 || v.len() > 256)
            || boundaries.iter().map(Vec::len).sum::<usize>() > brep_topology::MAX_COEDGES
            || !(1..=100000).contains(&max_cells)
            || *cells > max_cells
        {
            return Err(refuse(
                "BREP_PATCH_NETWORK_LIMITS",
                "Choose bounded complete prepared face boundaries and lift work",
            ));
        }
    }
    if surfaces.len() < 4
        || surfaces.len() > 1024
        || surfaces.len() != reversed.len()
        || !tolerance.is_finite()
        || tolerance <= 0.
        || max_work > 1_000_000
    {
        return Err(refuse(
            "BREP_PATCH_NETWORK_LIMITS",
            "Expected 4..1024 patches, one orientation per patch and a bounded exact ledger",
        ));
    }
    for s in surfaces {
        s.validate()?;
        if s.periodic_u
            || s.periodic_v
            || s.control_points.iter().flatten().any(|p| p.len() != 3)
            || !clamped_single(&s.knots_u, s.degree_u, s.control_points.len())
            || !clamped_single(&s.knots_v, s.degree_v, s.control_points[0].len())
        {
            return Err(refuse(
                "BREP_PATCH_NETWORK_UNSUPPORTED",
                "Sewing requires full rectangular rational Bezier patches; trimmed or multi-span faces need explicit decomposition and ownership",
            ));
        }
    }
    let mut vertices = Vec::<Vertex>::new();
    let mut vertex_map = BTreeMap::new();
    let mut edges = Vec::<Edge>::new();
    let mut candidates = BTreeMap::<[usize; 2], Vec<usize>>::new();
    let mut uses = Vec::<Vec<(usize, bool)>>::new();
    let mut loops = Vec::new();
    let mut faces = Vec::new();
    let mut work = 0u64;
    let mut direct = 0;
    let mut proved = 0;
    for (face, surface) in surfaces.iter().enumerate() {
        let mut coedges = Vec::new();
        let boundary_curves = if let Some(boundaries) = prepared {
            boundaries[face].clone()
        } else {
            (0..4).map(|side| boundary(surface, side)).collect()
        };
        for (curve, mut pcurve) in boundary_curves {
            if prepared.is_some() {
                curve.validate()?;
                pcurve.validate()?;
                if curve.control_points[0].len() != 3 || pcurve.control_points[0].len() != 2 {
                    return Err(refuse(
                        "BREP_PATCH_NETWORK_UNSUPPORTED",
                        "Prepared world/UV boundary dimensions must be 3/2",
                    ));
                }
                if !prepared_lift(&curve, &pcurve, surface, false, tolerance, max_cells, cells)? {
                    return Err(refuse(
                        "BREP_PATCH_NETWORK_UNPROVEN",
                        "Prepared original face lift remains unproven",
                    ));
                }
            }
            let mut vs = [0; 2];
            for (i, end) in [false, true].into_iter().enumerate() {
                let p = endpoint(&curve, end);
                let next = vertices.len();
                vs[i] = *vertex_map.entry(key(p)).or_insert_with(|| {
                    vertices.push(Vertex { point: p });
                    next
                });
            }
            if vs[0] == vs[1] {
                return Err(refuse(
                    "BREP_PATCH_NETWORK_SINGULAR",
                    "A boundary has coincident endpoints; regular multi-patch decomposition is required",
                ));
            }
            let pair = if vs[0] < vs[1] { vs } else { [vs[1], vs[0]] };
            let mut chosen = None;
            for &edge in candidates.get(&pair).into_iter().flatten() {
                let reverse = edges[edge].vertices != vs;
                let count = 4
                    * (curve.control_points.len() + edges[edge].curve.control_points.len()) as u64;
                if count > max_work - work {
                    return Err(refuse(
                        "BREP_PATCH_NETWORK_BUDGET",
                        "Exact boundary ownership budget exhausted",
                    ));
                }
                work += count;
                let owned = same_owned_boundary(&edges[edge].curve, &curve, reverse);
                let candidate_pc = if prepared.is_some() && !owned {
                    prepared_clock(&edges[edge].curve, &curve, &pcurve, reverse)?
                } else {
                    pcurve.clone()
                };
                let equal = if owned {
                    true
                } else {
                    compare_prepared(
                        &edges[edge].curve,
                        &candidate_pc,
                        surface,
                        reverse,
                        tolerance,
                        max_work,
                        &mut work,
                        prepared.is_some(),
                        max_cells,
                        cells,
                    )?
                };
                if equal {
                    if uses[edge].len() != 1 {
                        return Err(refuse(
                            "BREP_PATCH_NETWORK_NONMANIFOLD",
                            "More than two patches share the same complete boundary",
                        ));
                    }
                    if (uses[edge][0].1 ^ reversed[uses[edge][0].0]) == (reverse ^ reversed[face]) {
                        return Err(refuse(
                            "BREP_PATCH_NETWORK_ORIENTATION",
                            "Shared boundary traversal is not opposite in the shell",
                        ));
                    }
                    if owned {
                        direct += 1;
                    } else {
                        proved += 1;
                    }
                    pcurve = candidate_pc;
                    chosen = Some((edge, reverse));
                    break;
                }
            }
            let (edge, reverse) = chosen.unwrap_or_else(|| {
                let id = edges.len();
                edges.push(Edge {
                    vertices: vs,
                    curve,
                    degenerate: false,
                });
                uses.push(Vec::new());
                candidates.entry(pair).or_default().push(id);
                (id, false)
            });
            uses[edge].push((face, reverse));
            coedges.push(Coedge::new(edge, reverse, pcurve));
        }
        let outer = loops.len();
        loops.push(Loop { coedges });
        faces.push(Face {
            surface: surface.clone(),
            outer,
            holes: vec![],
        });
    }
    if uses.iter().any(|u| u.len() != 2) {
        return Err(refuse(
            "BREP_PATCH_NETWORK_OPEN",
            "Unpaired boundaries remain; source trims or end transitions are missing",
        ));
    }
    // Edge-manifold incidence alone permits pinched vertices. Every vertex
    // must have one cyclic face link; disjoint fans cannot share a solid vertex.
    let mut vertex_edges = vec![Vec::new(); vertices.len()];
    for (edge, e) in edges.iter().enumerate() {
        for &v in &e.vertices {
            vertex_edges[v].push(edge);
        }
    }
    for star in vertex_edges {
        let mut link = BTreeMap::<usize, Vec<usize>>::new();
        for edge in star {
            let (a, b) = (uses[edge][0].0, uses[edge][1].0);
            link.entry(a).or_default().push(b);
            link.entry(b).or_default().push(a);
        }
        let mut fan = BTreeSet::new();
        let mut pending = link.keys().next().copied().into_iter().collect::<Vec<_>>();
        while let Some(face) = pending.pop() {
            if fan.insert(face) {
                pending.extend(link[&face].iter().copied());
            }
        }
        if link.len() < 2 || link.values().any(|n| n.len() != 2) || fan.len() != link.len() {
            return Err(refuse(
                "BREP_PATCH_NETWORK_VERTEX_LINK",
                "A vertex contains disconnected or nonmanifold face fans; separate bodies or complete the corner transitions",
            ));
        }
    }
    let mut neighbors = vec![Vec::new(); faces.len()];
    for u in &uses {
        neighbors[u[0].0].push(u[1].0);
        neighbors[u[1].0].push(u[0].0);
    }
    let mut visited = BTreeSet::new();
    let mut pending = vec![0];
    while let Some(f) = pending.pop() {
        if visited.insert(f) {
            pending.extend(neighbors[f].iter().copied());
        }
    }
    if visited.len() != faces.len() {
        return Err(refuse(
            "BREP_PATCH_NETWORK_COMPONENTS",
            "Expected one connected shell; multiple bodies and cavity shells require explicit ownership",
        ));
    }
    let shell_faces = (0..faces.len())
        .map(|face| FaceUse {
            face,
            reversed: reversed[face],
        })
        .collect();
    let mut model = Model(
        brep_topology::Model {
            vertices,
            edges,
            loops,
            faces,
            shells: vec![Shell {
                faces: shell_faces,
                closed: true,
            }],
            bodies: vec![Body {
                outer_shell: 0,
                inner_shells: vec![],
            }],
            tolerance_mm: tolerance,
        },
        TopologyIds::default(),
    );
    model.inherit_topology_ids(sources);
    model.validate()?;
    Ok(Assembly {
        model,
        correspondence_work: work,
        directly_owned_uses: direct,
        proved_uses: proved,
    })
}

#[allow(clippy::too_many_arguments)]
fn compare_prepared(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    tolerance: f64,
    max_work: u64,
    work: &mut u64,
    prepared: bool,
    max_cells: usize,
    cells: &mut usize,
) -> Result<bool> {
    let proof = if prepared && *work == max_work {
        None
    } else {
        curve_surface_agreement::verify_exact(c, p, s, reversed, max_work - *work)?
    };
    if let Some(proof) = proof {
        if proof.work_used > max_work - *work {
            return Err(refuse(
                "BREP_PATCH_NETWORK_BUDGET",
                "Exact correspondence accounting exceeds allowance",
            ));
        }
        *work += proof.work_used;
        match proof.outcome {
            cad_predicates::BezierIdentity::Equal => return Ok(true),
            cad_predicates::BezierIdentity::Different if !prepared => return Ok(false),
            _ if !prepared => {
                return Err(refuse(
                    "BREP_PATCH_NETWORK_UNPROVEN",
                    "Complete boundary equality remains unproved",
                ));
            }
            _ => {}
        }
    } else if !prepared {
        return Err(refuse(
            "BREP_PATCH_NETWORK_UNPROVEN",
            "Boundary correspondence is not supported by the exact prover",
        ));
    }
    prepared_lift(c, p, s, reversed, tolerance, max_cells, cells)
}
mod clocks;
pub(crate) use clocks::{needs_prepared, prepared_clock};
pub(crate) fn prepared_lift(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    tolerance: f64,
    max_cells: usize,
    cells: &mut usize,
) -> Result<bool> {
    if *cells >= max_cells {
        return Err(refuse(
            "BREP_PATCH_NETWORK_BUDGET",
            "Prepared face lift work exhausted",
        ));
    }
    let proof = curve_surface_agreement::verify(c, p, s, reversed, tolerance, max_cells - *cells)?;
    if proof.cells > max_cells - *cells {
        return Err(refuse(
            "BREP_PATCH_NETWORK_BUDGET",
            "Prepared face lift exceeds allowance",
        ));
    }
    *cells += proof.cells;
    match proof.status {
        curve_surface_agreement::Status::WithinTolerance => Ok(true),
        curve_surface_agreement::Status::Mismatch => Ok(false),
        _ => Err(refuse(
            "BREP_PATCH_NETWORK_UNPROVEN",
            "Complete prepared shared face lift remains unresolved",
        )),
    }
}
impl Assembly {
    /// Fresh geometric admission on the saved sewn model. Projection matrices
    /// are proposals only; they are independently checked on every face.
    pub fn qualify(
        &self,
        proposals: &[Option<[[i8; 3]; 2]>],
    ) -> Result<crate::volume_validity::Report> {
        use crate::{boundary_embedding, face_contacts, sweep_cap_contacts, volume_validity};
        if !proposals.is_empty() && proposals.len() != self.model.faces.len() {
            return Err(refuse(
                "BREP_PATCH_NETWORK_LIMITS",
                "One projection proposal per face is required",
            ));
        }
        let contacts = face_contacts::Limits {
            pairs: 10000,
            cells: 100000,
            domain_cells: 1000000,
            cells_per_pair: 1000,
            domain_cells_per_pair: 10000,
        };
        let limits = volume_validity::Limits {
            boundary: boundary_embedding::Limits {
                exact_work: 1000000,
                trim_pairs: 10000,
                trim_cells: 100000,
                trim_domain_cells: 1000000,
                spans: 1024,
                contacts,
            },
            nesting_pairs: 100,
            nesting_cells: 100000,
            nesting_domain_cells: 1000000,
            orientation_cells: 100000,
            orientation_domain_cells: 1000000,
            orientation_spans: 100,
        };
        volume_validity::inspect_sweep_with_projections(
            &self.model,
            1e-8,
            limits,
            100000,
            &[],
            sweep_cap_contacts::Budgets {
                max_walls: 1024,
                max_exact_work: 1000000,
                max_chart_cells: 1000,
                max_trim_pairs: 10000,
                max_trim_cells: 100000,
                max_trim_domain_cells: 1000000,
            },
            proposals,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn patches(model: &Model) -> (Vec<Surface>, Vec<bool>) {
        let surfaces = model.faces.iter().map(|f| f.surface.clone()).collect();
        let mut reversed = vec![false; model.faces.len()];
        for u in &model.shells[0].faces {
            reversed[u.face] = u.reversed;
        }
        (surfaces, reversed)
    }
    #[test]
    fn reconstructs_closed_rational_rounds_without_endpoint_only_admission() {
        let built = crate::analytic::g2_annular::cylinder(10., 7., 20., 0.1, 1e-8).unwrap();
        let (surfaces, reversed) = patches(&built.model);
        let before = built.model.clone();
        let sewn = assemble(&surfaces, &reversed, 1e-8, 1000000, &[&built.model]).unwrap();
        assert_eq!(sewn.model.faces.len(), 32);
        assert_eq!(sewn.model.edges.len(), 64);
        assert_eq!(
            sewn.model.1.faces, built.model.1.faces,
            "Unchanged support/transition faces retain native identities"
        );
        assert_eq!(sewn.directly_owned_uses, 64);
        assert!(sewn.correspondence_work <= 1000000);
        let jets = crate::sweep_smoothness::inspect_profile(&sewn.model, &[], 2000000).unwrap();
        assert!(jets.profile.g2_certified && jets.station.g2_certified);
        assert!(sewn.qualify(&built.projection_hints()).unwrap().proven);
        assert_eq!(built.model, before);
    }
    #[test]
    fn missing_end_cap_wrong_curve_duplicate_face_and_budget_refuse_atomically() {
        let source = crate::cuboid([0.; 3], [10.; 3]).unwrap();
        let saved = source.clone();
        let (surfaces, reversed) = patches(&source);
        assert!(assemble(&surfaces[..5], &reversed[..5], 1e-8, 1000000, &[&source]).is_err());
        assert!(assemble(&surfaces, &reversed, 1e-8, 0, &[&source]).is_err());
        let mut duplicate = surfaces.clone();
        duplicate.push(surfaces[0].clone());
        let mut orientations = reversed.clone();
        orientations.push(reversed[0]);
        assert!(assemble(&duplicate, &orientations, 1e-8, 1000000, &[&source]).is_err());
        let mut wrong = surfaces.clone();
        // Elevate one face so a boundary can keep its endpoints while bowing.
        wrong[0] =
            nurbs_core::surface_edit::elevate(&wrong[0], nurbs_core::surface::Axis::U, 2).unwrap();
        wrong[0].control_points[1][0][2] += 0.125;
        assert!(assemble(&wrong, &reversed, 1e-8, 1000000, &[&source]).is_err());
        let touching = crate::cuboid([10.; 3], [20.; 3]).unwrap();
        let (other, orientation) = patches(&touching);
        let mut pinched = surfaces.clone();
        pinched.extend(other);
        let mut flags = reversed.clone();
        flags.extend(orientation);
        let error = assemble(&pinched, &flags, 1e-8, 1000000, &[&source, &touching])
            .err()
            .unwrap();
        assert_eq!(error.code, "BREP_PATCH_NETWORK_VERTEX_LINK");
        assert_eq!(source, saved);
    }
    #[test]
    fn noncircular_rational_edge_network_retains_its_actual_curvature_and_closes() {
        let built = crate::analytic::g2_annular::cylinder(8., 4., 16., 0.5, 1e-8).unwrap();
        let (mut surfaces, reversed) = patches(&built.model);
        for s in &mut surfaces {
            for p in s.control_points.iter_mut().flatten() {
                p[0] *= 2.;
                p[1] *= 0.5;
            }
        }
        let sewn = assemble(&surfaces, &reversed, 1e-8, 1000000, &[]).unwrap();
        let jets = crate::sweep_smoothness::inspect_profile(&sewn.model, &[], 2000000).unwrap();
        assert!(jets.profile.g2_certified && jets.station.g2_certified);
        assert!(sewn.qualify(&[]).unwrap().proven);
    }
}
