//! step v3 export algorithm: periodicized_step_v6.
use super::*;
pub fn periodicized_step_v6(model: &Model) -> Result<Model> {
    if model.faces.len() == 8
        && model.vertices.len() == 6
        && model
            .faces
            .iter()
            .all(|face| face.surface.degree_u == 2 && face.surface.degree_v == 2)
    {
        periodicized_sphere(model)
    } else if model.faces.len() >= 4
        && model.faces[..4].iter().all(|face| {
            face.surface.degree_u == 2
                && face.surface.degree_v == 1
                && face.surface.control_points.len() == 3
        })
    {
        if periodic_patch_grid_compatible(model, 4, 1) {
            merge_periodic_patch_grid(model, &[0, 1, 2, 3], 4, 1)
        } else {
            Ok(model.clone())
        }
    } else if model.faces.len() == 16
        && model.faces.iter().all(|face| {
            face.surface.degree_u == 2
                && face.surface.degree_v == 2
                && face.surface.control_points.len() == 3
                && face.surface.control_points[0].len() == 3
        })
    {
        if periodic_patch_grid_compatible(model, 4, 4) {
            merge_periodic_patch_grid(model, &(0..16).collect::<Vec<_>>(), 4, 4)
        } else {
            Ok(model.clone())
        }
    } else {
        Ok(model.clone())
    }
}
