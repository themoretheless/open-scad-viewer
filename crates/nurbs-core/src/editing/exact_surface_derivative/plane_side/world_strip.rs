//! Continuous spatial polylines across certified original knot seams.
use super::{
    PResult, Q, Refusal, Result, Surface, SurfacePlaneGraphStrip, SurfacePlaneGraphStripOptions,
    SurfacePlaneWorldPolylineOptions, Work, check, stitch_surface_plane_graph_strip,
    world_polyline::build,
};
use num_traits::{Signed, Zero};

#[derive(Clone, Copy, Debug)]
pub struct SurfacePlaneWorldStripOptions {
    pub strip: SurfacePlaneGraphStripOptions,
    pub max_xyz_error: f64,
    pub max_segments_per_cell: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlaneWorldStrip {
    pub strip: SurfacePlaneGraphStrip,
    /// Exact vertex coordinates in the original surface chart.
    pub uv: Vec<[Q; 2]>,
    pub xyz: Vec<[Q; 3]>,
    /// Inclusive vertex ranges in knot-cell order; neighbors share one vertex.
    pub cell_vertices: Vec<[usize; 2]>,
    /// Continuous XYZ error bounds for each cell at equal transverse parameter.
    pub cell_error_upper: Vec<Q>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePlaneWorldStripReport {
    pub outcome: std::result::Result<SurfacePlaneWorldStrip, Refusal>,
    pub work_used: u64,
}

/// Approximate a continuous section strip with exactly shared XYZ and UV
/// seam vertices. Moving a cell's first vertex contributes its exact
/// displacement to the continuous chord error bound.
pub fn approximate_surface_plane_strip_xyz(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    options: SurfacePlaneWorldStripOptions,
) -> Result<SurfacePlaneWorldStripReport> {
    check(
        options.max_xyz_error.is_finite()
            && options.max_xyz_error > 0.
            && (1..=1024).contains(&options.max_segments_per_cell),
        "World strip approximation needs a positive XYZ tolerance and 1..1024 segments per cell",
    )?;
    let stitched = stitch_surface_plane_graph_strip(surface, normal, offset, options.strip)?;
    let mut work = Work {
        used: stitched.work_used,
        limit: options.strip.max_work,
    };
    let outcome = stitched.outcome.and_then(|strip| {
        let tolerance = work.lift(options.max_xyz_error)?;
        approximate(
            surface, normal, offset, options, strip, &tolerance, &mut work,
        )
    });
    Ok(SurfacePlaneWorldStripReport {
        outcome,
        work_used: work.used,
    })
}

pub(super) fn approximate(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    options: SurfacePlaneWorldStripOptions,
    strip: SurfacePlaneGraphStrip,
    tolerance: &Q,
    work: &mut Work,
) -> PResult<SurfacePlaneWorldStrip> {
    // The previous cell's endpoint error and the next cell's endpoint error
    // both bound the possible displacement when the shared endpoint is chosen.
    let cell_tolerance = work.op(tolerance, &Q::from_integer(3.into()), |a, b| a / b)?;
    let mut uv = Vec::new();
    let mut xyz = Vec::new();
    let mut cell_vertices = Vec::new();
    let mut cell_error_upper = Vec::new();
    for cell in &strip.cells {
        let original = build(
            surface,
            normal,
            offset,
            SurfacePlaneWorldPolylineOptions {
                span: cell.span,
                solve_axis: options.strip.solve_axis,
                max_xyz_error: options.max_xyz_error,
                max_segments: options.max_segments_per_cell,
                max_work: options.strip.max_work,
            },
            cell.domain,
            &cell_tolerance,
            work,
        )?;
        let mut error = original.xyz_error_upper;
        let start = xyz.len().saturating_sub(1);
        let mut skip = 0;
        if let Some(prior) = xyz.last() {
            let displacement = distance_l1(prior, &original.xyz[0], work)?;
            error = work.op(&error, &displacement, |a, b| a + b)?;
            skip = 1;
        }
        if error > *tolerance {
            return Err(Refusal::UnprovedIdentity);
        }
        let [u, v] = cell.domain;
        let u = [work.lift(u[0])?, work.lift(u[1])?];
        let v = [work.lift(v[0])?, work.lift(v[1])?];
        let du = work.op(&u[1], &u[0], |a, b| a - b)?;
        let dv = work.op(&v[1], &v[0], |a, b| a - b)?;
        for [a, b] in original.uv.into_iter().skip(skip) {
            let x = work.op(&a, &du, |a, b| a * b)?;
            let y = work.op(&b, &dv, |a, b| a * b)?;
            uv.push([
                work.op(&u[0], &x, |a, b| a + b)?,
                work.op(&v[0], &y, |a, b| a + b)?,
            ]);
        }
        xyz.extend(original.xyz.into_iter().skip(skip));
        cell_vertices.push([start, xyz.len() - 1]);
        cell_error_upper.push(error);
    }
    Ok(SurfacePlaneWorldStrip {
        strip,
        uv,
        xyz,
        cell_vertices,
        cell_error_upper,
    })
}

pub(super) fn distance_l1(a: &[Q; 3], b: &[Q; 3], work: &mut Work) -> PResult<Q> {
    let mut total = Q::zero();
    for (a, b) in a.iter().zip(b) {
        let term = work.op(a, b, |a, b| (a - b).abs())?;
        total = work.op(&total, &term, |a, b| a + b)?;
    }
    Ok(total)
}
