//! Bounded offline laser-job planning and deterministic GRBL emission.
//!
//! This crate owns no UI, document, serial, network or printer transport. Hosts
//! provide already-flattened Line/Fill paths and may send only a successfully
//! compiled immutable [`Program`] through a separately qualified transport.

mod gcode;
mod model;
mod planner;

pub use math_core::{Error, Result};
pub use model::{
    Bounds, KerfMode, MachineProfile, Operation, OperationKind, Path, PathOrder, Plan, PowerMode,
    Preview, PreviewOperation, Program, Summary,
};

/// Validate and summarize the exact geometry that [`generate_grbl`] would emit.
/// Laser mode confirmation is required because this is job readiness.
pub fn preflight(plan: &Plan) -> Result<Summary> {
    Ok(preview(plan)?.summary)
}

/// Return the exact quantized, compensated and ordered paths used by job
/// compilation. Hosts should render these paths rather than the source plan.
pub fn preview(plan: &Plan) -> Result<Preview> {
    preview_prepared(planner::prepare(plan, true)?)
}

/// Return exact frame geometry without requiring job-only controller/process
/// confirmation. This follows the same safety boundary as [`generate_frame`].
pub fn preview_frame(plan: &Plan) -> Result<Preview> {
    let prepared = planner::prepare(plan, false)?;
    let bounds = prepared.summary.bounds;
    Ok(Preview {
        summary: gcode::frame_summary(&prepared, &plan.machine),
        operations: vec![PreviewOperation {
            name: "Frame".into(),
            kind: OperationKind::Line,
            paths: vec![Path {
                points: vec![
                    bounds.min,
                    [bounds.max[0], bounds.min[1]],
                    bounds.max,
                    [bounds.min[0], bounds.max[1]],
                ],
                closed: true,
            }],
        }],
    })
}

fn preview_prepared(prepared: planner::PreparedPlan) -> Result<Preview> {
    Ok(Preview {
        summary: prepared.summary,
        operations: prepared
            .operations
            .into_iter()
            .map(|operation| PreviewOperation {
                name: operation.name,
                kind: operation.kind,
                paths: operation
                    .paths
                    .into_iter()
                    .map(|path| Path {
                        points: path.points,
                        closed: path.closed,
                    })
                    .collect(),
            })
            .collect(),
    })
}

/// Compile a deterministic GRBL laser program. This fails closed until the
/// caller confirms `$32=1` and any requested M8/M9 support.
pub fn generate_grbl(plan: &Plan) -> Result<Program> {
    let prepared = planner::prepare(plan, true)?;
    gcode::compile_job(prepared, &plan.machine)
}

/// Compile a separately validated, laser-off rectangular bounds frame.
/// The result contains setup/comments, `M5`, and `G0` only.
pub fn generate_frame(plan: &Plan) -> Result<Program> {
    gcode::compile_frame(planner::prepare(plan, false)?, &plan.machine)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_plan() -> Plan {
        Plan {
            machine: MachineProfile {
                laser_mode_confirmed: true,
                ..MachineProfile::default()
            },
            operations: vec![Operation {
                name: "Outline".into(),
                paths: vec![Path {
                    points: vec![[10.0, 20.0], [30.0, 20.0], [30.0, 40.0], [10.0, 40.0]],
                    closed: true,
                }],
                ..Operation::default()
            }],
        }
    }

    #[test]
    fn deterministic_job_uses_grbl_laser_commands() {
        let plan = square_plan();
        let first = generate_grbl(&plan).unwrap();
        let second = generate_grbl(&plan).unwrap();
        assert_eq!(first, second);
        assert!(
            first
                .gcode
                .starts_with("; open-scad-viewer/laser-grbl 1\nG21\nG90\nM5\n")
        );
        assert!(first.gcode.contains("M3 S100\n"));
        assert!(first.gcode.contains("G1 X30.000 Y20.000 F1000.000\n"));
        assert_eq!(first.summary.segment_count, 4);
        assert_eq!(first.summary.cut_distance_mm, 80.0);
    }

    #[test]
    fn job_requires_laser_mode_but_frame_remains_available() {
        let mut plan = square_plan();
        plan.machine.laser_mode_confirmed = false;
        assert_eq!(
            generate_grbl(&plan).unwrap_err().code,
            "LASER_MODE_UNCONFIRMED"
        );
        plan.machine.max_power = 0;
        plan.operations[0].speed_mm_min = f64::NAN;
        plan.operations[0].passes = 0;
        plan.operations[0].air_assist = true;
        let frame = generate_frame(&plan).unwrap();
        assert!(!frame.gcode.contains("M3"));
        assert!(!frame.gcode.contains("M4"));
        assert!(!frame.gcode.contains("G1"));
        assert!(!frame.gcode.lines().any(|line| line.starts_with('S')));
        assert_eq!(frame.gcode.matches("G0 ").count(), 5);
        assert!(frame.summary.estimated_time_s.is_finite());
        assert_eq!(frame.summary.operation_count, 0);
        assert_eq!(frame.summary.path_count, 1);
        assert_eq!(frame.summary.segment_count, 5);
        assert_eq!(frame.summary.cut_distance_mm, 0.0);
    }

    #[test]
    fn dynamic_power_air_assist_and_passes_are_scoped() {
        let mut plan = square_plan();
        plan.machine.power_mode = PowerMode::DynamicM4;
        plan.machine.supports_air_assist = true;
        plan.operations[0].air_assist = true;
        plan.operations[0].passes = 2;
        let program = generate_grbl(&plan).unwrap();
        assert_eq!(program.gcode.matches("M4 S100").count(), 2);
        assert_eq!(program.gcode.matches("M8\n").count(), 1);
        assert_eq!(program.gcode.matches("M9\n").count(), 1);
        assert_eq!(program.summary.segment_count, 8);
        assert_eq!(program.summary.cut_distance_mm, 160.0);
    }

    #[test]
    fn coordinates_are_flipped_and_quantized_once() {
        let mut plan = square_plan();
        plan.machine.height_mm = 100.0;
        plan.machine.flip_y = true;
        plan.operations[0].paths[0].points[0] = [10.000_49, 20.000_49];
        let program = generate_grbl(&plan).unwrap();
        assert!(program.gcode.contains("G0 X10.000 Y80.000\n"));
        assert_eq!(program.summary.bounds.min, [10.0, 60.0]);
        assert_eq!(program.summary.bounds.max, [30.0, 80.0]);
    }

    #[test]
    fn invalid_or_collapsed_geometry_fails_closed() {
        let mut plan = square_plan();
        plan.operations[0].paths[0].points[1][0] = f64::NAN;
        assert_eq!(generate_grbl(&plan).unwrap_err().code, "LASER_INVALID_PATH");

        let mut collapsed = square_plan();
        collapsed.operations[0].paths[0] = Path {
            points: vec![[1.0, 1.0], [1.000_1, 1.000_1], [1.000_2, 1.000_2]],
            closed: true,
        };
        assert_eq!(
            generate_grbl(&collapsed).unwrap_err().code,
            "LASER_COLLAPSED_PATH"
        );
    }

    #[test]
    fn out_of_bounds_and_unconfirmed_air_assist_fail_closed() {
        let mut out = square_plan();
        out.operations[0].paths[0].points[0][0] = -0.01;
        assert_eq!(generate_grbl(&out).unwrap_err().code, "LASER_OUT_OF_BOUNDS");

        let mut air = square_plan();
        air.operations[0].air_assist = true;
        assert_eq!(
            generate_grbl(&air).unwrap_err().code,
            "LASER_AIR_ASSIST_UNCONFIRMED"
        );
    }

    #[test]
    fn disabled_operations_do_not_affect_output_or_bounds() {
        let mut plan = square_plan();
        plan.operations.push(Operation {
            name: "Disabled".into(),
            output: false,
            paths: vec![Path {
                points: vec![[-100.0, -100.0], [-50.0, -50.0]],
                closed: false,
            }],
            ..Operation::default()
        });
        let summary = preflight(&plan).unwrap();
        assert_eq!(summary.operation_count, 1);
        assert_eq!(summary.path_count, 1);
        assert_eq!(summary.bounds.min, [10.0, 20.0]);
    }

    #[test]
    fn part_kerf_grows_outer_boundary_and_shrinks_hole() {
        let mut plan = square_plan();
        plan.operations[0].kerf_mm = 0.2;
        plan.operations[0].kerf_mode = KerfMode::Part;
        plan.operations[0].paths = vec![
            Path {
                points: vec![[10.0, 10.0], [50.0, 10.0], [50.0, 50.0], [10.0, 50.0]],
                closed: true,
            },
            Path {
                // Same winding as the outer ring: laser nesting is geometric.
                points: vec![[20.0, 20.0], [30.0, 20.0], [30.0, 30.0], [20.0, 30.0]],
                closed: true,
            },
        ];
        let program = generate_grbl(&plan).unwrap();
        assert_eq!(program.summary.bounds.min, [9.9, 9.9]);
        assert_eq!(program.summary.bounds.max, [50.1, 50.1]);
        assert!(program.gcode.contains("X20.100 Y20.100"));
    }

    #[test]
    fn inner_first_nearest_cuts_holes_before_outer_boundaries() {
        let mut plan = square_plan();
        plan.operations[0].path_order = PathOrder::InnerFirstNearest;
        plan.operations[0].paths = vec![
            Path {
                points: vec![[10.0, 10.0], [50.0, 10.0], [50.0, 50.0], [10.0, 50.0]],
                closed: true,
            },
            Path {
                points: vec![[20.0, 20.0], [30.0, 20.0], [30.0, 30.0], [20.0, 30.0]],
                closed: true,
            },
        ];
        let program = generate_grbl(&plan).unwrap();
        let first_rapid = program
            .gcode
            .lines()
            .find(|line| line.starts_with("G0 "))
            .unwrap();
        assert_eq!(first_rapid, "G0 X20.000 Y20.000");
    }

    #[test]
    fn nearest_order_reduces_rapid_travel_and_can_reverse_open_paths() {
        let mut preserved = square_plan();
        preserved.machine.return_to_origin = false;
        preserved.operations[0].paths = vec![
            Path {
                points: vec![[100.0, 0.0], [110.0, 0.0]],
                closed: false,
            },
            Path {
                points: vec![[10.0, 0.0], [5.0, 0.0]],
                closed: false,
            },
        ];
        let preserved_program = generate_grbl(&preserved).unwrap();
        let mut optimized = preserved;
        optimized.operations[0].path_order = PathOrder::Nearest;
        let optimized_program = generate_grbl(&optimized).unwrap();
        assert!(
            optimized_program.summary.rapid_distance_mm
                < preserved_program.summary.rapid_distance_mm
        );
        assert!(optimized_program.gcode.contains("G0 X5.000 Y0.000\n"));
    }

    #[test]
    fn kerf_compensation_rejects_open_paths() {
        let mut plan = square_plan();
        plan.operations[0].paths[0].closed = false;
        plan.operations[0].kerf_mm = 0.2;
        plan.operations[0].kerf_mode = KerfMode::Part;
        assert_eq!(
            generate_grbl(&plan).unwrap_err().code,
            "LASER_KERF_REQUIRES_CLOSED_LINE"
        );
    }
}
