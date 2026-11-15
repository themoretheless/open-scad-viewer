use nurbs_core::{
    surface::Surface,
    surface_chord_bound::{
        cell_interpolation_error, grid_interpolation_error, triangle_interpolation_error,
    },
};

const UNIT: [[f64; 2]; 2] = [[0., 1.], [0., 1.]];

// S(u,v) = (u, v, u² + uv + v²). For the diagonal pair with equal
// weights the error is 3/4, attaining the rectangle-wide bound.
fn quadratic(split_u: bool) -> Surface {
    let (u, uu, knots_u) = if split_u {
        (
            vec![0., 0.25, 0.75, 1.],
            vec![0., 0., 0.5, 1.],
            vec![0., 0., 0., 0.5, 1., 1., 1.],
        )
    } else {
        (
            vec![0., 0.5, 1.],
            vec![0., 0., 1.],
            vec![0., 0., 0., 1., 1., 1.],
        )
    };
    let control_points = u
        .iter()
        .zip(uu)
        .map(|(&u, uu)| {
            [0., 0.5, 1.]
                .into_iter()
                .zip([0., 0., 1.])
                .map(|(v, vv)| vec![u, v, uu + u * v + vv])
                .collect()
        })
        .collect();
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u,
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points,
        weights: vec![vec![1.; 3]; u.len()],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn diagonal_error_attains_bound_and_subcell_scales_quadratically() {
    let s = quadratic(false);
    let whole = cell_interpolation_error(&s, UNIT, 1).unwrap();
    assert!((0.75..0.750_000_01).contains(&whole), "{whole}");
    let sub = cell_interpolation_error(&s, [[0.25, 0.75]; 2], 1).unwrap();
    assert!((0.1875..0.187_500_01).contains(&sub), "{sub}");
}

#[test]
fn nonuniform_axis_scaling_across_c1_knot_is_outward() {
    let s = quadratic(true);
    let domain = [[0.25, 1.], [0., 0.5]];
    let bound = cell_interpolation_error(&s, domain, 2).unwrap();
    // Exact diagonal error: (3/4)²/4 + (3/4)(1/2)/4 + (1/2)²/4.
    assert!((0.296875..0.296_875_01).contains(&bound), "{bound}");
    assert!(cell_interpolation_error(&s, domain, 1).is_err());
}

#[test]
fn rejects_bad_cells_and_unvalidated_storage() {
    let mut s = quadratic(true);
    assert!(cell_interpolation_error(&s, [[0., f64::NAN], [0., 1.]], 2).is_err());
    assert!(cell_interpolation_error(&s, [[0., 0.], [0., 1.]], 2).is_err());
    assert!(cell_interpolation_error(&s, UNIT, 0).is_err());
    assert!(cell_interpolation_error(&s, UNIT, 4097).is_err());
    s.knots_u.insert(4, 0.5);
    s.control_points.insert(2, s.control_points[2].clone());
    s.weights.insert(2, vec![1.; 3]);
    assert_eq!(
        cell_interpolation_error(&s, UNIT, 2).unwrap_err().code,
        "NURBS_CHORD_CELL_CROSSES_KINK"
    );
    assert!(cell_interpolation_error(&s, [[0., 0.5], [0., 1.]], 1).is_ok());
    s.control_points.clear();
    assert!(cell_interpolation_error(&s, UNIT, 2).is_err());
}

#[test]
fn rational_denominator_is_included_in_the_interpolation_bound() {
    // S(u,v) = (2u/(1+u), v, 0). At (1/2,1/2), the diagonal chord
    // is exactly 1/6 away. A polynomial treatment of the same poles
    // would give zero error because their unweighted net is planar affine.
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2], vec![2.; 2]],
        periodic_u: false,
        periodic_v: false,
    };
    let bound = cell_interpolation_error(&s, UNIT, 1).unwrap();
    assert!(
        bound >= (1f64 / 6.).next_up() && bound.is_finite(),
        "{bound}"
    );
}

#[test]
fn stored_triangle_bound_includes_vertex_displacement() {
    let s = quadratic(false);
    let uv = [[0., 0.], [1., 1.], [0., 1.]];
    let stored = [[0., 0., 0.25], [1., 1., 3.25], [0., 1., 1.25]];
    let error = triangle_interpolation_error(&s, uv, stored, 1).unwrap();
    assert!((0.75..0.750_000_01).contains(&error.interpolation));
    assert!((0.25..0.250_000_01).contains(&error.vertex_displacement));
    // At the midpoint of the diagonal, the two deviations have the
    // same direction. The surface is one whole unit below the mesh.
    assert!((1.0..1.000_000_01).contains(&error.total));
}

#[test]
fn stored_triangle_validates_coordinates_and_respects_cell_budget() {
    let s = quadratic(true);
    let uv = [[0., 0.], [1., 1.], [0., 1.]];
    let stored = [[0., 0., 0.], [1., 1., 3.], [0., 1., 1.]];
    assert!(triangle_interpolation_error(&s, uv, stored, 1).is_err());
    let mut invalid = stored;
    invalid[1][2] = f64::INFINITY;
    assert!(triangle_interpolation_error(&s, uv, invalid, 2).is_err());
    let mut outside = uv;
    outside[0][1] = -0.5;
    assert!(triangle_interpolation_error(&s, outside, stored, 2).is_err());
    let error = triangle_interpolation_error(&s, uv, stored, 2).unwrap();
    assert!((0.75..0.750_000_01).contains(&error.total));
}

fn quadratic_grid(u: &[f64], v: &[f64]) -> Vec<[f64; 3]> {
    u.iter()
        .flat_map(|&u| v.iter().map(move |&v| [u, v, u * u + u * v + v * v]))
        .collect()
}

#[test]
fn certifies_complete_grid_and_identifies_worst_cell() {
    let s = quadratic(true);
    let lines = [0., 0.5, 1.];
    let mut vertices = quadratic_grid(&lines, &lines);
    vertices[8][2] += 0.375;
    let report = grid_interpolation_error(&s, &lines, &lines, &vertices, 4).unwrap();
    assert_eq!(report.knot_rectangles, 4);
    assert_eq!(report.worst_cell, [1, 1]);
    assert!((0.5625..0.562_500_01).contains(&report.upper), "{report:?}");
}

#[test]
fn global_grid_work_charges_the_original_knot_pieces() {
    let s = quadratic(true);
    let u = [0., 1.];
    let v = [0., 0.5, 1.];
    let vertices = quadratic_grid(&u, &v);
    assert!(grid_interpolation_error(&s, &u, &v, &vertices, 3).is_err());
    let report = grid_interpolation_error(&s, &u, &v, &vertices, 4).unwrap();
    assert_eq!(report.knot_rectangles, 4);
    assert!((0.4375..0.437_500_01).contains(&report.upper), "{report:?}");
}

#[test]
fn grid_refuses_missing_vertices_and_out_of_order_parameters() {
    let s = quadratic(false);
    let u = [0., 0.5, 1.];
    let v = [0., 1.];
    let mut vertices = quadratic_grid(&u, &v);
    assert!(grid_interpolation_error(&s, &u, &v, &vertices[1..], 2).is_err());
    assert!(grid_interpolation_error(&s, &[0., 0.5, 0.5], &v, &vertices, 2).is_err());
    assert!(grid_interpolation_error(&s, &[0., 1., 0.5], &v, &vertices, 2).is_err());
    vertices[5][2] = f64::NAN;
    assert!(grid_interpolation_error(&s, &u, &v, &vertices, 2).is_err());
}
