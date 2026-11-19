use super::{Contour, outset_rings};
use crate::{Budget, PathRole, Result, Toolpath, infill::hatch_angled, invalid, valid_coordinate};
use planar_geometry::rings;

type RaftSegments = Vec<[[f64; 3]; 2]>;

/// Bed-adhesion raft configuration with coarse base layer and fine orthogonal interface layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaftConfig {
    pub margin_mm: f64,
    pub base_spacing_mm: f64,
    pub interface_spacing_mm: f64,
    pub base_z_mm: f64,
    pub interface_z_mm: f64,
}

impl Default for RaftConfig {
    fn default() -> Self {
        Self {
            margin_mm: 5.0,
            base_spacing_mm: 2.0,
            interface_spacing_mm: 0.6,
            base_z_mm: 0.3,
            interface_z_mm: 0.5,
        }
    }
}

/// Generates a two-layer orthogonal adhesion raft beneath `outer_loops` expanded by `margin_mm`.
/// Returns `(base_segments_3d, interface_segments_3d)`, where `base_segments_3d` are horizontal
/// ($0^\circ$) hatch segments at `base_z` with pitch `base_spacing_mm`, and `interface_segments_3d`
/// are orthogonal ($90^\circ$) hatch segments at `interface_z` with pitch `interface_spacing_mm`.
pub fn generate_raft(
    outer_loops: &[Contour],
    margin_mm: f64,
    base_spacing_mm: f64,
    interface_spacing_mm: f64,
    base_z: f64,
    interface_z: f64,
) -> Result<(RaftSegments, RaftSegments)> {
    if !margin_mm.is_finite()
        || margin_mm < 0.0
        || !base_spacing_mm.is_finite()
        || base_spacing_mm < gcode_core::COORDINATE_RESOLUTION_MM
        || !interface_spacing_mm.is_finite()
        || interface_spacing_mm < gcode_core::COORDINATE_RESOLUTION_MM
        || !valid_coordinate(base_z)
        || !valid_coordinate(interface_z)
        || interface_z <= base_z
    {
        return Err(invalid(
            "TOOLPATH_INVALID_SETTINGS",
            "Raft margin, spacings, and Z heights (interface_z > base_z) must be valid",
        ));
    }
    if outer_loops.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut budget = Budget::default();
    let norm = rings::nonzero(&outer_loops.to_vec())?;
    let footprint = if margin_mm > 1e-6 {
        outset_rings(&norm, margin_mm, &mut budget)?
    } else {
        norm
    };
    let base_paths = hatch_angled(&footprint, base_spacing_mm, 0.0, PathRole::Support, &mut budget)?;
    let interface_paths = hatch_angled(
        &footprint,
        interface_spacing_mm,
        std::f64::consts::FRAC_PI_2,
        PathRole::Support,
        &mut budget,
    )?;
    let to_3d = |paths: Vec<Toolpath>, z: f64| -> Vec<[[f64; 3]; 2]> {
        let mut segs = Vec::with_capacity(paths.len());
        for p in paths {
            if let (Some(&a), Some(&b)) = (p.points.first(), p.points.last())
                && p.points.len() >= 2
            {
                segs.push([[a[0], a[1], z], [b[0], b[1], z]]);
            }
        }
        segs
    };
    Ok((to_3d(base_paths, base_z), to_3d(interface_paths, interface_z)))
}
