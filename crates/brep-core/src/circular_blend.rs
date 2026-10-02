//! Exact support geometry for a convex plane/cylinder rim blend.
//! Authors support patches, contact rails and open B-rep sheets, not sewn solids.
use crate::{Result, invalid};
use nurbs_core::{curve::Curve, surface::Surface};

pub struct CircularBlendSpan {
    pub centers: Curve,
    pub plane_contact: Curve,
    pub cylinder_contact: Curve,
    pub surface: Surface,
    radius_law: [f64; 4],
    cylinder_height: f64,
    cylinder_radius: f64,
}

pub struct CircularBlendBoundary {
    pub curve: Curve,
    pub pcurve: Curve,
    pub collapsed_pole: Option<[f64; 3]>,
}

impl CircularBlendSpan {
    /// Assemble one open B-rep face. This intentionally has no volume body;
    /// the pole edge retains a full UV boundary while its 3D curve is constant.
    pub fn to_open_sheet(&self, tolerance_mm: f64) -> Result<crate::Model> {
        open_face(self.surface.clone(), self.boundaries()?, tolerance_mm)
    }

    /// Retained cylinder strip trimmed below the blend contact rail. The
    /// cubic UV height law has the same parameter as the authored 3D rail.
    pub fn trimmed_cylinder_sheet(&self, tolerance_mm: f64) -> Result<crate::Model> {
        trimmed_cylinder(
            &self.cylinder_contact,
            self.cylinder_height,
            self.radius_law,
            tolerance_mm,
        )
    }

    /// Retained planar cap sector between the blend rail and a circular inner
    /// boundary. Curves map affinely to the planar chart without approximation.
    pub fn trimmed_plane_sheet(
        &self,
        inner_radius: f64,
        tolerance_mm: f64,
    ) -> Result<crate::Model> {
        trimmed_plane(
            &self.plane_contact,
            &self.cylinder_contact,
            self.cylinder_radius,
            self.cylinder_height,
            self.radius_law,
            inner_radius,
            tolerance_mm,
        )
    }

    /// Combine the blend and its two retained neighbor sectors. Still open at
    /// the inner/bottom boundaries and angular sides; this is not a volume body.
    pub fn trimmed_support_region(
        &self,
        inner_radius: f64,
        tolerance_mm: f64,
    ) -> Result<crate::Model> {
        let p = &self.cylinder_contact.control_points;
        let positive = p[0][0] * p[1][1] - p[0][1] * p[1][0] > 0.;
        assemble_support_sheets(vec![
            (self.to_open_sheet(tolerance_mm)?, positive),
            (self.trimmed_plane_sheet(inner_radius, tolerance_mm)?, false),
            (self.trimmed_cylinder_sheet(tolerance_mm)?, !positive),
        ])
    }

    /// Add the inner cylindrical wall and bottom cap. Only the two angular
    /// cuts stay open; these five faces surround the complete annular section.
    pub fn annular_side_sector(
        &self,
        inner_radius: f64,
        tolerance_mm: f64,
    ) -> Result<crate::Model> {
        assemble_support_sheets(self.annular_sheets(inner_radius, tolerance_mm)?)
    }

    fn annular_sheets(
        &self,
        inner_radius: f64,
        tolerance_mm: f64,
    ) -> Result<Vec<(crate::Model, bool)>> {
        let rail = &self.cylinder_contact;
        let radius = self.cylinder_radius;
        let height = self.cylinder_height;
        let p = &rail.control_points;
        let positive = p[0][0] * p[1][1] - p[0][1] * p[1][0] > 0.;
        let inner = Curve {
            control_points: p
                .iter()
                .map(|p| {
                    vec![
                        p[0] * inner_radius / radius,
                        p[1] * inner_radius / radius,
                        height,
                    ]
                })
                .collect(),
            ..rail.clone()
        };
        let bottom = Curve {
            control_points: p.iter().map(|p| vec![p[0], p[1], 0.]).collect(),
            ..rail.clone()
        };
        Ok(vec![
            (self.to_open_sheet(tolerance_mm)?, positive),
            (self.trimmed_plane_sheet(inner_radius, tolerance_mm)?, false),
            (self.trimmed_cylinder_sheet(tolerance_mm)?, !positive),
            (
                trimmed_cylinder(&inner, height, [0.; 4], tolerance_mm)?,
                positive,
            ),
            (
                trimmed_plane(
                    &bottom,
                    rail,
                    radius,
                    0.,
                    [0.; 4],
                    inner_radius,
                    tolerance_mm,
                )?,
                true,
            ),
        ])
    }

    /// Four oriented chart boundaries. A collapsed endpoint is certified by
    /// its complete control row using the same pole check as Model::validate.
    pub fn boundaries(&self) -> Result<[CircularBlendBoundary; 4]> {
        self.surface.validate()?;
        let meridian = |row: usize| Curve {
            degree: self.surface.degree_v,
            knots: self.surface.knots_v.clone(),
            control_points: self.surface.control_points[row].clone(),
            weights: self.surface.weights[row].clone(),
            periodic: false,
        };
        let last = self.surface.control_points.len() - 1;
        let curves = [
            self.plane_contact.clone(),
            meridian(last),
            self.cylinder_contact.reverse()?,
            meridian(0).reverse()?,
        ];
        let uv = [
            ([0., 0.], [1., 0.]),
            ([1., 0.], [1., 1.]),
            ([1., 1.], [0., 1.]),
            ([0., 1.], [0., 0.]),
        ];
        let mut result = Vec::new();
        for (curve, (a, b)) in curves.into_iter().zip(uv) {
            curve.validate()?;
            let pcurve = Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![a.to_vec(), b.to_vec()],
                weights: vec![1., 1.],
                periodic: false,
            };
            let first = &curve.control_points[0];
            let collapsed_pole = curve
                .control_points
                .iter()
                .all(|p| p == first)
                .then(|| [first[0], first[1], first[2]]);
            if let Some(pole) = collapsed_pole {
                crate::validate_pole_boundary(&self.surface, &pcurve, pole)?;
            }
            result.push(CircularBlendBoundary {
                curve,
                pcurve,
                collapsed_pole,
            });
        }
        Ok(result.try_into().ok().unwrap())
    }
}

fn trimmed_cylinder(
    rail: &Curve,
    height: f64,
    radius_law: [f64; 4],
    tolerance_mm: f64,
) -> Result<crate::Model> {
    let surface = Surface {
        degree_u: rail.degree,
        degree_v: 1,
        knots_u: rail.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: rail
            .control_points
            .iter()
            .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], height]])
            .collect(),
        weights: rail.weights.iter().map(|w| vec![*w, *w]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let bottom = Curve {
        control_points: rail
            .control_points
            .iter()
            .map(|p| vec![p[0], p[1], 0.])
            .collect(),
        ..rail.clone()
    };
    let a = rail.control_points.first().unwrap();
    let b = rail.control_points.last().unwrap();
    let line = |a: Vec<f64>, b: Vec<f64>| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a, b],
        weights: vec![1., 1.],
        periodic: false,
    };
    let top_uv = if radius_law.iter().all(|r| *r == radius_law[0]) {
        let z = (height - radius_law[0]) / height;
        line(vec![0., z], vec![1., z])
    } else {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: radius_law
                .iter()
                .enumerate()
                .map(|(i, r)| vec![i as f64 / 3., (height - r) / height])
                .collect(),
            weights: vec![1.; 4],
            periodic: false,
        }
    };
    let curves = [
        bottom,
        line(vec![b[0], b[1], 0.], b.clone()),
        rail.reverse()?,
        line(a.clone(), vec![a[0], a[1], 0.]),
    ];
    let uv = [
        line(vec![0., 0.], vec![1., 0.]),
        line(vec![1., 0.], vec![1., b[2] / height]),
        top_uv.reverse()?,
        line(vec![0., a[2] / height], vec![0., 0.]),
    ];
    let boundaries: Vec<_> = curves
        .into_iter()
        .zip(uv)
        .map(|(curve, pcurve)| CircularBlendBoundary {
            curve,
            pcurve,
            collapsed_pole: None,
        })
        .collect();
    open_face(surface, boundaries.try_into().ok().unwrap(), tolerance_mm)
}

fn trimmed_plane(
    outer: &Curve,
    angular: &Curve,
    radius: f64,
    height: f64,
    radius_law: [f64; 4],
    inner_radius: f64,
    tolerance_mm: f64,
) -> Result<crate::Model> {
    let maximum = radius_law.iter().copied().fold(0., f64::max);
    if !inner_radius.is_finite() || inner_radius <= 0. || inner_radius >= radius - maximum {
        return Err(invalid(
            "Planar blend trim requires an inner circle strictly inside the contact rail",
        ));
    }
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![
                vec![-radius, -radius, height],
                vec![-radius, radius, height],
            ],
            vec![vec![radius, -radius, height], vec![radius, radius, height]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let inner = Curve {
        control_points: angular
            .control_points
            .iter()
            .map(|p| {
                vec![
                    p[0] * inner_radius / radius,
                    p[1] * inner_radius / radius,
                    height,
                ]
            })
            .collect(),
        ..angular.clone()
    };
    let a = outer.control_points.first().unwrap();
    let b = outer.control_points.last().unwrap();
    let ia = inner.control_points.first().unwrap();
    let ib = inner.control_points.last().unwrap();
    let line = |a: Vec<f64>, b: Vec<f64>| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a, b],
        weights: vec![1., 1.],
        periodic: false,
    };
    let curves = [
        outer.clone(),
        line(b.clone(), ib.clone()),
        inner.reverse()?,
        line(ia.clone(), a.clone()),
    ];
    let mut boundaries: Vec<_> = curves
        .into_iter()
        .map(|curve| {
            let pcurve = Curve {
                control_points: curve
                    .control_points
                    .iter()
                    .map(|p| {
                        vec![
                            (p[0] + radius) / (2. * radius),
                            (p[1] + radius) / (2. * radius),
                        ]
                    })
                    .collect(),
                ..curve.clone()
            };
            CircularBlendBoundary {
                curve,
                pcurve,
                collapsed_pole: None,
            }
        })
        .collect();
    let angular_a = &angular.control_points[0];
    let angular_b = &angular.control_points[1];
    if angular_a[0] * angular_b[1] - angular_a[1] * angular_b[0] < 0. {
        boundaries.reverse();
        for boundary in &mut boundaries {
            boundary.curve = boundary.curve.reverse()?;
            boundary.pcurve = boundary.pcurve.reverse()?;
        }
    }
    open_face(surface, boundaries.try_into().ok().unwrap(), tolerance_mm)
}

fn open_face(
    surface: Surface,
    boundaries: [CircularBlendBoundary; 4],
    tolerance_mm: f64,
) -> Result<crate::Model> {
    if !tolerance_mm.is_finite() || tolerance_mm <= 0. {
        return Err(invalid(
            "Circular blend sheet requires positive finite tolerance",
        ));
    }
    let corners: Vec<_> = boundaries
        .iter()
        .map(|b| &b.curve.control_points[0])
        .collect();
    let mut vertices: Vec<crate::Vertex> = Vec::new();
    let mut indices = Vec::new();
    for p in corners {
        let point = [p[0], p[1], p[2]];
        let index = vertices
            .iter()
            .position(|v| v.point == point)
            .unwrap_or_else(|| {
                vertices.push(crate::Vertex { point });
                vertices.len() - 1
            });
        indices.push(index);
    }
    let mut edges = Vec::new();
    let mut coedges = Vec::new();
    for (i, boundary) in boundaries.into_iter().enumerate() {
        edges.push(crate::Edge {
            vertices: [indices[i], indices[(i + 1) % 4]],
            curve: boundary.curve,
            degenerate: boundary.collapsed_pole.is_some(),
        });
        coedges.push(crate::Coedge {
            edge: i,
            reversed: false,
            pcurve: boundary.pcurve,
        });
    }
    let mut model = crate::Model(
        brep_topology::Model {
            vertices,
            edges,
            loops: vec![crate::Loop { coedges }],
            faces: vec![crate::Face {
                surface,
                outer: 0,
                holes: vec![],
            }],
            shells: vec![crate::Shell {
                faces: vec![crate::FaceUse {
                    face: 0,
                    reversed: false,
                }],
                closed: false,
            }],
            bodies: vec![],
            tolerance_mm,
        },
        crate::TopologyIds::default(),
    );
    model.rebuild_topology_ids();
    model.validate()?;
    Ok(model)
}

fn assemble_support_sheets(sheets: Vec<(crate::Model, bool)>) -> Result<crate::Model> {
    let mut iter = sheets.into_iter();
    let (mut result, reversed) = iter
        .next()
        .ok_or_else(|| invalid("No support sheets to assemble"))?;
    result.0.shells[0].faces[0].reversed = reversed;
    for (sheet, reversed) in iter {
        sheet.validate()?;
        let mut vertex_map = Vec::new();
        for vertex in &sheet.vertices {
            let index = result
                .vertices
                .iter()
                .position(|v| v.point == vertex.point)
                .unwrap_or_else(|| {
                    result.0.vertices.push(vertex.clone());
                    result.vertices.len() - 1
                });
            vertex_map.push(index);
        }
        let mut coedges = Vec::new();
        for use_ in &sheet.loops[0].coedges {
            let edge = &sheet.edges[use_.edge];
            let ends = edge.vertices.map(|v| vertex_map[v]);
            let shared = result.edges.iter().enumerate().find(|(_, e)| {
                !e.degenerate && (e.vertices == ends || e.vertices == [ends[1], ends[0]])
            });
            let (index, edge_reversed) = if let Some((i, existing)) = shared {
                let edge_reversed = existing.vertices != ends;
                let curve = if edge_reversed {
                    edge.curve.reverse()?
                } else {
                    edge.curve.clone()
                };
                if curve.degree != existing.curve.degree
                    || curve.knots != existing.curve.knots
                    || curve.control_points != existing.curve.control_points
                    || curve.weights != existing.curve.weights
                    || curve.periodic != existing.curve.periodic
                {
                    return Err(invalid(
                        "Support seam definitions differ; no tolerance welding admitted",
                    ));
                }
                (i, edge_reversed)
            } else {
                let index = result.edges.len();
                result.0.edges.push(crate::Edge {
                    vertices: ends,
                    curve: edge.curve.clone(),
                    degenerate: edge.degenerate,
                });
                (index, false)
            };
            coedges.push(crate::Coedge {
                edge: index,
                reversed: edge_reversed,
                pcurve: use_.pcurve.clone(),
            });
        }
        let outer = result.loops.len();
        result.0.loops.push(crate::Loop { coedges });
        let face = result.faces.len();
        result.0.faces.push(crate::Face {
            surface: sheet.faces[0].surface.clone(),
            outer,
            holes: vec![],
        });
        result.0.shells[0]
            .faces
            .push(crate::FaceUse { face, reversed });
    }
    result.rebuild_topology_ids();
    result.validate()?;
    Ok(result)
}

fn angular_unit(angle: f64) -> [f64; 2] {
    let quadrant = angle / std::f64::consts::FRAC_PI_2;
    if quadrant.is_finite() && quadrant.fract() == 0. {
        [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]][quadrant.rem_euclid(4.) as usize]
    } else {
        [angle.cos(), angle.sin()]
    }
}

fn arc(radius: f64, z: f64, start: f64, sweep: f64) -> Curve {
    let middle = start + sweep / 2.;
    let weight = (sweep / 2.).cos();
    let first = angular_unit(start);
    let last = angular_unit(start + sweep);
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![radius * first[0], radius * first[1], z],
            vec![
                radius * middle.cos() / weight,
                radius * middle.sin() / weight,
                z,
            ],
            vec![radius * last[0], radius * last[1], z],
        ],
        weights: vec![1., weight, 1.],
        periodic: false,
    }
}

/// Local cylinder Z axis, top plane z=height, positive convex radius.
/// Each span covers at most pi/2, including reversed and seam-crossing arcs.
pub fn plane_cylinder_rim(
    outer_radius: f64,
    height: f64,
    radius: f64,
    start: f64,
    sweep: f64,
) -> Result<Vec<CircularBlendSpan>> {
    if [outer_radius, height, radius, start, sweep]
        .iter()
        .any(|v| !v.is_finite())
        || radius <= 0.
        || outer_radius <= radius
        || height <= radius
        || sweep.abs() < 1e-12
        || sweep.abs() > std::f64::consts::TAU
    {
        return Err(invalid(
            "Circular blend requires a fitting radius and a finite nonzero arc of at most one turn",
        ));
    }
    let count = (sweep.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize;
    let delta = sweep / count as f64;
    let mut spans = Vec::with_capacity(count);
    for i in 0..count {
        let angle = start + i as f64 * delta;
        let unit = arc(1., 0., angle, delta);
        let centers = arc(outer_radius - radius, height - radius, angle, delta);
        let plane_contact = arc(outer_radius - radius, height, angle, delta);
        let cylinder_contact = arc(outer_radius, height - radius, angle, delta);
        // Meridian controls describe an exact quarter circle centered at
        // (outer_radius-radius,height-radius). Tensor product with the angular
        // circle gives the torus patch, with strictly positive product weights.
        let meridian = [
            (outer_radius - radius, height),
            (outer_radius, height),
            (outer_radius, height - radius),
        ];
        let meridian_weights = [1., std::f64::consts::FRAC_1_SQRT_2, 1.];
        let surface = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: unit.knots.clone(),
            knots_v: unit.knots.clone(),
            control_points: unit
                .control_points
                .iter()
                .map(|p| {
                    meridian
                        .iter()
                        .map(|&(r, z)| vec![r * p[0], r * p[1], z])
                        .collect()
                })
                .collect(),
            weights: unit
                .weights
                .iter()
                .map(|w| meridian_weights.iter().map(|v| w * v).collect())
                .collect(),
            periodic_u: false,
            periodic_v: false,
        };
        centers.validate()?;
        plane_contact.validate()?;
        cylinder_contact.validate()?;
        surface.validate()?;
        spans.push(CircularBlendSpan {
            centers,
            plane_contact,
            cylinder_contact,
            surface,
            radius_law: [radius; 4],
            cylinder_height: height,
            cylinder_radius: outer_radius,
        });
    }
    Ok(spans)
}

/// Exact end-transition support with cubic smoothstep radius in rational arc
/// parameter u. This is a cross-section radius law, not a general variable-radius
/// rolling-ball envelope certificate. Zero-radius ends collapse to a point; they require explicit
/// degenerate topology handling before this can participate in a sewn solid.
pub fn plane_cylinder_transition(
    outer_radius: f64,
    height: f64,
    start_radius: f64,
    end_radius: f64,
    start: f64,
    sweep: f64,
) -> Result<CircularBlendSpan> {
    if [outer_radius, height, start_radius, end_radius, start, sweep]
        .iter()
        .any(|v| !v.is_finite())
        || start_radius < 0.
        || end_radius < 0.
        || start_radius.max(end_radius) <= 0.
        || outer_radius <= start_radius.max(end_radius)
        || height <= start_radius.max(end_radius)
        || sweep.abs() < 1e-12
        || sweep.abs() > std::f64::consts::FRAC_PI_2
    {
        return Err(invalid(
            "Circular transition requires fitting nonnegative end radii and a nonzero arc of at most pi/2",
        ));
    }
    let unit = arc(1., 0., start, sweep);
    let law = [start_radius, start_radius, end_radius, end_radius];
    let angular_binomial = [1., 2., 1.];
    let law_binomial = [1., 3., 3., 1.];
    let product_binomial = [1., 5., 10., 10., 5., 1.];
    let meridian = [(0., 1.), (1., 1.), (1., 0.)];
    let meridian_weights = [1., std::f64::consts::FRAC_1_SQRT_2, 1.];
    // Multiply homogeneous angular degree-2 Bernstein polynomials by the
    // degree-3 radius law. All resulting denominator controls are positive.
    let mut controls = vec![vec![vec![0.; 3]; 3]; 6];
    let mut weights = vec![vec![0.; 3]; 6];
    let mut center_controls = vec![vec![0.; 3]; 6];
    let mut center_weights = vec![0.; 6];
    for i in 0..3 {
        for l in 0..4 {
            let k = i + l;
            let coefficient = angular_binomial[i] * law_binomial[l] / product_binomial[k];
            let angular_weight = unit.weights[i] * coefficient;
            let p = &unit.control_points[i];
            let radius = law[l];
            center_weights[k] += angular_weight;
            center_controls[k][0] += angular_weight * p[0] * (outer_radius - radius);
            center_controls[k][1] += angular_weight * p[1] * (outer_radius - radius);
            center_controls[k][2] += angular_weight * (height - radius);
            for j in 0..3 {
                let weight = angular_weight * meridian_weights[j];
                weights[k][j] += weight;
                let radial = outer_radius + radius * (meridian[j].0 - 1.);
                controls[k][j][0] += weight * p[0] * radial;
                controls[k][j][1] += weight * p[1] * radial;
                controls[k][j][2] += weight * (height + radius * (meridian[j].1 - 1.));
            }
        }
    }
    for k in 0..6 {
        for d in 0..3 {
            center_controls[k][d] /= center_weights[k];
        }
        for j in 0..3 {
            for d in 0..3 {
                controls[k][j][d] /= weights[k][j];
            }
        }
    }
    // Author shared endpoint definitions directly, avoiding multiply/divide
    // roundoff in equivalent homogeneous endpoint coefficients.
    for (k, r, angular_index) in [(0, start_radius, 0), (5, end_radius, 2)] {
        let point = &unit.control_points[angular_index];
        center_controls[k] = vec![
            (outer_radius - r) * point[0],
            (outer_radius - r) * point[1],
            height - r,
        ];
        controls[k] = [
            (outer_radius - r, height),
            (outer_radius, height),
            (outer_radius, height - r),
        ]
        .map(|(radial, z)| vec![radial * point[0], radial * point[1], z])
        .to_vec();
        weights[k] = meridian_weights.to_vec();
        if r == 0. {
            controls[k] = vec![center_controls[k].clone(); 3];
        }
    }
    let knots: Vec<_> = std::iter::repeat_n(0., 6)
        .chain(std::iter::repeat_n(1., 6))
        .collect();
    let centers = Curve {
        degree: 5,
        knots: knots.clone(),
        control_points: center_controls,
        weights: center_weights,
        periodic: false,
    };
    let boundary = |j: usize| Curve {
        degree: 5,
        knots: knots.clone(),
        control_points: controls
            .iter()
            .map(|row: &Vec<Vec<f64>>| row[j].clone())
            .collect(),
        weights: weights.iter().map(|row| row[j]).collect(),
        periodic: false,
    };
    let plane_contact = boundary(0);
    let cylinder_contact = boundary(2);
    let surface = Surface {
        degree_u: 5,
        degree_v: 2,
        knots_u: knots,
        knots_v: unit.knots,
        control_points: controls,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    centers.validate()?;
    plane_contact.validate()?;
    cylinder_contact.validate()?;
    surface.validate()?;
    Ok(CircularBlendSpan {
        centers,
        plane_contact,
        cylinder_contact,
        surface,
        radius_law: law,
        cylinder_height: height,
        cylinder_radius: outer_radius,
    })
}

/// Assemble a start transition, constant span and end transition into an open
/// shell. Shared seams must match complete curve definitions exactly; this is
/// authored shared topology, not tolerance welding or a closed-solid claim.
pub fn circular_blend_strip(
    outer_radius: f64,
    height: f64,
    radius: f64,
    start: f64,
    entry_sweep: f64,
    middle_sweep: f64,
    exit_sweep: f64,
    tolerance_mm: f64,
) -> Result<crate::Model> {
    let [entry, middle, exit] = circular_strip_spans(
        outer_radius,
        height,
        radius,
        start,
        entry_sweep,
        middle_sweep,
        exit_sweep,
    )?;
    assemble_support_sheets(vec![
        (entry.to_open_sheet(tolerance_mm)?, false),
        (middle.to_open_sheet(tolerance_mm)?, false),
        (exit.to_open_sheet(tolerance_mm)?, false),
    ])
}

/// Entire three-span region with its retained top and cylindrical neighbors.
/// Its inner, bottom and two angular boundaries still need the remainder of
/// the original body. Shared curves are definition-exact, not mesh-welded.
pub fn circular_blend_retained_strip(
    outer_radius: f64,
    inner_radius: f64,
    height: f64,
    radius: f64,
    start: f64,
    entry_sweep: f64,
    middle_sweep: f64,
    exit_sweep: f64,
    tolerance_mm: f64,
) -> Result<crate::Model> {
    let spans = circular_strip_spans(
        outer_radius,
        height,
        radius,
        start,
        entry_sweep,
        middle_sweep,
        exit_sweep,
    )?;
    let mut sheets = Vec::new();
    for span in spans {
        let p = &span.cylinder_contact.control_points;
        let positive = p[0][0] * p[1][1] - p[0][1] * p[1][0] > 0.;
        sheets.push((span.to_open_sheet(tolerance_mm)?, positive));
        sheets.push((span.trimmed_plane_sheet(inner_radius, tolerance_mm)?, false));
        sheets.push((span.trimmed_cylinder_sheet(tolerance_mm)?, !positive));
    }
    assemble_support_sheets(sheets)
}

/// Five retained side surfaces per span, joined through both end transitions.
/// The two angular section loops remain open; no body is issued here.
pub fn circular_blend_annular_strip(
    outer_radius: f64,
    inner_radius: f64,
    height: f64,
    radius: f64,
    start: f64,
    entry_sweep: f64,
    middle_sweep: f64,
    exit_sweep: f64,
    tolerance_mm: f64,
) -> Result<crate::Model> {
    let spans = circular_strip_spans(
        outer_radius,
        height,
        radius,
        start,
        entry_sweep,
        middle_sweep,
        exit_sweep,
    )?;
    let mut sheets = Vec::new();
    for span in spans {
        sheets.extend(span.annular_sheets(inner_radius, tolerance_mm)?);
    }
    assemble_support_sheets(sheets)
}

/// Prototype closed annular solid with a top-outer quarter-rim blend and
/// smoothstep endpoint transitions. This is topology-valid construction,
/// not a feature certificate, source edit or complete geometric solid audit.
pub fn partial_annular_quarter(
    outer_radius: f64,
    inner_radius: f64,
    height: f64,
    radius: f64,
    direction: f64,
    tolerance_mm: f64,
) -> Result<crate::Model> {
    if direction != 1. && direction != -1. {
        return Err(invalid("Quarter blend direction must be +1 or -1"));
    }
    partial_annular_arc(
        outer_radius,
        inner_radius,
        height,
        radius,
        direction * std::f64::consts::FRAC_PI_2,
        tolerance_mm,
    )
}

/// Prototype annular construction for a signed top-rim sweep up to half a
/// circle. The blend consumes one quarter of the sweep at each transition.
/// Like the quarter wrapper, this does not certify a source-edit feature.
pub fn partial_annular_arc(
    outer_radius: f64,
    inner_radius: f64,
    height: f64,
    radius: f64,
    sweep: f64,
    tolerance_mm: f64,
) -> Result<crate::Model> {
    if !sweep.is_finite() || sweep.abs() < 1e-10 || sweep.abs() > std::f64::consts::PI {
        return Err(invalid(
            "Partial annular sweep must be finite, nonzero and at most pi",
        ));
    }
    let direction = sweep.signum();
    let q = sweep;
    let spans = circular_strip_spans(outer_radius, height, radius, 0., q / 4., q / 2., q / 4.)?;
    let mut endpoint = spans[2]
        .cylinder_contact
        .control_points
        .last()
        .unwrap()
        .clone();
    let mut sheets = Vec::new();
    for span in spans {
        sheets.extend(span.annular_sheets(inner_radius, tolerance_mm)?);
    }
    let remaining = direction * std::f64::consts::TAU - q;
    let count = (remaining.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize;
    let step = remaining / count as f64;
    let mut angle = q;
    for i in 0..count {
        let mut top = arc(outer_radius, height, angle, step);
        // Author common endpoint controls from the preceding boundary. This
        // avoids independently rounded trig endpoints; no tolerance weld.
        top.control_points[0] = endpoint;
        if i + 1 == count {
            *top.control_points.last_mut().unwrap() = vec![outer_radius, 0., height];
        }
        endpoint = top.control_points.last().unwrap().clone();
        angle += step;
        let bottom = Curve {
            control_points: top
                .control_points
                .iter()
                .map(|p| vec![p[0], p[1], 0.])
                .collect(),
            ..top.clone()
        };
        let inner = Curve {
            control_points: top
                .control_points
                .iter()
                .map(|p| {
                    vec![
                        p[0] * inner_radius / outer_radius,
                        p[1] * inner_radius / outer_radius,
                        height,
                    ]
                })
                .collect(),
            ..top.clone()
        };
        sheets.push((
            trimmed_plane(
                &top,
                &top,
                outer_radius,
                height,
                [0.; 4],
                inner_radius,
                tolerance_mm,
            )?,
            false,
        ));
        sheets.push((
            trimmed_cylinder(&top, height, [0.; 4], tolerance_mm)?,
            direction < 0.,
        ));
        sheets.push((
            trimmed_cylinder(&inner, height, [0.; 4], tolerance_mm)?,
            direction > 0.,
        ));
        sheets.push((
            trimmed_plane(
                &bottom,
                &top,
                outer_radius,
                0.,
                [0.; 4],
                inner_radius,
                tolerance_mm,
            )?,
            true,
        ));
    }
    let mut model = assemble_support_sheets(sheets)?;
    if model.validate()?.boundary_edge_count != 0 {
        return Err(invalid(
            "Partial annular construction has unmatched boundaries",
        ));
    }
    model.0.shells[0].closed = true;
    model.0.bodies.push(crate::Body {
        outer_shell: 0,
        inner_shells: vec![],
    });
    model.rebuild_topology_ids();
    model.validate()?;
    Ok(model)
}

fn circular_strip_spans(
    outer_radius: f64,
    height: f64,
    radius: f64,
    start: f64,
    entry_sweep: f64,
    middle_sweep: f64,
    exit_sweep: f64,
) -> Result<[CircularBlendSpan; 3]> {
    let sweeps = [entry_sweep, middle_sweep, exit_sweep];
    if sweeps.iter().any(|s| {
        !s.is_finite()
            || s.abs() < 1e-12
            || s.abs() > std::f64::consts::FRAC_PI_2
            || s.signum() != entry_sweep.signum()
    }) || sweeps.iter().map(|s| s.abs()).sum::<f64>() >= std::f64::consts::TAU
    {
        return Err(invalid(
            "Circular strip requires three finite consistently directed spans of at most pi/2",
        ));
    }
    let middle_start = start + entry_sweep;
    let end_start = middle_start + middle_sweep;
    let entry = plane_cylinder_transition(outer_radius, height, 0., radius, start, entry_sweep)?;
    let middle =
        plane_cylinder_rim(outer_radius, height, radius, middle_start, middle_sweep)?.remove(0);
    let exit = plane_cylinder_transition(outer_radius, height, radius, 0., end_start, exit_sweep)?;
    Ok([entry, middle, exit])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn torus_support_has_exact_radius_and_contact_boundaries() {
        for sweep in [0.31, 2.2, -2.2, std::f64::consts::TAU] {
            let spans = plane_cylinder_rim(20., 6., 1.25, 5.9, sweep).unwrap();
            for span in &spans {
                for i in 0..=20 {
                    let u = i as f64 / 20.;
                    let center = span.centers.evaluate(u).unwrap().point;
                    for j in 0..=20 {
                        let point = span.surface.evaluate(u, j as f64 / 20.).unwrap().point;
                        let distance = (point[0] - center[0])
                            .hypot(point[1] - center[1])
                            .hypot(point[2] - center[2]);
                        assert!((distance - 1.25).abs() < 1e-11);
                    }
                    for (v, rail) in [(0., &span.plane_contact), (1., &span.cylinder_contact)] {
                        let evaluation = span.surface.evaluate(u, v).unwrap();
                        let point = evaluation.point;
                        let normal = evaluation.unit_normal().unwrap();
                        let target = if v == 0. {
                            [0., 0., 1.]
                        } else {
                            [point[0] / 20., point[1] / 20., 0.]
                        };
                        let alignment: f64 = (0..3).map(|k| normal[k] * target[k]).sum();
                        assert!((alignment.abs() - 1.).abs() < 1e-11);
                        let expected = rail.evaluate(u).unwrap().point;
                        for k in 0..3 {
                            assert!((point[k] - expected[k]).abs() < 1e-11);
                        }
                    }
                }
            }
            for pair in spans.windows(2) {
                for v in [0., 0.3, 1.] {
                    let a = pair[0].surface.evaluate(1., v).unwrap().point;
                    let b = pair[1].surface.evaluate(0., v).unwrap().point;
                    assert!((0..3).all(|k| (a[k] - b[k]).abs() < 1e-11));
                }
            }
        }
    }
    #[test]
    fn transition_radius_law_contacts_and_collapsed_end() {
        for (r0, r1) in [(0., 1.25), (1.25, 0.), (0.5, 1.25)] {
            let span = plane_cylinder_transition(20., 6., r0, r1, 5.9, -0.7).unwrap();
            for i in 0..=40 {
                let u = i as f64 / 40.;
                let smooth = 3. * u * u - 2. * u * u * u;
                let radius = r0 + (r1 - r0) * smooth;
                let center = span.centers.evaluate(u).unwrap().point;
                for j in 0..=20 {
                    let point = span.surface.evaluate(u, j as f64 / 20.).unwrap().point;
                    let distance = (point[0] - center[0])
                        .hypot(point[1] - center[1])
                        .hypot(point[2] - center[2]);
                    assert!((distance - radius).abs() < 1e-10);
                }
                let plane = span.surface.evaluate(u, 0.).unwrap();
                let cylinder = span.surface.evaluate(u, 1.).unwrap();
                assert!((plane.point[2] - 6.).abs() < 1e-10);
                assert!((cylinder.point[0].hypot(cylinder.point[1]) - 20.).abs() < 1e-10);
                if radius > 1e-5 {
                    assert!((plane.unit_normal().unwrap()[2].abs() - 1.).abs() < 1e-10);
                    let n = cylinder.unit_normal().unwrap();
                    let alignment = (n[0] * cylinder.point[0] + n[1] * cylinder.point[1]) / 20.;
                    assert!((alignment.abs() - 1.).abs() < 1e-10);
                }
            }
        }
        assert!(plane_cylinder_transition(20., 6., 0., 0., 0., 0.5).is_err());
    }

    #[test]
    fn polar_projection_certifies_constant_torus_chart_from_original_controls() {
        for (outer,height,radius,start,sweep) in [(20.,6.,1.25,0.,std::f64::consts::FRAC_PI_2),(8.,4.,0.5,5.9,0.31),(12.,8.,3.,-1.2,0.7)] {
            for direction in [-1., 1.] {
                let span = plane_cylinder_rim(outer,height,radius,start,direction*sweep).unwrap().remove(0);
                let (sin,cos) = (start+direction*sweep*0.5).sin_cos();
                let basis = [[cos,sin,0.,0.],[-sin,cos,0.,0.],[0.,0.,1.,0.]];
                let proof = nurbs_core::surface_injectivity::certify_polar_projection(&span.surface,basis,16,256).unwrap();
                assert!(proof.is_some(), "polar contraction: {outer}, {radius}, {start}, {direction}: {proof:?}");
                assert!(nurbs_core::surface_injectivity::certify_polar_projection(&span.surface,basis,16,255).unwrap().is_none());
                let mut folded = span.surface.clone();
                folded.control_points[2] = folded.control_points[0].clone();
                folded.weights[2] = folded.weights[0].clone();
                assert!(nurbs_core::surface_injectivity::certify_polar_projection(&folded,basis,16,256).unwrap().is_none());
                let automatic = nurbs_core::surface_injectivity::certify(&span.surface,1000).unwrap();
                assert!(automatic.proven, "automatic polar candidate: {automatic:?}");
                assert!(automatic.polar_projection.is_some());
                assert_eq!(automatic.reason,"global-polar-projection-contraction");
            }
        }
    }

    #[test]
    fn collapsed_transition_tip_has_distinct_normal_limits_not_a_regular_g1_point() {
        // Both contact rails end at the original sharp rim. Their limiting
        // tangent planes differ, even though every nonzero section is tangent
        // to both supports. A collapsed UV edge cannot certify a regular G1 tip.
        for direction in [-1., 1.] {
            for (r0, r1, pole_u) in [(0., 1.25, 0.), (1.25, 0., 1.)] {
                let span = plane_cylinder_transition(20., 6., r0, r1, 0.3, direction * 0.7).unwrap();
                for distance in [0.1, 0.01, 0.001] {
                    let u = if pole_u == 0. { distance } else { 1. - distance };
                    let plane = span.surface.evaluate(u, 0.).unwrap();
                    let cylinder = span.surface.evaluate(u, 1.).unwrap();
                    let np = plane.unit_normal().unwrap();
                    let nc = cylinder.unit_normal().unwrap();
                    let dot: f64 = (0..3).map(|k| np[k] * nc[k]).sum();
                    assert!(dot.abs() < 1e-7, "distinct limiting support normals: {dot}");
                    assert!((np[2].abs() - 1.).abs() < 1e-7);
                    assert!(nc[2].abs() < 1e-7);
                }
                assert!(span.surface.evaluate(pole_u, 0.5).unwrap().unit_normal().is_none());
            }
        }
    }

    #[test]
    fn transition_joins_constant_radius_patch_with_matching_tangent_planes() {
        for direction in [-1., 1.] {
            let transition =
                plane_cylinder_transition(20., 6., 0., 1.25, 0.3, direction * 0.7).unwrap();
            let constant =
                plane_cylinder_rim(20., 6., 1.25, 0.3 + direction * 0.7, direction * 0.4).unwrap();
            for i in 0..=40 {
                let v = i as f64 / 40.;
                let a = transition.surface.evaluate(1., v).unwrap();
                let b = constant[0].surface.evaluate(0., v).unwrap();
                assert!((0..3).all(|k| (a.point[k] - b.point[k]).abs() < 1e-10));
                let na = a.unit_normal().unwrap();
                let nb = b.unit_normal().unwrap();
                let alignment: f64 = (0..3).map(|k| na[k] * nb[k]).sum();
                assert!((alignment - 1.).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn transition_boundaries_certify_poles_and_form_an_oriented_loop() {
        for (r0, r1, pole_index) in [(0., 1.25, 3), (1.25, 0., 1)] {
            let span = plane_cylinder_transition(20., 6., r0, r1, 5.9, 0.7).unwrap();
            let boundaries = span.boundaries().unwrap();
            assert_eq!(
                boundaries
                    .iter()
                    .filter(|b| b.collapsed_pole.is_some())
                    .count(),
                1
            );
            assert!(boundaries[pole_index].collapsed_pole.is_some());
            for i in 0..4 {
                let a = boundaries[i].curve.evaluate(1.).unwrap().point;
                let b = boundaries[(i + 1) % 4].curve.evaluate(0.).unwrap().point;
                assert!((0..3).all(|k| (a[k] - b[k]).abs() < 1e-10));
            }
            let mut bad = span.surface.clone();
            let row = if r0 == 0. { 0 } else { 5 };
            bad.control_points[row][1][0] += 1e-8;
            let boundary = &boundaries[pole_index];
            assert!(
                crate::validate_pole_boundary(
                    &bad,
                    &boundary.pcurve,
                    boundary.collapsed_pole.unwrap()
                )
                .is_err()
            );
        }
    }

    #[test]
    fn transition_open_sheet_validates_pole_incidence_and_uv_agreement() {
        for (r0, r1) in [(0., 1.25), (1.25, 0.), (0.5, 1.25)] {
            let span = plane_cylinder_transition(20., 6., r0, r1, 5.9, -0.7).unwrap();
            let sheet = span.to_open_sheet(1e-7).unwrap();
            let report = sheet.validate().unwrap();
            assert_eq!(report.body_count, 0);
            assert_eq!(report.face_count, 1);
            assert_eq!(
                report.boundary_edge_count,
                if r0 == 0. || r1 == 0. { 3 } else { 4 }
            );
            assert_eq!(
                sheet.vertices.len(),
                if r0 == 0. || r1 == 0. { 3 } else { 4 }
            );
            assert!(
                sheet
                    .edges
                    .iter()
                    .filter(|e| e.degenerate)
                    .all(|e| e.vertices[0] == e.vertices[1])
            );
            let mut invalid_body = sheet.clone();
            invalid_body.0.bodies.push(crate::Body {
                outer_shell: 0,
                inner_shells: vec![],
            });
            invalid_body.rebuild_topology_ids();
            assert!(invalid_body.validate().is_err());
        }
    }

    #[test]
    fn three_span_strip_shares_two_seams_and_keeps_two_poles() {
        for direction in [-1., 1.] {
            let strip = circular_blend_strip(
                20.,
                6.,
                1.25,
                5.9,
                direction * 0.3,
                direction * 0.7,
                direction * 0.4,
                1e-7,
            )
            .unwrap();
            let report = strip.validate().unwrap();
            assert_eq!(report.face_count, 3);
            assert_eq!(report.body_count, 0);
            assert_eq!(strip.edges.len(), 10);
            assert_eq!(strip.vertices.len(), 6);
            assert_eq!(strip.edges.iter().filter(|e| e.degenerate).count(), 2);
            assert_eq!(report.boundary_edge_count, 6);
            let uses: Vec<_> = (0..strip.edges.len())
                .map(|i| {
                    strip
                        .loops
                        .iter()
                        .flat_map(|l| &l.coedges)
                        .filter(|c| c.edge == i)
                        .count()
                })
                .collect();
            assert_eq!(uses.iter().filter(|&&n| n == 2).count(), 2);
        }
    }

    #[test]
    fn cylinder_trim_uses_contact_rail_and_cubic_uv_height() {
        for (r0, r1) in [(0., 1.25), (1.25, 0.), (0.5, 1.25)] {
            let span = plane_cylinder_transition(20., 6., r0, r1, 5.9, -0.7).unwrap();
            let sheet = span.trimmed_cylinder_sheet(1e-7).unwrap();
            assert_eq!(sheet.validate().unwrap().boundary_edge_count, 4);
            let mut bad = sheet.clone();
            bad.0.loops[0].coedges[2].pcurve.control_points[1][1] += 0.01;
            assert!(bad.validate().is_err());
            for i in 0..=40 {
                let t = i as f64 / 40.;
                let uv = sheet.loops[0].coedges[2].pcurve.evaluate(t).unwrap().point;
                let actual = sheet.faces[0].surface.evaluate(uv[0], uv[1]).unwrap().point;
                let expected = sheet.edges[2].curve.evaluate(t).unwrap().point;
                assert!((0..3).all(|k| (actual[k] - expected[k]).abs() < 1e-10));
            }
        }
    }

    #[test]
    fn plane_trim_maps_all_boundaries_and_rejects_contact_overlap() {
        for direction in [-1., 1.] {
            for (r0, r1) in [(0., 1.25), (1.25, 0.), (0.5, 1.25)] {
                let span =
                    plane_cylinder_transition(20., 6., r0, r1, 5.9, direction * 0.7).unwrap();
                let sheet = span.trimmed_plane_sheet(5., 1e-7).unwrap();
                assert_eq!(sheet.validate().unwrap().boundary_edge_count, 4);
                for coedge in &sheet.loops[0].coedges {
                    for i in 0..=40 {
                        let t = i as f64 / 40.;
                        let uv = coedge.pcurve.evaluate(t).unwrap().point;
                        let actual = sheet.faces[0].surface.evaluate(uv[0], uv[1]).unwrap().point;
                        let expected = sheet.edges[coedge.edge].curve.evaluate(t).unwrap().point;
                        assert!((0..3).all(|k| (actual[k] - expected[k]).abs() < 1e-10));
                    }
                }
                assert!(span.trimmed_plane_sheet(19., 1e-7).is_err());
                assert!(span.trimmed_plane_sheet(0., 1e-7).is_err());
            }
        }
    }

    #[test]
    fn neighbor_regions_share_plane_and_cylinder_contact_edges() {
        for direction in [-1., 1.] {
            for (r0, r1) in [(0., 1.25), (1.25, 0.), (0.5, 1.25)] {
                let span =
                    plane_cylinder_transition(20., 6., r0, r1, 5.9, direction * 0.7).unwrap();
                let region = span.trimmed_support_region(5., 1e-7).unwrap();
                let report = region.validate().unwrap();
                assert_eq!(report.face_count, 3);
                assert_eq!(report.body_count, 0);
                assert_eq!(region.edges.len(), 10);
                assert_eq!(
                    report.boundary_edge_count,
                    if r0 == 0. || r1 == 0. { 7 } else { 8 }
                );
                let shared = (0..region.edges.len())
                    .filter(|&i| {
                        region
                            .loops
                            .iter()
                            .flat_map(|l| &l.coedges)
                            .filter(|c| c.edge == i)
                            .count()
                            == 2
                    })
                    .count();
                assert_eq!(shared, 2);
            }
        }
    }

    #[test]
    fn retained_strip_shares_all_internal_seams_and_keeps_only_outer_boundary() {
        for direction in [-1., 1.] {
            let region = circular_blend_retained_strip(
                20.,
                5.,
                6.,
                1.25,
                5.9,
                direction * 0.3,
                direction * 0.7,
                direction * 0.4,
                1e-7,
            )
            .unwrap();
            let report = region.validate().unwrap();
            assert_eq!(report.face_count, 9);
            assert_eq!(report.body_count, 0);
            assert_eq!(region.edges.len(), 24);
            assert_eq!(region.vertices.len(), 14);
            assert_eq!(report.boundary_edge_count, 10);
            assert_eq!(region.edges.iter().filter(|e| e.degenerate).count(), 2);
            let shared = (0..region.edges.len())
                .filter(|&i| {
                    region
                        .loops
                        .iter()
                        .flat_map(|l| &l.coedges)
                        .filter(|c| c.edge == i)
                        .count()
                        == 2
                })
                .count();
            assert_eq!(shared, 12);
            for edge in 0..region.edges.len() {
                let senses: Vec<_> = region.shells[0]
                    .faces
                    .iter()
                    .flat_map(|face_use| {
                        region.loops[region.faces[face_use.face].outer]
                            .coedges
                            .iter()
                            .filter(move |c| c.edge == edge)
                            .map(move |c| c.reversed ^ face_use.reversed)
                    })
                    .collect();
                assert!(senses.len() == 1 || senses.len() == 2);
                if senses.len() == 2 {
                    assert_ne!(senses[0], senses[1]);
                }
            }
        }
    }

    #[test]
    fn annular_sector_has_complete_walls_and_only_two_angular_cuts() {
        for direction in [-1., 1.] {
            for (r0, r1) in [(0., 1.25), (1.25, 0.), (0.5, 1.25)] {
                let span =
                    plane_cylinder_transition(20., 6., r0, r1, 5.9, direction * 0.7).unwrap();
                let sector = span.annular_side_sector(5., 1e-7).unwrap();
                let report = sector.validate().unwrap();
                assert_eq!(report.face_count, 5);
                assert_eq!(report.body_count, 0);
                assert_eq!(sector.edges.len(), 15);
                assert_eq!(
                    report.boundary_edge_count,
                    if r0 == 0. || r1 == 0. { 9 } else { 10 }
                );
                let shared = (0..sector.edges.len())
                    .filter(|&i| {
                        sector
                            .loops
                            .iter()
                            .flat_map(|l| &l.coedges)
                            .filter(|c| c.edge == i)
                            .count()
                            == 2
                    })
                    .count();
                assert_eq!(shared, 5);
            }
        }
    }

    #[test]
    fn annular_strip_keeps_only_the_two_unrounded_end_sections() {
        for direction in [-1., 1.] {
            let strip = circular_blend_annular_strip(
                20.,
                5.,
                6.,
                1.25,
                5.9,
                direction * 0.3,
                direction * 0.7,
                direction * 0.4,
                1e-7,
            )
            .unwrap();
            let report = strip.validate().unwrap();
            assert_eq!(report.face_count, 15);
            assert_eq!(report.body_count, 0);
            assert_eq!(strip.edges.len(), 35);
            assert_eq!(strip.vertices.len(), 18);
            assert_eq!(report.boundary_edge_count, 8);
            assert_eq!(strip.edges.iter().filter(|e| e.degenerate).count(), 2);
            for edge in 0..strip.edges.len() {
                let senses: Vec<_> = strip.shells[0]
                    .faces
                    .iter()
                    .flat_map(|f| {
                        strip.loops[strip.faces[f.face].outer]
                            .coedges
                            .iter()
                            .filter(move |c| c.edge == edge)
                            .map(move |c| c.reversed ^ f.reversed)
                    })
                    .collect();
                assert!(senses.len() == 1 || senses.len() == 2);
                if senses.len() == 2 {
                    assert_ne!(senses[0], senses[1]);
                }
            }
        }
    }

    #[test]
    fn quarter_annular_blend_closes_without_welding_and_has_one_body() {
        for direction in [-1., 1.] {
            let model = partial_annular_quarter(20., 5., 6., 1.25, direction, 1e-7).unwrap();
            let report = model.validate().unwrap();
            assert_eq!(report.body_count, 1);
            assert_eq!(report.boundary_edge_count, 0);
            assert_eq!(model.faces.len(), 27);
            assert_eq!(model.edges.len(), 55);
            assert_eq!(model.vertices.len(), 26);
            assert_eq!(model.edges.iter().filter(|e| e.degenerate).count(), 2);
        }
    }

    #[test]
    fn different_partial_arc_angles_close_with_shared_endpoints() {
        for sweep in [0.31, std::f64::consts::PI / 3., std::f64::consts::PI] {
            for sign in [-1., 1.] {
                let model = partial_annular_arc(20., 5., 6., 1.25, sign * sweep, 1e-7).unwrap();
                let report = model.validate().unwrap();
                assert_eq!(report.body_count, 1);
                assert_eq!(report.boundary_edge_count, 0);
                assert_eq!(model.edges.iter().filter(|e| e.degenerate).count(), 2);
            }
        }
        for sweep in [0., f64::NAN, 4.] {
            assert!(partial_annular_arc(20., 5., 6., 1.25, sweep, 1e-7).is_err());
        }
    }

    #[test]
    fn impossible_and_nonfinite_support_refuses() {
        for radius in [0., -1., 20., f64::NAN] {
            assert!(plane_cylinder_rim(20., 6., radius, 0., 1.).is_err());
        }
        assert!(plane_cylinder_rim(20., 6., 1., 0., 7.).is_err());
    }

    #[test]
    fn source_preview_admits_only_four_outer_rim_arcs_and_preserves_source() {
        let source = crate::tube(20., 5., 6.).unwrap();
        let original = format!("{source:?}");
        let mut admitted = 0;
        for edge in 0..source.edges.len() {
            if let Ok(result) =
                crate::analytic_features::build_partial_annular_preview(&source, edge, 1.25)
            {
                assert_eq!(result.validate().unwrap().boundary_edge_count, 0);
                assert_eq!(result.bodies.len(), 1);
                assert!(result.persistent_naming_complete());
                assert!(result.1.change_set.validate().is_ok());
                let face_changes: Vec<_> = result
                    .1
                    .change_set
                    .changes
                    .iter()
                    .filter(|change| {
                        change.topo_kind == crate::TopoKind::Face && !change.parents.is_empty()
                    })
                    .collect();
                for (index, id) in result.1.faces.iter().enumerate() {
                    let owners: Vec<_> = face_changes
                        .iter()
                        .filter(|change| change.children.contains(id))
                        .collect();
                    if [0, 5, 10].contains(&index) {
                        assert!(owners.is_empty());
                    } else {
                        assert_eq!(owners.len(), 1);
                        assert_eq!(owners[0].parents.len(), 1);
                        assert!(source.1.faces.contains(&owners[0].parents[0]));
                    }
                }
                for source_face in &source.1.faces {
                    assert!(
                        face_changes
                            .iter()
                            .any(|change| change.parents.contains(source_face))
                    );
                }
                assert_eq!(source.1.bodies, result.1.bodies);
                let restored: crate::Model =
                    value_codec::from_str(&value_codec::to_string(&result).unwrap()).unwrap();
                restored.validate().unwrap();
                assert_eq!(restored.1.bodies, source.1.bodies);
                assert!(restored.persistent_naming_complete());
                assert!(result.1.change_set.changes.iter().any(|change| change.kind
                    == crate::ChangeKind::Persisted
                    && change.topo_kind == crate::TopoKind::Body
                    && change.parents == source.1.bodies
                    && change.children == result.1.bodies));
                for (index, vertex) in source.vertices.iter().enumerate() {
                    let target = result
                        .vertices
                        .iter()
                        .position(|candidate| candidate.point == vertex.point)
                        .expect("original sharp endpoints must remain authored vertices");
                    assert_eq!(source.1.vertices[index], result.1.vertices[target]);
                }
                assert_eq!(
                    source
                        .1
                        .edges
                        .iter()
                        .filter(|id| result.1.edges.contains(id))
                        .count(),
                    8
                );
                admitted += 1;
            }
        }
        assert_eq!(admitted, 4);
        assert_eq!(original, format!("{source:?}"));
        assert!(
            crate::analytic_features::build_partial_annular_preview(
                &source,
                source.edges.len(),
                1.25
            )
            .is_err()
        );
    }

    #[test]
    fn source_preview_face_ownership_survives_rigid_placement() {
        let base = crate::tube(20., 5., 6.).unwrap();
        let (sa, ca) = 0.37_f64.sin_cos();
        let (sb, cb) = (-0.61_f64).sin_cos();
        for matrix in [
            [
                [0., 0., 1., 7.],
                [1., 0., 0., -3.],
                [0., 1., 0., 11.],
                [0., 0., 0., 1.],
            ],
            [
                [ca * cb, -sa, ca * sb, 17.],
                [sa * cb, ca, sa * sb, -9.],
                [-sb, 0., cb, 23.],
                [0., 0., 0., 1.],
            ],
        ] {
            let source = crate::transform::affine(&base, matrix).unwrap();
            let original = format!("{source:?}");
            let mut admitted = 0;
            for edge in 0..source.edges.len() {
                if let Ok(result) =
                    crate::analytic_features::build_partial_annular_preview(&source, edge, 1.25)
                {
                    result.validate().unwrap();
                    assert_eq!(source.1.bodies, result.1.bodies);
                    let mut owned = 0;
                    for id in &result.1.faces {
                        let owners: Vec<_> = result
                            .1
                            .change_set
                            .changes
                            .iter()
                            .filter(|c| {
                                c.topo_kind == crate::TopoKind::Face
                                    && !c.parents.is_empty()
                                    && c.children.contains(id)
                            })
                            .collect();
                        if !owners.is_empty() {
                            assert_eq!(owners.len(), 1);
                            assert_eq!(owners[0].parents.len(), 1);
                            assert!(source.1.faces.contains(&owners[0].parents[0]));
                            owned += 1;
                        }
                    }
                    assert_eq!(owned, 24);
                    admitted += 1;
                }
            }
            assert_eq!(admitted, 4);
            assert_eq!(original, format!("{source:?}"));
        }
    }
}
