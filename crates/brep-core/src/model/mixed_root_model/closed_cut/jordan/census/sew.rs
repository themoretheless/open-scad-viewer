//! Paired retained patches whose complete cross-source contacts are their
//! literal common root boundary. Outward material orientation remains separate.
use super::*;
mod patch_shells;

type RegionSeed = [[f64; 2]; 2];
type MaterialSeed = (RegionSeed, Option<bool>);

#[derive(Clone, Copy)]
pub struct ClosedSsBooleanLimits {
    pub census: ClosedSsCensusLimits,
    pub material: CutMaterialLimits,
    pub arrangement: [CensusArrangementLimits; 2],
    pub volume: RootBoundaryVolumeLimits,
    pub max_pair_attempts: usize,
    pub max_carrier_cells: usize,
    pub cells_per_support: usize,
}

impl Default for ClosedSsBooleanLimits {
    fn default() -> Self {
        Self {
            census: Default::default(),
            material: Default::default(),
            arrangement: [Default::default(); 2],
            volume: Default::default(),
            max_pair_attempts: 16,
            max_carrier_cells: 4096,
            cells_per_support: 64,
        }
    }
}

enum RegionSearch {
    Provided([Vec<MaterialSeed>; 2]),
    Automatic { cells_per_support: usize },
}

fn explicit_material_seeds(
    components: &[[RegionSeed; 2]],
    excluded: [&[RegionSeed]; 2],
) -> [Vec<MaterialSeed>; 2] {
    std::array::from_fn(|side| {
        components.iter().map(|pair| (pair[side], Some(true)))
            .chain(excluded[side].iter().map(|&seed| (seed, Some(false))))
            .collect()
    })
}

/// Selected and oriented faces on the two immutable arrangements. Face IDs
/// address the corresponding arrangement, including untouched source topology.
pub struct PairedBooleanPatches {
    layouts: [Arc<CensusFaceArrangement>; 2],
    regions: [Vec<QualifiedArrangementMaterialRectangle>; 2],
    uncut: [Vec<brep_topology::FaceUse>; 2],
}

impl PairedBooleanPatches {
    pub fn arrangements(&self) -> [&CensusFaceArrangement; 2] {
        self.layouts.each_ref().map(Arc::as_ref)
    }

    pub fn cut_regions(&self) -> [&[QualifiedArrangementMaterialRectangle]; 2] {
        self.regions.each_ref().map(Vec::as_slice)
    }

    pub fn uncut_faces(&self) -> [&[brep_topology::FaceUse]; 2] {
        self.uncut.each_ref().map(Vec::as_slice)
    }
}

pub struct QualifiedPairedRootBoundary {
    census: Arc<QualifiedInteriorSsCensus>,
    signatures: [Vec<bool>; 2],
    limits: [CensusArrangementLimits; 2],
    model: MixedBrep,
    genus: usize,
}
impl QualifiedPairedRootBoundary {
    pub fn model(&self) -> &MixedBrep {
        &self.model
    }
    pub fn genus(&self) -> usize {
        self.genus
    }
    pub fn boundary_embedded(&self) -> bool {
        true
    }
    pub fn material_admitted(&self) -> bool {
        false
    }
    pub fn signatures(&self) -> [&[bool]; 2] {
        [&self.signatures[0], &self.signatures[1]]
    }
    pub fn reverify(&self) -> Result<()> {
        self.census.reverify()?;
        let fresh = self.census.sew_paired_regions(
            self.signatures(),
            self.model.topology.tolerance_mm,
            self.limits,
        )?;
        if fresh.genus != self.genus {
            return Err(unproved("Paired root boundary topology changed"));
        }
        Ok(())
    }
}
#[derive(Default)]
struct RootSewIndex {
    vertices_by_curve: std::collections::HashMap<*const ss::OriginalSsRootCurve, Vec<usize>>,
    edges_by_vertices: std::collections::HashMap<[usize; 2], Vec<usize>>,
}

impl RootSewIndex {
    fn insert_edge(&mut self, topology: &mut MixedTopology, edge: &ss::OriginalSsRootEdge) -> Result<usize> {
        let vertices = [edge.start(), edge.end()].map(|port| {
            let point = port.point();
            // Literal curve identity is required by same_selector. The map
            // narrows candidates while exact clock equality still decides.
            let candidates = self.vertices_by_curve.entry(Arc::as_ptr(point.curve())).or_default();
            if let Some(id) = candidates.iter().copied().find(|&id| {
                matches!(&topology.vertices[id].point, MixedVertex::Root(p) if p.same_selector(point))
            }) {
                return id;
            }
            let id = topology.vertices.len();
            topology.vertices.push(brep_topology::Vertex { point: MixedVertex::Root(point.clone()) });
            candidates.push(id);
            id
        });
        let candidates = self.edges_by_vertices.entry(vertices).or_default();
        for &id in candidates.iter() {
            if let MixedEdge::Root(old) = &topology.edges[id].curve {
                if old.curve().same_definition(edge.curve())? {
                    return Ok(id);
                }
            }
        }
        let id = topology.edges.len();
        topology.edges.push(brep_topology::Edge {
            degenerate: false,
            vertices,
            curve: MixedEdge::Root(edge.clone()),
        });
        candidates.push(id);
        Ok(id)
    }
}

/// Classify every untouched finite face, excluding cells outside its trim.
fn classify_uncut_faces(
    solids: [&crate::closed_solid::QualifiedClosedSolid; 2],
    is_cut: impl Fn(usize) -> bool,
    tolerance_uv: f64,
    operation: crate::closed_solid::BooleanMaterialOp,
    role: crate::closed_solid::BooleanSourceSide,
    directions: &[[f64; 3]],
    material: CutMaterialLimits,
    max_carrier_cells: &mut usize,
) -> Result<Vec<brep_topology::FaceUse>> {
    use crate::closed_solid::{BooleanFaceDisposition, qualify_surface_material_box};

    let [source, opposing] = solids;
    let mut uncut = Vec::new();
    for (id, face) in source.model().faces.iter().enumerate() {
        if is_cut(id) {
            continue;
        }
        let domain = crate::face_domain::FaceDomain::new(
            source.model(),
            id,
            tolerance_uv,
        )?;
        let s = &face.surface;
        let cover = finite_face_cover(source.model(), id)?;
        let mut pending = vec![cover];
        let mut disposition = None;
        while let Some(mut cell) = pending.pop() {
            *max_carrier_cells = max_carrier_cells.checked_sub(1)
                .ok_or_else(|| unproved("Boolean carrier cover exhausted"))?;
            if domain.classify(cell, material.boolean.domain_cells)?.location == Location::Outside {
                continue;
            }
            let cell_disposition = qualify_surface_material_box(
                opposing, s, cell, directions, material.boolean.material,
            ).ok().map(|proof| operation.face_disposition(role, proof.inside()));
            if let Some(current) = cell_disposition {
                if disposition.is_some_and(|prior| prior != current) {
                    return Err(unproved("Uncut Boolean face has no proved uniform disposition"));
                }
                disposition = Some(current);
                continue;
            }
            let (axis, mid, _) = cell.iter().zip(cover).enumerate()
                .filter_map(|(axis, (&[lo, hi], [start, end]))| {
                    let mid = lo * 0.5 + hi * 0.5;
                    (lo < mid && mid < hi).then_some((axis, mid, (hi - lo) / (end - start)))
                })
                .max_by(|a, b| a.2.total_cmp(&b.2))
                .ok_or_else(|| unproved("Boolean carrier material remains unresolved"))?;
            let mut right = cell;
            cell[axis][1] = mid;
            right[axis][0] = mid;
            pending.extend([right, cell]);
        }
        match disposition {
            Some(BooleanFaceDisposition::Discard) => (),
            Some(BooleanFaceDisposition::Keep | BooleanFaceDisposition::Reverse) => {
                let mut uses = source.model().shells.iter()
                    .flat_map(|shell| &shell.faces).filter(|use_| use_.face == id);
                let use_ = uses.next().filter(|_| uses.next().is_none())
                    .ok_or_else(|| unproved("Uncut Boolean face needs one source material use"))?;
                uncut.push(brep_topology::FaceUse {
                    face: id,
                    reversed: use_.reversed ^ (disposition == Some(BooleanFaceDisposition::Reverse)),
                });
            }
            None => return Err(unproved("Uncut Boolean face has no covered material domain")),
        }
    }
    Ok(uncut)
}

fn boolean_containment_limits(
    tolerance_uv: f64,
    volume: RootBoundaryVolumeLimits,
) -> RootMaterialBoxLimits {
    RootMaterialBoxLimits {
        point: RootSolidPointLimits {
            tolerance_uv,
            arc_subdivisions: volume.arc_subdivisions,
            band_queries: volume.band_queries,
            winding_queries: volume.winding_queries,
            winding: volume.winding,
            geometry: volume.geometry,
            ..Default::default()
        },
        ..Default::default()
    }
}

impl QualifiedInteriorSsCensus {
    /// Search face pairs and publish only an independently admitted Boolean.
    /// A successful pair must account for all other faces on both operands.
    pub fn assemble_closed_ss_boolean_between(
        solids: [&crate::closed_solid::QualifiedClosedSolid; 2],
        operation: crate::closed_solid::BooleanMaterialOp,
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        limits: ClosedSsBooleanLimits,
    ) -> Result<(Arc<Self>, MixedBrep, [f64; 2])> {
        let (mut censuses, body, volume) = Self::search_closed_ss_boolean(
            solids, operation, directions, tolerance_mm, limits, false,
        )?;
        let census = censuses.pop().ok_or_else(|| unproved("Missing closed SS Boolean census"))?;
        Ok((census, body, volume))
    }

    /// Discover original-face SS cuts and try a joint material assembly when
    /// no individual pair accounts for the full Boolean.
    pub fn assemble_closed_ss_network_boolean_between(
        solids: [&crate::closed_solid::QualifiedClosedSolid; 2],
        operation: crate::closed_solid::BooleanMaterialOp,
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        limits: ClosedSsBooleanLimits,
    ) -> Result<(Vec<Arc<Self>>, MixedBrep, [f64; 2])> {
        Self::search_closed_ss_boolean(solids, operation, directions, tolerance_mm, limits, true)
    }

    fn search_closed_ss_boolean(
        solids: [&crate::closed_solid::QualifiedClosedSolid; 2],
        operation: crate::closed_solid::BooleanMaterialOp,
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        limits: ClosedSsBooleanLimits,
        allow_network: bool,
    ) -> Result<(Vec<Arc<Self>>, MixedBrep, [f64; 2])> {
        let mut first_failure = None;
        let mut network_failure = None;
        let mut censuses = Vec::new();
        for [a, b] in overlapping_face_pairs(solids, limits.max_pair_attempts)? {
            let attempt = match Self::discover_closed_branches(
                [(solids[0], a), (solids[1], b)], limits.census,
            ) {
                Ok(census) => {
                    let census = Arc::new(census);
                    match census.assemble_automatic_paired_boolean(
                        operation, directions, tolerance_mm, limits.material,
                        limits.arrangement, limits.volume, limits.max_carrier_cells,
                        limits.cells_per_support,
                    ) {
                        Ok((body, volume)) => return Ok((vec![census], body, volume)),
                        Err(error) => {
                            if allow_network {
                                censuses.push(census);
                                if censuses.len() > 1 {
                                    match Self::assemble_multi_face_boolean(
                                        solids, &censuses, operation, directions, tolerance_mm, limits,
                                    ) {
                                        Ok((body, volume)) => return Ok((censuses, body, volume)),
                                        Err(error) => network_failure = Some(error),
                                    }
                                }
                            }
                            error
                        }
                    }
                }
                Err(error) => error,
            };
            first_failure.get_or_insert_with(|| {
                unproved(&format!("Closed SS Boolean face pair [{a}, {b}]: {}", attempt.message))
            });
        }
        Err(network_failure.or(first_failure)
            .unwrap_or_else(|| unproved("No overlapping face pair qualified for closed SS Boolean")))
    }

    /// Assemble the boundary of a Boolean from a network of certified closed
    /// cuts. Unmatched cut-face pairs must be separated.
    pub fn sew_multi_face_boolean(
        solids: [&crate::closed_solid::QualifiedClosedSolid; 2],
        censuses: &[Arc<Self>],
        operation: crate::closed_solid::BooleanMaterialOp,
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        limits: ClosedSsBooleanLimits,
    ) -> Result<MixedBrep> {
        if operation == crate::closed_solid::BooleanMaterialOp::Xor {
            return Err(unproved("Use material body assembly for multi-face XOR"));
        }
        let [(a, first), (b, second)] =
            Self::select_multi_face_boolean(solids, censuses, operation, directions, limits)?;
        patch_shells::sew_selected_shells(
            [(a.topology(), first), (b.topology(), second)], tolerance_mm,
        )
    }

    fn select_multi_face_boolean(
        solids: [&crate::closed_solid::QualifiedClosedSolid; 2],
        censuses: &[Arc<Self>],
        operation: crate::closed_solid::BooleanMaterialOp,
        directions: &[[f64; 3]],
        limits: ClosedSsBooleanLimits,
    ) -> Result<[(MixedBrep, Vec<brep_topology::FaceUse>); 2]> {
        use crate::closed_solid::BooleanSourceSide;
        if censuses.is_empty() || !(1..=16384).contains(&limits.cells_per_support)
            || !(1..=16384).contains(&limits.max_carrier_cells)
        {
            return Err(unproved("Multi-face Boolean requires cuts and bounded material search"));
        }
        let mut groups = [std::collections::BTreeMap::new(), std::collections::BTreeMap::new()];
        let mut matched = std::collections::BTreeSet::new();
        for census in censuses {
            let parents = [census.parent(0)?, census.parent(1)?];
            let pair = parents.map(|cut| cut.face);
            if !matched.insert(pair) {
                return Err(unproved("Duplicate complete census for one face pair"));
            }
            for (side, cut) in parents.into_iter().enumerate() {
                if !std::ptr::eq(cut.source.model(), solids[side].model()) {
                    return Err(unproved("Multi-face Boolean cut operands do not match"));
                }
                groups[side].entry(cut.face).or_insert_with(Vec::new)
                    .extend(census.loops.iter().map(|component| (&component.cuts[side], side)));
            }
        }
        // Loops from different pairs on the same face cannot meet: their
        // opposing loops lie strictly inside distinct embedded operand faces.
        let faces = groups.each_ref().map(|group| group.keys().copied().collect::<Vec<_>>());
        let tolerance_uv = censuses.iter().flat_map(|census| &census.loops[0].cuts)
            .map(|cut| cut.limits.tolerance_uv).fold(f64::INFINITY, f64::min);
        let bounds = std::array::from_fn::<Vec<_>, 2, _>(|side| {
            faces[side].iter().map(|&face| {
                finite_face_world_bounds(solids[side].model(), face)
            }).collect()
        });
        let mut budget = limits.max_carrier_cells;
        let mut domain_budget = limits.material.boolean.domain_cells;
        for (i, first) in bounds[0].iter().enumerate() {
            for (j, second) in bounds[1].iter().enumerate() {
                let indices = [faces[0][i], faces[1][j]];
                if matched.contains(&indices) || bounds_separated(first, second) {
                    continue;
                }
                let models = solids.map(|solid| solid.model());
                let [a, b] = std::array::from_fn::<_, 2, _>(|side| {
                    &models[side].faces[indices[side]].surface
                });
                use nurbs_core::surface_contact_search as contact;
                if budget == 0 {
                    return Err(unproved("Cut-face separation work exhausted"));
                }
                let report = if [a, b].iter().any(|s| s.periodic_u || s.periodic_v) {
                    let covers = [
                        finite_face_cover(models[0], indices[0])?,
                        finite_face_cover(models[1], indices[1])?,
                    ];
                    contact::search_chart_rectangles(a, b, covers, budget)?
                } else {
                    if domain_budget == 0 {
                        return Err(unproved("Cut-face trim separation work exhausted"));
                    }
                    let first = crate::face_domain::FaceDomain::new(
                        models[0], indices[0], tolerance_uv,
                    )?;
                    let second = crate::face_domain::FaceDomain::new(
                        models[1], indices[1], tolerance_uv,
                    )?;
                    contact::search_trimmed_with_control_hulls(
                        a, b, [&first.region, &second.region], budget, domain_budget,
                    )?
                };
                if !report.absence_proven {
                    return Err(unproved(&format!(
                        "Unmatched cut-face pair {indices:?} has no complete separation proof"
                    )));
                }
                budget -= report.cells;
                domain_budget -= report.domain_cells;
            }
        }
        let [(a, a_layouts), (b, b_layouts)] = [
            Self::build_network_face_layout(solids[0], &groups[0], limits.arrangement[0])?,
            Self::build_network_face_layout(solids[1], &groups[1], limits.arrangement[1])?,
        ];
        let mut selected = [Vec::new(), Vec::new()];
        for (side, (role, layouts)) in [
            (BooleanSourceSide::First, a_layouts),
            (BooleanSourceSide::Second, b_layouts),
        ].into_iter().enumerate() {
            selected[side] = classify_uncut_faces(
                [solids[side], solids[1 - side]], |face| groups[side].contains_key(&face),
                tolerance_uv, operation, role, directions, limits.material, &mut budget,
            )?;
            for ((&face, cuts), (parents, faces)) in groups[side].iter().zip(layouts) {
                selected[side].extend(arrangement::discover_network_face_material(
                    crate::closed_solid::BooleanFace { solids, operation, side: role, face },
                    side, cuts, &parents, &faces, directions, limits.material, limits.cells_per_support,
                )?);
            }
        }
        let [first, second] = selected;
        Ok([(a, first), (b, second)])
    }

    /// Assemble and validate material bodies from a certified multi-face cut network.
    pub fn assemble_multi_face_boolean(
        solids: [&crate::closed_solid::QualifiedClosedSolid; 2],
        censuses: &[Arc<Self>],
        operation: crate::closed_solid::BooleanMaterialOp,
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        limits: ClosedSsBooleanLimits,
    ) -> Result<(MixedBrep, [f64; 2])> {
        let tolerance_uv = censuses.iter().flat_map(|census| &census.loops[0].cuts)
            .map(|cut| cut.limits.tolerance_uv)
            .fold(f64::INFINITY, f64::min);
        let containment = boolean_containment_limits(tolerance_uv, limits.volume);
        if operation != crate::closed_solid::BooleanMaterialOp::Xor {
            let boundary = Self::sew_multi_face_boolean(
                solids, censuses, operation, directions, tolerance_mm, limits,
            )?;
            return patch_shells::admit_disjoint_boolean_shells(
                boundary, directions, containment, limits.volume,
            );
        }

        let layouts = Self::select_multi_face_boolean(solids, censuses, operation, directions, limits)?;
        let mut halves = std::array::from_fn::<_, 2, _>(|_| [Vec::new(), Vec::new()]);
        for (side, (layout, selected)) in layouts.iter().enumerate() {
            let mut normals = vec![None; layout.topology().faces.len()];
            for use_ in layout.topology().shells.iter().flat_map(|shell| &shell.faces) {
                normals[use_.face] = Some(use_.reversed);
            }
            for face in selected {
                let original = normals[face.face]
                    .ok_or_else(|| unproved("XOR face has no original material orientation"))?;
                halves[side ^ usize::from(face.reversed != original)][side].push(face.clone());
            }
        }
        let [a, b] = layouts.each_ref().map(|(layout, _)| layout.topology());
        let volume = RootBoundaryVolumeLimits {
            absolute_error: limits.volume.absolute_error / 2.1,
            ..limits.volume
        };
        let admit = |[first, second]: [Vec<brep_topology::FaceUse>; 2]| {
            let boundary = patch_shells::sew_selected_shells(
                [(a, first), (b, second)], tolerance_mm,
            )?;
            patch_shells::admit_disjoint_boolean_shells(boundary, directions, containment, volume)
        };
        let [first, second] = halves;
        patch_shells::combine_boolean_differences(admit(first)?, admit(second)?, limits.volume)
    }

    /// Build a closed two-patch intersection candidate from certified
    /// material seeds. The seeds select regions on these two supports.
    /// Additional operand faces require a separate completeness proof.
    pub fn sew_intersection_candidate(
        self: &Arc<Self>,
        seeds: [[[f64; 2]; 2]; 2],
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        material: CutMaterialLimits,
        arrangement: [CensusArrangementLimits; 2],
        volume: RootBoundaryVolumeLimits,
    ) -> Result<QualifiedPairedRootSolid> {
        self.sew_boolean_candidate(
            crate::closed_solid::BooleanMaterialOp::Intersection,
            seeds, directions, tolerance_mm, material, arrangement, volume,
        )
    }

    /// Certify a two-face intersection with a single interior SS contour
    /// and disk-like cut faces. Each exterior seed selects the other region
    /// of the cut face. All uncut carriers must be outside opposing material.
    pub fn sew_complete_single_loop_intersection(
        self: &Arc<Self>,
        seeds: [[[f64; 2]; 2]; 2],
        exterior_seeds: [[[f64; 2]; 2]; 2],
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        material: CutMaterialLimits,
        arrangement: [CensusArrangementLimits; 2],
        volume: RootBoundaryVolumeLimits,
        max_carrier_cells: usize,
    ) -> Result<QualifiedPairedRootSolid> {
        if self.loops.len() != 1 {
            return Err(unproved("Single-loop intersection requires one contour"));
        }
        self.sew_complete_paired_intersection(
            seeds,
            [&exterior_seeds[..1], &exterior_seeds[1..]],
            directions,
            tolerance_mm,
            material,
            arrangement,
            volume,
            max_carrier_cells,
        )
    }

    /// Certify a two-face intersection when all other arrangement regions
    /// and uncut faces are proved outside opposing material.
    pub fn sew_complete_paired_intersection(
        self: &Arc<Self>,
        seeds: [[[f64; 2]; 2]; 2],
        excluded_seeds: [&[[[f64; 2]; 2]]; 2],
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        material: CutMaterialLimits,
        arrangement: [CensusArrangementLimits; 2],
        volume: RootBoundaryVolumeLimits,
        max_carrier_cells: usize,
    ) -> Result<QualifiedPairedRootSolid> {
        self.sew_complete_paired_boolean(
            crate::closed_solid::BooleanMaterialOp::Intersection,
            seeds, excluded_seeds, directions, tolerance_mm, material, arrangement, volume,
            max_carrier_cells,
        )
    }

    /// Certify a Boolean whose entire boundary consists of one retained
    /// region per support. Prove that the Boolean discards every other face.
    pub fn sew_complete_paired_boolean(
        self: &Arc<Self>,
        operation: crate::closed_solid::BooleanMaterialOp,
        seeds: [[[f64; 2]; 2]; 2],
        excluded_seeds: [&[[[f64; 2]; 2]]; 2],
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        material: CutMaterialLimits,
        arrangement: [CensusArrangementLimits; 2],
        volume: RootBoundaryVolumeLimits,
        max_carrier_cells: usize,
    ) -> Result<QualifiedPairedRootSolid> {
        let mut parts = self.sew_complete_paired_boolean_components(
            operation, &[seeds], excluded_seeds, directions, tolerance_mm,
            material, arrangement, volume, max_carrier_cells,
        )?;
        Arc::try_unwrap(parts.remove(0)).map_err(|_| unproved("Paired Boolean component has unexpected shared ownership"))
    }

    /// Certify a complete selection across the two cut supports. Each seed
    /// pair specifies a shell. Check the resulting shells for disjointness
    /// before returning them as independent material components.
    pub fn sew_complete_paired_boolean_components(
        self: &Arc<Self>,
        operation: crate::closed_solid::BooleanMaterialOp,
        components: &[[[[f64; 2]; 2]; 2]],
        excluded_seeds: [&[[[f64; 2]; 2]]; 2],
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        material: CutMaterialLimits,
        arrangement: [CensusArrangementLimits; 2],
        volume: RootBoundaryVolumeLimits,
        max_carrier_cells: usize,
    ) -> Result<Vec<Arc<QualifiedPairedRootSolid>>> {
        let seeds = explicit_material_seeds(components, excluded_seeds);
        let (solids, _) = self.qualify_paired_boolean_components(
            operation, RegionSearch::Provided(seeds), directions, tolerance_mm,
            material, arrangement, volume, max_carrier_cells, false,
        )?;
        Ok(solids)
    }

    /// Assemble the certified disjoint components into one model and bound
    /// the total volume to the requested accuracy.
    pub fn assemble_complete_paired_boolean(
        self: &Arc<Self>,
        operation: crate::closed_solid::BooleanMaterialOp,
        components: &[[[[f64; 2]; 2]; 2]],
        excluded_seeds: [&[[[f64; 2]; 2]]; 2],
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        material: CutMaterialLimits,
        arrangement: [CensusArrangementLimits; 2],
        volume: RootBoundaryVolumeLimits,
        max_carrier_cells: usize,
    ) -> Result<(MixedBrep, [f64; 2])> {
        self.assemble_paired_boolean_with_search(
            operation, RegionSearch::Provided(explicit_material_seeds(components, excluded_seeds)),
            directions, tolerance_mm, material, arrangement, volume, max_carrier_cells,
        )
    }

    /// Classify one UV rectangle per arrangement region on each support.
    /// The material proofs select and orient the retained regions.
    pub fn assemble_paired_boolean_from_region_seeds(
        self: &Arc<Self>,
        operation: crate::closed_solid::BooleanMaterialOp,
        region_seeds: [&[RegionSeed]; 2],
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        material: CutMaterialLimits,
        arrangement: [CensusArrangementLimits; 2],
        volume: RootBoundaryVolumeLimits,
        max_carrier_cells: usize,
    ) -> Result<(MixedBrep, [f64; 2])> {
        self.assemble_paired_boolean_with_search(
            operation, RegionSearch::Provided(region_seeds.map(|side| {
                side.iter().map(|&seed| (seed, None)).collect()
            })),
            directions, tolerance_mm, material, arrangement, volume, max_carrier_cells,
        )
    }

    /// Discover and prove material witnesses for every region on both supports.
    /// Each support has an independent search budget. Sew the oriented patches
    /// into material bodies, including whole retained source faces and cavities.
    pub fn assemble_automatic_paired_boolean(
        self: &Arc<Self>,
        operation: crate::closed_solid::BooleanMaterialOp,
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        material: CutMaterialLimits,
        arrangement: [CensusArrangementLimits; 2],
        volume: RootBoundaryVolumeLimits,
        max_carrier_cells: usize,
        cells_per_support: usize,
    ) -> Result<(MixedBrep, [f64; 2])> {
        let tolerance_uv = self.parent(0)?.limits.tolerance_uv
            .min(self.parent(1)?.limits.tolerance_uv);
        let containment = boolean_containment_limits(tolerance_uv, volume);
        let patches = self.select_automatic_paired_boolean_patches(
            operation, directions, material, arrangement, max_carrier_cells, cells_per_support,
        )?;
        if operation == crate::closed_solid::BooleanMaterialOp::Xor {
            return patches.assemble_symmetric_difference(tolerance_mm, directions, containment, volume);
        }
        patches.assemble_bodies(tolerance_mm, directions, containment, volume)
    }

    /// Sew the union boundary and bound the volume of its material using
    /// inclusion-exclusion on the admitted operands and their admitted intersection.
    /// The returned shells retain their material-side orientation.
    pub fn assemble_paired_union(
        self: &Arc<Self>,
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        material: CutMaterialLimits,
        arrangement: [CensusArrangementLimits; 2],
        volume: RootBoundaryVolumeLimits,
        mut operand_volume: crate::closed_solid::VolumeLimits,
        max_carrier_cells: usize,
        cells_per_support: usize,
    ) -> Result<(MixedBrep, [f64; 2])> {
        use crate::closed_solid::BooleanMaterialOp;
        use nurbs_core::interval_eval::Interval;

        let allowance = volume.absolute_error / 4.;
        if !allowance.is_finite() || allowance <= 0. {
            return Err(unproved("Union volume needs positive finite accuracy"));
        }
        operand_volume.absolute_error = allowance;
        let mut sum = Interval::point(0.);
        for side in 0..2 {
            let [lo, hi] = self.parent(side)?.source.volume_bounds(operand_volume)?
                .positive_volume
                .ok_or_else(|| unproved("Union operand volume is unproved"))?;
            sum = sum.add(Interval::new(lo, hi)?)?;
        }
        let (_, [lo, hi]) = self.assemble_automatic_paired_boolean(
            BooleanMaterialOp::Intersection, directions, tolerance_mm, material, arrangement,
            RootBoundaryVolumeLimits { absolute_error: allowance, ..volume },
            max_carrier_cells, cells_per_support,
        )?;
        let result = sum.sub(Interval::new(lo, hi)?)?;
        if result.lo <= 0.
            || Interval::point(result.hi).sub(Interval::point(result.lo))?.hi > volume.absolute_error
        {
            return Err(unproved("Union material volume does not meet accuracy"));
        }
        let boundary = self.select_automatic_paired_boolean_patches(
            BooleanMaterialOp::Union, directions, material, arrangement,
            max_carrier_cells, cells_per_support,
        )?.sew_disjoint_shells(tolerance_mm)?;
        Ok((boundary, [result.lo, result.hi]))
    }

    fn assemble_paired_boolean_with_search(
        self: &Arc<Self>,
        operation: crate::closed_solid::BooleanMaterialOp,
        search: RegionSearch,
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        material: CutMaterialLimits,
        arrangement: [CensusArrangementLimits; 2],
        volume: RootBoundaryVolumeLimits,
        max_carrier_cells: usize,
    ) -> Result<(MixedBrep, [f64; 2])> {
        let (solids, relations) = self.qualify_paired_boolean_components(
            operation, search, directions, tolerance_mm,
            material, arrangement, volume, max_carrier_cells, true,
        )?;
        if let [solid] = solids.as_slice() {
            return Ok((solid.model().clone(), solid.volume()));
        }
        let forest = QualifiedRootShellForest::qualify(solids, relations, volume.absolute_error)?;
        Ok((forest.model().clone(), forest.volume()))
    }

    /// Select all cut and uncut material patches for a certified paired
    /// contour arrangement. This exposes the boundary inputs for sewing.
    pub fn select_automatic_paired_boolean_patches(
        self: &Arc<Self>,
        operation: crate::closed_solid::BooleanMaterialOp,
        directions: &[[f64; 3]],
        material: CutMaterialLimits,
        arrangement: [CensusArrangementLimits; 2],
        max_carrier_cells: usize,
        cells_per_support: usize,
    ) -> Result<PairedBooleanPatches> {
        self.select_paired_boolean_patches(
            operation, RegionSearch::Automatic { cells_per_support }, directions,
            material, arrangement, max_carrier_cells,
        )
    }

    fn select_paired_boolean_patches(
        self: &Arc<Self>,
        operation: crate::closed_solid::BooleanMaterialOp,
        search: RegionSearch,
        directions: &[[f64; 3]],
        material: CutMaterialLimits,
        arrangement: [CensusArrangementLimits; 2],
        mut max_carrier_cells: usize,
    ) -> Result<PairedBooleanPatches> {
        if let RegionSearch::Automatic { cells_per_support } = &search {
            if !(1..=16384).contains(cells_per_support) {
                return Err(unproved("Boolean requires a bounded region witness search"));
            }
        }
        use crate::closed_solid::BooleanSourceSide;
        if !(1..=16384).contains(&max_carrier_cells) {
            return Err(unproved("Boolean requires a bounded carrier cover"));
        }
        let parents = [self.parent(0)?, self.parent(1)?];
        let layouts = [
            Arc::new(self.build_face_arrangement(0, arrangement[0])?),
            Arc::new(self.build_face_arrangement(1, arrangement[1])?),
        ];
        let mut selections = [Vec::new(), Vec::new()];
        let mut uncut = [Vec::new(), Vec::new()];
        for (side, role) in [BooleanSourceSide::First, BooleanSourceSide::Second].into_iter().enumerate() {
            let cut = parents[side];
            let layout = &layouts[side];
            selections[side] = match &search {
                RegionSearch::Automatic { cells_per_support } => layout
                    .discover_boolean_regions(operation, directions, material, *cells_per_support)?,
                RegionSearch::Provided(seeds) => {
                    let mut retained = Vec::new();
                    if seeds[side].len() != layout.region_faces().len() {
                        return Err(unproved("Boolean requires a seed for every arrangement region"));
                    }
                    let mut classified = std::collections::BTreeSet::new();
                    for &(seed, expected_retention) in &seeds[side] {
                        let region = layout.qualify_boolean_rectangle(operation, seed, directions, material)?;
                        let retain = region.material().boolean_material().retain();
                        if !classified.insert(region.face()) {
                            return Err(unproved("Boolean seeds must select distinct arrangement regions"));
                        }
                        if expected_retention.is_some_and(|expected| expected != retain) {
                            return Err(unproved("Boolean seed disagrees with proved material disposition"));
                        }
                        if retain {
                            retained.push(region);
                        }
                    }
                    retained
                }
            };
            uncut[side] = classify_uncut_faces(
                [&cut.source, &parents[1 - side].source],
                |face| face == cut.face,
                cut.limits.tolerance_uv,
                operation,
                role,
                directions,
                material,
                &mut max_carrier_cells,
            )?;
        }
        Ok(PairedBooleanPatches { layouts, regions: selections, uncut })
    }

    fn qualify_paired_boolean_components(
        self: &Arc<Self>,
        operation: crate::closed_solid::BooleanMaterialOp,
        search: RegionSearch,
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        material: CutMaterialLimits,
        arrangement: [CensusArrangementLimits; 2],
        volume: RootBoundaryVolumeLimits,
        max_carrier_cells: usize,
        bound_total_volume: bool,
    ) -> Result<(Vec<Arc<QualifiedPairedRootSolid>>, Vec<Arc<QualifiedRootRelation>>)> {
        let PairedBooleanPatches { layouts, regions: selections, uncut } =
            self.select_paired_boolean_patches(
                operation, search, directions, material, arrangement, max_carrier_cells,
            )?;
        if uncut.iter().any(|side| !side.is_empty()) {
            return Err(unproved("Paired root sewing needs mixed original and root rim stitching"));
        }
        let count = selections[0].len();
        if !(1..=64).contains(&count) || selections[1].len() != count {
            return Err(unproved("Boolean requires matching nonempty bounded retained region sets"));
        }
        let mut component_volume = volume;
        if bound_total_volume && count > 1 {
            // Reserve room for outward rounding when the shell intervals are summed.
            component_volume.absolute_error /= 2. * count as f64;
            if component_volume.absolute_error <= 0. {
                return Err(unproved("Boolean volume tolerance is too small to divide among components"));
            }
        }
        let mut solids = Vec::with_capacity(count);
        let mut relations = Vec::new();
        let contours = |layout: &CensusFaceArrangement, face| -> Result<Vec<usize>> {
            let region = layout.region_faces().iter().position(|&id| id == face)
                .ok_or_else(|| unproved("Missing selected Boolean region"))?;
            let outer = region.checked_sub(1);
            Ok(layout.parents().iter().enumerate().filter_map(|(index, &parent)| {
                (Some(index) == outer || parent == outer).then_some(index)
            }).collect())
        };
        let mut second_by_contour = std::collections::BTreeMap::new();
        for second in &selections[1] {
            let key = contours(&layouts[1], second.face())?;
            if second_by_contour.insert(key, second).is_some() {
                return Err(unproved("Ambiguous second-support Boolean contour pairing"));
            }
        }
        for first in &selections[0] {
            let key = contours(&layouts[0], first.face())?;
            let second = second_by_contour.remove(&key)
                .ok_or_else(|| unproved("Boolean region has no counterpart with the same SS contours"))?;
            let selected = [first.material(), second.material()];
            let signatures = selected.map(|selection| selection.region().signature().unwrap());
            let boundary = self.sew_paired_arrangements(
                signatures, tolerance_mm, arrangement, [&layouts[0], &layouts[1]],
            )?;
            let solid = Arc::new(admit_boolean_boundary(boundary, selected, component_volume)?);
            for previous in &solids {
                let relation = solid.qualify_relation(
                    Arc::clone(previous), directions, Default::default(),
                )?;
                if relation.kind() != RootRelationKind::Disjoint {
                    return Err(unproved("Paired Boolean components are not disjoint"));
                }
                relations.push(Arc::new(relation));
            }
            solids.push(solid);
        }
        Ok((solids, relations))
    }

    /// Sew exactly two selected regions, checking their Boolean orientation
    /// against the globally outward orientation of the resulting shell.
    pub fn sew_boolean_candidate(
        self: &Arc<Self>,
        operation: crate::closed_solid::BooleanMaterialOp,
        seeds: [[[f64; 2]; 2]; 2],
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        material: CutMaterialLimits,
        arrangement: [CensusArrangementLimits; 2],
        volume: RootBoundaryVolumeLimits,
    ) -> Result<QualifiedPairedRootSolid> {
        use crate::closed_solid::BooleanSourceSide;
        let [first, second] = [BooleanSourceSide::First, BooleanSourceSide::Second];
        let selections = [
            self.qualify_boolean_rectangle(operation, first, seeds[0], directions, material)?,
            self.qualify_boolean_rectangle(operation, second, seeds[1], directions, material)?,
        ];
        if selections.iter().any(|s| !s.boolean_material().retain()) {
            return Err(unproved("Boolean candidate seed selects a discarded face"));
        }
        let signatures = selections.each_ref().map(|s| s.region().signature().unwrap());
        let boundary = self.sew_paired_regions(signatures, tolerance_mm, arrangement)?;
        admit_boolean_boundary(boundary, selections.each_ref(), volume)
    }

    /// The sealed census owns ALL contacts of the two original supports. A
    /// retained arrangement region has no root component in its interior; its
    /// only contacts are its authored boundary loops. Matching those loops by
    /// exact graph/selectors sews two injective patches into an embedded closed
    /// orientable boundary. Rational rims/holes need separate stitching proof.
    pub fn sew_paired_regions(
        self: &Arc<Self>,
        signatures: [&[bool]; 2],
        tolerance_mm: f64,
        limits: [CensusArrangementLimits; 2],
    ) -> Result<QualifiedPairedRootBoundary> {
        let arrangements = [
            self.build_face_arrangement(0, limits[0])?,
            self.build_face_arrangement(1, limits[1])?,
        ];
        self.sew_paired_arrangements(signatures, tolerance_mm, limits, arrangements.each_ref())
    }

    fn sew_paired_arrangements(
        self: &Arc<Self>,
        signatures: [&[bool]; 2],
        tolerance_mm: f64,
        limits: [CensusArrangementLimits; 2],
        arrangements: [&CensusFaceArrangement; 2],
    ) -> Result<QualifiedPairedRootBoundary> {
        if !tolerance_mm.is_finite() || tolerance_mm <= 0. {
            return Err(unproved("Paired boundary needs positive finite tolerance"));
        }
        let mut t = MixedTopology {
            vertices: Vec::new(),
            edges: Vec::new(),
            loops: Vec::new(),
            faces: Vec::new(),
            shells: Vec::new(),
            bodies: Vec::new(),
            tolerance_mm,
        };
        let mut root_index = RootSewIndex::default();
        let mut incidence: Vec<Vec<(usize, bool)>> = Vec::new();
        let mut hole_counts = [0; 2];
        for support in 0..2 {
            let source = arrangements[support].model().topology();
            let face = arrangements[support].face_for_signature(signatures[support])?;
            let face = &source.faces[face];
            hole_counts[support] = face.holes.len();
            let mut loops = Vec::new();
            for l in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                let mut coedges = Vec::new();
                for c in &source.loops[l].coedges {
                    let (MixedEdge::Root(edge), MixedPcurve::Root(pcurve)) =
                        (&source.edges[c.edge].curve, &c.pcurve)
                    else {
                        return Err(unproved(
                            "Paired boundary needs explicit rational/root rim stitching",
                        ));
                    };
                    if c.trim().is_some() || pcurve.support != support {
                        return Err(unproved("Foreign paired root projection"));
                    }
                    let id = root_index.insert_edge(&mut t, edge)?;
                    while incidence.len() <= id {
                        incidence.push(Vec::new());
                    }
                    incidence[id].push((support, c.reversed));
                    coedges.push(brep_topology::Coedge::new(
                        id,
                        c.reversed,
                        MixedPcurve::Root(crate::root_model::RootPcurve {
                            world_edge: id,
                            support,
                            reversed: c.reversed,
                        }),
                    ));
                }
                let id = t.loops.len();
                t.loops.push(brep_topology::Loop { coedges });
                loops.push(id);
            }
            t.faces.push(brep_topology::Face {
                surface: face.surface.clone(),
                outer: loops[0],
                holes: loops[1..].to_vec(),
            });
        }
        if hole_counts[0] != hole_counts[1] {
            return Err(unproved(
                "Paired patches have different boundary component counts",
            ));
        }
        let mut reverse = None;
        for uses in &incidence {
            if uses.len() != 2 || uses[0].0 != 0 || uses[1].0 != 1 {
                return Err(unproved(
                    "Paired regions have unmatched or repeated original world edges",
                ));
            }
            let needed = uses[0].1 == uses[1].1;
            if reverse.is_some_and(|r| r != needed) {
                return Err(unproved(
                    "Paired root boundary has inconsistent orientation",
                ));
            }
            reverse = Some(needed);
        }
        let reverse =
            reverse.ok_or_else(|| unproved("Paired boundary has no shared root edges"))?;
        t.shells.push(brep_topology::Shell {
            faces: vec![
                brep_topology::FaceUse {
                    face: 0,
                    reversed: false,
                },
                brep_topology::FaceUse {
                    face: 1,
                    reversed: reverse,
                },
            ],
            closed: true,
        });
        t.bodies.push(brep_topology::Body {
            outer_shell: 0,
            inner_shells: Vec::new(),
        });
        let model = MixedBrep::from_topology(t)?;
        Ok(QualifiedPairedRootBoundary {
            census: self.clone(),
            signatures: signatures.map(|s| s.to_vec()),
            limits,
            model,
            genus: hole_counts[0],
        })
    }
}

fn admit_boolean_boundary(
    boundary: QualifiedPairedRootBoundary,
    selections: [&QualifiedCensusMaterialRectangle; 2],
    volume: RootBoundaryVolumeLimits,
) -> Result<QualifiedPairedRootSolid> {
    let solid = Arc::new(boundary)
        .admit_material(volume)?
        .into_solid()
        .ok_or_else(|| unproved("Boolean candidate volume is unproved"))?;
    for (face, selection) in solid.model().topology().shells[0].faces.iter().zip(selections) {
        if face.reversed != selection.boolean_material().reversed_surface() {
            return Err(unproved("Boolean candidate disagrees with operand material orientation"));
        }
    }
    Ok(solid)
}

mod volume;
pub use volume::{QualifiedRootBoolean, SeparatedRootBoolean,QualifiedRootShellForest,RootRelationKind,RootRelationLimits,RootRelationWork,QualifiedRootRelation,RootDisjointBodyAssembly, RootShellSubtraction, RootCavityLimits,RootCavityCoverageWork,QualifiedRootCavitySolid,RootSurfaceMaterialLimits,QualifiedRootSurfaceMaterialBox,RootMaterialBoxLimits,RootMaterialBoxWork,QualifiedRootMaterialBox,RootSolidPointLimits, RootSolidPointReport, RootSolidRayReport, RootBoundaryVolumeLimits,RootBoundaryVolumeReport};

mod material;
pub use material::{PairedRootSolidAdmission,QualifiedPairedRootSolid};
