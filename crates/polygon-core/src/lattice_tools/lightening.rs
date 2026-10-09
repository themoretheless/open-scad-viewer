use super::*;

pub(super) fn clip_polygon(poly: Vec<[f64; 2]>, n: [f64; 2], d: f64) -> Vec<[f64; 2]> {
    let mut result = Vec::new();
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let da = a[0] * n[0] + a[1] * n[1] - d;
        let db = b[0] * n[0] + b[1] * n[1] - d;
        if da <= 1e-9 {
            result.push(a);
        }
        if (da < 0.) != (db < 0.) {
            let t = da / (da - db);
            result.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
        }
    }
    result
}

pub(super) fn inset_polygon(poly: Vec<[f64; 2]>, distance: f64) -> Vec<[f64; 2]> {
    let mut result = poly.clone();
    // Clipping may remove vertices; offset the original edges, not the result.
    for i in 0..poly.len() {
        if result.is_empty() {
            break;
        }
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let len = dx.hypot(dy);
        if len < 1e-8 {
            continue;
        }
        let n = [dy / len, -dx / len];
        let boundary = a[0] * n[0] + a[1] * n[1] - distance;
        result = clip_polygon(result, n, boundary);
    }
    result
}

pub(super) fn area(poly: &[[f64; 2]]) -> f64 {
    let s = poly.iter().enumerate().fold(0.0, |acc, (i, a)| {
        let b = poly[(i + 1) % poly.len()];
        acc + a[0] * b[1] - a[1] * b[0]
    });
    s.abs() * 0.5
}

#[cfg(test)]
mod cell_tests {
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
    if !cell.is_finite()
        || cell <= 0.
        || !rib.is_finite()
        || rib <= 0.
        || !seed0.is_finite()
        || seed0.fract() != 0.
    {
        return Err(input(
            "Lightening cells must use positive finite parameters.",
        ));
    }
    if !jitter.is_finite() || !(0. ..=1.).contains(&jitter) {
        return Err(input("Lightening jitter must be between 0 and 1."));
    }

    let mut cells = Vec::<Vec<[f64; 2]>>::new();
    let width = max[0] - min[0];
    let height = max[1] - min[1];
    let nx = (width / cell).ceil().max(1.);
    let ny = (height / cell).ceil().max(1.);
    if !nx.is_finite() || !ny.is_finite() {
        return Err(input("Lightening domain has non-finite size."));
    }
    let nx = nx as usize;
    let ny = ny as usize;
    let dx = width / nx as f64;
    let dy = height / ny as f64;
    let rect = |x: f64, y: f64, w: f64, h: f64| -> Vec<[f64; 2]> {
        vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]]
    };
    let boundary = rect(min[0], min[1], width, height);

    if pattern == "grid" || pattern == "triangles" {
        if nx.saturating_mul(ny) > 144 {
            return Err(input("More than 144 cells. Increase cell size."));
        }
        for y in 0..ny {
            for x in 0..nx {
                let p = rect(min[0] + x as f64 * dx, min[1] + y as f64 * dy, dx, dy);
                if pattern == "grid" {
                    cells.push(p);
                } else {
                    cells.push(vec![p[0], p[1], p[2]]);
                    cells.push(vec![p[0], p[2], p[3]]);
                }
            }
        }
    } else if pattern == "isogrid" {
        let h = cell * 3f64.sqrt() / 2.;
        if !min.iter().chain(max.iter()).all(|v| v.is_finite())
            || width <= 0.
            || height <= 0.
            || !h.is_finite()
            || h <= 0.
        {
            return Err(input("Isogrid requires finite increasing bounds."));
        }
        let cols = (width / cell).ceil() + 2.;
        let rows = (height / h).ceil() + 2.;
        if !cols.is_finite() || !rows.is_finite() || cols * rows * 2. > 144. {
            return Err(input("More than 144 cells. Increase cell size."));
        }
        let at = |i: i32, j: i32| {
            [
                min[0] + (i as f64 + j as f64 * 0.5) * cell,
                min[1] + j as f64 * h,
            ]
        };
        for j in -1i32..rows as i32 {
            for i in -j.div_euclid(2) - 2..cols as i32 {
                for tri in [
                    vec![at(i, j), at(i + 1, j), at(i, j + 1)],
                    vec![at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)],
                ] {
                    if tri.iter().flatten().any(|v| !v.is_finite()) {
                        return Err(input("Isogrid exceeds finite numeric range."));
                    }
                    let mut polygon = tri;
                    for (normal, bound) in [
                        ([1., 0.], max[0]),
                        ([-1., 0.], -min[0]),
                        ([0., 1.], max[1]),
                        ([0., -1.], -min[1]),
                    ] {
                        polygon = clip_polygon(polygon, normal, bound);
                    }
                    if polygon.len() >= 3 && area(&polygon) > rib * rib / 4. {
                        cells.push(polygon);
                    }
                }
            }
        }
    } else if pattern == "honeycomb" || pattern == "web" {
        let mut seed = seed0.rem_euclid(4294967296.) as u32;
        let mut random = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            seed as f64 / 4294967296.
        };

        let rows = (height / (cell * 3f64.sqrt() / 2.)).ceil().max(1.) as usize;
        if nx.saturating_mul(rows) > 144 {
            return Err(input("More than 144 sites. Increase cell size."));
        }

        let mut sites = Vec::<[f64; 2]>::new();
        for y in 0..rows {
            for x in 0..nx {
                let jitter_x = if pattern == "web" { jitter } else { 0. };
                let jitter_y = jitter_x;
                let xx = min[0]
                    + (x as f64 + 0.25 + (y % 2) as f64 * 0.5 + (random() - 0.5) * jitter_x) * dx;
                let yy = min[1]
                    + (y as f64 + 0.5 + (random() - 0.5) * jitter_y) * (height / rows as f64);
                sites.push([xx, yy]);
            }
        }

        for a in &sites {
            let mut polygon = boundary.clone();
            for b in &sites {
                if a == b {
                    continue;
                }
                let n = [b[0] - a[0], b[1] - a[1]];
                let d = (b[0] * b[0] + b[1] * b[1] - a[0] * a[0] - a[1] * a[1]) / 2.;
                polygon = clip_polygon(polygon, n, d);
                if polygon.len() < 3 {
                    break;
                }
            }
            cells.push(polygon);
        }
    } else {
        return Err(input("Unknown lightening pattern."));
    }

    Ok(cells
        .into_iter()
        .map(|p| inset_polygon(p, rib / 2.))
        .filter(|p| p.len() >= 3 && area(p) > rib * rib / 8.)
        .collect::<Vec<_>>())
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
        let mut reduced_mesh = crate::mesh_shell::lattice(
            &source_mesh,
            nodes,
            edges,
            rib / 2.,
            skin,
            step,
            pattern == "bone",
            open_top,
            wall_depth,
            keep_core,
        )?
        .mesh;
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
