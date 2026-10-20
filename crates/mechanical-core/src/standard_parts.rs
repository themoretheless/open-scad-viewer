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

/// Standard synchronous belt pulley tooth series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PulleyStandard {
    Gt2_2M,
    Gt2_3M,
    Htd3M,
    Htd5M,
    Htd8M,
}

/// Returns a pre-configured `TimingPulleyOptions` for standard GT2 and HTD timing belt profiles.
pub fn timing_pulley_standard(
    standard: PulleyStandard,
    teeth: usize,
    bore_mm: f64,
) -> TimingPulleyOptions {
    let (pitch_mm, tooth_depth_mm) = match standard {
        PulleyStandard::Gt2_2M => (2.0, 0.75),
        PulleyStandard::Gt2_3M => (3.0, 1.14),
        PulleyStandard::Htd3M => (3.0, 1.22),
        PulleyStandard::Htd5M => (5.0, 2.08),
        PulleyStandard::Htd8M => (8.0, 3.45),
    };
    TimingPulleyOptions {
        teeth,
        pitch_mm,
        tooth_depth_mm,
        bore_mm,
        segments_per_tooth: 10,
    }
}

/// Options for an ISO 606 roller chain sprocket profile.
#[derive(Debug, Clone)]
pub struct SprocketOptions {
    pub teeth: usize,
    pub chain_pitch_mm: f64,
    pub roller_diameter_mm: f64,
    pub bore_mm: f64,
    pub segments_per_tooth: usize,
}

impl Default for SprocketOptions {
    fn default() -> Self {
        // ISO 606 08B (1/2" pitch) roller chain defaults
        Self {
            teeth: 18,
            chain_pitch_mm: 12.7,
            roller_diameter_mm: 8.51,
            bore_mm: 10.0,
            segments_per_tooth: 10,
        }
    }
}

/// Generates outer and optional inner bore 2D profile loops for an ISO 606 roller chain sprocket.
pub fn roller_chain_sprocket(o: &SprocketOptions) -> Result<Vec<Vec<Point>>> {
    if !(6..=150).contains(&o.teeth) {
        return Err(err("sprocket", "Sprocket teeth must be from 6 to 150."));
    }
    if !o.chain_pitch_mm.is_finite() || !(4.0..=100.0).contains(&o.chain_pitch_mm) {
        return Err(err(
            "sprocket",
            "Sprocket chain_pitch_mm must be from 4 to 100 mm.",
        ));
    }
    if !o.roller_diameter_mm.is_finite()
        || o.roller_diameter_mm <= 1.0
        || o.roller_diameter_mm >= o.chain_pitch_mm * 0.85
    {
        return Err(err(
            "sprocket",
            "Sprocket roller_diameter_mm must be between 1.0 mm and 0.85 * chain_pitch_mm.",
        ));
    }
    if !(4..=24).contains(&o.segments_per_tooth) {
        return Err(err("sprocket", "segments_per_tooth must be from 4 to 24."));
    }
    let z = o.teeth as f64;
    let pitch_radius = o.chain_pitch_mm / (2.0 * (PI / z).sin());
    let seating_radius = 0.5025 * o.roller_diameter_mm + 0.025;
    let root_radius = pitch_radius - seating_radius;
    let tip_radius =
        o.chain_pitch_mm * (0.6 + 1.0 / (PI / z).tan()) * 0.5 - 0.15 * o.roller_diameter_mm;
    if !o.bore_mm.is_finite() || o.bore_mm < 0.0 || o.bore_mm >= 2.0 * root_radius - 1.0 {
        return Err(err(
            "sprocket",
            "Sprocket bore_mm must leave at least 0.5 mm radial wall inside root circle.",
        ));
    }
    let total_steps = o.teeth * o.segments_per_tooth;
    let mut outer = Vec::with_capacity(total_steps);
    for i in 0..total_steps {
        let theta = TAU * (i as f64) / (total_steps as f64);
        let phase = (i % o.segments_per_tooth) as f64 / (o.segments_per_tooth as f64);
        // Smooth roller seating pocket + tooth crest transition
        let u = (phase - 0.5).abs() * 2.0; // 0 at roller center, 1 at tooth tip
        let blend = u * u * (3.0 - 2.0 * u);
        let r = root_radius + (tip_radius - root_radius) * blend;
        outer.push([r * theta.cos(), r * theta.sin()]);
    }
    let mut loops = vec![outer];
    if o.bore_mm > 0.0 {
        let r_bore = o.bore_mm * 0.5;
        let bore: Vec<Point> = (0..32)
            .rev()
            .map(|i| {
                let a = TAU * (i as f64) / 32.0;
                [r_bore * a.cos(), r_bore * a.sin()]
            })
            .collect();
        loops.push(bore);
    }
    Ok(loops)
}

/// Helical gear design inputs (normal module system).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HelicalGearOptions {
    pub teeth: usize,
    pub normal_module_mm: f64,
    pub normal_pressure_angle_deg: f64,
    pub helix_angle_deg: f64,
    pub face_width_mm: f64,
}

/// Computed geometric parameters for a cylindrical helical gear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HelicalGearReport {
    pub transverse_module_mm: f64,
    pub transverse_pressure_angle_deg: f64,
    pub pitch_diameter_mm: f64,
    pub tip_diameter_mm: f64,
    pub root_diameter_mm: f64,
    pub axial_pitch_mm: f64,
    pub lead_mm: f64,
    pub helix_overlap_ratio: f64,
    pub virtual_spur_teeth: f64,
    pub twist_angle_deg: f64,
}

/// Computes helical gear geometry, virtual spur tooth count, and twist angle for extrusion.
pub fn helical_gear_params(o: &HelicalGearOptions) -> Result<HelicalGearReport> {
    if !(3..=500).contains(&o.teeth) {
        return Err(err("helical_gear", "Teeth must be between 3 and 500."));
    }
    if !o.normal_module_mm.is_finite() || !(0.1..=100.0).contains(&o.normal_module_mm) {
        return Err(err(
            "helical_gear",
            "normal_module_mm must be between 0.1 and 100 mm.",
        ));
    }
    if !o.normal_pressure_angle_deg.is_finite()
        || !(12.0..=35.0).contains(&o.normal_pressure_angle_deg)
    {
        return Err(err(
            "helical_gear",
            "normal_pressure_angle_deg must be between 12 and 35 degrees.",
        ));
    }
    if !o.helix_angle_deg.is_finite()
        || o.helix_angle_deg.abs() < 1.0
        || o.helix_angle_deg.abs() > 60.0
    {
        return Err(err(
            "helical_gear",
            "helix_angle_deg magnitude must be between 1 and 60 degrees.",
        ));
    }
    if !o.face_width_mm.is_finite() || !(0.5..=2000.0).contains(&o.face_width_mm) {
        return Err(err(
            "helical_gear",
            "face_width_mm must be between 0.5 and 2000 mm.",
        ));
    }
    let beta = o.helix_angle_deg.to_radians();
    let cos_b = beta.cos();
    let sin_b = beta.sin().abs();
    let alpha_n = o.normal_pressure_angle_deg.to_radians();
    let mt = o.normal_module_mm / cos_b;
    let alpha_t = (alpha_n.tan() / cos_b).atan();
    let d = mt * o.teeth as f64;
    let da = d + 2.0 * o.normal_module_mm;
    let df = d - 2.5 * o.normal_module_mm;
    let px = (PI * o.normal_module_mm) / sin_b;
    let lead = PI * d / beta.tan();
    let overlap = o.face_width_mm / px;
    let zv = (o.teeth as f64) / (cos_b * cos_b * cos_b);
    let twist_deg = 360.0 * o.face_width_mm / lead;
    Ok(HelicalGearReport {
        transverse_module_mm: mt,
        transverse_pressure_angle_deg: alpha_t.to_degrees(),
        pitch_diameter_mm: d,
        tip_diameter_mm: da,
        root_diameter_mm: df,
        axial_pitch_mm: px,
        lead_mm: lead.abs(),
        helix_overlap_ratio: overlap,
        virtual_spur_teeth: zv,
        twist_angle_deg: twist_deg,
    })
}

/// Straight bevel gear pair inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BevelGearOptions {
    pub pinion_teeth: usize,
    pub gear_teeth: usize,
    pub module_mm: f64,
    pub shaft_angle_deg: f64,
    pub pressure_angle_deg: f64,
    pub face_width_mm: f64,
}

/// Computed pitch cones, outer cone distance, and Tredgold virtual tooth counts for a bevel gear pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BevelGearReport {
    pub pinion_pitch_angle_deg: f64,
    pub gear_pitch_angle_deg: f64,
    pub pinion_pitch_diameter_mm: f64,
    pub gear_pitch_diameter_mm: f64,
    pub outer_cone_distance_mm: f64,
    pub pinion_tip_diameter_mm: f64,
    pub gear_tip_diameter_mm: f64,
    pub pinion_virtual_teeth: f64,
    pub gear_virtual_teeth: f64,
}

/// Computes straight bevel gear blank and cone geometry.
pub fn bevel_gear_params(o: &BevelGearOptions) -> Result<BevelGearReport> {
    if !(6..=300).contains(&o.pinion_teeth) || !(6..=300).contains(&o.gear_teeth) {
        return Err(err("bevel_gear", "Tooth counts must be between 6 and 300."));
    }
    if !o.module_mm.is_finite() || !(0.2..=100.0).contains(&o.module_mm) {
        return Err(err(
            "bevel_gear",
            "module_mm must be between 0.2 and 100 mm.",
        ));
    }
    if !o.shaft_angle_deg.is_finite() || !(15.0..=165.0).contains(&o.shaft_angle_deg) {
        return Err(err(
            "bevel_gear",
            "shaft_angle_deg must be between 15 and 165 degrees.",
        ));
    }
    if !o.pressure_angle_deg.is_finite() || !(14.5..=30.0).contains(&o.pressure_angle_deg) {
        return Err(err(
            "bevel_gear",
            "pressure_angle_deg must be between 14.5 and 30 degrees.",
        ));
    }
    let z1 = o.pinion_teeth as f64;
    let z2 = o.gear_teeth as f64;
    let sigma = o.shaft_angle_deg.to_radians();
    let delta1 = (sigma.sin() / (z2 / z1 + sigma.cos())).atan();
    let delta1 = if delta1 < 0.0 { delta1 + PI } else { delta1 };
    let delta2 = sigma - delta1;
    let d1 = o.module_mm * z1;
    let d2 = o.module_mm * z2;
    let r_cone = d1 / (2.0 * delta1.sin());
    if !o.face_width_mm.is_finite() || o.face_width_mm <= 0.1 || o.face_width_mm > r_cone * 0.5 {
        return Err(err(
            "bevel_gear",
            "face_width_mm must be positive and at most 0.5 * outer_cone_distance.",
        ));
    }
    let da1 = d1 + 2.0 * o.module_mm * delta1.cos();
    let da2 = d2 + 2.0 * o.module_mm * delta2.cos();
    let zv1 = z1 / delta1.cos().abs().max(1e-6);
    let zv2 = z2 / delta2.cos().abs().max(1e-6);
    Ok(BevelGearReport {
        pinion_pitch_angle_deg: delta1.to_degrees(),
        gear_pitch_angle_deg: delta2.to_degrees(),
        pinion_pitch_diameter_mm: d1,
        gear_pitch_diameter_mm: d2,
        outer_cone_distance_mm: r_cone,
        pinion_tip_diameter_mm: da1,
        gear_tip_diameter_mm: da2,
        pinion_virtual_teeth: zv1,
        gear_virtual_teeth: zv2,
    })
}

/// Worm drive (cylindrical worm + worm wheel) inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WormDriveOptions {
    pub worm_starts: usize,
    pub wheel_teeth: usize,
    pub axial_module_mm: f64,
    pub worm_pitch_diameter_mm: f64,
    pub friction_coefficient: f64,
}

/// Computed worm drive parameters, lead angle, center distance, and self-locking indicator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WormDriveReport {
    pub gear_ratio: f64,
    pub axial_pitch_mm: f64,
    pub lead_mm: f64,
    pub lead_angle_deg: f64,
    pub wheel_pitch_diameter_mm: f64,
    pub center_distance_mm: f64,
    pub worm_tip_diameter_mm: f64,
    pub worm_root_diameter_mm: f64,
    pub efficiency: f64,
    pub self_locking: bool,
}

/// Calculates worm drive geometry, mechanical efficiency, and self-locking condition.
pub fn worm_drive_params(o: &WormDriveOptions) -> Result<WormDriveReport> {
    if !(1..=8).contains(&o.worm_starts) || !(10..=300).contains(&o.wheel_teeth) {
        return Err(err(
            "worm_drive",
            "worm_starts must be 1..=8 and wheel_teeth 10..=300.",
        ));
    }
    if !o.axial_module_mm.is_finite() || !(0.3..=50.0).contains(&o.axial_module_mm) {
        return Err(err(
            "worm_drive",
            "axial_module_mm must be between 0.3 and 50 mm.",
        ));
    }
    if !o.worm_pitch_diameter_mm.is_finite()
        || o.worm_pitch_diameter_mm < 3.0 * o.axial_module_mm
        || o.worm_pitch_diameter_mm > 50.0 * o.axial_module_mm
    {
        return Err(err(
            "worm_drive",
            "worm_pitch_diameter_mm must be between 3*m and 50*m.",
        ));
    }
    if !o.friction_coefficient.is_finite() || !(0.0..=0.5).contains(&o.friction_coefficient) {
        return Err(err(
            "worm_drive",
            "friction_coefficient must be between 0.0 and 0.5.",
        ));
    }
    let z1 = o.worm_starts as f64;
    let z2 = o.wheel_teeth as f64;
    let px = PI * o.axial_module_mm;
    let lead = z1 * px;
    let gamma = (lead / (PI * o.worm_pitch_diameter_mm)).atan();
    let d2 = o.axial_module_mm * z2;
    let center = 0.5 * (o.worm_pitch_diameter_mm + d2);
    let rho = o.friction_coefficient.atan();
    let efficiency = gamma.tan() / (gamma + rho).tan();
    Ok(WormDriveReport {
        gear_ratio: z2 / z1,
        axial_pitch_mm: px,
        lead_mm: lead,
        lead_angle_deg: gamma.to_degrees(),
        wheel_pitch_diameter_mm: d2,
        center_distance_mm: center,
        worm_tip_diameter_mm: o.worm_pitch_diameter_mm + 2.0 * o.axial_module_mm,
        worm_root_diameter_mm: o.worm_pitch_diameter_mm - 2.4 * o.axial_module_mm,
        efficiency: efficiency.clamp(0.0, 1.0),
        self_locking: gamma <= rho,
    })
}

/// 3D-printed heat-set threaded insert pocket specification (CNC Kitchen / Ruthex standard).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeatSetInsertSpec {
    pub nominal_thread_mm: f64,
    pub insert_outer_diameter_mm: f64,
    pub recommended_hole_diameter_mm: f64,
    pub insert_length_mm: f64,
    pub min_pocket_depth_mm: f64,
    pub min_boss_outer_diameter_mm: f64,
}

/// Looks up standard 3D-printing heat-set insert pocket & boss dimensions for M2, M2.5, M3, M4, M5, M6, M8.
pub fn heat_set_insert_spec(nominal_mm: f64) -> Result<HeatSetInsertSpec> {
    let table: &[(f64, HeatSetInsertSpec)] = &[
        (
            2.0,
            HeatSetInsertSpec {
                nominal_thread_mm: 2.0,
                insert_outer_diameter_mm: 3.5,
                recommended_hole_diameter_mm: 3.2,
                insert_length_mm: 3.0,
                min_pocket_depth_mm: 4.0,
                min_boss_outer_diameter_mm: 6.4,
            },
        ),
        (
            2.5,
            HeatSetInsertSpec {
                nominal_thread_mm: 2.5,
                insert_outer_diameter_mm: 3.8,
                recommended_hole_diameter_mm: 3.5,
                insert_length_mm: 4.0,
                min_pocket_depth_mm: 5.0,
                min_boss_outer_diameter_mm: 7.0,
            },
        ),
        (
            3.0,
            HeatSetInsertSpec {
                nominal_thread_mm: 3.0,
                insert_outer_diameter_mm: 4.5,
                recommended_hole_diameter_mm: 4.0,
                insert_length_mm: 5.7,
                min_pocket_depth_mm: 6.7,
                min_boss_outer_diameter_mm: 8.0,
            },
        ),
        (
            4.0,
            HeatSetInsertSpec {
                nominal_thread_mm: 4.0,
                insert_outer_diameter_mm: 6.0,
                recommended_hole_diameter_mm: 5.6,
                insert_length_mm: 8.1,
                min_pocket_depth_mm: 9.1,
                min_boss_outer_diameter_mm: 10.6,
            },
        ),
        (
            5.0,
            HeatSetInsertSpec {
                nominal_thread_mm: 5.0,
                insert_outer_diameter_mm: 7.0,
                recommended_hole_diameter_mm: 6.4,
                insert_length_mm: 9.5,
                min_pocket_depth_mm: 10.5,
                min_boss_outer_diameter_mm: 12.4,
            },
        ),
        (
            6.0,
            HeatSetInsertSpec {
                nominal_thread_mm: 6.0,
                insert_outer_diameter_mm: 8.7,
                recommended_hole_diameter_mm: 8.0,
                insert_length_mm: 12.7,
                min_pocket_depth_mm: 14.0,
                min_boss_outer_diameter_mm: 15.0,
            },
        ),
        (
            8.0,
            HeatSetInsertSpec {
                nominal_thread_mm: 8.0,
                insert_outer_diameter_mm: 10.5,
                recommended_hole_diameter_mm: 9.8,
                insert_length_mm: 12.7,
                min_pocket_depth_mm: 14.5,
                min_boss_outer_diameter_mm: 18.0,
            },
        ),
    ];
    for &(size, spec) in table {
        if (size - nominal_mm).abs() < 1e-6 {
            return Ok(spec);
        }
    }
    Err(err(
        "heat_set_insert",
        "Unsupported heat-set insert size (supported: M2, M2.5, M3, M4, M5, M6, M8).",
    ))
}

/// DIN 6885 parallel keyway specification for shafts and hubs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeywaySpec {
    pub key_width_b_mm: f64,
    pub key_height_h_mm: f64,
    pub shaft_groove_depth_t1_mm: f64,
    pub hub_groove_depth_t2_mm: f64,
}

/// Looks up DIN 6885 parallel key and keyway dimensions for a shaft diameter in [6, 130] mm.
pub fn shaft_keyway_din6885(shaft_diameter_mm: f64) -> Result<KeywaySpec> {
    if !shaft_diameter_mm.is_finite() || !(6.0..=130.0).contains(&shaft_diameter_mm) {
        return Err(err(
            "keyway",
            "Shaft diameter for DIN 6885 keyway must be between 6 and 130 mm.",
        ));
    }
    let d = shaft_diameter_mm;
    let (b, h, t1, t2) = if d <= 8.0 {
        (2.0, 2.0, 1.2, 1.0)
    } else if d <= 10.0 {
        (3.0, 3.0, 1.8, 1.4)
    } else if d <= 12.0 {
        (4.0, 4.0, 2.5, 1.8)
    } else if d <= 17.0 {
        (5.0, 5.0, 3.0, 2.3)
    } else if d <= 22.0 {
        (6.0, 6.0, 3.5, 2.8)
    } else if d <= 30.0 {
        (8.0, 7.0, 4.0, 3.3)
    } else if d <= 38.0 {
        (10.0, 8.0, 5.0, 3.3)
    } else if d <= 44.0 {
        (12.0, 8.0, 5.0, 3.3)
    } else if d <= 50.0 {
        (14.0, 9.0, 5.5, 3.8)
    } else if d <= 58.0 {
        (16.0, 10.0, 6.0, 4.3)
    } else if d <= 65.0 {
        (18.0, 11.0, 7.0, 4.4)
    } else if d <= 75.0 {
        (20.0, 12.0, 7.5, 4.9)
    } else if d <= 85.0 {
        (22.0, 14.0, 9.0, 5.4)
    } else if d <= 95.0 {
        (25.0, 14.0, 9.0, 5.4)
    } else if d <= 110.0 {
        (28.0, 16.0, 10.0, 6.4)
    } else {
        (32.0, 18.0, 11.0, 7.4)
    };
    Ok(KeywaySpec {
        key_width_b_mm: b,
        key_height_h_mm: h,
        shaft_groove_depth_t1_mm: t1,
        hub_groove_depth_t2_mm: t2,
    })
}

/// DIN 471 (external shaft) / DIN 472 (internal bore) circlip groove specification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetainingRingGrooveSpec {
    pub nominal_diameter_mm: f64,
    pub groove_diameter_d2_mm: f64,
    pub groove_width_m_mm: f64,
    pub ring_thickness_s_mm: f64,
}

/// Computes DIN 471 (external) or DIN 472 (internal) retaining ring groove dimensions.
pub fn retaining_ring_groove(
    nominal_diameter_mm: f64,
    internal_bore: bool,
) -> Result<RetainingRingGrooveSpec> {
    if !nominal_diameter_mm.is_finite() || !(4.0..=100.0).contains(&nominal_diameter_mm) {
        return Err(err(
            "retaining_ring",
            "Nominal diameter must be between 4 and 100 mm.",
        ));
    }
    let d = nominal_diameter_mm;
    let (s, m, radial_depth) = if d < 10.0 {
        (0.8, 0.9, 0.3)
    } else if d < 18.0 {
        (1.0, 1.1, 0.45)
    } else if d < 25.0 {
        (1.2, 1.3, 0.6)
    } else if d < 35.0 {
        (1.5, 1.6, 0.75)
    } else if d < 50.0 {
        (1.75, 1.85, 1.0)
    } else {
        (2.0, 2.15, 1.25)
    };
    let d2 = if internal_bore {
        d + 2.0 * radial_depth
    } else {
        d - 2.0 * radial_depth
    };
    Ok(RetainingRingGrooveSpec {
        nominal_diameter_mm: d,
        groove_diameter_d2_mm: d2,
        groove_width_m_mm: m,
        ring_thickness_s_mm: s,
    })
}

/// ISO 3601 static O-ring gland/groove specification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ORingGrooveSpec {
    pub cross_section_d2_mm: f64,
    pub groove_depth_h_mm: f64,
    pub groove_width_b_mm: f64,
    pub compression_ratio: f64,
    pub corner_radius_mm: f64,
}

/// Computes static axial/radial O-ring groove dimensions for standard ISO 3601 cross-sections.
pub fn o_ring_groove_spec(cross_section_d2_mm: f64, dynamic_seal: bool) -> Result<ORingGrooveSpec> {
    if !cross_section_d2_mm.is_finite() || !(1.0..=10.0).contains(&cross_section_d2_mm) {
        return Err(err(
            "o_ring",
            "O-ring cross-section diameter d2 must be between 1.0 and 10.0 mm.",
        ));
    }
    let squeeze = if dynamic_seal { 0.14 } else { 0.22 };
    let depth = cross_section_d2_mm * (1.0 - squeeze);
    let width = cross_section_d2_mm * if dynamic_seal { 1.38 } else { 1.32 };
    Ok(ORingGrooveSpec {
        cross_section_d2_mm,
        groove_depth_h_mm: (depth * 100.0).round() / 100.0,
        groove_width_b_mm: (width * 100.0).round() / 100.0,
        compression_ratio: squeeze,
        corner_radius_mm: (cross_section_d2_mm * 0.15).clamp(0.2, 1.0),
    })
}

/// ISO 286 bearing / shaft fit class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BearingFitClass {
    /// Clearance fit (sliding shaft in H7 bore: H7/g6)
    ClearanceH7g6,
    /// Transition locating fit (light press / keying: H7/k6)
    TransitionH7k6,
    /// Interference press fit (heavy rotating load: H7/p6)
    InterferenceH7p6,
}

/// ISO 286 hole-basis H7 bore and shaft limits in millimeters and micrometers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BearingFitReport {
    pub nominal_diameter_mm: f64,
    pub hole_min_mm: f64,
    pub hole_max_mm: f64,
    pub shaft_min_mm: f64,
    pub shaft_max_mm: f64,
    /// Positive means clearance, negative means interference.
    pub min_clearance_um: f64,
    pub max_clearance_um: f64,
}

/// Computes ISO 286 H7 bore and g6/k6/p6 shaft tolerance bands for diameters in [3, 120] mm.
pub fn iso286_shaft_housing_fit(
    nominal_diameter_mm: f64,
    fit: BearingFitClass,
) -> Result<BearingFitReport> {
    if !nominal_diameter_mm.is_finite() || !(3.0..=120.0).contains(&nominal_diameter_mm) {
        return Err(err(
            "iso286_fit",
            "Nominal diameter for ISO 286 fit must be between 3 and 120 mm.",
        ));
    }
    let d = nominal_diameter_mm;
    // Standard tolerance unit i (um) = 0.45 * D^(1/3) + 0.001 * D
    let i_um = 0.45 * d.cbrt() + 0.001 * d;
    let it7_um = (16.0 * i_um).round();
    let it6_um = (10.0 * i_um).round();
    let (shaft_lower_um, shaft_upper_um) = match fit {
        BearingFitClass::ClearanceH7g6 => {
            let es = -(2.5 * d.powf(0.34)).round();
            (es - it6_um, es)
        }
        BearingFitClass::TransitionH7k6 => {
            let ei = (0.6 * d.cbrt()).round();
            (ei, ei + it6_um)
        }
        BearingFitClass::InterferenceH7p6 => {
            let ei = (it7_um + 2.0).round();
            (ei, ei + it6_um)
        }
    };
    let hole_min_mm = d;
    let hole_max_mm = d + it7_um * 1e-3;
    let shaft_min_mm = d + shaft_lower_um * 1e-3;
    let shaft_max_mm = d + shaft_upper_um * 1e-3;
    Ok(BearingFitReport {
        nominal_diameter_mm: d,
        hole_min_mm,
        hole_max_mm,
        shaft_min_mm,
        shaft_max_mm,
        min_clearance_um: -shaft_upper_um,
        max_clearance_um: it7_um - shaft_lower_um,
    })
}

/// Belleville disc spring (DIN 2092) parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiscSpringOptions {
    pub outer_diameter_mm: f64,
    pub inner_diameter_mm: f64,
    pub thickness_mm: f64,
    pub free_cone_height_mm: f64,
    pub deflection_mm: f64,
    pub young_mpa: f64,
    pub poisson: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiscSpringReport {
    pub axial_load_n: f64,
    pub flat_load_n: f64,
    pub spring_rate_n_per_mm: f64,
}

/// Computes Almen–Laszlo / DIN 2092 Belleville disc spring axial load and stiffness.
pub fn disc_spring_load(o: &DiscSpringOptions) -> Result<DiscSpringReport> {
    if !o.outer_diameter_mm.is_finite()
        || !o.inner_diameter_mm.is_finite()
        || o.inner_diameter_mm <= 0.5
        || o.outer_diameter_mm <= o.inner_diameter_mm * 1.15
    {
        return Err(err(
            "disc_spring",
            "outer_diameter_mm must exceed 1.15 * inner_diameter_mm.",
        ));
    }
    if !o.thickness_mm.is_finite()
        || o.thickness_mm <= 0.05
        || !o.free_cone_height_mm.is_finite()
        || o.free_cone_height_mm <= 0.01
        || !o.deflection_mm.is_finite()
        || o.deflection_mm < 0.0
    {
        return Err(err(
            "disc_spring",
            "thickness, cone height, and deflection must be finite and valid.",
        ));
    }
    if !o.young_mpa.is_finite()
        || o.young_mpa <= 100.0
        || !o.poisson.is_finite()
        || !(0.0..0.49).contains(&o.poisson)
    {
        return Err(err(
            "disc_spring",
            "Invalid Young's modulus or Poisson ratio.",
        ));
    }
    let delta = o.outer_diameter_mm / o.inner_diameter_mm;
    let k1 = (1.0 / PI)
        * ((delta - 1.0) / delta).powi(2)
        / ((delta + 1.0) / (delta - 1.0) - 2.0 / delta.ln());
    let coeff = (4.0 * o.young_mpa)
        / (1.0 - o.poisson * o.poisson)
        * (o.thickness_mm.powi(4) / (k1 * o.outer_diameter_mm.powi(2)));
    let s = o.deflection_mm;
    let h0 = o.free_cone_height_mm;
    let t = o.thickness_mm;
    let load_at = |def: f64| {
        let u = def / t;
        let ratio = h0 / t;
        coeff * u * ((ratio - u) * (ratio - 0.5 * u) + 1.0)
    };
    let f = load_at(s);
    let f_flat = load_at(h0);
    let ds = 1e-4 * t;
    let rate = (load_at(s + ds) - load_at((s - ds).max(0.0))) / (2.0 * ds);
    Ok(DiscSpringReport {
        axial_load_n: f,
        flat_load_n: f_flat,
        spring_rate_n_per_mm: rate,
    })
}

/// Helical cylindrical compression spring inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HelicalSpringOptions {
    pub wire_diameter_mm: f64,
    pub mean_coil_diameter_mm: f64,
    pub active_coils: f64,
    pub shear_modulus_mpa: f64,
    pub axial_load_n: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HelicalSpringReport {
    pub spring_index: f64,
    pub wahl_factor: f64,
    pub spring_rate_n_per_mm: f64,
    pub deflection_mm: f64,
    pub max_shear_stress_mpa: f64,
    pub solid_length_mm: f64,
}

/// Computes helical spring stiffness, deflection, and Wahl-corrected torsional shear stress.
pub fn helical_spring_analysis(o: &HelicalSpringOptions) -> Result<HelicalSpringReport> {
    if !o.wire_diameter_mm.is_finite()
        || o.wire_diameter_mm <= 0.05
        || !o.mean_coil_diameter_mm.is_finite()
        || o.mean_coil_diameter_mm < 3.0 * o.wire_diameter_mm
    {
        return Err(err(
            "helical_spring",
            "mean_coil_diameter_mm must be at least 3 * wire_diameter_mm.",
        ));
    }
    if !o.active_coils.is_finite()
        || o.active_coils < 1.0
        || !o.shear_modulus_mpa.is_finite()
        || o.shear_modulus_mpa <= 100.0
        || !o.axial_load_n.is_finite()
        || o.axial_load_n < 0.0
    {
        return Err(err(
            "helical_spring",
            "active_coils, shear_modulus_mpa, and axial_load_n must be valid positive numbers.",
        ));
    }
    let d = o.wire_diameter_mm;
    let dm = o.mean_coil_diameter_mm;
    let c = dm / d;
    let wahl = (4.0 * c - 1.0) / (4.0 * c - 4.0) + 0.615 / c;
    let k = (o.shear_modulus_mpa * d.powi(4)) / (8.0 * dm.powi(3) * o.active_coils);
    let deflection = o.axial_load_n / k;
    let tau = wahl * (8.0 * o.axial_load_n * dm) / (PI * d.powi(3));
    Ok(HelicalSpringReport {
        spring_index: c,
        wahl_factor: wahl,
        spring_rate_n_per_mm: k,
        deflection_mm: deflection,
        max_shear_stress_mpa: tau,
        solid_length_mm: (o.active_coils + 2.0) * d,
    })
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

    #[test]
    fn helical_bevel_worm_and_sprocket_calculations() {
        let h = helical_gear_params(&HelicalGearOptions {
            teeth: 20,
            normal_module_mm: 2.0,
            normal_pressure_angle_deg: 20.0,
            helix_angle_deg: 15.0,
            face_width_mm: 12.0,
        })
        .unwrap();
        assert!(h.transverse_module_mm > 2.0);
        assert!(h.virtual_spur_teeth > 20.0);

        let b = bevel_gear_params(&BevelGearOptions {
            pinion_teeth: 20,
            gear_teeth: 40,
            module_mm: 2.0,
            shaft_angle_deg: 90.0,
            pressure_angle_deg: 20.0,
            face_width_mm: 10.0,
        })
        .unwrap();
        assert!((b.pinion_pitch_angle_deg + b.gear_pitch_angle_deg - 90.0).abs() < 1e-9);
        assert!((b.pinion_pitch_diameter_mm - 40.0).abs() < 1e-9);

        let w = worm_drive_params(&WormDriveOptions {
            worm_starts: 1,
            wheel_teeth: 40,
            axial_module_mm: 2.0,
            worm_pitch_diameter_mm: 20.0,
            friction_coefficient: 0.15,
        })
        .unwrap();
        assert_eq!(w.gear_ratio, 40.0);
        assert!(w.self_locking);

        let sp = roller_chain_sprocket(&SprocketOptions::default()).unwrap();
        assert_eq!(sp.len(), 2);
        assert_eq!(sp[0].len(), 180);

        let htd5m = timing_pulley_standard(PulleyStandard::Htd5M, 24, 8.0);
        let htd_loops = timing_pulley_profile(&htd5m).unwrap();
        assert_eq!(htd_loops.len(), 2);
    }

    #[test]
    fn inserts_keyways_grooves_fits_and_springs() {
        let m3_insert = heat_set_insert_spec(3.0).unwrap();
        assert!((m3_insert.recommended_hole_diameter_mm - 4.0).abs() < 1e-6);

        let kw = shaft_keyway_din6885(20.0).unwrap();
        assert_eq!(kw.key_width_b_mm, 6.0);

        let circlip = retaining_ring_groove(20.0, false).unwrap();
        assert!(circlip.groove_diameter_d2_mm < 20.0);

        let oring = o_ring_groove_spec(2.5, false).unwrap();
        assert!(oring.groove_depth_h_mm < 2.5);
        assert!(oring.groove_width_b_mm > 2.5);

        let fit_g6 = iso286_shaft_housing_fit(20.0, BearingFitClass::ClearanceH7g6).unwrap();
        assert!(fit_g6.min_clearance_um > 0.0);
        let fit_p6 = iso286_shaft_housing_fit(20.0, BearingFitClass::InterferenceH7p6).unwrap();
        assert!(fit_p6.max_clearance_um < 0.0);

        let disc = disc_spring_load(&DiscSpringOptions {
            outer_diameter_mm: 20.0,
            inner_diameter_mm: 10.2,
            thickness_mm: 1.0,
            free_cone_height_mm: 0.55,
            deflection_mm: 0.3,
            young_mpa: 206_000.0,
            poisson: 0.3,
        })
        .unwrap();
        assert!(disc.axial_load_n > 100.0);
        assert!(disc.flat_load_n > disc.axial_load_n);

        let spring = helical_spring_analysis(&HelicalSpringOptions {
            wire_diameter_mm: 2.0,
            mean_coil_diameter_mm: 16.0,
            active_coils: 8.0,
            shear_modulus_mpa: 79_300.0,
            axial_load_n: 50.0,
        })
        .unwrap();
        assert_eq!(spring.spring_index, 8.0);
        assert!(spring.wahl_factor > 1.15);
        assert!(spring.deflection_mm > 0.0);
    }
}

