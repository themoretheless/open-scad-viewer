use super::voronoi::{GradientMode, VoronoiLatticeOptions, generate_voronoi_cells};
use super::*;

#[cfg(test)]
mod cell_tests {
    use super::super::voronoi::inset_polygon;
    use super::*;

    #[test]
    fn triangle_inset_offsets_each_original_edge_once() {
        let polygon = vec![[0., 0.], [8., 0.], [4., 4. * 3f64.sqrt()]];
        let inset = inset_polygon(polygon, 0.5);
        assert_eq!(inset.len(), 3);
        for i in 0..3 {
            let a = inset[i];
            let b = inset[(i + 1) % 3];
            assert!(((a[0] - b[0]).hypot(a[1] - b[1]) - (8. - 3f64.sqrt())).abs() < 1e-10);
        }
    }

    #[test]
    fn oversized_inset_returns_empty_without_indexing_removed_vertices() {
        assert!(inset_polygon(vec![[0., 0.], [1., 0.], [0., 1.]], 2.).is_empty());
    }

    #[test]
    fn honeycomb_does_not_apply_web_jitter() {
        let regular =
            generate_lightening_cells([0., 0.], [1., 9.], "honeycomb", 6., 0.2, 42., 0.).unwrap();
        let candidate =
            generate_lightening_cells([0., 0.], [1., 9.], "honeycomb", 6., 0.2, 99., 1.).unwrap();
        assert_eq!(candidate, regular);
    }
}

pub fn generate_lightening_cells(
    min: [f64; 2],
    max: [f64; 2],
    pattern: &str,
    cell: f64,
    rib: f64,
    seed0: f64,
    jitter: f64,
) -> Result<Vec<Vec<[f64; 2]>>> {
    generate_voronoi_cells(&VoronoiLatticeOptions {
        min,
        max,
        pattern: pattern.to_string(),
        cell,
        rib,
        seed: seed0,
        jitter,
        overhang_angle: None,
        stress_gradient: 0.0,
        gradient_mode: GradientMode::None,
        corner_radius: 0.0,
        slope: 0.0,
        frame: 0.0,
        keep_out: Vec::new(),
        max_sites: 144,
    })
}

#[derive(Clone, Debug)]
pub struct LighteningOptions {
    pub pattern: String,
    pub axis: String,
    pub cell: f64,
    pub rib: f64,
    pub rim: f64,
    pub bottom: f64,
    pub top: f64,
    pub seed: f64,
    pub jitter: f64,
    pub line_width: f64,
    pub perimeters: f64,
    pub skin: f64,
    pub step: f64,
    pub wall_depth: f64,
    pub open_top: bool,
    pub keep_core: bool,
    pub diagonals: bool,
}
pub fn lighten(source_mesh: Mesh, options: LighteningOptions) -> Result<Mesh> {
    use crate::solid::{
        boolean::{Operation, Options, boolean},
        modeling::{Profile, extrude},
    };
    let LighteningOptions {
        pattern,
        axis,
        cell,
        rib,
        rim,
        bottom,
        top,
        seed: seed0,
        jitter,
        line_width,
        perimeters,
        skin,
        step,
        wall_depth,
        open_top,
        keep_core,
        diagonals,
    } = options;
    if ![
        cell, rib, rim, bottom, top, seed0, jitter, line_width, perimeters,
    ]
    .iter()
    .all(|x| x.is_finite())
        || cell <= 0.
        || rib <= 0.
        || rib >= cell / 2.
        || rim < 0.
        || bottom < 0.
        || top < 0.
        || !(0. ..=1.).contains(&jitter)
        || perimeters.fract() != 0.
        || perimeters < 1.
        || perimeters > 8.
        || seed0.fract() != 0.
        || line_width <= 0.
    {
        return Err(input(
            "Check cell size, rib width, borders, seed and print settings. Rib must be smaller than half a cell.",
        ));
    }
    if rib + 1e-6 < line_width * perimeters {
        return Err(input(
            "Rib is thinner than the requested number of extrusion lines. Increase rib width or change the print settings.",
        ));
    }
    if matches!(pattern.as_str(), "bone" | "spatial" | "bcc" | "octet") {
        let g = graph(
            source_mesh.clone(),
            cell,
            jitter,
            seed0,
            &pattern,
            diagonals,
        )
        .map_err(|e| input(format!("Cad lightening graph generation failed: {e}")))?;
        let nodes = g.nodes;
        let edges = g.edges;
        let style = crate::mesh_shell::ShellStyle { skin, organic: pattern == "bone", open_top, wall_depth, keep_core };
        let mut reduced_mesh = crate::mesh_shell::lattice(&source_mesh, nodes, edges, rib / 2., step, style)?.mesh;
        reduced_mesh = decimate(reduced_mesh, step * 1.5, 2600.)?;
        let compact_positions = reduced_mesh
            .positions
            .iter()
            .map(|v| (v * 1e6).round() / 1e6)
            .collect::<Vec<_>>();
        reduced_mesh = Mesh {
            positions: compact_positions,
            indices: reduced_mesh.indices,
            uv: reduced_mesh.uv,
        };
        let reduced = reduced_mesh.inspect()?;
        let source = source_mesh.inspect()?;
        if !reduced.closed
            || reduced.degenerate_triangles > 0
            || reduced.signed_volume_mm3 <= 0.
            || reduced.signed_volume_mm3 >= source.signed_volume_mm3
        {
            return Err(input(
                "Lattice simplification failed topology checks. Increase grid resolution.",
            ));
        }
        if component_count(&reduced_mesh, true)? > component_count(&source_mesh, true)? {
            return Err(input(
                "Clipping the spatial graph creates disconnected pieces. Increase strut thickness or add a skin.",
            ));
        }
        Ok(reduced_mesh)
    } else {
        let axis_index = match axis.as_str() {
            "x" => 0,
            "y" => 1,
            "z" => 2,
            _ => return Err(input("Choose X, Y or Z channel direction.")),
        };
        let u = (axis_index + 1) % 3;
        let v_index = (axis_index + 2) % 3;
        let (min, max) =
            crate::scene_flatten::bounds(std::slice::from_ref(&source_mesh.positions))?;

        if max[axis_index] - min[axis_index] <= bottom + top
            || max[u] - min[u] <= 2. * rim + rib
            || max[v_index] - min[v_index] <= 2. * rim + rib
        {
            return Err(input("Skins or frame consume the available body."));
        }

        let before = source_mesh.inspect()?;
        if !before.closed || before.signed_volume_mm3 <= 0. {
            return Err(input("Select a closed outward-oriented solid."));
        }

        let cells = generate_lightening_cells(
            [min[u] + rim, min[v_index] + rim],
            [max[u] - rim, max[v_index] - rim],
            &pattern,
            cell,
            rib,
            seed0,
            jitter,
        )?;
        if cells.is_empty() {
            return Err(input("No openings fit. Reduce rib or frame width."));
        }

        let extension = (max
            .iter()
            .zip(min.iter())
            .map(|(a, b)| a - b)
            .fold(0.0, f64::max))
            * 1e-5
            + 1e-5;
        let start = min[axis_index] + bottom - if bottom == 0. { extension } else { 0. };
        let end = max[axis_index] - top + if top == 0. { extension } else { 0. };
        let height = end - start;
        if !height.is_finite() || height <= 0. {
            return Err(input("Skins or frame consume the available body."));
        }

        let mut cutters = Mesh {
            positions: Vec::new(),
            indices: Vec::new(),
            uv: None,
        };

        let transform = match axis_index {
            0 => [
                [0., 0., 1., start],
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 0., 1.],
            ],
            1 => [
                [1., 0., 0., 0.],
                [0., 0., 1., start],
                [0., 1., 0., 0.],
                [0., 0., 0., 1.],
            ],
            2 => [
                [1., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., start],
                [0., 0., 0., 1.],
            ],
            _ => unreachable!(),
        };

        for cell in cells {
            let mut cutter = extrude(
                &Profile {
                    outer: cell,
                    holes: vec![],
                },
                [0., 0., height],
            )?
            .mesh;
            cutter = cutter.transform(transform)?;
            let offset = cutters.positions.len() / 3;
            cutters.positions.extend(cutter.positions);
            cutters
                .indices
                .extend(cutter.indices.into_iter().map(|i| i + offset));
        }

        let result = boolean(
            &source_mesh,
            &cutters,
            Operation::Difference,
            &Options::default(),
        )?;
        if result.mesh.indices.is_empty() {
            return Err(input("Lightening removed the entire body."));
        }
        if !result.mesh.inspect()?.closed || result.mesh.inspect()?.signed_volume_mm3 <= 0. {
            return Err(input("Lightening did not produce a closed solid."));
        }
        if result.mesh.inspect()?.signed_volume_mm3 >= before.signed_volume_mm3 - 1e-6 {
            return Err(input(
                "No material was removed. Change channel direction or cell size.",
            ));
        }
        if component_count(&result.mesh, false)? > component_count(&source_mesh, false)? {
            return Err(input(
                "Pattern creates disconnected pieces. Increase the frame/rib width or keep a bottom skin.",
            ));
        }

        let compact_positions = result
            .mesh
            .positions
            .iter()
            .map(|v| (v * 1e6).round() / 1e6)
            .collect::<Vec<_>>();
        let mesh = crate::Mesh {
            positions: compact_positions,
            indices: result.mesh.indices,
            uv: None,
        };
        let check = mesh.inspect()?;
        if !check.closed || check.degenerate_triangles != 0 {
            return Err(input("Pattern creates details below export precision."));
        }

        Ok(mesh)
    }
}
