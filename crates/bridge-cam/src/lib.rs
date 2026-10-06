//! G-code toolpaths/export and laser CAM request handlers.
//!
//! Routed by `geometry-bridge`: [`dispatch`] hands back operations this
//! domain does not own.

pub(crate) use bridge_codec::{Error, Result, Routed, Value, field, input, require_exact_fields};
pub(crate) use polygon_core::Mesh;

pub mod gcode;
pub mod laser;

/// Handles the CAM operations; other operations are handed back.
pub fn dispatch(v: Value) -> Routed {
    Routed::Handled(match bridge_codec::op(&v) {
        "mesh_toolpaths" => gcode::toolpaths(&v),
        "mesh_gcode" => gcode::export(&v),
        "mesh_gcode_job" => gcode::export_job(&v),
        "gcode_preview" => gcode::parse(&v),
        "gcode_parse" => gcode::inspect(&v),
        "laser_preflight" | "laser_frame_preview" | "laser_grbl" | "laser_frame" => {
            laser::dispatch(&v)
        }
        _ => return Routed::Unhandled(v),
    })
}
