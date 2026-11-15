//! Paired retained patches whose complete cross-source contacts are their
//! literal common root boundary. Outward material orientation remains separate.
use super::*;
mod patch_shells;

type RegionSeed = [[f64; 2]; 2];
type MaterialSeed = (RegionSeed, Option<bool>);

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
fn insert_edge(t: &mut MixedTopology, edge: &ss::OriginalSsRootEdge) -> Result<usize> {
    for (id, old) in t.edges.iter().enumerate() {
        if let MixedEdge::Root(old) = &old.curve {
            if old.start().point().same_selector(edge.start().point())
                && old.end().point().same_selector(edge.end().point())
                && old.curve().same_definition(edge.curve())?
            {
                return Ok(id);
            }
        }
    }
    let mut vertices = [0; 2];
    for (i, port) in [edge.start(), edge.end()].into_iter().enumerate() {
        vertices[i] =
            if let Some(id) = t.vertices.iter().position(
                |v| matches!(&v.point,MixedVertex::Root(p) if p.same_selector(port.point())),
            ) {
                id
            } else {
                let id = t.vertices.len();
                t.vertices.push(brep_topology::Vertex {
                    point: MixedVertex::Root(port.point().clone()),
                });
                id
            };
    }
    let id = t.edges.len();
    t.edges.push(brep_topology::Edge {
        degenerate: false,
        vertices,
        curve: MixedEdge::Root(edge.clone()),
    });
    Ok(id)
}
impl QualifiedInteriorSsCensus {
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
        let containment = RootMaterialBoxLimits {
            point: RootSolidPointLimits {
                tolerance_uv: self.parent(0)?.limits.tolerance_uv
                    .min(self.parent(1)?.limits.tolerance_uv),
                arc_subdivisions: volume.arc_subdivisions,
                band_queries: volume.band_queries,
                winding_queries: volume.winding_queries,
                winding: volume.winding,
                geometry: volume.geometry,
                ..Default::default()
            },
            ..Default::default()
        };
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
        use crate::closed_solid::{BooleanFaceDisposition, BooleanSourceSide, qualify_surface_material_box};
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
            for (id, face) in cut.source.model().faces.iter().enumerate() {
                if id == cut.face {
                    continue;
                }
                let domain = crate::face_domain::FaceDomain::new(
                    cut.source.model(),
                    id,
                    cut.limits.tolerance_uv,
                )?;
                let s = &face.surface;
                let mut pending = vec![[
                    [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                    [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
                ]];
                let mut disposition = None;
                while let Some(mut cell) = pending.pop() {
                    max_carrier_cells = max_carrier_cells.checked_sub(1)
                        .ok_or_else(|| unproved("Boolean carrier cover exhausted"))?;
                    let opposing_material = qualify_surface_material_box(
                        &parents[1 - side].source, s, cell, directions, material.boolean.material,
                    );
                    let cell_disposition = opposing_material.ok()
                        .map(|proof| operation.face_disposition(role, proof.inside()));
                    if cell_disposition == Some(BooleanFaceDisposition::Discard)
                        && disposition.is_none_or(|prior| prior == BooleanFaceDisposition::Discard)
                    {
                        disposition = cell_disposition;
                        continue;
                    }
                    if domain.classify(cell, material.boolean.domain_cells)?.location == Location::Outside {
                        continue;
                    }
                    if let Some(current) = cell_disposition {
                        if disposition.is_some_and(|prior| prior != current) {
                            return Err(unproved("Uncut Boolean face has no proved uniform disposition"));
                        }
                        disposition = Some(current);
                        continue;
                    }
                    let axis = usize::from(cell[1][1] - cell[1][0] > cell[0][1] - cell[0][0]);
                    let [lo, hi] = cell[axis];
                    let mid = lo * 0.5 + hi * 0.5;
                    if !(lo < mid && mid < hi) {
                        return Err(unproved("Boolean carrier material remains unresolved"));
                    }
                    let mut right = cell;
                    cell[axis][1] = mid;
                    right[axis][0] = mid;
                    pending.extend([right, cell]);
                }
                match disposition {
                    Some(BooleanFaceDisposition::Discard) => (),
                    Some(BooleanFaceDisposition::Keep | BooleanFaceDisposition::Reverse) => {
                        let mut uses = cut.source.model().shells.iter()
                            .flat_map(|shell| &shell.faces).filter(|use_| use_.face == id);
                        let use_ = uses.next().filter(|_| uses.next().is_none())
                            .ok_or_else(|| unproved("Uncut Boolean face needs one source material use"))?;
                        uncut[side].push(brep_topology::FaceUse {
                            face: id,
                            reversed: use_.reversed ^ (disposition == Some(BooleanFaceDisposition::Reverse)),
                        });
                    }
                    None => return Err(unproved("Uncut Boolean face has no covered material domain")),
                }
            }
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
                    let id = insert_edge(&mut t, edge)?;
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
