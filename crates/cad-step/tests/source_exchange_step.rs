//! AP242 candidates for original source bodies: the annular pole fixture
//! through the full support/geometry/volume pipeline, and the curved
//! tetrahedron body restored from its saved definition.
use brep_core::{source_exchange_endpoints, source_support_shell};

fn limits() -> source_support_shell::Limits {
    source_support_shell::Limits {
        tolerance_uv: 1e-8,
        faces: 4096,
        uses: 100000,
        regions: brep_core::trimmed_face_recipe::Limits {
            pairs: 10000,
            region_cells: 10000,
            domain_cells: 10000,
            agreement_cells: 10000,
        },
        exact_work: 100_000_000,
    }
}
fn geometry_limits() -> brep_core::source_shell_geometry::Limits {
    brep_core::source_shell_geometry::Limits {
        tolerance_uv: 1e-8,
        corners: 10000,
        spans: 100000,
        linear_cells: 100000,
        exact_work: 100_000_000,
        driver_cells: 100000,
        pairs: brep_core::face_contacts::Limits {
            pairs: 10000,
            cells: 10000,
            domain_cells: 100000,
            cells_per_pair: 32,
            domain_cells_per_pair: 512,
        },
    }
}
fn replay_body_limits() -> brep_core::source_body_restore::Limits {
    brep_core::source_body_restore::Limits {
        shell: brep_core::source_shell_restore::Limits {
            regions: brep_core::source_region_restore::Limits {
                region: brep_core::trimmed_face_recipe::Limits {
                    pairs: 1000,
                    region_cells: 10000,
                    domain_cells: 10000,
                    agreement_cells: 10000,
                },
                search_cells: 10000,
                point_checks: 10000,
                mapping_cells: 10000,
                driver_cells: 10000,
                membership_cells: 10000,
                controls: 10000,
                winding_cells: 10000,
                steps: 64,
            },
            exact_work: 100_000_000,
            driver_cells: 10000,
        },
        embedding: brep_core::source_shell_geometry::Limits {
            tolerance_uv: 1e-8,
            corners: 14,
            spans: 1000,
            linear_cells: 1000,
            pairs: brep_core::face_contacts::Limits {
                pairs: 6,
                cells: 10000,
                domain_cells: 10000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 1000,
            },
            exact_work: 100_000_000,
            driver_cells: 10000,
        },
        volume: brep_core::source_volume::Limits {
            axis: 2,
            origin: 0.,
            absolute_error: 0.02,
            tolerance_uv: 1e-8,
            cells: 10000,
            spans: 100000,
            domain_cells: 1000000,
        },
    }
}

#[test]
fn original_annular_poles_are_preserved_in_step_exchange() {
    let model =
        brep_core::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
    let shell = source_support_shell::prepare(&model, limits())
        .unwrap()
        .shell
        .unwrap();
    let geometry = brep_core::source_shell_geometry::qualify(shell, geometry_limits())
        .unwrap()
        .geometry
        .unwrap();
    let volume = brep_core::source_volume::qualify(
        geometry,
        brep_core::source_volume::Limits {
            axis: 2,
            origin: 0.,
            absolute_error: 20.,
            tolerance_uv: 1e-8,
            cells: 100000,
            spans: 100000,
            domain_cells: 1000000,
        },
    )
    .unwrap();
    let body = volume.body.unwrap();
    let endpoint_limits = || brep_core::source_exchange_endpoints::Limits {
        root_checks: 100000,
        mapping_cells: 100000,
        replay_mapping_per_use: 10000,
        exact_work: 100_000_000,
        driver_cells: 100000,
        spans: 100000,
        endpoints: 100000,
    };
    assert!(
        cad_step::source_exchange_step::prepare(&body, 1e-7, endpoint_limits(), 1)
            .unwrap()
            .is_none()
    );
    assert!(cad_step::source_exchange_step::prepare(&body, 1e-7, endpoint_limits(), 0).is_err());
    assert!(
        cad_step::source_exchange_step::prepare(&body, 1e-7, endpoint_limits(), 10_000_001)
            .is_err()
    );
    let candidate = cad_step::source_exchange_step::prepare(&body, 1e-7, endpoint_limits(), 100000)
        .unwrap()
        .expect("original poles must export");
    assert_eq!(candidate.faces, 27);
    assert_eq!(candidate.edges, body.geometry().shell().edges().len() + 2);
    assert!(candidate.endpoint_error_upper <= 1e-7);
    eprintln!(
        "original annular STEP faces={} edges={} vertices={} bound={}",
        candidate.faces, candidate.edges, candidate.vertices, candidate.endpoint_error_upper
    );
    if let Some(path) = std::env::var_os("CAD_ANNULAR_SOURCE_STEP_OUTPUT") {
        std::fs::write(path, candidate.text).unwrap();
    }
}

/// The curved tetrahedron body from `source_shell_incidence`'s inverse-shear
/// replay test; regenerate the definition with
/// `CAD_SOURCE_CURVED_DEFINITION_OUTPUT=<absolute path> cargo test -p brep-core --lib curved_shell_preserves_roots`
/// (from `crates/`; the test binary runs with the crate directory as cwd).
#[test]
fn curved_tetrahedron_body_exports_a_bounded_candidate() {
    let definition: value_codec::Value = value_codec::from_str(include_str!(
        "../../../tests/fixtures/cad-step/curved-tetrahedron-body.definition.json"
    ))
    .unwrap();
    let restored =
        brep_core::source_body_restore::restore(definition, replay_body_limits()).unwrap();
    let body = restored.body().expect(restored.reason());
    let step = cad_step::source_exchange_step::prepare(
        body,
        1e-7,
        source_exchange_endpoints::Limits {
            root_checks: 1000,
            mapping_cells: 10000,
            replay_mapping_per_use: 10000,
            exact_work: 100_000_000,
            driver_cells: 10000,
            spans: 14,
            endpoints: 14,
        },
        100000,
    )
    .unwrap()
    .expect("source STEP candidate");
    assert_eq!((step.vertices, step.edges, step.faces), (5, 7, 4));
    assert!(step.endpoint_error_upper <= 1e-7);
    if let Ok(path) = std::env::var("CAD_SOURCE_CURVED_STEP_OUTPUT") {
        std::fs::write(path, step.text).unwrap();
    }
}
