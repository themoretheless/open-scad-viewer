//! Runnable companion to docs/manifold-user-guide.md.

use manifold_core::{RepairMode, check, metrics, repair_with_mode, validate_indexed_mesh};
use manifold_ops::{BooleanOp, boolean_manifold};

fn tetrahedron() -> (Vec<f64>, Vec<usize>) {
    (
        vec![0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1.],
        vec![0, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3],
    )
}

fn cube(origin: [f64; 3], size: f64) -> (Vec<f64>, Vec<usize>) {
    let [x, y, z] = origin;
    let s = size;
    #[rustfmt::skip]
    let positions = vec![
        x, y, z, x + s, y, z, x + s, y + s, z, x, y + s, z,
        x, y, z + s, x + s, y, z + s, x + s, y + s, z + s, x, y + s, z + s,
    ];
    let indices = vec![
        0, 2, 1, 0, 3, 2, // bottom
        4, 5, 6, 4, 6, 7, // top
        0, 1, 5, 0, 5, 4, // front
        2, 3, 7, 2, 7, 6, // back
        1, 2, 6, 1, 6, 5, // right
        3, 0, 4, 3, 4, 7, // left
    ];
    (positions, indices)
}

fn main() {
    let (positions, indices) = tetrahedron();
    validate_indexed_mesh(&positions, &indices).unwrap();

    let report = check(&positions, &indices);
    assert!(report.is_manifold());

    // Remove the last triangle, then close the remaining simple boundary.
    let open = repair_with_mode(&positions, &indices[..9], 0.0, RepairMode::Full);
    assert!(open.is_manifold());
    assert_eq!(open.report.filled_triangles, 1);

    let measured = metrics(&open.positions, &open.indices);
    assert!(measured.watertight);
    assert!((measured.signed_volume - 1.0 / 6.0).abs() < 1e-12);

    let (ap, ai) = cube([0.0, 0.0, 0.0], 1.0);
    let (bp, bi) = cube([0.5, 0.5, 0.5], 1.0);
    let union = boolean_manifold(&ap, &ai, &bp, &bi, BooleanOp::Union).unwrap();
    assert!(union.output_report.is_manifold());
    assert!((union.volume() - 1.875).abs() < 1e-9);

    println!(
        "tetra volume={:.6}; repaired triangles={}; union volume={:.3}",
        measured.signed_volume,
        open.indices.len() / 3,
        union.volume(),
    );
}
