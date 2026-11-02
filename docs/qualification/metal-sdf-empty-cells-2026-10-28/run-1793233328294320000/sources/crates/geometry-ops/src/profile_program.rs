//! Kernel-neutral planar expressions for language compilation.
use crate::solid_program::Boolean;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetJoin {
    Round,
    Miter,
    Square,
}
impl OffsetJoin {
    pub fn name(self) -> &'static str {
        match self {
            Self::Round => "Round",
            Self::Miter => "Miter",
            Self::Square => "Square",
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Empty,
    Resize {
        invalid_newsize: bool,
        input: usize,
        targets: [Option<f64>; 2],
        automatic: [bool; 2],
    },
    Projection {
        solid: Box<crate::solid_program::Program>,
        cut: bool,
    },
    Rectangle {
        size: [f64; 2],
        center: bool,
    },
    Circle {
        radius: f64,
        segments: usize,
    },
    Rings(Vec<Vec<[f64; 2]>>),
    EvenOddRings(Vec<Vec<[f64; 2]>>),
    Offset {
        input: usize,
        distance: f64,
        join: OffsetJoin,
        segments: usize,
    },
    Transform {
        input: usize,
        matrix: [[f64; 3]; 3],
    },
    Minkowski {
        inputs: Vec<usize>,
    },
    Hull {
        inputs: Vec<usize>,
    },
    Boolean {
        operation: Boolean,
        inputs: Vec<usize>,
    },
}
impl Node {
    pub fn inputs(&self) -> &[usize] {
        match self {
            Self::Resize { input, .. }
            | Self::Transform { input, .. }
            | Self::Offset { input, .. } => std::slice::from_ref(input),
            Self::Boolean { inputs, .. } | Self::Hull { inputs } | Self::Minkowski { inputs } => {
                inputs
            }
            _ => &[],
        }
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    pub nodes: Vec<Node>,
    pub roots: Vec<usize>,
}
impl Program {
    /// Copy the selected expressions, preserving shared inputs and root order.
    /// Geometry admission applies to the resulting program, not unused arena data.
    pub fn from_roots(arena: &[Node], roots: &[usize]) -> crate::Result<Self> {
        let (nodes, roots) = crate::program_graph::compact(
            arena,
            roots,
            Node::inputs,
            |node, indices| match node {
                Node::Resize { input, .. }
                | Node::Transform { input, .. }
                | Node::Offset { input, .. } => *input = indices[*input],
                Node::Boolean { inputs, .. }
                | Node::Hull { inputs }
                | Node::Minkowski { inputs } => {
                    for input in inputs {
                        *input = indices[*input];
                    }
                }
                _ => {}
            },
        )?;
        let program = Self { nodes, roots };
        program.validate()?;
        Ok(program)
    }

    pub fn validate(&self) -> crate::Result<()> {
        crate::program_graph::validate_nested(crate::program_graph::NestedProgram::Profile(self))?;
        self.validate_local()
    }
    pub(crate) fn validate_local(&self) -> crate::Result<()> {
        crate::program_graph::validate(
            self.nodes.len(),
            &self.roots,
            self.nodes.iter().map(Node::inputs),
            "Profile program budget exceeded",
            "Profile program references must precede consumers",
        )?;
        let points = self.nodes.iter().try_fold(0usize, |sum, node| match node {
            Node::Rings(rings) | Node::EvenOddRings(rings) => rings
                .iter()
                .try_fold(sum, |sum, ring| sum.checked_add(ring.len())),
            _ => Some(sum),
        });
        if points.is_none_or(|count| count > 100_000) {
            return Err(crate::fail("Profile program point budget exceeded"));
        }
        for node in &self.nodes {
            let valid = match node {
                Node::Resize { targets, .. } => {
                    targets.iter().flatten().all(|v| v.is_finite() && *v > 0.)
                }
                Node::Projection { solid, .. } => {
                    solid.validate_local()?;
                    if solid.roots.len() != 1 {
                        return Err(crate::fail("Projection requires exactly one solid root"));
                    }
                    true
                }
                Node::Rectangle { size, .. } => size.iter().all(|v| v.is_finite() && *v > 0.),
                Node::Circle { radius, segments } => {
                    radius.is_finite() && *radius > 0. && (3..=512).contains(segments)
                }
                Node::Rings(rings) | Node::EvenOddRings(rings) => {
                    rings.iter().flatten().flatten().all(|v| v.is_finite())
                }
                Node::Transform { matrix, .. } => matrix.iter().flatten().all(|v| v.is_finite()),
                Node::Offset {
                    distance, segments, ..
                } => distance.is_finite() && (3..=512).contains(segments),
                _ => true,
            };
            if !valid {
                return Err(crate::fail("Invalid profile program geometry"));
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alternating_programs_share_nesting_and_aggregate_budgets() {
        use crate::solid_program::{Node as S, Program as Solid};
        let mut profile = Program {
            nodes: vec![Node::Empty],
            roots: vec![0],
        };
        for _ in 0..17 {
            let solid = Solid {
                nodes: vec![S::ExtrudeProfile {
                    profile,
                    slice_policy: None,
                    height: 1.,
                    slices: 0,
                    twist: 0.,
                    scale: [1.; 2],
                    center: false,
                }],
                roots: vec![0],
            };
            profile = Program {
                nodes: vec![Node::Projection {
                    solid: Box::new(solid),
                    cut: false,
                }],
                roots: vec![0],
            };
        }
        assert!(profile.validate().unwrap_err().message.contains("nesting"));
        let solid = Solid {
            nodes: vec![S::Empty; 25_000],
            roots: vec![0],
        };
        let profile = Program {
            nodes: vec![Node::Projection {
                solid: Box::new(solid),
                cut: false,
            }],
            roots: vec![0],
        };
        assert!(
            profile
                .validate()
                .unwrap_err()
                .message
                .contains("aggregate")
        );
    }
    #[test]
    fn extracts_shared_subgraphs_without_unused_geometry() {
        let arena = vec![
            Node::Rings(vec![vec![[f64::NAN; 2]; 100_001]]),
            Node::Rectangle {
                size: [2., 3.],
                center: false,
            },
            Node::Empty,
            Node::Transform {
                input: 1,
                matrix: [[1., 0., 2.], [0., 1., 0.], [0., 0., 1.]],
            },
            Node::Boolean {
                operation: Boolean::Difference,
                inputs: vec![3, 1, 1],
            },
        ];
        let program = Program::from_roots(&arena, &[4, 3, 4]).unwrap();
        assert_eq!(program.roots, vec![2, 1, 2]);
        assert_eq!(program.nodes.len(), 3);
        assert_eq!(program.nodes[0], arena[1]);
        assert!(matches!(program.nodes[1], Node::Transform { input: 0, .. }));
        assert!(matches!(&program.nodes[2], Node::Boolean { inputs, .. } if inputs == &[1, 0, 0]));
        assert!(Program::from_roots(&arena, &[]).unwrap().nodes.is_empty());
        assert!(Program::from_roots(&arena, &[0]).is_err());
        assert!(Program::from_roots(&arena, &[5]).is_err());
        assert!(
            Program::from_roots(
                &[Node::Transform {
                    input: 0,
                    matrix: [[0.; 3]; 3]
                }],
                &[0]
            )
            .is_err()
        );
    }
    #[test]
    fn accepts_ordered_profiles_and_rejects_cycles_and_point_budget() {
        let mut program = Program {
            nodes: vec![
                Node::Rectangle {
                    size: [2., 3.],
                    center: true,
                },
                Node::Boolean {
                    operation: Boolean::Union,
                    inputs: vec![0],
                },
            ],
            roots: vec![1],
        };
        assert!(program.validate().is_ok());
        program.nodes[1] = Node::Boolean {
            operation: Boolean::Union,
            inputs: vec![1],
        };
        assert!(program.validate().is_err());
        program.nodes = vec![Node::Rings(vec![vec![[0.; 2]; 100_001]])];
        program.roots = vec![0];
        assert!(program.validate().is_err());
    }
}

#[cfg(feature = "codec")]
mod serialization {
    use super::*;
    use crate::codec;
    use value_codec::{Deserialize, Result, Serialize, Value, json};
    impl Serialize for Node {
        fn to_value(&self) -> Value {
            match self {
                Self::Empty => json!({"kind":"empty"}),
                Self::Resize {
                    invalid_newsize,
                    input,
                    targets,
                    automatic,
                } => {
                    json!({"kind":"resize","invalidNewsize":invalid_newsize,"input":input,"targets":targets,"automatic":automatic})
                }
                Self::Projection { solid, cut } => {
                    json!({"kind":"projection","solid":solid.as_ref(),"cut":cut})
                }
                Self::Rectangle { size, center } => {
                    json!({"kind":"rectangle","size":size,"center":center})
                }
                Self::Circle { radius, segments } => {
                    json!({"kind":"circle","radius":radius,"segments":segments})
                }
                Self::Rings(rings) => json!({"kind":"rings","rings":rings}),
                Self::EvenOddRings(rings) => json!({"kind":"even_odd_rings","rings":rings}),
                Self::Offset {
                    input,
                    distance,
                    join,
                    segments,
                } => {
                    json!({"kind":"offset","input":input,"distance":distance,"join":join.name(),"segments":segments})
                }
                Self::Transform { input, matrix } => {
                    json!({"kind":"transform","input":input,"matrix":matrix})
                }
                Self::Minkowski { inputs } => json!({"kind":"minkowski","inputs":inputs}),
                Self::Hull { inputs } => json!({"kind":"hull","inputs":inputs}),
                Self::Boolean { operation, inputs } => {
                    json!({"kind":match operation {Boolean::Union=>"union",Boolean::Intersection=>"intersection",Boolean::Difference=>"difference"},"inputs":inputs})
                }
            }
        }
    }
    impl<'de> Deserialize<'de> for Node {
        fn from_value(value: Value) -> Result<Self> {
            let mut object = codec::object(value)?;
            let kind: String = codec::required(&mut object, "kind")?;
            Ok(match kind.as_str() {
                "empty" => Self::Empty,
                "resize" => Self::Resize {
                    invalid_newsize: codec::optional(&mut object, "invalidNewsize", false)?,
                    input: codec::required(&mut object, "input")?,
                    targets: codec::required(&mut object, "targets")?,
                    automatic: codec::required(&mut object, "automatic")?,
                },
                "projection" => Self::Projection {
                    solid: Box::new(codec::required(&mut object, "solid")?),
                    cut: codec::required(&mut object, "cut")?,
                },
                "rectangle" => Self::Rectangle {
                    size: codec::required(&mut object, "size")?,
                    center: codec::required(&mut object, "center")?,
                },
                "circle" => Self::Circle {
                    radius: codec::required(&mut object, "radius")?,
                    segments: codec::required(&mut object, "segments")?,
                },
                "rings" => Self::Rings(codec::required(&mut object, "rings")?),
                "even_odd_rings" => Self::EvenOddRings(codec::required(&mut object, "rings")?),
                "offset" => Self::Offset {
                    input: codec::required(&mut object, "input")?,
                    distance: codec::required(&mut object, "distance")?,
                    segments: codec::required(&mut object, "segments")?,
                    join: match codec::required::<String>(&mut object, "join")?.as_str() {
                        "Round" => OffsetJoin::Round,
                        "Miter" => OffsetJoin::Miter,
                        "Square" => OffsetJoin::Square,
                        _ => return Err(value_codec::error("Unsupported profile offset join")),
                    },
                },
                "transform" => Self::Transform {
                    input: codec::required(&mut object, "input")?,
                    matrix: codec::required(&mut object, "matrix")?,
                },
                "minkowski" => Self::Minkowski {
                    inputs: codec::required(&mut object, "inputs")?,
                },
                "hull" => Self::Hull {
                    inputs: codec::required(&mut object, "inputs")?,
                },
                "union" | "intersection" | "difference" => Self::Boolean {
                    operation: match kind.as_str() {
                        "union" => Boolean::Union,
                        "intersection" => Boolean::Intersection,
                        _ => Boolean::Difference,
                    },
                    inputs: codec::required(&mut object, "inputs")?,
                },
                _ => return Err(value_codec::error("Unsupported profile program node")),
            })
        }
    }
    impl Serialize for Program {
        fn to_value(&self) -> Value {
            json!({"nodes":self.nodes,"roots":self.roots})
        }
    }
    impl<'de> Deserialize<'de> for Program {
        fn from_value(value: Value) -> Result<Self> {
            let mut object = codec::object(value)?;
            Ok(Self {
                nodes: codec::required(&mut object, "nodes")?,
                roots: codec::required(&mut object, "roots")?,
            })
        }
    }
}
#[cfg(all(test, feature = "codec"))]
mod codec_tests {
    use super::*;
    use value_codec::{Deserialize, Serialize};
    #[test]
    fn resize_transport_and_compaction_preserve_deferred_parameters() {
        let arena = vec![
            Node::Empty,
            Node::Rectangle {
                size: [5., 3.],
                center: false,
            },
            Node::Resize {
                invalid_newsize: false,
                input: 1,
                targets: [Some(10.), None],
                automatic: [false, true],
            },
        ];
        let program = Program::from_roots(&arena, &[2]).unwrap();
        assert_eq!(program.nodes[1].inputs(), &[0]);
        assert_eq!(Program::from_value(program.to_value()).unwrap(), program);
    }
    #[test]
    fn projection_transport_preserves_the_selected_solid_program() {
        let program = Program {
            nodes: vec![Node::Projection {
                solid: Box::new(crate::solid_program::Program {
                    nodes: vec![crate::solid_program::Node::Cube {
                        size: [2.; 3],
                        center: true,
                    }],
                    roots: vec![0],
                }),
                cut: true,
            }],
            roots: vec![0],
        };
        program.validate().unwrap();
        assert_eq!(Program::from_value(program.to_value()).unwrap(), program);
    }
    #[test]
    fn hull_transport_preserves_duplicate_inputs_after_compaction() {
        let arena = vec![
            Node::Empty,
            Node::Circle {
                radius: 1.,
                segments: 8,
            },
            Node::Hull { inputs: vec![1, 1] },
        ];
        let program = Program::from_roots(&arena, &[2]).unwrap();
        assert!(matches!(&program.nodes[1], Node::Hull { inputs } if inputs == &[0,0]));
        assert_eq!(Program::from_value(program.to_value()).unwrap(), program);
    }
    #[test]
    fn offset_joins_survive_transport_and_input_remapping() {
        for join in [OffsetJoin::Round, OffsetJoin::Miter, OffsetJoin::Square] {
            let arena = vec![
                Node::Empty,
                Node::Rectangle {
                    size: [4.; 2],
                    center: false,
                },
                Node::Offset {
                    input: 1,
                    distance: -1.,
                    join,
                    segments: 16,
                },
            ];
            let program = Program::from_roots(&arena, &[2]).unwrap();
            assert_eq!(Program::from_value(program.to_value()).unwrap(), program);
            assert!(matches!(program.nodes[1], Node::Offset { input: 0, .. }));
        }
        assert!(Node::from_value(value_codec::json!({"kind":"offset","input":0,"distance":1,"segments":16,"join":"bad"})).is_err());
    }
    #[test]
    fn every_profile_node_survives_transport() {
        let program = Program {
            nodes: vec![
                Node::Empty,
                Node::Rectangle {
                    size: [2., 3.],
                    center: true,
                },
                Node::Circle {
                    radius: 2.,
                    segments: 3,
                },
                Node::Rings(vec![vec![[0., 0.], [1., 0.], [0., 1.]]]),
                Node::Transform {
                    input: 1,
                    matrix: [[1., 0., 2.], [0., 1., 3.], [0., 0., 1.]],
                },
                Node::Boolean {
                    operation: Boolean::Union,
                    inputs: vec![2, 4],
                },
                Node::Boolean {
                    operation: Boolean::Intersection,
                    inputs: vec![3, 5],
                },
                Node::Boolean {
                    operation: Boolean::Difference,
                    inputs: vec![5, 6],
                },
            ],
            roots: vec![7],
        };
        let decoded = Program::from_value(program.to_value()).unwrap();
        assert_eq!(decoded, program);
        assert!(decoded.validate().is_ok());
        assert!(Node::from_value(value_codec::json!({"kind":"unsupported"})).is_err());
    }
}

#[cfg(test)]
mod parameter_tests {
    use super::*;
    #[test]
    fn rejects_nonfinite_coordinates_and_invalid_resolution() {
        for node in [
            Node::Circle {
                radius: 1.,
                segments: 2,
            },
            Node::Rectangle {
                size: [0., 2.],
                center: false,
            },
            Node::Rings(vec![vec![[f64::NAN, 0.]]]),
            Node::Transform {
                input: 0,
                matrix: [[f64::INFINITY; 3]; 3],
            },
        ] {
            let program = Program {
                nodes: vec![Node::Empty, node],
                roots: vec![1],
            };
            assert!(program.validate().is_err());
        }
    }
}

#[cfg(all(test, feature = "codec"))]
mod even_odd_codec_tests {
    use super::*;
    use value_codec::{Deserialize, Serialize};
    #[test]
    fn even_odd_fill_survives_transport() {
        let node = Node::EvenOddRings(vec![vec![[0., 0.], [2., 0.], [0., 2.]]]);
        assert_eq!(Node::from_value(node.to_value()).unwrap(), node);
    }
}
