use super::{invalid, Result, MAX_TEMPLATE_BYTES};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExtrusionMode {
    #[default]
    AbsoluteFilament,
    RelativeFilament,
    VolumetricMm3,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Units {
    #[default]
    Millimeters,
    Inches,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MachineProfile {
    pub layer_height_mm: f64,
    pub line_width_mm: f64,
    pub print_feedrate_mm_s: f64,
    pub travel_feedrate_mm_s: f64,
    pub filament_diameter_mm: f64,
    pub hotend_c: f64,
    pub bed_c: f64,
    pub chamber_c: f64,
    pub fan_pwm: f64,
    pub retract_mm: f64,
    pub retract_feedrate_mm_s: f64,
    pub z_hop_mm: f64,
    pub firmware_retract_mm: f64,
    pub gcode_rounding_mm: f64,
    /// When `Some`, `G2`/`G3` are linearized to this chord error (mm). `None` rejects arcs.
    pub linearize_arcs_mm: Option<f64>,
    pub xyz_absolute: bool,
    pub e_mode: ExtrusionMode,
    pub units: Units,
    pub start_gcode: String,
    pub end_gcode: String,
}

impl Default for MachineProfile {
    fn default() -> Self {
        Self {
            layer_height_mm: 0.2,
            line_width_mm: 0.4,
            print_feedrate_mm_s: 50.0,
            travel_feedrate_mm_s: 120.0,
            filament_diameter_mm: 1.75,
            hotend_c: 0.0,
            bed_c: 0.0,
            chamber_c: 0.0,
            fan_pwm: 0.0,
            retract_mm: 0.0,
            retract_feedrate_mm_s: 25.0,
            z_hop_mm: 0.0,
            firmware_retract_mm: 1.0,
            gcode_rounding_mm: 1e-5,
            linearize_arcs_mm: None,
            xyz_absolute: true,
            e_mode: ExtrusionMode::AbsoluteFilament,
            units: Units::Millimeters,
            start_gcode: String::new(),
            end_gcode: String::new(),
        }
    }
}

impl MachineProfile {
    pub fn validate(&self) -> Result<()> {
        for (value, name) in [
            (self.layer_height_mm, "layer height"),
            (self.line_width_mm, "line width"),
            (self.print_feedrate_mm_s, "print feedrate"),
            (self.travel_feedrate_mm_s, "travel feedrate"),
            (self.filament_diameter_mm, "filament diameter"),
            (self.retract_feedrate_mm_s, "retract feedrate"),
            (self.firmware_retract_mm, "firmware retract"),
            (self.gcode_rounding_mm, "G-code rounding"),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(invalid(
                    "GCODE_INVALID_SETTINGS",
                    &format!("Machine profile {name} must be finite and positive"),
                ));
            }
        }
        for (value, name) in [
            (self.hotend_c, "hotend"),
            (self.bed_c, "bed"),
            (self.chamber_c, "chamber"),
            (self.fan_pwm, "fan"),
            (self.retract_mm, "retract"),
            (self.z_hop_mm, "z-hop"),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(invalid(
                    "GCODE_INVALID_SETTINGS",
                    &format!("Machine profile {name} must be finite and non-negative"),
                ));
            }
        }
        if let Some(chord) = self.linearize_arcs_mm
            && (!chord.is_finite() || chord <= 0.0)
        {
            return Err(invalid(
                "GCODE_INVALID_SETTINGS",
                "Machine profile arc linearization must be finite and positive",
            ));
        }
        if self.fan_pwm > 255.0 {
            return Err(invalid(
                "GCODE_INVALID_SETTINGS",
                "Fan PWM must be at most 255",
            ));
        }
        if self.start_gcode.len() > MAX_TEMPLATE_BYTES || self.end_gcode.len() > MAX_TEMPLATE_BYTES {
            return Err(invalid(
                "GCODE_TEMPLATE_LIMIT",
                "Start/end G-code exceeds 64 KiB",
            ));
        }
        Ok(())
    }

    pub fn bead_area_mm2(&self) -> f64 {
        self.line_width_mm * self.layer_height_mm
    }

    pub fn filament_area_mm2(&self) -> f64 {
        std::f64::consts::PI * (self.filament_diameter_mm * 0.5).powi(2)
    }

    pub fn xyz_decimals(&self) -> usize {
        decimals_for(self.gcode_rounding_mm).max(3)
    }
}

fn decimals_for(step: f64) -> usize {
    if step >= 1.0 {
        return 0;
    }
    let digits = (-step.log10()).ceil() as i32;
    digits.clamp(0, 7) as usize
}
