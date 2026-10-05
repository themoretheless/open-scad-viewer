//! Sufficient allowed-contact proof on a natural source chart boundary.
//! Original restrictions and exact shared ownership are retained unchanged.
use crate::{
    source_contour_proposal::SourceRegion,
    source_fiber_boundary::{self, Locus},
    source_plane_fiber,
    source_shared_edge::SharedEdge,
    source_shell_incidence::{Address, Shell},
};
use nurbs_core::{Error, Result};
pub struct Certificate {
    faces: [usize; 2],
    regions: [SourceRegion; 2],
    edges: Vec<SharedEdge>,
    fiber: source_plane_fiber::Certificate,
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
    pub fn fiber(&self) -> &source_plane_fiber::Certificate {
        &self.fiber
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
    pub spans: usize,
    pub driver_cells: usize,
    pub uncertain_boundary: Option<Address>,
    pub reason: &'static str,
}
/// First face must be planar. The second meets its plane only on a natural
/// chart edge. Every retained boundary fragment on that edge must be paired
/// with the first face; every possible endpoint contact must join such an edge.
/// Thus every possible material contact has exact shared source ownership.
pub fn certify(
    shell: &Shell,
    faces: [usize; 2],
    max_work: u64,
    max_spans: usize,
    max_driver_cells: usize,
) -> Result<Report> {
    if faces[0] == faces[1]
        || faces.iter().any(|&f| f >= shell.faces().len())
        || !(1..=100_000_000).contains(&max_work)
        || !(1..=100000).contains(&max_spans)
        || !(1..=100000).contains(&max_driver_cells)
    {
        return Err(Error::new(
            "BREP_SOURCE_FIBER_CONTACT",
            "Choose distinct faces and bounded work",
        ));
    }
    let regions = shell.regions().ok_or_else(|| {
        Error::new(
            "BREP_SOURCE_FIBER_CONTACT",
            "Qualified material regions are required",
        )
    })?;
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        spans: 0,
        driver_cells: 0,
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
            return Ok(out);
        }
    }
    out.reason = "source-fiber-planarity-unproven";
    let Some(plane) = crate::source_allowed_contact::plane(
        regions[faces[0]].loops()[0][0].surface(),
        &mut out.exact_work,
        max_work,
    )?
    else {
        return Ok(out);
    };
    let s = regions[faces[1]].loops()[0][0].surface();
    out.reason = "source-fiber-support-unproven";
    for axis in 0..2 {
        for upper in [false, true] {
            if out.exact_work == max_work {
                return Ok(out);
            }
            let r = source_plane_fiber::certify(s, plane, axis, upper, max_work - out.exact_work)?;
            out.exact_work += r.exact_work;
            let Some(fiber) = r.certificate else {
                continue;
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
            out.reason = "source-fiber-ownership-unproven";
            for (wire, edges) in regions[faces[1]].loops().iter().enumerate() {
                for (edge, fragment) in edges.iter().enumerate() {
                    let address = Address {
                        face: faces[1],
                        wire,
                        edge,
                    };
                    out.uncertain_boundary = Some(address);
                    if out.driver_cells == max_driver_cells {
                        return Ok(out);
                    }
                    let report = source_fiber_boundary::inspect(
                        fragment,
                        &fiber,
                        max_driver_cells - out.driver_cells,
                    )?;
                    out.driver_cells += report.driver_cells;
                    let vertex_owned = |end: usize| {
                        let v = (edge + end) % edges.len();
                        addresses.iter().any(|a| {
                            a.wire == wire && (a.edge == v || (a.edge + 1) % edges.len() == v)
                        })
                    };
                    match report.locus {
                        Locus::EntireFragment if addresses.contains(&address) => {}
                        Locus::Endpoints(ends) if (0..2).all(|i| !ends[i] || vertex_owned(i)) => {}
                        Locus::Away => {}
                        _ => return Ok(out),
                    }
                }
            }
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
            out.reason = "source-fiber-allowed-contact-proven";
            return Ok(out);
        }
    }
    Ok(out)
}
