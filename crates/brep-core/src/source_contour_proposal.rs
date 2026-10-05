//! Source-expression contour clipping proposal. Exact joins are separate from
//! region containment, face immersion and closed-solid admission.
use crate::{
    source_boundary_fragment::{Endpoint, Fragment, Role},
    trimmed_face_recipe::{self, Boundary, Limits, SourcePointSearch},
};
use cad_predicates::ToleranceContext;
use nurbs_core::{curve::Curve, surface::Surface, Error, Result};
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
    loops: Vec<Vec<Fragment>>,
    source_loop_indices: Vec<usize>,
}
impl SourceRegion {
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
    out.region = Some(SourceRegion {
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
        self.partition_boundary(loop_index,edge_index,edge.split_at(point,role)?)
    }
    /// Preserve the qualified region while partitioning an original boundary
    /// at an explicit parameter. This never approximates a crossing root.
    pub fn split_boundary_parameter(&self,loop_index:usize,edge_index:usize,t:f64) -> Result<Self> {
        let edge=self.loops.get(loop_index).and_then(|w|w.get(edge_index))
            .ok_or_else(||Error::new("BREP_SOURCE_REGION","Unknown source boundary address"))?;
        self.partition_boundary(loop_index,edge_index,edge.split_at_parameter(t)?)
    }
    fn partition_boundary(&self,loop_index:usize,edge_index:usize,parts:[Fragment;2]) -> Result<Self> {
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
    pub work_stopped: bool,
    pub kept_side_proven: bool,
    pub removed_side_proven: bool,
    pub holes_kept_proven: bool,
    pub hole_placement_proven: bool,
    pub removed_holes: Vec<usize>,
    pub region: Option<SourceRegion>,
    pub reason: &'static str,
}
/// Qualify a straight source contact as an exact half-plane intersection of
/// the original material region. All retained/removed arcs and holes must have
/// whole-source side proofs. This does not approximate a curved contact by a line.
pub fn qualify_linear_region(
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
    max_controls: usize,
) -> Result<LinearRegion> {
    use nurbs_core::interval_eval::{self, Interval as I};
    if !(1..=100000).contains(&max_controls) {
        return Err(Error::new(
            "BREP_SOURCE_REGION_WORK",
            "Bound half-plane source control work",
        ));
    }
    let interior = qualify_interior_contact(
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
        driver_axis,
        max_driver_cells,
        max_membership_cells,
    )?;
    let mut out = LinearRegion {
        interior,
        controls: 0,
        work_stopped: false,
        kept_side_proven: false,
        removed_side_proven: false,
        holes_kept_proven: false,
        hole_placement_proven: false,
        removed_holes: vec![],
        region: None,
        reason: "source-contact-interior-unqualified",
    };
    if !out.interior.contact_inside_proven {
        return Ok(out);
    }
    if loop_index != 0 {
        out.reason = "source-hole-boundary-clipping-unqualified";
        return Ok(out);
    }
    let single_line = |c: &Curve| {
        let d = c.domain();
        c.degree == 1
            && c.control_points.len() == 2
            && c.knots[..=1].iter().all(|&k| k == d[0])
            && c.knots[2..].iter().all(|&k| k == d[1])
    };
    if !single_line(contact) {
        out.reason = "source-contact-is-not-an-admitted-line";
        return Ok(out);
    }
    let loops = out.interior.proposal.candidate_loops.as_ref().unwrap();
    let reversed = loops[0][0].reversed();
    let a = &contact.control_points[usize::from(reversed)];
    let b = &contact.control_points[usize::from(!reversed)];
    let winding = out.interior.proposal.search.search.original.region.winding[0];
    let Some(winding) = winding.filter(|w| w.abs() == 1) else {
        out.reason = "source-region-winding-unproven";
        return Ok(out);
    };
    let direction = [
        I::point(b[0]).sub(I::point(a[0]))?,
        I::point(b[1]).sub(I::point(a[1]))?,
    ];
    let side = |p: [I; 2]| -> Result<I> {
        direction[0]
            .mul(p[1].sub(I::point(a[1]))?)?
            .sub(direction[1].mul(p[0].sub(I::point(a[0]))?)?)?
            .mul(I::point(winding as f64))
    };
    let mut controls = 0;
    let mut work_stopped = false;
    let mut prove = |fragment: &Fragment, kept: bool| -> Result<bool> {
        let accept = |s: I| if kept { s.lo >= 0. } else { s.hi <= 0. };
        let c = fragment.curve();
        let d = c.domain();
        let ends = fragment.endpoints();
        let full = matches!(ends[0],Endpoint::Parameter(t) if t==d[0])
            && matches!(ends[1],Endpoint::Parameter(t) if t==d[1]);
        if full {
            for p in &c.control_points {
                if controls == max_controls {
                    work_stopped = true;
                    return Ok(false);
                }
                controls += 1;
                if !accept(side([I::point(p[0]), I::point(p[1])])?) {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        if single_line(c) {
            for end in ends {
                match end {
                    Endpoint::Crossing { point, .. } if point.contact() == contact => continue,
                    Endpoint::Parameter(t) if *t == d[0] || *t == d[1] => {
                        if controls == max_controls {
                            work_stopped = true;
                            return Ok(false);
                        }
                        controls += 1;
                        let p = &c.control_points[usize::from(*t == d[1])];
                        if !accept(side([I::point(p[0]), I::point(p[1])])?) {
                            return Ok(false);
                        }
                    }
                    _ => return Ok(false),
                }
            }
            return Ok(true);
        }
        let cost = c.control_points.len();
        if cost > max_controls - controls {
            work_stopped = true;
            return Ok(false);
        }
        controls += cost;
        let bounds = fragment.parameter_bounds();
        let domain = I::new(
            bounds[0][0].min(bounds[1][0]),
            bounds[0][1].max(bounds[1][1]),
        )?;
        let image = interval_eval::evaluate_interval(c, domain)?;
        Ok(accept(side([image[0], image[1]])?))
    };
    let mut kept = true;
    for fragment in &loops[0][1..] {
        kept &= prove(fragment, true)?;
        if !kept {
            break;
        }
    }
    out.kept_side_proven = kept;
    let original = &wires[0];
    let p0 = match &loops[0][0].endpoints()[0] {
        Endpoint::Crossing { point, .. } => point.clone(),
        _ => unreachable!(),
    };
    let p1 = match &loops[0][0].endpoints()[1] {
        Endpoint::Crossing { point, .. } => point.clone(),
        _ => unreachable!(),
    };
    let head = &original[start_edge].pcurve;
    let tail = &original[end_edge].pcurve;
    let removed0 = Fragment::new(
        surface,
        head,
        Endpoint::Crossing {
            point: p0,
            role: Role::Boundary,
        },
        Endpoint::Parameter(head.domain()[1]),
    )?;
    let removed1 = Fragment::new(
        surface,
        tail,
        Endpoint::Parameter(tail.domain()[0]),
        Endpoint::Crossing {
            point: p1,
            role: Role::Boundary,
        },
    )?;
    let mut removed = prove(&removed0, false)? && prove(&removed1, false)?;
    let mut i = (start_edge + 1) % original.len();
    while i != end_edge && removed {
        let c = &original[i].pcurve;
        let full = Fragment::new(
            surface,
            c,
            Endpoint::Parameter(c.domain()[0]),
            Endpoint::Parameter(c.domain()[1]),
        )?;
        removed &= prove(&full, false)?;
        i = (i + 1) % original.len();
    }
    out.removed_side_proven = removed;
    let mut holes = true;
    let mut retained = vec![0];
    let mut removed_holes = vec![];
    for (index, hole) in loops.iter().enumerate().skip(1) {
        let mut on_kept_side = true;
        for fragment in hole {
            on_kept_side &= prove(fragment, true)?;
            if !on_kept_side {
                break;
            }
        }
        if on_kept_side {
            retained.push(index);
            continue;
        }
        let mut on_removed_side = true;
        for fragment in hole {
            on_removed_side &= prove(fragment, false)?;
            if !on_removed_side {
                break;
            }
        }
        if on_removed_side {
            removed_holes.push(index);
        } else {
            holes = false;
            break;
        }
    }
    out.holes_kept_proven = holes && removed_holes.is_empty();
    out.hole_placement_proven = holes;
    out.removed_holes = removed_holes;
    out.controls = controls;
    out.work_stopped = work_stopped;
    if kept && removed && holes {
        out.interior.region_subset_proven = true;
        let qualified = retained
            .iter()
            .map(|&i| loops[i].clone())
            .collect::<Vec<_>>();
        out.region = Some(SourceRegion {
            loops: qualified.clone(),
            source_loop_indices: retained.clone(),
        });
        out.interior.proposal.candidate_loops = Some(qualified);
        out.interior.proposal.candidate_source_loops = Some(retained);
        out.interior.proposal.reason = "source-contour-half-plane-region-qualified";
        out.reason = "source-half-plane-material-region-qualified";
    } else {
        out.reason = if work_stopped {
            "source-half-plane-work-limit"
        } else {
            "source-half-plane-side-unproven"
        };
    }
    // Whole original outer arcs occupy opposite sides, meeting the line only at
    // the two owned transverse roots. The closed candidate bounds the retained
    // half-plane intersection. Holes proven on the kept side retain their
    // definitions; whole holes on the removed side disappear from that region.
    Ok(out)
}

pub struct CurvedHole {
    pub source_loop: usize,
    pub classification: crate::source_contour_winding::Report,
}
pub struct CurvedRegion {
    pub interior: InteriorContact,
    pub holes: Vec<CurvedHole>,
    pub winding_cells: usize,
    pub removed_holes: Vec<usize>,
    pub region: Option<SourceRegion>,
    pub reason: &'static str,
}
/// Qualify a source contact by Jordan arc replacement and original-fragment
/// winding, without substituting a line or a rounded curve partition.
/// Complete simplicity/inside/no-extra-crossing proofs precede hole queries.
pub fn qualify_curved_region(
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
    max_winding_cells: usize,
) -> Result<CurvedRegion> {
    use nurbs_core::{
        interval_eval::{self, Interval as I},
        trim_domain::Location,
    };
    if !(1..=100000).contains(&max_winding_cells) {
        return Err(Error::new(
            "BREP_CURVED_REGION_WORK",
            "Bound source contour winding work",
        ));
    }
    let interior = qualify_interior_contact(
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
        driver_axis,
        max_driver_cells,
        max_membership_cells,
    )?;
    let mut out = CurvedRegion {
        interior,
        holes: vec![],
        winding_cells: 0,
        removed_holes: vec![],
        region: None,
        reason: "source-contact-interior-unqualified",
    };
    if !out.interior.contact_inside_proven {
        return Ok(out);
    }
    if loop_index != 0 {
        out.reason = "source-hole-boundary-clipping-unqualified";
        return Ok(out);
    }
    let loops = out.interior.proposal.candidate_loops.as_ref().unwrap();
    let mut retained = vec![0];
    for i in 1..loops.len() {
        if out.winding_cells == max_winding_cells {
            out.reason = "source-hole-winding-work-limit";
            return Ok(out);
        }
        let c = &wires[i][0].pcurve;
        let d = c.domain();
        let query = if c.knots[..=c.degree].iter().all(|&k| k == d[0]) {
            [[c.control_points[0][0]; 2], [c.control_points[0][1]; 2]]
        } else {
            let p = interval_eval::evaluate_interval(c, I::point(d[0]))?;
            [[p[0].lo, p[0].hi], [p[1].lo, p[1].hi]]
        };
        let classification = crate::source_contour_winding::classify(
            std::slice::from_ref(&loops[0]),
            query,
            tolerance_uv,
            max_winding_cells - out.winding_cells,
        )?;
        out.winding_cells += classification.cells;
        let location = classification.location;
        out.holes.push(CurvedHole {
            source_loop: i,
            classification,
        });
        match location {
            Location::Inside => retained.push(i),
            Location::Outside => out.removed_holes.push(i),
            Location::Unresolved => {
                out.reason = "source-hole-winding-unproven";
                return Ok(out);
            }
        }
    }
    // The original outer Jordan curve and simple interior crosscut bound two
    // subregions. The retained original arc fixes the selected subregion and
    // orientation. Complete original boundary search proves each old hole
    // disjoint from the crosscut; one certified boundary witness fixes its side.
    let qualified = retained
        .iter()
        .map(|&i| loops[i].clone())
        .collect::<Vec<_>>();
    out.region = Some(SourceRegion {
        loops: qualified.clone(),
        source_loop_indices: retained.clone(),
    });
    out.interior.proposal.candidate_loops = Some(qualified);
    out.interior.proposal.candidate_source_loops = Some(retained);
    out.interior.proposal.reason = "source-curved-contour-region-qualified";
    out.interior.region_subset_proven = true;
    out.reason = "source-curved-material-region-qualified";
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_crossings_clip_source_expressions_without_rounded_curve_trims() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 1.], vec![0., 1., 1.]],
                vec![vec![1., 0., 1.], vec![1., 1., 1.]],
            ],
            weights: vec![vec![1., 1.], vec![1., 1.]],
            periodic_u: false,
            periodic_v: false,
        };
        let p = [vec![1., 0.], vec![1., 1.], vec![0., 1.], vec![0., 0.]];
        let wire = (0..4)
            .map(|i| {
                let pcurve =
                    Curve::from_polyline(vec![p[i].clone(), p[(i + 1) % 4].clone()]).unwrap();
                let mut curve = pcurve.clone();
                for p in &mut curve.control_points {
                    p.push(1.);
                }
                Boundary {
                    curve,
                    pcurve,
                    reversed: false,
                }
            })
            .collect::<Vec<_>>();
        let contact = Curve::from_polyline(vec![vec![-0.2, 0.3], vec![1.2, 0.3]]).unwrap();
        let limits = Limits {
            pairs: 10000,
            region_cells: 100000,
            domain_cells: 100000,
            agreement_cells: 100000,
        };
        let out = propose(
            &ToleranceContext::default_valid(),
            &s,
            &[wire.clone()],
            &contact,
            0,
            0,
            2,
            1e-8,
            limits,
            10000,
            [1e-7, 1e-7],
            16,
            16,
        )
        .unwrap();
        assert!(out.source_joins_proven);
        let loops = out.candidate_loops.unwrap();
        assert_eq!(loops[0].len(), 4);
        assert_eq!(loops[0][0].curve(), &contact);
        assert!(loops[0][0].reversed());
        assert_eq!(loops[0][1].curve(), &wire[2].pcurve);
        assert_eq!(loops[0][2].curve(), &wire[3].pcurve);
        assert_eq!(loops[0][3].curve(), &wire[0].pcurve);
        let interior = qualify_interior_contact(
            &ToleranceContext::default_valid(),
            &s,
            &[wire.clone()],
            &contact,
            0,
            0,
            2,
            1e-8,
            limits,
            10000,
            [1e-7, 1e-7],
            16,
            16,
            0,
            1000,
            10000,
        )
        .unwrap();
        assert!(
            interior.only_owned_crossings_proven
                && interior.contact_simple_proven
                && interior.contact_inside_proven
        );
        assert!(!interior.region_subset_proven);
        let hole_points = [
            vec![0.4, 0.2],
            vec![0.4, 0.4],
            vec![0.6, 0.4],
            vec![0.6, 0.2],
        ];
        let hole = (0..4)
            .map(|i| {
                let pcurve = Curve::from_polyline(vec![
                    hole_points[i].clone(),
                    hole_points[(i + 1) % 4].clone(),
                ])
                .unwrap();
                let mut curve = pcurve.clone();
                for p in &mut curve.control_points {
                    p.push(1.);
                }
                Boundary {
                    curve,
                    pcurve,
                    reversed: false,
                }
            })
            .collect::<Vec<_>>();
        let bad = qualify_interior_contact(
            &ToleranceContext::default_valid(),
            &s,
            &[wire.clone(), hole.clone()],
            &contact,
            0,
            0,
            2,
            1e-8,
            limits,
            10000,
            [1e-7, 1e-7],
            16,
            16,
            0,
            1000,
            10000,
        )
        .unwrap();
        assert!(bad.proposal.search.complete);
        assert_eq!(bad.proposal.search.points.len(), 4);
        let across_hole = Curve::from_polyline(vec![vec![0.3, 0.3], vec![0.7, 0.3]]).unwrap();
        let void = qualify_interior_contact(
            &ToleranceContext::default_valid(),
            &s,
            &[wire.clone(), hole],
            &across_hole,
            1,
            0,
            2,
            1e-8,
            limits,
            10000,
            [1e-7, 1e-7],
            16,
            16,
            0,
            1000,
            10000,
        )
        .unwrap();
        assert!(void.only_owned_crossings_proven && void.contact_simple_proven);
        assert!(!void.contact_inside_proven);
        assert_eq!(
            void.membership.as_ref().unwrap().location,
            nurbs_core::trim_domain::Location::Outside
        );

        assert!(
            !bad.only_owned_crossings_proven
                && !bad.contact_inside_proven
                && !bad.region_subset_proven
        );

        let region = qualify_linear_region(
            &ToleranceContext::default_valid(),
            &s,
            &[wire.clone()],
            &contact,
            0,
            0,
            2,
            1e-8,
            limits,
            10000,
            [1e-7, 1e-7],
            16,
            16,
            0,
            1000,
            10000,
            1000,
        )
        .unwrap();
        assert!(region.kept_side_proven && region.removed_side_proven && region.holes_kept_proven);
        assert!(region.interior.region_subset_proven && region.region.is_some());
        let make_hole = |y0: f64, y1: f64| {
            let points = [vec![0.4, y0], vec![0.4, y1], vec![0.6, y1], vec![0.6, y0]];
            (0..4)
                .map(|i| {
                    let pcurve =
                        Curve::from_polyline(vec![points[i].clone(), points[(i + 1) % 4].clone()])
                            .unwrap();
                    let mut curve = pcurve.clone();
                    for p in &mut curve.control_points {
                        p.push(1.);
                    }
                    Boundary {
                        curve,
                        pcurve,
                        reversed: false,
                    }
                })
                .collect::<Vec<_>>()
        };
        let kept = qualify_linear_region(
            &ToleranceContext::default_valid(),
            &s,
            &[wire.clone(), make_hole(0.05, 0.15)],
            &contact,
            0,
            0,
            2,
            1e-8,
            limits,
            10000,
            [1e-7, 1e-7],
            16,
            16,
            0,
            1000,
            10000,
            1000,
        )
        .unwrap();
        assert!(kept.holes_kept_proven && kept.region.is_some());
        assert_eq!(kept.region.unwrap().loops().len(), 2);
        let removed_hole = qualify_linear_region(
            &ToleranceContext::default_valid(),
            &s,
            &[wire.clone(), make_hole(0.5, 0.7)],
            &contact,
            0,
            0,
            2,
            1e-8,
            limits,
            10000,
            [1e-7, 1e-7],
            16,
            16,
            0,
            1000,
            10000,
            1000,
        )
        .unwrap();
        assert!(removed_hole.interior.contact_inside_proven);
        assert!(!removed_hole.holes_kept_proven && removed_hole.hole_placement_proven);
        assert!(removed_hole.interior.region_subset_proven);
        assert_eq!(removed_hole.removed_holes, vec![1]);
        let region = removed_hole.region.unwrap();
        assert_eq!(region.loops().len(), 1);
        assert_eq!(region.source_loop_indices(), &[0]);
        assert_eq!(
            removed_hole
                .interior
                .proposal
                .candidate_loops
                .as_ref()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            removed_hole
                .interior
                .proposal
                .candidate_source_loops
                .as_ref()
                .unwrap(),
            &vec![0]
        );
        let mixed = qualify_linear_region(
            &ToleranceContext::default_valid(),
            &s,
            &[wire.clone(), make_hole(0.5, 0.7), make_hole(0.05, 0.15)],
            &contact,
            0,
            0,
            2,
            1e-8,
            limits,
            10000,
            [1e-7, 1e-7],
            16,
            16,
            0,
            1000,
            10000,
            1000,
        )
        .unwrap();
        assert_eq!(mixed.removed_holes, vec![1]);
        let region = mixed.region.unwrap();
        assert_eq!(region.source_loop_indices(), &[0, 2]);
        assert_eq!(region.loops().len(), 2);

        let limited = qualify_linear_region(
            &ToleranceContext::default_valid(),
            &s,
            &[wire.clone()],
            &contact,
            0,
            0,
            2,
            1e-8,
            limits,
            10000,
            [1e-7, 1e-7],
            16,
            16,
            0,
            1000,
            10000,
            1,
        )
        .unwrap();
        assert!(limited.region.is_none() && !limited.interior.region_subset_proven);
        assert_eq!(limited.controls, 1);

        let mut clockwise = wire.clone();
        clockwise.reverse();
        for b in &mut clockwise {
            b.pcurve.control_points.reverse();
            b.curve.control_points.reverse();
        }
        let clockwise = qualify_linear_region(
            &ToleranceContext::default_valid(),
            &s,
            &[clockwise],
            &contact,
            0,
            1,
            3,
            1e-8,
            limits,
            10000,
            [1e-7, 1e-7],
            16,
            16,
            0,
            1000,
            10000,
            1000,
        )
        .unwrap();
        assert!(clockwise.region.is_none() && !clockwise.interior.region_subset_proven);
        assert_eq!(
            clockwise.interior.proposal.search.search.original.reason,
            "face-region-orientation-invalid"
        );
        let vertical = Curve::from_polyline(vec![vec![0.3, -0.2], vec![0.3, 1.2]]).unwrap();
        let vertical = qualify_linear_region(
            &ToleranceContext::default_valid(),
            &s,
            &[wire.clone()],
            &vertical,
            0,
            3,
            1,
            1e-8,
            limits,
            10000,
            [1e-7, 1e-7],
            16,
            16,
            1,
            1000,
            10000,
            1000,
        )
        .unwrap();
        assert!(vertical.region.is_some() && vertical.interior.region_subset_proven);

        let curved = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![-0.2, 0.25], vec![0.5, 0.3], vec![1.2, 0.6]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let curved_region = qualify_curved_region(
            &ToleranceContext::default_valid(),
            &s,
            &[wire.clone(), make_hole(0.05, 0.15), make_hole(0.7, 0.85)],
            &curved,
            0,
            0,
            2,
            1e-8,
            limits,
            10000,
            [1e-7, 1e-7],
            16,
            16,
            0,
            1000,
            10000,
            10000,
        )
        .unwrap();
        assert!(
            curved_region.region.is_some(),
            "{} / {}",
            curved_region.reason,
            curved_region.interior.reason
        );
        assert_eq!(curved_region.removed_holes, vec![2]);
        assert_eq!(
            curved_region.region.as_ref().unwrap().source_loop_indices(),
            &[0, 1]
        );
        assert_eq!(
            curved_region.region.as_ref().unwrap().loops()[0][0].curve(),
            &curved
        );

        let mut rational = curved.clone();
        rational.weights[1] = 0.7;
        let rational_region = qualify_curved_region(
            &ToleranceContext::default_valid(),
            &s,
            &[wire.clone(), make_hole(0.05, 0.15), make_hole(0.7, 0.85)],
            &rational,
            0,
            0,
            2,
            1e-8,
            limits,
            10000,
            [1e-7, 1e-7],
            16,
            16,
            0,
            1000,
            10000,
            10000,
        )
        .unwrap();
        assert!(
            rational_region.region.is_some(),
            "{} / {}",
            rational_region.reason,
            rational_region.interior.reason
        );
        assert_eq!(rational_region.removed_holes, vec![2]);
        let world_wires = rational_region
            .region
            .as_ref()
            .unwrap()
            .world_wires()
            .unwrap();
        assert_eq!(world_wires.len(), 2);
        for wire in &world_wires {
            assert!(wire.world_mapping(1000).unwrap().complete);
            assert_eq!(wire.vertices().last().unwrap()[1], 0);
        }
        assert_eq!(world_wires[0].edges()[0].curve(), &rational);
        assert_eq!(
            rational_region.region.as_ref().unwrap().loops()[0][0].curve(),
            &rational
        );
        let stopped_curved = qualify_curved_region(
            &ToleranceContext::default_valid(),
            &s,
            &[wire.clone(), make_hole(0.05, 0.15), make_hole(0.7, 0.85)],
            &curved,
            0,
            0,
            2,
            1e-8,
            limits,
            10000,
            [1e-7, 1e-7],
            16,
            16,
            0,
            1000,
            10000,
            1,
        )
        .unwrap();
        assert!(stopped_curved.region.is_none() && !stopped_curved.interior.region_subset_proven);
        let stopped = propose(
            &ToleranceContext::default_valid(),
            &s,
            &[wire],
            &contact,
            0,
            0,
            2,
            1e-8,
            limits,
            1,
            [1e-7, 1e-7],
            16,
            16,
        )
        .unwrap();
        assert!(!stopped.source_joins_proven && stopped.candidate_loops.is_none());
    }
}
