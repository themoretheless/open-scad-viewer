use gcode_core::{GcodePreview, PlannedLayer, PlannedPath};

use crate::travel::dist;
use crate::{invalid, Result};

pub fn from_gcode(gcode: &str) -> Result<Vec<PlannedLayer>> {
    from_preview(&gcode_core::parse(gcode)?)
}

pub fn from_preview(preview: &GcodePreview) -> Result<Vec<PlannedLayer>> {
    if preview.moves.len() > gcode_core::MAX_MOVES {
        return Err(invalid(
            "OPT_MOVE_LIMIT",
            "Preview has more moves than the optimizer accepts",
        ));
    }
    let mut layers: Vec<PlannedLayer> = Vec::new();
    let mut current_layer: Option<usize> = None;
    let mut path: Vec<[f64; 2]> = Vec::new();
    let mut previous: Option<[f64; 2]> = None;
    for mv in &preview.moves {
        if current_layer != Some(mv.layer_index) {
            flush_path(&mut layers, &mut path);
            current_layer = Some(mv.layer_index);
            while layers.len() <= mv.layer_index {
                layers.push(PlannedLayer {
                    z_mm: mv.z,
                    paths: Vec::new(),
                });
            }
            layers[mv.layer_index].z_mm = mv.z;
        }
        let xy = [mv.x, mv.y];
        if mv.extruded {
            if path.is_empty()
                && let Some(start) = previous
            {
                path.push(start);
            }
            if path.last() != Some(&xy) {
                path.push(xy);
            }
        } else {
            flush_path(&mut layers, &mut path);
        }
        previous = Some(xy);
    }
    flush_path(&mut layers, &mut path);
    layers.retain(|layer| !layer.paths.is_empty());
    Ok(layers)
}

fn flush_path(layers: &mut [PlannedLayer], path: &mut Vec<[f64; 2]>) {
    if path.len() < 2 {
        path.clear();
        return;
    }
    let mut closed = false;
    if path.len() >= 4 && dist(path[0], path[path.len() - 1]) <= 1e-5 {
        path.pop();
        closed = path.len() >= 3;
    }
    if let Some(layer) = layers.last_mut() {
        layer.paths.push(PlannedPath {
            points: std::mem::take(path),
            closed,
        });
    } else {
        path.clear();
    }
}
