use super::*;

pub struct CircularBlendSpan {
    pub centers: Curve,
    pub plane_contact: Curve,
    pub cylinder_contact: Curve,
    pub surface: Surface,
    pub(super) radius_law: [f64; 4],
    pub(super) cylinder_height: f64,
    pub(super) cylinder_radius: f64,
}

/// Radius-stage diagnostics are retained after its private proof is transferred
/// into normal qualification. A successful envelope owns both original proofs.
pub struct CircularEnvelopeReport {
    pub radius: nurbs_core::moving_radius::Report,
    pub normals: Option<nurbs_core::moving_envelope::Report>,
}

pub struct CircularBlendBoundary {
    pub curve: Curve,
    pub pcurve: Curve,
    pub collapsed_pole: Option<[f64; 3]>,
}

impl CircularBlendSpan {
    /// Original cubic radius coefficients for independent body-face admission.
    pub fn radius_curve(&self) -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: self.radius_law.iter().map(|&r| vec![r, 0.]).collect(),
            weights: vec![1.; 4],
            periodic: false,
        }
    }
    /// Qualify the full original support patch against its authored moving
    /// center and cubic radius law. Does not admit a sewn fillet body.
    pub fn qualify_radius(
        &self,
        tolerance_mm: f64,
        max_cells: usize,
        max_work: u64,
    ) -> nurbs_core::Result<nurbs_core::moving_radius::Report> {
        let radius = self.radius_curve();
        nurbs_core::moving_radius::qualify(
            &self.surface,
            &self.centers,
            &radius,
            tolerance_mm,
            max_cells,
            max_work,
        )
    }
    /// Recheck radial/normal agreement on the original support after a fresh
    /// whole-patch radius check. The two qualification stages have separate budgets.
    pub fn qualify_envelope(
        &self,
        tolerance_mm: f64,
        radius_cells: usize,
        radius_work: u64,
        limits: nurbs_core::moving_envelope::Limits,
    ) -> nurbs_core::Result<CircularEnvelopeReport> {
        let mut radius = self.qualify_radius(tolerance_mm, radius_cells, radius_work)?;
        let normals = radius
            .certificate
            .take()
            .map(|proof| nurbs_core::moving_envelope::qualify(proof, limits))
            .transpose()?;
        Ok(CircularEnvelopeReport { radius, normals })
    }
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

    pub(super) fn annular_sheets(
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

pub(super) fn trimmed_cylinder(
    rail: &Curve,
    height: f64,
    _radius_law: [f64; 4],
    tolerance_mm: f64,
) -> Result<crate::Model> {
    if !height.is_finite()||height<=0.||rail.control_points.iter().any(|p|p[2]<=0.||p[2]>height){return Err(invalid("Cylinder contact rail must stay strictly above the bottom and below its declared height"));}
    let surface = Surface {
        degree_u: rail.degree,
        degree_v: 1,
        knots_u: rail.knots.iter().map(|k|3.*k).collect(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: rail
            .control_points
            .iter()
            .map(|p| vec![vec![p[0], p[1], 0.], p.clone()])
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
    // A ruled cylinder chart between the original bottom and contact rail.
    // Its top is the authored rational curve itself, so no rounded polynomial
    // height law or U-coordinate degree elevation enters boundary agreement.
    let top_uv=line(vec![0.,1.],vec![3.,1.]);
    let curves = [
        bottom,
        line(vec![b[0], b[1], 0.], b.clone()),
        rail.reverse()?,
        line(a.clone(), vec![a[0], a[1], 0.]),
    ];
    let uv = [
        line(vec![0., 0.], vec![3., 0.]),
        line(vec![3., 0.], vec![3., 1.]),
        top_uv.reverse()?,
        line(vec![0., 1.], vec![0., 0.]),
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

pub(super) fn trimmed_plane(
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
    // Enclose every authored outer control without rounding a UV inverse.
    // Rational arc controls can exceed nominal +/-R by a binary64 ulp.
    let bounds=std::array::from_fn::<_,2,_>(|axis|outer.control_points.iter().fold([-radius,radius],|r,p|[r[0].min(p[axis]),r[1].max(p[axis])]));
    let surface = Surface {
        degree_u: 1,degree_v: 1,
        knots_u: vec![bounds[0][0],bounds[0][0],bounds[0][1],bounds[0][1]],
        knots_v: vec![bounds[1][0],bounds[1][0],bounds[1][1],bounds[1][1]],
        control_points: (0..2).map(|u|(0..2).map(|v|vec![bounds[0][u],bounds[1][v],height]).collect()).collect(),
        weights: vec![vec![1.,1.],vec![1.,1.]],periodic_u:false,periodic_v:false,
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
                        // Identity plane chart: retain authored X/Y exactly.
                        // A rounded normalization followed by its inverse
                        // otherwise changes the lifted rational boundary.
                        vec![p[0], p[1]]
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

pub(crate) fn open_face(
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
        coedges.push(crate::Coedge::new(
            i,
            false,
            boundary.pcurve,
        ));
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

pub(crate) fn assemble_support_sheets(sheets: Vec<(crate::Model, bool)>) -> Result<crate::Model> {
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
            coedges.push(crate::Coedge::new(
                index,
                edge_reversed,
                use_.pcurve.clone(),
            ));
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

pub(crate) fn angular_unit(angle: f64) -> [f64; 2] {
    let quadrant = angle / std::f64::consts::FRAC_PI_2;
    if quadrant.is_finite() && quadrant.fract() == 0. {
        [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]][quadrant.rem_euclid(4.) as usize]
    } else {
        [angle.cos(), angle.sin()]
    }
}

pub(super) fn arc(radius: f64, z: f64, start: f64, sweep: f64) -> Curve {
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
                radius * (middle.cos() / weight),
                radius * (middle.sin() / weight),
                z,
            ],
            vec![radius * last[0], radius * last[1], z],
        ],
        weights: vec![1., weight, 1.],
        periodic: false,
    }
}
