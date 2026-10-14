//! Binary-coefficient spherical charts with explicitly authored pole quotients.
use super::*;

/// Exact binary-rational construction; the layout is selected before building.
/// Intersection operations tied to stereographic UVs must explicitly support
/// the pole layout before accepting it; no recognition metadata is trusted.
pub fn sphere_exact(radius: f64) -> Result<Model> {
    size(radius, "Sphere radius")?;
    let significand = (radius.to_bits() & ((1_u64 << 52) - 1)) | (1_u64 << 52);
    if significand % 3 == 0 {
        super::sphere_regular::build(radius)
    } else {
        sphere_pole_charts(radius)
    }
}

pub fn sphere_pole_charts(radius: f64) -> Result<Model> {
    size(radius, "Sphere radius")?;
    let mut build = Builder::new();
    let equator = build.ring(radius, 0.);
    let c = [1., 1., 0.];
    let s = [0., 1., 1.];
    let w = [1., 1., 2.];
    for sign in [1., -1.] {
        let pole = build.ring(0., sign * radius)[0];
        for q in 0..4 {
            let next = (q + 1) % 4;
            let a = QUADRANTS[q];
            let b = QUADRANTS[next];
            let longitude = rational_quarter_arc(1., 0., q);
            let surface = Surface {
                degree_u: 2,
                degree_v: 2,
                knots_u: vec![0., 0., 0., 1., 1., 1.],
                knots_v: vec![0., 0., 0., 1., 1., 1.],
                control_points: (0..3)
                    .map(|i| {
                        (0..3)
                            .map(|j| {
                                vec![
                                    radius * c[i] * longitude.control_points[j][0],
                                    radius * c[i] * longitude.control_points[j][1],
                                    sign * radius * s[i],
                                ]
                            })
                            .collect()
                    })
                    .collect(),
                weights: (0..3)
                    .map(|i| (0..3).map(|j| w[i] * w[j]).collect())
                    .collect(),
                periodic_u: false,
                periodic_v: false,
            };
            let meridian = |d: [f64; 2]| Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![
                    vec![radius * d[0], radius * d[1], 0.],
                    vec![radius * d[0], radius * d[1], sign * radius],
                    vec![0., 0., sign * radius],
                ],
                weights: w.to_vec(),
                periodic: false,
            };
            let edges = [
                meridian(a),
                line(vec![0., 0., sign * radius], vec![0., 0., sign * radius]),
                reversed(meridian(b)),
                reversed(rational_quarter_arc(radius, 0., q)),
            ];
            let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
            let corners = [equator[q], pole, pole, equator[next]];
            let uses = edges
                .into_iter()
                .enumerate()
                .map(|(k, curve)| {
                    build.coedge(
                        corners[k],
                        corners[(k + 1) % 4],
                        curve,
                        line(uv[k].to_vec(), uv[(k + 1) % 4].to_vec()),
                    )
                })
                .collect();
            let outer = build.wire(uses);
            build.face(surface, outer, vec![], sign > 0.);
        }
    }
    build.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_constructor_selects_representable_layout_before_certification() {
        for radius in [2., 3., 3_f64.next_up(), 12., 0.125] {
            let model = sphere_exact(radius).unwrap();
            let report = crate::boundary_agreement::verify_exact(&model, 1000000).unwrap();
            assert!(
                report.all_equal && report.all_joins_exact,
                "radius={radius}"
            );
            let significand = (radius.to_bits() & ((1_u64 << 52) - 1)) | (1_u64 << 52);
            assert_eq!(
                model.edges.iter().any(|e| e.degenerate),
                significand % 3 != 0
            );
        }
        for radius in [0., -1., f64::NAN, f64::INFINITY] {
            assert!(sphere_exact(radius).is_err());
        }
    }

    #[test]
    fn binary_sphere_poles_have_exact_boundaries_and_quotient_charts() {
        for radius in [2., 3., 0.125] {
            let m = sphere_pole_charts(radius).unwrap();
            let boundary = crate::boundary_agreement::verify_exact(&m, 1000000).unwrap();
            assert!(
                boundary.all_equal && boundary.all_joins_exact,
                "r={radius}: exact={} joins={}",
                boundary.all_equal,
                boundary.all_joins_exact
            );
            let charts = crate::face_injectivity::inspect(&m, 10000).unwrap();
            assert!(charts.all_faces_injective, "r={radius}: {charts:?}");
            let limits = crate::boundary_embedding::Limits {
                exact_work: 1000000,
                trim_pairs: 1000,
                trim_cells: 10000,
                trim_domain_cells: 100000,
                spans: 10000,
                contacts: crate::face_contacts::Limits {
                    pairs: 100,
                    cells: 10000,
                    domain_cells: 100000,
                    cells_per_pair: 64,
                    domain_cells_per_pair: 1000,
                },
            };
            let boundary = crate::boundary_embedding::inspect(&m, 1e-8, limits).unwrap();
            assert!(
                boundary.proven,
                "r={radius}: exact={} trim={} faces={} pairs={:?}",
                boundary.agreement.all_equal,
                boundary.trim.all_valid,
                boundary.intersections.faces.all_faces_injective,
                boundary
                    .intersections
                    .pairs
                    .pairs
                    .iter()
                    .map(|p| (p.faces, p.reason))
                    .collect::<Vec<_>>()
            );
        }
    }
    #[test]
    fn pole_regular_boundary_requires_complete_budget_and_owned_support_contacts() {
        let model = sphere_pole_charts(2.).unwrap();
        let before = format!("{model:?}");
        let proof = crate::boundary_embedding::regularity::inspect(&model, 10000).unwrap();
        assert!(proof.all_faces_regular_modulo_poles);
        assert!(
            !proof.all_faces_regular,
            "a pole is not an ordinary nonzero Jacobian chart"
        );
        assert_eq!(proof.pole_quotients.iter().flatten().count(), 8);
        let short =
            crate::boundary_embedding::regularity::inspect(&model, proof.cells - 1).unwrap();
        assert!(!short.all_faces_regular_modulo_poles);
        assert!(!short.unresolved_faces.is_empty());
        assert!(short.cells <= proof.cells - 1);
        let zero = crate::boundary_embedding::regularity::inspect(&model, 0).unwrap();
        assert_eq!(zero.unresolved_faces, (0..8).collect::<Vec<_>>());
        assert!(
            crate::shared_boundary::inspect_opposite_pair(&model, [0, 1])
                .unwrap()
                .is_some()
        );
        let mut extra_contact = model.clone();
        extra_contact.faces[0].surface.control_points[0][0][0] = 0.;
        assert!(
            crate::shared_boundary::inspect_opposite_pair(&extra_contact, [0, 1])
                .unwrap()
                .is_none()
        );
        let mut forged = model.clone();
        let pole_edge = proof.pole_quotients[0].as_ref().unwrap().edge;
        forged.edges[pole_edge].degenerate = false;
        assert!(crate::boundary_embedding::regularity::inspect(&forged, 10000).is_err());
        assert_eq!(format!("{model:?}"), before);
    }
}
