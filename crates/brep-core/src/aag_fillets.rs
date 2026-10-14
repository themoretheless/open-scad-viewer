//! Fillet-chain recognition over the AAG (checklist 869), for later
//! feature suppression (870).
//!
//! A fillet chain is a connected run of small-radius blend faces joined by
//! tangential ([`DihedralClass::Smooth`] / [`DihedralClass::Tangent`]) edges,
//! terminating on two *support* faces — the faces the ball rolled on. Three
//! signatures are reported: constant-radius (rolling ball), variable-radius
//! and face-fillet.
//!
//! Corner resolution (the valence-3 pitfall): a corner blend face (the
//! sphere octant of `exact_valence3_corner_blend`) is tangentially adjacent
//! to every meeting chain. Chains are built on the candidate graph with
//! junction faces — candidate faces with three or more tangential candidate
//! neighbors — removed; each junction face is then assigned to exactly one
//! adjacent chain (the one with the smallest minimum face index); corner
//! faces that fail the candidacy bar but touch a chain tangentially are
//! absorbed by the same rule, so face sets partition and every reported
//! chain edge sits in exactly one chain.
//! This mirrors the deterministic corner ownership of
//! `analytic_features::exact_valence3_corner_blend`.
//!
//! Radii come from the cached best-fit [`FaceAttrs::radius`] (or an on-demand
//! best-fit classification when attributes were never attached), falling back
//! to principal curvatures sampled from the surface evaluation
//! (`mean`/`gaussian` → `k1,2 = H ± √(H²−K)`).

use crate::aag::{Aag, DihedralClass};
use crate::analysis::surface_classify::{Axis, SurfaceClass, classify_surface};
use crate::{Error, Model, Result};
use nurbs_core::foundation::guards::{Budget, BudgetGuard, require_finite_f64};
use nurbs_core::surface::SurfaceSampler;
use std::collections::{BTreeMap, BTreeSet};

/// Default relative spread (`(max - min) / max`) below which a chain reads as
/// constant-radius: 1%, matching the acceptance tolerance.
pub const DEFAULT_CONSTANT_RADIUS_TOLERANCE: f64 = 0.01;
/// Interior samples per UV direction for the curvature fallback.
const CURVATURE_SAMPLES: usize = 3;
/// |cos| threshold between an edge tangent and the fillet axis above which
/// the tangential contact edge counts as longitudinal (rolling) rather than
/// transverse (chain end / corner junction).
const LONGITUDINAL_COS_MIN: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// Fillet-chain signature.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FilletKind {
    /// Rolling-ball blend: face radii agree within tolerance.
    ConstantRadius,
    /// Radii vary along the chain beyond tolerance.
    VariableRadius,
    /// Blend between faces that meet the chain through transverse contact
    /// lines only (no longitudinal rolling edge on either support).
    FaceFillet,
}

/// One recognized fillet chain.
#[derive(Clone, Debug, PartialEq)]
pub struct FilletChain {
    /// Face indices, ascending; includes at most the junction (corner) faces
    /// assigned to this chain. Face sets of distinct chains never overlap.
    pub faces: Vec<usize>,
    /// Internal tangential B-rep edges (both endpoint faces in `faces`),
    /// ascending. Edge lists of distinct chains never overlap.
    pub edges: Vec<usize>,
    pub kind: FilletKind,
    /// Smallest / largest blend radius across chain faces.
    pub radius_min: f64,
    pub radius_max: f64,
    /// The two support faces the blend rolls on, ascending by face index.
    pub supports: [usize; 2],
}

/// Tuning for [`find_fillet_chains_with`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FilletChainOptions {
    /// Radius at or below which a rotational face may join a chain. `None`
    /// defaults to 20% of the model bbox diagonal — deliberately more
    /// generous than [`crate::aag::FaceAttrsOptions::fillet_radius_max`],
    /// because candidacy is confirmed by tangential connectivity anyway and
    /// suppression targets like "r = 0.1 · box" must not fall out by default.
    pub radius_max: Option<f64>,
    /// Relative radius spread below which a chain is
    /// [`FilletKind::ConstantRadius`]. Defaults to
    /// [`DEFAULT_CONSTANT_RADIUS_TOLERANCE`].
    pub constant_radius_tolerance: f64,
}

impl Default for FilletChainOptions {
    fn default() -> Self {
        Self {
            radius_max: None,
            constant_radius_tolerance: DEFAULT_CONSTANT_RADIUS_TOLERANCE,
        }
    }
}

fn fillet_error(message: impl Into<String>) -> Error {
    Error::new("BREP_AAG_FILLETS_INPUT", message)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn model_bbox_diagonal(model: &Model) -> f64 {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in model
        .faces
        .iter()
        .flat_map(|f| f.surface.control_points.iter().flatten())
    {
        for i in 0..3.min(p.len()) {
            lo[i] = lo[i].min(p[i]);
            hi[i] = hi[i].max(p[i]);
        }
    }
    let diag = norm([hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]]);
    if diag.is_finite() && diag > 0. { diag } else { 1. }
}

/// Per-face geometry snapshot used by the matcher.
#[derive(Clone, Copy, Debug)]
struct FaceInfo {
    class: SurfaceClass,
    axis: Option<Axis>,
    radius: Option<f64>,
}

/// Smallest principal radius of curvature of one surface, from sampled
/// mean/gaussian curvatures (`k1,2 = H ± √(H²−K)`). `None` when no sample
/// produced a finite nonzero curvature (planar or singular).
fn principal_radius(surface: &crate::Face, budget: &BudgetGuard) -> Result<Option<f64>> {
    let sampler = SurfaceSampler::new(&surface.surface)?;
    let knots_u = &surface.surface.knots_u;
    let knots_v = &surface.surface.knots_v;
    let du = surface.surface.degree_u;
    let dv = surface.surface.degree_v;
    if knots_u.len() < 2 * (du + 1) || knots_v.len() < 2 * (dv + 1) {
        return Ok(None);
    }
    let u_range = [knots_u[du], knots_u[knots_u.len() - 1 - du]];
    let v_range = [knots_v[dv], knots_v[knots_v.len() - 1 - dv]];
    let mut best: Option<f64> = None;
    let _ = budget;
    for i in 1..=CURVATURE_SAMPLES {
        for j in 1..=CURVATURE_SAMPLES {
            let u = u_range[0]
                + (u_range[1] - u_range[0]) * i as f64 / (CURVATURE_SAMPLES + 1) as f64;
            let v = v_range[0]
                + (v_range[1] - v_range[0]) * j as f64 / (CURVATURE_SAMPLES + 1) as f64;
            let eval = sampler.evaluate(u, v)?;
            let Some((k, h)) = eval.curvatures() else {
                continue;
            };
            require_finite_f64(h, "mean_curvature")?;
            require_finite_f64(k, "gaussian_curvature")?;
            let disc = (h * h - k).max(0.).sqrt();
            for curvature in [h + disc, h - disc] {
                let magnitude = curvature.abs();
                if magnitude > 1e-12 && magnitude.is_finite() {
                    let r = 1. / magnitude;
                    best = Some(best.map_or(r, |b: f64| b.min(r)));
                }
            }
        }
    }
    Ok(best)
}

/// Recognize fillet chains with default options. See
/// [`find_fillet_chains_with`].
pub fn find_fillet_chains(model: &Model, aag: &Aag, budget: &Budget) -> Result<Vec<FilletChain>> {
    find_fillet_chains_with(model, aag, budget, &FilletChainOptions::default())
}

/// Recognize fillet chains over `aag`.
///
/// The graph must cover every face of `model`. Face attributes are read from
/// the cache when present ([`Aag::attach_face_attrs`]); missing attributes are
/// recomputed locally, so the matcher is correct on a plain graph too —
/// attaching once is simply cheaper across repeated queries.
///
/// Every face classification, edge sampling and graph step ticks `budget`;
/// exhaustion aborts with a typed resource error, never a partial chain list.
pub fn find_fillet_chains_with(
    model: &Model,
    aag: &Aag,
    budget: &Budget,
    options: &FilletChainOptions,
) -> Result<Vec<FilletChain>> {
    if aag.nodes.len() != model.faces.len() {
        return Err(fillet_error("AAG node count does not match model faces"));
    }
    require_finite_f64(
        options.constant_radius_tolerance,
        "constant_radius_tolerance",
    )?;
    if !(options.constant_radius_tolerance > 0.) {
        return Err(fillet_error("constant_radius_tolerance must be positive"));
    }
    let diagonal = model_bbox_diagonal(model);
    let radius_max = options.radius_max.unwrap_or(0.2 * diagonal);
    require_finite_f64(radius_max, "radius_max")?;
    if radius_max <= 0. {
        return Err(fillet_error("radius_max must be positive"));
    }
    let fit_tolerance = (1e-4 * diagonal).max(1e-6);
    let mut guard: BudgetGuard = budget.guard("aag-fillet-chains");
    guard.check()?;

    // 1. Per-face geometry: cached attributes when present, best-fit
    //    classification otherwise; radius falls back to principal curvature.
    let mut infos = Vec::with_capacity(model.faces.len());
    for (face, node) in aag.nodes.iter().enumerate() {
        guard.tick()?;
        let (class, axis, radius) = if let Some(attrs) = &node.attrs {
            (attrs.class, attrs.axis, attrs.radius)
        } else {
            let c = classify_surface(&model.faces[face].surface, fit_tolerance, budget)?;
            (c.class, c.axis, c.radius)
        };
        let radius = match radius {
            Some(r) => Some(r),
            None => principal_radius(&model.faces[face], &guard)?,
        };
        if let Some(r) = radius {
            require_finite_f64(r, "face radius")?;
        }
        infos.push(FaceInfo { class, axis, radius });
    }

    // 2. Candidate blend faces: small-radius rotational or spherical faces
    //    with at least two distinct tangential neighbors.
    let tangential = |class: DihedralClass| {
        matches!(class, DihedralClass::Smooth | DihedralClass::Tangent)
    };
    let mut tangential_neighbors: Vec<BTreeSet<usize>> =
        vec![BTreeSet::new(); model.faces.len()];
    for edge in &aag.edges {
        guard.tick()?;
        if !tangential(edge.class) || edge.uses.len() != 2 {
            continue;
        }
        let (a, b) = (edge.uses[0].face, edge.uses[1].face);
        if a != b {
            tangential_neighbors[a].insert(b);
            tangential_neighbors[b].insert(a);
        }
    }
    let is_candidate = |face: usize| -> bool {
        let info = &infos[face];
        let blend_surface = matches!(
            info.class,
            SurfaceClass::Cylinder | SurfaceClass::Torus | SurfaceClass::Sphere
        ) || (info.class == SurfaceClass::Freeform && info.radius.is_some());
        blend_surface
            && info.radius.is_some_and(|r| r <= radius_max)
            && tangential_neighbors[face].len() >= 2
    };
    let candidates: BTreeSet<usize> = (0..model.faces.len()).filter(|&f| is_candidate(f)).collect();
    if candidates.is_empty() {
        guard.check()?;
        return Ok(Vec::new());
    }

    // 3. Candidate adjacency through tangential edges only.
    let mut cand_adj: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for &face in &candidates {
        guard.tick()?;
        let adjacent: BTreeSet<usize> = tangential_neighbors[face]
            .iter()
            .copied()
            .filter(|other| candidates.contains(other))
            .collect();
        cand_adj.insert(face, adjacent);
    }

    // 4. Junction (corner) faces: candidates touched by three or more
    //    candidate chains. Removing them splits merged corner components.
    let junctions: BTreeSet<usize> = candidates
        .iter()
        .copied()
        .filter(|f| cand_adj[f].len() >= 3)
        .collect();
    let cores: BTreeSet<usize> = candidates.difference(&junctions).copied().collect();

    // 5. Connected components of the core graph, in deterministic order.
    let mut components: Vec<Vec<usize>> = Vec::new();
    let mut visited: BTreeSet<usize> = BTreeSet::new();
    for &seed in &cores {
        guard.tick()?;
        if visited.contains(&seed) {
            continue;
        }
        let mut component = vec![seed];
        visited.insert(seed);
        let mut cursor = 0;
        while cursor < component.len() {
            guard.tick()?;
            let face = component[cursor];
            cursor += 1;
            for &next in &cand_adj[&face] {
                if cores.contains(&next) && visited.insert(next) {
                    component.push(next);
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }
    components.sort_by_key(|c| c[0]);

    // 6. Deterministic junction assignment: each corner face joins the
    //    adjacent chain with the smallest minimum face index. Faces
    //    partition — a junction face never appears in two chains.
    let mut chain_of: BTreeMap<usize, usize> = BTreeMap::new();
    for (index, component) in components.iter().enumerate() {
        for &face in component {
            chain_of.insert(face, index);
        }
    }
    for &junction in &junctions {
        guard.tick()?;
        let adjacent_chains: BTreeSet<usize> = cand_adj[&junction]
            .iter()
            .filter_map(|other| chain_of.get(other).copied())
            .collect();
        if let Some(&target) = adjacent_chains.iter().next() {
            components[target].push(junction);
            components[target].sort_unstable();
            chain_of.insert(junction, target);
        }
    }

    // 6b. Orphan corner absorption: a small-radius blend face that failed the
    //     two-tangential-neighbor candidacy bar yet touches a chain
    //     tangentially (e.g. a corner sphere whose remaining joints classify
    //     non-smooth in the authored geometry) is absorbed into exactly one
    //     adjacent chain — smallest minimum face index wins. Without this the
    //     corner face of a valence-3 blend would escape every chain.
    let blend_surface = |face: usize| -> bool {
        matches!(
            infos[face].class,
            SurfaceClass::Cylinder | SurfaceClass::Torus | SurfaceClass::Sphere
        ) || (infos[face].class == SurfaceClass::Freeform && infos[face].radius.is_some())
    };
    for face in 0..model.faces.len() {
        guard.tick()?;
        if chain_of.contains_key(&face) || !blend_surface(face) {
            continue;
        }
        if !infos[face].radius.is_some_and(|r| r <= radius_max) {
            continue;
        }
        let adjacent_chains: BTreeSet<usize> = tangential_neighbors[face]
            .iter()
            .filter_map(|other| chain_of.get(other).copied())
            .collect();
        if let Some(&target) = adjacent_chains.iter().next() {
            components[target].push(face);
            components[target].sort_unstable();
            chain_of.insert(face, target);
        }
    }

    // 7. Per chain: internal edges, supports, radii, signature.
    let mut chains = Vec::with_capacity(components.len());
    for component in &components {
        guard.tick()?;
        let face_set: BTreeSet<usize> = component.iter().copied().collect();
        let mut edges = Vec::new();
        // Support scoring: longitudinal (rolling) tangential contacts beat
        // transverse ones; ties break on the lower face index.
        let mut longitudinal: BTreeMap<usize, usize> = BTreeMap::new();
        let mut transverse_only: BTreeSet<usize> = BTreeSet::new();
        let mut support_longitudinal_edges = 0usize;
        for edge in &aag.edges {
            guard.tick()?;
            if !tangential(edge.class) || edge.uses.len() != 2 {
                continue;
            }
            let (a, b) = (edge.uses[0].face, edge.uses[1].face);
            let (inside_a, inside_b) = (face_set.contains(&a), face_set.contains(&b));
            if inside_a && inside_b {
                edges.push(edge.edge);
                continue;
            }
            let (chain_face, other) = match (inside_a, inside_b) {
                (true, false) => (a, b),
                (false, true) => (b, a),
                _ => continue,
            };
            // Longitudinal test: edge tangent parallel to the blend axis.
            let mut is_longitudinal = false;
            if let Some(axis) = infos[chain_face].axis {
                let domain = model.edges[edge.edge].curve.domain();
                let mid = 0.5 * (domain[0] + domain[1]);
                let eval = model.edges[edge.edge].curve.evaluate(mid)?;
                if let Some(d1) = &eval.d1 {
                    let t = [d1[0], d1[1], d1[2]];
                    let length = norm(t);
                    if length.is_finite() && length > 0. {
                        let cos = dot(t.map(|v| v / length), axis.direction).abs();
                        is_longitudinal = cos >= LONGITUDINAL_COS_MIN;
                    }
                }
            }
            if is_longitudinal {
                *longitudinal.entry(other).or_insert(0) += 1;
                support_longitudinal_edges += 1;
            } else {
                transverse_only.insert(other);
            }
        }
        edges.sort_unstable();
        edges.dedup();
        // Deterministic support pick: most longitudinal contacts first.
        let mut ranked: Vec<(usize, usize)> = longitudinal
            .iter()
            .map(|(&face, &count)| (face, count))
            .chain(transverse_only.iter().map(|&face| (face, 0)))
            .collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        ranked.dedup_by_key(|entry| entry.0);
        let Some(supports) = ranked.get(..2) else {
            // A chain that does not close on two support faces is not a
            // suppressible fillet chain; skip it rather than guess.
            continue;
        };
        let mut support_pair = [supports[0].0, supports[1].0];
        support_pair.sort_unstable();

        let mut radius_min = f64::INFINITY;
        let mut radius_max_seen = 0f64;
        for &face in component {
            guard.tick()?;
            let Some(r) = infos[face].radius else {
                return Err(fillet_error(
                    "candidate blend face carries no finite radius estimate",
                ));
            };
            radius_min = radius_min.min(r);
            radius_max_seen = radius_max_seen.max(r);
        }
        let spread = (radius_max_seen - radius_min) / radius_max_seen;
        let kind = if spread > options.constant_radius_tolerance {
            FilletKind::VariableRadius
        } else if support_longitudinal_edges == 0 {
            FilletKind::FaceFillet
        } else {
            FilletKind::ConstantRadius
        };
        chains.push(FilletChain {
            faces: component.clone(),
            edges,
            kind,
            radius_min,
            radius_max: radius_max_seen,
            supports: support_pair,
        });
    }
    chains.sort_by_key(|c| c.faces[0]);
    guard.check()?;
    Ok(chains)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytic_features::{
        analytic_fillet, analytic_fillet_chain, exact_valence3_corner_blend,
    };

    fn budget() -> Budget {
        Budget::new(10_000_000, 8, 60_000).unwrap()
    }

    fn attributed(model: &Model) -> Aag {
        let mut aag = Aag::build(model, &budget()).unwrap();
        aag.attach_face_attrs(model, &budget()).unwrap();
        aag
    }

    fn vertical_edges(model: &Model) -> Vec<usize> {
        model
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let a = model.vertices[e.vertices[0]].point;
                let b = model.vertices[e.vertices[1]].point;
                ((a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12).then_some(i)
            })
            .collect()
    }

    #[test]
    fn single_constant_radius_fillet_is_one_chain() {
        let model = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let edges = vertical_edges(&model);
        let (out, _) = analytic_fillet(&model, edges[0], 1.).unwrap();
        let aag = attributed(&out);
        let chains = find_fillet_chains(&out, &aag, &budget()).unwrap();
        assert_eq!(chains.len(), 1, "one fillet → one chain: {chains:?}");
        let chain = &chains[0];
        assert_eq!(chain.kind, FilletKind::ConstantRadius);
        // Radius estimate within the 1% acceptance bound.
        assert!(
            (chain.radius_min - 1.).abs() <= 0.01 && (chain.radius_max - 1.).abs() <= 0.01,
            "radius estimate must hit 1%: {:?}",
            (chain.radius_min, chain.radius_max)
        );
        // The single cylindrical blend face, terminating on two planar walls.
        assert_eq!(chain.faces.len(), 1);
        let class = aag.nodes[chain.faces[0]].attrs.as_ref().unwrap().class;
        assert_eq!(class, SurfaceClass::Cylinder);
        for support in chain.supports {
            let class = aag.nodes[support].attrs.as_ref().unwrap().class;
            assert_eq!(class, SurfaceClass::Plane, "supports are the two walls");
        }
        assert_ne!(chain.supports[0], chain.supports[1]);
        // A single-face chain has no internal tangential edges by definition;
        // its two smooth cylinder–wall contacts are support contacts.
        assert!(chain.edges.is_empty());
    }

    #[test]
    fn fillet_chain_constructor_yields_one_chain_per_corner() {
        let model = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let edges = vertical_edges(&model);
        assert!(edges.len() >= 2);
        let (out, _) = analytic_fillet_chain(&model, &edges[..2], 0.8).unwrap();
        let aag = attributed(&out);
        let chains = find_fillet_chains(&out, &aag, &budget()).unwrap();
        assert_eq!(chains.len(), 2, "two rounded corners → two chains: {chains:?}");
        for chain in &chains {
            assert_eq!(chain.kind, FilletKind::ConstantRadius);
            assert!(
                (chain.radius_max - 0.8).abs() <= 0.008,
                "radius estimate within 1% of 0.8: {}",
                chain.radius_max
            );
        }
    }

    #[test]
    fn valence3_corner_splits_into_three_disjoint_chains() {
        let source = crate::cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
        let max = [10., 8., 6.];
        let edges: Vec<usize> = source
            .edges
            .iter()
            .enumerate()
            .filter_map(|(index, edge)| {
                let a = source.vertices[edge.vertices[0]].point;
                let b = source.vertices[edge.vertices[1]].point;
                let mid = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1]), 0.5 * (a[2] + b[2])];
                let touches = edge.vertices.iter().any(|&v| {
                    let p = source.vertices[v].point;
                    (0..3).all(|i| (p[i] - max[i]).abs() <= 1e-9)
                });
                let on_pair = |i: usize, j: usize| {
                    (mid[i] - max[i]).abs() <= 1e-9 && (mid[j] - max[j]).abs() <= 1e-9
                };
                (touches && (on_pair(0, 1) || on_pair(0, 2) || on_pair(1, 2))).then_some(index)
            })
            .collect();
        assert_eq!(edges.len(), 3);
        let out = exact_valence3_corner_blend(&source, &edges, 1.).unwrap();
        let aag = attributed(&out.model);
        let chains = find_fillet_chains(&out.model, &aag, &budget()).unwrap();
        assert_eq!(
            chains.len(),
            3,
            "corner blend splits into three chains, got {chains:?}"
        );
        // Cutting invariant: faces and edges partition — no face or edge is
        // claimed by two chains.
        let mut all_faces = BTreeSet::new();
        let mut all_edges = BTreeSet::new();
        for chain in &chains {
            for &face in &chain.faces {
                assert!(all_faces.insert(face), "face {face} in two chains");
            }
            for &edge in &chain.edges {
                assert!(all_edges.insert(edge), "edge {edge} in two chains");
                let class = aag.edge(edge).unwrap().class;
                assert!(
                    matches!(class, DihedralClass::Smooth | DihedralClass::Tangent),
                    "chain edges are tangential: {class:?}"
                );
            }
            assert!(
                (chain.radius_max - 1.).abs() <= 0.01,
                "equal-radius corner blend reads 1.0 within 1%: {}",
                chain.radius_max
            );
        }
        // The corner sphere was claimed by exactly one chain.
        let sphere_faces: usize = chains
            .iter()
            .flat_map(|c| c.faces.iter())
            .filter(|&&f| {
                aag.nodes[f].attrs.as_ref().unwrap().class == SurfaceClass::Sphere
            })
            .count();
        assert_eq!(sphere_faces, 1, "corner face assigned exactly once");
    }

    #[test]
    fn plain_cuboid_has_no_fillet_chains() {
        let model = crate::cuboid([0., 0., 0.], [4., 3., 2.]).unwrap();
        let aag = attributed(&model);
        let chains = find_fillet_chains(&model, &aag, &budget()).unwrap();
        assert!(chains.is_empty(), "no fillets → empty list: {chains:?}");
    }

    #[test]
    fn missing_face_attrs_are_computed_on_demand() {
        let model = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let edges = vertical_edges(&model);
        let (out, _) = analytic_fillet(&model, edges[0], 1.).unwrap();
        let aag = Aag::build(&out, &budget()).unwrap(); // no attach_face_attrs
        let chains = find_fillet_chains(&out, &aag, &budget()).unwrap();
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].kind, FilletKind::ConstantRadius);
        assert!((chains[0].radius_max - 1.).abs() <= 0.01);
    }

    #[test]
    fn budget_exhaustion_is_a_typed_error() {
        let model = crate::cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
        let edges = vertical_edges(&model);
        let (out, _) = analytic_fillet(&model, edges[0], 1.).unwrap();
        let aag = attributed(&out);
        let tight = Budget::with_iterations(2).unwrap();
        let error = find_fillet_chains(&out, &aag, &tight).unwrap_err();
        assert!(
            error.code.contains("RESOURCE") || error.code.contains("BUDGET"),
            "budget exhaustion must surface as a resource error: {error:?}"
        );
        assert!(find_fillet_chains(&out, &aag, &budget()).is_ok());
    }

    #[test]
    fn mismatched_graph_is_rejected() {
        let model = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
        let other = crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let aag = Aag::build(&other, &budget()).unwrap();
        // Same face count here, so force a mismatch through an empty graph.
        let empty = Aag {
            nodes: Vec::new(),
            edges: Vec::new(),
        };
        assert!(find_fillet_chains(&model, &empty, &budget()).is_err());
        let _ = aag;
    }
}
