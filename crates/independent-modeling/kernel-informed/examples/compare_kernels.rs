//! Reproducible geometric scenarios; not a performance benchmark or a test of
//! the installed Plasticity runtime. No target artifacts are read or executed.
use modeling_kernel_informed::{
    brep::{Body, Frame, Profile},
    curve::Curve,
    loft,
    math::*,
    planar::Convex,
};
use std::f64::consts::{FRAC_PI_4, PI};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/kernel-comparison".to_owned());
    let directory = std::path::Path::new(&directory);
    std::fs::create_dir_all(directory)?;
    let radius = 2.;
    let height = 5.;
    let reference = PI * radius * radius * height;
    let clean = modeling_clean::mesh::Mesh::cylinder(radius, height, 32)?;
    let retained = Body::extrude(
        Profile::disk([0.; 2], radius)?,
        Frame::xy(),
        [0., 0., height],
        1e-9,
    )?;
    let coarse = retained.tessellate(0.1, 4096)?;
    let fine = retained.tessellate(0.001, 4096)?;
    assert!(
        coarse.mesh.topology().is_closed_manifold() && fine.mesh.topology().is_closed_manifold()
    );
    let annulus = Body::extrude(
        Profile::annulus([0.; 2], 3., 1.)?,
        Frame::xy(),
        [0., 0., 4.],
        1e-9,
    )?;
    let annulus_mesh = annulus.tessellate(0.005, 4096)?;
    let lower = Curve::circle([0.; 3], 2.)?;
    let upper = Curve::circle([0., 0., 4.], 3.)?
        .insert_knot(0.13)?
        .insert_knot(0.77)?;
    let clean_loft_rejected =
        modeling_clean::surface::Surface::loft(&[lower.clone(), upper.clone()]).is_err();
    let compatible = loft::loft(&[lower.clone(), upper.clone()])?;
    let mut max_error: f64 = 0.;
    for i in 0..=200 {
        let t = i as f64 / 200.;
        max_error = max_error.max(norm(sub(
            compatible.surface.evaluate(0., t)?,
            lower.evaluate(t)?,
        )));
        max_error = max_error.max(norm(sub(
            compatible.surface.evaluate(1., t)?,
            upper.evaluate(t)?,
        )));
    }
    let bounds = Bounds::new([-1.; 3], [1.; 3])?;
    let a = Convex::cuboid(bounds, &Affine::identity(), 1e-9)?;
    let b = a.transformed(&Affine::rotation([0., 0., 1.], FRAC_PI_4)?)?;
    let overlap = a
        .intersection(&b)?
        .body
        .ok_or("expected nonempty intersection")?;
    let reference_intersection = 16. * (2f64.sqrt() - 1.);
    let partition = a.subtract_partition(&b)?;
    let partition_error = (partition
        .outside
        .iter()
        .map(Convex::volume)
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .sum::<f64>()
        + partition.overlap.as_ref().unwrap().volume()?
        - 8.)
        .abs();
    let report = format!(
        r#"{{
  "scope": "local clean versus independent informed geometry; no Plasticity execution or speed claim",
  "cylinder": {{
    "radius": {radius}, "height": {height}, "reference_volume": {reference},
    "clean_representation": "32-segment faceted solid",
    "clean_volume": {}, "clean_absolute_volume_error": {},
    "informed_representation": "retained rational boundary and analytic extrusion measure",
    "informed_volume": {}, "informed_absolute_volume_error": {},
    "informed_retained_faces": {}, "informed_retained_edges": {},
    "coarse_display_triangles": {}, "fine_display_triangles": {},
    "fine_boundary_chord_estimate": {}
  }},
  "annulus": {{"retained_faces": {}, "cap_boundary_loops": {}, "analytic_volume": {}, "display_closed_manifold": {}}},
  "incompatible_knots_loft": {{"clean_rejected": {clean_loft_rejected}, "informed_accepted": true, "common_spans": {}, "maximum_endpoint_profile_error_201_samples": {max_error}}},
  "rotated_convex_intersection": {{"clean_equivalent_api": "unavailable: clean CSG takes only axis-aligned boxes", "reference_volume": {reference_intersection}, "informed_volume": {}, "absolute_volume_error": {}, "closed_manifold": {}}},
  "convex_difference_partition": {{"pieces": {}, "volume_conservation_error": {partition_error}, "sewn_external_shell": false}}
}}
"#,
        clean.signed_volume(),
        (clean.signed_volume() - reference).abs(),
        retained.volume(),
        (retained.volume() - reference).abs(),
        retained.faces().len(),
        retained.edges().len(),
        coarse.mesh.triangles().len(),
        fine.mesh.triangles().len(),
        fine.boundary_chord_deviation,
        annulus.faces().len(),
        annulus.faces()[0].loops.len(),
        annulus.volume(),
        annulus_mesh.mesh.topology().is_closed_manifold(),
        compatible.report.spans,
        overlap.volume()?,
        (overlap.volume()? - reference_intersection).abs(),
        overlap.mesh()?.topology().is_closed_manifold(),
        partition.outside.len()
    );
    assert!(max_error < 1e-9 && partition_error < 1e-8);
    std::fs::write(directory.join("comparison.json"), &report)?;
    for (name, mesh) in [
        ("clean-cylinder", clean),
        ("informed-cylinder", fine.mesh),
        ("informed-annulus", annulus_mesh.mesh),
        ("rotated-intersection", overlap.mesh()?),
    ] {
        std::fs::write(directory.join(format!("{name}.obj")), mesh.to_obj())?;
    }
    println!("{report}");
    Ok(())
}
