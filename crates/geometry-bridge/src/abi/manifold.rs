use super::*;

fn validate_transport(fmt: u32, vp: usize, vl: usize, ip: usize, il: usize) -> Result<()> {
    if fmt > FMT_F64 || vl > vertex_limit(fmt, LIMIT) || il > LIMIT / 4 {
        return Err(input("Mesh exceeds transport limit"));
    }
    if (vp == 0 && vl != 0) || (ip == 0 && il != 0) {
        return Err(input("Mesh buffer is null"));
    }
    Ok(())
}

unsafe fn read_mesh(
    fmt: u32,
    vp: usize,
    vl: usize,
    ip: usize,
    il: usize,
) -> Result<(Vec<f64>, Vec<usize>)> {
    validate_transport(fmt, vp, vl, ip, il)?;
    let positions = unsafe { read_vertices_f64(fmt, vp, vl) }?;
    let indices: Vec<usize> = unsafe { read_u32(ip, il) }
        .into_iter()
        .map(|index| index as usize)
        .collect();
    manifold_core::validate_indexed_mesh(&positions, &indices)
        .map_err(|error| input(error.to_string()))?;
    Ok((positions, indices))
}

/// Run manifold-core `check` on an uploaded mesh and return a result handle
/// for `abi_array_field` (`ManifoldCheck` slots).
/// # Safety
/// vp/vl (vertices, format `fmt`) and ip/il (u32 indices) must reference live
/// caller-owned buffers; they are only read.
pub unsafe fn abi_manifold_check(vp: usize, vl: usize, ip: usize, il: usize, fmt: u32) -> u64 {
    let (vertices, indices) = match unsafe { read_mesh(fmt, vp, vl, ip, il) } {
        Ok(mesh) => mesh,
        Err(error) => return packed(geometry(Err(error))),
    };
    let report = manifold_core::check(&vertices, &indices);
    let flat = |edges: &[manifold_core::EdgeKey]| -> Vec<u32> {
        edges
            .iter()
            .flat_map(|&(a, b)| [a as u32, b as u32])
            .collect()
    };
    let flags = (report.is_manifold() as u32) | ((report.is_manifold_with_boundary() as u32) << 1);
    let buffers = mesh_analysis::AnalysisBuffers::ManifoldCheck {
        boundary_edges: flat(&report.boundary_edges),
        non_manifold_edges: flat(&report.non_manifold_edges),
        orientation_edges: flat(&report.orientation_edges),
        degenerate_triangles: report
            .degenerate_triangles
            .iter()
            .map(|&i| i as u32)
            .collect(),
        non_manifold_vertices: report
            .non_manifold_vertices
            .iter()
            .map(|&i| i as u32)
            .collect(),
        isolated_vertices: report.isolated_vertices.iter().map(|&i| i as u32).collect(),
        summary: [
            report.vertex_count as u32,
            report.triangle_count as u32,
            report.component_count as u32,
            flags,
        ],
    };
    packed(geometry(encode(mesh_analysis::store(buffers))))
}

/// Run manifold-core `repair` and return the repaired mesh plus stats
/// (`ManifoldRepair` slots). `mode`: 0 = Conservative (weld + de-degenerate +
/// orientation unify), 1 = Full (additionally splits non-manifold
/// edges/vertices and fills simple boundary loops).
/// # Safety
/// vp/vl (vertices, format `fmt`) and ip/il (u32 indices) must reference live
/// caller-owned buffers; they are only read. `epsilon` must be finite.
pub unsafe fn abi_manifold_repair(
    vp: usize,
    vl: usize,
    ip: usize,
    il: usize,
    fmt: u32,
    epsilon: f64,
    mode: u32,
) -> u64 {
    if !epsilon.is_finite() {
        return packed(geometry(Err(input("Invalid weld epsilon"))));
    }
    let mode = match mode {
        0 => manifold_core::RepairMode::Conservative,
        1 => manifold_core::RepairMode::Full,
        _ => return packed(geometry(Err(input("Invalid repair mode")))),
    };
    let (vertices, indices) = match unsafe { read_mesh(fmt, vp, vl, ip, il) } {
        Ok(mesh) => mesh,
        Err(error) => return packed(geometry(Err(error))),
    };
    let out = manifold_core::repair_with_mode(&vertices, &indices, epsilon, mode);
    let flags = (out.report.residual.is_manifold() as u32)
        | ((out.report.residual.is_manifold_with_boundary() as u32) << 1);
    let buffers = mesh_analysis::AnalysisBuffers::ManifoldRepair {
        positions: out.positions,
        indices: out.indices.iter().map(|&i| i as u32).collect(),
        stats: [
            out.report.welded_vertices as u32,
            out.report.removed_degenerate_triangles as u32,
            out.report.flipped_triangles as u32,
            flags,
            out.report.split_vertices as u32,
            out.report.filled_holes as u32,
            out.report.filled_triangles as u32,
            mode as u32,
        ],
    };
    packed(geometry(encode(mesh_analysis::store(buffers))))
}

/// Run manifold-core `metrics` on an uploaded mesh and return a result handle
/// for `abi_array_field` (`ManifoldMetrics` slots).
/// # Safety
/// vp/vl (vertices, format `fmt`) and ip/il (u32 indices) must reference live
/// caller-owned buffers; they are only read.
pub unsafe fn abi_manifold_metrics(vp: usize, vl: usize, ip: usize, il: usize, fmt: u32) -> u64 {
    let (vertices, indices) = match unsafe { read_mesh(fmt, vp, vl, ip, il) } {
        Ok(mesh) => mesh,
        Err(error) => return packed(geometry(Err(error))),
    };
    let m = manifold_core::metrics(&vertices, &indices);
    let mut component_stats = Vec::with_capacity(m.components.len() * 6);
    let mut component_floats = Vec::with_capacity(m.components.len() * 2);
    for c in &m.components {
        component_stats.extend_from_slice(&[
            c.triangle_count as u32,
            c.vertex_count as u32,
            c.edge_count as u32,
            c.boundary_edges as u32,
            c.euler_characteristic as i32 as u32,
            c.genus.map_or(0, |g| g as u32 + 1),
        ]);
        component_floats.extend_from_slice(&[c.signed_volume, c.surface_area]);
    }
    let buffers = mesh_analysis::AnalysisBuffers::ManifoldMetrics {
        component_stats,
        component_floats,
        floats: vec![m.signed_volume, m.surface_area],
        summary: [
            m.vertex_count as u32,
            m.triangle_count as u32,
            m.edge_count as u32,
            m.euler_characteristic as i32 as u32,
            m.components.len() as u32,
            m.watertight as u32,
            0,
            0,
        ],
    };
    packed(geometry(encode(mesh_analysis::store(buffers))))
}

/// Run a manifold-ops boolean `a OP b` (`op`: 0 = union, 1 = intersection,
/// 2 = difference) with guaranteed strictly manifold output and return the
/// result mesh plus kernel stats (`ManifoldBoolean` slots).
/// # Safety
/// All vertex/index buffers must reference live caller-owned memory; read only.
#[allow(clippy::too_many_arguments)]
pub unsafe fn abi_manifold_boolean(
    a_vp: usize,
    a_vl: usize,
    a_ip: usize,
    a_il: usize,
    a_fmt: u32,
    b_vp: usize,
    b_vl: usize,
    b_ip: usize,
    b_il: usize,
    b_fmt: u32,
    op: u32,
    weld_epsilon: f64,
    relative_tolerance: f64,
) -> u64 {
    let op = match op {
        0 => manifold_ops::BooleanOp::Union,
        1 => manifold_ops::BooleanOp::Intersection,
        2 => manifold_ops::BooleanOp::Difference,
        _ => return packed(geometry(Err(input("Invalid boolean operation")))),
    };
    let (a_positions, a_indices) = match unsafe { read_mesh(a_fmt, a_vp, a_vl, a_ip, a_il) } {
        Ok(mesh) => mesh,
        Err(error) => return packed(geometry(Err(error))),
    };
    let (b_positions, b_indices) = match unsafe { read_mesh(b_fmt, b_vp, b_vl, b_ip, b_il) } {
        Ok(mesh) => mesh,
        Err(error) => return packed(geometry(Err(error))),
    };
    let out = match manifold_ops::boolean_manifold_with_options(
        &a_positions,
        &a_indices,
        &b_positions,
        &b_indices,
        op,
        manifold_ops::BooleanOptions {
            weld_epsilon,
            relative_tolerance,
        },
    ) {
        Ok(out) => out,
        Err(manifold_ops::BooleanError::Kernel(e)) => return packed(geometry(Err(e))),
        Err(e) => {
            let code = match e {
                manifold_ops::BooleanError::UnrepairableInput { .. } => "GEOMETRY_INVALID_INPUT",
                _ => "GEOMETRY_KERNEL",
            };
            return packed(geometry(Err(Error::new(code, e.to_string()))));
        }
    };
    let repaired = |slot: usize| out.operand_repairs[slot].is_some() as u32;
    let buffers = mesh_analysis::AnalysisBuffers::ManifoldBoolean {
        positions: out.positions,
        indices: out.indices.iter().map(|&i| i as u32).collect(),
        floats: vec![out.kernel_report.tolerance_mm],
        stats: [
            op as u32,
            repaired(0),
            repaired(1),
            out.kernel_report.fragments as u32,
            out.kernel_report.work as u32,
            out.kernel_report.input_triangles[0] as u32,
            out.kernel_report.input_triangles[1] as u32,
            out.output_report.is_manifold() as u32,
        ],
    };
    packed(geometry(encode(mesh_analysis::store(buffers))))
}
