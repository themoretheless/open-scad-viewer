//! Execute typed solid expressions without depending on a language frontend or transport.
use super::{boolean, primitives};
use crate::{Mesh, Result, check};
use geometry_ops::solid_program::{Boolean, Node, Program};
/// Only reachable nodes run. Inputs are released after their last consumer.
pub fn execute(program: &Program, max_retained_triangles: usize) -> Result<Vec<Mesh>> {
    Ok(execute_report(program, max_retained_triangles)?.meshes)
}
#[derive(Debug)]
pub struct SliceReduction {
    pub node: usize,
    pub requested: f64,
    pub maximum: usize,
}
#[derive(Debug)]
pub struct SweepWarning {
    pub node: usize,
    pub maximum: usize,
    pub warning: geometry_ops::fragment_resolution::Warning,
}
#[derive(Debug)]
pub struct ExecutionReport {
    pub mesh_failures: Vec<usize>,
    pub resize_diagnostics: Vec<geometry_ops::resize::Diagnostic>,
    pub sweep_warnings: Vec<SweepWarning>,
    pub meshes: Vec<Mesh>,
    pub slice_reductions: Vec<SliceReduction>,
    pub profile_diagnostics: Vec<super::profile_program::Diagnostic>,
}
pub fn execute_report(program: &Program, max_retained_triangles: usize) -> Result<ExecutionReport> {
    program.validate()?;
    let mut mesh_failures = Vec::new();
    let mut sweep_warnings = Vec::new();
    let mut resize_diagnostics = Vec::new();
    let mut slice_reductions = Vec::new();
    let mut profile_diagnostics = Vec::new();
    let inputs = program.nodes.iter().map(Node::inputs).collect::<Vec<_>>();
    let geometry_ops::program_graph::ExecutionPlan {
        reachable,
        mut uses,
    } = geometry_ops::program_graph::execution_plan(&inputs, &program.roots)?;
    let mut results: Vec<Option<Mesh>> = vec![None; program.nodes.len()];
    let mut retained = 0;
    for (i, node) in program.nodes.iter().enumerate() {
        if !reachable[i] {
            continue;
        }
        let result = match node {
            Node::Empty => primitives::empty(),
            Node::Mesh {
                mesh,
                empty_on_failure,
            } => {
                let result = primitives::clean(mesh.clone().into()).and_then(|mesh| {
                    if *empty_on_failure {
                        check(
                            !mesh.indices.is_empty() && mesh.inspect()?.closed,
                            "Mesh topology did not produce a manifold solid",
                        )?;
                    }
                    Ok(mesh)
                });
                match result {
                    Ok(mesh) => mesh,
                    Err(_) if *empty_on_failure => {
                        mesh_failures.push(i);
                        primitives::empty()
                    }
                    Err(error) => return Err(error),
                }
            }
            Node::Cube { size, center } => primitives::cube(*size, *center)?,
            Node::Sphere { radius, segments } => primitives::sphere(*radius, *segments)?,
            Node::Cylinder {
                height,
                radii,
                segments,
                center,
            } => primitives::cylinder(*height, radii[0], radii[1], *segments, *center)?,
            Node::ExtrudeRings {
                rings,
                height,
                slices,
                twist,
                scale,
                center,
            } => super::modeling::extrude_rings(rings, *height, *slices, *twist, *scale, *center)?,
            Node::ExtrudeProfile {
                profile,
                slice_policy,
                height,
                slices,
                twist,
                scale,
                center,
            } => {
                let report = super::profile_program::execute_report(profile, 100_000)?;
                profile_diagnostics.extend(report.diagnostics);
                mesh_failures.extend(report.mesh_failures);
                resize_diagnostics.extend(report.resize_diagnostics);
                slice_reductions.extend(report.slice_reductions);
                sweep_warnings.extend(report.sweep_warnings);
                let profiles = report.profiles;
                let subdivisions = if let Some(policy) = slice_policy {
                    let points = profiles[0].iter().flatten().copied().collect::<Vec<_>>();
                    let resolution = policy.resolve(&points, *height, *twist, *scale)?;
                    if resolution.reduced {
                        slice_reductions.push(SliceReduction {
                            node: i,
                            requested: resolution.unbounded_slices,
                            maximum: policy.maximum,
                        });
                    }
                    resolution.slices
                } else {
                    *slices
                };
                super::modeling::extrude_rings(
                    &profiles[0],
                    *height,
                    subdivisions,
                    *twist,
                    *scale,
                    *center,
                )?
            }
            Node::RevolveProfile {
                fragment_policy,
                profile,
                angle,
                segments,
            } => {
                let report = super::profile_program::execute_report(profile, 100_000)?;
                profile_diagnostics.extend(report.diagnostics);
                mesh_failures.extend(report.mesh_failures);
                resize_diagnostics.extend(report.resize_diagnostics);
                slice_reductions.extend(report.slice_reductions);
                sweep_warnings.extend(report.sweep_warnings);
                let mut rings = report.profiles.into_iter().next().unwrap();
                let minimum = rings
                    .iter()
                    .flatten()
                    .map(|p| p[0])
                    .fold(f64::INFINITY, f64::min);
                let maximum = rings
                    .iter()
                    .flatten()
                    .map(|p| p[0])
                    .fold(f64::NEG_INFINITY, f64::max);
                let side = geometry_ops::revolution::profile_side(minimum, maximum);
                check(
                    side != "crossing",
                    "rotate_extrude() profile may not cross the Y axis",
                )?;
                if rings.is_empty() || side == "axis" {
                    primitives::empty()
                } else {
                    if side == "negative" {
                        for ring in &mut rings {
                            for point in ring.iter_mut() {
                                point[0] = -point[0];
                            }
                            ring.reverse();
                        }
                    }
                    let subdivisions = if let Some(policy) = fragment_policy {
                        let radius = rings
                            .iter()
                            .flatten()
                            .map(|p| p[0].abs())
                            .fold(0_f64, f64::max);
                        let resolution = policy.resolve(radius, *angle)?;
                        sweep_warnings.extend(resolution.circle.warnings.into_iter().map(
                            |warning| SweepWarning {
                                node: i,
                                maximum: policy.maximum,
                                warning,
                            },
                        ));
                        resolution.segments
                    } else {
                        *segments
                    };
                    let mesh = super::modeling::revolve_rings(&rings, *angle, subdivisions)?;
                    if side == "negative" {
                        mesh.transform([
                            [-1., 0., 0., 0.],
                            [0., -1., 0., 0.],
                            [0., 0., 1., 0.],
                            [0., 0., 0., 1.],
                        ])?
                    } else {
                        mesh
                    }
                }
            }
            Node::Resize {
                invalid_newsize,
                input,
                targets,
                automatic,
            } => {
                let mesh = results[*input].as_ref().unwrap();
                if let Some((min, max)) = boolean::bounds(mesh) {
                    if *invalid_newsize {
                        resize_diagnostics.push(geometry_ops::resize::Diagnostic {
                            node: i,
                            axis: None,
                        });
                    }
                    let selected = geometry_ops::resize::resolve(
                        *targets,
                        std::array::from_fn(|axis| Some(max[axis] - min[axis])),
                        *automatic,
                    );
                    if let Some(axis) = selected.invalid_axis {
                        resize_diagnostics.push(geometry_ops::resize::Diagnostic {
                            node: i,
                            axis: Some(axis),
                        });
                    }
                    mesh.transform(geometry_ops::solid_program::affine::scaling(
                        selected.scales,
                    ))?
                } else {
                    mesh.clone()
                }
            }
            Node::Translate { input, delta } => results[*input].as_ref().unwrap().transform([
                [1., 0., 0., delta[0]],
                [0., 1., 0., delta[1]],
                [0., 0., 1., delta[2]],
                [0., 0., 0., 1.],
            ])?,
            Node::Transform { input, matrix } => {
                let mesh = results[*input].as_ref().unwrap();
                if matrix[3] == [0., 0., 0., 1.] {
                    mesh.transform(*matrix)?
                } else {
                    mesh.transform_projective(*matrix)?
                }
            }
            Node::Minkowski { inputs } => {
                let mut result = inputs.first().map_or_else(primitives::empty, |&input| {
                    results[input].as_ref().unwrap().clone()
                });
                for &input in inputs.iter().skip(1) {
                    result = primitives::minkowski(&result, results[input].as_ref().unwrap())?;
                }
                result
            }
            Node::Hull { inputs } => primitives::hull3_refs(
                &inputs
                    .iter()
                    .map(|&input| results[input].as_ref().unwrap())
                    .collect::<Vec<_>>(),
            )?,
            Node::Boolean { operation, inputs } => {
                let mut result = inputs.first().map_or_else(primitives::empty, |&id| {
                    results[id].as_ref().unwrap().clone()
                });
                let operation = match operation {
                    Boolean::Union => boolean::Operation::Union,
                    Boolean::Intersection => boolean::Operation::Intersection,
                    Boolean::Difference => boolean::Operation::Difference,
                };
                for &id in inputs.iter().skip(1) {
                    result = boolean::boolean(
                        &result,
                        results[id].as_ref().unwrap(),
                        operation,
                        &boolean::Options::default(),
                    )?
                    .mesh;
                }
                result
            }
        };
        result.validate()?;
        retained += result.indices.len() / 3;
        check(
            retained <= max_retained_triangles,
            "Native solid execution retained triangle budget exceeded",
        )?;
        results[i] = Some(result);
        for &input in node.inputs() {
            uses[input] -= 1;
            if uses[input] == 0 {
                retained -= results[input].take().unwrap().indices.len() / 3;
            }
        }
    }
    let mut roots = Vec::with_capacity(program.roots.len());
    for &id in &program.roots {
        uses[id] -= 1;
        let mesh = if uses[id] == 0 {
            results[id].take().unwrap()
        } else {
            results[id].as_ref().unwrap().clone()
        };
        if !mesh.indices.is_empty() {
            roots.push(mesh);
        }
    }
    Ok(ExecutionReport {
        mesh_failures,
        resize_diagnostics,
        sweep_warnings,
        meshes: roots,
        slice_reductions,
        profile_diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn releases_inputs_and_skips_dead_geometry() {
        let p = Program {
            nodes: vec![
                Node::Cube {
                    size: [f64::NAN; 3],
                    center: false,
                },
                Node::Cube {
                    size: [1.; 3],
                    center: false,
                },
                Node::Translate {
                    input: 1,
                    delta: [2., 0., 0.],
                },
                Node::Translate {
                    input: 2,
                    delta: [3., 0., 0.],
                },
            ],
            roots: vec![3],
        };
        let meshes = execute(&p, 24).unwrap();
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].positions[0], 5.);
        assert!(execute(&p, 23).is_err());
    }
}

#[cfg(test)]
mod empty_tests {
    use super::*;
    #[test]
    fn explicit_empty_operands_keep_boolean_order() {
        for operation in [Boolean::Union, Boolean::Intersection, Boolean::Difference] {
            for inputs in [vec![0, 1], vec![1, 0]] {
                let empty = operation == Boolean::Intersection
                    || (operation == Boolean::Difference && inputs[0] == 0);
                let p = Program {
                    nodes: vec![
                        Node::Empty,
                        Node::Cube {
                            size: [1.; 3],
                            center: false,
                        },
                        Node::Boolean { operation, inputs },
                    ],
                    roots: vec![2],
                };
                let meshes = execute(&p, 24).unwrap();
                assert_eq!(meshes.is_empty(), empty);
                if !empty {
                    assert_eq!(meshes[0].indices.len(), 36);
                }
            }
        }
    }
}

#[cfg(test)]
mod projective_tests {
    use super::*;
    #[test]
    fn executes_full_matrix_and_refuses_crossing_horizon() {
        let mut matrix = math_core::affine::IDENTITY;
        matrix[3][0] = 0.25;
        let mut program = Program {
            nodes: vec![
                Node::Cube {
                    size: [2.; 3],
                    center: true,
                },
                Node::Transform { input: 0, matrix },
            ],
            roots: vec![1],
        };
        let meshes = execute(&program, 100).unwrap();
        assert!(
            (meshes[0]
                .positions
                .chunks_exact(3)
                .map(|p| p[0])
                .fold(f64::NEG_INFINITY, f64::max)
                - 0.8)
                .abs()
                < 1e-12
        );
        assert!(meshes[0].inspect().unwrap().signed_volume_mm3 > 0.);
        matrix[3][0] = 2.;
        program.nodes[1] = Node::Transform { input: 0, matrix };
        assert!(execute(&program, 100).is_err());
    }
}

#[cfg(test)]
mod extrusion_tests {
    use super::*;
    #[test]
    fn executes_centered_extrusion_with_a_hole() {
        let program = Program {
            nodes: vec![Node::ExtrudeRings {
                rings: vec![
                    vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]],
                    vec![[1., 1.], [1., 3.], [3., 3.], [3., 1.]],
                ],
                height: 3.,
                slices: 1,
                twist: 0.,
                scale: [1.; 2],
                center: true,
            }],
            roots: vec![0],
        };
        let meshes = execute(&program, 100).unwrap();
        assert!((meshes[0].inspect().unwrap().signed_volume_mm3 - 36.).abs() < 1e-10);
        assert_eq!(
            meshes[0]
                .positions
                .chunks_exact(3)
                .map(|p| p[2])
                .fold(f64::INFINITY, f64::min),
            -1.5
        );
    }
}

#[cfg(test)]
mod nested_profile_tests {
    use super::*;
    use geometry_ops::profile_program::{Node as ProfileNode, Program as ProfileProgram};
    #[test]
    fn executes_revolution_profile_and_releases_unreachable_nodes() {
        let profile = ProfileProgram {
            nodes: vec![ProfileNode::Rings(vec![
                vec![[1., 0.], [4., 0.], [4., 3.], [1., 3.]],
                vec![[2., 1.], [2., 2.], [3., 2.], [3., 1.]],
            ])],
            roots: vec![0],
        };
        let mut program = Program {
            nodes: vec![Node::RevolveProfile {
                fragment_policy: None,
                profile,
                angle: -90.,
                segments: 16,
            }],
            roots: vec![0],
        };
        let report = execute_report(&program, 1000).unwrap();
        assert!(report.meshes[0].inspect().unwrap().closed);
        let expected = 40. * 8. * (90_f64 / 16.).to_radians().sin();
        assert!((report.meshes[0].inspect().unwrap().signed_volume_mm3 - expected).abs() < 1e-8);
        assert!(execute(&program, 1).is_err());
        if let Node::RevolveProfile {
            fragment_policy,
            segments,
            ..
        } = &mut program.nodes[0]
        {
            *segments = 0;
            *fragment_policy = Some(geometry_ops::fragment_resolution::Policy {
                fragments: [100., 12., 2.],
                maximum: 16,
            });
        }
        let report = execute_report(&program, 1000).unwrap();
        assert_eq!(report.sweep_warnings.len(), 1);
        assert_eq!(report.sweep_warnings[0].node, 0);
        assert!(matches!(
            report.sweep_warnings[0].warning.kind,
            geometry_ops::fragment_resolution::WarningKind::Clamped
        ));
        let expected = 40. * 2. * (90_f64 / 4.).to_radians().sin();
        assert!((report.meshes[0].inspect().unwrap().signed_volume_mm3 - expected).abs() < 1e-8);
        program.roots.clear();
        assert!(
            execute_report(&program, 1)
                .unwrap()
                .sweep_warnings
                .is_empty()
        );
        assert!(execute(&program, 1).unwrap().is_empty());
    }
    #[test]
    fn executes_profile_boolean_inside_solid_program() {
        let profile = ProfileProgram {
            nodes: vec![
                ProfileNode::Rectangle {
                    size: [4.; 2],
                    center: true,
                },
                ProfileNode::Rectangle {
                    size: [2.; 2],
                    center: true,
                },
                ProfileNode::Boolean {
                    operation: Boolean::Difference,
                    inputs: vec![0, 1],
                },
            ],
            roots: vec![2],
        };
        let program = Program {
            nodes: vec![Node::ExtrudeProfile {
                profile,
                slice_policy: None,
                height: 3.,
                slices: 1,
                twist: 0.,
                scale: [1.; 2],
                center: true,
            }],
            roots: vec![0],
        };
        let mesh = execute(&program, 100).unwrap().remove(0);
        assert!((mesh.inspect().unwrap().signed_volume_mm3 - 36.).abs() < 1e-10);
    }
}

#[cfg(test)]
mod automatic_profile_tests {
    use super::*;
    #[test]
    fn deferred_twist_subdivision_matches_direct_mesh_construction() {
        use geometry_ops::{
            extrude_slices::Policy,
            profile_program::{Node as PNode, Program as PProgram},
        };
        let profile = PProgram {
            nodes: vec![PNode::Rectangle {
                size: [2.; 2],
                center: true,
            }],
            roots: vec![0],
        };
        let program = Program {
            nodes: vec![Node::ExtrudeProfile {
                profile,
                slice_policy: Some(Policy {
                    explicit: None,
                    fragments: [12., 12., 2.],
                    maximum: 512,
                }),
                height: 3.,
                slices: 0,
                twist: 360.,
                scale: [1.; 2],
                center: false,
            }],
            roots: vec![0],
        };
        let actual = execute(&program, 1000).unwrap().remove(0);
        let rings = planar_geometry::primitives::rectangle([2.; 2], true).unwrap();
        let expected =
            super::super::modeling::extrude_rings(&rings, 3., 12, 360., [1.; 2], false).unwrap();
        assert_eq!(actual.positions, expected.positions);
        assert_eq!(actual.indices, expected.indices);
        assert_eq!(actual.uv, expected.uv);
        let mut limited = program.clone();
        if let Node::ExtrudeProfile {
            slice_policy: Some(policy),
            ..
        } = &mut limited.nodes[0]
        {
            policy.maximum = 6;
        }
        let report = execute_report(&limited, 1000).unwrap();
        assert_eq!(report.slice_reductions.len(), 1);
        assert_eq!(report.slice_reductions[0].node, 0);
        assert_eq!(report.slice_reductions[0].requested, 12.);
        assert_eq!(report.slice_reductions[0].maximum, 6);
        limited.roots.clear();
        assert!(
            execute_report(&limited, 1000)
                .unwrap()
                .slice_reductions
                .is_empty()
        );
    }
}

#[cfg(test)]
mod hull_program_tests {
    use super::*;
    #[test]
    fn minkowski_sums_convex_boxes_and_preserves_empty_operands() {
        let mut program = Program {
            nodes: vec![
                Node::Cube {
                    size: [1.; 3],
                    center: false,
                },
                Node::Cube {
                    size: [2.; 3],
                    center: false,
                },
                Node::Minkowski { inputs: vec![0, 1] },
            ],
            roots: vec![2],
        };
        let meshes = execute(&program, 200).unwrap();
        let report = meshes[0].inspect().unwrap();
        assert!(report.closed);
        assert!((report.signed_volume_mm3 - 27.).abs() < 1e-10);
        assert!(execute(&program, 1).is_err());
        program.nodes[0] = Node::Empty;
        assert!(execute(&program, 200).unwrap().is_empty());
    }
    #[test]
    fn hull_retains_inputs_until_execution_and_obeys_triangle_budget() {
        let program = Program {
            nodes: vec![
                Node::Cube {
                    size: [1.; 3],
                    center: false,
                },
                Node::Translate {
                    input: 0,
                    delta: [3., 0., 0.],
                },
                Node::Hull {
                    inputs: vec![0, 1, 0],
                },
            ],
            roots: vec![2],
        };
        let mesh = execute(&program, 100).unwrap().remove(0);
        let report = mesh.inspect().unwrap();
        assert!(report.closed);
        assert!((report.signed_volume_mm3 - 4.).abs() < 1e-10);
        assert!(execute(&program, 1).is_err());
    }
}

#[cfg(test)]
mod resize_tests {
    use super::*;
    #[test]
    fn deferred_resize_scales_group_bounds_about_the_origin() {
        let program = Program {
            nodes: vec![
                Node::Cube {
                    size: [5., 3., 2.],
                    center: false,
                },
                Node::Translate {
                    input: 0,
                    delta: [2., 0., 0.],
                },
                Node::Resize {
                    invalid_newsize: false,
                    input: 1,
                    targets: [Some(10.), None, None],
                    automatic: [false, true, false],
                },
            ],
            roots: vec![2],
        };
        let report = execute_report(&program, 100).unwrap();
        let (min, max) = boolean::bounds(&report.meshes[0]).unwrap();
        assert_eq!(min, [4., 0., 0.]);
        assert_eq!(max, [14., 6., 2.]);
        assert!(report.resize_diagnostics.is_empty());
        assert!(execute(&program, 1).is_err());
    }
}

#[cfg(test)]
mod mesh_tests {
    use super::*;
    #[test]
    fn explicit_meshes_reuse_cleaning_and_existing_transform_execution() {
        let cube = primitives::cube([1.; 3], false).unwrap();
        let program = Program {
            nodes: vec![
                Node::Mesh {
                    mesh: geometry_ops::Triangles {
                        positions: cube.positions,
                        indices: cube.indices,
                    },
                    empty_on_failure: false,
                },
                Node::Translate {
                    input: 0,
                    delta: [2., 3., 4.],
                },
            ],
            roots: vec![1],
        };
        let meshes = execute(&program, 100).unwrap();
        assert_eq!(
            boolean::bounds(&meshes[0]),
            Some(([2., 3., 4.], [3., 4., 5.]))
        );
        let report = meshes[0].inspect().unwrap();
        assert!(report.closed);
        assert!((report.signed_volume_mm3 - 1.).abs() < 1e-8);
        assert!(execute(&program, 1).is_err());
    }
}

#[cfg(test)]
mod soft_mesh_tests {
    use super::*;
    #[test]
    fn degenerate_soft_mesh_reports_failure_and_allows_other_roots() {
        let program = Program {
            nodes: vec![
                Node::Mesh {
                    mesh: geometry_ops::Triangles {
                        positions: vec![0., 0., 0., 1., 0., 0., 2., 0., 0.],
                        indices: vec![0, 1, 2],
                    },
                    empty_on_failure: true,
                },
                Node::Cube {
                    size: [1.; 3],
                    center: false,
                },
            ],
            roots: vec![0, 1],
        };
        let report = execute_report(&program, 100).unwrap();
        assert_eq!(report.mesh_failures, vec![0]);
        assert_eq!(report.meshes.len(), 1);
        assert!(report.meshes[0].inspect().unwrap().closed);
        let open=Program {nodes:vec![Node::Mesh {
            mesh:geometry_ops::Triangles {positions:vec![0.,0.,0.,1.,0.,0.,0.,1.,0.],indices:vec![0,1,2]},empty_on_failure:true,
        }],roots:vec![0]};
        let report=execute_report(&open,100).unwrap();
        assert_eq!(report.mesh_failures,vec![0]);
        assert!(report.meshes.is_empty());
    }
}
