//! Adaptive sampling of spatial attributes on an existing triangle mesh.
//!
//! Nonlinear colors cannot be faithfully rendered by sampling only boundary
//! vertices: the result then depends on arbitrary tessellation diagonals. This
//! refinement checks edge midpoints and the centroid against interpolation.
//! Callers can additionally bound edge length to resolve periodic attributes.
use crate::tessellation::FillMesh;
use crate::{Result, check};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SampledMesh {
    pub positions: Vec<[f64; 2]>,
    pub values: Vec<[f64; 4]>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct SampleOptions {
    /// Maximum error at the four probes, in the caller's attribute units.
    pub max_error: f64,
    /// Optional anti-aliasing bound for periodic or narrowly localized data.
    pub max_edge_length: f64,
    /// Stop refining discontinuities below this document-space length.
    pub min_edge_length: f64,
    pub max_triangles: usize,
}

impl Default for SampleOptions {
    fn default() -> Self {
        Self {
            max_error: 1.,
            max_edge_length: f64::INFINITY,
            min_edge_length: 1e-5,
            max_triangles: 262_144,
        }
    }
}

#[derive(Clone, Copy)]
struct Vertex {
    p: [f64; 2],
    v: [f64; 4],
}

/// Split triangles at attribute discontinuities `a*x + b*y + c = 0` before
/// sampling. Boundary vertices use the one-sided value from their triangle's
/// interior. This represents hard gradient seams without refining a thin strip
/// down to the minimum edge length. Coverage remains the original mesh's.
pub fn sample_mesh_with_seams(
    mesh: &FillMesh,
    options: &SampleOptions,
    seams: &[[f64; 3]],
    sample: impl Fn([f64; 2]) -> [f64; 4],
) -> Result<SampledMesh> {
    if seams.is_empty() {
        return sample_mesh(mesh, options, sample);
    }
    check(
        seams.len() <= 1024 && seams.iter().flatten().all(|x| x.is_finite()),
        "Invalid attribute seams",
    )?;
    check(
        mesh.indices.len().is_multiple_of(3)
            && mesh
                .indices
                .iter()
                .all(|&i| (i as usize) < mesh.positions.len())
            && mesh.positions.iter().flatten().all(|v| v.is_finite()),
        "Invalid attribute mesh",
    )?;
    let mut output = SampledMesh::default();
    for ids in mesh.indices.as_chunks::<3>().0 {
        let mut pieces = vec![
            ids.iter()
                .map(|&i| mesh.positions[i as usize])
                .collect::<Vec<_>>(),
        ];
        for &[a, b, c] in seams {
            let side = |p: [f64; 2]| a * p[0] + b * p[1] + c;
            let mut next = Vec::new();
            for polygon in pieces {
                if !polygon.iter().any(|&p| side(p) < -1e-12)
                    || !polygon.iter().any(|&p| side(p) > 1e-12)
                {
                    next.push(polygon);
                    continue;
                }
                for sign in [-1., 1.] {
                    let mut clipped = Vec::new();
                    for i in 0..polygon.len() {
                        let (p, q) = (polygon[i], polygon[(i + 1) % polygon.len()]);
                        let (dp, dq) = (side(p) * sign, side(q) * sign);
                        if dp >= 0. {
                            clipped.push(p);
                        }
                        if (dp < 0. && dq > 0.) || (dp > 0. && dq < 0.) {
                            let t = dp / (dp - dq);
                            clipped.push([p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]);
                        }
                    }
                    if clipped.len() >= 3 {
                        next.push(clipped);
                    }
                }
            }
            check(
                next.len() + output.indices.len() / 3 <= options.max_triangles,
                "Attribute mesh seam budget exceeded",
            )?;
            pieces = next;
        }
        for polygon in pieces {
            let center = polygon.iter().fold([0., 0.], |v, p| {
                [
                    v[0] + p[0] / polygon.len() as f64,
                    v[1] + p[1] / polygon.len() as f64,
                ]
            });
            let local = FillMesh {
                positions: polygon.clone(),
                indices: (1..polygon.len() - 1)
                    .flat_map(|i| [0, i as u32, (i + 1) as u32])
                    .collect(),
            };
            let options = SampleOptions {
                max_triangles: options
                    .max_triangles
                    .saturating_sub(output.indices.len() / 3),
                ..*options
            };
            let refined = sample_mesh(&local, &options, |p| {
                let on_seam = seams
                    .iter()
                    .any(|&[a, b, c]| (a * p[0] + b * p[1] + c).abs() <= 1e-10 * (1. + c.abs()));
                let p = if on_seam {
                    [
                        p[0] + (center[0] - p[0]) * 1e-5,
                        p[1] + (center[1] - p[1]) * 1e-5,
                    ]
                } else {
                    p
                };
                sample(p)
            })?;
            let base = output.positions.len() as u32;
            output
                .indices
                .extend(refined.indices.into_iter().map(|i| i + base));
            output.positions.extend(refined.positions);
            output.values.extend(refined.values);
        }
    }
    Ok(output)
}

/// Preserve triangle coverage while adding samples for nonlinear attributes.
/// Input/output are detached, so errors cannot leave a partially updated mesh.
pub fn sample_mesh(
    mesh: &FillMesh,
    options: &SampleOptions,
    sample: impl Fn([f64; 2]) -> [f64; 4],
) -> Result<SampledMesh> {
    check(
        options.max_error.is_finite()
            && options.max_error > 0.
            && options.max_edge_length > 0.
            && !options.max_edge_length.is_nan()
            && options.min_edge_length.is_finite()
            && options.min_edge_length > 0.
            && options.max_triangles > 0
            && options.max_triangles <= (u32::MAX as usize) / 3,
        "Invalid attribute mesh sampling options",
    )?;
    check(
        mesh.indices.len().is_multiple_of(3)
            && mesh
                .indices
                .iter()
                .all(|&i| (i as usize) < mesh.positions.len())
            && mesh.positions.iter().flatten().all(|v| v.is_finite()),
        "Invalid attribute mesh",
    )?;
    let at = |p| -> Result<Vertex> {
        let v = sample(p);
        check(
            v.iter().all(|x| x.is_finite()),
            "Nonfinite sampled mesh attribute",
        )?;
        Ok(Vertex { p, v })
    };
    let input = mesh
        .positions
        .iter()
        .map(|&p| at(p))
        .collect::<Result<Vec<_>>>()?;
    let mut output = SampledMesh::default();
    let mut stack = Vec::new();
    for ids in mesh.indices.as_chunks::<3>().0 {
        stack.push((
            [
                input[ids[0] as usize],
                input[ids[1] as usize],
                input[ids[2] as usize],
            ],
            0u8,
        ));
        while let Some((tri, depth)) = stack.pop() {
            let mut max_error: f64 = 0.;
            let mut longest: f64 = 0.;
            let mut split = 0;
            let mut midpoints = [tri[0]; 3];
            for i in 0..3 {
                let (a, b) = (tri[i], tri[(i + 1) % 3]);
                let length = (a.p[0] - b.p[0]).hypot(a.p[1] - b.p[1]);
                if length > longest {
                    longest = length;
                    split = i;
                }
                let mid = at([(a.p[0] + b.p[0]) * 0.5, (a.p[1] + b.p[1]) * 0.5])?;
                midpoints[i] = mid;
                for k in 0..4 {
                    max_error = max_error.max((mid.v[k] - (a.v[k] + b.v[k]) * 0.5).abs());
                }
            }
            let center = at([
                (tri[0].p[0] + tri[1].p[0] + tri[2].p[0]) / 3.,
                (tri[0].p[1] + tri[1].p[1] + tri[2].p[1]) / 3.,
            ])?;
            for k in 0..4 {
                max_error = max_error
                    .max((center.v[k] - (tri[0].v[k] + tri[1].v[k] + tri[2].v[k]) / 3.).abs());
            }
            if longest > options.min_edge_length
                && (max_error > options.max_error || longest > options.max_edge_length)
            {
                check(depth < 48, "Attribute mesh refinement depth exceeded")?;
                check(
                    output.indices.len() / 3 + stack.len() + 2 <= options.max_triangles,
                    "Attribute mesh triangle budget exceeded",
                )?;
                let (a, b, c, m) = (
                    tri[split],
                    tri[(split + 1) % 3],
                    tri[(split + 2) % 3],
                    midpoints[split],
                );
                stack.push(([m, b, c], depth + 1));
                stack.push(([a, m, c], depth + 1));
            } else {
                check(
                    output.indices.len() / 3 < options.max_triangles,
                    "Attribute mesh triangle budget exceeded",
                )?;
                for vertex in tri {
                    output.indices.push(output.positions.len() as u32);
                    output.positions.push(vertex.p);
                    output.values.push(vertex.v);
                }
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn square(diagonal: bool) -> FillMesh {
        FillMesh {
            positions: vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
            indices: if diagonal {
                vec![0, 1, 2, 0, 2, 3]
            } else {
                vec![0, 1, 3, 1, 2, 3]
            },
        }
    }
    #[test]
    fn linear_values_need_no_refinement() {
        let m = sample_mesh(&square(true), &SampleOptions::default(), |p| {
            [p[0] * 255., p[1] * 255., 0., 255.]
        })
        .unwrap();
        assert_eq!(m.indices.len(), 6);
    }
    #[test]
    fn radial_center_is_represented_for_either_diagonal() {
        let value = |p: [f64; 2]| [(p[0] - 0.5).hypot(p[1] - 0.5).min(0.5) * 510., 0., 0., 255.];
        for diagonal in [false, true] {
            let m = sample_mesh(&square(diagonal), &SampleOptions::default(), value).unwrap();
            assert!(m.positions.len() > 6);
            assert!(m.values.iter().any(|v| v[0] < 1.));
            let mut area = 0.;
            for ids in m.indices.as_chunks::<3>().0 {
                let [a, b, c] = [
                    m.positions[ids[0] as usize],
                    m.positions[ids[1] as usize],
                    m.positions[ids[2] as usize],
                ];
                area += ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])) * 0.5;
                for weights in [[0.2, 0.3, 0.5], [0.6, 0.2, 0.2], [0.1, 0.8, 0.1]] {
                    let p = [
                        a[0] * weights[0] + b[0] * weights[1] + c[0] * weights[2],
                        a[1] * weights[0] + b[1] * weights[1] + c[1] * weights[2],
                    ];
                    let interpolated: f64 = ids
                        .iter()
                        .zip(weights)
                        .map(|(&i, w)| m.values[i as usize][0] * w)
                        .sum();
                    assert!((interpolated - value(p)[0]).abs() < 2.);
                }
            }
            assert!((area - 1.).abs() < 1e-12);
        }
    }
    #[test]
    fn invalid_samples_and_exhaustion_are_explicit() {
        assert!(sample_mesh(&square(true), &SampleOptions::default(), |_| [f64::NAN; 4]).is_err());
        assert!(
            sample_mesh(
                &square(true),
                &SampleOptions {
                    max_triangles: 2,
                    max_edge_length: 0.1,
                    ..Default::default()
                },
                |_| [0.; 4]
            )
            .is_err()
        );
    }

    #[test]
    fn repeated_linear_seams_are_sharp_without_excessive_triangles() {
        let seams = [
            [1., 0., -0.25],
            [1., 0., -0.5],
            [1., 0., -0.75],
            [1., 0., -1.],
            [1., 0., 0.],
        ];
        let mesh = sample_mesh_with_seams(&square(true), &SampleOptions::default(), &seams, |p| {
            [(p[0] * 4.).rem_euclid(1.) * 255., 0., 0., 255.]
        })
        .unwrap();
        assert!(mesh.indices.len() / 3 <= 16);
        for ids in mesh.indices.as_chunks::<3>().0 {
            let x = ids
                .iter()
                .map(|&i| mesh.positions[i as usize][0] / 3.)
                .sum::<f64>();
            let v = ids
                .iter()
                .map(|&i| mesh.values[i as usize][0] / 3.)
                .sum::<f64>();
            assert!((v - (x * 4.).rem_euclid(1.) * 255.).abs() < 0.01);
        }
    }
}
