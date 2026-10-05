use laser_core::{
    Bounds, MachineProfile, Operation, OperationKind, Path, Plan, PowerMode, Program, Summary,
};
use value_codec::{Value, json};

use crate::{Result, input, require_exact_fields};

const REQUEST_FIELDS: &[&str] = &["op", "machine", "operations"];
const MACHINE_FIELDS: &[&str] = &[
    "widthMm",
    "heightMm",
    "maxPower",
    "estimatedRapidMmMin",
    "powerMode",
    "laserModeConfirmed",
    "supportsAirAssist",
    "flipY",
    "returnToOrigin",
];
const OPERATION_FIELDS: &[&str] = &[
    "name",
    "kind",
    "output",
    "speedMmMin",
    "power",
    "passes",
    "airAssist",
    "paths",
];

pub(crate) fn dispatch(value: &Value) -> Result<Value> {
    let plan = parse_plan(value)?;
    match value["op"].as_str().unwrap_or_default() {
        "laser_preflight" => Ok(json!({"summary": summary_value(laser_core::preflight(&plan)?)})),
        "laser_grbl" => program_value(
            "open-scad-viewer/laser-grbl 1",
            laser_core::generate_grbl(&plan)?,
        ),
        "laser_frame" => program_value(
            "open-scad-viewer/laser-frame 1",
            laser_core::generate_frame(&plan)?,
        ),
        _ => Err(input("Unknown laser operation")),
    }
}

fn parse_plan(value: &Value) -> Result<Plan> {
    require_exact_fields(value, REQUEST_FIELDS, "laser request")?;
    let machine = &value["machine"];
    require_exact_fields(machine, MACHINE_FIELDS, "laser machine")?;
    let power_mode = match string(machine, "powerMode")? {
        "m3" => PowerMode::ConstantM3,
        "m4" => PowerMode::DynamicM4,
        _ => return Err(input("laser machine powerMode must be m3 or m4")),
    };
    let operations = value["operations"]
        .as_array()
        .ok_or_else(|| input("laser operations must be an array"))?
        .iter()
        .enumerate()
        .map(|(index, operation)| parse_operation(operation, index))
        .collect::<Result<Vec<_>>>()?;
    Ok(Plan {
        machine: MachineProfile {
            width_mm: number(machine, "widthMm")?,
            height_mm: number(machine, "heightMm")?,
            max_power: unsigned(machine, "maxPower")?,
            estimated_rapid_mm_min: number(machine, "estimatedRapidMmMin")?,
            power_mode,
            laser_mode_confirmed: boolean(machine, "laserModeConfirmed")?,
            supports_air_assist: boolean(machine, "supportsAirAssist")?,
            flip_y: boolean(machine, "flipY")?,
            return_to_origin: boolean(machine, "returnToOrigin")?,
        },
        operations,
    })
}

fn parse_operation(value: &Value, index: usize) -> Result<Operation> {
    let label = format!("laser operation {index}");
    require_exact_fields(value, OPERATION_FIELDS, &label)?;
    let kind = match string(value, "kind")? {
        "line" => OperationKind::Line,
        "fill" => OperationKind::Fill,
        _ => return Err(input(format!("{label} kind must be line or fill"))),
    };
    let paths = value["paths"]
        .as_array()
        .ok_or_else(|| input(format!("{label} paths must be an array")))?
        .iter()
        .enumerate()
        .map(|(path_index, path)| parse_path(path, index, path_index))
        .collect::<Result<Vec<_>>>()?;
    Ok(Operation {
        name: string(value, "name")?.to_owned(),
        kind,
        output: boolean(value, "output")?,
        speed_mm_min: number(value, "speedMmMin")?,
        power: unsigned(value, "power")?,
        passes: unsigned(value, "passes")?,
        air_assist: boolean(value, "airAssist")?,
        paths,
    })
}

fn parse_path(value: &Value, operation_index: usize, path_index: usize) -> Result<Path> {
    let label = format!("laser operation {operation_index} path {path_index}");
    require_exact_fields(value, &["points", "closed"], &label)?;
    let points = value["points"]
        .as_array()
        .ok_or_else(|| input(format!("{label} points must be an array")))?
        .iter()
        .enumerate()
        .map(|(point_index, point)| {
            let coordinates = point
                .as_array()
                .filter(|coordinates| coordinates.len() == 2)
                .ok_or_else(|| input(format!("{label} point {point_index} must be [x, y]")))?;
            let x = coordinates[0]
                .as_f64()
                .ok_or_else(|| input(format!("{label} point {point_index} x must be a number")))?;
            let y = coordinates[1]
                .as_f64()
                .ok_or_else(|| input(format!("{label} point {point_index} y must be a number")))?;
            Ok([x, y])
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Path {
        points,
        closed: boolean(value, "closed")?,
    })
}

fn string<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value[field]
        .as_str()
        .ok_or_else(|| input(format!("laser {field} must be a string")))
}

fn number(value: &Value, field: &str) -> Result<f64> {
    value[field]
        .as_f64()
        .ok_or_else(|| input(format!("laser {field} must be a number")))
}

fn unsigned(value: &Value, field: &str) -> Result<u32> {
    value[field]
        .as_u64()
        .and_then(|number| u32::try_from(number).ok())
        .ok_or_else(|| input(format!("laser {field} must be an unsigned 32-bit integer")))
}

fn boolean(value: &Value, field: &str) -> Result<bool> {
    value[field]
        .as_bool()
        .ok_or_else(|| input(format!("laser {field} must be boolean")))
}

fn program_value(dialect: &str, program: Program) -> Result<Value> {
    Ok(json!({
        "dialect": dialect,
        "gcode": program.gcode,
        "summary": summary_value(program.summary),
    }))
}

fn summary_value(summary: Summary) -> Value {
    json!({
        "operationCount": summary.operation_count,
        "pathCount": summary.path_count,
        "segmentCount": summary.segment_count,
        "cutDistanceMm": summary.cut_distance_mm,
        "rapidDistanceMm": summary.rapid_distance_mm,
        "estimatedTimeS": summary.estimated_time_s,
        "bounds": bounds_value(summary.bounds),
    })
}

fn bounds_value(bounds: Bounds) -> Value {
    json!({"min": bounds.min, "max": bounds.max})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(op: &str) -> Value {
        json!({
            "op": op,
            "machine": {
                "widthMm": 400,
                "heightMm": 300,
                "maxPower": 1000,
                "estimatedRapidMmMin": 6000,
                "powerMode": "m4",
                "laserModeConfirmed": true,
                "supportsAirAssist": true,
                "flipY": false,
                "returnToOrigin": true,
            },
            "operations": [{
                "name": "Outline",
                "kind": "line",
                "output": true,
                "speedMmMin": 900,
                "power": 250,
                "passes": 2,
                "airAssist": true,
                "paths": [{"points": [[10, 20], [30, 20], [30, 40]], "closed": true}],
            }],
        })
    }

    #[test]
    fn bridge_emits_job_preflight_and_laser_off_frame() {
        let job = dispatch(&request("laser_grbl")).unwrap();
        assert_eq!(
            job["dialect"].as_str(),
            Some("open-scad-viewer/laser-grbl 1")
        );
        assert!(job["gcode"].as_str().unwrap().contains("M4 S250"));
        assert_eq!(job["summary"]["operationCount"].as_u64(), Some(1));
        assert_eq!(job["summary"]["segmentCount"].as_u64(), Some(6));

        let preflight = dispatch(&request("laser_preflight")).unwrap();
        assert_eq!(preflight["summary"], job["summary"]);

        let mut frame_request = request("laser_frame");
        frame_request["machine"]["laserModeConfirmed"] = json!(false);
        frame_request["operations"][0]["power"] = json!(0);
        frame_request["operations"][0]["passes"] = json!(0);
        let frame = dispatch(&frame_request).unwrap();
        let gcode = frame["gcode"].as_str().unwrap();
        assert!(!gcode.contains("M3"));
        assert!(!gcode.contains("M4"));
        assert!(!gcode.contains("G1"));
        assert_eq!(gcode.matches("G0 ").count(), 5);
        assert_eq!(frame["summary"]["operationCount"].as_u64(), Some(0));
        assert_eq!(frame["summary"]["pathCount"].as_u64(), Some(1));
        assert_eq!(frame["summary"]["segmentCount"].as_u64(), Some(5));
    }

    #[test]
    fn bridge_rejects_schema_smuggling_and_unconfirmed_jobs() {
        let mut extra = request("laser_grbl");
        extra["machine"]["unexpected"] = json!(true);
        assert_eq!(dispatch(&extra).unwrap_err().code, "GEOMETRY_INVALID_INPUT");

        let mut unconfirmed = request("laser_grbl");
        unconfirmed["machine"]["laserModeConfirmed"] = json!(false);
        assert_eq!(
            dispatch(&unconfirmed).unwrap_err().code,
            "LASER_MODE_UNCONFIRMED"
        );
    }
}
