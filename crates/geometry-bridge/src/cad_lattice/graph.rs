use super::*;

pub(super) fn generate_lightening_cells(
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

pub fn graph(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    mesh.validate()?;
    let options: Value = field(&v, "options")?;
    let cell: f64 = field(&options, "cell")?;
    let jitter: f64 = field(&options, "jitter")?;
    let seed: f64 = field(&options, "seed")?;
    let pattern: String = field(&options, "pattern")?;
    let diagonals = if options.get("diagonals").is_some() {
        field::<bool>(&options, "diagonals")?
    } else {
        false
    };
    if !cell.is_finite()
        || cell <= 0.
        || !jitter.is_finite()
        || !(0. ..=1.).contains(&jitter)
        || !seed.is_finite()
        || seed.fract() != 0.
    {
        return Err(input(
            "Spatial graph requires positive cell size, jitter between 0 and 1, and an integer seed.",
        ));
    }
    if !["spatial", "bone", "bcc", "octet"].contains(&pattern.as_str()) {
        return Err(input("Invalid spatial lattice pattern."));
    }

    let (min, max) = polygon_core::scene_flatten::bounds(&[mesh.positions])?;
    let mut cells = [0usize; 3];
    let mut counts = [0usize; 3];
    let mut total = 1usize;
    for k in 0..3 {
        let n = ((max[k] - min[k]) / cell).ceil().max(1.);
        if !n.is_finite() || n > 124. {
            return Err(input(
                "Spatial graph exceeds 125 nodes. Increase cell size.",
            ));
        }
        cells[k] = n as usize;
        counts[k] = cells[k] + 1;
        total = total
            .checked_mul(counts[k])
            .ok_or_else(|| input("Spatial graph exceeds 125 nodes."))?;
    }
    if total > 125 {
        return Err(input(
            "Spatial graph exceeds 125 nodes. Increase cell size.",
        ));
    }

    if matches!(pattern.as_str(), "bcc" | "octet") {
        let g=polygon_core::lattice_tools::centered_graph(min,max,cells,&pattern)?;
        return encode(value_codec::json!({"nodes":g.nodes,"edges":g.edges}));
    }

    let mut seed = seed.trunc().rem_euclid(4294967296.) as u32;
    let mut random = || {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        seed as f64 / 4294967296.
    };

    let mut nodes = Vec::<[f64; 3]>::with_capacity(total);
    let mut edges = Vec::<[usize; 2]>::new();
    let id = |x: usize, y: usize, z: usize| (z * counts[1] + y) * counts[0] + x;

    for z in 0..counts[2] {
        for y in 0..counts[1] {
            for x in 0..counts[0] {
                let xyz = [x, y, z];
                let point = std::array::from_fn(|k| {
                    let v = xyz[k];
                    min[k]
                        + (v as f64
                            + if v > 0 && v < cells[k] {
                                (random() - 0.5) * jitter
                            } else {
                                0.
                            })
                            * (max[k] - min[k])
                            / cells[k] as f64
                });
                if !point.iter().all(|x| x.is_finite()) {
                    return Err(input("Spatial graph exceeds finite numeric range."));
                }
                nodes.push(point);
            }
        }
    }

    for z in 0..counts[2] {
        for y in 0..counts[1] {
            for x in 0..counts[0] {
                let a = id(x, y, z);
                if x < cells[0] {
                    edges.push([a, id(x + 1, y, z)]);
                }
                if y < cells[1] {
                    edges.push([a, id(x, y + 1, z)]);
                }
                if z < cells[2] {
                    edges.push([a, id(x, y, z + 1)]);
                }
                if (diagonals || pattern == "bone") && x < cells[0] && y < cells[1] && z < cells[2]
                {
                    if random() < 0.5 {
                        edges.push([a, id(x + 1, y + 1, z + 1)]);
                    } else {
                        edges.push([id(x + 1, y, z), id(x, y + 1, z + 1)]);
                    }
                }
            }
        }
    }

    encode(value_codec::json!({"nodes": nodes, "edges": edges}))
}

pub fn lightening(v: Value) -> Result<Value> {
    use polygon_core::solid::{
        boolean::{Operation, Options, boolean},
        modeling::{Profile, extrude},
    };
    use value_codec::json;

    let body: Value = field(&v, "body")?;
    let source_mesh: Mesh = field(&body, "mesh")?;
    let pattern: String = field(&v, "pattern")?;
    let axis: String = field(&v, "axis")?;
    let cell: f64 = field(&v, "cell")?;
    let rib: f64 = field(&v, "rib")?;
    let rim: f64 = field(&v, "rim")?;
    let bottom: f64 = field(&v, "bottom")?;
    let top: f64 = field(&v, "top")?;
    let seed0: f64 = field(&v, "seed")?;
    let jitter: f64 = field(&v, "jitter")?;
    let line_width: f64 = field(&v, "lineWidth")?;
    let perimeters: f64 = field(&v, "perimeters")?;

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
        let skin: f64 = if v.get("skin").is_some_and(|field| !field.is_null()) {
            field(&v, "skin")?
        } else {
            0.
        };
        let step: f64 = if v.get("step").is_some_and(|field| !field.is_null()) {
            field(&v, "step")?
        } else {
            (rib / 3.).min(if skin > 0. { skin / 2. } else { f64::INFINITY })
        };
        let open_top: bool = if v.get("openTop").is_some_and(|field| !field.is_null()) {
            field(&v, "openTop")?
        } else {
            false
        };
        let wall_depth: f64 = if v.get("wallDepth").is_some_and(|field| !field.is_null()) {
            field(&v, "wallDepth")?
        } else {
            0.
        };
        let keep_core: bool = if v.get("keepCore").is_some_and(|field| !field.is_null()) {
            field(&v, "keepCore")?
        } else {
            false
        };

        let graph = graph(
            json!({"mesh":source_mesh,"options":{"cell":cell,"jitter":jitter,"seed":seed0,"pattern":pattern,"diagonals":if v
                .get("diagonals")
                .is_some_and(|field| !field.is_null())
            {
                field(&v, "diagonals")?
            } else {
                false
            }}}),
        )
        .map_err(|e| input(format!("Cad lightening graph generation failed: {e}")))?;
        let nodes: Vec<[f64; 3]> = field(&graph, "nodes")?;
        let edges: Vec<[usize; 2]> = field(&graph, "edges")?;
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
        reduced_mesh = value_codec::from_value(decimate(value_codec::json!({
            "mesh": reduced_mesh,
            "tolerance": step * 1.5,
            "target": 2600f64
        }))?)
        .map_err(|e| input(format!("Invalid mesh: {e}")))?;
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
        let mut output = body
            .as_object()
            .ok_or_else(|| input("Expected body record"))?
            .clone();
        output.insert("mesh".into(), encode(reduced_mesh.clone())?);
        if component_count(&reduced_mesh, true)? > component_count(&source_mesh, true)? {
            return Err(input(
                "Clipping the spatial graph creates disconnected pieces. Increase strut thickness or add a skin.",
            ));
        }
        Ok(Value::Object(output))
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
            polygon_core::scene_flatten::bounds(std::slice::from_ref(&source_mesh.positions))?;

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
        let mesh = polygon_core::Mesh {
            positions: compact_positions,
            indices: result.mesh.indices,
            uv: None,
        };
        let check = mesh.inspect()?;
        if !check.closed || check.degenerate_triangles != 0 {
            return Err(input("Pattern creates details below export precision."));
        }

        let mut output = body
            .as_object()
            .ok_or_else(|| input("Expected body record"))?
            .clone();
        output.insert("mesh".into(), encode(mesh)?);
        Ok(Value::Object(output))
    }
}

/// User-facing admission message, kept identical to the pre-migration TS text.
pub(super) const PRINT_SETTINGS_MESSAGE: &str = "Проверьте сопло, высоту слоя, число линий и длину моста.";

pub(super) fn print_setting(settings: &Value, key: &str) -> Result<f64> {
    settings[key]
        .as_f64()
        .ok_or_else(|| input(PRINT_SETTINGS_MESSAGE))
}

/// Optional lightening option: absent/null stays absent, present values must be
/// finite numbers (the pre-migration TS computed NaN silently).
pub(super) fn lattice_option(options: &Value, key: &str) -> Result<Option<f64>> {
    let v = &options[key];
    if v.is_null() {
        return Ok(None);
    }
    match v.as_f64() {
        Some(x) if x.is_finite() => Ok(Some(x)),
        _ => Err(input("Lattice geometry options must be finite numbers.")),
    }
}

pub(super) fn required_lattice_option(options: &Value, key: &str) -> Result<f64> {
    lattice_option(options, key)?
        .ok_or_else(|| input("Lattice geometry options must be finite numbers."))
}

/// Legacy rounding order: ceil toward the step with a 1e-8 epsilon, then
/// compact to 1e6 grid. Kept bit-identical to the pre-migration TS helper.
pub(super) fn round_to_step(v: f64, s: f64) -> f64 {
    (((v - 1e-8) / s).ceil() * s * 1e6).round() / 1e6
}
