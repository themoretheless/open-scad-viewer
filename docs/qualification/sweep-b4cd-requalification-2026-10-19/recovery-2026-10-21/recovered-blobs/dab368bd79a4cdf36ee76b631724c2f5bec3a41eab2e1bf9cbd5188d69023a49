fn main() {
    for (name, model) in [
        ("box", brep_core::cuboid([0.; 3], [1.; 3]).unwrap()),
        ("cylinder", brep_core::cylinder(2., 3.).unwrap()),
        ("sphere", brep_core::sphere(2.).unwrap()),
    ] {
        let start = std::time::Instant::now();
        let report = brep_core::boundary_agreement::verify(&model, 4096).unwrap();
        println!(
            "{name}: complete={} cells={} uses={} unresolved={} mismatch={} elapsed_ms={}",
            report.complete,
            report.cells,
            report.uses.len(),
            report
                .uses
                .iter()
                .filter(|u| u.status == nurbs_core::curve_surface_agreement::Status::Unresolved)
                .count(),
            report
                .uses
                .iter()
                .filter(|u| u.status == nurbs_core::curve_surface_agreement::Status::Mismatch)
                .count(),
            start.elapsed().as_millis()
        );
    }
}
