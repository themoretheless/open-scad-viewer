use std::fmt::Write;

use math_core::{Error, Result};

use crate::{MachineProfile, OperationKind, PowerMode, Program, Summary, planner::PreparedPlan};

const MAX_OUTPUT_BYTES: usize = 64 * 1024 * 1024;

pub(crate) fn compile_job(prepared: PreparedPlan, machine: &MachineProfile) -> Result<Program> {
    let mut out = String::new();
    line(&mut out, "; open-scad-viewer/laser-grbl 1")?;
    line(&mut out, "G21")?;
    line(&mut out, "G90")?;
    line(&mut out, "M5")?;

    for operation in &prepared.operations {
        bounded_writeln(
            &mut out,
            format_args!(
                "; operation: {} ({})",
                safe_comment(&operation.name),
                match operation.kind {
                    OperationKind::Line => "line",
                    OperationKind::Fill => "fill",
                }
            ),
        )?;
        if operation.air_assist {
            line(&mut out, "M8")?;
        }
        for pass in 0..operation.passes {
            bounded_writeln(
                &mut out,
                format_args!("; pass: {}/{}", pass + 1, operation.passes),
            )?;
            for path in &operation.paths {
                let first = path.points[0];
                bounded_writeln(
                    &mut out,
                    format_args!("G0 X{} Y{}", coordinate(first[0]), coordinate(first[1])),
                )?;
                bounded_writeln(
                    &mut out,
                    format_args!(
                        "{} S{}",
                        match machine.power_mode {
                            PowerMode::ConstantM3 => "M3",
                            PowerMode::DynamicM4 => "M4",
                        },
                        operation.power
                    ),
                )?;
                for point in path.points.iter().skip(1) {
                    cut_move(&mut out, *point, operation.speed_mm_min)?;
                }
                if path.closed {
                    cut_move(&mut out, first, operation.speed_mm_min)?;
                }
                line(&mut out, "M5")?;
            }
        }
        if operation.air_assist {
            line(&mut out, "M9")?;
        }
    }
    if machine.return_to_origin {
        line(&mut out, "G0 X0.000 Y0.000")?;
    }
    line(&mut out, "M5")?;

    Ok(Program {
        gcode: out,
        summary: prepared.summary,
    })
}

pub(crate) fn compile_frame(prepared: PreparedPlan, machine: &MachineProfile) -> Result<Program> {
    let min = prepared.summary.bounds.min;
    let max = prepared.summary.bounds.max;
    let rapid_distance_mm =
        distance([0.0, 0.0], min) + 2.0 * ((max[0] - min[0]).abs() + (max[1] - min[1]).abs());
    let mut out = String::new();
    line(&mut out, "; open-scad-viewer/laser-frame 1")?;
    line(&mut out, "G21")?;
    line(&mut out, "G90")?;
    line(&mut out, "M5")?;
    for point in [min, [max[0], min[1]], max, [min[0], max[1]], min] {
        bounded_writeln(
            &mut out,
            format_args!("G0 X{} Y{}", coordinate(point[0]), coordinate(point[1])),
        )?;
    }
    line(&mut out, "M5")?;
    Ok(Program {
        gcode: out,
        summary: Summary {
            operation_count: 0,
            path_count: 1,
            segment_count: 5,
            cut_distance_mm: 0.0,
            rapid_distance_mm,
            estimated_time_s: rapid_distance_mm / machine.estimated_rapid_mm_min * 60.0,
            bounds: prepared.summary.bounds,
        },
    })
}

fn distance(from: [f64; 2], to: [f64; 2]) -> f64 {
    (to[0] - from[0]).hypot(to[1] - from[1])
}

fn cut_move(out: &mut String, point: [f64; 2], feed: f64) -> Result<()> {
    bounded_writeln(
        out,
        format_args!(
            "G1 X{} Y{} F{feed:.3}",
            coordinate(point[0]),
            coordinate(point[1])
        ),
    )
}

fn coordinate(value: f64) -> String {
    format!("{value:.3}")
}

fn safe_comment(value: &str) -> String {
    value
        .chars()
        .filter(|character| !matches!(character, '\r' | '\n' | ';' | '(' | ')'))
        .take(120)
        .collect()
}

fn line(out: &mut String, value: &str) -> Result<()> {
    bounded_writeln(out, format_args!("{value}"))
}

fn bounded_writeln(out: &mut String, arguments: std::fmt::Arguments<'_>) -> Result<()> {
    let before = out.len();
    writeln!(out, "{arguments}").expect("String writes cannot fail");
    if out.len() > MAX_OUTPUT_BYTES {
        out.truncate(before);
        return Err(Error::new(
            "LASER_OUTPUT_LIMIT",
            "Laser G-code exceeds 64 MiB",
        ));
    }
    Ok(())
}
