use super::*;
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
};
fn roof(rational: bool) -> Surface {
    let x = if rational {
        vec![0., 0.0625, 0.5, 0.9375, 1.]
    } else {
        vec![0., 0.5, 1.]
    };
    let square = [0., 0., 1., 26.25, 30.];
    let degree = x.len() - 1;
    Surface {
        degree_u: degree,
        degree_v: 2,
        knots_u: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: x
            .iter()
            .enumerate()
            .map(|(i, &x)| {
                (0..3)
                    .map(|j| {
                        vec![
                            x,
                            j as f64 / 2.,
                            2. + if rational {
                                square[i] / 64.
                            } else {
                                [0.25, -0.25, 0.25][i]
                            } + [0.25, -0.25, 0.25][j],
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: if rational {
            [1., 4., 5., 4., 1.]
                .into_iter()
                .map(|w| vec![w; 3])
                .collect()
        } else {
            vec![vec![1.; 3]; 3]
        },
        periodic_u: false,
        periodic_v: false,
    }
}
fn boundaries(s: &Surface) -> [crate::circular_blend::CircularBlendBoundary; 4] {
    let u = [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]];
    let v = [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]];
    let p = [[u[0], v[0]], [u[1], v[0]], [u[1], v[1]], [u[0], v[1]]];
    std::array::from_fn(|i| {
        let a = p[i];
        let b = p[(i + 1) % 4];
        let (axis, fixed, lo, hi) = if a[0] == b[0] {
            (Axis::U, a[0], a[1], b[1])
        } else {
            (Axis::V, a[1], a[0], b[0])
        };
        let curve = s.iso(axis, fixed).unwrap();
        crate::circular_blend::CircularBlendBoundary {
            curve: if lo > hi {
                curve.reverse().unwrap()
            } else {
                curve
            },
            pcurve: Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap(),
            collapsed_pole: None,
        }
    })
}
fn model(rational: bool) -> Model {
    column(roof(rational))
}
pub(crate) fn column(top: Surface) -> Model {
    column_at_floor(top, 0.)
}
pub(crate) fn column_at_floor(top: Surface, floor: f64) -> Model {
    let mut bottom = top.clone();
    for p in bottom.control_points.iter_mut().flatten() {
        p[2] = floor;
    }
    let bottom = nurbs_core::surface_edit::reverse(&bottom, Axis::V).unwrap();
    let mut surfaces = vec![top.clone(), bottom];
    for edge in boundaries(&top) {
        let high = edge.curve;
        let controls = high
            .control_points
            .iter()
            .map(|p| {
                let mut low = p.clone();
                low[2] = floor;
                let mut middle = p.clone();
                middle[2] = (middle[2] + floor) * 0.5;
                vec![low, middle, p.clone()]
            })
            .collect();
        surfaces.push(Surface {
            degree_u: high.degree,
            degree_v: 2,
            knots_u: high.knots.clone(),
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: controls,
            weights: high.weights.iter().map(|&w| vec![w; 3]).collect(),
            periodic_u: false,
            periodic_v: false,
        });
    }
    let sheets = surfaces
        .into_iter()
        .map(|s| {
            let wire = boundaries(&s);
            (
                crate::circular_blend::open_face(s, wire, 1e-7).unwrap(),
                false,
            )
        })
        .collect();
    let mut model = crate::circular_blend::assemble_support_sheets(sheets).unwrap();
    model.0.shells[0].closed = true;
    model.0.bodies.push(crate::Body {
        outer_shell: 0,
        inner_shells: vec![],
    });
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}
pub(crate) fn limits() -> Limits {
    Limits {
        operations: 1000000,
        body: source_body_restore::Limits {
            shell: crate::source_shell_restore::Limits {
                regions: crate::source_region_restore::test_limits(),
                exact_work: 100000000,
                driver_cells: 10000,
            },
            embedding: crate::source_shell_geometry::Limits {
                tolerance_uv: 1e-8,
                corners: 1000,
                spans: 100000,
                linear_cells: 100000,
                pairs: crate::face_contacts::Limits {
                    pairs: 15,
                    cells: 100000,
                    domain_cells: 100000,
                    cells_per_pair: 10000,
                    domain_cells_per_pair: 10000,
                },
                exact_work: 100000000,
                driver_cells: 100000,
            },
            volume: crate::source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 0.01,
                tolerance_uv: 1e-8,
                cells: 10000,
                spans: 100000,
                domain_cells: 100000,
            },
        },
    }
}
#[test]
fn actual_original_graph_column_has_coefficient_and_embedded_body_authority() {
    for rational in [false, true] {
        let model = model(rational);
        let saved = model.clone();
        let mut budget = limits();
        let attempt = qualify(&model, 0, 1, &budget);
        let proof = attempt.unwrap_or_else(|e| panic!("rational={rational}: {e:?}"));
        assert_eq!(proof.footprint(), [[0., 1.], [0., 1.]]);
        assert_eq!(proof.floor_height(), 0.);
        assert_eq!(proof.walls(), [5, 3, 2, 4]);
        assert!(
            proof
                .equivalent_body()
                .geometry()
                .contacts()
                .all_pairs_qualified
        );
        let expected = if rational {
            2. + 10. / 64. + 1. / 12.
        } else {
            2. + 1. / 6.
        };
        let volume = proof.equivalent_body().volume();
        assert!(volume[0] <= expected && expected <= volume[1]);
        proof.reverify().unwrap();
        assert_eq!(proof.equivalences().len(), 6);
        for (face, equivalent) in proof.equivalences().iter().enumerate() {
            assert_eq!(equivalent.source(), &model.faces[face].surface);
            assert_eq!(
                equivalent.polynomial(),
                proof.equivalent_body().geometry().shell().faces()[face][0].edges()[0].surface()
            );
            equivalent.reverify().unwrap();
        }

        let work = proof.operations();
        drop(proof);
        budget.operations = work;
        assert_eq!(qualify(&model, 0, 1, &budget).unwrap().operations(), work);
        budget.operations = work - 1;
        assert!(qualify(&model, 0, 1, &budget).is_err());
        assert_eq!(model, saved);
    }
}
#[test]
fn exact_in_plane_wall_reparameterization_keeps_the_same_material_column() {
    let mut model = model(false);
    let original = model.clone();
    model.0.faces[5].surface.control_points[1][1][2] += 0.125;
    model.rebuild_topology_ids();
    model.validate().unwrap();
    let budget = limits();
    let proof = qualify(&model, 0, 1, &budget).unwrap();
    let volume = proof.equivalent_body().volume();
    assert!(volume[0] <= 2. + 1. / 6. && 2. + 1. / 6. <= volume[1]);
    assert_eq!(model.edges, original.edges);
    assert_eq!(model.loops, original.loops);
    proof.reverify().unwrap();
}

#[test]
fn cached_closed_flags_and_sampled_agreement_do_not_admit_false_graph_columns() {
    let model = model(false);
    let budget = limits();
    let mut changed = model.clone();
    changed.0.faces[0].surface.control_points[1][1][0] = 0.5f64.next_up();
    changed.rebuild_topology_ids();
    changed.validate().unwrap();
    assert!(qualify(&changed, 0, 1, &budget).is_err());
    let mut changed = model.clone();
    changed.0.vertices[0].point[2] = changed.vertices[0].point[2].next_up();
    changed.rebuild_topology_ids();
    changed.validate().unwrap();
    assert!(qualify(&changed, 0, 1, &budget).is_err());
    let mut changed = model.clone();
    changed.0.faces[5].surface.control_points[1][1][0] = 0f64.next_up();
    changed.rebuild_topology_ids();
    changed.validate().unwrap();
    assert!(qualify(&changed, 0, 1, &budget).is_err());
    let mut changed = model.clone();
    changed.0.faces[5].surface.control_points[1][1][2] = 100.;
    changed.rebuild_topology_ids();
    changed.validate().unwrap();
    assert!(qualify(&changed, 0, 1, &budget).is_err());
    let mut changed = model.clone();
    changed.0.shells[0].faces[0].reversed = true;
    changed.rebuild_topology_ids();
    assert!(qualify(&changed, 0, 1, &budget).is_err());
    assert!(qualify(&model, 1, 0, &budget).is_err());
    let mut stopped = limits();
    stopped.body.embedding.pairs.pairs = 1;
    assert!(qualify(&model, 0, 1, &stopped).is_err());
}
