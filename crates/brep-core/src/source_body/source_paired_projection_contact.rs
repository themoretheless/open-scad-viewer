//! Ordered original graph projections confine contact to owned natural sides.
//! The exact coefficient predicate grants no topology or ownership by itself.
use crate::{
    source_contour_proposal::SourceRegion,
    source_fiber_boundary::{self, Locus},
    source_seam_tangency::Carrier,
    source_shell_incidence::{Address, Shell},
};
use cad_predicates::{AuthoredScalar, Limits, PredicateContext, SourceArena, ToleranceContext};
use nurbs_core::{Error, Result, surface::Surface};
pub struct Certificate<E> {
    faces: [usize; 2],
    regions: [SourceRegion; 2],
    boundaries: [(usize, bool); 2],
    axes: [usize; 2],
    reverse_second: bool,
    edges: Vec<E>,
}
impl<E> Certificate<E> {
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn regions(&self) -> &[SourceRegion; 2] {
        &self.regions
    }
    pub fn boundaries(&self) -> [(usize, bool); 2] {
        self.boundaries
    }
    pub fn projection_axes(&self) -> [usize; 2] {
        self.axes
    }
    pub fn reverse_second(&self) -> bool {
        self.reverse_second
    }
    pub fn edges(&self) -> &[E] {
        &self.edges
    }
}
pub struct Report<E> {
    pub certificate: Option<Certificate<E>>,
    pub exact_work: u64,
    pub driver_cells: usize,
    pub reason: &'static str,
}
pub(crate) fn clamped(s: &Surface) -> bool {
    let f = |p: usize, k: &[f64], n: usize| {
        (1..=8).contains(&p)
            && n == p + 1
            && k.len() == 2 * n
            && k[..n].iter().all(|v| *v == k[0])
            && k[n..].iter().all(|v| *v == k[n])
            && k[0] < k[n]
    };
    !s.periodic_u
        && !s.periodic_v
        && f(s.degree_u, &s.knots_u, s.control_points.len())
        && f(s.degree_v, &s.knots_v, s.control_points[0].len())
        && s.weights.iter().flatten().all(|w| *w > 0.)
        && s.control_points.iter().flatten().all(|p| p.len() == 3)
}
pub(crate) fn boundaries<E: Carrier>(
    shell: &Shell<E>,
    face: usize,
    addresses: &[Address],
    out: &mut Report<E>,
    max: usize,
) -> Result<Option<Vec<(usize, bool)>>> {
    let regions = shell.regions().unwrap();
    let s = regions[face].loops()[0][0].surface();
    let mut candidates = Vec::new();
    for axis in 0..2 {
        for upper in [false, true] {
            let mut owned = true;
            let mut entire = false;
            for (wire, fragments) in regions[face].loops().iter().enumerate() {
                for (edge, fragment) in fragments.iter().enumerate() {
                    if out.driver_cells == max {
                        out.reason = "source-paired-projection-driver-work-limit";
                        return Ok(None);
                    }
                    let r = source_fiber_boundary::inspect_natural(
                        fragment,
                        s,
                        (axis, upper),
                        max - out.driver_cells,
                    )?;
                    out.driver_cells += r.driver_cells;
                    // Literal UV decisions can cost zero primitive cells, but
                    // their complete ownership census is still source work.
                    if out.driver_cells == max {
                        out.reason = "source-paired-projection-driver-work-limit";
                        return Ok(None);
                    }
                    out.driver_cells += 1;
                    let address = Address { face, wire, edge };
                    let vertex_owned = |end| {
                        let vertex = (edge + end) % fragments.len();
                        addresses.iter().any(|a| {
                            a.wire == wire
                                && (a.edge == vertex || (a.edge + 1) % fragments.len() == vertex)
                        })
                    };
                    owned &= match r.locus {
                        Locus::EntireFragment => {
                            entire |= addresses.contains(&address);
                            addresses.contains(&address)
                        }
                        Locus::Endpoints(ends) => (0..2).all(|i| !ends[i] || vertex_owned(i)),
                        Locus::Away => true,
                        _ => false,
                    };
                }
            }
            if owned && entire {
                candidates.push((axis, upper));
            }
        }
    }
    Ok(Some(candidates))
}
/// Fresh exact source premise plus complete material-side ownership on BOTH
/// natural fibers. Equal projected coordinates force both preimages onto those
/// fibers; all their material contacts are already the same native shared edges.
pub fn certify<E: Carrier>(
    shell: &Shell<E>,
    faces: [usize; 2],
    max_work: u64,
    max_driver: usize,
) -> Result<Report<E>> {
    let regions = shell
        .regions()
        .ok_or_else(|| Error::new("BREP_SOURCE_PAIRED_PROJECTION", "Require original regions"))?;
    if faces[0] == faces[1]
        || faces.iter().any(|f| *f >= regions.len())
        || !(1..=100000000).contains(&max_work)
        || max_driver > 100000
    {
        return Err(Error::new(
            "BREP_SOURCE_PAIRED_PROJECTION",
            "Choose distinct faces and bounded original work",
        ));
    }
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        driver_cells: 0,
        reason: "source-paired-projection-unproven",
    };
    let shared = shell
        .uses()
        .iter()
        .enumerate()
        .filter(|(_, p)| p.iter().all(|a| faces.contains(&a.face)))
        .collect::<Vec<_>>();
    if shared.is_empty()
        || max_driver == 0
        || shell.poles().iter().any(|(a, _)| faces.contains(&a.face))
    {
        return Ok(out);
    }
    let surfaces = faces.map(|f| regions[f].loops()[0][0].surface());
    if surfaces.iter().any(|s| !clamped(s)) {
        return Ok(out);
    }
    let mut candidates = Vec::new();
    for face in faces {
        let addresses = shared
            .iter()
            .map(|(_, p)| *p.iter().find(|a| a.face == face).unwrap())
            .collect::<Vec<_>>();
        let Some(c) = boundaries(shell, face, &addresses, &mut out, max_driver)? else {
            return Ok(out);
        };
        candidates.push(c);
    }
    if candidates.iter().any(Vec::is_empty) {
        return Ok(out);
    }
    let values = surfaces
        .iter()
        .flat_map(|s| {
            s.control_points
                .iter()
                .enumerate()
                .flat_map(move |(i, row)| {
                    row.iter()
                        .enumerate()
                        .flat_map(move |(j, p)| [p[0], p[1], p[2], s.weights[i][j]])
                })
        })
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let source = SourceArena::authored("source-paired-graph-projection", 1, values)
        .map_err(|_| Error::new("BREP_SOURCE_PAIRED_PROJECTION", "Invalid original leaves"))?;
    let mut offset = 0;
    let refs = surfaces.map(|s| {
        s.control_points
            .iter()
            .map(|row| {
                row.iter()
                    .map(|_| {
                        std::array::from_fn(|_| {
                            let r = source.leaf(offset).unwrap();
                            offset += 1;
                            r
                        })
                    })
                    .collect()
            })
            .collect::<Vec<_>>()
    });
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: max_work.min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    for &a in &candidates[0] {
        for &b in &candidates[1] {
            for reverse in [false, true] {
                for axes in [[0, 1], [0, 2], [1, 0], [1, 2], [2, 0], [2, 1]] {
                    if ctx.work_used() == max_work {
                        return Ok(out);
                    }
                    let r = cad_predicates::binary_paired_graph_projection(
                        &mut ctx,
                        [&refs[0], &refs[1]],
                        [a.0, b.0],
                        [a.1, b.1],
                        reverse,
                        axes,
                    )
                    .map_err(|_| {
                        Error::new(
                            "BREP_SOURCE_PAIRED_PROJECTION",
                            "Invalid exact projection query",
                        )
                    })?;
                    out.exact_work = ctx.work_used();
                    if r.directions.is_some() {
                        out.certificate = Some(Certificate {
                            faces,
                            regions: [regions[faces[0]].clone(), regions[faces[1]].clone()],
                            boundaries: [a, b],
                            axes,
                            reverse_second: reverse,
                            edges: shared
                                .iter()
                                .map(|(i, _)| shell.edges()[*i].clone())
                                .collect(),
                        });
                        out.reason = "source-paired-graph-projection-qualified";
                        return Ok(out);
                    }
                }
            }
        }
    }
    Ok(out)
}
