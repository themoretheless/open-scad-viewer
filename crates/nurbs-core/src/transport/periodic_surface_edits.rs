//! Rush transport for exact operations on periodic tensor-product surfaces.
use super::*;
use crate::editing::exact_periodic_surface as periodic;

pub(super) fn dispatch(v: &Value, op: &str) -> Result<Value> {
    let source: Surface = field(v, "surface")?;
    let work: u64 = field(v, "max_work")?;
    match op {
        "rotate_periodic_surface_at_exact_knot" => {
            let report = periodic::rotate_periodic_surface_at_exact_knot(
                &source,
                field(v, "axis")?,
                field(v, "parameter")?,
                work,
            )?;
            let geometry = report
                .surface
                .ok_or_else(|| input(format!("Periodic edit refused: {:?}", report.refusal)))?;
            encode(geometry)
        }
        "insert_and_rotate_periodic_surface_seam" => {
            let report = periodic::insert_and_rotate_periodic_surface_seam(
                &source,
                field(v, "axis")?,
                field(v, "parameter")?,
                work,
            )?;
            let geometry = report
                .surface
                .ok_or_else(|| input(format!("Periodic edit refused: {:?}", report.refusal)))?;
            encode(geometry)
        }
        "open_exact_periodic_surface_axis" => {
            let report =
                periodic::open_exact_periodic_surface_axis(&source, field(v, "axis")?, work)?;
            let geometry = report
                .surface
                .ok_or_else(|| input(format!("Periodic edit refused: {:?}", report.refusal)))?;
            encode(geometry)
        }
        "restrict_exact_periodic_surface_axis" => {
            let report = periodic::restrict_exact_periodic_surface_axis(
                &source,
                field(v, "axis")?,
                field(v, "interval")?,
                work,
            )?;
            let geometry = report
                .surface
                .ok_or_else(|| input(format!("Periodic edit refused: {:?}", report.refusal)))?;
            encode(geometry)
        }
        "restrict_exact_periodic_surface_over_turns" => {
            let report = periodic::restrict_exact_periodic_surface_over_turns(
                &source,
                field(v, "axis")?,
                field(v, "interval")?,
                work,
            )?;
            let geometry = report
                .surface
                .ok_or_else(|| input(format!("Periodic edit refused: {:?}", report.refusal)))?;
            encode(geometry)
        }
        "restrict_exact_doubly_periodic_surface" => {
            let report = periodic::restrict_exact_doubly_periodic_surface(
                &source,
                field(v, "u")?,
                field(v, "v")?,
                work,
            )?;
            let geometry = report
                .surface
                .ok_or_else(|| input(format!("Periodic edit refused: {:?}", report.refusal)))?;
            encode(geometry)
        }
        "restrict_exact_doubly_periodic_surface_over_turns" => {
            let report = periodic::restrict_exact_doubly_periodic_surface_over_turns(
                &source,
                field(v, "u")?,
                field(v, "v")?,
                work,
            )?;
            let geometry = report
                .surface
                .ok_or_else(|| input(format!("Periodic edit refused: {:?}", report.refusal)))?;
            encode(geometry)
        }
        "partition_exact_periodic_surface_axis" => {
            let report = periodic::partition_exact_periodic_surface_axis(
                &source,
                field(v, "axis")?,
                field(v, "seam")?,
                &field::<Vec<f64>>(v, "cuts")?,
                work,
            )?;
            let geometry = report
                .strips
                .ok_or_else(|| input(format!("Periodic edit refused: {:?}", report.refusal)))?;
            encode(
                geometry
                    .get(field::<usize>(v, "part")?)
                    .ok_or_else(|| input("Periodic strip index outside partition"))?,
            )
        }
        "partition_exact_periodic_surface_over_turns" => {
            let report = periodic::partition_exact_periodic_surface_over_turns(
                &source,
                field(v, "axis")?,
                field(v, "interval")?,
                &field::<Vec<f64>>(v, "cuts")?,
                work,
            )?;
            let geometry = report
                .strips
                .ok_or_else(|| input(format!("Periodic edit refused: {:?}", report.refusal)))?;
            encode(
                geometry
                    .get(field::<usize>(v, "part")?)
                    .ok_or_else(|| input("Periodic strip index outside partition"))?,
            )
        }
        "partition_exact_doubly_periodic_surface" => {
            let report = periodic::partition_exact_doubly_periodic_surface(
                &source,
                field(v, "seams")?,
                &field::<Vec<f64>>(v, "u_cuts")?,
                &field::<Vec<f64>>(v, "v_cuts")?,
                work,
            )?;
            let geometry = report
                .patches
                .ok_or_else(|| input(format!("Periodic edit refused: {:?}", report.refusal)))?;
            let u = field::<usize>(v, "u_index")?;
            let v = field::<usize>(v, "v_index")?;
            encode(
                geometry
                    .get(u)
                    .and_then(|row| row.get(v))
                    .ok_or_else(|| input("Periodic grid index outside patch set"))?,
            )
        }
        "partition_exact_doubly_periodic_surface_over_turns" => {
            let report = periodic::partition_exact_doubly_periodic_surface_over_turns(
                &source,
                field(v, "u")?,
                field(v, "v")?,
                &field::<Vec<f64>>(v, "u_cuts")?,
                &field::<Vec<f64>>(v, "v_cuts")?,
                work,
            )?;
            let geometry = report
                .patches
                .ok_or_else(|| input(format!("Periodic edit refused: {:?}", report.refusal)))?;
            let u = field::<usize>(v, "u_index")?;
            let v = field::<usize>(v, "v_index")?;
            encode(
                geometry
                    .get(u)
                    .and_then(|row| row.get(v))
                    .ok_or_else(|| input("Periodic grid index outside patch set"))?,
            )
        }
        "remove_exact_periodic_surface_knot" => {
            let report = periodic::remove_exact_periodic_surface_knot(
                &source,
                field(v, "axis")?,
                field(v, "knot")?,
                work,
            )?;
            let geometry = report
                .surface
                .ok_or_else(|| input(format!("Periodic edit refused: {:?}", report.refusal)))?;
            encode(geometry)
        }
        "simplify_exact_periodic_surface_axis" => {
            let report =
                periodic::simplify_exact_periodic_surface_axis(&source, field(v, "axis")?, work)?;
            let geometry = report
                .surface
                .ok_or_else(|| input(format!("Periodic edit refused: {:?}", report.refusal)))?;
            encode(geometry)
        }
        "remove_exact_periodic_surface_seam_knot" => {
            let report = periodic::remove_exact_periodic_surface_seam_knot(
                &source,
                field(v, "axis")?,
                field(v, "anchor")?,
                work,
            )?;
            let geometry = report
                .surface
                .ok_or_else(|| input(format!("Periodic edit refused: {:?}", report.refusal)))?;
            encode(geometry)
        }
        "simplify_exact_doubly_periodic_surface" => {
            let report = periodic::simplify_exact_doubly_periodic_surface(&source, work)?;
            let geometry = report.surface.ok_or_else(|| {
                input(format!(
                    "Periodic edit refused: {:?}",
                    report.refusal.map(|(axis, reason)| (
                        match axis {
                            crate::surface::Axis::U => "u",
                            crate::surface::Axis::V => "v",
                        },
                        reason
                    ))
                ))
            })?;
            encode(geometry)
        }
        "reduce_exact_periodic_surface_period" => {
            let report =
                periodic::reduce_exact_periodic_surface_period(&source, field(v, "axis")?, work)?;
            let geometry = report
                .surface
                .ok_or_else(|| input(format!("Periodic edit refused: {:?}", report.refusal)))?;
            encode(geometry)
        }
        "repeat_exact_periodic_surface_period" => {
            let report = periodic::repeat_exact_periodic_surface_period(
                &source,
                field(v, "axis")?,
                field(v, "turns")?,
                work,
            )?;
            let geometry = report
                .surface
                .ok_or_else(|| input(format!("Periodic edit refused: {:?}", report.refusal)))?;
            encode(geometry)
        }
        _ => Err(input("Unknown exact periodic surface edit")),
    }
}
