//! Exercise the production exact primitive without weakening its visibility.
struct Polygon {
    points: Vec<[f64; 2]>,
    vertices: Vec<usize>,
    edges: Vec<usize>,
}
struct Work {
    used: usize,
    limit: usize,
}
#[path = "support/polygon_network_host.rs"]
mod network;
fn pair(a: &[[f64; 2]], b: &[[f64; 2]]) -> (Polygon, Polygon) {
    let mut points = Vec::new();
    let mut edges = std::collections::BTreeMap::new();
    let mut make = |p: &[[f64; 2]]| {
        let vertices = p
            .iter()
            .map(|q| {
                if let Some(i) = points.iter().position(|v| v == q) {
                    i
                } else {
                    points.push(*q);
                    points.len() - 1
                }
            })
            .collect::<Vec<_>>();
        let es = (0..p.len())
            .map(|i| {
                let a = vertices[i];
                let b = vertices[(i + 1) % p.len()];
                let key = (a.min(b), a.max(b));
                let n = edges.len();
                *edges.entry(key).or_insert(n)
            })
            .collect();
        Polygon {
            points: p.to_vec(),
            vertices,
            edges: es,
        }
    };
    (make(a), make(b))
}
fn notch() -> (Polygon, Polygon) {
    pair(
        &[
            [-1., -1.],
            [3., -1.],
            [3., 0.],
            [2., 0.],
            [2., -0.25],
            [0., -0.25],
            [0., 0.],
            [-1., 0.],
        ],
        &[
            [0., -0.25],
            [2., -0.25],
            [2., 0.],
            [2., 0.25],
            [0., 0.25],
            [0., 0.],
        ],
    )
}
#[test]
fn concave_three_edge_contact_is_owned_symmetric_and_bounded() {
    let (a, b) = notch();
    let mut w = Work {
        used: 0,
        limit: 1000,
    };
    let c = network::contact_network(&a, &b, &mut w).unwrap();
    assert_eq!(c.0.len(), 3);
    assert_eq!(c.1.len(), 4);
    assert!(w.used > 0 && w.used <= 1000);
    assert_eq!(
        network::contact_network(
            &b,
            &a,
            &mut Work {
                used: 0,
                limit: 1000
            }
        ),
        Some(c)
    );
    assert!(
        network::contact_network(
            &a,
            &b,
            &mut Work {
                used: 0,
                limit: w.used - 1
            }
        )
        .is_none()
    );
}
#[test]
fn crossing_nested_and_identical_regions_never_become_shared_boundary() {
    for (a, b) in [
        pair(
            &[[0., 0.], [2., 0.], [2., 2.], [0., 2.]],
            &[[1., 0.], [3., 0.], [3., 2.], [1., 2.]],
        ),
        pair(
            &[
                [-2., 0.],
                [-1., -2.],
                [1., -2.],
                [2., 0.],
                [1., 2.],
                [-1., 2.],
            ],
            &[[-2., 0.], [1., -2.], [1., 2.]],
        ),
        pair(
            &[[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
            &[[0., 0.], [0., 1.], [1., 1.], [1., 0.]],
        ),
    ] {
        assert!(
            network::contact_network(
                &a,
                &b,
                &mut Work {
                    used: 0,
                    limit: 1000
                }
            )
            .is_none()
        );
    }
}
#[test]
fn unowned_contacts_t_junctions_and_self_crossings_refuse() {
    let (a, mut b) = notch();
    for v in &mut b.vertices {
        *v += 100;
    }
    for e in &mut b.edges {
        *e += 100;
    }
    assert!(
        network::contact_network(
            &a,
            &b,
            &mut Work {
                used: 0,
                limit: 1000
            }
        )
        .is_none()
    );
    for (a, b) in [
        pair(
            &[[0., 0.], [2., 0.], [2., 2.], [0., 2.]],
            &[[1., 0.], [1., -1.], [3., -1.], [3., 0.]],
        ),
        pair(
            &[[0., 0.], [2., 2.], [0., 2.], [2., 0.]],
            &[[2., 0.], [3., 0.], [3., 1.]],
        ),
    ] {
        assert!(
            network::contact_network(
                &a,
                &b,
                &mut Work {
                    used: 0,
                    limit: 1000
                }
            )
            .is_none()
        );
    }
}
