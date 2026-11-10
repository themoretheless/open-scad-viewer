use super::*;
use std::collections::BTreeSet;
fn edges(m: &Model, face: usize) -> BTreeSet<usize> {
    m.loops[m.faces[face].outer]
        .coedges
        .iter()
        .map(|c| c.edge)
        .collect()
}
pub(super) fn layout(m: &Model, roof: usize, floor: usize, work: &mut Work) -> Result<()> {
    work.charge(1)?;
    require(
        m.bodies.len() == 1
            && m.shells.len() == 1
            && m.faces.len() == 6
            && m.loops.len() == 6
            && m.edges.len() == 12
            && m.vertices.len() == 8
            && m.bodies[0].outer_shell == 0
            && m.bodies[0].inner_shells.is_empty()
            && roof < 6
            && floor < 6
            && roof != floor,
        "Need one original six-face eight-vertex graph column without cavities",
    )?;
    let actual: BTreeSet<_> = m.shells[0].faces.iter().map(|u| u.face).collect();
    require(
        m.shells[0].faces.len() == 6 && actual == (0..6).collect(),
        "Actual shell must own each source face once",
    )?;
    require(
        m.faces.iter().all(|f| f.holes.is_empty() && f.outer < 6)
            && m.faces
                .iter()
                .map(|f| f.outer)
                .collect::<BTreeSet<_>>()
                .len()
                == 6,
        "Need six distinct unholed source loops",
    )?;
    require(
        m.loops
            .iter()
            .all(|l| l.coedges.len() == 4 && l.coedges.iter().all(|c| c.trim().is_none())),
        "Need complete four-edge source charts without retained trim clocks",
    )?;
    m.validate()?;
    let mut referenced = BTreeSet::new();
    for edge in &m.edges {
        work.charge(1)?;
        require(
            !edge.degenerate,
            "Graph column has no contracted source edges",
        )?;
        let c = &edge.curve;
        let d = c.domain();
        let n = c.control_points.len();
        require(
            !c.periodic
                && c.knots[..=c.degree].iter().all(|&t| t == d[0])
                && c.knots[n..].iter().all(|&t| t == d[1]),
            "Original graph edges must retain exact clamped endpoints",
        )?;
        for (vertex, pole) in edge
            .vertices
            .iter()
            .zip([&c.control_points[0], &c.control_points[n - 1]])
        {
            work.charge(1)?;
            require(
                pole.as_slice() == m.vertices[*vertex].point.as_slice(),
                "Actual vertex disagrees with exact original endpoint pole",
            )?;
            referenced.insert(*vertex);
        }
    }
    require(
        referenced.len() == 8,
        "Every original graph vertex must be referenced",
    )?;
    for a in 0..8 {
        for b in a + 1..8 {
            work.charge(1)?;
            require(
                m.vertices[a].point != m.vertices[b].point,
                "Graph corner vertices must be literally distinct",
            )?;
        }
    }
    Ok(())
}
pub(super) fn walls(
    m: &Model,
    roof: usize,
    floor: usize,
    xy: [[f64; 2]; 2],
    work: &mut Work,
) -> Result<[usize; 4]> {
    let top = edges(m, roof);
    let bottom = edges(m, floor);
    require(
        top.len() == 4 && bottom.len() == 4 && top.is_disjoint(&bottom),
        "Roof/floor boundaries must be distinct complete cycles",
    )?;
    let mut walls = [usize::MAX; 4];
    for face in 0..6 {
        if face == roof || face == floor {
            continue;
        }
        let boundary = edges(m, face);
        require(
            boundary.len() == 4
                && boundary.intersection(&top).count() == 1
                && boundary.intersection(&bottom).count() == 1,
            "Every wall must share one original edge with roof and floor",
        )?;
        let surface = &m.faces[face].surface;
        surface.validate()?;
        let mut candidates = Vec::new();
        for side in 0..4 {
            let axis = side / 2;
            let value = xy[axis][side % 2];
            let mut on = true;
            for p in surface.control_points.iter().flatten() {
                work.charge(1)?;
                if p[axis] != value {
                    on = false;
                }
            }
            if on {
                candidates.push(side);
            }
        }
        require(
            candidates.len() == 1,
            "Original wall coefficients do not prove one exact footprint plane",
        )?;
        let side = candidates[0];
        require(
            walls[side] == usize::MAX,
            "Footprint wall plane has multiple source owners",
        )?;
        walls[side] = face;
    }
    require(
        walls.iter().all(|&f| f != usize::MAX),
        "Every footprint side needs an original wall",
    )?;
    Ok(walls)
}
pub(super) fn embedding(
    l: &crate::source_shell_geometry::Limits,
) -> crate::source_shell_geometry::Limits {
    crate::source_shell_geometry::Limits {
        tolerance_uv: l.tolerance_uv,
        corners: l.corners,
        spans: l.spans,
        linear_cells: l.linear_cells,
        pairs: l.pairs,
        exact_work: l.exact_work,
        driver_cells: l.driver_cells,
    }
}
pub(super) fn volume(l: &crate::source_volume::Limits) -> crate::source_volume::Limits {
    crate::source_volume::Limits {
        axis: l.axis,
        origin: l.origin,
        absolute_error: l.absolute_error,
        tolerance_uv: l.tolerance_uv,
        cells: l.cells,
        spans: l.spans,
        domain_cells: l.domain_cells,
    }
}
