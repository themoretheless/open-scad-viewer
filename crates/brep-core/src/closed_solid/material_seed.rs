//! Classification of an exact original surface point, including rational values
//! not representable by binary64. A rounded point is only a parity witness;
//! a continuously separated enclosure connects it to the original source point.
use super::*;
use crate::Surface;
use nurbs_core::{
    interval_eval::{self, Interval},
    surface_distance,
};
#[derive(Clone, Copy, Debug)]
pub struct SurfaceMaterialLimits {
    pub source_spans: usize,
    pub distance_cells: usize,
    pub parity_cells: usize,
    pub parity_domain_cells: usize,
}
impl Default for SurfaceMaterialLimits {
    fn default() -> Self {
        Self {
            source_spans: 1024,
            distance_cells: 100000,
            parity_cells: 100000,
            parity_domain_cells: 1000000,
        }
    }
}
#[derive(Clone)]
pub struct QualifiedSurfaceMaterialSeed {
    solid: QualifiedClosedSolid,
    surface: Surface,
    uv: [f64; 2],
    bounds: [[f64; 2]; 3],
    witness: [f64; 3],
    inside: bool,
    distance_cells: usize,
    parity_cells: usize,
    parity_domain_cells: usize,
    directions: Vec<[f64; 3]>,
    limits: SurfaceMaterialLimits,
}
impl QualifiedSurfaceMaterialSeed {
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn parameters(&self) -> [f64; 2] {
        self.uv
    }
    pub fn point_enclosure(&self) -> [[f64; 2]; 3] {
        self.bounds
    }
    pub fn witness(&self) -> [f64; 3] {
        self.witness
    }
    pub fn inside(&self) -> bool {
        self.inside
    }
    pub fn distance_cells(&self) -> usize {
        self.distance_cells
    }
    pub fn parity_work(&self) -> [usize; 2] {
        [self.parity_cells, self.parity_domain_cells]
    }
    pub fn solid(&self) -> &QualifiedClosedSolid {
        &self.solid
    }
    pub fn reverify(&self) -> Result<()> {
        self.solid.reverify()?;
        let f = qualify_surface_material_seed(
            &self.solid,
            &self.surface,
            self.uv,
            &self.directions,
            self.limits,
        )?;
        if f.inside != self.inside || f.bounds != self.bounds || f.witness != self.witness {
            return Err(Error::new(
                "BREP_MATERIAL_SEED_UNPROVED",
                "Original surface material classification changed",
            ));
        }
        Ok(())
    }
}
fn unproved(message: &str) -> Error {
    Error::new("BREP_MATERIAL_SEED_UNPROVED", message)
}
pub fn qualify_surface_material_seed(
    solid: &QualifiedClosedSolid,
    surface: &Surface,
    uv: [f64; 2],
    directions: &[[f64; 3]],
    limits: SurfaceMaterialLimits,
) -> Result<QualifiedSurfaceMaterialSeed> {
    surface.validate()?;
    if !(1..=100000).contains(&limits.source_spans)
        || !(1..=1000000).contains(&limits.distance_cells)
        || !(1..=1000000).contains(&limits.parity_cells)
        || !(1..=8000000).contains(&limits.parity_domain_cells)
        || !uv.iter().all(|v| v.is_finite())
        || surface
            .control_points
            .iter()
            .flatten()
            .any(|p| p.len() != 3)
    {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Material seed requires a spatial original source and bounded positive work",
        ));
    }
    let spans = |knots: &[f64], p: usize, n: usize, x: f64| -> Result<usize> {
        if x < knots[p] || knots[n] < x {
            return Err(unproved(
                "Material seed leaves the original natural support",
            ));
        }
        Ok((p..n)
            .filter(|&i| knots[i] < knots[i + 1] && knots[i] <= x && x <= knots[i + 1])
            .count())
    };
    let u = spans(
        &surface.knots_u,
        surface.degree_u,
        surface.control_points.len(),
        uv[0],
    )?;
    let v = spans(
        &surface.knots_v,
        surface.degree_v,
        surface.control_points[0].len(),
        uv[1],
    )?;
    if u.checked_mul(v)
        .is_none_or(|n| n == 0 || limits.source_spans < n)
    {
        return Err(unproved("Original material seed span allowance exhausted"));
    }
    // Native evaluation admission prevents inventing values at unsupported
    // endpoint aliases; its floating result is never used as exact geometry.
    surface.evaluate(uv[0], uv[1])?;
    let b = interval_eval::evaluate_surface_interval(
        surface,
        Interval::point(uv[0]),
        Interval::point(uv[1]),
    )?;
    let bounds = std::array::from_fn(|k| [b[k].lo, b[k].hi]);
    let witness = std::array::from_fn(|k| b[k].lo * 0.5 + b[k].hi * 0.5);
    let radius = surface_distance::enclosure_distance(&bounds, &witness.map(|x| [x; 2]))?.1;
    let mut cells = 0usize;
    for (i, face) in solid.model.faces.iter().enumerate() {
        let remaining = limits.distance_cells - cells;
        if remaining == 0 {
            return Err(unproved("Original boundary separation work exhausted"));
        }
        let share = (remaining / (solid.model.faces.len() - i)).max(1);
        let distance = surface_distance::nearest_point(
            &face.surface,
            witness,
            solid.model.tolerance_mm,
            share,
        )?;
        cells += distance.cells;
        if !(distance.distance_interval[0] > radius) {
            return Err(unproved(
                "Source point enclosure is not separated from the closed material boundary",
            ));
        }
    }
    let report = solid.classify_point(
        witness,
        directions,
        limits.parity_cells,
        limits.parity_domain_cells,
    )?;
    let inside = report
        .parity
        .ok_or_else(|| unproved("Material witness parity remains unresolved"))?;
    Ok(QualifiedSurfaceMaterialSeed {
        solid: solid.clone(),
        surface: surface.clone(),
        uv,
        bounds,
        witness,
        inside,
        distance_cells: cells,
        parity_cells: report.cells,
        parity_domain_cells: report.domain_cells,
        directions: directions.to_vec(),
        limits,
    })
}
