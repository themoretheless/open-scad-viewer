//! Boundary-preserving exact weight-profile proposal on an immutable original Body.
//! Closed topology is inherited only after every retained contour is checked;
//! embedding, normals, volume and wall qualification must be rebuilt separately.
use crate::source_volume::Body;
use nurbs_core::{Error, Result, normal_alignment::WeightProfile, surface::Surface};
#[derive(Clone, Copy)]
pub struct FaceSpec {
    pub face: usize,
    pub anchor: [usize; 2],
}
pub struct Limits {
    pub controls: usize,
    pub fragments: usize,
    pub position_tolerance: f64,
}
struct Face<'a> {
    spec: FaceSpec,
    profile: WeightProfile<'a>,
}
pub struct Proposal<'a> {
    body: &'a Body,
    faces: Vec<Face<'a>>,
    axis_deviation_upper: [f64; 3],
    position_tolerance: f64,
}
/// Tangent-plane certificate attached to this exact proposal and an owned
/// original shared carrier. It does not admit volume or shell embedding.
pub struct Seam<'p, 'b> {
    proposal: &'p Proposal<'b>,
    faces: [usize; 2],
    edge: usize,
    threshold: f64,
    upper: f64,
}
impl Seam<'_, '_> {
    pub fn proposal(&self) -> &Proposal<'_> {
        self.proposal
    }
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn threshold(&self) -> f64 {
        self.threshold
    }
    pub fn upper_sine_squared(&self) -> f64 {
        self.upper
    }
}
pub struct SeamReport<'p, 'b> {
    pub seam: Option<Seam<'p, 'b>>,
    pub angular: Option<nurbs_core::normal_alignment::ProfilePathReport>,
    pub reason: &'static str,
}
/// An owned new section end with an exact source-parameter recipe. Interval
/// coordinates enclose that recipe; they are not rounded replacement vertices.
pub struct CutEndpoint<'p, 'b> {
    proposal: &'p Proposal<'b>,
    face: usize,
    neighbor: usize,
    edge: usize,
    axis: usize,
    parameter: f64,
    high: bool,
    canonical: [f64; 2],
    neighbor_uv: [[f64; 2]; 2],
    world: [[f64; 2]; 3],
}
/// Two unchanged selected-face fragments sharing one certified crossing root.
/// Neighbor ranges are transported through the same private canonical carrier;
/// they remain expression views until neighbor contour admission is rebuilt.
pub struct CutPartition<'c, 'p, 'b> {
    endpoint: &'c CutEndpoint<'p, 'b>,
    point: crate::source_contact_point::SourcePoint,
    selected: [crate::source_boundary_fragment::Fragment; 2],
    neighbor_ranges: [[[f64; 2]; 2]; 2],
}
/// Freshly admitted original shared-edge halves, retaining both root definitions.
/// This is not a closed replacement contour or a new corrected Body.
pub struct NeighborPartition {
    fragments: [crate::source_boundary_fragment::Fragment; 2],
    edges: [crate::source_shared_edge::SharedEdge; 2],
}
impl NeighborPartition {
    pub fn fragments(&self) -> &[crate::source_boundary_fragment::Fragment; 2] {
        &self.fragments
    }
    pub fn edges(&self) -> &[crate::source_shared_edge::SharedEdge; 2] {
        &self.edges
    }
}
pub struct NeighborPartitionReport {
    pub partition: Option<NeighborPartition>,
    pub mapping_cells: usize,
    pub exact_work: u64,
    pub root_checks: usize,
    pub driver_cells: usize,
    pub reason: &'static str,
}
impl CutPartition<'_, '_, '_> {
    /// Treat the neighbor cutter as a proposal: requalify its root and both
    /// restricted world-edge halves. Parameter enclosure overlap is not identity.
    pub fn qualify_neighbor(
        &self,
        contact: &nurbs_core::curve::Curve,
        selector: [[f64; 2]; 2],
        mapping_cells: usize,
        max_work: u64,
        max_driver: usize,
    ) -> Result<NeighborPartitionReport> {
        if !(1..=100000000).contains(&max_work) || max_driver > 100000 {
            return Err(Error::new(
                "BREP_PROFILE_CUT",
                "Bound fresh neighbor edge work",
            ));
        }
        let shell = self.endpoint.proposal.body.geometry().shell();
        let index = self.endpoint.edge;
        let edge = &shell.edges()[index];
        let addresses = &shell.uses()[index];
        let side = usize::from(addresses[0].face != self.endpoint.face);
        let other = &edge.uses()[1 - side];
        let root = crate::source_contact_point::qualify(
            other.surface(),
            other.curve(),
            contact,
            selector,
            mapping_cells,
        )?;
        let mut out = NeighborPartitionReport {
            partition: None,
            mapping_cells: root.mapping_cells,
            exact_work: 0,
            root_checks: 0,
            driver_cells: 0,
            reason: root.reason,
        };
        let Some(point) = root.point else {
            return Ok(out);
        };
        let fragments = other.split_at(&point, crate::source_boundary_fragment::Role::Boundary)?;
        let ranges = crate::source_seam_tangency::ranges(edge);
        let mut halves = Vec::new();
        for piece in 0..2 {
            if out.exact_work == max_work {
                out.reason = "profile-cut-neighbor-work-limit";
                return Ok(out);
            }
            let checked = crate::source_mapped_edge::qualify(
                edge.world(),
                [&self.selected[piece], &fragments[1 - piece]],
                [ranges[side], ranges[1 - side]],
                [None, None],
                max_work - out.exact_work,
                max_driver - out.driver_cells,
            )?;
            out.exact_work += checked.work_used;
            out.root_checks += checked.root_checks;
            out.driver_cells += checked.driver_cells;
            out.reason = checked.reason;
            let Some(half) = checked.edge else {
                return Ok(out);
            };
            halves.push(half);
        }
        out.partition = Some(NeighborPartition {
            fragments,
            edges: [halves.remove(0), halves.remove(0)],
        });
        out.reason = "profile-cut-neighbor-partition-qualified";
        Ok(out)
    }
    pub fn endpoint(&self) -> &CutEndpoint<'_, '_> {
        self.endpoint
    }
    pub fn point(&self) -> &crate::source_contact_point::SourcePoint {
        &self.point
    }
    pub fn selected_fragments(&self) -> &[crate::source_boundary_fragment::Fragment; 2] {
        &self.selected
    }
    pub fn neighbor_ranges(&self) -> [[[f64; 2]; 2]; 2] {
        self.neighbor_ranges
    }
}
pub struct CutPartitionReport<'c, 'p, 'b> {
    pub partition: Option<CutPartition<'c, 'p, 'b>>,
    pub mapping_cells: usize,
    pub reason: &'static str,
}
impl<'p, 'b> CutEndpoint<'p, 'b> {
    /// Split by an original UV crossing expression, preserving its root on both
    /// directed fragments. No rounded world point or parameter becomes an end.
    pub fn split_source_use<'c>(
        &'c self,
        max_mapping_cells: usize,
    ) -> Result<CutPartitionReport<'c, 'p, 'b>> {
        use nurbs_core::{curve::Curve, interval_eval::Interval as I};
        let shell = self.proposal.body.geometry().shell();
        let edge = &shell.edges()[self.edge];
        let addresses = &shell.uses()[self.edge];
        let side = usize::from(addresses[0].face != self.face);
        let source = &edge.uses()[side];
        let other = &edge.uses()[1 - side];
        let profile = self.proposal.profile(self.face).unwrap();
        let section = profile.section(self.axis, self.parameter)?;
        let [lo, hi] = section.range();
        let width = hi - lo;
        let ends = [lo - width, hi + width];
        if !ends.iter().all(|x| x.is_finite()) || ends[0] >= lo || ends[1] <= hi {
            return Err(Error::new(
                "BREP_PROFILE_CUT",
                "Bound the original UV cut line",
            ));
        }
        let contact = Curve::from_polyline(
            ends.map(|free| {
                let mut p = vec![self.parameter; 2];
                p[1 - self.axis] = free;
                p
            })
            .to_vec(),
        )?;
        let t = crate::source_seam_tangency::inverse(
            edge,
            side,
            I::new(self.canonical[0], self.canonical[1])?,
        )?;
        let contact_t = I::point(section.range()[usize::from(self.high)])
            .sub(I::point(ends[0]))?
            .div(I::point(ends[1]).sub(I::point(ends[0]))?)?;
        let d = source.curve().domain();
        let margin = (d[1] - d[0]) * 0.05;
        let selector = [
            [(t.lo - margin).max(d[0]), (t.hi + margin).min(d[1])],
            [(contact_t.lo - 0.05).max(0.), (contact_t.hi + 0.05).min(1.)],
        ];
        let report = crate::source_contact_point::qualify(
            source.surface(),
            source.curve(),
            &contact,
            selector,
            max_mapping_cells,
        )?;
        let mut out = CutPartitionReport {
            partition: None,
            mapping_cells: report.mapping_cells,
            reason: report.reason,
        };
        let Some(point) = report.point else {
            return Ok(out);
        };
        let selected = source.split_at(&point, crate::source_boundary_fragment::Role::Boundary)?;
        let root = point.root_parameters()[0];
        let canonical = crate::source_seam_tangency::map(edge, side, I::new(root[0], root[1])?)?
            .intersect(0., 1.)?;
        let transported = crate::source_seam_tangency::inverse(edge, 1 - side, canonical)?;
        let bounds = other.parameter_bounds();
        let cut = [transported.lo, transported.hi];
        if cut[0] <= bounds[0][1].min(bounds[1][1]) || cut[1] >= bounds[0][0].max(bounds[1][0]) {
            out.reason = "profile-cut-neighbor-order-unproven";
            return Ok(out);
        }
        out.partition = Some(CutPartition {
            endpoint: self,
            point,
            selected,
            neighbor_ranges: [[bounds[0], cut], [cut, bounds[1]]],
        });
        out.reason = "profile-cut-source-partition-qualified";
        Ok(out)
    }
    pub fn proposal(&self) -> &Proposal<'_> {
        self.proposal
    }
    pub fn faces(&self) -> [usize; 2] {
        [self.face, self.neighbor]
    }
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn canonical_bounds(&self) -> [f64; 2] {
        self.canonical
    }
    pub fn neighbor_uv_bounds(&self) -> [[f64; 2]; 2] {
        self.neighbor_uv
    }
    pub fn world_bounds(&self) -> [[f64; 2]; 3] {
        self.world
    }
    pub fn definition(&self) -> value_codec::Value {
        value_codec::json!({"version":1,"kind":"owned-profile-section-end","face":self.face,
            "edge":self.edge,"axis":self.axis,"parameter":self.parameter,"highFreeEnd":self.high})
    }
}
pub struct CutEndpointReport<'p, 'b> {
    pub endpoint: Option<CutEndpoint<'p, 'b>>,
    pub spans: usize,
    pub reason: &'static str,
}
impl<'a> Proposal<'a> {
    /// Bind a new section end to both retained original edge uses. The selected
    /// use must be affine on a preserved natural boundary; its neighbor may
    /// have a nonlinear pcurve. No world-distance test grants ownership.
    pub fn cut_endpoint<'p>(
        &'p self,
        face: usize,
        axis: usize,
        parameter: f64,
        high: bool,
        index: usize,
        max_spans: usize,
    ) -> Result<CutEndpointReport<'p, 'a>> {
        use nurbs_core::interval_eval::Interval as I;
        let profile = self
            .profile(face)
            .ok_or_else(|| Error::new("BREP_PROFILE_CUT", "Choose a proposed face"))?;
        let section = profile.section(axis, parameter)?;
        let shell = self.body.geometry().shell();
        let addresses = shell
            .uses()
            .get(index)
            .ok_or_else(|| Error::new("BREP_PROFILE_CUT", "Choose an owned edge"))?;
        if max_spans > 100000 {
            return Err(Error::new("BREP_PROFILE_CUT", "Bound cut parameter work"));
        }
        let mut out = CutEndpointReport {
            endpoint: None,
            spans: 0,
            reason: "profile-cut-carrier-unproven",
        };
        let Some(side) = addresses.iter().position(|u| u.face == face) else {
            return Ok(out);
        };
        let edge = &shell.edges()[index];
        let regions = shell.regions().unwrap();
        if (0..2).any(|i| {
            !same(
                edge.uses()[i].surface(),
                regions[addresses[i].face].loops()[0][0].surface(),
            )
        }) {
            return Ok(out);
        }
        let use_ = &edge.uses()[side];
        let c = use_.curve();
        let boundary = section.range()[usize::from(high)];
        if !edge.maps_complete_canonical_source()
            || !same(use_.surface(), profile.source())
            || c.periodic
            || c.degree != 1
            || c.control_points.len() != 2
            || c.weights[0] != c.weights[1]
            || c.control_points
                .iter()
                .any(|p| p.len() != 2 || p[1 - axis] != boundary)
            || c.control_points[0][axis] == c.control_points[1][axis]
        {
            return Ok(out);
        }
        let ends = [c.control_points[0][axis], c.control_points[1][axis]];
        if parameter < ends[0].min(ends[1]) || parameter > ends[0].max(ends[1]) {
            return Ok(out);
        }
        let normalized = I::point(parameter)
            .sub(I::point(ends[0]))?
            .div_signed(I::point(ends[1]).sub(I::point(ends[0]))?)?
            .intersect(0., 1.)?;
        let d = c.domain();
        let t = I::point(d[0])
            .add(normalized.mul(I::point(d[1]).sub(I::point(d[0]))?)?)?
            .intersect(d[0], d[1])?;
        let canonical = crate::source_seam_tangency::map(edge, side, t)?.intersect(0., 1.)?;
        for current in 0..2 {
            let t = crate::source_seam_tangency::inverse(edge, current, canonical)?;
            let ends = edge.uses()[current].parameter_bounds();
            if t.lo < ends[0][1].min(ends[1][1]) || t.hi > ends[0][0].max(ends[1][0]) {
                out.reason = "profile-cut-retained-parameter-unproven";
                return Ok(out);
            }
        }
        let neighbor = 1 - side;
        let nc = edge.uses()[neighbor].curve();
        let t = crate::source_seam_tangency::inverse(edge, neighbor, canonical)?;
        let world = edge.world();
        let d = world.domain();
        let w = I::point(d[0])
            .add(canonical.mul(I::point(d[1]).sub(I::point(d[0]))?)?)?
            .intersect(d[0], d[1])?;
        for (curve, t) in [(nc, t), (world, w)] {
            let spent = (curve.degree..curve.control_points.len())
                .filter(|&i| {
                    curve.knots[i] < curve.knots[i + 1]
                        && curve.knots[i] <= t.hi
                        && curve.knots[i + 1] >= t.lo
                })
                .count();
            if spent > max_spans - out.spans {
                out.reason = "profile-cut-work-limit";
                return Ok(out);
            }
            out.spans += spent;
        }
        let uv = nurbs_core::interval_eval::evaluate_interval(nc, t)?;
        let point = nurbs_core::interval_eval::evaluate_interval(world, w)?;
        out.endpoint = Some(CutEndpoint {
            proposal: self,
            face,
            neighbor: addresses[neighbor].face,
            edge: index,
            axis,
            parameter,
            high,
            canonical: [canonical.lo, canonical.hi],
            neighbor_uv: [[uv[0].lo, uv[0].hi], [uv[1].lo, uv[1].hi]],
            world: std::array::from_fn(|k| [point[k].lo, point[k].hi]),
        });
        out.reason = "profile-cut-endpoint-owned";
        Ok(out)
    }
    /// Derive both UV paths from private original carrier ownership and compare
    /// each incident face's actual original or proposed weight representation.
    pub fn qualify_seam<'p>(
        &'p self,
        face: usize,
        neighbor: usize,
        max_edges: usize,
        threshold: f64,
        cells: usize,
        spans: usize,
    ) -> Result<SeamReport<'p, 'a>> {
        self.qualify_selected_seam(face, neighbor, None, max_edges, threshold, cells, spans)
    }
    /// Qualify a single owned edge; never reuse another edge between its faces.
    pub fn qualify_edge<'p>(
        &'p self,
        edge: usize,
        max_edges: usize,
        threshold: f64,
        cells: usize,
        spans: usize,
    ) -> Result<SeamReport<'p, 'a>> {
        let uses = self
            .body
            .geometry()
            .shell()
            .uses()
            .get(edge)
            .ok_or_else(|| Error::new("BREP_PROFILE_SEAM", "Choose an owned edge"))?;
        let mut faces = [uses[0].face, uses[1].face];
        if self.profile(faces[0]).is_none() {
            faces.reverse();
        }
        self.qualify_selected_seam(
            faces[0],
            faces[1],
            Some(edge),
            max_edges,
            threshold,
            cells,
            spans,
        )
    }
    fn qualify_selected_seam<'p>(
        &'p self,
        face: usize,
        neighbor: usize,
        edge: Option<usize>,
        max_edges: usize,
        threshold: f64,
        cells: usize,
        spans: usize,
    ) -> Result<SeamReport<'p, 'a>> {
        let mut out = SeamReport {
            seam: None,
            angular: None,
            reason: "profile-seam-carrier-unproven",
        };
        let profile = self
            .profile(face)
            .ok_or_else(|| Error::new("BREP_PROFILE_SEAM", "Choose a proposed face"))?;
        let map = match edge {
            Some(edge) => crate::source_shared_chord_map::prepare_edge(
                self.body,
                [face, neighbor],
                edge,
                max_edges,
            )?,
            None => {
                crate::source_shared_chord_map::prepare(self.body, [face, neighbor], max_edges)?
            }
        };
        let Some(map) = map else {
            if let Some(index) = edge.filter(|index| *index < max_edges) {
                let angular =
                    self.inspect_curved_edge(face, neighbor, index, threshold, cells, spans)?;
                out.reason = angular.reason;
                if angular.aligned == Some(true) {
                    out.seam = Some(Seam {
                        proposal: self,
                        faces: [face, neighbor],
                        edge: index,
                        threshold,
                        upper: angular.upper_sine_squared.unwrap(),
                    });
                }
                out.angular = Some(angular);
            }
            return Ok(out);
        };
        let axes = map.fixed_axes();
        let fixed = map.fixed_values();
        let free = map.free_endpoints();
        let lines = std::array::from_fn(|side| {
            std::array::from_fn(|end| {
                std::array::from_fn(|axis| {
                    if axis == axes[side] {
                        fixed[side]
                    } else {
                        free[side][end]
                    }
                })
            })
        });
        if !(1..=100000).contains(&cells) || !(1..=100000).contains(&spans) {
            return Err(Error::new(
                "BREP_PROFILE_SEAM",
                "Bound endpoint and path work",
            ));
        }
        let mut pre = nurbs_core::normal_alignment::ProfilePathReport {
            aligned: None,
            cells: 0,
            spans: 0,
            accepted_cells: 0,
            upper_sine_squared: None,
            pending: vec![[0., 1.]],
            reason: "profile-path-work-limit",
        };
        let mut endpoint_upper = 0_f64;
        for end in 0..2 {
            if pre.cells == cells || pre.spans == spans {
                out.reason = pre.reason;
                out.angular = Some(pre);
                return Ok(out);
            }
            let rectangles = std::array::from_fn(|side| lines[side][end].map(|x| [x, x]));
            pre.cells += 1;
            let r = match self.profile(neighbor) {
                Some(other) => profile.inspect_profile_normals(
                    other,
                    rectangles,
                    threshold,
                    spans - pre.spans,
                )?,
                None => profile.inspect_original_normals(
                    map.surfaces()[1],
                    rectangles,
                    threshold,
                    spans - pre.spans,
                )?,
            };
            pre.spans += r.spans;
            if r.aligned != Some(true) {
                pre.pending.push([end as f64; 2]);
                pre.reason = if r.reason == "span-limit" {
                    "profile-path-work-limit"
                } else if r.aligned == Some(false) {
                    pre.aligned = Some(false);
                    "profile-path-angular-mismatch"
                } else {
                    "profile-seam-endpoint-normal-unresolved"
                };
                out.reason = pre.reason;
                out.angular = Some(pre);
                return Ok(out);
            }
            endpoint_upper = endpoint_upper.max(r.sine_squared_interval.unwrap()[1]);
            pre.accepted_cells += 1;
        }
        if pre.cells == cells || pre.spans == spans {
            out.reason = pre.reason;
            out.angular = Some(pre);
            return Ok(out);
        }
        let mut angular = match self.profile(neighbor) {
            Some(other) => profile.inspect_profile_path(
                other,
                lines,
                threshold,
                cells - pre.cells,
                spans - pre.spans,
            )?,
            None => profile.inspect_original_path(
                map.surfaces()[1],
                lines,
                threshold,
                cells - pre.cells,
                spans - pre.spans,
            )?,
        };
        angular.cells += pre.cells;
        angular.spans += pre.spans;
        angular.accepted_cells += pre.accepted_cells;
        if let Some(upper) = angular.upper_sine_squared.as_mut() {
            *upper = upper.max(endpoint_upper);
        }

        out.reason = angular.reason;
        if angular.aligned == Some(true) {
            out.seam = Some(Seam {
                proposal: self,
                faces: [face, neighbor],
                edge: map.edge(),
                threshold,
                upper: angular.upper_sine_squared.unwrap(),
            });
        }
        out.angular = Some(angular);
        Ok(out)
    }
    /// Original nonlinear pcurves remain source-owned; inverse canonical maps
    /// and complete curve-span enclosures replace manually supplied UV lines.
    fn inspect_curved_edge(
        &self,
        face: usize,
        neighbor: usize,
        index: usize,
        threshold: f64,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<nurbs_core::normal_alignment::ProfilePathReport> {
        use nurbs_core::{interval_eval::Interval as I, normal_alignment::ProfilePathReport};
        if !(1..=100000).contains(&max_cells)
            || !(1..=100000).contains(&max_spans)
            || !threshold.is_finite()
            || !(0. ..1.).contains(&threshold)
        {
            return Err(Error::new(
                "BREP_PROFILE_SEAM",
                "Bound original pcurve and normal work",
            ));
        }
        let shell = self.body.geometry().shell();
        let edge = &shell.edges()[index];
        let addresses = &shell.uses()[index];
        let order = if addresses[0].face == face {
            [0, 1]
        } else {
            [1, 0]
        };
        let mut out = ProfilePathReport {
            aligned: None,
            cells: 0,
            spans: 0,
            accepted_cells: 0,
            upper_sine_squared: None,
            pending: vec![[0., 1.]],
            reason: "profile-seam-carrier-unproven",
        };
        let regions = shell.regions().unwrap();
        if !edge.maps_complete_canonical_source()
            || [face, neighbor].iter().enumerate().any(|(side, &f)| {
                !same(
                    edge.uses()[order[side]].surface(),
                    regions[f].loops()[0][0].surface(),
                ) || edge.uses()[order[side]].curve().periodic
            })
        {
            return Ok(out);
        }
        let profile = self.profile(face).unwrap();
        // Singular literal source endpoints are explicitly inspected before
        // repeatedly subdividing an interval that contains a zero normal.
        out.pending.push([1., 1.]);
        out.pending.push([0., 0.]);
        out.reason = "profile-path-work-limit";
        let mut upper = 0_f64;
        while let Some(q) = out.pending.pop() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(q);
                return Ok(out);
            }
            out.cells += 1;
            let mut rectangles = [[[0.; 2]; 2]; 2];
            for side in 0..2 {
                let use_ = &edge.uses()[order[side]];
                let c = use_.curve();
                let t =
                    crate::source_seam_tangency::inverse(edge, order[side], I::new(q[0], q[1])?)?;
                let spent = (c.degree..c.control_points.len())
                    .filter(|&i| {
                        c.knots[i] < c.knots[i + 1] && c.knots[i] <= t.hi && c.knots[i + 1] >= t.lo
                    })
                    .count();
                if spent > max_spans - out.spans {
                    out.pending.push(q);
                    return Ok(out);
                }
                out.spans += spent;
                let uv = nurbs_core::interval_eval::evaluate_interval(c, t)?;
                let s = use_.surface();
                let natural = [
                    [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                    [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
                ];
                for axis in 0..2 {
                    let r = uv[axis].intersect(natural[axis][0], natural[axis][1])?;
                    rectangles[side][axis] = [r.lo, r.hi];
                }
            }
            if out.spans == max_spans {
                out.pending.push(q);
                return Ok(out);
            }
            let r = match self.profile(neighbor) {
                Some(other) => profile.inspect_profile_normals(
                    other,
                    rectangles,
                    threshold,
                    max_spans - out.spans,
                )?,
                None => profile.inspect_original_normals(
                    edge.uses()[order[1]].surface(),
                    rectangles,
                    threshold,
                    max_spans - out.spans,
                )?,
            };
            out.spans += r.spans;
            if r.aligned == Some(true) {
                upper = upper.max(r.sine_squared_interval.unwrap()[1]);
                out.accepted_cells += 1;
                continue;
            }
            if r.aligned == Some(false) {
                out.pending.push(q);
                out.aligned = Some(false);
                out.reason = "profile-path-angular-mismatch";
                return Ok(out);
            }
            if q[0] == q[1] {
                out.pending.push(q);
                out.reason = "profile-seam-endpoint-normal-unresolved";
                return Ok(out);
            }
            if r.reason == "span-limit" {
                out.pending.push(q);
                return Ok(out);
            }
            let mid = q[0] + (q[1] - q[0]) * 0.5;
            if mid <= q[0] || mid >= q[1] {
                out.pending.push(q);
                out.reason = "profile-path-resolution-limit";
                return Ok(out);
            }
            out.pending.push([mid, q[1]]);
            out.pending.push([q[0], mid]);
        }
        out.aligned = Some(true);
        out.upper_sine_squared = Some(upper);
        out.reason = "profile-path-tangent-planes-qualified";
        Ok(out)
    }
    pub fn original_body(&self) -> &'a Body {
        self.body
    }
    pub fn profile(&self, face: usize) -> Option<&WeightProfile<'a>> {
        self.faces
            .iter()
            .find(|f| f.spec.face == face)
            .map(|f| &f.profile)
    }
    pub fn specs(&self) -> Vec<FaceSpec> {
        self.faces.iter().map(|f| f.spec).collect()
    }
    pub fn axis_deviation_upper(&self) -> [f64; 3] {
        self.axis_deviation_upper
    }
    /// Persist source and exact factor recipes, never flattened target weights or
    /// claimed geometry certificates. Restoration must rerun all admission gates.
    pub fn definition(&self) -> Result<value_codec::Value> {
        Ok(
            value_codec::json!({"version":1,"kind":"boundary-preserving-weight-profiles",
            "originalBody":self.body.definition()?,"positionTolerance":self.position_tolerance,
            "profiles":self.faces.iter().map(|f|value_codec::json!({"face":f.spec.face,"anchor":f.spec.anchor})).collect::<Vec<_>>()}),
        )
    }
}
pub struct Report<'a> {
    pub proposal: Option<Proposal<'a>>,
    pub controls: usize,
    pub fragments: usize,
    pub reason: &'static str,
}
fn same(a: &Surface, b: &Surface) -> bool {
    a.degree_u == b.degree_u
        && a.degree_v == b.degree_v
        && a.periodic_u == b.periodic_u
        && a.periodic_v == b.periodic_v
        && a.knots_u == b.knots_u
        && a.knots_v == b.knots_v
        && a.control_points == b.control_points
        && a.weights == b.weights
}
pub fn prepare<'a>(body: &'a Body, specs: &[FaceSpec], limits: Limits) -> Result<Report<'a>> {
    let regions = body
        .geometry()
        .shell()
        .regions()
        .ok_or_else(|| Error::new("BREP_PROFILE_BODY", "Require original retained regions"))?;
    if specs.is_empty()
        || specs.len() > regions.len()
        || limits.controls > 1_000_000
        || limits.fragments > 100000
        || !limits.position_tolerance.is_finite()
        || limits.position_tolerance < 0.
        || specs.iter().any(|f| f.face >= regions.len())
        || specs
            .iter()
            .enumerate()
            .any(|(i, f)| specs[..i].iter().any(|a| a.face == f.face))
    {
        return Err(Error::new(
            "BREP_PROFILE_BODY",
            "Choose distinct owned faces and finite bounded proposal limits",
        ));
    }
    let mut out = Report {
        proposal: None,
        controls: 0,
        fragments: 0,
        reason: "profile-body-work-limit",
    };
    let mut faces = Vec::new();
    let mut deviation = [0_f64; 3];
    for spec in specs {
        let region = &regions[spec.face];
        let source = region.loops()[0][0].surface();
        let (profile, used) = nurbs_core::normal_alignment::qualify_weight_profile(
            source,
            spec.anchor,
            limits.controls - out.controls,
        )?;
        out.controls += used;
        let Some(profile) = profile else {
            return Ok(out);
        };
        if !profile.within_position_tolerance(limits.position_tolerance)? {
            out.reason = "profile-body-position-tolerance-unproven";
            return Ok(out);
        }
        // Check all natural boundaries, including collapsed rails absent from
        // the retained wire. This preserves original pole and edge positions.
        for axis in 0..2 {
            for high in [false, true] {
                let (boundary, used) =
                    profile.preserved_boundary(axis, high, limits.controls - out.controls)?;
                out.controls += used;
                if boundary.is_none() {
                    out.reason = "profile-body-boundary-identity-unproven";
                    return Ok(out);
                }
            }
        }
        let domains = [
            [
                source.knots_u[source.degree_u],
                source.knots_u[source.control_points.len()],
            ],
            [
                source.knots_v[source.degree_v],
                source.knots_v[source.control_points[0].len()],
            ],
        ];
        for wire in region.loops() {
            for fragment in wire {
                if out.fragments == limits.fragments {
                    return Ok(out);
                }
                out.fragments += 1;
                if !same(fragment.surface(), source) || fragment.curve().periodic {
                    out.reason = "profile-body-contour-source-unproven";
                    return Ok(out);
                }
                let mut fixed = [true; 4];
                for point in &fragment.curve().control_points {
                    if out.controls == limits.controls {
                        return Ok(out);
                    }
                    out.controls += 1;
                    if point.len() != 2
                        || (0..2).any(|a| point[a] < domains[a][0] || point[a] > domains[a][1])
                    {
                        out.reason = "profile-body-contour-boundary-unproven";
                        return Ok(out);
                    }
                    for axis in 0..2 {
                        for high in 0..2 {
                            fixed[2 * axis + high] &= point[axis] == domains[axis][high];
                        }
                    }
                }
                if !fixed.iter().any(|x| *x) {
                    out.reason = "profile-body-interior-trim-unproven";
                    return Ok(out);
                }
            }
        }
        for (k, error) in profile.axis_deviation_upper().into_iter().enumerate() {
            deviation[k] = deviation[k].max(error);
        }
        faces.push(Face {
            spec: *spec,
            profile,
        });
    }
    out.reason = "profile-body-boundaries-preserved";
    out.proposal = Some(Proposal {
        body,
        faces,
        axis_deviation_upper: deviation,
        position_tolerance: limits.position_tolerance,
    });
    Ok(out)
}
