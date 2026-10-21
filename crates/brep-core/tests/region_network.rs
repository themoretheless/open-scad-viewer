use brep_core::region_replacement::{self, RegionPatch};
#[path = "../examples/support/region_network_fixture.rs"]
mod fixture;
#[test]
fn prepared_corner_sectors_replace_atomically_and_share_the_saved_diagonal() {
    let f = fixture::build();
    let source = &f.source;
    let before = source.clone();
    let chart = &f.chart;
    let loops = &f.loops;
    let selected = &f.selected;
    let reversed = f.reversed;
    let plans = f.plans();
    let out = region_replacement::replace_g2_network(
        &source,
        &plans,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    assert!(out.seams.complete && out.seams.within_tolerance);
    assert!(out.interiors.iter().all(|p| p.certified));
    assert!(out.trims.all_valid);
    assert_eq!(
        out.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert_eq!(out.patch_faces.len(), 2);
    assert_eq!(out.model.bodies.len(), 1);
    assert!(out.cells <= 100000 && out.exact_work <= 1000000);
    #[cfg(feature = "codec")]
    {
        let reloaded: brep_core::Model =
            value_codec::from_str(&value_codec::to_string(&out.model).unwrap()).unwrap();
        assert_eq!(reloaded, out.model);
        let closure =
            brep_core::watertight::inspect_with_exact(&reloaded, 100000, 1000000).unwrap();
        assert_eq!(
            closure.closure.status,
            brep_core::watertight::Closure::WithinTolerance
        );
        let ids = out.seams.seams.iter().map(|s| s.edge).collect::<Vec<_>>();
        let seams = brep_core::sweep_smoothness::geometric::inspect_edges_with_exact(
            &reloaded,
            &ids,
            brep_core::sweep_smoothness::geometric::Tolerances {
                position: 1e-7,
                normal: 1e-5,
                curvature: 1e-4,
            },
            100000,
            1000000,
        )
        .unwrap();
        assert!(seams.complete && seams.within_tolerance);
    }
    let shared = out.source_edges[f.cut_edges[0]].unwrap();
    assert_eq!(
        out.seams.seams.iter().filter(|s| s.edge == shared).count(),
        1
    );
    assert!(
        out.seams
            .seams
            .iter()
            .find(|s| s.edge == shared)
            .unwrap()
            .faces
            .iter()
            .all(|f| out.patch_faces.contains(f))
    );
    for (i, mapped) in out.source_edges.iter().enumerate() {
        if let Some(j) = mapped {
            assert_eq!(source.1.edges[i], out.model.1.edges[*j]);
        }
    }
    for (i, mapped) in out.source_vertices.iter().enumerate() {
        if let Some(j) = mapped {
            assert_eq!(source.1.vertices[i], out.model.1.vertices[*j]);
        }
    }
    let mut bad = chart.clone();
    for p in bad.control_points.iter_mut().flatten() {
        p[2] += 0.01;
    }
    let invalid = [
        RegionPatch {
            selected: &selected[0],
            surface: &chart,
            loops: &loops[0],
            reversed: reversed[0],
        },
        RegionPatch {
            selected: &selected[1],
            surface: &bad,
            loops: &loops[1],
            reversed: reversed[1],
        },
    ];
    assert!(
        region_replacement::replace_g2_network(
            &source,
            &invalid,
            1e-8,
            [1e-7, 1e-5, 1e-4],
            100000,
            1000000
        )
        .is_err()
    );
    // Preserve every outer rim and the diagonal, but change transverse jets.
    // Positional sewing alone must not publish the partially replaced network.
    assert_eq!((chart.degree_u, chart.degree_v), (5, 5));
    assert!(chart.weights.iter().flatten().all(|w| *w == 1.));
    let mut kink = chart.clone();
    let choose = |n: usize, k: usize| {
        if k > n {
            return 0.;
        }
        let mut v = 1.;
        for j in 0..k {
            v *= (n - j) as f64 / (j + 1) as f64;
        }
        v
    };
    let terms = [
        (2, 1, 1.),
        (3, 1, -1.),
        (3, 2, 1.),
        (1, 2, -1.),
        (1, 3, 1.),
        (2, 3, -1.),
    ];
    for i in 0..6 {
        for j in 0..6 {
            let f = terms
                .iter()
                .map(|&(a, b, sign)| {
                    sign * choose(i, a) / choose(5, a) * choose(j, b) / choose(5, b)
                })
                .sum::<f64>();
            kink.control_points[i][j][2] += 0.1 * f;
        }
    }
    let kinked = [
        RegionPatch {
            selected: &selected[0],
            surface: &chart,
            loops: &loops[0],
            reversed: reversed[0],
        },
        RegionPatch {
            selected: &selected[1],
            surface: &kink,
            loops: &loops[1],
            reversed: reversed[1],
        },
    ];
    let error = region_replacement::replace_g2_network(
        &source,
        &kinked,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .err()
    .unwrap();
    assert!(
        error.message.contains("Final saved network seams"),
        "{}",
        error.message
    );
    let overlap = [
        RegionPatch {
            selected: &selected[0],
            surface: &chart,
            loops: &loops[0],
            reversed: reversed[0],
        },
        RegionPatch {
            selected: &selected[0],
            surface: &chart,
            loops: &loops[0],
            reversed: reversed[0],
        },
    ];
    assert!(
        region_replacement::replace_g2_network(
            &source,
            &overlap,
            1e-8,
            [1e-7, 1e-5, 1e-4],
            100000,
            1000000
        )
        .is_err()
    );
    assert!(
        region_replacement::replace_g2_network(&source, &plans, 1e-8, [1e-7, 1e-5, 1e-4], 4, 0)
            .is_err()
    );
    assert_eq!(source, &before);
}
