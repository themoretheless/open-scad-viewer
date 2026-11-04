//! Uniform material of the entire original NURBS image of a closed UV box.
use super::material_seed::SurfaceMaterialLimits;
use super::*;
use nurbs_core::interval_eval::{self, Interval};
#[derive(Clone, Copy, Debug)]
pub struct SurfaceMaterialBoxLimits {
    pub source: SurfaceMaterialLimits,
    pub distance_domain_cells: usize,
}
impl Default for SurfaceMaterialBoxLimits {
    fn default() -> Self {
        Self {
            source: Default::default(),
            distance_domain_cells: 1000000,
        }
    }
}
#[derive(Clone)]
pub struct QualifiedSurfaceMaterialBox {
    solid: QualifiedClosedSolid,
    surface: crate::Surface,
    rectangle: [[f64; 2]; 2],
    enclosure: [[f64; 2]; 3],
    inside: bool,
    witness: [f64; 3],
    distance_cells: usize,
    domain_cells: usize,
    parity: [usize; 2],
    directions: Vec<[f64; 3]>,
    limits: SurfaceMaterialBoxLimits,
}
impl QualifiedSurfaceMaterialBox {
    pub fn inside(&self) -> bool {
        self.inside
    }
    pub fn rectangle(&self) -> [[f64; 2]; 2] {
        self.rectangle
    }
    pub fn surface(&self) -> &crate::Surface {
        &self.surface
    }
    pub fn solid(&self) -> &QualifiedClosedSolid {
        &self.solid
    }
    pub fn image_enclosure(&self) -> [[f64; 2]; 3] {
        self.enclosure
    }
    pub fn witness(&self) -> [f64; 3] {
        self.witness
    }
    pub fn separation_work(&self) -> [usize; 2] {
        [self.distance_cells, self.domain_cells]
    }
    pub fn parity_work(&self) -> [usize; 2] {
        self.parity
    }
    pub fn reverify(&self) -> Result<()> {
        self.solid.reverify()?;
        let f = qualify_surface_material_box(
            &self.solid,
            &self.surface,
            self.rectangle,
            &self.directions,
            self.limits,
        )?;
        if f.inside != self.inside || f.enclosure != self.enclosure || f.witness != self.witness {
            return Err(unproved("Original NURBS material box changed"));
        }
        Ok(())
    }
}
fn unproved(s: &str) -> Error {
    Error::new("BREP_MATERIAL_BOX_UNPROVED", s)
}
pub fn qualify_surface_material_box(
    solid: &QualifiedClosedSolid,
    surface: &crate::Surface,
    rectangle: [[f64; 2]; 2],
    directions: &[[f64; 3]],
    limits: SurfaceMaterialBoxLimits,
) -> Result<QualifiedSurfaceMaterialBox> {
    surface.validate()?;
    let l = limits.source;
    if !(1..=100000).contains(&l.source_spans)
        || !(1..=1000000).contains(&l.distance_cells)
        || !(1..=1000000).contains(&l.parity_cells)
        || !(1..=8000000).contains(&l.parity_domain_cells)
        || !(1..=8000000).contains(&limits.distance_domain_cells)
        || surface
            .control_points
            .iter()
            .flatten()
            .any(|p| p.len() != 3)
    {
        return Err(unproved(
            "Invalid original surface material box work or spatial source",
        ));
    }
    let knots = [&surface.knots_u, &surface.knots_v];
    let degrees = [surface.degree_u, surface.degree_v];
    let counts = [
        surface.control_points.len(),
        surface.control_points[0].len(),
    ];
    let periodic = [surface.periodic_u, surface.periodic_v];
    let mut spans = [0usize; 2];
    for k in 0..2 {
        let d = rectangle[k];
        let low = knots[k][degrees[k]];
        let high = knots[k][counts[k]];
        if !d.iter().all(|x| x.is_finite()) || d[0] > d[1] || d[0] < low || high < d[1] {
            return Err(unproved("Material box leaves its original natural support"));
        }
        if periodic[k]
            && nurbs_core::periodic_trim::verify_surface_seam_identity(surface, k)?.is_none()
        {
            return Err(unproved(
                "Original periodic material box lacks native seam identity",
            ));
        }
        spans[k] = (degrees[k]..counts[k])
            .filter(|&i| {
                knots[k][i] < knots[k][i + 1] && knots[k][i] <= d[1] && d[0] <= knots[k][i + 1]
            })
            .count();
    }
    if spans[0]
        .checked_mul(spans[1])
        .is_none_or(|n| n == 0 || l.source_spans < n)
    {
        return Err(unproved("Original material box span allowance exhausted"));
    }
    for corner in 0..4 {
        let p: [f64; 2] = std::array::from_fn(|k| {
            let x = rectangle[k][(corner >> k) & 1];
            if periodic[k] && x == knots[k][counts[k]] {
                knots[k][degrees[k]]
            } else {
                x
            }
        });
        surface.evaluate(p[0], p[1])?;
    }
    let bounds = interval_eval::evaluate_surface_interval(
        surface,
        Interval::new(rectangle[0][0], rectangle[0][1])?,
        Interval::new(rectangle[1][0], rectangle[1][1])?,
    )?;
    let enclosure = std::array::from_fn(|k| [bounds[k].lo, bounds[k].hi]);
    let material = super::separation::qualify(
        solid,
        enclosure,
        directions,
        l,
        limits.distance_domain_cells,
    )?;
    Ok(QualifiedSurfaceMaterialBox {
        solid: solid.clone(),
        surface: surface.clone(),
        rectangle,
        enclosure,
        inside: material.inside,
        witness: material.witness,
        distance_cells: material.distance_cells,
        domain_cells: material.domain_cells,
        parity: [material.parity_cells, material.parity_domain_cells],
        directions: directions.to_vec(),
        limits,
    })
}
