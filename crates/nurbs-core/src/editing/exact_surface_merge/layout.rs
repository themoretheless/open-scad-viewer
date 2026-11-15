//! Identify a Cartesian UV tiling without changing chart coordinates.
use super::*;
use crate::check;

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceGridLayout {
    pub u_breaks: Vec<f64>,
    pub v_breaks: Vec<f64>,
    /// Input indices in V-major order, with U varying fastest.
    pub order: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridLayoutRefusal {
    ResourceLimit,
    NotTwoDimensional,
    SpansMultipleCells { input: usize },
    DuplicateCell { first: usize, second: usize },
    MissingCells,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceGridLayoutAudit {
    pub layout: Option<SurfaceGridLayout>,
    pub work_used: u64,
    pub refusal: Option<GridLayoutRefusal>,
}

/// Recover the exact rectangular tiling of unordered chart rectangles.
/// Rejects holes, overlaps and T-junctions. The ordered indices can be
/// used for exact tensor merging, which separately verifies seam geometry
/// and basis compatibility. Work charges input chart storage.
pub fn infer_surface_grid_layout(
    patches: &[Surface],
    max_work: u64,
) -> Result<SurfaceGridLayoutAudit> {
    check(
        !patches.is_empty(),
        "A patch layout requires at least one chart",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Patch layout work must be in [1,1000000]",
    )?;
    let mut work_used = 0;
    let result = layout(patches, max_work, &mut work_used)?;
    let (layout, refusal) = match result {
        Ok(layout) => (Some(layout), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(SurfaceGridLayoutAudit {
        layout,
        work_used,
        refusal,
    })
}

fn layout(
    patches: &[Surface],
    max_work: u64,
    used: &mut u64,
) -> Result<std::result::Result<SurfaceGridLayout, GridLayoutRefusal>> {
    use GridLayoutRefusal::*;
    let mut domains = Vec::new();
    let mut breaks = [Vec::new(), Vec::new()];
    for surface in patches {
        surface.validate()?;
        let charge = (surface.control_points.len() * surface.control_points[0].len()) as u64;
        if charge > max_work - *used {
            return Ok(Err(ResourceLimit));
        }
        *used += charge;
        let domain = [
            [
                surface.knots_u[surface.degree_u],
                surface.knots_u[surface.control_points.len()],
            ],
            [
                surface.knots_v[surface.degree_v],
                surface.knots_v[surface.control_points[0].len()],
            ],
        ]
        .map(|axis| axis.map(|k| if k == 0. { 0. } else { k }));
        for (breaks, pair) in breaks.iter_mut().zip(domain) {
            breaks.extend(pair);
        }
        domains.push(domain);
    }
    for axis in &mut breaks {
        axis.sort_by(f64::total_cmp);
        axis.dedup();
    }
    let [u_breaks, v_breaks] = breaks;
    let columns = u_breaks.len() - 1;
    let rows = v_breaks.len() - 1;
    if columns < 2 || rows < 2 {
        return Ok(Err(NotTwoDimensional));
    }
    // Sparse occupancy detects overlap and holes even for huge empty grids.
    let mut occupancy = std::collections::BTreeMap::new();
    for (input, [u, v]) in domains.into_iter().enumerate() {
        let [left, right] = u.map(|x| u_breaks.binary_search_by(|p| p.total_cmp(&x)).unwrap());
        let [lower, upper] = v.map(|x| v_breaks.binary_search_by(|p| p.total_cmp(&x)).unwrap());
        if right != left + 1 || upper != lower + 1 {
            return Ok(Err(SpansMultipleCells { input }));
        }
        if let Some(first) = occupancy.insert((lower, left), input) {
            return Ok(Err(DuplicateCell {
                first,
                second: input,
            }));
        }
    }
    if columns.checked_mul(rows) != Some(occupancy.len()) {
        return Ok(Err(MissingCells));
    }
    Ok(Ok(SurfaceGridLayout {
        u_breaks,
        v_breaks,
        order: occupancy.into_values().collect(),
    }))
}
