//! Native, topologically ordered solid expressions shared by language compilers and executors.
pub use math_core::affine;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Boolean {
    Union,
    Intersection,
    Difference,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Empty,
    Mesh {
        mesh: crate::Triangles,
        empty_on_failure: bool,
    },
    Resize {
        invalid_newsize: bool,
        input: usize,
        targets: [Option<f64>; 3],
        automatic: [bool; 3],
    },
    Cube {
        size: [f64; 3],
        center: bool,
    },
    Sphere {
        radius: f64,
        segments: usize,
    },
    Cylinder {
        height: f64,
        radii: [f64; 2],
        segments: usize,
        center: bool,
    },
    /// Extrusion of already evaluated planar rings, independent of a language frontend.
    ExtrudeRings {
        rings: Vec<Vec<[f64; 2]>>,
        height: f64,
        slices: usize,
        twist: f64,
        scale: [f64; 2],
        center: bool,
    },
    ExtrudeProfile {
        profile: crate::profile_program::Program,
        slice_policy: Option<crate::extrude_slices::Policy>,
        height: f64,
        slices: usize,
        twist: f64,
        scale: [f64; 2],
        center: bool,
    },
    RevolveProfile {
        fragment_policy: Option<crate::fragment_resolution::Policy>,
        profile: crate::profile_program::Program,
        angle: f64,
        segments: usize,
    },
    Translate {
        input: usize,
        delta: [f64; 3],
    },
    Transform {
        input: usize,
        matrix: affine::AffineMatrix,
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
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    pub nodes: Vec<Node>,
    pub roots: Vec<usize>,
}
impl Node {
    pub fn inputs(&self) -> &[usize] {
        match self {
            Self::Resize { input, .. }
            | Self::Translate { input, .. }
            | Self::Transform { input, .. } => std::slice::from_ref(input),
            Self::Boolean { inputs, .. } | Self::Hull { inputs } | Self::Minkowski { inputs } => {
                inputs
            }
            _ => &[],
        }
    }
}
impl Program {
    /// Extract selected solid expressions without unrelated arena geometry.
    pub fn from_roots(arena: &[Node], roots: &[usize]) -> crate::Result<Self> {
        let (nodes, roots) = crate::program_graph::compact(
            arena,
            roots,
            Node::inputs,
            |node, indices| match node {
                Node::Resize { input, .. }
                | Node::Translate { input, .. }
                | Node::Transform { input, .. } => *input = indices[*input],
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
        crate::program_graph::validate_nested(crate::program_graph::NestedProgram::Solid(self))?;
        self.validate_local()
    }
    pub(crate) fn validate_local(&self) -> crate::Result<()> {
        let profile_points = self.nodes.iter().try_fold(0usize, |sum, node| match node {
            Node::ExtrudeRings { rings, .. } => rings
                .iter()
                .try_fold(sum, |sum, ring| sum.checked_add(ring.len())),
            Node::ExtrudeProfile { profile, .. } | Node::RevolveProfile { profile, .. } => {
                profile.nodes.iter().try_fold(sum, |sum, node| match node {
                    crate::profile_program::Node::Rings(rings)
                    | crate::profile_program::Node::EvenOddRings(rings) => rings
                        .iter()
                        .try_fold(sum, |sum, ring| sum.checked_add(ring.len())),
                    _ => Some(sum),
                })
            }
            _ => Some(sum),
        });
        if profile_points.is_none_or(|count| count > 100_000) {
            return Err(crate::fail("Solid program profile point budget exceeded"));
        }
        let inputs = self.nodes.iter().map(Node::inputs).collect::<Vec<_>>();
        let reachable = crate::program_graph::execution_plan(&inputs, &self.roots)?.reachable;
        let mut profile_nodes = 0usize;
        let mut profile_edges = 0usize;
        for (index, node) in self.nodes.iter().enumerate() {
            let finite_parameters = match node {
                Node::Mesh { mesh, .. } => {
                    mesh.positions.len() % 3 == 0
                        && mesh.indices.len() % 3 == 0
                        && mesh.positions.iter().all(|value| value.is_finite())
                        && mesh
                            .indices
                            .iter()
                            .all(|&index| index < mesh.positions.len() / 3)
                }
                Node::Resize { targets, .. } => {
                    targets.iter().flatten().all(|v| v.is_finite() && *v > 0.)
                }
                Node::Cube { size, .. } => size.iter().all(|value| value.is_finite()),
                Node::Sphere { radius, .. } => radius.is_finite(),
                Node::Cylinder { height, radii, .. } => {
                    height.is_finite() && radii.iter().all(|value| value.is_finite())
                }
                Node::Translate { delta, .. } => delta.iter().all(|value| value.is_finite()),
                Node::Transform { matrix, .. } => {
                    matrix.iter().flatten().all(|value| value.is_finite())
                }
                _ => true,
            };
            if reachable[index] && !finite_parameters {
                return Err(crate::fail("Invalid solid program geometry"));
            }
            if let Node::ExtrudeProfile { profile, .. } | Node::RevolveProfile { profile, .. } =
                node
            {
                profile.validate_local()?;
                for node in &profile.nodes {
                    profile_edges =
                        profile_edges
                            .checked_add(node.inputs().len())
                            .ok_or_else(|| {
                                crate::fail("Solid program nested profile budget exceeded")
                            })?;
                }
                if profile_edges > 100_000 {
                    return Err(crate::fail("Solid program nested profile budget exceeded"));
                }
                if profile.roots.len() != 1 {
                    return Err(crate::fail(
                        "Solid construction requires exactly one profile root",
                    ));
                }
                profile_nodes = profile_nodes
                    .checked_add(profile.nodes.len())
                    .ok_or_else(|| crate::fail("Solid program nested profile budget exceeded"))?;
                if profile_nodes > 25_000 {
                    return Err(crate::fail("Solid program nested profile budget exceeded"));
                }
            }
            if let Node::RevolveProfile {
                angle,
                segments,
                fragment_policy,
                ..
            } = node
            {
                if !angle.is_finite()
                    || *angle == 0.
                    || angle.abs() > 360.
                    || (fragment_policy.is_none()
                        && (!(1..=512).contains(segments)
                            || (angle.abs() == 360. && *segments < 3)))
                {
                    return Err(crate::fail("Invalid solid program revolution"));
                }
                if let Some(policy) = fragment_policy {
                    policy.validate()?;
                }
            }

            if let Node::ExtrudeProfile {
                slice_policy,
                height,
                slices,
                twist,
                scale,
                ..
            } = node
            {
                if slice_policy.as_ref().is_some_and(|policy| {
                    policy.maximum == 0
                        || policy.maximum > 512
                        || policy
                            .explicit
                            .is_some_and(|value| !value.is_finite() || value < 1.)
                }) {
                    return Err(crate::fail("Invalid extrusion subdivision policy"));
                }
                if !height.is_finite()
                    || *height <= 0.
                    || *slices > 513
                    || !twist.is_finite()
                    || !scale.iter().all(|v| v.is_finite())
                {
                    return Err(crate::fail("Invalid solid program extrusion"));
                }
            }
            if let Node::ExtrudeRings {
                rings,
                height,
                slices,
                twist,
                scale,
                ..
            } = node
            {
                if !height.is_finite()
                    || *height <= 0.
                    || *slices > 513
                    || !twist.is_finite()
                    || !scale.iter().all(|v| v.is_finite())
                    || !rings.iter().flatten().flatten().all(|v| v.is_finite())
                {
                    return Err(crate::fail("Invalid solid program extrusion"));
                }
            }
        }
        crate::program_graph::validate(
            self.nodes.len(),
            &self.roots,
            self.nodes.iter().map(Node::inputs),
            "Solid program budget exceeded",
            "Solid program references must precede consumers",
        )?;
        Ok(())
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
                Self::Mesh {
                    mesh,
                    empty_on_failure,
                } => json!({"kind":"mesh","mesh":mesh,"emptyOnFailure":empty_on_failure}),
                Self::Resize {
                    invalid_newsize,
                    input,
                    targets,
                    automatic,
                } => {
                    json!({"kind":"resize","invalidNewsize":invalid_newsize,"input":input,"targets":targets,"automatic":automatic})
                }
                Self::Cube { size, center } => json!({"kind":"cube","size":size,"center":center}),
                Self::Sphere { radius, segments } => {
                    json!({"kind":"sphere","radius":radius,"segments":segments})
                }
                Self::Cylinder {
                    height,
                    radii,
                    segments,
                    center,
                } => {
                    json!({"kind":"cylinder","height":height,"radii":radii,"segments":segments,"center":center})
                }
                Self::ExtrudeRings {
                    rings,
                    height,
                    slices,
                    twist,
                    scale,
                    center,
                } => {
                    json!({"kind":"extrude_rings","rings":rings,"height":height,"slices":slices,"twist":twist,"scale":scale,"center":center})
                }
                Self::ExtrudeProfile {
                    profile,
                    slice_policy,
                    height,
                    slices,
                    twist,
                    scale,
                    center,
                } => {
                    json!({"kind":"extrude_profile","profile":profile,"slicePolicy":slice_policy,"height":height,"slices":slices,"twist":twist,"scale":scale,"center":center})
                }
                Self::RevolveProfile {
                    fragment_policy,
                    profile,
                    angle,
                    segments,
                } => {
                    json!({"kind":"revolve_profile","fragmentPolicy":fragment_policy,"profile":profile,"angle":angle,"segments":segments})
                }
                Self::Translate { input, delta } => {
                    json!({"kind":"translate","input":input,"delta":delta})
                }
                Self::Transform { input, matrix } => {
                    json!({"kind":"transform","input":input,"matrix":matrix})
                }
                Self::Minkowski { inputs } => json!({"kind":"minkowski","inputs":inputs}),
                Self::Hull { inputs } => json!({"kind":"hull","inputs":inputs}),
                Self::Boolean { operation, inputs } => {
                    json!({"kind":match operation{Boolean::Union=>"union",Boolean::Intersection=>"intersection",Boolean::Difference=>"difference"},"inputs":inputs})
                }
            }
        }
    }
    impl<'de> Deserialize<'de> for Node {
        fn from_value(v: Value) -> Result<Self> {
            let mut o = codec::object(v)?;
            let kind: String = codec::required(&mut o, "kind")?;
            Ok(match kind.as_str() {
                "empty" => Self::Empty,
                "mesh" => Self::Mesh {
                    mesh: codec::required(&mut o, "mesh")?,
                    empty_on_failure: codec::optional(&mut o, "emptyOnFailure", false)?,
                },
                "resize" => Self::Resize {
                    invalid_newsize: codec::optional(&mut o, "invalidNewsize", false)?,
                    input: codec::required(&mut o, "input")?,
                    targets: codec::required(&mut o, "targets")?,
                    automatic: codec::required(&mut o, "automatic")?,
                },
                "cube" => Self::Cube {
                    size: codec::required(&mut o, "size")?,
                    center: codec::required(&mut o, "center")?,
                },
                "sphere" => Self::Sphere {
                    radius: codec::required(&mut o, "radius")?,
                    segments: codec::required(&mut o, "segments")?,
                },
                "cylinder" => Self::Cylinder {
                    height: codec::required(&mut o, "height")?,
                    radii: codec::required(&mut o, "radii")?,
                    segments: codec::required(&mut o, "segments")?,
                    center: codec::required(&mut o, "center")?,
                },
                "extrude_rings" => Self::ExtrudeRings {
                    rings: codec::required(&mut o, "rings")?,
                    height: codec::required(&mut o, "height")?,
                    slices: codec::required(&mut o, "slices")?,
                    twist: codec::required(&mut o, "twist")?,
                    scale: codec::required(&mut o, "scale")?,
                    center: codec::required(&mut o, "center")?,
                },
                "extrude_profile" => Self::ExtrudeProfile {
                    profile: codec::required(&mut o, "profile")?,
                    slice_policy: o
                        .remove("slicePolicy")
                        .map(Option::<crate::extrude_slices::Policy>::from_value)
                        .transpose()?
                        .flatten(),
                    height: codec::required(&mut o, "height")?,
                    slices: codec::required(&mut o, "slices")?,
                    twist: codec::required(&mut o, "twist")?,
                    scale: codec::required(&mut o, "scale")?,
                    center: codec::required(&mut o, "center")?,
                },
                "revolve_profile" => Self::RevolveProfile {
                    fragment_policy: codec::optional(&mut o, "fragmentPolicy", None)?,
                    profile: codec::required(&mut o, "profile")?,
                    angle: codec::required(&mut o, "angle")?,
                    segments: codec::required(&mut o, "segments")?,
                },
                "translate" => Self::Translate {
                    input: codec::required(&mut o, "input")?,
                    delta: codec::required(&mut o, "delta")?,
                },
                "transform" => Self::Transform {
                    input: codec::required(&mut o, "input")?,
                    matrix: codec::required(&mut o, "matrix")?,
                },
                "minkowski" => Self::Minkowski {
                    inputs: codec::required(&mut o, "inputs")?,
                },
                "hull" => Self::Hull {
                    inputs: codec::required(&mut o, "inputs")?,
                },
                "union" | "intersection" | "difference" => Self::Boolean {
                    operation: match kind.as_str() {
                        "union" => Boolean::Union,
                        "intersection" => Boolean::Intersection,
                        _ => Boolean::Difference,
                    },
                    inputs: codec::required(&mut o, "inputs")?,
                },
                _ => return Err(value_codec::error("Unsupported solid program node")),
            })
        }
    }
    impl Serialize for Program {
        fn to_value(&self) -> Value {
            json!({"nodes":self.nodes,"roots":self.roots})
        }
    }
    impl<'de> Deserialize<'de> for Program {
        fn from_value(v: Value) -> Result<Self> {
            let mut o = codec::object(v)?;
            Ok(Self {
                nodes: codec::required(&mut o, "nodes")?,
                roots: codec::required(&mut o, "roots")?,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extraction_remaps_shared_inputs_and_ignores_unrelated_invalid_geometry() {
        let arena = vec![
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
            Node::Transform {
                input: 2,
                matrix: affine::IDENTITY,
            },
            Node::Boolean {
                operation: Boolean::Union,
                inputs: vec![3, 1, 3],
            },
            Node::Hull { inputs: vec![4, 4] },
            Node::Minkowski { inputs: vec![5, 1] },
        ];
        let program = Program::from_roots(&arena, &[6, 3, 6]).unwrap();
        assert_eq!(program.nodes.len(), 6);
        assert_eq!(program.roots, vec![5, 2, 5]);
        assert_eq!(program.nodes[1].inputs(), &[0]);
        assert_eq!(program.nodes[2].inputs(), &[1]);
        assert_eq!(program.nodes[3].inputs(), &[2, 0, 2]);
        assert_eq!(program.nodes[4].inputs(), &[3, 3]);
        assert_eq!(program.nodes[5].inputs(), &[4, 0]);
        assert!(Program::from_roots(&arena, &[0]).is_err());
        assert!(Program::from_roots(&arena, &[arena.len()]).is_err());
        #[cfg(feature = "codec")]
        {
            use value_codec::{Deserialize, Serialize};
            assert_eq!(Program::from_value(program.to_value()).unwrap(), program);
        }
    }
    #[test]
    fn arena_rejects_cycles_forward_roots_and_edge_budgets() {
        assert!(
            Program {
                nodes: vec![Node::Translate {
                    input: 0,
                    delta: [0.; 3]
                }],
                roots: vec![0]
            }
            .validate()
            .is_err()
        );
        assert!(
            Program {
                nodes: vec![],
                roots: vec![0]
            }
            .validate()
            .is_err()
        );
        assert!(
            Program {
                nodes: vec![
                    Node::Cube {
                        size: [1.; 3],
                        center: false
                    },
                    Node::Boolean {
                        operation: Boolean::Union,
                        inputs: vec![0; 100_001]
                    }
                ],
                roots: vec![1]
            }
            .validate()
            .is_err()
        );
    }
}

#[cfg(all(test, feature = "codec"))]
mod extrusion_codec_tests {
    use super::*;
    use value_codec::{Deserialize, Serialize};
    #[test]
    fn minkowski_transport_preserves_operand_order() {
        let program = Program {
            nodes: vec![Node::Empty, Node::Minkowski { inputs: vec![0, 0] }],
            roots: vec![1],
        };
        assert_eq!(Program::from_value(program.to_value()).unwrap(), program);
        assert!(program.validate().is_ok());
    }
    #[test]
    fn hull_transport_validates_input_order() {
        let mut program = Program {
            nodes: vec![Node::Empty, Node::Hull { inputs: vec![0, 0] }],
            roots: vec![1],
        };
        assert_eq!(Program::from_value(program.to_value()).unwrap(), program);
        assert!(program.validate().is_ok());
        program.nodes[1] = Node::Hull { inputs: vec![1] };
        assert!(program.validate().is_err());
    }
    #[test]
    fn revolution_transport_and_admission() {
        let profile = crate::profile_program::Program {
            nodes: vec![crate::profile_program::Node::Rectangle {
                size: [2., 3.],
                center: false,
            }],
            roots: vec![0],
        };
        let node = Node::RevolveProfile {
            fragment_policy: None,
            profile,
            angle: -90.,
            segments: 8,
        };
        assert_eq!(Node::from_value(node.to_value()).unwrap(), node);
        let mut deferred = node.clone();
        if let Node::RevolveProfile {
            fragment_policy,
            segments,
            ..
        } = &mut deferred
        {
            *segments = 0;
            *fragment_policy = Some(crate::fragment_resolution::Policy {
                fragments: [12., 12., 2.],
                maximum: 128,
            });
        }
        assert_eq!(Node::from_value(deferred.to_value()).unwrap(), deferred);
        assert!(
            Program {
                nodes: vec![deferred],
                roots: vec![0]
            }
            .validate()
            .is_ok()
        );
        let mut program = Program {
            nodes: vec![node],
            roots: vec![0],
        };
        assert!(program.validate().is_ok());
        for angle_value in [0., 361., f64::NAN] {
            if let Node::RevolveProfile { angle, .. } = &mut program.nodes[0] {
                *angle = angle_value;
            }
            assert!(program.validate().is_err());
        }
        if let Node::RevolveProfile {
            angle,
            segments,
            profile,
            ..
        } = &mut program.nodes[0]
        {
            *angle = 360.;
            *segments = 2;
            assert!(profile.validate().is_ok());
        }
        assert!(program.validate().is_err());
        if let Node::RevolveProfile {
            segments, profile, ..
        } = &mut program.nodes[0]
        {
            *segments = 8;
            profile.roots.clear();
        }
        assert!(program.validate().is_err());
    }
    fn node() -> Node {
        Node::ExtrudeRings {
            rings: vec![vec![[0., 0.], [2., 0.], [0., 2.]]],
            height: 3.,
            slices: 4,
            twist: 30.,
            scale: [0.5, 1.],
            center: true,
        }
    }
    #[test]
    fn extrusion_transport_preserves_all_parameters() {
        let program = Program {
            nodes: vec![node()],
            roots: vec![0],
        };
        let decoded = Program::from_value(program.to_value()).unwrap();
        assert_eq!(decoded, program);
        assert!(decoded.validate().is_ok());
    }
    #[test]
    fn invalid_extrusions_and_aggregate_profile_budget_are_rejected() {
        for height in [0., -1., f64::INFINITY, f64::NAN] {
            let mut n = node();
            if let Node::ExtrudeRings { height: h, .. } = &mut n {
                *h = height;
            }
            assert!(
                Program {
                    nodes: vec![n],
                    roots: vec![0]
                }
                .validate()
                .is_err()
            );
        }
        let mut n = node();
        if let Node::ExtrudeRings { rings, .. } = &mut n {
            *rings = vec![vec![[0.; 2]; 50_001]];
        }
        assert!(
            Program {
                nodes: vec![n.clone(), n],
                roots: vec![1]
            }
            .validate()
            .is_err()
        );
    }
}

#[cfg(all(test, feature = "codec"))]
mod resize_codec_tests {
    use super::*;
    use value_codec::{Deserialize, Serialize};
    #[test]
    fn resize_keeps_axis_policy_and_remaps_selected_inputs() {
        let arena = vec![
            Node::Empty,
            Node::Cube {
                size: [5., 3., 2.],
                center: false,
            },
            Node::Resize {
                invalid_newsize: false,
                input: 1,
                targets: [Some(10.), None, None],
                automatic: [false, true, false],
            },
        ];
        let program = Program::from_roots(&arena, &[2]).unwrap();
        assert_eq!(program.nodes[1].inputs(), &[0]);
        assert_eq!(Program::from_value(program.to_value()).unwrap(), program);
    }
}

#[cfg(test)]
mod mesh_tests {
    use super::*;
    #[test]
    fn mesh_buffers_require_complete_finite_and_in_bounds_triangles() {
        let make = |positions, indices| Program {
            nodes: vec![Node::Mesh {
                mesh: crate::Triangles { positions, indices },
                empty_on_failure: false,
            }],
            roots: vec![0],
        };
        for invalid in [
            make(vec![0.; 8], vec![0, 1, 2]),
            make(vec![0.; 9], vec![0, 1]),
            make(vec![0.; 9], vec![0, 1, 3]),
            make(vec![f64::NAN; 9], vec![0, 1, 2]),
        ] {
            assert!(invalid.validate().is_err());
        }
        let valid = make(vec![0., 0., 0., 1., 0., 0., 0., 1., 0.], vec![0, 1, 2]);
        valid.validate().unwrap();
        #[cfg(feature = "codec")]
        {
            use value_codec::{Deserialize, Serialize};
            assert_eq!(Program::from_value(valid.to_value()).unwrap(), valid);
        }
        let huge = make(vec![0.; 300_001 * 3], vec![]);
        assert!(huge.validate().is_err());
        let mut arena = huge.nodes;
        arena.push(Node::Empty);
        assert!(Program::from_roots(&arena, &[1]).is_ok());
    }
}
