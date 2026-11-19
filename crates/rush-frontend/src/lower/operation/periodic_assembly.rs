use super::*;

pub(super) const OPS: &[&str] = &[
    "assemble_exact_periodic_surface_axis",
    "assemble_periodic_surface_axis_with_exact_weight_scaling",
    "assemble_periodic_surface_axis_with_exact_knot_union",
    "assemble_periodic_surface_axis_with_exact_basis_matching",
    "assemble_periodic_surface_axis_with_exact_degree_matching",
    "assemble_exact_doubly_periodic_surface",
    "assemble_doubly_periodic_surface_with_exact_weight_scaling",
];

impl Compiler {
    pub(super) fn periodic_assembly_operation(
        &mut self,
        name: &str,
        input: Option<String>,
        mut args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        let grid = matches!(
            name,
            "assemble_exact_doubly_periodic_surface"
                | "assemble_doubly_periodic_surface_with_exact_weight_scaling"
        );
        let key = if grid { "columns" } else { "direction" };
        let mode = raw(take_named(
            &mut args,
            key,
            "Missing periodic assembly layout or axis",
        )?)?;
        let budget = raw(take_named(
            &mut args,
            "max_work",
            "Missing periodic assembly work budget",
        )?)?;
        let inputs =
            self.collect_positional_geometry(input, args, "Use positional surfaces for assembly")?;
        let mut node = json!({"op":name, "inputs":inputs, "max_work":budget});
        node[key] = mode;
        self.add(node)
    }
}
