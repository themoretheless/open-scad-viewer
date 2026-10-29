//! Closed oriented incidence for exact profile/transition expression faces.
//! Global embedding, volume, radius and wall admission remain separate.
#[path = "source_profile_shell_tangency.rs"]
mod tangency;
pub use tangency::{TransitionNeighborTangency,TransitionNeighborTangencyReport};
#[path = "source_profile_shell_geometry.rs"]
mod geometry;
pub use geometry::EmbeddedExpressionGeometry;
#[path = "source_profile_shell_flux.rs"]
mod flux;
pub use flux::MaterialFluxReport;
#[path = "source_profile_shell_gap.rs"]
mod gap;
pub use gap::{ExpressionFaceClearance, ExpressionGapReport};
#[path = "source_profile_shell_segment.rs"]
mod segment;
pub use segment::{
    ExpressionEndpointNormalReport, ExpressionLineReport, ExpressionMaterialInterval,
    ExpressionMaterialLine, ExpressionMaterialLineQualification, ExpressionMaterialLineReport,
    ExpressionSegmentCandidate, ExpressionSegmentReport, ExpressionTrimmedLineReport,
};
#[path = "source_profile_shell_volume.rs"]
mod volume;
pub use volume::{
    ExpressionVolumeLimits, ExpressionVolumeQualification, ExpressionVolumeReport,
    OrientedExpressionVolume,
};
#[path = "source_profile_shell_wall.rs"]
mod wall;
use super::*;
use crate::source_boundary_fragment::Fragment;
use crate::source_shell_incidence::Address;
use std::collections::{BTreeMap, BTreeSet};
pub use wall::{
    ExpressionAlignedChordMinimum, ExpressionWallCell, ExpressionWallPendingReason,
    ExpressionWallQualification, ExpressionWallReport,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellFaceKind {
    Original(usize),
    Retained(usize),
    Transition,
}
pub enum ShellUse {
    Source(Fragment),
    Transition(usize),
}
pub struct ShellFace {
    kind: ShellFaceKind,
    loops: Vec<Vec<ShellUse>>,
}
impl ShellFace {
    pub fn kind(&self) -> ShellFaceKind {
        self.kind
    }
    pub fn loops(&self) -> &[Vec<ShellUse>] {
        &self.loops
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellPairKind {
    Original(usize),
    Outer(usize),
    Rail(usize),
    Section(usize),
}
pub struct ShellPair {
    uses: [Address; 2],
    reversed: [bool; 2],
    kind: ShellPairKind,
}
impl ShellPair {
    pub fn uses(&self) -> [Address; 2] {
        self.uses
    }
    pub fn reversed(&self) -> [bool; 2] {
        self.reversed
    }
    pub fn kind(&self) -> ShellPairKind {
        self.kind
    }
}
pub struct ExpressionShell<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    assembly: &'x OuterEdgeAssembly<'o, 'i, 'f, 'a, 's, 'p, 'b>,
    faces: Vec<ShellFace>,
    pairs: Vec<ShellPair>,
    poles: Vec<Address>,
    vertices: usize,
    vertex_ids: BTreeMap<(usize, usize, usize), usize>,
    components: usize,
}
fn original_support_derivatives(
    source: &nurbs_core::surface::Surface,
    domain: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
    source.validate()?;
    let natural = [
        [
            source.knots_u[source.degree_u],
            source.knots_u[source.control_points.len()],
        ],
        [
            source.knots_v[source.degree_v],
            source.knots_v[source.control_points[0].len()],
        ],
    ];
    if max_spans > 100000
        || (0..2).any(|k| {
            !domain[k].iter().all(|v| v.is_finite())
                || domain[k][0] > domain[k][1]
                || domain[k][0] < natural[k][0]
                || domain[k][1] > natural[k][1]
        })
    {
        return Err(Error::new(
            "BREP_PROFILE_SHELL",
            "Keep original derivatives in the natural domain and bounded work",
        ));
    }
    let mut out = [[[0.; 2]; 3]; 2];
    let mut spent = 0;
    for axis in 0..2 {
        if spent == max_spans {
            return Ok((None, spent));
        }
        let (bounds, used) = nurbs_core::normal_alignment::derivative_path_bounds(
            source,
            domain,
            axis,
            max_spans - spent,
        )?;
        spent += used;
        let Some(bounds) = bounds else {
            return Ok((None, spent));
        };
        out[axis] = bounds.components();
    }
    Ok((Some(out), spent))
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Flux of a complete support rectangle in its native chart orientation.
    /// The caller must classify material trim and apply shell orientation before
    /// summing volume. A support rectangle alone never admits a solid.
    pub fn enclose_support_flux(
        &self,
        face: usize,
        domain: [[f64; 2]; 2],
        axis: usize,
        origin: f64,
        max_controls: usize,
        max_spans: usize,
    ) -> Result<(Option<nurbs_core::surface_flux::Bound>, usize)> {
        if axis >= 3 || !origin.is_finite() {
            return Err(Error::new(
                "BREP_PROFILE_SHELL",
                "Choose a finite support flux axis and origin",
            ));
        }
        let (position, position_spans) =
            self.enclose_support(face, domain, max_controls, max_spans)?;
        let Some(position) = position else {
            return Ok((None, position_spans));
        };
        let (derivatives, derivative_spans) = self.enclose_support_derivatives(
            face,
            domain,
            max_controls,
            max_spans - position_spans,
        )?;
        let spent = position_spans + derivative_spans;
        let Some(derivatives) = derivatives else {
            return Ok((None, spent));
        };
        let bound = nurbs_core::surface_flux::bound_expression(
            position,
            derivatives,
            domain,
            axis,
            origin,
            false,
        )?;
        Ok((Some(bound), spent))
    }
    /// First derivatives in the same parameters as `enclose_support`.
    /// These enclose the supporting expression, without a trim classification.
    pub fn enclose_support_derivatives(
        &self,
        face: usize,
        domain: [[f64; 2]; 2],
        max_controls: usize,
        max_spans: usize,
    ) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
        if max_controls > 1000000
            || max_spans > 100000
            || domain
                .iter()
                .any(|r| !r.iter().all(|v| v.is_finite()) || r[0] > r[1])
        {
            return Err(Error::new(
                "BREP_PROFILE_SHELL",
                "Bound support derivative parameters and work",
            ));
        }
        let kind = self
            .faces
            .get(face)
            .ok_or_else(|| Error::new("BREP_PROFILE_SHELL", "Unknown expression face"))?
            .kind;
        if max_spans == 0 {
            let natural = self.support_domain(face)?;
            if (0..2).any(|k| domain[k][0] < natural[k][0] || domain[k][1] > natural[k][1]) {
                return Err(Error::new(
                    "BREP_PROFILE_SHELL",
                    "Keep exhausted derivative queries in the owned support domain",
                ));
            }
            return Ok((None, 0));
        }

        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        match kind {
            ShellFaceKind::Retained(_) => {
                let (bounds, spent) = profile.derivative_bounds(domain, max_spans)?;
                Ok((bounds.map(|b| std::array::from_fn(|axis| std::array::from_fn(|k| b[k][axis]))), spent))
            }
            ShellFaceKind::Transition => {
                let first = profile.section(root.axis, root.parameters[0])?;
                let last = profile.section(root.axis, root.parameters[1])?;
                let width = root.parameters[1] - root.parameters[0];
                let bridge = nurbs_core::normal_alignment::ProfileSectionBridge::new(first, last, [width; 2])?;
                let (constrained, _) = bridge.constrain_original_rails(max_controls)?;
                let Some(constrained) = constrained else { return Ok((None, 0)); };
                constrained.derivatives_with_transverse_correction(domain[0], domain[1], max_spans)
            }
            ShellFaceKind::Original(index) => {
                let source = root.restricted.proposal.body.geometry().shell().faces()[index][0].edges()[0].surface();
                original_support_derivatives(source,domain,max_spans)
            }
        }
    }
    pub fn assembly(&self) -> &OuterEdgeAssembly<'_, '_, '_, '_, '_, '_, '_> {
        self.assembly
    }
    /// Enclose original/exact-profile normals per knot span before hulling.
    /// Corrected transitions retain their own expression derivative bounds.
    pub fn enclose_support_normals(
        &self, face:usize, domain:[[f64;2];2], max_controls:usize, max_spans:usize,
    ) -> Result<(Option<[[f64;2];3]>,usize)> {
        // Reuse owner/domain validation, including the exhausted-budget path.
        self.enclose_support_derivatives(face,domain,max_controls,0)?;
        if max_spans>100000 {return Err(Error::new("BREP_PROFILE_SHELL","Bound normal span work"));}
        if max_spans==0 {return Ok((None,0));}
        let root=self.assembly.patch.rails.face.boundary.rooted[0];
        match self.faces[face].kind {
            ShellFaceKind::Original(index)=> {
                let source=root.restricted.proposal.body.geometry().shell();
                nurbs_core::normal_alignment::normal_bounds(source.faces()[index][0].edges()[0].surface(),domain,max_spans)
                    .map_err(Into::into)
            }
            ShellFaceKind::Retained(_)=>root.restricted.proposal.profile(root.face).unwrap()
                .normal_bounds(domain,max_spans).map_err(Into::into),
            ShellFaceKind::Transition=> {
                let (jet,spent)=self.enclose_support_derivatives(face,domain,max_controls,max_spans)?;
                Ok((jet.map(segment::normal_from_derivatives).transpose()?,spent))
            }
        }
    }
    /// Direction bounds may divide out a proven nonzero interior pole factor.
    /// Boundary rectangles keep the ordinary normal enclosure and uncertainty.
    pub fn enclose_support_normal_directions(
        &self,face:usize,domain:[[f64;2];2],max_controls:usize,max_spans:usize,
    ) -> Result<(Option<[[f64;2];3]>,usize)> {
        self.enclose_support_derivatives(face,domain,max_controls,0)?;
        if matches!(self.faces[face].kind,ShellFaceKind::Retained(_)) {
            let root=self.assembly.patch.rails.face.boundary.rooted[0];
            return root.restricted.proposal.profile(root.face).unwrap()
                .interior_normal_direction_bounds(domain,max_spans).map_err(Into::into);
        }
        self.enclose_support_normals(face,domain,max_controls,max_spans)
    }
    pub fn faces(&self) -> &[ShellFace] {
        &self.faces
    }
    pub fn pairs(&self) -> &[ShellPair] {
        &self.pairs
    }
    pub fn poles(&self) -> &[Address] {
        &self.poles
    }
    pub fn vertices(&self) -> usize {
        self.vertices
    }
    /// Qualified shell vertex at a traversal endpoint. IDs are local to this
    /// shell and come from exact incidence, never Cartesian proximity.
    pub fn boundary_vertex(&self, address: Address, end: usize) -> Option<usize> {
        if end > 1 { return None; }
        let wire = self.faces.get(address.face)?.loops.get(address.wire)?;
        if address.edge >= wire.len() { return None; }
        let edge = (address.edge + end) % wire.len();
        self.vertex_ids.get(&(address.face, address.wire, edge)).copied()
    }
    pub fn components(&self) -> usize {
        self.components
    }
    /// Conservative position enclosure of the exact supporting expression.
    /// This does not assert that the rectangle lies inside its trimmed region.
    /// Original/retained coordinates are (u,v); transition coordinates are
    /// (source free parameter, normalized across parameter).
    pub fn enclose_support(
        &self,
        face: usize,
        domain: [[f64; 2]; 2],
        max_controls: usize,
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        use nurbs_core::interval_eval::{Interval, evaluate_surface_interval};
        if max_controls > 1000000
            || max_spans > 100000
            || domain
                .iter()
                .any(|r| !r.iter().all(|x| x.is_finite()) || r[0] > r[1])
        {
            return Err(Error::new(
                "BREP_PROFILE_SHELL",
                "Bound support enclosure parameters and work",
            ));
        }
        let kind = self
            .faces
            .get(face)
            .ok_or_else(|| Error::new("BREP_PROFILE_SHELL", "Unknown expression face"))?
            .kind;
        let boundary = self.assembly.patch.rails.face.boundary;
        let root = boundary.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        match kind {
            ShellFaceKind::Retained(_) => profile.enclose(domain, max_spans),
            ShellFaceKind::Transition => {
                let first = profile.section(root.axis, root.parameters[0])?;
                let last = profile.section(root.axis, root.parameters[1])?;
                let width = root.parameters[1] - root.parameters[0];
                let bridge = nurbs_core::normal_alignment::ProfileSectionBridge::new(
                    first, last, [width; 2],
                )?;
                let (constrained, _) = bridge.constrain_original_rails(max_controls)?;
                let Some(constrained) = constrained else {
                    return Ok((None, 0));
                };
                constrained.enclose_with_transverse_correction(domain[0], domain[1], max_spans)
            }
            ShellFaceKind::Original(index) => {
                let original = root.restricted.proposal.body.geometry().shell();
                let surface = original.faces()[index][0].edges()[0].surface();
                let spans = [
                    (
                        &surface.knots_u,
                        surface.degree_u,
                        surface.control_points.len(),
                    ),
                    (
                        &surface.knots_v,
                        surface.degree_v,
                        surface.control_points[0].len(),
                    ),
                ];
                let mut counts = [0usize; 2];
                for axis in 0..2 {
                    let (knots, degree, count) = spans[axis];
                    if domain[axis][0] < knots[degree] || domain[axis][1] > knots[count] {
                        return Err(Error::new(
                            "BREP_PROFILE_SHELL",
                            "Keep support rectangle in active knot domain",
                        ));
                    }
                    counts[axis] = (degree..count)
                        .filter(|&i| {
                            knots[i] < knots[i + 1]
                                && knots[i] <= domain[axis][1]
                                && knots[i + 1] >= domain[axis][0]
                        })
                        .count();
                }
                let needed = counts[0] * counts[1];
                if needed > max_spans {
                    return Ok((None, 0));
                }
                let result = evaluate_surface_interval(
                    surface,
                    Interval::new(domain[0][0], domain[0][1])?,
                    Interval::new(domain[1][0], domain[1][1])?,
                )?;
                let bounds = std::array::from_fn(|i| [result[i].lo, result[i].hi]);
                Ok((Some(bounds), needed))
            }
        }
    }
}
pub struct ShellAssemblyReport<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    pub shell: Option<ExpressionShell<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>>,
    pub uses: usize,
    pub reason: &'static str,
}
fn key(a: Address) -> (usize, usize, usize) {
    (a.face, a.wire, a.edge)
}
fn locate(faces: &[ShellFace], ids: &[usize], fragment: &Fragment) -> Option<Address> {
    let mut found = None;
    for &face in ids {
        for (wire, loop_) in faces[face].loops.iter().enumerate() {
            for (edge, use_) in loop_.iter().enumerate() {
                if matches!(use_,ShellUse::Source(f) if f.same_definition(fragment)) {
                    if found.is_some() {
                        return None;
                    }
                    found = Some(Address { face, wire, edge });
                }
            }
        }
    }
    found
}
fn component_count(n: usize, pairs: &[[Address; 2]]) -> usize {
    let mut seen = vec![false; n];
    let mut count = 0;
    for start in 0..n {
        if seen[start] {
            continue;
        }
        count += 1;
        let mut stack = vec![start];
        seen[start] = true;
        while let Some(face) = stack.pop() {
            for pair in pairs {
                for side in 0..2 {
                    if pair[side].face == face && !seen[pair[1 - side].face] {
                        seen[pair[1 - side].face] = true;
                        stack.push(pair[1 - side].face);
                    }
                }
            }
        }
    }
    count
}
fn qualify_vertex_link(edges: &[(usize, usize)]) -> std::result::Result<(), &'static str> {
    if edges.is_empty() {
        return Err("profile-shell-empty-vertex-link");
    }
    let mut degrees = BTreeMap::new();
    for &(a, b) in edges {
        *degrees.entry(a).or_insert(0) += 1;
        *degrees.entry(b).or_insert(0) += 1;
    }
    if degrees.values().any(|d| *d != 2) {
        return Err("profile-shell-nonmanifold-vertex");
    }
    let mut seen = BTreeSet::new();
    let mut stack = vec![*degrees.keys().next().unwrap()];
    while let Some(node) = stack.pop() {
        if !seen.insert(node) {
            continue;
        }
        for &(a, b) in edges {
            if a == node {
                stack.push(b);
            }
            if b == node {
                stack.push(a);
            }
        }
    }
    if seen.len() != degrees.len() {
        return Err("profile-shell-disconnected-vertex-link");
    }
    Ok(())
}
struct Union {
    parents: Vec<usize>,
}
impl Union {
    fn root(&mut self, mut n: usize) -> usize {
        while self.parents[n] != n {
            let p = self.parents[n];
            self.parents[n] = self.parents[p];
            n = self.parents[n];
        }
        n
    }
    fn join(&mut self, a: usize, b: usize) {
        let a = self.root(a);
        let b = self.root(b);
        self.parents[b] = a;
    }
}
impl<'o, 'i, 'f, 'a, 's, 'p, 'b> OuterEdgeAssembly<'o, 'i, 'f, 'a, 's, 'p, 'b> {
    pub fn assemble_shell<'x>(
        &'x self,
        max_uses: usize,
    ) -> Result<ShellAssemblyReport<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>> {
        if max_uses > 100000 {
            return Err(Error::new(
                "BREP_PROFILE_SHELL",
                "Bound expression shell uses",
            ));
        }
        let mut out = ShellAssemblyReport {
            shell: None,
            uses: 0,
            reason: "profile-shell-work-limit",
        };
        let patch = self.patch;
        let boundary = patch.rails.face.boundary;
        let selected = boundary.rooted[0].face;
        let original = boundary.rooted[0]
            .restricted
            .proposal
            .body
            .geometry()
            .shell();
        let regions = original.regions().unwrap();
        let retained_ids = [selected, regions.len()];
        let transition_id = regions.len() + 1;
        let cost: usize = regions
            .iter()
            .enumerate()
            .filter(|(face, _)| *face != selected)
            .map(|(_, r)| r.loops().iter().map(Vec::len).sum::<usize>())
            .sum::<usize>()
            + patch.retained.iter().map(Vec::len).sum::<usize>()
            + 8;
        if cost > max_uses {
            return Ok(out);
        }
        out.uses = cost;
        let mut replacements = BTreeMap::new();
        for rail in 0..2 {
            let root = boundary.rooted[rail];
            let neighbor = 1 - root.points[0].source_side();
            let address = original.uses()[root.restricted.edge][neighbor];
            let contour = &patch.rails.rails[rail].neighbor_contour;
            let [wire, index] = contour.middle;
            if replacements
                .insert(
                    key(address),
                    contour.loops[wire][index - 1..=index + 1].to_vec(),
                )
                .is_some()
            {
                out.reason = "profile-shell-replacement-conflict";
                return Ok(out);
            }
        }
        let mut faces = Vec::new();
        let mut unchanged = BTreeMap::new();
        for (face, region) in regions.iter().enumerate() {
            if face == selected {
                faces.push(ShellFace {
                    kind: ShellFaceKind::Retained(0),
                    loops: vec![patch.retained[0]
                        .iter()
                        .cloned()
                        .map(ShellUse::Source)
                        .collect()],
                });
                continue;
            }
            let mut loops = Vec::new();
            for (wire, source) in region.loops().iter().enumerate() {
                let mut loop_ = Vec::new();
                for (edge, fragment) in source.iter().enumerate() {
                    let address = Address { face, wire, edge };
                    if let Some(replacement) = replacements.get(&key(address)) {
                        loop_.extend(replacement.iter().cloned().map(ShellUse::Source));
                    } else {
                        unchanged.insert(
                            key(address),
                            Address {
                                face,
                                wire,
                                edge: loop_.len(),
                            },
                        );
                        loop_.push(ShellUse::Source(fragment.clone()));
                    }
                }
                loops.push(loop_);
            }
            faces.push(ShellFace {
                kind: ShellFaceKind::Original(face),
                loops,
            });
        }
        faces.push(ShellFace {
            kind: ShellFaceKind::Retained(1),
            loops: vec![patch.retained[1]
                .iter()
                .cloned()
                .map(ShellUse::Source)
                .collect()],
        });
        let order = if patch.rails.reverse_face {
            [3, 2, 1, 0]
        } else {
            [0, 1, 2, 3]
        };
        faces.push(ShellFace {
            kind: ShellFaceKind::Transition,
            loops: vec![order.into_iter().map(ShellUse::Transition).collect()],
        });
        for face in &faces {
            if face.kind == ShellFaceKind::Transition {
                continue;
            }
            for loop_ in &face.loops {
                for i in 0..loop_.len() {
                    let (ShellUse::Source(a), ShellUse::Source(b)) =
                        (&loop_[i], &loop_[(i + 1) % loop_.len()])
                    else {
                        unreachable!()
                    };
                    if !a.joins(b) {
                        out.reason = "profile-shell-loop-joins-unproven";
                        return Ok(out);
                    }
                }
            }
        }
        let map_original = |address: Address| -> Option<Address> {
            if address.face == selected {
                locate(
                    &faces,
                    &retained_ids,
                    &regions[selected].loops()[address.wire][address.edge],
                )
            } else {
                unchanged.get(&key(address)).copied()
            }
        };
        let mut pairs = Vec::new();
        for (edge, uses) in original.uses().iter().enumerate() {
            if boundary.rooted.iter().any(|r| r.restricted.edge == edge) {
                continue;
            }
            let [Some(a), Some(b)] = uses.map(map_original) else {
                out.reason = "profile-shell-original-use-unmapped";
                return Ok(out);
            };
            pairs.push(ShellPair {
                uses: [a, b],
                reversed: original.edges()[edge].reversed(),
                kind: ShellPairKind::Original(edge),
            });
        }
        for rail in 0..2 {
            let root = boundary.rooted[rail];
            let neighbor = 1 - root.points[0].source_side();
            let neighbor_face = original.uses()[root.restricted.edge][neighbor].face;
            for end in 0..2 {
                let edge = &self.edges[rail * 2 + end];
                let a = locate(&faces, &[retained_ids[end]], &edge.uses()[0]);
                let b = locate(&faces, &[neighbor_face], &edge.uses()[1]);
                let (Some(a), Some(b)) = (a, b) else {
                    out.reason = "profile-shell-outer-use-unmapped";
                    return Ok(out);
                };
                pairs.push(ShellPair {
                    uses: [a, b],
                    reversed: edge.reversed(),
                    kind: ShellPairKind::Outer(rail * 2 + end),
                });
            }
            let certified = &patch.rails.rails[rail];
            let edge_index = if rail == 0 { 0 } else { 2 };
            let transition = Address {
                face: transition_id,
                wire: 0,
                edge: order.iter().position(|e| *e == edge_index).unwrap(),
            };
            let Some(neighbor_use) =
                locate(&faces, &[neighbor_face], &certified.edge.uses()[neighbor])
            else {
                out.reason = "profile-shell-rail-use-unmapped";
                return Ok(out);
            };
            pairs.push(ShellPair {
                uses: [transition, neighbor_use],
                reversed: [
                    certified.transition_reversed ^ patch.rails.reverse_face,
                    certified.edge.reversed()[neighbor],
                ],
                kind: ShellPairKind::Rail(rail),
            });
        }
        for end in 0..2 {
            let fragment = patch.retained[end].last().unwrap();
            let Some(retained) = locate(&faces, &[retained_ids[end]], fragment) else {
                unreachable!()
            };
            let edge_index = if end == 0 { 3 } else { 1 };
            let transition = Address {
                face: transition_id,
                wire: 0,
                edge: order.iter().position(|e| *e == edge_index).unwrap(),
            };
            let axis = boundary.rooted[0].axis;
            let descending = fragment.curve().control_points[1][1 - axis]
                < fragment.curve().control_points[0][1 - axis];
            pairs.push(ShellPair {
                uses: [retained, transition],
                reversed: [
                    descending ^ fragment.reversed(),
                    (end == 0) ^ patch.rails.reverse_face,
                ],
                kind: ShellPairKind::Section(end),
            });
        }
        let mut poles = Vec::new();
        for (address, pole) in original.poles() {
            let mapped = if address.face == selected {
                locate(&faces, &retained_ids, pole.source())
            } else {
                unchanged.get(&key(*address)).copied()
            };
            let Some(mapped) = mapped else {
                out.reason = "profile-shell-pole-unmapped";
                return Ok(out);
            };
            poles.push(mapped);
        }
        let mut indices = BTreeMap::new();
        let mut n = 0;
        for (face, f) in faces.iter().enumerate() {
            for (wire, w) in f.loops.iter().enumerate() {
                for edge in 0..w.len() {
                    indices.insert((face, wire, edge), n);
                    n += 1;
                }
            }
        }
        if n != cost {
            out.reason = "profile-shell-use-accounting-conflict";
            return Ok(out);
        }
        let next = |a: Address| Address {
            edge: (a.edge + 1) % faces[a.face].loops[a.wire].len(),
            ..a
        };
        let mut union = Union {
            parents: (0..n).collect(),
        };
        let mut owned = vec![false; n];
        let mut pair_ids = BTreeMap::new();
        for (id, pair) in pairs.iter().enumerate() {
            if pair.uses[0].face == pair.uses[1].face || pair.reversed[0] == pair.reversed[1] {
                out.reason = "profile-shell-opposite-use-unproven";
                return Ok(out);
            }
            for address in pair.uses {
                let i = indices[&key(address)];
                if owned[i] {
                    out.reason = "profile-shell-duplicate-use";
                    return Ok(out);
                }
                owned[i] = true;
                pair_ids.insert(key(address), id);
            }
            union.join(
                indices[&key(pair.uses[0])],
                indices[&key(next(pair.uses[1]))],
            );
            union.join(
                indices[&key(next(pair.uses[0]))],
                indices[&key(pair.uses[1])],
            );
        }
        let pole_set = poles.iter().map(|a| key(*a)).collect::<BTreeSet<_>>();
        for &address in &poles {
            let i = indices[&key(address)];
            if owned[i] {
                out.reason = "profile-shell-pole-use-conflict";
                return Ok(out);
            }
            owned[i] = true;
            union.join(i, indices[&key(next(address))]);
        }
        if owned.iter().any(|x| !*x) {
            out.reason = "profile-shell-open-use";
            return Ok(out);
        }
        let mut links: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
        for (face, f) in faces.iter().enumerate() {
            for (wire, w) in f.loops.iter().enumerate() {
                let regular = (0..w.len())
                    .filter(|edge| !pole_set.contains(&(face, wire, *edge)))
                    .collect::<Vec<_>>();
                if regular.len() < 2 {
                    out.reason = "profile-shell-degenerate-link";
                    return Ok(out);
                }
                for i in 0..regular.len() {
                    let a = Address {
                        face,
                        wire,
                        edge: regular[i],
                    };
                    let previous = Address {
                        face,
                        wire,
                        edge: regular[(i + regular.len() - 1) % regular.len()],
                    };
                    let vertex = union.root(indices[&key(a)]);
                    links
                        .entry(vertex)
                        .or_default()
                        .push((pair_ids[&key(previous)], pair_ids[&key(a)]));
                }
            }
        }
        for edges in links.values() {
            if let Err(reason) = qualify_vertex_link(edges) {
                out.reason = reason;
                return Ok(out);
            }
        }
        let vertices = (0..n).map(|i| union.root(i)).collect::<BTreeSet<_>>().len();
        let original_vertices = original
            .vertices()
            .iter()
            .flatten()
            .flatten()
            .flat_map(|ends| ends.iter().copied())
            .collect::<BTreeSet<_>>()
            .len();
        let components = component_count(
            faces.len(),
            &pairs.iter().map(|p| p.uses).collect::<Vec<_>>(),
        );
        if vertices != original_vertices + 4
            || pairs.len() != original.edges().len() + 6
            || components != component_count(regions.len(), original.uses())
        {
            out.reason = "profile-shell-topology-change-unproven";
            return Ok(out);
        }
        // Preserve the already qualified endpoint equivalence classes for
        // conforming boundary meshes. Dense IDs are deterministic in address
        // order; discarded union representatives never become geometry.
        let mut representatives = BTreeMap::new();
        let mut vertex_ids = BTreeMap::new();
        for (address, index) in &indices {
            let root = union.root(*index);
            let next_id = representatives.len();
            let id = *representatives.entry(root).or_insert(next_id);
            vertex_ids.insert(*address, id);
        }
        if representatives.len() != vertices {
            return Err(Error::new("BREP_PROFILE_SHELL", "Vertex incidence accounting conflict"));
        }
        out.shell = Some(ExpressionShell {
            assembly: self,
            faces,
            pairs,
            poles,
            vertices,
            vertex_ids,
            components,
        });
        out.reason = "profile-shell-closed-oriented-incidence-qualified";
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn original_derivative_budget_refuses_zero_and_partial_axes() {
        let surface=nurbs_core::surface::Surface {degree_u:1,degree_v:1,
            knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false};
        assert!(super::original_support_derivatives(&surface,[[-1.,1.],[0.,1.]],0).is_err());
        for budget in [0,1] {
            let (bound,spent)=super::original_support_derivatives(&surface,[[0.,1.];2],budget).unwrap();
            assert!(bound.is_none());assert_eq!(spent,budget);
        }
        let (bound,spent)=super::original_support_derivatives(&surface,[[0.,1.];2],2).unwrap();
        assert!(bound.is_some());assert_eq!(spent,2);
    }

    use super::qualify_vertex_link;
    #[test]
    fn vertex_links_refuse_pinches_and_branching() {
        // Two otherwise closed fans touching at a vertex are not one manifold fan.
        assert_eq!(
            qualify_vertex_link(&[(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3)]),
            Err("profile-shell-disconnected-vertex-link")
        );
        // Three faces along a physical edge create branching in the vertex link.
        assert_eq!(
            qualify_vertex_link(&[(0, 1), (1, 2), (2, 0), (0, 3)]),
            Err("profile-shell-nonmanifold-vertex")
        );
        assert!(qualify_vertex_link(&[(0, 1), (1, 2), (2, 0)]).is_ok());
        // A valid two-edge lens has parallel link arcs, each counted once.
        assert!(qualify_vertex_link(&[(0, 1), (1, 0)]).is_ok());
        assert!(qualify_vertex_link(&[]).is_err());
    }
}

#[path = "source_profile_shell_contacts.rs"]
mod contacts;
pub use contacts::{ShellSupportPair, ShellSupportReport, SupportPairReport, SupportProduct};
#[path = "source_profile_shell_original_contact.rs"]
mod original_contact;
pub use original_contact::{
    OriginalAdditionalContact, OriginalAdditionalProof, OriginalAdditionalReport,
    OriginalContactReport, OriginalFiberContact, OriginalFiberProof, OriginalFiberReport,
    OriginalHullContact, OriginalHullReport, OriginalPlanarContact, OriginalPoleContact,
    OriginalPoleProof, OriginalPoleReport,
};
#[path = "source_profile_shell_audit.rs"]
mod audit;
pub use audit::{AuditedContactPair, ContactAuditLimits, PairContactProof, ShellContactAudit};
#[path = "source_profile_shell_transition_contact.rs"]
mod transition_contact;
pub use transition_contact::{TransitionPlaneContact, TransitionPlaneReport};

#[path = "source_profile_shell_ruled_contact.rs"]
mod ruled_contact;
pub use ruled_contact::{
    ProfileRuledContact, ProfileRuledContactReport, ProfileRuledImage, ProfileRuledImageReport,
    TransitionRuledContact, TransitionRuledReport,
};

#[path = "source_profile_shell_section_contact.rs"]
mod section_contact;
pub use section_contact::{TransitionSectionContact, TransitionSectionContactReport};

pub use original_contact::{ProfileHullContact, ProfileHullReport};

pub use original_contact::{
    OriginalBoundaryContact, OriginalBoundaryProof, OriginalBoundaryReport,
};

#[path = "source_profile_shell_xy_contact.rs"]
mod xy_contact;
pub use xy_contact::{TransitionXyHullContact, TransitionXyHullReport};

pub use xy_contact::{TransitionXyBoundsContact, TransitionXyBoundsReport};

#[path = "source_profile_shell_vertex_contact.rs"]
mod vertex_contact;
pub use vertex_contact::{ProfileVertexContact, ProfileVertexReport};

#[path = "source_profile_shell_coordinate_contact.rs"]
mod coordinate_contact;
pub use coordinate_contact::{TransitionCoordinateContact, TransitionCoordinateReport};

#[path = "source_profile_shell_boundary_contact.rs"]
mod profile_boundary_contact;
pub use profile_boundary_contact::{ProfileBoundaryContact, ProfileBoundaryReport};

#[path = "source_profile_shell_paired_contact.rs"]
mod profile_paired_contact;
pub use profile_paired_contact::{ProfilePairedContact, ProfilePairedReport};

#[path = "source_profile_shell_xy_plane_contact.rs"]
mod xy_plane_contact;
pub use xy_plane_contact::{TransitionXyPlaneContact, TransitionXyPlaneReport};

#[path = "source_profile_shell_display.rs"]
mod display;
pub use display::{ExpressionDisplayCell, ExpressionDisplayLimits, ExpressionDisplayReport};

#[path = "source_profile_shell_mesh.rs"]
mod display_mesh;
pub use display_mesh::{
    ExpressionMeshLimits, ExpressionMeshPending, ExpressionMeshReport, ExpressionMeshTile,
};

#[path = "source_profile_shell_edge_mesh.rs"]
mod edge_mesh;
pub use edge_mesh::{ShellBoundaryMeshReport, ShellBoundaryPartition};

#[path = "source_profile_shell_boundary_graph.rs"]
mod boundary_graph;
pub use boundary_graph::{
    BoundaryGraphPending, BoundaryGraphReport, BoundaryGraphSegment, BoundaryGraphVertex,
};

#[path = "source_profile_shell_boundary_loops.rs"]
mod boundary_loops;
pub use boundary_loops::{BoundaryFaceLoop, BoundaryLoopSegment};

#[path = "source_profile_shell_face_candidates.rs"]
mod face_candidates;
pub use face_candidates::{FaceCandidatePending, FaceCandidateReport, FaceTriangleCandidate};

#[path = "source_profile_shell_refine_triangles.rs"]
mod refine_triangles;
pub use refine_triangles::{
    RefinedTriangle, RegionalTransitionReport, TriangleNumericFailure, TriangleRefinementLimits,
    TriangleRefinementPending, TriangleRefinementReport,
};

#[path = "source_triangle_incidence.rs"]
mod triangle_incidence;
pub use triangle_incidence::TriangleIncidenceReport;
impl FaceCandidateReport {
    pub fn inspect_incidence(&self) -> TriangleIncidenceReport {
        triangle_incidence::inspect(
            &self
                .triangles
                .iter()
                .map(|t| t.vertices)
                .collect::<Vec<_>>(),
        )
    }
}

impl TriangleRefinementReport {
    /// Counts all leaves, including unqualified geometry. This audits topology
    /// preservation only, and does not admit pending leaves as material.
    pub fn inspect_leaf_incidence(&self) -> TriangleIncidenceReport {
        triangle_incidence::inspect(
            &self
                .accepted
                .iter()
                .map(|t| t.triangle.vertices)
                .chain(self.pending.iter().map(|t| t.triangle.vertices))
                .collect::<Vec<_>>(),
        )
    }
}

#[path = "source_profile_shell_bisect_edges.rs"]
mod bisect_edges;
pub use bisect_edges::{AdaptiveInteriorLimits, InteriorBisectionReport};
