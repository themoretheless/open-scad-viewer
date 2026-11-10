//! Full filling comparison with the old bilinear trimmed cap. This certificate
//! is relative to that retained reference, not to the authored sweep source.
use super::{CapReplacement, construct, refused, replace, uv_line};
use crate::{Coedge, Edge, Face, FaceUse, Loop, Model, Shell, Vertex};
use nurbs_core::{Result, curve::Curve, interval_eval::Interval as I, surface::Surface};

#[derive(Clone, Copy)]
pub struct Limits {
    pub exact_work: u64,
    pub products: usize,
    pub trim_pairs: usize,
    pub trim_cells: usize,
    pub trim_domain_cells: usize,
    pub spans: usize,
    pub linear_cells: usize,
    pub contacts: crate::face_contacts::Limits,
}
pub struct Report {
    /// Symmetric set-distance bound for the entire filled cap, only after UV
    /// coverage is proved. It is never an original-source continuousBound.
    pub error_upper: Option<f64>,
    pub within_budget: bool,
    pub uv_region_covered: bool,
    pub boundary_error_upper: Option<f64>,
    pub bilinear_defect_upper: Option<f64>,
    pub exact_work: u64,
    pub products: usize,
    pub reference_region: nurbs_core::trim_region_audit::Report,
    pub uv_faces: crate::face_injectivity::Report,
    pub uv_pairs: crate::face_contacts::Report,
    pub reason: &'static str,
}

fn aligned_uv(model: &Model, face: usize) -> Result<[Vec<Curve>; 2]> {
    let f = &model.faces[face];
    let outer = model.loops[f.outer]
        .coedges
        .iter()
        .map(|c| c.pcurve.clone())
        .collect();
    let inner = model.loops[f.holes[0]]
        .coedges
        .iter()
        .rev()
        .map(|c| c.pcurve.reverse())
        .collect::<Result<Vec<_>>>()?;
    Ok([outer, inner])
}
fn lift_uv(ring: &[Curve]) -> Vec<Curve> {
    ring.iter()
        .map(|c| Curve {
            degree: c.degree,
            knots: c.knots.clone(),
            control_points: c
                .control_points
                .iter()
                .map(|p| vec![p[0], p[1], 0.])
                .collect(),
            weights: c.weights.clone(),
            periodic: c.periodic,
        })
        .collect()
}

/// The generated sheet is a connected compact annulus with exactly the two
/// supplied boundary rings. Complete-chart injectivity and all pair exclusions
/// then prove an embedded planar annulus, whose image is precisely the Jordan
/// region between its boundary loops. No grid or area heuristic is involved.
pub(super) fn uv_sheet(
    rings: &[Vec<Curve>; 2],
    max_work: u64,
    tolerance: f64,
) -> Result<(Model, u64)> {
    let rings = [lift_uv(&rings[0]), lift_uv(&rings[1])];
    let cap = construct(&rings[0], &rings[1], max_work)?;
    let n = cap.patches.len();
    let mut model = Model::empty(tolerance)?;
    model.vertices.reserve(2 * n);
    model.edges.reserve(3 * n);
    model.loops.reserve(n);
    model.faces.reserve(n);
    for ring in &rings {
        for c in ring {
            let p = &c.control_points[0];
            model.vertices.push(Vertex {
                point: [p[0], p[1], p[2]],
            });
        }
    }
    for (r, ring) in rings.iter().enumerate() {
        for (i, curve) in ring.iter().enumerate() {
            model.edges.push(Edge {
                degenerate: false,
                vertices: [r * n + i, r * n + (i + 1) % n],
                curve: curve.clone(),
            });
        }
    }
    for i in 0..n {
        model.edges.push(Edge {
            degenerate: false,
            vertices: [i, n + i],
            curve: Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                periodic: false,
                control_points: vec![
                    rings[0][i].control_points[0].clone(),
                    rings[1][i].control_points[0].clone(),
                ],
                weights: vec![rings[0][i].weights[0], rings[1][i].weights[0]],
            },
        });
    }
    for (i, surface) in cap.patches.into_iter().enumerate() {
        model.loops.push(Loop {
            coedges: vec![
                Coedge::new(
                    i,
                    false,
                    uv_line([0., 0.], [1., 0.]),
                ),
                Coedge::new(
                    2 * n + (i + 1) % n,
                    false,
                    uv_line([1., 0.], [1., 1.]),
                ),
                Coedge::new(
                    n + i,
                    true,
                    uv_line([1., 1.], [0., 1.]),
                ),
                Coedge::new(
                    2 * n + i,
                    true,
                    uv_line([0., 1.], [0., 0.]),
                ),
            ],
        });
        model.faces.push(Face {
            surface,
            outer: i,
            holes: vec![],
        });
    }
    model.shells.push(Shell {
        closed: false,
        faces: (0..n)
            .map(|face| FaceUse {
                face,
                reversed: false,
            })
            .collect(),
    });
    model.rebuild_topology_ids();
    model.validate()?;
    Ok((model, cap.exact_work))
}
fn absolute_upper(x: I) -> f64 {
    x.lo.abs().max(x.hi.abs())
}

fn bilinear_defect(
    s: &Surface,
    rings: &[Vec<Curve>; 2],
    max_products: usize,
) -> Result<Option<(f64, usize)>> {
    // C((1-b)a+bb) - (1-b)C(a)-bC(b) is the mixed bilinear
    // coefficient times -b(1-b)(a.x-b.x)(a.y-b.y), with b in [0,1].
    let products = rings[0]
        .iter()
        .zip(&rings[1])
        .map(|(a, b)| a.control_points.len() * b.control_points.len())
        .sum();
    if products > max_products {
        return Ok(None);
    }
    let mut square = I::point(0.);
    for k in 0..3 {
        let d = I::point(s.control_points[1][1][k])
            .sub(I::point(s.control_points[1][0][k]))?
            .sub(I::point(s.control_points[0][1][k]))?
            .add(I::point(s.control_points[0][0][k]))?;
        let radius = I::point(absolute_upper(d));
        square = square.add(radius.mul(radius)?)?;
    }
    let norm = square.hi.sqrt().next_up();
    if !norm.is_finite() {
        return Err(refused("Bilinear filling enclosure overflow"));
    }
    let mut delta = 0_f64;
    for (a, b) in rings[0].iter().zip(&rings[1]) {
        // Independent positive ring weights may give different convex
        // coefficients. The Cartesian pole-product hull still encloses a-b.
        let mut widths = [0_f64; 2];
        for pa in &a.control_points {
            for pb in &b.control_points {
                for k in 0..2 {
                    widths[k] =
                        widths[k].max(absolute_upper(I::point(pa[k]).sub(I::point(pb[k]))?));
                }
            }
        }
        delta = delta.max(I::point(widths[0]).mul(I::point(widths[1]))?.hi);
    }
    let du = I::point(s.knots_u[2]).sub(I::point(s.knots_u[1]))?;
    let dv = I::point(s.knots_v[2]).sub(I::point(s.knots_v[1]))?;
    Ok(Some((
        I::point(norm)
            .mul(I::point(delta))?
            .mul(I::point(0.25))?
            .div(du.mul(dv)?)?
            .hi,
        products,
    )))
}

/// Recompute ownership and the actual replacement coefficients; supplied
/// flags, face maps and work counters never act as certificate premises.
pub fn inspect(
    source: &Model,
    cap: &CapReplacement,
    tolerance: f64,
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report> {
    if !tolerance.is_finite() || tolerance < 0. || limits.products > 100_000 {
        return Err(refused("Invalid filled-reference error budget"));
    }
    let expected = replace(source, cap.replaced_face, limits.exact_work)?;
    if expected.model != cap.model || expected.cap_faces != cap.cap_faces {
        return Err(refused(
            "Filled-reference certificate requires the fresh native replacement",
        ));
    }
    let mut exact_work = expected.exact_work;
    let f = &source.faces[cap.replaced_face];
    let rings = aligned_uv(source, cap.replaced_face)?;
    let reference_loops = [
        rings[0].clone(),
        source.loops[f.holes[0]]
            .coedges
            .iter()
            .map(|c| c.pcurve.clone())
            .collect(),
    ];
    let reference_region = nurbs_core::trim_region_audit::inspect(
        &reference_loops,
        tolerance_uv,
        limits.trim_pairs,
        limits.trim_cells,
        limits.trim_domain_cells,
    )?;
    let (sheet, used) = uv_sheet(&rings, limits.exact_work - exact_work, source.tolerance_mm)?;
    exact_work += used;
    let uv_faces =
        crate::face_injectivity::inspect_with_linear(&sheet, limits.spans, limits.linear_cells)?;
    let uv_pairs = crate::face_contacts::inspect(&sheet, tolerance_uv, limits.contacts)?;
    let coverage = reference_region.valid == Some(true)
        && uv_faces.all_faces_injective
        && uv_pairs.all_pairs_classified;
    let mut out = Report {
        error_upper: None,
        within_budget: false,
        uv_region_covered: coverage,
        boundary_error_upper: None,
        bilinear_defect_upper: None,
        exact_work,
        products: 0,
        reference_region,
        uv_faces,
        uv_pairs,
        reason: "uv-filling-coverage-unproven",
    };
    if !coverage {
        return Ok(out);
    }
    let mut upper = 0_f64;
    for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
        for c in &source.loops[wire].coedges {
            let edge = &source.edges[c.edge].curve;
            let world = if c.reversed {
                edge.reverse()?
            } else {
                edge.clone()
            };
            let proof = nurbs_core::sweep_cap_boundary::inspect(
                &f.surface,
                &world,
                &c.pcurve,
                tolerance,
                limits.products - out.products,
            )?;
            out.products += proof.products;
            let Some(error) = proof.error_upper else {
                out.reason = "cap-product-budget-exhausted";
                return Ok(out);
            };
            upper = upper.max(error);
        }
    }
    out.boundary_error_upper = Some(upper);
    let Some((defect, products)) =
        bilinear_defect(&f.surface, &rings, limits.products - out.products)?
    else {
        out.reason = "cap-product-budget-exhausted";
        return Ok(out);
    };
    out.products += products;
    out.bilinear_defect_upper = Some(defect);
    let error = I::point(upper).add(I::point(defect))?.hi;
    out.error_upper = Some(error);
    out.within_budget = error <= tolerance;
    out.reason = if out.within_budget {
        "filled-reference-error-certified"
    } else {
        "filled-reference-error-exceeds-budget"
    };
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bilinear_interior_defect_is_nonzero_even_with_exact_boundary_lifts() {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 8.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let square = |lo: f64, hi: f64, weight: f64| {
            let points = [[lo, lo], [hi, lo], [hi, hi], [lo, hi]];
            (0..4)
                .map(|i| Curve {
                    degree: 1,
                    knots: vec![0., 0., 1., 1.],
                    periodic: false,
                    control_points: vec![points[i].to_vec(), points[(i + 1) % 4].to_vec()],
                    weights: vec![weight; 2],
                })
                .collect()
        };
        for inner_weight in [1., 2.] {
            let rings = [square(0., 1., 1.), square(0.25, 0.75, inner_weight)];
            let (bound, products) = bilinear_defect(&s, &rings, 16).unwrap().unwrap();
            // At the common corner and rational radial mixing b=1/2,
            // (C(0,0)+C(1/4,1/4))/2 - C(1/8,1/8) has z=1/8.
            assert!(bound >= 0.125 && bound < 0.376);
            assert_eq!(products, 16);
            assert!(bilinear_defect(&s, &rings, 15).unwrap().is_none());
        }
    }
}
