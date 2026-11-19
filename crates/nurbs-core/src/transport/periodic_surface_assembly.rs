use super::*;
use crate::editing::exact_periodic_surface as periodic;

pub(super) const OPS: &[&str] = &[
    "assemble_exact_periodic_surface_axis",
    "assemble_periodic_surface_axis_with_exact_weight_scaling",
    "assemble_periodic_surface_axis_with_exact_knot_union",
    "assemble_periodic_surface_axis_with_exact_basis_matching",
    "assemble_periodic_surface_axis_with_exact_degree_matching",
    "assemble_exact_doubly_periodic_surface",
    "assemble_doubly_periodic_surface_with_exact_weight_scaling",
];

pub(super) fn dispatch(v: &Value, op: &str) -> Result<Value> {
    let inputs: Vec<Surface> = field(v, "inputs")?;
    let work: u64 = field(v, "max_work")?;
    if matches!(
        op,
        "assemble_exact_doubly_periodic_surface"
            | "assemble_doubly_periodic_surface_with_exact_weight_scaling"
    ) {
        let columns: usize = field(v, "columns")?;
        check(
            !inputs.is_empty() && inputs.len() <= 256 && columns > 0 && inputs.len() % columns == 0,
            "Expected a rectangular patch grid",
        )?;
        let mut source = inputs.into_iter();
        let mut rows = Vec::new();
        while source.len() > 0 {
            rows.push(source.by_ref().take(columns).collect());
        }
        let report = if op == "assemble_exact_doubly_periodic_surface" {
            periodic::assemble_exact_doubly_periodic_surface(&rows, work)?
        } else {
            periodic::assemble_doubly_periodic_surface_with_exact_weight_scaling(&rows, work)?
        };
        return encode(report.surface.ok_or_else(|| {
            input(format!(
                "Periodic grid assembly refused: {:?}",
                report.refusal
            ))
        })?);
    }
    let axis = field(v, "axis")?;
    let report = match op {
        "assemble_exact_periodic_surface_axis" => {
            periodic::assemble_exact_periodic_surface_axis(&inputs, axis, work)?
        }
        "assemble_periodic_surface_axis_with_exact_weight_scaling" => {
            periodic::assemble_periodic_surface_axis_with_exact_weight_scaling(&inputs, axis, work)?
        }
        "assemble_periodic_surface_axis_with_exact_knot_union" => {
            periodic::assemble_periodic_surface_axis_with_exact_knot_union(&inputs, axis, work)?
        }
        "assemble_periodic_surface_axis_with_exact_basis_matching" => {
            periodic::assemble_periodic_surface_axis_with_exact_basis_matching(&inputs, axis, work)?
        }
        "assemble_periodic_surface_axis_with_exact_degree_matching" => {
            periodic::assemble_periodic_surface_axis_with_exact_degree_matching(
                &inputs, axis, work,
            )?
        }
        _ => return Err(input("Unknown periodic assembly operation")),
    };
    encode(report.surface.ok_or_else(|| {
        input(format!(
            "Periodic strip assembly refused: {:?}",
            report.refusal
        ))
    })?)
}
