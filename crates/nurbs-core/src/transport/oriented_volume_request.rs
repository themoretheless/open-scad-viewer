use super::*;
use crate::analysis::surface_volume;
use crate::surface::Surface;

pub(super) fn volume(v: &Value) -> Result<Value> {
    let surfaces: Vec<Surface> = field(v, "surfaces")?;
    let reversed: Vec<usize> = field(v, "reversed_faces")?;
    let mut flags = vec![false; surfaces.len()];
    for index in reversed {
        let flag = flags
            .get_mut(index)
            .ok_or_else(|| input("Reversed face index outside shell"))?;
        check(!*flag, "Duplicate reversed face index")?;
        *flag = true;
    }
    let axis = if v.get("flux_axis").is_some() {
        Some(field(v, "flux_axis")?)
    } else {
        None
    };
    let patches = surfaces.iter().zip(flags).collect::<Vec<_>>();
    let r = surface_volume::enclosed_volume_oriented(
        &patches,
        axis,
        field(v, "tolerance_mm3")?,
        field(v, "max_cells")?,
    )?;
    Ok(
        json!({"signedVolumeMm3":r.value,"boundsMm3":r.bounds,"errorUpperMm3":r.error_upper,"withinTolerance":r.within_tolerance,
      "cells":r.cells,"stopReason":format!("{:?}",r.stop_reason),"shellClosure":"caller-supplied","faceCount":patches.len()}),
    )
}
