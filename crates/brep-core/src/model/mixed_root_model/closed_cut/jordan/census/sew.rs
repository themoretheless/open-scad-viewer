//! Paired retained patches whose complete cross-source contacts are their
//! literal common root boundary. Outward material orientation remains separate.
use super::*;
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
        let (solids, _) = self.qualify_paired_boolean_components(
            operation, components, excluded_seeds, directions, tolerance_mm,
            material, arrangement, volume, max_carrier_cells,
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
        let (solids, relations) = self.qualify_paired_boolean_components(
            operation, components, excluded_seeds, directions, tolerance_mm,
            material, arrangement, volume, max_carrier_cells,
        )?;
        if let [solid] = solids.as_slice() {
            return Ok((solid.model().clone(), solid.volume()));
        }
        let forest = QualifiedRootShellForest::qualify(solids, relations, volume.absolute_error)?;
        Ok((forest.model().clone(), forest.volume()))
    }

    fn qualify_paired_boolean_components(
        self: &Arc<Self>,
        operation: crate::closed_solid::BooleanMaterialOp,
        components: &[[[[f64; 2]; 2]; 2]],
        excluded_seeds: [&[[[f64; 2]; 2]]; 2],
        directions: &[[f64; 3]],
        tolerance_mm: f64,
        material: CutMaterialLimits,
        arrangement: [CensusArrangementLimits; 2],
        volume: RootBoundaryVolumeLimits,
        mut max_carrier_cells: usize,
    ) -> Result<(Vec<Arc<QualifiedPairedRootSolid>>, Vec<Arc<QualifiedRootRelation>>)> {
        use crate::closed_solid::{BooleanFaceDisposition, BooleanSourceSide, qualify_surface_material_box};
        if !(1..=16384).contains(&max_carrier_cells) || !(1..=64).contains(&components.len()) {
            return Err(unproved("Boolean requires a bounded carrier cover"));
        }
        let parents = [self.parent(0)?, self.parent(1)?];
        let layouts = [
            Arc::new(self.build_face_arrangement(0, arrangement[0])?),
            Arc::new(self.build_face_arrangement(1, arrangement[1])?),
        ];
        let mut selections = [Vec::new(), Vec::new()];
        for (side, role) in [BooleanSourceSide::First, BooleanSourceSide::Second].into_iter().enumerate() {
            let cut = parents[side];
            if !cut.source.model().faces[cut.face].holes.is_empty() {
                return Err(unproved("Complete paired Boolean requires cut faces without original holes"));
            }
            let layout = &layouts[side];
            if excluded_seeds[side].len() + components.len() != layout.region_faces().len() {
                return Err(unproved("Boolean requires a seed for every arrangement region"));
            }
            let mut classified = std::collections::BTreeSet::new();
            for seeds in components {
                let retained = layout.qualify_boolean_rectangle(
                    operation, seeds[side], directions, material,
                )?;
                if !retained.material().boolean_material().retain() || !classified.insert(retained.face()) {
                    return Err(unproved("Boolean component seeds must select distinct retained regions"));
                }
                selections[side].push(retained);
            }
            for &seed in excluded_seeds[side] {
                let outside = layout.qualify_boolean_rectangle(
                    operation, seed, directions, material,
                )?;
                if outside.material().boolean_material().retain() || !classified.insert(outside.face()) {
                    return Err(unproved("Boolean exclusion seeds must select distinct discarded regions"));
                }
            }
            for (id, face) in cut.source.model().faces.iter().enumerate() {
                if id == cut.face {
                    continue;
                }
                let s = &face.surface;
                let mut pending = vec![[
                    [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                    [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
                ]];
                while let Some(mut cell) = pending.pop() {
                    max_carrier_cells = max_carrier_cells.checked_sub(1)
                        .ok_or_else(|| unproved("Boolean carrier cover exhausted"))?;
                    match qualify_surface_material_box(
                        &parents[1 - side].source, s, cell, directions, material.boolean.material,
                    ) {
                        Ok(proof) if operation.face_disposition(role, proof.inside()) == BooleanFaceDisposition::Discard => continue,
                        Ok(_) => return Err(unproved("Boolean has an additional material face")),
                        Err(_) => {}
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
            }
        }
        let mut solids = Vec::with_capacity(components.len());
        let mut relations = Vec::new();
        for (first, second) in selections[0].iter().zip(&selections[1]) {
            let selected = [first.material(), second.material()];
            let signatures = selected.map(|selection| selection.region().signature().unwrap());
            let boundary = self.sew_paired_arrangements(
                signatures, tolerance_mm, arrangement, [&layouts[0], &layouts[1]],
            )?;
            let solid = Arc::new(admit_boolean_boundary(boundary, selected, volume)?);
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
