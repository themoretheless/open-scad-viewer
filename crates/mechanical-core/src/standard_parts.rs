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



/// DIN 6885 parallel keyway specification for shafts and hubs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeywaySpec {
    pub key_width_b_mm: f64,
    pub key_height_h_mm: f64,
    pub shaft_groove_depth_t1_mm: f64,
    pub hub_groove_depth_t2_mm: f64,
}



/// DIN 471 (external shaft) / DIN 472 (internal bore) circlip groove specification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetainingRingGrooveSpec {
    pub nominal_diameter_mm: f64,
    pub groove_diameter_d2_mm: f64,
    pub groove_width_m_mm: f64,
    pub ring_thickness_s_mm: f64,
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



/// ISO 3601 full O-ring gland specification including inner/outer gland diameters and fill ratio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OringGlandSpec {
    pub cord_diameter_mm: f64,
    pub inner_diameter_mm: f64,
    pub gland_inner_diameter_mm: f64,
    pub gland_outer_diameter_mm: f64,
    pub groove_depth_mm: f64,
    pub groove_width_mm: f64,
    pub compression_ratio: f64,
    pub gland_fill_ratio: f64,
    pub corner_radius_mm: f64,
}



/// Epicyclic planetary gearset kinematic and geometric specification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanetaryGearsetSpec {
    pub sun_teeth: usize,
    pub planet_teeth: usize,
    pub ring_teeth: usize,
    pub num_planets: usize,
    pub module_mm: f64,
    pub center_distance_mm: f64,
    pub sun_pitch_diameter_mm: f64,
    pub planet_pitch_diameter_mm: f64,
    pub ring_pitch_diameter_mm: f64,
    /// Reduction ratio with fixed ring gear, sun input, carrier output: $1 + Z_{\text{ring}} / Z_{\text{sun}}$.
    pub sun_to_carrier_ratio: f64,
    /// Clear gap between adjacent planet tip circles in millimeters.
    pub adjacent_planet_tip_clearance_mm: f64,
}

/// Computes epicyclic planetary gearset parameters and verifies concentricity ($Z_{\text{ring}} = Z_{\text{sun}} + 2 Z_{\text{planet}}$),
/// equal-spacing assembly condition ($(Z_{\text{sun}} + Z_{\text{ring}}) \bmod N_{\text{planets}} = 0$),
/// and adjacent planet tip clearance.
pub fn planetary_gearset_params(
    sun_teeth: usize,
    planet_teeth: usize,
    num_planets: usize,
    module_mm: f64,
) -> std::result::Result<PlanetaryGearsetSpec, String> {
    if sun_teeth < 6 || planet_teeth < 6 {
        return Err("Sun and planet tooth counts must be at least 6.".into());
    }
    if !(2..=8).contains(&num_planets) {
        return Err("Number of planets must be between 2 and 8.".into());
    }
    if !module_mm.is_finite() || !(0.1..=100.0).contains(&module_mm) {
        return Err("Module must be finite and between 0.1 and 100 mm.".into());
    }
    let ring_teeth = sun_teeth + 2 * planet_teeth;
    if !(sun_teeth + ring_teeth).is_multiple_of(num_planets) {
        return Err(format!(
            "Assembly condition failed: (Z_sun + Z_ring) = {} is not divisible by num_planets = {}.",
            sun_teeth + ring_teeth,
            num_planets
        ));
    }
    let zs = sun_teeth as f64;
    let zp = planet_teeth as f64;
    let zr = ring_teeth as f64;
    let center_distance_mm = 0.5 * module_mm * (zs + zp);
    let planet_tip_diameter_mm = module_mm * (zp + 2.0);
    let center_to_center = 2.0 * center_distance_mm * (PI / num_planets as f64).sin();
    let tip_clearance = center_to_center - planet_tip_diameter_mm;
    if tip_clearance <= 0.0 {
        return Err(format!(
            "Adjacent planet tips collide (clearance = {tip_clearance:.3} mm)."
        ));
    }
    Ok(PlanetaryGearsetSpec {
        sun_teeth,
        planet_teeth,
        ring_teeth,
        num_planets,
        module_mm,
        center_distance_mm,
        sun_pitch_diameter_mm: module_mm * zs,
        planet_pitch_diameter_mm: module_mm * zp,
        ring_pitch_diameter_mm: module_mm * zr,
        sun_to_carrier_ratio: 1.0 + zr / zs,
        adjacent_planet_tip_clearance_mm: tip_clearance,
    })
}

#[cfg(test)]
#[path = "standard_parts/tests/tests_integration.rs"]
mod tests;


#[path = "standard_parts/fastener_standards.rs"]
mod fastener_standards;
pub use fastener_standards::iso_metric_fastener;
pub use fastener_standards::heat_set_insert_spec;
pub use fastener_standards::hex_nut_profile;
pub use fastener_standards::shaft_keyway_din6885;
pub use fastener_standards::retaining_ring_groove;
pub use fastener_standards::o_ring_groove_spec;
pub use fastener_standards::iso286_shaft_housing_fit;
pub use fastener_standards::oring_gland_spec;

#[path = "standard_parts/spring_standards.rs"]
mod spring_standards;
pub use spring_standards::helical_spring_analysis;
pub use spring_standards::disc_spring_load;


/// Alias for [`shaft_keyway_din6885`]: standard parallel shaft key & keyway dimensions
/// ($b \times h$, shaft groove depth $t_1$, hub groove depth $t_2$) per DIN 6885 / ISO R773.
pub fn keyway_din_6885(shaft_diameter_mm: f64) -> Result<KeywaySpec> {
    shaft_keyway_din6885(shaft_diameter_mm)
}

/// Generates the exact 2D equidistant epitrochoid profile loop (`samples` vertices) for a cycloidal
/// speed reducer disc with `num_lobes` lobes ($N$), outer ring pin circle radius $R$ (`pin_circle_radius_mm`),
/// roller pin radius $R_r$ (`pin_radius_mm`), and eccentric shaft offset $e$ (`eccentricity_mm`).
pub fn cycloidal_disc_profile(
    pin_circle_radius_mm: f64,
    pin_radius_mm: f64,
    num_lobes: usize,
    eccentricity_mm: f64,
    samples: usize,
) -> Result<Vec<[f64; 2]>> {
    if !pin_circle_radius_mm.is_finite() || !(2.0..=2000.0).contains(&pin_circle_radius_mm) {
        return Err(err(
            "cycloidal_disc",
            "pin_circle_radius_mm must be finite and between 2 and 2000 mm.",
        ));
    }
    if !pin_radius_mm.is_finite()
        || pin_radius_mm <= 0.1
        || pin_radius_mm >= 0.5 * pin_circle_radius_mm
    {
        return Err(err(
            "cycloidal_disc",
            "pin_radius_mm must be positive and less than 0.5 * pin_circle_radius_mm.",
        ));
    }
    if !(3..=200).contains(&num_lobes) {
        return Err(err(
            "cycloidal_disc",
            "num_lobes must be between 3 and 200.",
        ));
    }
    let n_pins = (num_lobes + 1) as f64;
    let max_ecc = pin_circle_radius_mm / n_pins;
    if !eccentricity_mm.is_finite() || eccentricity_mm <= 0.01 || eccentricity_mm >= max_ecc {
        return Err(err(
            "cycloidal_disc",
            "eccentricity_mm must be positive and strictly less than pin_circle_radius_mm / (num_lobes + 1) to prevent cusps.",
        ));
    }
    if !(16..=8192).contains(&samples) {
        return Err(err(
            "cycloidal_disc",
            "samples must be between 16 and 8192.",
        ));
    }
    let r = pin_circle_radius_mm;
    let rr = pin_radius_mm;
    let e = eccentricity_mm;
    let n = num_lobes as f64;
    let mut profile = Vec::with_capacity(samples);
    for i in 0..samples {
        let theta = TAU * (i as f64) / (samples as f64);
        let (sin_t, cos_t) = theta.sin_cos();
        let (sin_nt, cos_nt) = (n * theta).sin_cos();
        let (sin_n1t, cos_n1t) = ((n + 1.0) * theta).sin_cos();
        // Contact angle psi between roller pin and trochoid normal:
        // tan(psi) = sin(N * theta) / (R / (e * (N + 1)) - cos(N * theta))
        let num = sin_nt;
        let den = r / (e * n_pins) - cos_nt;
        let psi = num.atan2(den);
        let x = r * cos_t - rr * (theta - psi).cos() - e * cos_n1t;
        let y = -r * sin_t + rr * (theta - psi).sin() + e * sin_n1t;
        profile.push([x, y]);
    }
    Ok(profile)
}

/// Computes DIN 471 external shaft retaining ring (circlip) groove diameter $d_2$, groove width $m$,
/// ring thickness $s$, and minimum shaft edge margin $n_{\min}$ for shaft diameters in $[3, 100]\text{ mm}$.
pub fn retaining_ring_din_471(shaft_diameter_mm: f64) -> Result<RetainingRingSpec> {
    if !shaft_diameter_mm.is_finite() || !(3.0..=100.0).contains(&shaft_diameter_mm) {
        return Err(err(
            "retaining_ring_din_471",
            "Shaft diameter for DIN 471 retaining ring must be between 3 and 100 mm.",
        ));
    }
    let d = shaft_diameter_mm;
    let (s, m, radial_depth) = if d < 5.0 {
        (0.4, 0.5, 0.2)
    } else if d < 7.0 {
        (0.7, 0.8, 0.25)
    } else if d < 10.0 {
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
    let d2 = d - 2.0 * radial_depth;
    let n_min = (3.0 * radial_depth).max(0.6);
    Ok(RetainingRingSpec {
        shaft_diameter_d1_mm: d,
        groove_diameter_d2_mm: d2,
        groove_width_m_mm: m,
        ring_thickness_s_mm: s,
        min_edge_margin_n_mm: n_min,
    })
}

/// Computes ISO 2338 cylindrical dowel pin m6 diameter tolerance limits, mating H7 locating hole
/// limits, end chamfer length, and recommended minimum press engagement depth.
pub fn dowel_pin_iso_2338(nominal_diameter_mm: f64, length_mm: f64) -> Result<DowelPinSpec> {
    if !nominal_diameter_mm.is_finite() || !(1.0..=50.0).contains(&nominal_diameter_mm) {
        return Err(err(
            "dowel_pin_iso_2338",
            "Nominal dowel pin diameter must be between 1.0 and 50.0 mm.",
        ));
    }
    if !length_mm.is_finite()
        || length_mm < 2.0 * nominal_diameter_mm
        || length_mm > 20.0 * nominal_diameter_mm
    {
        return Err(err(
            "dowel_pin_iso_2338",
            "Dowel pin length_mm must be between 2 * nominal_diameter_mm and 20 * nominal_diameter_mm.",
        ));
    }
    let d = nominal_diameter_mm;
    // ISO 286 m6 pin limits (um) and H7 hole limits (um) by diameter step
    let (m6_ei_um, m6_es_um, h7_es_um, chamfer_mm) = if d <= 3.0 {
        (2.0, 8.0, 10.0, 0.4)
    } else if d <= 6.0 {
        (4.0, 12.0, 12.0, 0.8)
    } else if d <= 10.0 {
        (6.0, 15.0, 15.0, 1.2)
    } else if d <= 18.0 {
        (7.0, 18.0, 18.0, 1.6)
    } else if d <= 30.0 {
        (8.0, 21.0, 21.0, 2.0)
    } else {
        (9.0, 25.0, 25.0, 2.5)
    };
    Ok(DowelPinSpec {
        nominal_diameter_mm: d,
        length_mm,
        pin_min_diameter_mm: d + m6_ei_um * 1e-3,
        pin_max_diameter_mm: d + m6_es_um * 1e-3,
        hole_h7_min_diameter_mm: d,
        hole_h7_max_diameter_mm: d + h7_es_um * 1e-3,
        chamfer_length_mm: chamfer_mm,
        recommended_min_engagement_mm: (1.5 * d).min(0.45 * length_mm),
    })
}

/// Computes ISO 2901 / DIN 103 metric trapezoidal thread ($30^\circ$ total thread angle, $\beta = 15^\circ$)
/// pitch diameter $d_2 = d - 0.5 p$, root diameter $d_3 = d - p - 2 a_c$, lead $l = p \cdot n$,
/// helix lead angle $\lambda$, thread-flank friction angle $\rho$, self-locking status ($\lambda < \rho$),
/// forward mechanical efficiency $\eta$, and raising torque per Newton of axial load.
pub fn lead_screw_trapezoidal_iso_2901(
    nominal_dia_mm: f64,
    pitch_mm: f64,
    starts: usize,
    friction_coeff: f64,
) -> Result<LeadScrewSpec> {
    if !nominal_dia_mm.is_finite() || !(6.0..=200.0).contains(&nominal_dia_mm) {
        return Err(err(
            "lead_screw",
            "nominal_dia_mm must be finite and between 6 and 200 mm.",
        ));
    }
    if !pitch_mm.is_finite()
        || !(1.0..=24.0).contains(&pitch_mm)
        || nominal_dia_mm <= 2.2 * pitch_mm
    {
        return Err(err(
            "lead_screw",
            "pitch_mm must be between 1 and 24 mm with nominal_dia_mm > 2.2 * pitch_mm.",
        ));
    }
    if !(1..=8).contains(&starts) {
        return Err(err("lead_screw", "starts must be between 1 and 8."));
    }
    if !friction_coeff.is_finite() || !(0.0..=0.8).contains(&friction_coeff) {
        return Err(err(
            "lead_screw",
            "friction_coeff must be finite and between 0.0 and 0.8.",
        ));
    }
    // ISO 2901 crest clearance a_c by pitch
    let a_c = if pitch_mm <= 1.5 {
        0.15
    } else if pitch_mm <= 5.0 {
        0.25
    } else if pitch_mm <= 12.0 {
        0.5
    } else {
        1.0
    };
    let d2 = nominal_dia_mm - 0.5 * pitch_mm;
    let d3 = nominal_dia_mm - pitch_mm - 2.0 * a_c;
    let lead = pitch_mm * (starts as f64);
    let lambda = (lead / (PI * d2)).atan();
    // ISO trapezoidal thread has 30 deg total angle (15 deg half-angle beta)
    let cos_beta = 15.0_f64.to_radians().cos();
    let mu_prime = friction_coeff / cos_beta;
    let rho = mu_prime.atan();
    let self_locking = lambda < rho;
    let eta = (lambda.tan() / (lambda + rho).tan()).clamp(0.0, 1.0);
    let torque_per_n_nm = 0.5 * (d2 * 1e-3) * (lambda + rho).tan();

    Ok(LeadScrewSpec {
        nominal_diameter_mm: nominal_dia_mm,
        pitch_mm,
        starts,
        pitch_diameter_d2_mm: d2,
        root_diameter_d3_mm: d3,
        lead_mm: lead,
        lead_angle_rad: lambda,
        lead_angle_deg: lambda.to_degrees(),
        friction_angle_rad: rho,
        friction_angle_deg: rho.to_degrees(),
        self_locking,
        forward_efficiency: eta,
        raising_torque_per_n_nm: torque_per_n_nm,
    })
}

/// DIN 471 external shaft retaining ring (circlip) and groove specification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetainingRingSpec {
    pub shaft_diameter_d1_mm: f64,
    pub groove_diameter_d2_mm: f64,
    pub groove_width_m_mm: f64,
    pub ring_thickness_s_mm: f64,
    /// Minimum shaft shoulder edge margin $n_{\min}$ (mm) required to prevent groove shear-out.
    pub min_edge_margin_n_mm: f64,
}

/// ISO 2338 (type m6) unhardened precision cylindrical dowel pin and H7 locating hole specification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DowelPinSpec {
    pub nominal_diameter_mm: f64,
    pub length_mm: f64,
    pub pin_min_diameter_mm: f64,
    pub pin_max_diameter_mm: f64,
    pub hole_h7_min_diameter_mm: f64,
    pub hole_h7_max_diameter_mm: f64,
    pub chamfer_length_mm: f64,
    pub recommended_min_engagement_mm: f64,
}

/// ISO 2901 / DIN 103 metric trapezoidal lead screw ($30^\circ$ flank angle) geometric and mechanical specification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeadScrewSpec {
    pub nominal_diameter_mm: f64,
    pub pitch_mm: f64,
    pub starts: usize,
    pub pitch_diameter_d2_mm: f64,
    pub root_diameter_d3_mm: f64,
    pub lead_mm: f64,
    pub lead_angle_rad: f64,
    pub lead_angle_deg: f64,
    pub friction_angle_rad: f64,
    pub friction_angle_deg: f64,
    pub self_locking: bool,
    pub forward_efficiency: f64,
    /// Raising torque required per Newton of axial load ($T / F$ in $\text{N}\cdot\text{m}/\text{N}$).
    pub raising_torque_per_n_nm: f64,
}

/// DIN EN 13906-1 cylindrical helical compression spring specification and stress report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompressionSpringSpec {
    pub wire_diameter_mm: f64,
    pub mean_coil_diameter_mm: f64,
    pub active_coils: f64,
    pub spring_index: f64,
    pub wahl_correction_factor: f64,
    pub spring_rate_n_per_mm: f64,
    pub deflection_mm: f64,
    pub corrected_shear_stress_mpa: f64,
    pub solid_length_mm: f64,
}

/// Performs a DIN EN 13906-1 cylindrical helical compression spring analysis, computing
/// spring index $C = D / d$, Wahl curvature & direct-shear factor $K_W = \frac{4C - 1}{4C - 4} + \frac{0.615}{C}$,
/// axial spring rate $k = \frac{G d^4}{8 D^3 n_a}$ (N/mm), axial deflection $s = F / k$ (mm),
/// and corrected maximum torsional shear stress $\tau_{\max} = K_W \frac{8 F D}{\pi d^3}$ (MPa).
pub fn helical_compression_spring_analysis(
    wire_dia_mm: f64,
    mean_coil_dia_mm: f64,
    active_coils: f64,
    shear_modulus_mpa: f64,
    axial_load_n: f64,
) -> Result<CompressionSpringSpec> {
    let report = helical_spring_analysis(&HelicalSpringOptions {
        wire_diameter_mm: wire_dia_mm,
        mean_coil_diameter_mm: mean_coil_dia_mm,
        active_coils,
        shear_modulus_mpa,
        axial_load_n,
    })?;
    Ok(CompressionSpringSpec {
        wire_diameter_mm: wire_dia_mm,
        mean_coil_diameter_mm: mean_coil_dia_mm,
        active_coils,
        spring_index: report.spring_index,
        wahl_correction_factor: report.wahl_factor,
        spring_rate_n_per_mm: report.spring_rate_n_per_mm,
        deflection_mm: report.deflection_mm,
        corrected_shear_stress_mpa: report.max_shear_stress_mpa,
        solid_length_mm: report.solid_length_mm,
    })
}

