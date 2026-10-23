use super::*;

#[test]
fn geometry_recording_keeps_evaluated_parameters_and_rejects_missing_operations() {
    let result = evaluate_source(
        "w=2; module part(x){translate([x,0,0])cube([w,3,4],center=true);}for(x=[1,5])part(x);",
        LanguageProfile::ViewerSubset,
        EvaluatorOptions {
            record_geometry: true,
            ..Default::default()
        },
    )
    .unwrap();
    let program = result.geometry.unwrap();
    program.validate().unwrap();
    assert_eq!(program.roots.len(), 2);
    assert_eq!(program.nodes.len(), 4);
    assert!(matches!(
        program.nodes[0],
        geometry_ops::solid_program::Node::Cube {
            size: [2., 3., 4.],
            center: true
        }
    ));
    assert!(
        evaluate_source(
            "hull()cube(1);",
            LanguageProfile::ViewerSubset,
            EvaluatorOptions {
                record_geometry: true,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn stable_cube_geometry_uses_shared_parameter_plan() {
    let options = || EvaluatorOptions {
        record_geometry: true,
        ..Default::default()
    };
    let result = evaluate_source(
        "cube([2,\"bad\",4],center=true);",
        LanguageProfile::Stable2021,
        options(),
    )
    .unwrap();
    let program = result.geometry.unwrap();
    assert!(matches!(
        program.nodes[0],
        geometry_ops::solid_program::Node::Cube {
            size: [2., 1., 1.],
            center: true
        }
    ));
    assert_eq!(result.warnings.len(), 1);
    let result = evaluate_source("cube(0);", LanguageProfile::Stable2021, options()).unwrap();
    assert!(matches!(
        result.geometry.unwrap().nodes[0],
        geometry_ops::solid_program::Node::Empty
    ));
}

#[test]
fn stable_difference_records_empty_base_and_cutters() {
    for source in [
        "difference(){cube(0);cube(1);}",
        "difference(){union(){}cube(1);}",
    ] {
        let result = evaluate_source(
            source,
            LanguageProfile::Stable2021,
            EvaluatorOptions {
                record_geometry: true,
                ..Default::default()
            },
        )
        .unwrap();
        let program = result.geometry.unwrap();
        assert!(matches!(
            program.nodes[program.roots[0]],
            geometry_ops::solid_program::Node::Boolean {
                operation: geometry_ops::solid_program::Boolean::Difference,
                ..
            }
        ));
    }
}

#[test]
fn stable_cylinder_geometry_uses_bound_values_and_shared_fragments() {
    let result = evaluate_source(
        "$fn=8;cylinder(h=2,r=9,d=2,r2=.5);",
        LanguageProfile::Stable2021,
        EvaluatorOptions {
            record_geometry: true,
            ..Default::default()
        },
    )
    .unwrap();
    let program = result.geometry.unwrap();
    assert!(matches!(
        program.nodes[0],
        geometry_ops::solid_program::Node::Cylinder {
            height: 2.,
            radii: [1., 0.5],
            segments: 8,
            center: false
        }
    ));
    assert_eq!(result.warnings.len(), 2);
}

#[test]
fn stable_square_extrusion_compiles_full_profile_graph() {
    for source in [
        "linear_extrude(3) square(2,true);",
        "linear_extrude(height=3,twist=360) difference(){square(4,true);square(2,true);}",
        "linear_extrude(3) translate([1,2]) scale([2,3,0]) square(2);",
        "linear_extrude(0) square(2);",
        "linear_extrude(3);",
    ] {
        let result = evaluate_source(
            source,
            STABLE,
            EvaluatorOptions {
                record_geometry: true,
                ..Default::default()
            },
        )
        .unwrap();
        let program = result.geometry.unwrap();
        assert!(program.validate().is_ok(), "{source}");
        assert_eq!(program.roots.len(), 1, "{source}");
    }
}

#[test]
fn stable_offset_records_join_and_fragment_selection() {
    for (source, join, segments) in [
        (
            "linear_extrude(2)offset(r=1,$fn=16)square(4);",
            geometry_ops::profile_program::OffsetJoin::Round,
            16,
        ),
        (
            "linear_extrude(2)offset(delta=1,chamfer=true,$fn=-2)square(4);",
            geometry_ops::profile_program::OffsetJoin::Square,
            8,
        ),
    ] {
        let result = evaluate_source(
            source,
            LanguageProfile::Stable2021,
            EvaluatorOptions {
                record_geometry: true,
                ..Default::default()
            },
        )
        .unwrap();
        let program = result.geometry.unwrap();
        program.validate().unwrap();
        let geometry_ops::solid_program::Node::ExtrudeProfile { profile, .. } =
            &program.nodes[program.roots[0]]
        else {
            panic!("Expected extrusion");
        };
        assert!(
            matches!(profile.nodes[profile.roots[0]], geometry_ops::profile_program::Node::Offset { input: 0, join: actual_join, segments: actual_segments, .. } if actual_join == join && actual_segments == segments)
        );
    }
}
