#[derive(Clone, Debug, PartialEq)]
pub struct GcodeMove {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub e: f64,
    pub f_mm_min: f64,
    pub extruded: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThermalState {
    pub hotend_c: f64,
    pub bed_c: f64,
    pub chamber_c: f64,
    pub fan_pwm: f64,
    pub wait: bool,
    pub tool: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GcodePreview {
    pub layers: usize,
    pub extrusion_mm: f64,
    pub deposited_volume_mm3: f64,
    pub moves: Vec<GcodeMove>,
    pub thermal: ThermalState,
    pub thermals: Vec<ThermalState>,
    pub other_commands: usize,
}

impl GcodePreview {
    pub fn with_volume(mut self, volume_mm3: f64) -> Self {
        self.deposited_volume_mm3 = volume_mm3;
        self
    }
}
