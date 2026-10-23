//! Allowed contacts on a supported interior source coordinate fiber.
//! Original restrictions and exact shared ownership are retained unchanged.
use crate::{
    source_contour_proposal::SourceRegion,
    source_fiber_boundary::Locus,
    source_interior_fiber,
    source_shared_edge::SharedEdge,
    source_shell_incidence::{Address, Shell},
};
use nurbs_core::{Error, Result};
pub struct Certificate {
    faces: [usize; 2],
    regions: [SourceRegion; 2],
    edges: Vec<SharedEdge>,
    fiber: source_interior_fiber::Certificate,
}
impl Certificate {
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn regions(&self) -> &[SourceRegion; 2] {
        &self.regions
    }
    pub fn edges(&self) -> &[SharedEdge] {
        &self.edges
    }
    pub fn fiber(&self) -> &source_interior_fiber::Certificate {
        &self.fiber
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
    pub spans: usize,
    pub driver_cells: usize,
    pub linear_cells: usize,
    pub uncertain_boundary: Option<Address>,
    pub reason: &'static str,
}
/// Recheck chart injectivity, the complete original plane preimage, retained
/// region support on one side, and every possible contact's shared ownership.
pub fn certify_with_linear_chart(
    shell: &Shell,
    faces: [usize; 2],
    max_work: u64,
    max_spans: usize,
    max_driver_cells: usize,
    max_linear_cells: usize,
) -> Result<Report> {
    if faces[0] == faces[1]
        || faces.iter().any(|&f| f >= shell.faces().len())
        || !(1..=100_000_000).contains(&max_work)
        || !(1..=100000).contains(&max_spans)
        || !(1..=100000).contains(&max_driver_cells)
        || max_linear_cells > 100000
    {
        return Err(Error::new(
            "BREP_SOURCE_INTERIOR_CONTACT",
            "Choose distinct faces and bounded work",
        ));
    }
    let regions = shell.regions().ok_or_else(|| {
        Error::new(
            "BREP_SOURCE_INTERIOR_CONTACT",
            "Qualified material regions are required",
        )
    })?;
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        spans: 0,
        driver_cells: 0,
        linear_cells: 0,
        uncertain_boundary: None,
        reason: "source-fiber-chart-unproven",
    };
    for &f in &faces {
        if out.spans == max_spans {
            return Ok(out);
        }
        let r = nurbs_core::surface_injectivity::certify_contraction(
            regions[f].loops()[0][0].surface(),
            max_spans - out.spans,
        )?;
        out.spans += r.spans;
        if !r.proven {
            if out.linear_cells == max_linear_cells {
                return Ok(out);
            }
            let linear = nurbs_core::surface_linear_monotonicity::inspect_candidate(
                regions[f].loops()[0][0].surface(),
                max_linear_cells - out.linear_cells,
            )?;
            out.linear_cells += linear.cells;
            if !linear.certified {
                return Ok(out);
            }
        }
    }
    let Some(plane) = crate::source_allowed_contact::plane(
        regions[faces[0]].loops()[0][0].surface(),
        &mut out.exact_work,
        max_work,
    )?
    else {
        return Ok(out);
    };
    let shared = shell
        .uses()
        .iter()
        .enumerate()
        .filter(|(_, uses)| uses.iter().all(|a| faces.contains(&a.face)))
        .collect::<Vec<_>>();
    let addresses = shared
        .iter()
        .map(|(_, uses)| *uses.iter().find(|a| a.face == faces[1]).unwrap())
        .collect::<Vec<_>>();
    let mut levels = Vec::new();
    for address in &addresses {
        let f = &regions[faces[1]].loops()[address.wire][address.edge];
        for axis in 0..2 {
            let level = f.curve().control_points[0][axis];
            if f.curve().control_points.iter().all(|p| p[axis] == level)
                && !levels.contains(&(axis, level))
            {
                levels.push((axis, level));
            }
        }
    }
    for (axis, level) in levels {
        if out.exact_work == max_work || out.driver_cells == max_driver_cells {
            return Ok(out);
        }
        let r = source_interior_fiber::certify(
            regions[faces[1]].loops()[0][0].surface(),
            plane,
            axis,
            level,
            max_work - out.exact_work,
            max_driver_cells - out.driver_cells,
        )?;
        out.exact_work += r.exact_work;
        out.driver_cells += r.driver_cells;
        let Some(fiber) = r.certificate else {
            continue;
        };
        let mut support = None;
        let mut accepted = true;
        for (wire, edges) in regions[faces[1]].loops().iter().enumerate() {
            for (edge, fragment) in edges.iter().enumerate() {
                let address = Address {
                    face: faces[1],
                    wire,
                    edge,
                };
                out.uncertain_boundary = Some(address);
                let r = crate::source_interior_boundary::inspect(
                    fragment,
                    &fiber,
                    max_driver_cells - out.driver_cells,
                )?;
                out.driver_cells += r.driver_cells;
                let Some(side) = r.side else {
                    accepted = false;
                    break;
                };
                if side != crate::source_interior_boundary::Side::On {
                    if support.is_some_and(|s| s != side) {
                        accepted = false;
                        break;
                    }
                    support = Some(side);
                }
                let vertex_owned = |end: usize| {
                    let vertex = shell.vertices()[faces[1]][wire][edge][end];
                    addresses
                        .iter()
                        .any(|a| shell.vertices()[a.face][a.wire][a.edge].contains(&vertex))
                };
                match r.locus {
                    Locus::EntireFragment if addresses.contains(&address) => {}
                    Locus::Endpoints(ends) if (0..2).all(|i| !ends[i] || vertex_owned(i)) => {}
                    Locus::Away => {}
                    _ => {
                        accepted = false;
                        break;
                    }
                }
            }
            if !accepted {
                break;
            }
        }
        // Linear UV coordinate extrema lie on the boundary of a bounded
        // qualified material region. One common support side excludes interior
        // plane contact; every boundary contact above is source-owned.
        if accepted && support.is_some() {
            out.uncertain_boundary = None;
            out.certificate = Some(Certificate {
                faces,
                regions: [regions[faces[0]].clone(), regions[faces[1]].clone()],
                edges: shared
                    .iter()
                    .map(|(i, _)| shell.edges()[*i].clone())
                    .collect(),
                fiber,
            });
            out.reason = "source-interior-fiber-allowed-contact-proven";
            return Ok(out);
        }
    }
    out.reason = "source-interior-fiber-support-or-ownership-unproven";
    Ok(out)
}
