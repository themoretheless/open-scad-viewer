use super::*;
use crate::closed_solid::{
    BooleanFace, BooleanFaceDisposition, BooleanFaceSeedLimits, BooleanMaterialOp,
    BooleanSourceSide, QualifiedClosedSolid, ShellRepairPolicy, rebuild_even_odd_shells,
};
use num_rational::BigRational;
use num_traits::ToPrimitive;
use nurbs_core::curve_partition_agreement::{
    verify_exact_bspline_restriction, verify_source_closure,
};
use std::collections::{BTreeMap, BTreeSet};

type Entity = (usize, usize);
type EdgeAliases = BTreeMap<Entity, (Entity, bool)>;

impl QualifiedOpenSSNetwork<'_, '_, '_, '_, '_> {
    /// Complete the rim-connected components of this open SS intersection
    /// using uniform source faces and exact common outward coplanar patches.
    /// Components that do not meet this network require their own SS coverage.
    pub fn complete_source_intersection(
        &self,
        solids: [&QualifiedClosedSolid; 2],
        directions: &[[f64; 3]],
        seeds: BooleanFaceSeedLimits,
        limits: OpenSSIntersectionCompletionLimits,
        policy: ShellRepairPolicy,
    ) -> Result<Vec<QualifiedClosedSolid>> {
        self.complete_source_intersection_with_networks(
            &[], solids, directions, seeds, limits, policy,
        )
    }

    /// Complete intersected components from independently qualified face pairs.
    /// Each original face must have a single partition owner. Rims between
    /// networks are checked by exact oriented curve identity during assembly.
    /// Components outside all supplied networks require separate coverage.
    pub fn complete_source_intersection_with_networks(
        &self,
        others: &[&Self],
        solids: [&QualifiedClosedSolid; 2],
        directions: &[[f64; 3]],
        seeds: BooleanFaceSeedLimits,
        limits: OpenSSIntersectionCompletionLimits,
        policy: ShellRepairPolicy,
    ) -> Result<Vec<QualifiedClosedSolid>> {
        if [
            limits.face_pairs,
            limits.polygon_tests_per_pair,
            limits.rim_pairs,
        ]
        .into_iter()
        .any(|n| !(1..=1_000_000).contains(&n))
        {
            return Err(invalid("Need bounded open SS intersection completion"));
        }
        let networks = std::iter::once(self).chain(others.iter().copied());
        let mut cuts = [BTreeSet::new(), BTreeSet::new()];
        let mut tolerance_uv = f64::INFINITY;
        for network in networks.clone() {
            let (faces, tolerance) = network.original_boolean_faces(solids)?;
            for (side, face) in faces.into_iter().enumerate() {
                if !cuts[side].insert(face) {
                    return Err(unproven("Open SS networks need a joint partition for a shared source face"));
                }
            }
            tolerance_uv = tolerance_uv.min(tolerance);
        }
        let [a, b] = solids.map(QualifiedClosedSolid::model);
        let count = (a.faces.len() - cuts[0].len())
            .saturating_mul(b.faces.len() - cuts[1].len());
        if count > limits.face_pairs {
            return Err(crate::Error::new(
                "BREP_RESOURCE_LIMIT",
                "Open SS completion exceeded its face-pair budget",
            ));
        }
        let operation = BooleanMaterialOp::Intersection;
        let mut candidates = discover_uncut_patches(
            solids, operation, directions, seeds, tolerance_uv,
            |side, face| cuts[side].contains(&face),
        )?;
        for first in (0..a.faces.len()).filter(|face| !cuts[0].contains(face)) {
            for second in (0..b.faces.len()).filter(|face| !cuts[1].contains(face)) {
                match crate::boolean::operations::exact_coplanar_intersection_patches(
                    solids,
                    [first, second],
                    limits.polygon_tests_per_pair,
                ) {
                    Ok(patches) => candidates.extend(patches),
                    Err(error) if error.code == "BREP_UNSUPPORTED_OPERATION" => {}
                    Err(error) => return Err(error),
                }
            }
        }
        let mut retained = Vec::new();
        for network in networks {
            retained.extend(network.sew_boolean_patches(solids, operation, directions, seeds)?);
        }
        discover_patch_components(&retained, &candidates, limits.rim_pairs, policy)
    }

    /// Build boundary candidates from complete source faces with a proved
    /// uniform material decision. Faces needing further trim are omitted.
    pub fn discover_uncut_boolean_patches(
        &self,
        solids: [&QualifiedClosedSolid; 2],
        operation: BooleanMaterialOp,
        directions: &[[f64; 3]],
        limits: BooleanFaceSeedLimits,
    ) -> Result<Vec<Model>> {
        let (faces, tolerance_uv) = self.original_boolean_faces(solids)?;
        discover_uncut_patches(solids, operation, directions, limits, tolerance_uv,
            |side, face| face == faces[side])
    }

    fn original_boolean_faces(
        &self, solids: [&QualifiedClosedSolid; 2],
    ) -> Result<([usize; 2], f64)> {
        let input = match &self.source {
            NetworkSource::Paths(paths) => paths[0].input,
            NetworkSource::Boundary { input, .. } => input,
        };
        let original = input.source.roots.faces;
        if original
            .iter()
            .zip(solids)
            .any(|((model, _), solid)| *model != solid.model())
        {
            return Err(unproven(
                "Uncut Boolean patches need the original SS operands",
            ));
        }
        Ok((original.map(|(_, face)| face), input.source.limits.uv_tolerance))
    }

    /// Find the exact rim-connected component of each selected SS patch in a
    /// pool of oriented boundary candidates and certify the resulting solids.
    pub fn discover_selected_ss_solids(
        &self,
        solids: [&QualifiedClosedSolid; 2],
        operation: BooleanMaterialOp,
        directions: &[[f64; 3]],
        limits: BooleanFaceSeedLimits,
        candidates: &[Model],
        max_pairs: usize,
        policy: ShellRepairPolicy,
    ) -> Result<Vec<QualifiedClosedSolid>> {
        if !(1..=1_000_000).contains(&max_pairs) {
            return Err(invalid("Need bounded exact rim matching"));
        }
        let patches = self.sew_boolean_patches(solids, operation, directions, limits)?;
        for candidate in candidates {
            candidate.validate()?;
        }
        // Keep the two material halves of XOR independently owned.
        patches.iter().map(|patch| {
            let models: Vec<_> = std::iter::once(patch).chain(candidates).collect();
            let component = discover_rim_component(&models, 0, max_pairs)?;
            let boundary: Vec<_> = component.iter().map(|&slot| models[slot].clone()).collect();
            assemble_exact_patch_solid(&boundary, max_pairs, policy)
        }).collect()
    }

    /// Close each selected SS patch with explicitly supplied boundary patches.
    /// The caller supplies one group per material result, in the order returned
    /// by sew_boolean_patches. The assembly certifies shell geometry.
    pub fn assemble_selected_ss_solids(
        &self,
        solids: [&QualifiedClosedSolid; 2],
        operation: BooleanMaterialOp,
        directions: &[[f64; 3]],
        limits: BooleanFaceSeedLimits,
        completion: &[&[Model]],
        max_pairs: usize,
        policy: ShellRepairPolicy,
    ) -> Result<Vec<QualifiedClosedSolid>> {
        let patches = self.sew_boolean_patches(solids, operation, directions, limits)?;
        if patches.len() != completion.len() {
            return Err(invalid(
                "Each selected SS patch needs one boundary completion group",
            ));
        }
        patches
            .into_iter()
            .zip(completion)
            .map(|(patch, remaining)| {
                let mut boundary = Vec::with_capacity(remaining.len() + 1);
                boundary.push(patch);
                boundary.extend_from_slice(remaining);
                assemble_exact_patch_solid(&boundary, max_pairs, policy)
            })
            .collect()
    }

    /// Weld the selected faces incident to the certified SS seams. Other
    /// operand rims remain open for subsequent shell assembly. XOR returns
    /// independently owned patches for A−B and B−A, in that order.
    pub fn sew_boolean_patches(
        &self,
        solids: [&QualifiedClosedSolid; 2],
        operation: BooleanMaterialOp,
        directions: &[[f64; 3]],
        limits: BooleanFaceSeedLimits,
    ) -> Result<Vec<Model>> {
        let seams = self.qualify_boolean_seams(solids, operation, directions, limits)?;
        if seams.is_empty() {
            return Ok(Vec::new());
        }
        let groups = if operation == BooleanMaterialOp::Xor {
            2
        } else {
            1
        };
        (0..groups)
            .map(|group| sew_patches(&self.models, seams.iter().skip(group).step_by(groups)))
            .collect()
    }
}

fn discover_uncut_patches(
    solids: [&QualifiedClosedSolid; 2],
    operation: BooleanMaterialOp,
    directions: &[[f64; 3]],
    limits: BooleanFaceSeedLimits,
    tolerance_uv: f64,
    is_cut: impl Fn(usize, usize) -> bool,
) -> Result<Vec<Model>> {
    let mut patches = Vec::new();
    for (side, source_side) in [BooleanSourceSide::First, BooleanSourceSide::Second]
        .into_iter()
        .enumerate()
    {
        let source = solids[side].model();
        for use_ in source.shells.iter().flat_map(|shell| &shell.faces) {
            let face = use_.face;
            if is_cut(side, face) {
                continue;
            }
            let query = BooleanFace {
                solids,
                operation,
                side: source_side,
                face,
            };
            let result = super::material::prove_face_material(
                query,
                source,
                face,
                tolerance_uv,
                directions,
                limits,
            );
            let reverse = match result {
                Ok(BooleanFaceDisposition::Keep) => false,
                Ok(BooleanFaceDisposition::Reverse) => true,
                _ => continue,
            };
            patches.push(assemble_faces(
                std::slice::from_ref(source),
                BTreeMap::from([((0, face), use_.reversed ^ reverse)]),
                &BTreeMap::new(),
                &BTreeMap::new(),
            )?);
        }
    }
    Ok(patches)
}

fn discover_patch_components(
    seeds: &[Model], candidates: &[Model], max_pairs: usize, policy: ShellRepairPolicy,
) -> Result<Vec<QualifiedClosedSolid>> {
    let models: Vec<_> = seeds.iter().chain(candidates).collect();
    for model in &models {
        model.validate()?;
    }
    let mut consumed = BTreeSet::new();
    let mut results = Vec::new();
    for seed in 0..seeds.len() {
        if consumed.contains(&seed) {
            continue;
        }
        let component = discover_rim_component(&models, seed, max_pairs)?;
        if component.iter().any(|slot| consumed.contains(slot)) {
            return Err(unproven("Open SS components share an oriented patch"));
        }
        let boundary: Vec<_> = component.iter().map(|&slot| models[slot].clone()).collect();
        results.push(assemble_exact_patch_solid(&boundary, max_pairs, policy)?);
        consumed.extend(component);
    }
    Ok(results)
}

fn discover_rim_component(
    models: &[&Model],
    seed: usize,
    max_pairs: usize,
) -> Result<Vec<usize>> {
    let rims = models
        .iter()
        .map(|model| patch_rim_directions(model))
        .collect::<Result<Vec<_>>>()?;
    let mut buckets = BTreeMap::<_, Vec<Entity>>::new();
    for (slot, edges) in rims.iter().enumerate() {
        for &edge in edges.keys() {
            buckets
                .entry(rim_key(edge_ports(models[slot], edge)))
                .or_default()
                .push((slot, edge));
        }
    }
    let mut selected = vec![false; models.len()];
    selected[seed] = true;
    let mut pending = vec![seed];
    let mut matched = BTreeMap::new();
    let mut work = 0;
    let mut exact_work = cad_predicates::MAX_WORK;
    while let Some(slot) = pending.pop() {
        for (&edge, &direction) in &rims[slot] {
            if matched.contains_key(&(slot, edge)) {
                continue;
            }
            let ports = edge_ports(models[slot], edge);
            let mut mate = None;
            for &(other, e) in &buckets[&rim_key(ports)] {
                if slot == other {
                    continue;
                }
                spend(&mut work, max_pairs, 1)?;
                let reversed = direction == rims[other][&e];
                if oriented_rim_match(
                    [(models[slot], edge), (models[other], e)], reversed, &mut exact_work,
                )?
                    && mate.replace((other, e)).is_some()
                {
                    return Err(unproven("Exact patch rim mate is ambiguous"));
                }
            }
            let (other, e) = mate.ok_or_else(|| unproven("Exact patch rim has no mate"))?;
            if matched.insert((other, e), (slot, edge)).is_some() {
                return Err(unproven("Exact patch rim mate is already paired"));
            }
            matched.insert((slot, edge), (other, e));
            if !std::mem::replace(&mut selected[other], true) {
                pending.push(other);
            }
        }
    }
    Ok(selected
        .iter()
        .enumerate()
        .filter_map(|(i, &keep)| keep.then_some(i))
        .collect())
}

fn edge_ports(model: &Model, edge: usize) -> [[f64; 3]; 2] {
    model.edges[edge].vertices.map(|v| model.vertices[v].point)
}

fn patch_rim_directions(model: &Model) -> Result<BTreeMap<usize, bool>> {
    let mut rims = brep_topology::explorer::boundary_edges(&model.0)
        .into_iter().map(|edge| (edge, None)).collect::<BTreeMap<_, _>>();
    for use_ in model.shells.iter().flat_map(|shell| &shell.faces) {
        let face = &model.faces[use_.face];
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for coedge in &model.loops[wire].coedges {
                if let Some(direction) = rims.get_mut(&coedge.edge)
                    && direction.replace(use_.reversed ^ coedge.reversed).is_some()
                {
                    return Err(unproven("Exact patch rim has multiple oriented uses"));
                }
            }
        }
    }
    rims.into_iter().map(|(edge, direction)| {
        direction.map(|direction| (edge, direction))
            .ok_or_else(|| unproven("Exact patch rim has no oriented use"))
    }).collect()
}

fn oriented_rim_match(
    [(a, edge), (b, other)]: [(&Model, usize); 2],
    reversed: bool,
    work: &mut u64,
) -> Result<bool> {
    let endpoints = edge_ports(a, edge);
    let mut ports = edge_ports(b, other);
    if reversed {
        ports.reverse();
    }
    Ok(endpoints == ports
        && same_curve(&b.edges[other].curve, &a.edges[edge].curve, reversed, work)?)
}

fn bind<T: PartialEq + Copy>(map: &mut BTreeMap<Entity, T>, from: Entity, to: T) -> Result<()> {
    if map.insert(from, to).is_some_and(|prior| prior != to) {
        return Err(unproven("SS seam has inconsistent shared topology"));
    }
    Ok(())
}

fn sew_patches<'a>(
    sources: &[Model],
    seams: impl Iterator<Item = &'a [OpenSSBooleanEdgeUse; 2]>,
) -> Result<Model> {
    let mut edge_aliases = BTreeMap::new();
    let mut vertex_aliases = BTreeMap::new();
    let mut vertex_inverse = BTreeMap::new();
    let mut faces = BTreeMap::new();
    for &[left, right] in seams {
        if left.model == right.model {
            return Err(unproven("SS patch sewing needs distinct operand storage"));
        }
        bind(
            &mut edge_aliases,
            (right.model, right.edge),
            ((left.model, left.edge), false),
        )?;
        let a = &sources[left.model].edges[left.edge];
        let b = &sources[right.model].edges[right.edge];
        for (u, v) in a.vertices.into_iter().zip(b.vertices) {
            bind(&mut vertex_aliases, (right.model, v), (left.model, u))?;
            bind(&mut vertex_inverse, (left.model, u), (right.model, v))?;
        }
        for use_ in [left, right] {
            let source = &sources[use_.model];
            let face = &source.faces[use_.face];
            let coedge = std::iter::once(face.outer)
                .chain(face.holes.iter().copied())
                .flat_map(|wire| &source.loops[wire].coedges)
                .find(|coedge| coedge.edge == use_.edge)
                .ok_or_else(|| unproven("Selected face lost its SS coedge"))?;
            let reversed = coedge.reversed ^ use_.reversed;
            let key = (use_.model, use_.face);
            if faces
                .insert(key, reversed)
                .is_some_and(|prior| prior != reversed)
            {
                return Err(unproven("SS seams disagree on selected face orientation"));
            }
        }
    }
    assemble_faces(sources, faces, &edge_aliases, &vertex_aliases)
}

/// Join complete oriented patch rims by exact edge identity and admit
/// the resulting shell forest as material. No endpoints are moved.
pub fn assemble_exact_patch_solid(
    patches: &[Model],
    max_pairs: usize,
    policy: ShellRepairPolicy,
) -> Result<QualifiedClosedSolid> {
    if patches.is_empty() || !(1..=1_000_000).contains(&max_pairs) {
        return Err(invalid(
            "Need patch geometry and bounded exact rim matching",
        ));
    }
    let mut faces = BTreeMap::new();
    let mut rims = Vec::new();
    let mut directions = Vec::new();
    for (slot, patch) in patches.iter().enumerate() {
        patch.validate()?;
        let start = faces.len();
        for use_ in patch.shells.iter().flat_map(|shell| &shell.faces) {
            if faces.insert((slot, use_.face), use_.reversed).is_some() {
                return Err(unproven("Exact patch face has multiple shell uses"));
            }
        }
        if faces.len() - start != patch.faces.len() {
            return Err(unproven("Exact patch contains an unowned face"));
        }
        for (edge, direction) in patch_rim_directions(patch)? {
            rims.push((slot, edge));
            directions.push(direction);
        }
    }
    let mut edge_aliases = EdgeAliases::new();
    let mut vertex_aliases = BTreeMap::new();
    let mut paired = vec![false; rims.len()];
    let ports: Vec<_> = rims
        .iter()
        .map(|&(slot, edge)| {
            let patch = &patches[slot];
            patch.edges[edge].vertices.map(|v| patch.vertices[v].point)
        })
        .collect();
    let mut buckets = BTreeMap::<_, Vec<usize>>::new();
    for (index, &endpoints) in ports.iter().enumerate() {
        buckets.entry(rim_key(endpoints)).or_default().push(index);
    }
    let mut work = 0;
    let mut exact_work = cad_predicates::MAX_WORK;
    for (i, &(slot, edge)) in rims.iter().enumerate() {
        if paired[i] {
            continue;
        }
        let a = &patches[slot].edges[edge];
        let endpoints = ports[i];
        let mut mate = None;
        let bucket = &buckets[&rim_key(endpoints)];
        for &j in &bucket[bucket.partition_point(|&index| index <= i)..] {
            let (other, e) = rims[j];
            if paired[j] || slot == other {
                continue;
            }
            spend(&mut work, max_pairs, 1)?;
            let reversed = directions[i] == directions[j];
            if oriented_rim_match(
                [(&patches[slot], edge), (&patches[other], e)], reversed, &mut exact_work,
            )?
                && mate.replace((j, reversed)).is_some()
            {
                return Err(unproven("Exact patch rim mate is ambiguous"));
            }
        }
        let (j, reversed) = mate.ok_or_else(|| unproven("Exact patch rim has no mate"))?;
        paired[j] = true;
        let (other, e) = rims[j];
        let mut vertices = patches[other].edges[e].vertices;
        if reversed {
            vertices.reverse();
        }
        for (u, v) in a.vertices.into_iter().zip(vertices) {
            let a = vertex_root(&vertex_aliases, (slot, u));
            let b = vertex_root(&vertex_aliases, (other, v));
            if a != b {
                vertex_aliases.insert(a.max(b), a.min(b));
            }
        }
        // Preserve the original periodic parameterization regardless of the
        // order in which its patches reach the assembly. Authored seam uses
        // can rely on this representation when verifying the surface lift.
        let first = (slot, edge);
        let second = (other, e);
        let (from, to) = if !a.curve.periodic && patches[other].edges[e].curve.periodic {
            (first, second)
        } else {
            (second, first)
        };
        edge_aliases.insert(from, (to, reversed));
    }
    let candidate = assemble_faces(patches, faces, &edge_aliases, &vertex_aliases)?;
    if candidate.shells.is_empty() || candidate.shells.iter().any(|shell| !shell.closed) {
        return Err(unproven("Exact patch sewing leaves an open shell"));
    }
    let admitted = rebuild_even_odd_shells(&candidate, policy);
    admitted.solid().cloned().ok_or_else(|| {
        admitted
            .refusal()
            .cloned()
            .unwrap_or_else(|| unproven("Exact patch shell failed material admission"))
    })
}

fn rim_key(endpoints: [[f64; 3]; 2]) -> [[u64; 3]; 2] {
    let mut key = endpoints.map(|point| point.map(|x| if x == 0. { 0 } else { x.to_bits() }));
    key.sort_unstable();
    key
}

fn vertex_root(aliases: &BTreeMap<Entity, Entity>, mut vertex: Entity) -> Entity {
    while let Some(&canonical) = aliases.get(&vertex) {
        vertex = canonical;
    }
    vertex
}

fn map_clock(parameter: f64, source: [f64; 2], target: [f64; 2], reversed: bool) -> Option<f64> {
    let [source_lo, source_hi] = source.map(BigRational::from_float);
    let [target_lo, target_hi] = target.map(BigRational::from_float);
    let source_lo = source_lo?;
    let target_lo = target_lo?;
    let normalized = (BigRational::from_float(parameter)? - &source_lo) / (source_hi? - source_lo);
    let span = target_hi? - &target_lo;
    let mapped = if reversed {
        &target_lo + &span - normalized * &span
    } else {
        target_lo + normalized * span
    };
    let value = mapped.to_f64()?;
    (BigRational::from_float(value)? == mapped).then_some(value)
}

// Keep exactly the poles and knots read by the nonempty native spans.
fn open_active_basis(curve: &Curve) -> Curve {
    let degree = curve.degree;
    let active = (degree..curve.control_points.len())
        .filter(|&i| curve.knots[i] < curve.knots[i + 1]);
    let first = active.clone().next().expect("validated active curve domain") - degree;
    let last = active.last().expect("validated active curve domain");
    Curve {
        degree,
        knots: curve.knots[first..last + degree + 2].to_vec(),
        control_points: curve.control_points[first..=last].to_vec(),
        weights: curve.weights[first..=last].to_vec(),
        periodic: false,
    }
}

fn same_curve(source: &Curve, target: &Curve, reversed: bool, remaining: &mut u64) -> Result<bool> {
    if !reversed && source == target {
        return Ok(true);
    }
    let domain = source.domain();
    let target_domain = target.domain();
    let mut normalized = source.clone();
    for knot in &mut normalized.knots {
        let Some(mapped) = map_clock(*knot, domain, target_domain, reversed) else {
            return Ok(false);
        };
        *knot = mapped;
    }
    if reversed {
        normalized.knots.reverse();
        normalized.control_points.reverse();
        normalized.weights.reverse();
    }
    if &normalized == target {
        return Ok(true);
    }
    // Compare ordinary B-spline basis functions over one complete native
    // period. Check exact closure first because periodic evaluation identifies
    // the two ends, whereas the interval proof uses their separate limits.
    let open_target;
    let target = if normalized.periodic || target.periodic {
        if !verify_source_closure(&normalized)? || !verify_source_closure(target)? {
            return Ok(false);
        }
        normalized = open_active_basis(&normalized);
        open_target = open_active_basis(target);
        &open_target
    } else {
        target
    };
    if *remaining == 0 {
        return Err(unproven("Exact rim curve proof work exhausted"));
    }
    let Some(proof) = verify_exact_bspline_restriction(
        target,
        &normalized,
        *remaining,
    )?
    else {
        return Ok(false);
    };
    *remaining -= proof.work_used;
    match proof.outcome {
        cad_predicates::BezierIdentity::Equal => Ok(true),
        cad_predicates::BezierIdentity::Different => Ok(false),
        cad_predicates::BezierIdentity::Indeterminate(_) => {
            Err(unproven("Exact rim curve identity remains unresolved"))
        }
    }
}

fn assemble_faces(
    sources: &[Model],
    faces: BTreeMap<Entity, bool>,
    edge_aliases: &EdgeAliases,
    vertex_aliases: &BTreeMap<Entity, Entity>,
) -> Result<Model> {
    let tolerance_mm = faces
        .keys()
        .map(|&(slot, _)| sources[slot].tolerance_mm)
        .fold(f64::INFINITY, f64::min);
    let mut topology = brep_topology::Model {
        vertices: Vec::new(),
        edges: Vec::new(),
        loops: Vec::new(),
        faces: Vec::new(),
        shells: Vec::new(),
        bodies: Vec::new(),
        tolerance_mm,
    };
    let mut vertices = BTreeMap::new();
    let mut edges = BTreeMap::new();
    let mut face_uses = Vec::new();
    for ((slot, id), reversed) in faces {
        let source = &sources[slot];
        let mut face = source.faces[id].clone();
        for wire in std::iter::once(&mut face.outer).chain(&mut face.holes) {
            let mut loop_ = source.loops[*wire].clone();
            for coedge in &mut loop_.coedges {
                let key = (slot, coedge.edge);
                let (key, reversed) = edge_aliases.get(&key).copied().unwrap_or((key, false));
                let domain = source.edges[coedge.edge].curve.domain();
                coedge.reversed ^= reversed;
                coedge.edge = *edges.entry(key).or_insert_with(|| {
                    let mut edge = sources[key.0].edges[key.1].clone();
                    edge.vertices = edge.vertices.map(|v| {
                        let key = vertex_root(vertex_aliases, (key.0, v));
                        *vertices.entry(key).or_insert_with(|| {
                            let index = topology.vertices.len();
                            topology
                                .vertices
                                .push(sources[key.0].vertices[key.1].clone());
                            index
                        })
                    });
                    let index = topology.edges.len();
                    topology.edges.push(edge);
                    index
                });
                let target_domain = sources[key.0].edges[key.1].curve.domain();
                if (reversed || domain != target_domain)
                    && let Some(mut trim) = coedge.trim()
                {
                    for parameter in &mut trim.curve_parameter {
                        *parameter = map_clock(*parameter, domain, target_domain, reversed)
                            .ok_or_else(|| unproven("Rim trim leaves exact parameter precision"))?;
                    }
                    *coedge = coedge.clone().with_trim(trim)?;
                }
            }
            *wire = topology.loops.len();
            topology.loops.push(loop_);
        }
        face_uses.push(crate::FaceUse {
            face: topology.faces.len(),
            reversed,
        });
        topology.faces.push(face);
    }
    let mut incidence = vec![Vec::new(); topology.edges.len()];
    for use_ in &face_uses {
        let face = &topology.faces[use_.face];
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for coedge in &topology.loops[wire].coedges {
                if !topology.edges[coedge.edge].degenerate {
                    incidence[coedge.edge].push((use_.face, coedge.reversed ^ use_.reversed));
                }
            }
        }
    }
    let mut adjacent = vec![Vec::new(); face_uses.len()];
    let mut open = vec![false; face_uses.len()];
    for uses in incidence {
        match uses.as_slice() {
            [] => {}
            [(face, _)] => open[*face] = true,
            [(a, a_rev), (b, b_rev)] if a_rev != b_rev => {
                adjacent[*a].push(*b);
                adjacent[*b].push(*a);
            }
            _ => return Err(unproven("SS patch edge incidence is not oriented manifold")),
        }
    }
    let mut seen = vec![false; face_uses.len()];
    for id in 0..face_uses.len() {
        if seen[id] {
            continue;
        }
        let mut shell = crate::Shell {
            faces: Vec::new(),
            closed: true,
        };
        let mut pending = vec![id];
        while let Some(face) = pending.pop() {
            if std::mem::replace(&mut seen[face], true) {
                continue;
            }
            shell.faces.push(face_uses[face].clone());
            shell.closed &= !open[face];
            pending.extend(&adjacent[face]);
        }
        topology.shells.push(shell);
    }
    let mut model = Model(topology, crate::TopologyIds::default());
    model.rebuild_topology_ids();
    model.validate()?;
    Ok(model)
}

#[cfg(test)]
mod component_tests {
    use super::*;

    #[test]
    fn exact_rim_component_connects_multiple_seed_patches() {
        let source = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let patches: Vec<_> = source.shells[0].faces.iter().map(|use_| {
            assemble_faces(
                std::slice::from_ref(&source),
                BTreeMap::from([((0, use_.face), use_.reversed)]),
                &BTreeMap::new(),
                &BTreeMap::new(),
            ).unwrap()
        }).collect();
        let models: Vec<_> = patches.iter().collect();
        for seed in [0, 1] {
            assert_eq!(
                discover_rim_component(&models, seed, 24).unwrap(),
                (0..6).collect::<Vec<_>>(),
            );
        }
        assert!(discover_rim_component(&models[..5], 0, 24).is_err());
    }
}
