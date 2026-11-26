//! Group disjoint and nested planar section rings into material caps.
use super::{
    Curve, PResult, PlanarSectionCapWithHoles, Refusal, Result, Ring, Work, cap, contact, inside,
    reverse_ring, validate_rings,
};

#[derive(Clone, Debug)]
pub struct PlanarSectionCapRegion {
    /// Input ring indices in the cap's outer-then-holes order.
    pub rings: Vec<usize>,
    pub cap: PlanarSectionCapWithHoles,
}

#[derive(Clone, Debug)]
pub struct PlanarSectionCapArrangement {
    /// Immediate containing ring for each input ring.
    pub parents: Vec<Option<usize>>,
    pub regions: Vec<PlanarSectionCapRegion>,
}

#[derive(Clone, Debug)]
pub struct PlanarSectionCapArrangementReport {
    pub outcome: std::result::Result<PlanarSectionCapArrangement, Refusal>,
    pub work_used: u64,
}

/// Arrange arbitrary nonintersecting section rings using even-odd material
/// membership. An island inside a hole receives a separate cap. Ring order
/// and authored winding do not affect which regions are material.
pub fn arrange_surface_plane_section_caps(
    rings: &[&[Curve]],
    normal: [f64; 3],
    offset: f64,
    boundary_tolerance: f64,
    max_work: u64,
) -> Result<PlanarSectionCapArrangementReport> {
    validate_rings(rings, normal, offset, boundary_tolerance, max_work)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let outcome = arrange(rings, normal, offset, boundary_tolerance, &mut work);
    Ok(PlanarSectionCapArrangementReport {
        outcome,
        work_used: work.used,
    })
}

fn arrange(
    rings: &[&[Curve]],
    normal: [f64; 3],
    offset: f64,
    tolerance: f64,
    work: &mut Work,
) -> PResult<PlanarSectionCapArrangement> {
    let mut caps = Vec::with_capacity(rings.len());
    let mut polygons = Vec::with_capacity(rings.len());
    for edges in rings {
        let face = cap(edges, normal, offset, tolerance, work)?;
        polygons.push(Ring::new(&face.face.loops[0], work)?);
        caps.push(Some(face));
    }
    let containers = containment(&polygons, work)?;
    let depth: Vec<_> = containers.iter().map(Vec::len).collect();
    let parents = containers
        .iter()
        .zip(&depth)
        .map(|(ancestors, &level)| ancestors.iter().copied().find(|&i| depth[i] + 1 == level))
        .collect::<Vec<_>>();
    let mut holes = vec![Vec::new(); rings.len()];
    for (i, &level) in depth.iter().enumerate() {
        if level % 2 == 1 {
            let parent = parents[i].ok_or(Refusal::UnprovedIdentity)?;
            holes[parent].push(i);
        }
    }
    let mut regions = Vec::new();
    for (i, &level) in depth.iter().enumerate() {
        if level % 2 == 1 {
            continue;
        }
        let mut outer = caps[i].take().ok_or(Refusal::InvalidOutput)?;
        let mut reversed = vec![outer.reversed];
        let mut ring_ids = vec![i];
        for &j in &holes[i] {
            let hole = caps[j].take().ok_or(Refusal::InvalidOutput)?;
            let mut contour = hole
                .face
                .loops
                .into_iter()
                .next()
                .ok_or(Refusal::InvalidOutput)?;
            reverse_ring(&mut contour);
            outer.face.loops.push(contour);
            outer.boundary_gap_upper = outer.boundary_gap_upper.max(hole.boundary_gap_upper);
            reversed.push(!hole.reversed);
            ring_ids.push(j);
        }
        regions.push(PlanarSectionCapRegion {
            rings: ring_ids,
            cap: PlanarSectionCapWithHoles {
                face: outer.face,
                reversed,
                boundary_gap_upper: outer.boundary_gap_upper,
            },
        });
    }
    Ok(PlanarSectionCapArrangement { parents, regions })
}

fn containment(polygons: &[Ring], work: &mut Work) -> PResult<Vec<Vec<usize>>> {
    let mut containers = vec![Vec::new(); polygons.len()];
    for (i, a) in polygons.iter().enumerate() {
        for (j, b) in polygons.iter().enumerate().skip(i + 1) {
            if contact(a, b, work)? {
                return Err(Refusal::UnprovedIdentity);
            }
            if inside(a, &b.exact[0], work)? {
                containers[j].push(i);
            } else if inside(b, &a.exact[0], work)? {
                containers[i].push(j);
            }
        }
    }
    Ok(containers)
}
