use math_core::{Error, Result};

use crate::{Bounds, MachineProfile, OperationKind, Plan, Summary};

pub(crate) const COORDINATE_RESOLUTION_MM: f64 = 0.001;
const MIN_MACHINE_MM: f64 = 1.0;
const MAX_MACHINE_MM: f64 = 10_000.0;
const MIN_FEED_MM_MIN: f64 = 1.0;
const MAX_FEED_MM_MIN: f64 = 100_000.0;
const MAX_POWER: u32 = 100_000;
const MAX_PASSES: u32 = 100;
const MAX_PATHS: usize = 100_000;
const MAX_SEGMENTS: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PreparedPath {
    pub(crate) points: Vec<[f64; 2]>,
    pub(crate) closed: bool,
}

impl PreparedPath {
    pub(crate) fn for_each_segment(&self, mut visit: impl FnMut([f64; 2], [f64; 2])) {
        for pair in self.points.windows(2) {
            visit(pair[0], pair[1]);
        }
        if self.closed {
            visit(*self.points.last().expect("validated path"), self.points[0]);
        }
    }

    fn segment_count(&self) -> usize {
        self.points.len() - 1 + usize::from(self.closed)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PreparedOperation {
    pub(crate) name: String,
    pub(crate) kind: OperationKind,
    pub(crate) speed_mm_min: f64,
    pub(crate) power: u32,
    pub(crate) passes: u32,
    pub(crate) air_assist: bool,
    pub(crate) paths: Vec<PreparedPath>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PreparedPlan {
    pub(crate) operations: Vec<PreparedOperation>,
    pub(crate) summary: Summary,
}

pub(crate) fn prepare(plan: &Plan, require_laser_mode: bool) -> Result<PreparedPlan> {
    validate_machine(&plan.machine, require_laser_mode)?;

    let mut operations = Vec::new();
    let mut bounds: Option<Bounds> = None;
    let mut path_count = 0usize;
    let mut segment_count = 0usize;
    let mut cut_distance_mm = 0.0;
    let mut rapid_distance_mm = 0.0;
    let mut head = [0.0, 0.0];

    for operation in plan.operations.iter().filter(|operation| operation.output) {
        validate_operation(operation, &plan.machine, require_laser_mode)?;
        let effective_passes = if require_laser_mode {
            operation.passes
        } else {
            1
        };
        let mut paths = Vec::new();
        for path in &operation.paths {
            let prepared = prepare_path(path, &plan.machine)?;
            path_count = path_count.checked_add(1).ok_or_else(path_limit)?;
            if path_count > MAX_PATHS {
                return Err(path_limit());
            }
            let base_segments = prepared.segment_count();
            let executed_segments = base_segments
                .checked_mul(effective_passes as usize)
                .ok_or_else(segment_limit)?;
            segment_count = segment_count
                .checked_add(executed_segments)
                .ok_or_else(segment_limit)?;
            if segment_count > MAX_SEGMENTS {
                return Err(segment_limit());
            }

            for point in &prepared.points {
                admit_point(&mut bounds, *point);
            }
            for _ in 0..effective_passes {
                rapid_distance_mm += distance(head, prepared.points[0]);
                prepared.for_each_segment(|from, to| cut_distance_mm += distance(from, to));
                head = if prepared.closed {
                    prepared.points[0]
                } else {
                    *prepared.points.last().expect("validated path")
                };
            }
            paths.push(prepared);
        }
        if !paths.is_empty() {
            operations.push(PreparedOperation {
                name: operation.name.clone(),
                kind: operation.kind,
                speed_mm_min: operation.speed_mm_min,
                power: operation.power,
                passes: effective_passes,
                air_assist: operation.air_assist,
                paths,
            });
        }
    }

    let bounds =
        bounds.ok_or_else(|| invalid("LASER_EMPTY_JOB", "Laser plan has no output geometry"))?;
    if plan.machine.return_to_origin {
        rapid_distance_mm += distance(head, [0.0, 0.0]);
    }
    let cut_time_s = operations.iter().fold(0.0, |sum, operation| {
        let one_pass = operation.paths.iter().fold(0.0, |distance_sum, path| {
            let mut path_distance = 0.0;
            path.for_each_segment(|from, to| path_distance += distance(from, to));
            distance_sum + path_distance
        });
        sum + one_pass * f64::from(operation.passes) / operation.speed_mm_min * 60.0
    });
    let estimated_time_s =
        cut_time_s + rapid_distance_mm / plan.machine.estimated_rapid_mm_min * 60.0;

    Ok(PreparedPlan {
        summary: Summary {
            operation_count: operations.len(),
            path_count,
            segment_count,
            cut_distance_mm,
            rapid_distance_mm,
            estimated_time_s,
            bounds,
        },
        operations,
    })
}

fn validate_machine(machine: &MachineProfile, require_laser_mode: bool) -> Result<()> {
    let valid_dimension =
        |value: f64| value.is_finite() && (MIN_MACHINE_MM..=MAX_MACHINE_MM).contains(&value);
    if !valid_dimension(machine.width_mm) || !valid_dimension(machine.height_mm) {
        return Err(invalid(
            "LASER_INVALID_MACHINE",
            "Laser work area must be finite and within 1..10000 mm per axis",
        ));
    }
    if require_laser_mode && (machine.max_power == 0 || machine.max_power > MAX_POWER) {
        return Err(invalid(
            "LASER_INVALID_MACHINE",
            "Laser max power must be within 1..100000",
        ));
    }
    if !machine.estimated_rapid_mm_min.is_finite()
        || !(MIN_FEED_MM_MIN..=MAX_FEED_MM_MIN).contains(&machine.estimated_rapid_mm_min)
    {
        return Err(invalid(
            "LASER_INVALID_MACHINE",
            "Estimated rapid speed must be within 1..100000 mm/min",
        ));
    }
    if require_laser_mode && !machine.laser_mode_confirmed {
        return Err(invalid(
            "LASER_MODE_UNCONFIRMED",
            "Confirm that the GRBL controller has laser mode enabled ($32=1)",
        ));
    }
    Ok(())
}

fn validate_operation(
    operation: &crate::Operation,
    machine: &MachineProfile,
    require_laser_mode: bool,
) -> Result<()> {
    if !require_laser_mode {
        return Ok(());
    }
    if !operation.speed_mm_min.is_finite()
        || !(MIN_FEED_MM_MIN..=MAX_FEED_MM_MIN).contains(&operation.speed_mm_min)
    {
        return Err(invalid(
            "LASER_INVALID_OPERATION",
            "Operation speed must be within 1..100000 mm/min",
        ));
    }
    if operation.passes == 0 || operation.passes > MAX_PASSES {
        return Err(invalid(
            "LASER_INVALID_OPERATION",
            "Operation passes must be within 1..100",
        ));
    }
    if operation.power == 0 || operation.power > machine.max_power {
        return Err(invalid(
            "LASER_INVALID_OPERATION",
            "Operation power must be within 1..machine max power",
        ));
    }
    if operation.air_assist && !machine.supports_air_assist {
        return Err(invalid(
            "LASER_AIR_ASSIST_UNCONFIRMED",
            "Confirm controller support before emitting M8/M9 air-assist commands",
        ));
    }
    Ok(())
}

fn prepare_path(path: &crate::Path, machine: &MachineProfile) -> Result<PreparedPath> {
    if path.points.len() < 2 {
        return Err(invalid(
            "LASER_INVALID_PATH",
            "Laser paths need at least two points",
        ));
    }
    let mut points = Vec::with_capacity(path.points.len());
    for point in &path.points {
        if !point.iter().all(|value| value.is_finite()) {
            return Err(invalid(
                "LASER_INVALID_PATH",
                "Laser path coordinates must be finite",
            ));
        }
        let mapped = [
            quantize(point[0]),
            quantize(if machine.flip_y {
                machine.height_mm - point[1]
            } else {
                point[1]
            }),
        ];
        if mapped[0] < 0.0
            || mapped[1] < 0.0
            || mapped[0] > machine.width_mm
            || mapped[1] > machine.height_mm
        {
            return Err(invalid(
                "LASER_OUT_OF_BOUNDS",
                "Laser geometry lies outside the configured work area",
            ));
        }
        if points.last() != Some(&mapped) {
            points.push(mapped);
        }
    }
    if points.len() < 2 || (path.closed && points.len() < 3) {
        return Err(invalid(
            "LASER_COLLAPSED_PATH",
            "Laser path collapsed at 0.001 mm output precision",
        ));
    }
    if path.closed && points.first() == points.last() {
        points.pop();
        if points.len() < 3 {
            return Err(invalid(
                "LASER_COLLAPSED_PATH",
                "Closed laser path needs three distinct output points",
            ));
        }
    }
    Ok(PreparedPath {
        points,
        closed: path.closed,
    })
}

fn quantize(value: f64) -> f64 {
    (value / COORDINATE_RESOLUTION_MM).round() * COORDINATE_RESOLUTION_MM
}

fn admit_point(bounds: &mut Option<Bounds>, point: [f64; 2]) {
    match bounds {
        Some(bounds) => {
            bounds.min[0] = bounds.min[0].min(point[0]);
            bounds.min[1] = bounds.min[1].min(point[1]);
            bounds.max[0] = bounds.max[0].max(point[0]);
            bounds.max[1] = bounds.max[1].max(point[1]);
        }
        None => {
            *bounds = Some(Bounds {
                min: point,
                max: point,
            });
        }
    }
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    (b[0] - a[0]).hypot(b[1] - a[1])
}

fn invalid(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}

fn path_limit() -> Error {
    invalid("LASER_PATH_LIMIT", "Laser plan exceeds 100000 paths")
}

fn segment_limit() -> Error {
    invalid(
        "LASER_SEGMENT_LIMIT",
        "Laser plan exceeds 1000000 executed segments",
    )
}
