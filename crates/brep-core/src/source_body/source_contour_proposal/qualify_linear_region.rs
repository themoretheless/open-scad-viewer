use super::*;

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
        driver_cells: 0,
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
    let mut driver_cells = out.interior.driver.as_ref().map_or(0, |r| r.visited);
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
        if driver_cells < max_driver_cells {
            // A source-owned root lies exactly on this original line. Strict
            // monotonicity decides its entire retained/removed side even when
            // an outward root enclosure straddles the line.
            // Orientation is expressed by winding, preserving source ownership.
            let r = crate::source_halfplane_side::prove(
                fragment,
                contact,
                if reversed { -winding } else { winding },
                kept,
                max_driver_cells - driver_cells,
            )?;
            driver_cells += r.driver_cells;
            if r.proven {
                return Ok(true);
            }
        } else {
            work_stopped = true;
        }
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
    out.driver_cells = driver_cells;
    out.work_stopped = work_stopped;
    if kept && removed && holes {
        out.interior.region_subset_proven = true;
        let qualified = retained
            .iter()
            .map(|&i| loops[i].clone())
            .collect::<Vec<_>>();
        out.region = Some(SourceRegion {
            recipe: Recipe::Cut {
                original: Original::new(context, surface, wires, tolerance_uv),
                kind: CutKind::Linear,
                contact: contact.clone(),
                address: [loop_index, start_edge, end_edge],
                target_width,
                axis: driver_axis,
            },
            chart_winding: winding,
            whole_chart: false,
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
        recipe: Recipe::Cut {
            original: Original::new(context, surface, wires, tolerance_uv),
            kind: CutKind::Curved,
            contact: contact.clone(),
            address: [loop_index, start_edge, end_edge],
            target_width,
            axis: driver_axis,
        },
        chart_winding: out.interior.proposal.search.search.original.region.winding[0].unwrap(),
        whole_chart: false,
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
