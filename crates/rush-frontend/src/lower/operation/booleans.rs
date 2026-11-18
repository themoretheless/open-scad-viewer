use super::*;

pub(super) const POSITIONAL_AND_BOOLEAN_OPS: &[&str] = &[
    "surface_sweep",
    "surface_loft",
    "ruled_surface",
    "triangular_patch",
    "coons_patch",
    "curve_compose",
    "sdf_union",
    "sdf_intersection",
    "sdf_difference",
    "sdf_smooth_union",
    "mesh_union",
    "brep_union",
    "brep_subtract",
    "brep_intersection",
    "brep_xor",
    "mesh_intersection",
    "mesh_subtract",
    "union",
    "intersection",
    "subtract",
    "hull",
];

impl Compiler {
    pub(super) fn boolean_or_patch_operation(
        &mut self,
        name: &str,
        input: Option<String>,
        mut args: Vec<(Option<String>, V)>,
    ) -> R<V> {
        let smooth = name == "sdf_smooth_union";
        let radius = if smooth {
            take_optional(&mut args, "radius")
        } else {
            None
        };
        let err_msg = if name.starts_with("sdf_") {
            "SDF operands are positional; smooth radius is named"
        } else {
            "Boolean arguments are positional"
        };
        let inputs = self.collect_positional_geometry(input, args, err_msg)?;
        if name.starts_with("sdf_") && (inputs.len() != 2 || smooth && radius.is_none()) {
            return Err(crate::error(
                "SDF operation requires two fields; smooth union also requires radius",
            ));
        }
        if name.starts_with("brep_") {
            if inputs.len() != 2 {
                return Err(crate::error("B-rep Boolean requires exactly two bodies"));
            }
            let op = match name {
                "brep_union" => "union",
                "brep_intersection" => "intersection",
                "brep_xor" => "xor",
                _ => "difference",
            };
            return self.add(json!({"op":"brep_boolean","inputs":inputs,"operation":op}));
        }
        if name.starts_with("mesh_") {
            if inputs.len() != 2 {
                return Err(crate::error(
                    "Mesh Boolean operation requires exactly two meshes",
                ));
            }
            let op = match name {
                "mesh_union" => "union",
                "mesh_intersection" => "intersection",
                _ => "difference",
            };
            return self.add(json!({"op":"mesh_boolean","inputs":inputs,"operation":op}));
        }
        if inputs.is_empty() && ["union", "intersection", "subtract", "hull"].contains(&name) {
            return Err(crate::error("Boolean operation needs geometry"));
        }
        let mut node = if name == "subtract" {
            json!({"op":"difference","base":&inputs[0],"subtract":&inputs[1..]})
        } else {
            json!({"op":name,"inputs":inputs})
        };
        if let Some(v) = radius {
            node["radius"] = raw(v)?;
        }
        self.add(node)
    }
}
