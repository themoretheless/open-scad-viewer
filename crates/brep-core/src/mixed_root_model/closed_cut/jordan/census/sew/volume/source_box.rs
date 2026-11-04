//! Bind a complete original NURBS image to a uniform root-solid material box.
use super::*;
use nurbs_core::interval_eval::{evaluate_surface_interval, Interval};
#[derive(Clone, Copy)]
pub struct RootSurfaceMaterialLimits {
    pub source_spans: usize,
    pub material: RootMaterialBoxLimits,
}
impl Default for RootSurfaceMaterialLimits {
    fn default() -> Self {
        Self {
            source_spans: 100000,
            material: Default::default(),
        }
    }
}
pub struct QualifiedRootSurfaceMaterialBox {
    surface: crate::Surface,
    rectangle: [[f64; 2]; 2],
    image: [[f64; 2]; 3],
    spans: usize,
    material: Arc<QualifiedRootMaterialBox>,
    max_spans: usize,
}
impl QualifiedRootSurfaceMaterialBox {
    pub fn surface(&self) -> &crate::Surface {
        &self.surface
    }
    pub fn rectangle(&self) -> [[f64; 2]; 2] {
        self.rectangle
    }
    pub fn image_enclosure(&self) -> [[f64; 2]; 3] {
        self.image
    }
    pub fn source_spans(&self) -> usize {
        self.spans
    }
    pub fn material(&self) -> &Arc<QualifiedRootMaterialBox> {
        &self.material
    }
    pub fn inside(&self) -> bool {
        self.material.inside()
    }
    pub fn reverify(&self) -> Result<()> {
        self.material.reverify()?;
        let fresh = self
            .material
            .qualify_source(&self.surface, self.rectangle, self.max_spans)?;
        if fresh.image != self.image || fresh.spans != self.spans {
            return Err(unproved("Root surface material source enclosure changed"));
        }
        Ok(())
    }
}
fn image(
    surface: &crate::Surface,
    rectangle: [[f64; 2]; 2],
    max_spans: usize,
) -> Result<([[f64; 2]; 3], usize)> {
    surface.validate()?;
    if !(1..=100000).contains(&max_spans)
        || surface
            .control_points
            .iter()
            .flatten()
            .any(|p| p.len() != 3)
    {
        return Err(unproved(
            "Invalid root material source dimension or span allowance",
        ));
    }
    let knots = [&surface.knots_u, &surface.knots_v];
    let degrees = [surface.degree_u, surface.degree_v];
    let counts = [
        surface.control_points.len(),
        surface.control_points[0].len(),
    ];
    let mut spans = [0usize; 2];
    for k in 0..2 {
        let d = rectangle[k];
        if !d.iter().all(|x| x.is_finite())
            || d[0] > d[1]
            || d[0] < knots[k][degrees[k]]
            || knots[k][counts[k]] < d[1]
        {
            return Err(unproved(
                "Root material source rectangle leaves natural support",
            ));
        }
        if [surface.periodic_u, surface.periodic_v][k]
            && nurbs_core::periodic_trim::verify_surface_seam_identity(surface, k)?.is_none()
        {
            return Err(unproved(
                "Root material source has no exact periodic seam identity",
            ));
        }
        spans[k] = (degrees[k]..counts[k])
            .filter(|&i| {
                knots[k][i] < knots[k][i + 1] && knots[k][i] <= d[1] && d[0] <= knots[k][i + 1]
            })
            .count();
    }
    let spans = spans[0]
        .checked_mul(spans[1])
        .filter(|n| *n > 0 && *n <= max_spans)
        .ok_or_else(|| unproved("Root material source span allowance exhausted"))?;
    let bounds = evaluate_surface_interval(
        surface,
        Interval::new(rectangle[0][0], rectangle[0][1])?,
        Interval::new(rectangle[1][0], rectangle[1][1])?,
    )?;
    Ok((std::array::from_fn(|k| [bounds[k].lo, bounds[k].hi]), spans))
}
impl QualifiedRootMaterialBox {
    /// Source identity is owned with its original basis, weights and full UV
    /// restriction. A bound supplied by the caller cannot authorize this owner.
    pub fn qualify_source(
        self: &Arc<Self>,
        surface: &crate::Surface,
        rectangle: [[f64; 2]; 2],
        max_spans: usize,
    ) -> Result<QualifiedRootSurfaceMaterialBox> {
        let (image, spans) = image(surface, rectangle, max_spans)?;
        let bounds = self.bounds();
        if !(0..3).all(|k| bounds[k][0] <= image[k][0] && image[k][1] <= bounds[k][1]) {
            return Err(unproved(
                "Complete original source image leaves certified material enclosure",
            ));
        }
        Ok(QualifiedRootSurfaceMaterialBox {
            surface: surface.clone(),
            rectangle,
            image,
            spans,
            material: self.clone(),
            max_spans,
        })
    }
}
impl QualifiedPairedRootSolid {
    /// Automatically enclose all original source spans, prove uniform material
    /// of the full image enclosure, and retain the exact source binding.
    pub fn qualify_surface_material_box(
        self: &Arc<Self>,
        surface: &crate::Surface,
        rectangle: [[f64; 2]; 2],
        directions: &[[f64; 3]],
        limits: RootSurfaceMaterialLimits,
    ) -> Result<QualifiedRootSurfaceMaterialBox> {
        let (bounds, _) = image(surface, rectangle, limits.source_spans)?;
        let material = Arc::new(self.qualify_material_box(bounds, directions, limits.material)?);
        material.qualify_source(surface, rectangle, limits.source_spans)
    }
}
