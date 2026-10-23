use super::*;

/// Loft two homologous closed curve loops at z0/z1 into a closed solid.
/// Bottom and top must share degree/knot/weight structure per span so side
/// patches remain exact rational ruled surfaces (cylinders or cones).
pub(super) fn loft_curve_loops(
    bottom_curves: &[Curve],
    top_curves: &[Curve],
    z0: f64,
    z1: f64,
    tolerance: f64,
    sources: &[&Model],
) -> Result<Model> {
    let n = bottom_curves.len();
    if n < 2 || z1 <= z0 || top_curves.len() != n {
        return Err(refuse(
            "Curve-loop loft requires homologous closed loops and positive height",
        ));
    }
    for (bottom, top) in bottom_curves.iter().zip(top_curves) {
        if bottom.degree != top.degree
            || bottom.knots != top.knots
            || bottom.weights != top.weights
            || bottom.control_points.len() != top.control_points.len()
        {
            return Err(refuse(
                "Curve-loop loft requires matching span structure on bottom and top",
            ));
        }
    }
    let bottom_points: Vec<[f64; 3]> = bottom_curves
        .iter()
        .map(|curve| [curve.control_points[0][0], curve.control_points[0][1], z0])
        .collect();
    let top_points: Vec<[f64; 3]> = top_curves
        .iter()
        .map(|curve| [curve.control_points[0][0], curve.control_points[0][1], z1])
        .collect();
    let mut vertices: Vec<Vertex> = bottom_points
        .iter()
        .chain(&top_points)
        .map(|point| Vertex { point: *point })
        .collect();
    let mut edges = Vec::with_capacity(n * 3);
    let mut bottom_edges = Vec::with_capacity(n);
    let mut top_edges = Vec::with_capacity(n);
    let mut vertical_edges = Vec::with_capacity(n);
    for i in 0..n {
        let j = (i + 1) % n;
        bottom_edges.push(edges.len());
        edges.push(Edge {
            degenerate: false,
            vertices: [i, j],
            curve: bottom_curves[i].clone(),
        });
        top_edges.push(edges.len());
        edges.push(Edge {
            degenerate: false,
            vertices: [n + i, n + j],
            curve: top_curves[i].clone(),
        });
    }
    for i in 0..n {
        vertical_edges.push(edges.len());
        edges.push(Edge {
            degenerate: false,
            vertices: [i, n + i],
            curve: crate::line(bottom_points[i].to_vec(), top_points[i].to_vec()),
        });
    }
    let mut loops = Vec::new();
    let mut faces = Vec::new();
    let mut shell_faces = Vec::new();
    for i in 0..n {
        let j = (i + 1) % n;
        let bottom_curve = &bottom_curves[i];
        let top_curve = &top_curves[i];
        let outer = loops.len();
        loops.push(Loop {
            coedges: vec![
                Coedge {
                    edge: bottom_edges[i],
                    reversed: false,
                    pcurve: crate::line(vec![0., 0.], vec![1., 0.]),
                },
                Coedge {
                    edge: vertical_edges[j],
                    reversed: false,
                    pcurve: crate::line(vec![1., 0.], vec![1., 1.]),
                },
                Coedge {
                    edge: top_edges[i],
                    reversed: true,
                    pcurve: crate::line(vec![1., 1.], vec![0., 1.]),
                },
                Coedge {
                    edge: vertical_edges[i],
                    reversed: true,
                    pcurve: crate::line(vec![0., 1.], vec![0., 0.]),
                },
            ],
        });
        let face = faces.len();
        faces.push(Face {
            surface: Surface {
                degree_u: bottom_curve.degree,
                degree_v: 1,
                knots_u: bottom_curve.knots.clone(),
                knots_v: vec![0., 0., 1., 1.],
                control_points: (0..bottom_curve.control_points.len())
                    .map(|k| {
                        vec![
                            bottom_curve.control_points[k].clone(),
                            top_curve.control_points[k].clone(),
                        ]
                    })
                    .collect(),
                weights: bottom_curve
                    .weights
                    .iter()
                    .map(|weight| vec![*weight, *weight])
                    .collect(),
                periodic_u: false,
                periodic_v: false,
            },
            outer,
            holes: vec![],
        });
        shell_faces.push(FaceUse {
            face,
            reversed: false,
        });
    }
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for curve in bottom_curves.iter().chain(top_curves) {
        for point in &curve.control_points {
            min[0] = min[0].min(point[0]);
            min[1] = min[1].min(point[1]);
            max[0] = max[0].max(point[0]);
            max[1] = max[1].max(point[1]);
        }
    }
    if !(max[0] > min[0] && max[1] > min[1]) {
        return Err(refuse("Curve-loop loft caps require a positive XY span"));
    }
    let cap_surface = |z: f64| Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![min[0], min[1], z], vec![min[0], max[1], z]],
            vec![vec![max[0], min[1], z], vec![max[0], max[1], z]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    for (z, edge_ids, curves, reversed) in [
        (z0, &bottom_edges, bottom_curves, true),
        (z1, &top_edges, top_curves, false),
    ] {
        let outer = loops.len();
        loops.push(Loop {
            coedges: edge_ids
                .iter()
                .enumerate()
                .map(|(i, edge)| {
                    let curve = curves[i].clone();
                    let pcurve = Curve {
                        control_points: curve
                            .control_points
                            .iter()
                            .map(|p| {
                                vec![
                                    (p[0] - min[0]) / (max[0] - min[0]),
                                    (p[1] - min[1]) / (max[1] - min[1]),
                                ]
                            })
                            .collect(),
                        ..curve
                    };
                    Coedge {
                        edge: *edge,
                        reversed: false,
                        pcurve,
                    }
                })
                .collect(),
        });
        let face = faces.len();
        faces.push(Face {
            surface: cap_surface(z),
            outer,
            holes: vec![],
        });
        shell_faces.push(FaceUse { face, reversed });
    }
    assemble_imprint_solid(
        std::mem::take(&mut vertices),
        edges,
        loops,
        faces,
        shell_faces,
        tolerance,
        sources,
    )
}

pub(super) fn extrude_arc_loop(
    arcs: &[CircleArc],
    z0: f64,
    z1: f64,
    tolerance: f64,
    sources: &[&Model],
) -> Result<Model> {
    let curves: Vec<Curve> = arcs.iter().map(|arc| arc_curve(*arc, z0)).collect();
    extrude_curve_loop(&curves, z0, z1, tolerance, sources)
}

/// Exact AF-01 authoring for vertical cuboid edges. The XY boundary is rebuilt
/// from line segments and positive-weight quarter-circle arcs, then extruded
/// through the shared closed-shell assembly path.
pub(crate) fn rounded_cuboid_vertical_edges(
    source: &Model,
    min: [f64; 3],
    max: [f64; 3],
    rounded: [bool; 4],
    radius: f64,
) -> Result<Model> {
    variable_radius_cuboid_vertical_edges(source, min, max, rounded, radius, radius)
}

/// Linear radius law along +Z for selected vertical cuboid corners. Endpoint
/// radii must be positive and fit the profile; equal radii recover the cylinder
/// AF-01 cell as a cone-of-zero-taper special case.
pub(crate) fn variable_radius_cuboid_vertical_edges(
    source: &Model,
    min: [f64; 3],
    max: [f64; 3],
    rounded: [bool; 4],
    radius_bottom: f64,
    radius_top: f64,
) -> Result<Model> {
    independent_radius_cuboid_vertical_edges(
        source,
        min,
        max,
        rounded,
        [radius_bottom; 4],
        [radius_top; 4],
    )
}

/// Independent linear radius laws for the four longitudinal corners. Adjacent
/// setbacks are checked at both ends; their sum is affine along the whole edge.
/// This constructs a prism family, not a valence-three spherical corner.
pub(crate) fn independent_radius_cuboid_vertical_edges(
    source: &Model,
    min: [f64; 3],
    max: [f64; 3],
    rounded: [bool; 4],
    radius_bottom: [f64; 4],
    radius_top: [f64; 4],
) -> Result<Model> {
    if !rounded.iter().any(|&x| x)
        || (0..3).any(|i| !min[i].is_finite() || !max[i].is_finite() || max[i] <= min[i])
    {
        return Err(refuse(
            "Independent-radius cuboid needs finite positive extents and selected corners",
        ));
    }
    for radii in [radius_bottom, radius_top] {
        if (0..4).any(|i| rounded[i] && (!radii[i].is_finite() || radii[i] <= 0.)) {
            return Err(refuse("Selected corner radii must be finite and positive"));
        }
        for i in 0..4 {
            let next = (i + 1) % 4;
            let extent = max[i % 2] - min[i % 2];
            let a = if rounded[i] { radii[i] } else { 0. };
            let b = if rounded[next] { radii[next] } else { 0. };
            if !((extent - a) - b > source.tolerance_mm) {
                return Err(refuse(
                    "Adjacent corner radii consume their shared support edge",
                ));
            }
        }
    }
    let profile_at = |radii: [f64; 4], z: f64| -> Result<Vec<Curve>> {
        let corners = [
            [min[0], min[1]],
            [max[0], min[1]],
            [max[0], max[1]],
            [min[0], max[1]],
        ];
        let entry = [
            [min[0], min[1] + radii[0]],
            [max[0] - radii[1], min[1]],
            [max[0], max[1] - radii[2]],
            [min[0] + radii[3], max[1]],
        ];
        let exit = [
            [min[0] + radii[0], min[1]],
            [max[0], min[1] + radii[1]],
            [max[0] - radii[2], max[1]],
            [min[0], max[1] - radii[3]],
        ];
        let centers = [
            [min[0] + radii[0], min[1] + radii[0]],
            [max[0] - radii[1], min[1] + radii[1]],
            [max[0] - radii[2], max[1] - radii[2]],
            [min[0] + radii[3], max[1] - radii[3]],
        ];
        let starts = [
            std::f64::consts::PI,
            -std::f64::consts::FRAC_PI_2,
            0.,
            std::f64::consts::FRAC_PI_2,
        ];
        let effective_entry: Vec<[f64; 2]> = (0..4)
            .map(|i| if rounded[i] { entry[i] } else { corners[i] })
            .collect();
        let effective_exit: Vec<[f64; 2]> = (0..4)
            .map(|i| if rounded[i] { exit[i] } else { corners[i] })
            .collect();
        let mut curves = Vec::new();
        for i in 0..4 {
            if rounded[i] {
                curves.push(arc_curve(
                    CircleArc {
                        center: centers[i],
                        radius: radii[i],
                        start: starts[i],
                        sweep: std::f64::consts::FRAC_PI_2,
                    },
                    z,
                ));
            }
            let next = (i + 1) % 4;
            let a = effective_exit[i];
            let b = effective_entry[next];
            curves.push(crate::line(vec![a[0], a[1], z], vec![b[0], b[1], z]));
        }
        Ok(curves)
    };
    let bottom = profile_at(radius_bottom, min[2])?;
    let top = profile_at(radius_top, max[2])?;
    loft_curve_loops(
        &bottom,
        &top,
        min[2],
        max[2],
        source.tolerance_mm,
        &[source],
    )
}


/// Equal-radius valence-3 blend at the axis-aligned cuboid max corner:

pub(super) const ARC_WEIGHT: f64 = std::f64::consts::FRAC_1_SQRT_2;
