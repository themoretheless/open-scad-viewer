//! source profile body operations: cut_endpoint.
use super::*;
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
        if !same(use_.surface(), profile.source())
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
        if !edge.maps_complete_canonical_source()
            && !edge.covers_retained_canonical_interval([canonical.lo, canonical.hi])?
        {
            out.reason = "profile-cut-retained-parameter-unproven";
            return Ok(out);
        }
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
pub(super) fn qualify_selected_seam<'p>(
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
/// Bind angular qualification to two owned section roots on one carrier.
    /// The interval covers their exact transported enclosures; it is not made
    /// from independently rounded canonical endpoint scalars.
    pub fn qualify_root_ended_seam<'p>(
        &'p self,
        parts: [&CutPartition<'_, 'p, 'a>; 2],
        threshold: f64,
        cells: usize,
        spans: usize,
    ) -> Result<RootEndedSeamReport<'p, 'a>> {
        if spans > 100000
            || !(1..=100000).contains(&cells)
            || !threshold.is_finite()
            || !(0. ..1.).contains(&threshold)
        {
            return Err(Error::new(
                "BREP_PROFILE_SEAM",
                "Bound root-ended seam angular and carrier work",
            ));
        }
        let mut out = RootEndedSeamReport {
            seam: None,
            carrier_spans: 0,
            angular: None,
            reason: "profile-root-seam-ownership-unproven",
        };
        let ends = [parts[0].endpoint, parts[1].endpoint];
        if ends.iter().any(|e| !std::ptr::eq(e.proposal, self))
            || ends[0].edge != ends[1].edge
            || ends[0].face != ends[1].face
            || ends[0].neighbor != ends[1].neighbor
            || ends[0].axis != ends[1].axis
            || ends[0].high != ends[1].high
            || ends[0].parameter == ends[1].parameter
        {
            return Ok(out);
        }
        let shell = self.body.geometry().shell();
        let edge = &shell.edges()[ends[0].edge];
        let side = usize::from(shell.uses()[ends[0].edge][0].face != ends[0].face);
        let mut points = Vec::new();
        for part in parts {
            if out.carrier_spans == spans {
                out.reason = "profile-root-seam-carrier-work-limit";
                return Ok(out);
            }
            let checked = crate::source_carrier_point::qualify(
                edge,
                &part.point,
                side,
                spans - out.carrier_spans,
            )?;
            out.carrier_spans += checked.spans;
            out.reason = checked.reason;
            let Some(point) = checked.point else {
                return Ok(out);
            };
            points.push(point);
        }
        let ranges = [points[0].canonical_bounds(), points[1].canonical_bounds()];
        if !(ranges[0][1] < ranges[1][0] || ranges[1][1] < ranges[0][0]) {
            out.reason = "profile-root-seam-order-unproven";
            return Ok(out);
        }
        let canonical = [
            ranges[0][0].min(ranges[1][0]),
            ranges[0][1].max(ranges[1][1]),
        ];
        if out.carrier_spans == spans {
            out.reason = "profile-root-seam-angular-work-limit";
            return Ok(out);
        }
        let angular = self.qualify_edge_interval(
            ends[0].edge,
            canonical,
            threshold,
            cells,
            spans - out.carrier_spans,
        )?;
        out.reason = angular.angular.reason;
        if let Some(restricted) = angular.seam {
            out.seam = Some(RootEndedSeam {
                restricted,
                points: [points.remove(0), points.remove(0)],
                face: ends[0].face,
                axis: ends[0].axis,
                parameters: [ends[0].parameter, ends[1].parameter],
                high: ends[0].high,
            });
            out.reason = "profile-root-ended-seam-qualified";
        }
        out.angular = Some(angular.angular);
        Ok(out)
    }
/// Qualify only the requested interval of an owned original carrier.
    /// UVs are freshly enclosed from both original pcurves and canonical maps.
    pub fn qualify_edge_interval<'p>(
        &'p self,
        edge: usize,
        canonical: [f64; 2],
        threshold: f64,
        cells: usize,
        spans: usize,
    ) -> Result<RestrictedSeamReport<'p, 'a>> {
        let uses = self
            .body
            .geometry()
            .shell()
            .uses()
            .get(edge)
            .ok_or_else(|| Error::new("BREP_PROFILE_SEAM", "Choose an owned interval edge"))?;
        let mut faces = [uses[0].face, uses[1].face];
        if self.profile(faces[0]).is_none() {
            faces.reverse();
        }
        if self.profile(faces[0]).is_none() {
            return Err(Error::new(
                "BREP_PROFILE_SEAM",
                "Require an owned corrected face",
            ));
        }
        let angular = self.inspect_curved_edge_interval(
            faces[0], faces[1], edge, threshold, cells, spans, canonical,
        )?;
        let seam = if angular.aligned == Some(true) {
            Some(RestrictedSeam {
                proposal: self,
                faces,
                edge,
                canonical,
                threshold,
                upper: angular.upper_sine_squared.unwrap(),
            })
        } else {
            None
        };
        Ok(RestrictedSeamReport { seam, angular })
    }
/// Original nonlinear pcurves remain source-owned; inverse canonical maps
    /// and complete curve-span enclosures replace manually supplied UV lines.
    pub(super) fn inspect_curved_edge(
        &self,
        face: usize,
        neighbor: usize,
        index: usize,
        threshold: f64,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<nurbs_core::normal_alignment::ProfilePathReport> {
        self.inspect_curved_edge_interval(
            face,
            neighbor,
            index,
            threshold,
            max_cells,
            max_spans,
            [0., 1.],
        )
    }
pub(super) fn inspect_curved_edge_interval(
        &self,
        face: usize,
        neighbor: usize,
        index: usize,
        threshold: f64,
        max_cells: usize,
        max_spans: usize,
        canonical: [f64; 2],
    ) -> Result<nurbs_core::normal_alignment::ProfilePathReport> {
        self.inspect_curved_edge_interval_policy(
            face, neighbor, index, threshold, max_cells, max_spans, canonical, None, false,
        )
    }
pub(crate) fn inspect_original_neighbor_interval(
        &self,
        face: usize,
        neighbor: usize,
        index: usize,
        threshold: f64,
        cells: usize,
        spans: usize,
        canonical: [f64; 2],
    ) -> Result<nurbs_core::normal_alignment::ProfilePathReport> {
        let uses = self
            .body
            .geometry()
            .shell()
            .uses()
            .get(index)
            .ok_or_else(|| Error::new("BREP_PROFILE_SEAM", "Choose an owned edge"))?;
        if face == neighbor
            || !uses.iter().any(|u| u.face == face)
            || !uses.iter().any(|u| u.face == neighbor)
        {
            return Err(Error::new(
                "BREP_PROFILE_SEAM",
                "Require the actual incident neighbor",
            ));
        }
        self.inspect_curved_edge_interval_policy(
            face, neighbor, index, threshold, cells, spans, canonical, None, true,
        )
    }
/// Diagnostic full-edge limit angles on the owned nonlinear pcurve map.
    /// This does not issue a regular seam or admit a collapsed vertex.
    pub fn inspect_edge_pole_limit_angles(
        &self,
        face: usize,
        index: usize,
        high: bool,
        threshold: f64,
        cells: usize,
        spans: usize,
    ) -> Result<nurbs_core::normal_alignment::ProfilePathReport> {
        let uses = self
            .body
            .geometry()
            .shell()
            .uses()
            .get(index)
            .ok_or_else(|| Error::new("BREP_PROFILE_SEAM", "Choose an owned edge"))?;
        let side = uses
            .iter()
            .position(|u| u.face == face)
            .ok_or_else(|| Error::new("BREP_PROFILE_SEAM", "Choose an incident proposed face"))?;
        if self.profile(face).is_none() || self.profile(uses[1 - side].face).is_some() {
            return Err(Error::new(
                "BREP_PROFILE_SEAM",
                "Require proposed face and unchanged neighbor",
            ));
        }
        self.inspect_curved_edge_interval_policy(
            face,
            uses[1 - side].face,
            index,
            threshold,
            cells,
            spans,
            [0., 1.],
            Some(high),
            false,
        )
    }
}
