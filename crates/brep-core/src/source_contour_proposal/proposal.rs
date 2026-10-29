use super::*;

pub struct Proposal {
    pub search: SourcePointSearch,
    pub candidate_loops: Option<Vec<Vec<Fragment>>>,
    pub candidate_source_loops: Option<Vec<usize>>,
    pub source_joins_proven: bool,
    pub reason: &'static str,
}
/// Find both contact parameters from original boundaries, without supplied cuts.
/// Endpoint edges select the cyclic removed arc; each must have one mapped root.
/// The result is a contour candidate, never a qualified replacement face.
pub fn propose(
    context: &ToleranceContext,
    surface: &Surface,
    wires: &[Vec<Boundary>],
    contact: &Curve,
    loop_index: usize,
    start_edge: usize,
    end_edge: usize,
    tolerance_uv: f64,
    limits: Limits,
    max_cells: usize,
    target_width: [f64; 2],
    max_point_checks: usize,
    max_mapping_cells: usize,
) -> Result<Proposal> {
    if loop_index >= wires.len()
        || start_edge == end_edge
        || start_edge >= wires[loop_index].len()
        || end_edge >= wires[loop_index].len()
    {
        return Err(Error::new(
            "BREP_SOURCE_CONTOUR",
            "Choose two distinct original endpoint edges on one contour",
        ));
    }
    let search = trimmed_face_recipe::locate_source_contact_points(
        context,
        surface,
        wires,
        contact,
        tolerance_uv,
        limits,
        max_cells,
        target_width,
        max_point_checks,
        max_mapping_cells,
    )?;
    let mut out = Proposal {
        search,
        candidate_loops: None,
        candidate_source_loops: None,
        source_joins_proven: false,
        reason: "source-crossing-search-unqualified",
    };
    if !out.search.complete {
        return Ok(out);
    }
    let choose = |edge: usize| {
        out.search
            .points
            .iter()
            .filter(|p| p.loop_index == loop_index && p.boundary_index == edge)
            .filter_map(|p| p.report.as_ref().and_then(|r| r.point.as_ref()))
            .collect::<Vec<_>>()
    };
    let start = choose(start_edge);
    let end = choose(end_edge);
    if start.len() != 1 || end.len() != 1 {
        out.reason = "source-endpoint-root-ambiguous";
        return Ok(out);
    }
    let p0 = start[0].clone();
    let p1 = end[0].clone();
    let original = &wires[loop_index];
    let head = &original[start_edge].pcurve;
    let tail = &original[end_edge].pcurve;
    let contact_fragment = Fragment::new(
        surface,
        contact,
        Endpoint::Crossing {
            point: p0.clone(),
            role: Role::Contact,
        },
        Endpoint::Crossing {
            point: p1.clone(),
            role: Role::Contact,
        },
    )?;
    let head_fragment = Fragment::new(
        surface,
        head,
        Endpoint::Parameter(head.domain()[0]),
        Endpoint::Crossing {
            point: p0,
            role: Role::Boundary,
        },
    )?;
    let tail_fragment = Fragment::new(
        surface,
        tail,
        Endpoint::Crossing {
            point: p1,
            role: Role::Boundary,
        },
        Endpoint::Parameter(tail.domain()[1]),
    )?;
    let full = |b: &Boundary| {
        Fragment::new(
            surface,
            &b.pcurve,
            Endpoint::Parameter(b.pcurve.domain()[0]),
            Endpoint::Parameter(b.pcurve.domain()[1]),
        )
    };
    let mut loops = wires
        .iter()
        .map(|wire| wire.iter().map(full).collect::<Result<Vec<_>>>())
        .collect::<Result<Vec<_>>>()?;
    let mut replacement = vec![contact_fragment, tail_fragment];
    let mut i = (end_edge + 1) % original.len();
    while i != start_edge {
        replacement.push(full(&original[i])?);
        i = (i + 1) % original.len();
    }
    replacement.push(head_fragment);
    loops[loop_index] = replacement;
    let closed = loops.iter().all(|wire| {
        !wire.is_empty() && (0..wire.len()).all(|i| wire[i].joins(&wire[(i + 1) % wire.len()]))
    });
    if !closed {
        out.reason = "source-contour-joins-unproven";
        return Ok(out);
    }
    out.candidate_loops = Some(loops);
    out.candidate_source_loops = Some((0..wires.len()).collect());
    out.source_joins_proven = true;
    out.reason = "source-contour-candidate-closed-region-unverified";
    Ok(out)
}
pub struct InteriorContact {
    pub proposal: Proposal,
    pub driver: Option<nurbs_core::curve_axis_driver::Report>,
    pub membership: Option<nurbs_core::trim_domain::Classification>,
    pub only_owned_crossings_proven: bool,
    pub contact_simple_proven: bool,
    pub contact_inside_proven: bool,
    /// Hole placement and replacement winding still need independent proof.
    pub region_subset_proven: bool,
    pub reason: &'static str,
}
/// Freshly prove a simple contact arc remains inside the original material
/// region, with its only boundary contacts the two selected endpoint roots.
/// This does not certify replacement winding or placement of retained holes.
pub fn qualify_interior_contact(
    context: &ToleranceContext,
    surface: &Surface,
    wires: &[Vec<Boundary>],
    contact: &Curve,
    loop_index: usize,
    start_edge: usize,
    end_edge: usize,
    tolerance_uv: f64,
    limits: Limits,
    max_cells: usize,
    target_width: [f64; 2],
    max_point_checks: usize,
    max_mapping_cells: usize,
    driver_axis: usize,
    max_driver_cells: usize,
    max_membership_cells: usize,
) -> Result<InteriorContact> {
    if driver_axis >= 2
        || !(1..=100000).contains(&max_driver_cells)
        || !(1..=100000).contains(&max_membership_cells)
    {
        return Err(Error::new(
            "BREP_SOURCE_CONTACT_INTERIOR",
            "Choose a UV driver axis and bounded proof work",
        ));
    }
    let proposal = propose(
        context,
        surface,
        wires,
        contact,
        loop_index,
        start_edge,
        end_edge,
        tolerance_uv,
        limits,
        max_cells,
        target_width,
        max_point_checks,
        max_mapping_cells,
    )?;
    let mut out = InteriorContact {
        proposal,
        driver: None,
        membership: None,
        only_owned_crossings_proven: false,
        contact_simple_proven: false,
        contact_inside_proven: false,
        region_subset_proven: false,
        reason: "source-contour-unqualified",
    };
    if !out.proposal.source_joins_proven {
        return Ok(out);
    }
    // Complete original span-product search also covers all unaffected holes.
    // Any additional contact, including one on the removed arc, prevents this
    // no-crossing authority; unresolved/tangent cells already stop the proposal.
    if out.proposal.search.points.len() != 2 {
        out.reason = "source-contact-has-additional-boundary-crossings";
        return Ok(out);
    }
    out.only_owned_crossings_proven = true;
    let driver = nurbs_core::curve_axis_driver::certify(contact, driver_axis, max_driver_cells)?;
    out.contact_simple_proven = driver.monotonic_proven;
    out.driver = Some(driver);
    if !out.contact_simple_proven {
        out.reason = "source-contact-simplicity-unproven";
        return Ok(out);
    }
    let fragment = &out.proposal.candidate_loops.as_ref().unwrap()[loop_index][0];
    let ends = fragment.parameter_bounds();
    let between = if fragment.reversed() {
        [ends[1][1], ends[0][0]]
    } else {
        [ends[0][1], ends[1][0]]
    };
    let t = between[0] * 0.5 + between[1] * 0.5;
    if !(between[0] < t && t < between[1]) {
        out.reason = "source-contact-interior-station-unrepresentable";
        return Ok(out);
    }
    let image = nurbs_core::interval_eval::evaluate_interval(
        contact,
        nurbs_core::interval_eval::Interval::point(t),
    )?;
    let uv = wires
        .iter()
        .map(|wire| wire.iter().map(|b| b.pcurve.clone()).collect())
        .collect::<Vec<Vec<Curve>>>();
    let domain = nurbs_core::trim_domain::TrimDomain::new(&uv, tolerance_uv)?;
    let membership = domain.classify(
        [[image[0].lo, image[0].hi], [image[1].lo, image[1].hi]],
        max_membership_cells,
    )?;
    out.contact_inside_proven = membership.location == nurbs_core::trim_domain::Location::Inside;
    out.membership = Some(membership);
    out.reason = if out.contact_inside_proven {
        "source-contact-arc-inside-original-region"
    } else {
        "source-contact-interior-membership-unproven"
    };
    // A continuous simple arc cannot leave this original material component
    // without meeting its boundary. Full original boundary search excludes all
    // such meetings except the two owned endpoints; one interior witness fixes
    // the component. This proves arc membership, not replacement-region winding.
    Ok(out)
}

/// Qualified source UV region, not an embedded 3D face or closed solid.
#[derive(Clone)]
pub struct SourceRegion {
    pub(super) chart_winding: i32,
    pub(super) whole_chart: bool,
    pub(super) loops: Vec<Vec<Fragment>>,
    pub(super) source_loop_indices: Vec<usize>,
    pub(super) recipe: Recipe,
}
impl SourceRegion {
    #[cfg(feature = "codec")]
    pub fn definition(&self) -> value_codec::Value {
        self.recipe.definition()
    }
    /// Material orientation relative to the unchanged original UV chart.
    pub fn chart_winding(&self) -> i32 {
        self.chart_winding
    }
    /// Exact original four-line natural rectangle, with no holes or cuts.
    pub fn whole_chart_material(&self) -> bool {
        self.whole_chart
    }
    pub fn world_wires(&self) -> Result<Vec<crate::source_world_wire::Wire>> {
        self.loops
            .iter()
            .map(|edges| crate::source_world_wire::Wire::new(edges))
            .collect()
    }
    pub fn source_loop_indices(&self) -> &[usize] {
        &self.source_loop_indices
    }
    pub fn loops(&self) -> &[Vec<Fragment>] {
        &self.loops
    }
}
pub struct OriginalRegion {
    pub audit: trimmed_face_recipe::Report,
    pub region: Option<SourceRegion>,
    pub reason: &'static str,
}
/// Original UV material region, independently audited before source ownership.
pub fn qualify_original_region(
    context: &ToleranceContext,
    surface: &Surface,
    wires: &[Vec<Boundary>],
    tolerance_uv: f64,
    limits: Limits,
) -> Result<OriginalRegion> {
    let audit = trimmed_face_recipe::assemble(context, surface, wires, tolerance_uv, limits)?;
    let mut out = OriginalRegion {
        audit,
        region: None,
        reason: "original-source-region-unqualified",
    };
    if out.audit.face.is_none() {
        return Ok(out);
    }
    let loops = wires
        .iter()
        .map(|wire| {
            wire.iter()
                .map(|b| {
                    Fragment::new(
                        surface,
                        &b.pcurve,
                        Endpoint::Parameter(b.pcurve.domain()[0]),
                        Endpoint::Parameter(b.pcurve.domain()[1]),
                    )
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    if loops.iter().any(|wire| {
        wire.is_empty() || (0..wire.len()).any(|i| !wire[i].joins(&wire[(i + 1) % wire.len()]))
    }) {
        out.reason = "original-source-region-joins-unproven";
        return Ok(out);
    }
    let corners = [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_v[surface.degree_v],
        ],
        [
            surface.knots_u[surface.control_points.len()],
            surface.knots_v[surface.degree_v],
        ],
        [
            surface.knots_u[surface.control_points.len()],
            surface.knots_v[surface.control_points[0].len()],
        ],
        [
            surface.knots_u[surface.degree_u],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ];
    let whole_chart = wires.len() == 1
        && wires[0].len() == 4
        && (0..4).any(|start| {
            wires[0].iter().enumerate().all(|(i, b)| {
                let c = &b.pcurve;
                let d = c.domain();
                c.degree == 1
                    && !c.periodic
                    && c.control_points.len() == 2
                    && c.knots[..2].iter().all(|&t| t == d[0])
                    && c.knots[2..].iter().all(|&t| t == d[1])
                    && c.control_points[0] == corners[(start + i) % 4]
                    && c.control_points[1] == corners[(start + i + 1) % 4]
            })
        });
    out.region = Some(SourceRegion {
        recipe: Recipe::Original(Original::new(context, surface, wires, tolerance_uv)),
        whole_chart,
        chart_winding: out.audit.region.winding[0].unwrap(),
        loops,
        source_loop_indices: (0..wires.len()).collect(),
    });
    out.reason = "original-source-region-qualified";
    Ok(out)
}
impl SourceRegion {
    /// Splitting one existing restriction preserves the exact UV region. Both
    /// children keep the same original curve, with a shared qualified source root.
    pub fn split_boundary(
        &self,
        loop_index: usize,
        edge_index: usize,
        point: &crate::source_contact_point::SourcePoint,
        role: Role,
    ) -> Result<Self> {
        let edge = self
            .loops
            .get(loop_index)
            .and_then(|w| w.get(edge_index))
            .ok_or_else(|| Error::new("BREP_SOURCE_REGION", "Unknown source boundary address"))?;
        let mut out =
            self.partition_boundary(loop_index, edge_index, edge.split_at(point, role)?)?;
        out.recipe = Recipe::SplitRoot {
            parent: Box::new(self.recipe.clone()),
            address: [loop_index, edge_index],
            point: point.clone(),
            role,
        };
        Ok(out)
    }
    /// Preserve the qualified region while partitioning an original boundary
    /// at an explicit parameter. This never approximates a crossing root.
    pub fn split_boundary_parameter(
        &self,
        loop_index: usize,
        edge_index: usize,
        t: f64,
    ) -> Result<Self> {
        let edge = self
            .loops
            .get(loop_index)
            .and_then(|w| w.get(edge_index))
            .ok_or_else(|| Error::new("BREP_SOURCE_REGION", "Unknown source boundary address"))?;
        let mut out =
            self.partition_boundary(loop_index, edge_index, edge.split_at_parameter(t)?)?;
        out.recipe = Recipe::SplitParameter {
            parent: Box::new(self.recipe.clone()),
            address: [loop_index, edge_index],
            parameter: t,
        };
        Ok(out)
    }
    fn partition_boundary(
        &self,
        loop_index: usize,
        edge_index: usize,
        parts: [Fragment; 2],
    ) -> Result<Self> {
        let mut out = self.clone();
        out.loops[loop_index].splice(edge_index..edge_index + 1, parts);
        if out.loops[loop_index].len() > 256 {
            return Err(Error::new(
                "BREP_SOURCE_REGION",
                "Source boundary partition work limit",
            ));
        }
        Ok(out)
    }
}
pub struct LinearRegion {
    pub interior: InteriorContact,
    pub controls: usize,
    pub driver_cells: usize,
    pub work_stopped: bool,
    pub kept_side_proven: bool,
    pub removed_side_proven: bool,
    pub holes_kept_proven: bool,
    pub hole_placement_proven: bool,
    pub removed_holes: Vec<usize>,
    pub region: Option<SourceRegion>,
    pub reason: &'static str,
}
