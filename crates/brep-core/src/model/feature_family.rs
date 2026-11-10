//! Feature routing hints. Authors still validate geometry and certify their result.
use crate::Model;
use nurbs_core::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstantFilletFamily {
    Annular,
    LayeredPrism,
    SimplePrism,
}
impl ConstantFilletFamily {
    pub fn name(self) -> &'static str {
        match self {
            Self::Annular => "annular",
            Self::LayeredPrism => "layered",
            Self::SimplePrism => "simple",
        }
    }
}
/// Preserve the host's routing convention without executing a feature or
/// claiming that the selected author will admit this geometry.
pub fn constant_fillet_family(model: &Model, edges: &[usize]) -> Result<ConstantFilletFamily> {
    let mut selected = Vec::with_capacity(edges.len());
    for &i in edges {
        selected.push(
            model.edges.get(i).ok_or_else(|| {
                Error::new("BREP_INVALID_INPUT", "Select an existing B-rep edge.")
            })?,
        );
    }
    if selected.iter().any(|e| e.curve.degree > 1) {
        return Ok(ConstantFilletFamily::Annular);
    }
    let Some(edge) = selected.first() else {
        return Ok(ConstantFilletFamily::SimplePrism);
    };
    let point = |i: usize| {
        model
            .vertices
            .get(i)
            .map(|v| v.point)
            .ok_or_else(|| Error::new("BREP_INVALID_INPUT", "Invalid B-rep edge vertex."))
    };
    let origin = point(edge.vertices[0])?;
    let end = point(edge.vertices[1])?;
    let direction: [f64; 3] = std::array::from_fn(|i| end[i] - origin[i]);
    let length = direction[0].hypot(direction[1]).hypot(direction[2]);
    if length == 0. {
        return Ok(ConstantFilletFamily::SimplePrism);
    }
    let levels: Vec<_> = model
        .vertices
        .iter()
        .map(|v| {
            let mut sum = 0.;
            for i in 0..3 {
                sum += (v.point[i] - origin[i]) * direction[i] / length;
            }
            sum
        })
        .collect();
    let low = levels.iter().copied().fold(f64::INFINITY, f64::min);
    let high = levels.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let tolerance = model.tolerance_mm * 16.;
    Ok(
        if levels
            .iter()
            .any(|&z| z > low + tolerance && z < high - tolerance)
        {
            ConstantFilletFamily::LayeredPrism
        } else {
            ConstantFilletFamily::SimplePrism
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routes_primitive_edges_without_mutating_the_model() {
        let box_model = crate::cuboid([0.; 3], [10., 8., 6.]).unwrap();
        let before = box_model.clone();
        assert_eq!(
            constant_fillet_family(&box_model, &[0]).unwrap(),
            ConstantFilletFamily::SimplePrism
        );
        assert_eq!(
            constant_fillet_family(&box_model, &[]).unwrap(),
            ConstantFilletFamily::SimplePrism
        );
        assert!(constant_fillet_family(&box_model, &[box_model.edges.len()]).is_err());
        assert_eq!(box_model, before);
        let tube = crate::tube(10., 5., 6.).unwrap();
        let curved = tube.edges.iter().position(|e| e.curve.degree > 1).unwrap();
        assert_eq!(
            constant_fillet_family(&tube, &[curved]).unwrap(),
            ConstantFilletFamily::Annular
        );
    }
}
