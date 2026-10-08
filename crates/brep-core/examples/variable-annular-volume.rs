//! Explicit collapsed-chart boundary proof before material-volume admission.
#[allow(dead_code)]
#[path = "variable-annular-wall-coverage.rs"]
mod wall;
fn main() {
    let output = std::env::args().nth(1).expect("output JSON path");
    let mut cases = Vec::new();
    for radii in [[0.5, 1.25], [1.25, 0.5]] {
        for direction in [1., -1.] {
            let model = brep_core::circular_blend::partial_annular_variable_arc(
                20.,
                5.,
                6.,
                radii,
                [0.3, 0.7, 0.4].map(|x| x * direction),
                1e-7,
            )
            .unwrap();
            let mut limits = wall::limits().wall.material.volume;
            limits.boundary.trim_cells = 100000;
            limits.boundary.trim_domain_cells = 1000000;
            limits.boundary.spans = 10000;
            limits.boundary.contacts.cells = 1000000;
            limits.boundary.contacts.domain_cells = 8000000;
            limits.boundary.contacts.cells_per_pair = 4096;
            limits.boundary.contacts.domain_cells_per_pair = 100000;
            limits.orientation_cells = 100000;
            limits.orientation_domain_cells = 1000000;
            limits.orientation_spans = 10000;
            let r = brep_core::volume_validity::inspect_with_linear(&model, 1e-8, limits, 100000)
                .unwrap();
            cases.push(value_codec::json!({"radii":radii,"direction":direction,"proven":r.proven,"boundaryProven":r.boundary.proven,"hullCertificates":r.boundary.hull_contacts.len(),"pairs":r.boundary.intersections.pairs.pairs.iter().map(|p|value_codec::json!({"faces":p.faces,"reason":p.reason})).collect::<Vec<_>>(),"agreementEqual":r.boundary.agreement.all_equal,"joinsExact":r.boundary.agreement.all_joins_exact,"trimsValid":r.boundary.trim.all_valid,"allFacesInjective":r.boundary.intersections.faces.all_faces_injective,"allPairsClassified":r.boundary.intersections.pairs.all_pairs_classified,"absenceProven":r.boundary.intersections.absence_proven,"nestingRoles":r.nesting.as_ref().and_then(|n|n.roles_consistent),"orientationCells":r.orientation_cells,"orientationDomainCells":r.orientation_domain_cells,"orientations":r.orientations.iter().map(|o|value_codec::json!({"shell":o.shell,"expectedOutward":o.expected_outward,"outward":o.outward,"attempts":o.attempts.len()})).collect::<Vec<_>>()}));
        }
    }
    std::fs::write(output,value_codec::to_string_pretty(&value_codec::json!({"schema":"cad-annular-volume/1","scope":"Fresh original boundary, material roles and independent orientation; all stages required","maxLinearCells":100000,"cases":cases})).unwrap()).unwrap();
}
