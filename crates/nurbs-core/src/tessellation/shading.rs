//! Analytic surface normals and orientation for an existing tessellation.
//!
//! Natural winding follows `S_u × S_v`; reversing a face swaps both its
//! triangle winding and its analytic normals. Mesh normals are used only as an
//! explicitly marked fallback, never presented as analytic CAD normals.
use crate::{
    Error, Result, check,
    surface::Surface,
    tessellation::surface_tessellation::{Report, Triangle},
};
use math_core::{cross, norm};

/// One-sided analytic normal at a collapsed domain-boundary row or column.
/// There the tangent along the collapsed boundary vanishes linearly, and the
/// mixed derivative S_uv takes its place in the cross product, which gives
/// the exact limiting normal. No finite-difference probe is used.
fn analytic_normal(surface: &Surface, u: f64, v: f64) -> Result<Option<[f64; 3]>> {
    let evaluation = surface.evaluate(u, v)?;
    if let Some(n) = evaluation.unit_normal() {
        return Ok(Some(n));
    }
    // Exact derivatives exist even where the evaluator withholds them at a
    // singular point; the collapsed-boundary limit below needs S_uv too.
    let d = surface.derivatives(u, v, 2)?;
    let (du, dv, duv) = (d[1][0], d[0][1], d[1][1]);

    let domains = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.knots_u.len() - surface.degree_u - 1],
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.knots_v.len() - surface.degree_v - 1],
    ];
    for (axis, at_start) in [(0, true), (0, false), (1, true), (1, false)] {
        if (axis == 0 && u != domains[if at_start { 0 } else { 1 }])
            || (axis == 1 && v != domains[if at_start { 2 } else { 3 }])
        {
            continue;
        }
        let row = if axis == 0 {
            if at_start {
                0
            } else {
                surface.control_points.len() - 1
            }
        } else {
            usize::MAX
        };
        let collapsed = if axis == 0 {
            surface.control_points[row].windows(2).all(|w| w[0] == w[1])
        } else {
            let column = if at_start {
                0
            } else {
                surface.control_points[0].len() - 1
            };
            surface
                .control_points
                .windows(2)
                .all(|w| w[0][column] == w[1][column])
        };
        if !collapsed {
            continue;
        }
        // Near a collapsed u-row the v tangent vanishes linearly:
        // S_v ~ (u - u0) S_uv, so the normal S_u x S_v tends to
        // sign(u - u0) S_u x S_uv; a collapsed v-column likewise gives
        // sign(v - v0) S_uv x S_v. The sign is fixed by which domain end the
        // collapsed boundary lies on.
        let side = if at_start { 1. } else { -1. };
        let limit = if axis == 0 { cross(du, duv) } else { cross(duv, dv) };
        let cross_product = normalize(limit.map(|x| x * side));
        if cross_product.is_some() {
            return Ok(cross_product);
        }
    }
    Ok(None)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Natural,
    Reversed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalPolicy {
    /// Return a typed refusal at every vertex without an analytic unit normal.
    RefuseDegenerate,
    /// Use area-weighted geometric mesh normals only where analytic normals
    /// are unavailable. A wholly collapsed neighbourhood is still a refusal.
    MeshFallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalSource {
    Analytic,
    MeshFallback,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VertexNormal {
    pub vector: [f64; 3],
    pub source: NormalSource,
}

#[derive(Clone, Debug)]
pub struct ShadedReport {
    pub points: Vec<[f64; 3]>,
    pub parameters: Vec<[f64; 2]>,
    pub normals: Vec<VertexNormal>,
    pub triangles: Vec<Triangle>,
}

fn normalize(vector: [f64; 3]) -> Option<[f64; 3]> {
    let length = norm(vector);
    (length.is_finite() && length > 0.).then(|| vector.map(|x| x / length))
}

/// Shade an existing mesh without changing its positions or error evidence.
/// This function makes no deviation or Hausdorff claim beyond the input report.
pub fn shade(
    surface: &Surface,
    report: &Report,
    orientation: Orientation,
    policy: NormalPolicy,
) -> Result<ShadedReport> {
    surface.validate()?;
    check(
        report.points.len() == report.parameters.len(),
        "Tessellation shading requires one UV parameter per point",
    )?;
    let mut triangles = report.triangles.clone();
    for triangle in &triangles {
        check(
            triangle.vertices.iter().all(|&i| i < report.points.len()),
            "Tessellation shading received an out-of-range triangle index",
        )?;
    }
    if orientation == Orientation::Reversed {
        for triangle in &mut triangles {
            triangle.vertices.swap(1, 2);
        }
    }
    let mut normals = Vec::with_capacity(report.points.len());
    let mut unresolved = Vec::new();
    for (index, uv) in report.parameters.iter().enumerate() {
        let analytic = analytic_normal(surface, uv[0], uv[1])?.map(|mut n| {
            if orientation == Orientation::Reversed {
                n = n.map(|x| -x);
            }
            VertexNormal {
                vector: n,
                source: NormalSource::Analytic,
            }
        });
        if let Some(normal) = analytic {
            normals.push(normal);
        } else {
            if policy == NormalPolicy::RefuseDegenerate {
                return Err(Error::new(
                    "NURBS_ANALYTIC_NORMAL_UNAVAILABLE",
                    "Analytic surface normal is unavailable at a tessellation vertex",
                ));
            }
            unresolved.push(index);
            normals.push(VertexNormal {
                vector: [0.; 3],
                source: NormalSource::MeshFallback,
            });
        }
    }
    if unresolved.is_empty() {
        return Ok(ShadedReport {
            points: report.points.clone(),
            parameters: report.parameters.clone(),
            normals,
            triangles,
        });
    }
    let mut accumulated = vec![[0.; 3]; report.points.len()];
    for triangle in &triangles {
        let [a, b, c] = triangle.vertices;
        let ab = std::array::from_fn(|axis| report.points[b][axis] - report.points[a][axis]);
        let ac = std::array::from_fn(|axis| report.points[c][axis] - report.points[a][axis]);
        let area_weighted = cross(ab, ac);
        for index in [a, b, c] {
            for axis in 0..3 {
                accumulated[index][axis] += area_weighted[axis];
            }
        }
    }
    for index in unresolved {
        normals[index].vector = normalize(accumulated[index]).ok_or_else(|| {
            Error::new(
                "NURBS_MESH_NORMAL_UNAVAILABLE",
                "Mesh-normal fallback is undefined at a wholly collapsed tessellation vertex",
            )
        })?;
    }
    Ok(ShadedReport {
        points: report.points.clone(),
        parameters: report.parameters.clone(),
        normals,
        triangles,
    })
}
