//! Derive one owned cap rim from selected face sectors; no proximity welding.
use super::*;
pub struct SurfaceCap {
    pub replacement: Replacement,
    pub interior: nurbs_core::surface_linear_monotonicity::Report,
    pub cells: usize,
}
/// Replace a connected selected region with one injective authored chart.
/// Boundary edges and orientation come from source incidence. Projection and
/// chord area only propose pcurves/orientation; final original-boundary, trim
/// and closure admission is performed by `replace`. No radius/G2/global proof.
pub fn cap_region(
    source: &Model,
    selected: &[usize],
    surface: &Surface,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<SurfaceCap> {
    source.validate()?;
    surface.validate()?;
    if !(1..=256).contains(&selected.len())
        || !(8..=100000).contains(&max_cells)
        || max_exact > 1000000
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || selected.iter().any(|&f| f >= source.faces.len())
        || selected.iter().copied().collect::<BTreeSet<_>>().len() != selected.len()
    {
        return Err(refused(
            "Need distinct original sectors and bounded cap work",
        ));
    }
    let interior =
        nurbs_core::surface_linear_monotonicity::inspect_candidate(surface, max_cells / 4)?;
    let mut cells = interior.cells;
    if !interior.certified {
        return Err(refused("Cap chart injectivity remains unresolved"));
    }
    let mut incidence: BTreeMap<usize, Vec<bool>> = BTreeMap::new();
    for &face in selected {
        let uses = source
            .shells
            .iter()
            .flat_map(|s| &s.faces)
            .filter(|u| u.face == face)
            .collect::<Vec<_>>();
        if uses.len() != 1 {
            return Err(refused("Selected cap sector needs unique shell ownership"));
        }
        let f = &source.faces[face];
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for c in &source.loops[wire].coedges {
                incidence
                    .entry(c.edge)
                    .or_default()
                    .push(c.reversed ^ uses[0].reversed);
            }
        }
    }
    let mut rim = BTreeMap::new();
    for (edge, uses) in incidence {
        match uses.as_slice() {
            [r] => {
                rim.insert(edge, *r);
            }
            [a, b] if a != b => {}
            _ => return Err(refused("Cap region incidence is not an oriented manifold")),
        }
    }
    if !(3..=256).contains(&rim.len()) {
        return Err(refused("Cap needs a bounded single outer rim"));
    }
    let first = *rim.keys().next().unwrap();
    let start = source.edges[first].vertices[usize::from(rim[&first])];
    let mut vertex = start;
    let mut ring = Vec::new();
    let mut owned_uv = BTreeMap::<usize, Vec<f64>>::new();
    while !rim.is_empty() {
        let next = rim
            .iter()
            .filter(|(edge, r)| source.edges[**edge].vertices[usize::from(**r)] == vertex)
            .map(|(&e, &r)| (e, r))
            .collect::<Vec<_>>();
        if next.len() != 1 {
            return Err(refused("Cap rim has ambiguous or disconnected ownership"));
        }
        let (edge, reversed) = next[0];
        rim.remove(&edge);
        if cells == max_cells {
            return Err(refused("Shared cap projection work exhausted"));
        }
        let projected = nurbs_core::curve_pullback::project(
            &source.edges[edge].curve,
            surface,
            nurbs_core::curve_pullback::Limits {
                tolerance: source.tolerance_mm / 2.,
                max_segments: 256,
                max_iterations: 32,
                agreement_cells: max_cells - cells,
            },
        )?;
        cells += projected.agreement_cells;
        if !projected.within_tolerance {
            return Err(refused("Complete owned cap edge misses the authored chart"));
        }
        let mut pc = projected
            .pcurve
            .ok_or_else(|| refused("Missing saved cap pcurve"))?;
        if pc.periodic
            || !pc.knots[..=pc.degree].iter().all(|&k| k == pc.domain()[0])
            || !pc.knots[pc.control_points.len()..]
                .iter()
                .all(|&k| k == pc.domain()[1])
        {
            return Err(refused(
                "Cap endpoint binding needs original clamped pcurves",
            ));
        }
        // One proposal per actual owned vertex, shared by all incident uses.
        // No world-distance welding and no inherited projection certificate.
        for end in 0..2 {
            let pole = if end == 0 {
                0
            } else {
                pc.control_points.len() - 1
            };
            let vertex = source.edges[edge].vertices[end];
            let uv = owned_uv
                .entry(vertex)
                .or_insert_with(|| pc.control_points[pole].clone());
            pc.control_points[pole] = uv.clone();
        }
        if cells == max_cells {
            return Err(refused("Bound cap pcurve work exhausted"));
        }
        let bound = nurbs_core::curve_surface_agreement::verify(
            &source.edges[edge].curve,
            &pc,
            surface,
            false,
            source.tolerance_mm / 2.,
            max_cells - cells,
        )?;
        cells += bound.cells;
        if bound.status != nurbs_core::curve_surface_agreement::Status::WithinTolerance {
            return Err(refused("Owned UV endpoint edits fail the whole cap lift"));
        }
        ring.push(Coedge::new(
            edge,
            reversed,
            if reversed { pc.reverse()? } else { pc },
        ));
        vertex = source.edges[edge].vertices[usize::from(!reversed)];
        if vertex == start && !rim.is_empty() {
            return Err(refused("Multiple cap rims need separate hole ownership"));
        }
    }
    if vertex != start {
        return Err(refused("Cap rim is not closed by source identity"));
    }
    // Orientation proposal only. Curved-loop simplicity and winding are freshly
    // proven on the actual saved pcurves by the replacement admission below.
    let mut area = 0.;
    for i in 0..ring.len() {
        let a = &ring[i].pcurve.control_points[0];
        let b = &ring[(i + 1) % ring.len()].pcurve.control_points[0];
        area += a[0] * b[1] - a[1] * b[0];
    }
    if !area.is_finite() || area == 0. {
        return Err(refused("Cap orientation proposal is degenerate"));
    }
    let reversed = area < 0.;
    if reversed {
        ring.reverse();
        for c in &mut ring {
            c.reversed = !c.reversed;
            c.pcurve = c.pcurve.reverse()?;
        }
    }
    if max_cells - cells < 3 {
        return Err(refused("No remaining closed cap admission work"));
    }
    let replacement = super::replace(
        source,
        selected,
        surface,
        &[ring],
        reversed,
        tolerance_uv,
        max_cells - cells,
        max_exact,
    )?;
    cells += replacement.cells;
    Ok(SurfaceCap {
        replacement,
        interior,
        cells,
    })
}
