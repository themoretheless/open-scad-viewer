#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PowerMode {
    #[default]
    ConstantM3,
    DynamicM4,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MachineProfile {
    pub width_mm: f64,
    pub height_mm: f64,
    pub max_power: u32,
    pub estimated_rapid_mm_min: f64,
    pub power_mode: PowerMode,
    pub laser_mode_confirmed: bool,
    pub supports_air_assist: bool,
    pub flip_y: bool,
    pub return_to_origin: bool,
}

impl Default for MachineProfile {
    fn default() -> Self {
        Self {
            width_mm: 400.0,
            height_mm: 400.0,
            max_power: 1_000,
            estimated_rapid_mm_min: 6_000.0,
            power_mode: PowerMode::ConstantM3,
            laser_mode_confirmed: false,
            supports_air_assist: false,
            flip_y: false,
            return_to_origin: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OperationKind {
    #[default]
    Line,
    Fill,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Path {
    pub points: Vec<[f64; 2]>,
    pub closed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Operation {
    pub name: String,
    pub kind: OperationKind,
    pub output: bool,
    pub speed_mm_min: f64,
    pub power: u32,
    pub passes: u32,
    pub air_assist: bool,
    pub paths: Vec<Path>,
}

impl Default for Operation {
    fn default() -> Self {
        Self {
            name: "Line".into(),
            kind: OperationKind::Line,
            output: true,
            speed_mm_min: 1_000.0,
            power: 100,
            passes: 1,
            air_assist: false,
            paths: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub machine: MachineProfile,
    pub operations: Vec<Operation>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: [f64; 2],
    pub max: [f64; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    pub operation_count: usize,
    pub path_count: usize,
    pub segment_count: usize,
    pub cut_distance_mm: f64,
    pub rapid_distance_mm: f64,
    pub estimated_time_s: f64,
    pub bounds: Bounds,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub gcode: String,
    pub summary: Summary,
}
