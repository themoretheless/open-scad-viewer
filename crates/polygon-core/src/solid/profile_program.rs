//! Execute neutral planar programs with the existing planar kernel.
use crate::{Result, check};
use geometry_ops::{
    profile_program::{Node, Program},
    solid_program::Boolean,
};
use planar_geometry::{
    primitives,
    rings::{self, Rings},
};
/// Evaluate reachable nodes only and release intermediate contours at last use.
pub fn execute(program: &Program, max_retained_points: usize) -> Result<Vec<Rings>> {
    Ok(execute_report(program, max_retained_points)?.profiles)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issue {
    EmptyContours,
    InvalidContours,
}
#[derive(Debug)]
pub struct Diagnostic {
    pub node: usize,
    pub issue: Issue,
}
#[derive(Debug)]
pub struct ExecutionReport {
    pub mesh_failures: Vec<usize>,
    pub resize_diagnostics: Vec<geometry_ops::resize::Diagnostic>,
    pub slice_reductions: Vec<super::program::SliceReduction>,
    pub sweep_warnings: Vec<super::program::SweepWarning>,
    pub profiles: Vec<Rings>,
    pub diagnostics: Vec<Diagnostic>,
}
pub fn execute_report(program: &Program, max_retained_points: usize) -> Result<ExecutionReport> {
    program.validate()?;
    let mut mesh_failures = Vec::new();
    let mut diagnostics = Vec::new();
    let mut slice_reductions = Vec::new();
    let mut sweep_warnings = Vec::new();
    let mut resize_diagnostics = Vec::new();
    let inputs = program.nodes.iter().map(Node::inputs).collect::<Vec<_>>();
    let geometry_ops::program_graph::ExecutionPlan {
        reachable,
        mut uses,
    } = geometry_ops::program_graph::execution_plan(&inputs, &program.roots)?;
    let mut results: Vec<Option<Rings>> = vec![None; program.nodes.len()];
    let mut retained = 0usize;
    for (index, node) in program.nodes.iter().enumerate() {
        if !reachable[index] {
            continue;
        }
        let result = match node {
            Node::Empty => vec![],
            Node::Projection { solid, cut } => {
                let report = super::program::execute_report(solid, 200_000)?;
                diagnostics.extend(report.profile_diagnostics);
                mesh_failures.extend(report.mesh_failures);
                resize_diagnostics.extend(report.resize_diagnostics);
                slice_reductions.extend(report.slice_reductions);
                sweep_warnings.extend(report.sweep_warnings);
                let meshes = report.meshes;
                if let Some(mesh) = meshes.first() {
                    if *cut {
                        mesh_section::slice(&mesh.view(), 0.)
                    } else {
                        mesh_section::project(&mesh.view())
                    }
                    .map_err(crate::mesh_error)?
                } else {
                    vec![]
                }
            }
            Node::Rectangle { size, center } => primitives::rectangle(*size, *center)?,
            Node::Circle { radius, segments } => primitives::circle(*radius, *segments as f64)?,
            Node::Rings(value) => rings::nonzero(value)?,
            Node::Resize {
                invalid_newsize,
                input,
                targets,
                automatic,
            } => {
                let mut rings = results[*input].as_ref().unwrap().clone();
                if !rings.is_empty() {
                    if *invalid_newsize {
                        resize_diagnostics.push(geometry_ops::resize::Diagnostic {
                            node: index,
                            axis: None,
                        });
                    }
                    let mut min = [f64::INFINITY; 2];
                    let mut max = [f64::NEG_INFINITY; 2];
                    for point in rings.iter().flatten() {
                        for axis in 0..2 {
                            min[axis] = min[axis].min(point[axis]);
                            max[axis] = max[axis].max(point[axis]);
                        }
                    }
                    let selected = geometry_ops::resize::resolve(
                        *targets,
                        std::array::from_fn(|axis| Some(max[axis] - min[axis])),
                        *automatic,
                    );
                    if let Some(axis) = selected.invalid_axis {
                        resize_diagnostics.push(geometry_ops::resize::Diagnostic {
                            node: index,
                            axis: Some(axis),
                        });
                    }
                    check(
                        selected.scales.iter().all(|v| v.is_finite()),
                        "Nonfinite profile resize scale",
                    )?;
                    for point in rings.iter_mut().flatten() {
                        for axis in 0..2 {
                            point[axis] *= selected.scales[axis];
                        }
                    }
                }
                rings
            }
            Node::Offset {
                input,
                distance,
                join,
                segments,
            } => rings::offset_join(
                results[*input].as_ref().unwrap(),
                *distance,
                join.name(),
                *segments,
            )?,
            Node::EvenOddRings(value) => {
                match rings::normalize(value, planar_geometry::tessellation::FillRule::EvenOdd) {
                    Ok(rings) => {
                        if rings.is_empty() {
                            diagnostics.push(Diagnostic {
                                node: index,
                                issue: Issue::EmptyContours,
                            });
                        }
                        rings
                    }
                    Err(_) => {
                        diagnostics.push(Diagnostic {
                            node: index,
                            issue: Issue::InvalidContours,
                        });
                        vec![]
                    }
                }
            }
            Node::Transform { input, matrix } => {
                check(
                    matrix[2] == [0., 0., 1.],
                    "Projective profile transforms are not supported",
                )?;
                let determinant = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
                check(
                    determinant.is_finite() && determinant != 0.,
                    "Profile transform must be nonsingular",
                )?;
                let mut result = results[*input].as_ref().unwrap().clone();
                for point in result.iter_mut().flatten() {
                    let [x, y] = *point;
                    *point = [
                        matrix[0][0] * x + matrix[0][1] * y + matrix[0][2],
                        matrix[1][0] * x + matrix[1][1] * y + matrix[1][2],
                    ];
                }
                if determinant < 0. {
                    for ring in &mut result {
                        ring.reverse();
                    }
                }
                result
            }
            Node::Minkowski { inputs } => super::modeling::minkowski_profiles(
                &inputs
                    .iter()
                    .map(|&input| results[input].as_ref().unwrap())
                    .collect::<Vec<_>>(),
            )?,
            Node::Hull { inputs } => {
                let points = inputs
                    .iter()
                    .flat_map(|&input| results[input].as_ref().unwrap().iter().flatten().copied())
                    .collect();
                let hull = rings::hull2(points);
                if hull.len() < 3 { vec![] } else { vec![hull] }
            }
            Node::Boolean { operation, inputs } => {
                let mut result = inputs
                    .first()
                    .map_or_else(Vec::new, |&input| results[input].as_ref().unwrap().clone());
                let operation = match operation {
                    Boolean::Union => "union",
                    Boolean::Intersection => "intersection",
                    Boolean::Difference => "difference",
                };
                for &input in inputs.iter().skip(1) {
                    result = rings::planar(&result, results[input].as_ref().unwrap(), operation)?;
                }
                result
            }
        };
        check(
            result
                .iter()
                .flatten()
                .flatten()
                .all(|value| value.is_finite()),
            "Profile execution produced non-finite coordinates",
        )?;
        let count = result.iter().map(Vec::len).sum::<usize>();
        retained = retained
            .checked_add(count)
            .ok_or_else(|| crate::error("Profile retained point budget exceeded"))?;
        check(
            retained <= max_retained_points,
            "Profile retained point budget exceeded",
        )?;
        results[index] = Some(result);
        for &input in node.inputs() {
            uses[input] -= 1;
            if uses[input] == 0 {
                retained -= results[input]
                    .take()
                    .unwrap()
                    .iter()
                    .map(Vec::len)
                    .sum::<usize>();
            }
        }
    }
    Ok(ExecutionReport {
        mesh_failures,
        resize_diagnostics,
        profiles: program
            .roots
            .iter()
            .map(|&root| results[root].as_ref().unwrap().clone())
            .collect(),
        diagnostics,
        slice_reductions,
        sweep_warnings,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn boolean_hole_reflection_and_retained_budget() {
        let program = Program {
            nodes: vec![
                Node::Rectangle {
                    size: [4.; 2],
                    center: true,
                },
                Node::Rectangle {
                    size: [2.; 2],
                    center: true,
                },
                Node::Boolean {
                    operation: Boolean::Difference,
                    inputs: vec![0, 1],
                },
                Node::Transform {
                    input: 2,
                    matrix: [[-1., 0., 5.], [0., 1., 0.], [0., 0., 1.]],
                },
            ],
            roots: vec![3],
        };
        let profiles = execute(&program, 100).unwrap();
        assert!(
            (profiles[0]
                .iter()
                .map(|ring| rings::area(ring))
                .sum::<f64>()
                - 12.)
                .abs()
                < 1e-10
        );
        assert!(execute(&program, 3).is_err());
        let mesh =
            super::super::modeling::extrude_rings(&profiles[0], 3., 1, 0., [1.; 2], false).unwrap();
        assert!((mesh.inspect().unwrap().signed_volume_mm3 - 36.).abs() < 1e-10);
    }
}

#[cfg(test)]
mod even_odd_tests {
    use super::*;
    #[test]
    fn equally_oriented_nested_contours_form_a_hole() {
        let contours = vec![
            vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]],
            vec![[1., 1.], [3., 1.], [3., 3.], [1., 3.]],
        ];
        let program = Program {
            nodes: vec![Node::EvenOddRings(contours.clone())],
            roots: vec![0],
        };
        let profiles = execute(&program, 100).unwrap();
        assert!(
            (profiles[0]
                .iter()
                .map(|ring| rings::area(ring))
                .sum::<f64>()
                - 12.)
                .abs()
                < 1e-10
        );
        let program = Program {
            nodes: vec![Node::Rings(contours)],
            roots: vec![0],
        };
        let profiles = execute(&program, 100).unwrap();
        assert!(
            (profiles[0]
                .iter()
                .map(|ring| rings::area(ring))
                .sum::<f64>()
                - 16.)
                .abs()
                < 1e-10
        );
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    #[test]
    fn reports_empty_contours_only_when_reachable() {
        let mut program = Program {
            nodes: vec![Node::EvenOddRings(vec![vec![[0., 0.], [1., 0.], [2., 0.]]])],
            roots: vec![0],
        };
        let report = execute_report(&program, 100).unwrap();
        assert!(report.profiles[0].is_empty());
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].issue, Issue::EmptyContours);
        program.roots.clear();
        assert!(
            execute_report(&program, 100)
                .unwrap()
                .diagnostics
                .is_empty()
        );
    }
}

#[cfg(test)]
mod offset_tests {
    use super::*;
    use geometry_ops::profile_program::OffsetJoin;
    #[test]
    fn offset_rectangle_expands_contracts_and_respects_liveness() {
        let mut program = Program {
            nodes: vec![
                Node::Rectangle {
                    size: [4.; 2],
                    center: false,
                },
                Node::Offset {
                    input: 0,
                    distance: 1.,
                    join: OffsetJoin::Miter,
                    segments: 16,
                },
            ],
            roots: vec![1],
        };
        let profiles = execute(&program, 100).unwrap();
        assert!(
            (profiles[0]
                .iter()
                .map(|ring| rings::area(ring))
                .sum::<f64>()
                - 36.)
                .abs()
                < 1e-10
        );
        if let Node::Offset { distance, .. } = &mut program.nodes[1] {
            *distance = -1.;
        }
        let profiles = execute(&program, 100).unwrap();
        assert!(
            (profiles[0]
                .iter()
                .map(|ring| rings::area(ring))
                .sum::<f64>()
                - 4.)
                .abs()
                < 1e-10
        );
        assert!(execute(&program, 1).is_err());
        program.roots.clear();
        assert!(execute(&program, 1).unwrap().is_empty());
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;
    #[test]
    fn section_projection_and_empty_roots_keep_profile_execution_limits() {
        use geometry_ops::solid_program::{Node as S, Program as Solid};
        for cut in [false, true] {
            let program = Program {
                nodes: vec![Node::Projection {
                    solid: Box::new(Solid {
                        nodes: vec![S::Cube {
                            size: [2., 3., 4.],
                            center: true,
                        }],
                        roots: vec![0],
                    }),
                    cut,
                }],
                roots: vec![0],
            };
            let profiles = execute(&program, 100).unwrap();
            assert!(
                (profiles[0]
                    .iter()
                    .map(|ring| rings::area(ring))
                    .sum::<f64>()
                    .abs()
                    - 6.)
                    .abs()
                    < 1e-8
            );
            assert!(execute(&program, 1).is_err());
        }
        let empty = Program {
            nodes: vec![Node::Projection {
                solid: Box::new(Solid {
                    nodes: vec![S::Empty],
                    roots: vec![0],
                }),
                cut: true,
            }],
            roots: vec![0],
        };
        assert!(execute(&empty, 100).unwrap()[0].is_empty());
    }
}

#[cfg(test)]
mod resize_tests {
    use super::*;
    #[test]
    fn deferred_resize_uses_evaluated_profile_extents() {
        let program = Program {
            nodes: vec![
                Node::Rectangle {
                    size: [5., 3.],
                    center: false,
                },
                Node::Resize {
                    invalid_newsize: false,
                    input: 0,
                    targets: [Some(10.), None],
                    automatic: [false, true],
                },
            ],
            roots: vec![1],
        };
        let report = execute_report(&program, 100).unwrap();
        assert!(
            (report.profiles[0]
                .iter()
                .map(|ring| rings::area(ring))
                .sum::<f64>()
                - 60.)
                .abs()
                < 1e-8
        );
        assert!(report.resize_diagnostics.is_empty());
        assert!(execute(&program, 1).is_err());
    }
}
