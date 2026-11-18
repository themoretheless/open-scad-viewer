use brep_core::closed_solid as solid;
pub fn policy() -> solid::AdmissionPolicy {
    solid::AdmissionPolicy {
        tolerance_uv: 1e-8,
        max_linear_cells: 100000,
        geometry: brep_core::volume_validity::Limits {
            boundary: brep_core::boundary_embedding::Limits {
                exact_work: 1000000,
                trim_pairs: 1000,
                trim_cells: 10000,
                trim_domain_cells: 100000,
                spans: 100,
                contacts: brep_core::face_contacts::Limits {
                    pairs: 1000,
                    cells: 10000,
                    domain_cells: 100000,
                    cells_per_pair: 16,
                    domain_cells_per_pair: 1000,
                },
            },
            nesting_pairs: 100,
            nesting_cells: 10000,
            nesting_domain_cells: 100000,
            orientation_cells: 10000,
            orientation_domain_cells: 100000,
            orientation_spans: 100,
        },
    }
}
