//! Standard mechanical drive profiles (racks, timing belt pulleys) and ISO/DIN fasteners & shaft features.
use crate::{Point, Result, err};
use std::f64::consts::{PI, TAU};

#[derive(Debug, Clone)]
pub struct RackOptions {
    pub teeth: usize,
    pub module: f64,
    pub pressure_angle: f64,
    pub base_height: f64,
    pub clearance: f64,
}

impl Default for RackOptions {
    fn default() -> Self {
        Self {
            teeth: 10,
            module: 1.0,
            pressure_angle: 20.0,
            base_height: 4.0,
            clearance: 0.25,
        }
    }
}

/// Generates a closed 2D profile loop for an involute linear gear rack along the X axis.
pub fn involute_rack_profile(o: &RackOptions) -> Result<Vec<Point>> {
    if !(1..=256).contains(&o.teeth) {
        return Err(err("rack", "Rack teeth must be from 1 to 256."));
    }
    if !o.module.is_finite() || !(0.1..=100.0).contains(&o.module) {
        return Err(err("rack", "Rack module must be from 0.1 to 100 mm."));
    }
    if !o.pressure_angle.is_finite() || !(14.5..=30.0).contains(&o.pressure_angle) {
        return Err(err(
            "rack",
            "Rack pressure_angle must be from 14.5 to 30 degrees.",
        ));
    }
    if !o.base_height.is_finite() || !(0.5..=1000.0).contains(&o.base_height) {
        return Err(err("rack", "Rack base_height must be from 0.5 to 1000 mm."));
    }
    if !o.clearance.is_finite() || !(0.0..=o.module).contains(&o.clearance) {
        return Err(err("rack", "Rack clearance must be from 0 to one module."));
    }
    let pitch = PI * o.module;
    let alpha = o.pressure_angle * PI / 180.0;
    let tan_a = alpha.tan();
    let addendum = o.module;
    let dedendum = o.module + o.clearance;
    let half_tooth_pitch = pitch * 0.25;
    let dx_add = addendum * tan_a;
    let dx_ded = dedendum * tan_a;
    if half_tooth_pitch <= dx_add + 1e-6 {
        return Err(err("rack", "Rack tooth tip collapses."));
    }
    let total_length = o.teeth as f64 * pitch;
    let y_bottom = -dedendum - o.base_height;
    let mut loop_pts = Vec::with_capacity(o.teeth * 4 + 4);
    loop_pts.push([0.0, y_bottom]);
    loop_pts.push([total_length, y_bottom]);
    for i in (0..o.teeth).rev() {
        let x0 = i as f64 * pitch;
        let center = x0 + pitch * 0.5;
        let x_right_root = center + half_tooth_pitch + dx_ded;
        let x_right_tip = center + half_tooth_pitch - dx_add;
        let x_left_tip = center - half_tooth_pitch + dx_add;
        let x_left_root = center - half_tooth_pitch - dx_ded;
        loop_pts.push([x_right_root.min(x0 + pitch), -dedendum]);
        loop_pts.push([x_right_tip, addendum]);
        loop_pts.push([x_left_tip, addendum]);
        loop_pts.push([x_left_root.max(x0), -dedendum]);
    }
    Ok(loop_pts)
}

#[derive(Debug, Clone)]
pub struct TimingPulleyOptions {
    pub teeth: usize,
    pub pitch_mm: f64,
    pub tooth_depth_mm: f64,
    pub bore_mm: f64,
    pub segments_per_tooth: usize,
}

impl Default for TimingPulleyOptions {
    fn default() -> Self {
        // Standard GT2-2M pulley defaults
        Self {
            teeth: 20,
            pitch_mm: 2.0,
            tooth_depth_mm: 0.75,
            bore_mm: 5.0,
            segments_per_tooth: 8,
        }
    }
}

/// Generates outer and optional inner bore 2D profile loops for a synchronous timing pulley (e.g. GT2/HTD).
pub fn timing_pulley_profile(o: &TimingPulleyOptions) -> Result<Vec<Vec<Point>>> {
    if !(8..=256).contains(&o.teeth) {
        return Err(err("pulley", "Pulley teeth must be from 8 to 256."));
    }
    if !o.pitch_mm.is_finite() || !(1.0..=50.0).contains(&o.pitch_mm) {
        return Err(err("pulley", "Pulley pitch_mm must be from 1 to 50 mm."));
    }
    if !o.tooth_depth_mm.is_finite()
        || o.tooth_depth_mm <= 0.05
        || o.tooth_depth_mm >= o.pitch_mm * 0.8
    {
        return Err(err(
            "pulley",
            "Pulley tooth_depth_mm must be positive and less than 0.8 * pitch.",
        ));
    }
    if !(4..=24).contains(&o.segments_per_tooth) {
        return Err(err("pulley", "segments_per_tooth must be from 4 to 24."));
    }
    let outer_radius = (o.teeth as f64 * o.pitch_mm) / TAU;
    let root_radius = outer_radius - o.tooth_depth_mm;
    if !o.bore_mm.is_finite() || o.bore_mm < 0.0 || o.bore_mm >= 2.0 * root_radius - 0.5 {
        return Err(err(
            "pulley",
            "Pulley bore_mm must leave at least 0.5 mm wall inside root circle.",
        ));
    }
    let total_steps = o.teeth * o.segments_per_tooth;
    let mut outer = Vec::with_capacity(total_steps);
    for i in 0..total_steps {
        let theta = TAU * (i as f64) / (total_steps as f64);
        let phase = (i % o.segments_per_tooth) as f64 / (o.segments_per_tooth as f64);
        // Smooth cosine groove profile matching GT2/HTD rounded tooth pockets
        let groove = if phase < 0.6 {
            let u = phase / 0.6;
            0.5 * (1.0 - (TAU * u).cos())
        } else {
            0.0
        };
        let r = outer_radius - o.tooth_depth_mm * groove;
        outer.push([r * theta.cos(), r * theta.sin()]);
    }
    let mut loops = vec![outer];
    if o.bore_mm > 0.0 {
        let r_bore = o.bore_mm * 0.5;
        let bore_steps = 32;
        let bore: Vec<Point> = (0..bore_steps)
            .rev()
            .map(|i| {
                let a = TAU * (i as f64) / (bore_steps as f64);
                [r_bore * a.cos(), r_bore * a.sin()]
            })
            .collect();
        loops.push(bore);
    }
    Ok(loops)
}

/// ISO 4032 / ISO 4762 metric fastener specification in millimeters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricFastenerSpec {
    pub nominal_diameter_mm: f64,
    pub coarse_pitch_mm: f64,
    pub hex_width_across_flats_mm: f64,
    pub hex_head_height_mm: f64,
    pub nut_height_mm: f64,
    pub socket_head_diameter_mm: f64,
    pub socket_head_height_mm: f64,
    pub socket_key_across_flats_mm: f64,
    pub clearance_hole_close_mm: f64,
    pub clearance_hole_medium_mm: f64,
}

/// Looks up standard ISO metric fastener dimensions for M2, M2.5, M3, M4, M5, M6, M8, M10, M12, M16, M20.
pub fn iso_metric_fastener(nominal_mm: f64) -> Result<MetricFastenerSpec> {
    let table: &[(f64, MetricFastenerSpec)] = &[
        (
            2.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 2.0,
                coarse_pitch_mm: 0.4,
                hex_width_across_flats_mm: 4.0,
                hex_head_height_mm: 1.4,
                nut_height_mm: 1.6,
                socket_head_diameter_mm: 3.8,
                socket_head_height_mm: 2.0,
                socket_key_across_flats_mm: 1.5,
                clearance_hole_close_mm: 2.2,
                clearance_hole_medium_mm: 2.4,
            },
        ),
        (
            2.5,
            MetricFastenerSpec {
                nominal_diameter_mm: 2.5,
                coarse_pitch_mm: 0.45,
                hex_width_across_flats_mm: 5.0,
                hex_head_height_mm: 1.7,
                nut_height_mm: 2.0,
                socket_head_diameter_mm: 4.5,
                socket_head_height_mm: 2.5,
                socket_key_across_flats_mm: 2.0,
                clearance_hole_close_mm: 2.7,
                clearance_hole_medium_mm: 2.9,
            },
        ),
        (
            3.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 3.0,
                coarse_pitch_mm: 0.5,
                hex_width_across_flats_mm: 5.5,
                hex_head_height_mm: 2.0,
                nut_height_mm: 2.4,
                socket_head_diameter_mm: 5.5,
                socket_head_height_mm: 3.0,
                socket_key_across_flats_mm: 2.5,
                clearance_hole_close_mm: 3.2,
                clearance_hole_medium_mm: 3.4,
            },
        ),
        (
            4.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 4.0,
                coarse_pitch_mm: 0.7,
                hex_width_across_flats_mm: 7.0,
                hex_head_height_mm: 2.8,
                nut_height_mm: 3.2,
                socket_head_diameter_mm: 7.0,
                socket_head_height_mm: 4.0,
                socket_key_across_flats_mm: 3.0,
                clearance_hole_close_mm: 4.3,
                clearance_hole_medium_mm: 4.5,
            },
        ),
        (
            5.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 5.0,
                coarse_pitch_mm: 0.8,
                hex_width_across_flats_mm: 8.0,
                hex_head_height_mm: 3.5,
                nut_height_mm: 4.7,
                socket_head_diameter_mm: 8.5,
                socket_head_height_mm: 5.0,
                socket_key_across_flats_mm: 4.0,
                clearance_hole_close_mm: 5.3,
                clearance_hole_medium_mm: 5.5,
            },
        ),
        (
            6.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 6.0,
                coarse_pitch_mm: 1.0,
                hex_width_across_flats_mm: 10.0,
                hex_head_height_mm: 4.0,
                nut_height_mm: 5.2,
                socket_head_diameter_mm: 10.0,
                socket_head_height_mm: 6.0,
                socket_key_across_flats_mm: 5.0,
                clearance_hole_close_mm: 6.4,
                clearance_hole_medium_mm: 6.6,
            },
        ),
        (
            8.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 8.0,
                coarse_pitch_mm: 1.25,
                hex_width_across_flats_mm: 13.0,
                hex_head_height_mm: 5.3,
                nut_height_mm: 6.8,
                socket_head_diameter_mm: 13.0,
                socket_head_height_mm: 8.0,
                socket_key_across_flats_mm: 6.0,
                clearance_hole_close_mm: 8.4,
                clearance_hole_medium_mm: 9.0,
            },
        ),
        (
            10.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 10.0,
                coarse_pitch_mm: 1.5,
                hex_width_across_flats_mm: 16.0,
                hex_head_height_mm: 6.4,
                nut_height_mm: 8.4,
                socket_head_diameter_mm: 16.0,
                socket_head_height_mm: 10.0,
                socket_key_across_flats_mm: 8.0,
                clearance_hole_close_mm: 10.5,
                clearance_hole_medium_mm: 11.0,
            },
        ),
        (
            12.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 12.0,
                coarse_pitch_mm: 1.75,
                hex_width_across_flats_mm: 18.0,
                hex_head_height_mm: 7.5,
                nut_height_mm: 10.8,
                socket_head_diameter_mm: 18.0,
                socket_head_height_mm: 12.0,
                socket_key_across_flats_mm: 10.0,
                clearance_hole_close_mm: 13.0,
                clearance_hole_medium_mm: 13.5,
            },
        ),
        (
            16.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 16.0,
                coarse_pitch_mm: 2.0,
                hex_width_across_flats_mm: 24.0,
                hex_head_height_mm: 10.0,
                nut_height_mm: 14.8,
                socket_head_diameter_mm: 24.0,
                socket_head_height_mm: 16.0,
                socket_key_across_flats_mm: 14.0,
                clearance_hole_close_mm: 17.0,
                clearance_hole_medium_mm: 17.5,
            },
        ),
        (
            20.0,
            MetricFastenerSpec {
                nominal_diameter_mm: 20.0,
                coarse_pitch_mm: 2.5,
                hex_width_across_flats_mm: 30.0,
                hex_head_height_mm: 12.5,
                nut_height_mm: 18.0,
                socket_head_diameter_mm: 30.0,
                socket_head_height_mm: 20.0,
                socket_key_across_flats_mm: 17.0,
                clearance_hole_close_mm: 21.0,
                clearance_hole_medium_mm: 22.0,
            },
        ),
    ];
    for &(size, spec) in table {
        if (size - nominal_mm).abs() < 1e-6 {
            return Ok(spec);
        }
    }
    Err(err(
        "fastener",
        "Unsupported ISO metric nominal diameter (supported: M2, M2.5, M3, M4, M5, M6, M8, M10, M12, M16, M20).",
    ))
}

/// Generates a 2D hex nut profile (outer hexagon + inner thread minor-diameter circle) for an ISO metric size.
pub fn hex_nut_profile(nominal_mm: f64) -> Result<Vec<Vec<Point>>> {
    let spec = iso_metric_fastener(nominal_mm)?;
    let r_circum = spec.hex_width_across_flats_mm / 3.0_f64.sqrt();
    let hex: Vec<Point> = (0..6)
        .map(|i| {
            let a = TAU * (i as f64) / 6.0;
            [r_circum * a.cos(), r_circum * a.sin()]
        })
        .collect();
    let r_hole = 0.5 * (spec.nominal_diameter_mm - 1.0825 * spec.coarse_pitch_mm);
    let hole: Vec<Point> = (0..32)
        .rev()
        .map(|i| {
            let a = TAU * (i as f64) / 32.0;
            [r_hole * a.cos(), r_hole * a.sin()]
        })
        .collect();
    Ok(vec![hex, hole])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rack_and_pulley_profiles_are_valid() {
        let rack = involute_rack_profile(&RackOptions::default()).unwrap();
        assert_eq!(rack.len(), 10 * 4 + 2);
        let pulley = timing_pulley_profile(&TimingPulleyOptions::default()).unwrap();
        assert_eq!(pulley.len(), 2);
        assert_eq!(pulley[0].len(), 160);
    }

    #[test]
    fn iso_fasteners_and_nut_profile_match_standards() {
        let m6 = iso_metric_fastener(6.0).unwrap();
        assert_eq!(m6.coarse_pitch_mm, 1.0);
        assert_eq!(m6.hex_width_across_flats_mm, 10.0);
        let nut = hex_nut_profile(6.0).unwrap();
        assert_eq!(nut.len(), 2);
        assert_eq!(nut[0].len(), 6);
        assert!(iso_metric_fastener(7.0).is_err());
    }
}
