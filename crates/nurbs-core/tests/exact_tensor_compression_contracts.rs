use nurbs_core::{editing::exact_surface_knot_removal::compress_tensor_surface, surface::Surface};

const WORK: u64 = 1_000_000;

fn refined_surface(quadratic: bool) -> Surface {
    let stations = [0., 0.25, 0.5, 0.75, 1.];
    let quadratic_values = [0., 0., 0.25, 0.5, 1.];
    let control_points = stations
        .iter()
        .enumerate()
        .map(|(i, &u)| {
            stations
                .iter()
                .map(|&v| {
                    let z = if quadratic { quadratic_values[i] } else { u };
                    vec![u, v, z + 2. * v]
                })
                .collect()
        })
        .collect();
    let knots = vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.];
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: knots.clone(),
        knots_v: knots,
        control_points,
        weights: vec![vec![1.; 5]; 5],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn removes_both_redundant_degrees_and_knots_from_an_affine_chart() {
    let source = refined_surface(false);
    let input_net = source.control_points.clone();
    let r = compress_tensor_surface(&source, WORK).unwrap();
    assert!(r.completed, "{r:?}");
    assert_eq!(r.reduced_degrees, [1, 1]);
    assert_eq!(r.removed_knots, [2, 2]);
    assert_eq!(
        (
            r.surface.control_points.len(),
            r.surface.control_points[0].len()
        ),
        (2, 2)
    );
    assert_eq!(source.control_points, input_net);
    assert_eq!(
        r.surface.control_points,
        vec![
            vec![vec![0., 0., 0.], vec![0., 1., 2.]],
            vec![vec![1., 0., 1.], vec![1., 1., 3.]],
        ]
    );
}

#[test]
fn keeps_the_degree_required_by_the_original_quadratic_law() {
    let source = refined_surface(true);
    let r = compress_tensor_surface(&source, WORK).unwrap();
    assert!(r.completed, "{r:?}");
    assert_eq!((r.surface.degree_u, r.surface.degree_v), (2, 1));
    assert_eq!(r.reduced_degrees, [0, 1]);
    assert_eq!(r.surface.control_points.len(), 3);
    for u in [0., 0.125, 0.5, 1.] {
        let point = r.surface.evaluate(u, 0.25).unwrap().point;
        assert!((point[2] - (u * u + 0.5)).abs() < 1e-14);
    }
}

#[test]
fn preserves_the_original_rational_weight_function() {
    let mut source = refined_surface(false);
    source.knots_u = vec![0., 0., 0., 1., 1., 1.];
    source.control_points = [0., 0.75, 1.]
        .map(|x| [0., 0.25, 0.5, 0.75, 1.].map(|v| vec![x, v, 0.]).to_vec())
        .to_vec();
    source.weights = [1., 2., 3.].map(|w| vec![w; 5]).to_vec();
    let r = compress_tensor_surface(&source, WORK).unwrap();
    assert!(r.completed, "{r:?}");
    assert_eq!(r.surface.weights, vec![vec![1.; 2], vec![3.; 2]]);
    let point = r.surface.evaluate(0.25, 0.5).unwrap().point;
    assert!((point[0] - 0.5).abs() < 1e-14);
    assert!((point[1] - 0.5).abs() < 1e-14);
}

#[test]
fn work_exhaustion_never_commits_unproved_steps() {
    let source = refined_surface(false);
    let tiny = compress_tensor_surface(&source, 1).unwrap();
    assert!(!tiny.completed);
    assert_eq!(tiny.surface.control_points, source.control_points);
    let full = compress_tensor_surface(&source, WORK).unwrap();
    let budget = full.work_used / 2;
    let partial = compress_tensor_surface(&source, budget).unwrap();
    assert!(partial.work_used <= budget && !partial.completed);
    for u in [0., 0.125, 0.5, 1.] {
        let point = partial.surface.evaluate(u, 0.25).unwrap().point;
        assert!((point[2] - (u + 0.5)).abs() < 1e-14);
    }
}
