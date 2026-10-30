use super::*;

#[test]
fn primitives_closed() {
    for m in [
        cube([2., 3., 4.], false).unwrap(),
        sphere(3., 24).unwrap(),
        cylinder(4., 2., 0., 24, false).unwrap(),
        torus(5., 1.5, 24, 16),
        capsule(2., 6., 24),
        cone_frustum(3., 1.5, 5., 24),
        cone_frustum(3., 0., 5., 24),
        ellipsoid(4., 2.5, 1.5, 16, 24),
    ] {
        let r = m.inspect().unwrap();
        assert!(r.closed, "{r:?}");
        assert_eq!(r.degenerate_triangles, 0);
        assert!(r.signed_volume_mm3 > 0.)
    }
}

#[test]
fn hull_box() {
    let m = cube([2., 3., 4.], false).unwrap();
    let h = hull3(&[m]).unwrap();
    let r = h.inspect().unwrap();
    assert!(r.closed);
    assert!((r.signed_volume_mm3 - 24.).abs() < 1e-8)
}

#[test]
fn prism_difference_accepts_a_cutter_that_spans_the_base() {
    let plate = cube([10., 10., 2.], true).unwrap();
    let hole = cylinder(4., 1., 1., 32, true).unwrap();
    let drilled = prism_boolean(&plate, &hole, "difference")
        .unwrap()
        .expect("a spanning vertical cutter keeps the result a prism");
    let report = drilled.inspect().unwrap();
    assert!(report.closed);
    let hole_area = 16. * (std::f64::consts::PI / 16.).sin();
    assert!((report.signed_volume_mm3 - (200. - 2. * hole_area)).abs() < 1e-6);

    // A cutter that stops inside the plate makes a pocket, not a prism.
    let pocket = cylinder(1., 1., 1., 32, false).unwrap();
    assert!(
        prism_boolean(&plate, &pocket, "difference")
            .unwrap()
            .is_none()
    );
    // Union still needs coincident end planes.
    assert!(prism_boolean(&plate, &hole, "union").unwrap().is_none());
    // Intersection uses the overlap of both ranges.
    let tall = cube([4., 4., 10.], false).unwrap();
    let overlap = prism_boolean(&plate, &tall, "intersection")
        .unwrap()
        .unwrap();
    assert!((overlap.inspect().unwrap().signed_volume_mm3 - 16.).abs() < 1e-8);
    // A cutter covering the whole base leaves nothing.
    let everything = cube([20., 20., 4.], true).unwrap();
    assert!(
        prism_boolean(&plate, &everything, "difference")
            .unwrap()
            .unwrap()
            .indices
            .is_empty()
    );
}

/// Regression for the aligned 4x4 bridge failure and the former 2048-vertex
/// ceiling. These valid profiles must stay on the exact prism path.
#[test]
fn prism_difference_grid_of_holes_preserves_closed_volume() {
    let plate = cube([86., 86., 8.], true).unwrap();
    let hole = cylinder(12., 3., 3., 32, true).unwrap();
    let grid = |count: usize| -> Mesh {
        let side = (count as f64).sqrt().ceil() as usize;
        let step = 80. / side as f64;
        let holes: Vec<Mesh> = (0..count)
            .map(|i| {
                let x = -40. + step / 2. + (i % side) as f64 * step;
                let y = -40. + step / 2. + (i / side) as f64 * step;
                hole.transform([
                    [1., 0., 0., x],
                    [0., 1., 0., y],
                    [0., 0., 1., 0.],
                    [0., 0., 0., 1.],
                ])
                .unwrap()
            })
            .collect();
        join(&holes).unwrap()
    };
    for count in [4, 16, 36, 64, 100] {
        let result = prism_boolean(&plate, &grid(count), "difference")
            .unwrap()
            .unwrap();
        let report = result.inspect().unwrap();
        assert!(report.closed, "{count} holes");
        let expected =
            86. * 86. * 8. - count as f64 * 16. * 9. * (std::f64::consts::PI / 16.).sin() * 8.;
        assert!(
            (report.signed_volume_mm3 - expected).abs() < expected * 1e-10,
            "{count} holes"
        );
    }
}
