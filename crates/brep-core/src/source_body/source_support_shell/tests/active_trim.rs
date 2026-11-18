use super::*;
use brep_topology::CoedgeTrim;

fn extend_uv(model: &mut crate::Model, descending: bool) {
    let wire = model.faces[0].outer;
    let coedge = &mut model.0.loops[wire].coedges[0];
    let a = &coedge.pcurve.control_points[0];
    let b = &coedge.pcurve.control_points[1];
    let mut ends = [
        a.iter().zip(b).map(|(a, b)| 2. * a - b).collect::<Vec<_>>(),
        a.iter().zip(b).map(|(a, b)| 3. * b - 2. * a).collect(),
    ];
    let mut range = [0.25, 0.5];
    if descending {
        ends.reverse();
        range = [0.75, 0.5];
    }
    *coedge = crate::Coedge::new(
        coedge.edge,
        coedge.reversed,
        crate::line(ends[0].clone(), ends[1].clone()),
    )
    .with_trim(CoedgeTrim {
        curve_parameter: model.0.edges[coedge.edge].curve.domain(),
        pcurve_parameter: range,
        periodic_lift: [[0; 2]; 2],
    })
    .unwrap();
}

#[test]
fn active_trim_owns_complete_material_with_independent_uv_clocks() {
    let box_ = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let reflected = crate::transform::affine(
        &box_,
        [
            [-1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    for (mut model, descending) in [(box_, false), (reflected, true)] {
        extend_uv(&mut model, descending);
        let before = model.clone();
        let report = prepare_projective(&model, limits()).unwrap();
        let spent = report.work_used;
        let shell = report.shell.expect(report.reason);
        let g = crate::source_shell_geometry::qualify_projective(shell, geometry_limits()).unwrap();
        let report = crate::source_volume::qualify(
            g.geometry.expect(g.reason),
            crate::source_volume::Limits {
                axis: 0,
                origin: 0.,
                absolute_error: 0.01,
                tolerance_uv: 1e-8,
                cells: 10000,
                spans: 100000,
                domain_cells: 100000,
            },
        )
        .unwrap();
        let volume = report.body.expect(report.reason).volume();
        assert!(volume[0] <= 1. && 1. <= volume[1]);
        assert_eq!(model, before);
        assert!(
            prepare_projective(
                &model,
                Limits {
                    exact_work: spent,
                    ..limits()
                }
            )
            .unwrap()
            .shell
            .is_some()
        );
        let short = prepare_projective(
            &model,
            Limits {
                exact_work: spent - 1,
                ..limits()
            },
        );
        assert!(short.is_err() || short.unwrap().shell.is_none());
    }
}

#[test]
fn active_trim_refuses_diagnostic_world_ranges_and_incorrect_lift_metadata() {
    let original = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    for (world, lift) in [([0.25, 0.75], [[0; 2]; 2]), ([0., 1.], [[1, 0], [0, 0]])] {
        let mut model = original.clone();
        let wire = model.faces[0].outer;
        let coedge = &mut model.loops[wire].coedges[0];
        *coedge = coedge
            .clone()
            .with_trim(CoedgeTrim {
                curve_parameter: world,
                pcurve_parameter: [0., 1.],
                periodic_lift: lift,
            })
            .unwrap();
        model.validate().unwrap();
        let Err(error) = prepare_projective(&model, limits()) else {
            panic!("wrong trim returned a source report");
        };
        assert_eq!(error.code, "BREP_SOURCE_SUPPORT_TRIM");
    }
}
