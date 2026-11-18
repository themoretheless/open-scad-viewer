use super::*;

/// The original face region and the contact that cuts it between two boundary edges.
#[derive(Clone, Copy)]
pub struct RegionSource<'a> {
    pub surface: &'a Surface,
    pub wires: &'a [Vec<Boundary>],
    pub contact: &'a Curve,
    pub address: RegionAddress,
}
/// Where the contact replaces the boundary: loop and the first and last replaced edges.
#[derive(Clone, Copy, Debug)]
pub struct RegionAddress {
    pub loop_index: usize,
    pub start_edge: usize,
    pub end_edge: usize,
}
/// Requested UV resolution and the parameter axis that drives the contact.
#[derive(Clone, Copy, Debug)]
pub struct RegionTarget {
    pub width: [f64; 2],
    pub driver_axis: usize,
}
/// Work limits shared by linear and curved region qualification.
#[derive(Clone, Copy, Debug)]
pub struct RegionBudget {
    pub cells: usize,
    pub point_checks: usize,
    pub mapping_cells: usize,
    pub driver_cells: usize,
    pub membership_cells: usize,
}

fn single_line(c: &Curve) -> bool {
    let d = c.domain();
    c.degree == 1
        && c.control_points.len() == 2
        && c.knots[..=1].iter().all(|&k| k == d[0])
        && c.knots[2..].iter().all(|&k| k == d[1])
}

struct HalfplaneProver<'a> {
    contact: &'a Curve,
    origin: [f64; 2],
    direction: [nurbs_core::interval_eval::Interval; 2],
    winding: i32,
    reversed: bool,
    max_controls: usize,
    max_driver_cells: usize,
    controls: usize,
    driver_cells: usize,
    work_stopped: bool,
}

impl HalfplaneProver<'_> {
    fn side(
        &self,
        p: [nurbs_core::interval_eval::Interval; 2],
    ) -> Result<nurbs_core::interval_eval::Interval> {
        use nurbs_core::interval_eval::Interval as I;
        self.direction[0]
            .mul(p[1].sub(I::point(self.origin[1]))?)?
            .sub(self.direction[1].mul(p[0].sub(I::point(self.origin[0]))?)?)?
            .mul(I::point(self.winding as f64))
    }

    fn prove(&mut self, fragment: &Fragment, kept: bool) -> Result<bool> {
        use nurbs_core::interval_eval::{self, Interval as I};
        let accept = |s: I| if kept { s.lo >= 0. } else { s.hi <= 0. };
        let c = fragment.curve();
        let d = c.domain();
        let ends = fragment.endpoints();
        let full = matches!(ends[0], Endpoint::Parameter(t) if t == d[0])
            && matches!(ends[1], Endpoint::Parameter(t) if t == d[1]);
        if full {
            for p in &c.control_points {
                if self.controls == self.max_controls {
                    self.work_stopped = true;
                    return Ok(false);
                }
                self.controls += 1;
                if !accept(self.side([I::point(p[0]), I::point(p[1])])?) {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        if single_line(c) {
            for end in ends {
                match end {
                    Endpoint::Crossing { point, .. } if point.contact() == self.contact => continue,
                    Endpoint::Parameter(t) if *t == d[0] || *t == d[1] => {
                        if self.controls == self.max_controls {
                            self.work_stopped = true;
                            return Ok(false);
                        }
                        self.controls += 1;
                        let p = &c.control_points[usize::from(*t == d[1])];
                        if !accept(self.side([I::point(p[0]), I::point(p[1])])?) {
                            return Ok(false);
                        }
                    }
                    _ => return Ok(false),
                }
            }
            return Ok(true);
        }
        let cost = c.control_points.len();
        if cost > self.max_controls - self.controls {
            self.work_stopped = true;
            return Ok(false);
        }
        self.controls += cost;
        if self.driver_cells < self.max_driver_cells {
            // A source-owned root lies exactly on this original line. Strict
            // monotonicity decides its entire retained/removed side even when
            // an outward root enclosure straddles the line.
            // Orientation is expressed by winding, preserving source ownership.
            let r = crate::source_halfplane_side::prove(
                fragment,
                self.contact,
                if self.reversed { -self.winding } else { self.winding },
                kept,
                self.max_driver_cells - self.driver_cells,
            )?;
            self.driver_cells += r.driver_cells;
            if r.proven {
                return Ok(true);
            }
        } else {
            self.work_stopped = true;
        }
        let bounds = fragment.parameter_bounds();
        let domain = I::new(
            bounds[0][0].min(bounds[1][0]),
            bounds[0][1].max(bounds[1][1]),
        )?;
        let image = interval_eval::evaluate_interval(c, domain)?;
        Ok(accept(self.side([image[0], image[1]])?))
    }

    fn classify_holes(&mut self, loops: &[Vec<Fragment>]) -> Result<(bool, Vec<usize>, Vec<usize>)> {
        let mut holes = true;
        let mut retained = vec![0];
        let mut removed_holes = vec![];
        for (index, hole) in loops.iter().enumerate().skip(1) {
            let mut on_kept_side = true;
            for fragment in hole {
                on_kept_side &= self.prove(fragment, true)?;
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
                on_removed_side &= self.prove(fragment, false)?;
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
        Ok((holes, retained, removed_holes))
    }
}

/// Qualify a straight source contact as an exact half-plane intersection of
/// the original material region. All retained/removed arcs and holes must have
/// whole-source side proofs. This does not approximate a curved contact by a line.
pub fn qualify_linear_region(
    context: &ToleranceContext,
    source: RegionSource<'_>,
    tolerance_uv: f64,
    limits: Limits,
    target: RegionTarget,
    budget: RegionBudget,
    max_controls: usize,
) -> Result<LinearRegion> {
    let RegionSource { surface, wires, contact, address } = source;
    let RegionAddress { loop_index, start_edge, end_edge } = address;
    let RegionTarget { width: target_width, driver_axis } = target;
    let max_driver_cells = budget.driver_cells;
    use nurbs_core::interval_eval::Interval as I;
    if !(1..=100000).contains(&max_controls) {
        return Err(Error::new(
            "BREP_SOURCE_REGION_WORK",
            "Bound half-plane source control work",
        ));
    }
    let interior = qualify_interior_contact(context, source, tolerance_uv, limits, target, budget)?;
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
    let mut prover = HalfplaneProver {
        contact,
        origin: [a[0], a[1]],
        direction: [
            I::point(b[0]).sub(I::point(a[0]))?,
            I::point(b[1]).sub(I::point(a[1]))?,
        ],
        winding,
        reversed,
        max_controls,
        max_driver_cells,
        controls: 0,
        driver_cells: out.interior.driver.as_ref().map_or(0, |r| r.visited),
        work_stopped: false,
    };
    let mut kept = true;
    for fragment in &loops[0][1..] {
        kept &= prover.prove(fragment, true)?;
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
        Endpoint::Crossing { point: p0, role: Role::Boundary },
        Endpoint::Parameter(head.domain()[1]),
    )?;
    let removed1 = Fragment::new(
        surface,
        tail,
        Endpoint::Parameter(tail.domain()[0]),
        Endpoint::Crossing { point: p1, role: Role::Boundary },
    )?;
    let mut removed = prover.prove(&removed0, false)? && prover.prove(&removed1, false)?;
    let mut i = (start_edge + 1) % original.len();
    while i != end_edge && removed {
        let c = &original[i].pcurve;
        let full = Fragment::new(surface, c, Endpoint::Parameter(c.domain()[0]), Endpoint::Parameter(c.domain()[1]))?;
        removed &= prover.prove(&full, false)?;
        i = (i + 1) % original.len();
    }
    out.removed_side_proven = removed;
    let (holes, retained, removed_holes) = prover.classify_holes(loops)?;
    out.holes_kept_proven = holes && removed_holes.is_empty();
    out.hole_placement_proven = holes;
    out.removed_holes = removed_holes;
    out.controls = prover.controls;
    out.driver_cells = prover.driver_cells;
    out.work_stopped = prover.work_stopped;
    if kept && removed && holes {
        out.interior.region_subset_proven = true;
        let qualified = retained.iter().map(|&i| loops[i].clone()).collect::<Vec<_>>();
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
        out.reason = if prover.work_stopped {
            "source-half-plane-work-limit"
        } else {
            "source-half-plane-side-unproven"
        };
    }
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
    source: RegionSource<'_>,
    tolerance_uv: f64,
    limits: Limits,
    target: RegionTarget,
    budget: RegionBudget,
    max_winding_cells: usize,
) -> Result<CurvedRegion> {
    let RegionSource { surface, wires, contact, address } = source;
    let RegionAddress { loop_index, start_edge, end_edge } = address;
    let RegionTarget { width: target_width, driver_axis } = target;
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
    let interior = qualify_interior_contact(context, source, tolerance_uv, limits, target, budget)?;
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
