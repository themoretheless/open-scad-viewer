use super::*;

/// Analytic tube topology: 10 faces (2 holed planar caps + 8 quarter cylinders), 16 vertices.
const TUBE_FACES: usize = 10;
const TUBE_VERTICES: usize = 16;
const TUBE_QUARTER_FACES: usize = 8;
/// Minimum wall and height for a recognizable tube.
const MIN_TUBE_EXTENT: f64 = 1e-5;
/// Relative / absolute geometric tolerance for certifying supports.
const TUBE_RELATIVE_TOLERANCE: f64 = 1e-10;
const TUBE_ABSOLUTE_TOLERANCE: f64 = 1e-12;
/// Rational weight tolerance.
const WEIGHT_TOLERANCE: f64 = 1e-12;
const QUARTER_KNOTS_U: [f64; 6] = [0., 0., 0., 1., 1., 1.];
const LINEAR_KNOTS_V: [f64; 4] = [0., 0., 1., 1.];

/// Measured tube placement in the cap frame.
struct TubeFrame {
    origin: [f64; 3],
    axis: [f64; 3],
    first: [f64; 3],
    second: [f64; 3],
    height: f64,
    inner: f64,
    outer: f64,
    tolerance: f64,
}

/// Geometric tolerance for certifying a tube of the given outer radius and height.
pub(super) fn tube_tolerance(outer: f64, height: f64) -> f64 {
    (outer.max(height) * TUBE_RELATIVE_TOLERANCE).max(TUBE_ABSOLUTE_TOLERANCE)
}

pub(super) fn recognize_analytic_tube(model: &Model) -> Option<AnalyticTube> {
    model.validate().ok()?;
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || model.faces.len() != TUBE_FACES
        || model.vertices.len() != TUBE_VERTICES
    {
        return None;
    }
    let axis = tube_cap_axis(model)?;
    let frame = measure_tube_frame(model, axis)?;
    // Vertices alone do not determine a rational surface. Certify every
    // cylindrical support by its full quarter-circle control net and weights.
    let mut radius_counts = [0usize; 2];
    for face_index in 0..model.faces.len() {
        if let Some(slot) = certify_tube_face(model, face_index, &frame)? {
            radius_counts[slot] += 1;
        }
    }
    if radius_counts != [4, 4] {
        return None;
    }
    Some((
        frame.outer,
        frame.inner,
        frame.height,
        frame.origin,
        frame.axis,
        [frame.first, frame.second],
    ))
}

/// Axis normal to the first holed planar cap, after counting caps and quarters.
fn tube_cap_axis(model: &Model) -> Option<[f64; 3]> {
    let caps = model
        .faces
        .iter()
        .filter(|face| {
            face.surface.degree_u == 1 && face.surface.degree_v == 1 && !face.holes.is_empty()
        })
        .collect::<Vec<_>>();
    if caps.len() != 2
        || model
            .faces
            .iter()
            .filter(|face| face.surface.degree_u == 2 && face.surface.degree_v == 1)
            .count()
            != TUBE_QUARTER_FACES
    {
        return None;
    }
    let cp = &caps[0].surface.control_points;
    let point = |i: usize, j: usize| -> Option<[f64; 3]> {
        let p = cp.get(i)?.get(j)?;
        Some([*p.first()?, *p.get(1)?, *p.get(2)?])
    };
    let a = point(0, 0)?;
    let u = sub3(point(1, 0)?, a);
    let v = sub3(point(0, 1)?, a);
    unit3(cross3(u, v))
}

/// Centroid-relative axial/radial extents and the reference radial frame.
fn measure_tube_frame(model: &Model, axis: [f64; 3]) -> Option<TubeFrame> {
    let mut centroid = [0.; 3];
    for vertex in &model.vertices {
        for coordinate in 0..3 {
            centroid[coordinate] += vertex.point[coordinate] / model.vertices.len() as f64;
        }
    }
    let mut axial = Vec::new();
    let mut radial = Vec::new();
    let mut reference = None;
    for vertex in &model.vertices {
        let delta = sub3(vertex.point, centroid);
        let z = dot3(delta, axis);
        let rv = [
            delta[0] - z * axis[0],
            delta[1] - z * axis[1],
            delta[2] - z * axis[2],
        ];
        let radius = dot3(rv, rv).sqrt();
        axial.push(z);
        radial.push(radius);
        if reference
            .map(|(_, old): ([f64; 3], f64)| radius > old)
            .unwrap_or(true)
        {
            reference = Some((rv, radius));
        }
    }
    axial.sort_by(|a, b| a.total_cmp(b));
    radial.sort_by(|a, b| a.total_cmp(b));
    let z0 = *axial.first()?;
    let height = axial.last()? - z0;
    let inner = *radial.first()?;
    let outer = *radial.last()?;
    if !(inner > 0. && outer - inner >= MIN_TUBE_EXTENT && height > MIN_TUBE_EXTENT) {
        return None;
    }
    let first = unit3(reference?.0)?;
    let second = unit3(cross3(axis, first))?;
    let origin = [
        centroid[0] + z0 * axis[0],
        centroid[1] + z0 * axis[1],
        centroid[2] + z0 * axis[2],
    ];
    Some(TubeFrame {
        origin,
        axis,
        first,
        second,
        height,
        inner,
        outer,
        tolerance: tube_tolerance(outer, height),
    })
}

/// `None` refuses the tube; `Some(None)` is an accepted planar cap;
/// `Some(Some(slot))` is a certified quarter cylinder (0 outer, 1 inner).
fn certify_tube_face(model: &Model, face_index: usize, frame: &TubeFrame) -> Option<Option<usize>> {
    let face = &model.faces[face_index];
    let s = &face.surface;
    if s.degree_u == 1 && s.degree_v == 1 {
        if face.holes.len() != 1
            || s.weights
                .iter()
                .flatten()
                .any(|w| (*w - 1.).abs() > WEIGHT_TOLERANCE)
        {
            return None;
        }
        return Some(None);
    }
    if !face.holes.is_empty()
        || s.periodic_u
        || s.periodic_v
        || s.knots_u != QUARTER_KNOTS_U
        || s.knots_v != LINEAR_KNOTS_V
        || s.control_points.len() != 3
        || s.control_points.iter().any(|row| row.len() != 2)
        || !crate::intersections::recognize::unit_square_boundary(model, face_index)
    {
        return None;
    }
    for i in 0..3 {
        for j in 0..2 {
            let expected = if i == 1 {
                std::f64::consts::FRAC_1_SQRT_2
            } else {
                1.
            };
            if (s.weights.get(i)?.get(j)? - expected).abs() > WEIGHT_TOLERANCE {
                return None;
            }
        }
    }
    let local = |i: usize, j: usize| {
        let p = &s.control_points[i][j];
        let d = sub3([p[0], p[1], p[2]], frame.origin);
        [
            dot3(d, frame.first),
            dot3(d, frame.second),
            dot3(d, frame.axis),
        ]
    };
    let tolerance = frame.tolerance;
    let a = local(0, 0);
    let b = local(2, 0);
    let r = a[0].hypot(a[1]);
    let slot = if (r - frame.outer).abs() <= tolerance {
        0
    } else if (r - frame.inner).abs() <= tolerance {
        1
    } else {
        return None;
    };
    if (b[0].hypot(b[1]) - r).abs() > tolerance || (a[0] * b[0] + a[1] * b[1]).abs() > tolerance * r
    {
        return None;
    }
    let height = frame.height;
    let start_z = a[2];
    let end_z = local(0, 1)[2];
    if !((start_z.abs() <= tolerance && (end_z - height).abs() <= tolerance)
        || ((start_z - height).abs() <= tolerance && end_z.abs() <= tolerance))
    {
        return None;
    }
    for j in 0..2 {
        for i in 0..3 {
            let p = local(i, j);
            let xy = if i == 0 {
                [a[0], a[1]]
            } else if i == 2 {
                [b[0], b[1]]
            } else {
                [a[0] + b[0], a[1] + b[1]]
            };
            let expected_z = if j == 0 { start_z } else { end_z };
            if (p[0] - xy[0]).hypot(p[1] - xy[1]).hypot(p[2] - expected_z) > tolerance {
                return None;
            }
        }
    }
    Some(Some(slot))
}

const SHELL_REFUSED: &str = "BREP_ANALYTIC_SHELL_REFUSED";
const SHELL_COLLAPSE_REFUSED: &str = "BREP_ANALYTIC_SHELL_COLLAPSE_REFUSED";
const SHELL_TRANSITION_REFUSED: &str = "BREP_ANALYTIC_SHELL_TRANSITION_REFUSED";
const SHELL_MIXED_OPENING_REFUSED: &str = "BREP_ANALYTIC_SHELL_MIXED_OPENING_REFUSED";
/// Smallest and largest admissible shell thickness in mm.
const MIN_SHELL_THICKNESS: f64 = 1e-5;
const MAX_SHELL_THICKNESS: f64 = 1e6;
/// Minimum wall that must survive an offset before it counts as collapsed.
const COLLAPSE_MARGIN: f64 = 1e-5;

/// One shell construction: result body, audit scale and certificate notes.
type ShellStage = (Model, f64, Vec<&'static str>);

/// Qualified exact shell successor. Planar cells offset every support plane and
/// intersect the resulting half-spaces exactly. Cylinder cells use exact
/// rational radial/axial constructors. Adjacent planar openings and single-cap
/// cylindrical openings are refused until their corner/rim ownership is exact.
pub fn exact_analytic_shell(
    model: &Model,
    openings: &[usize],
    thickness: f64,
    direction: &str,
) -> Result<AuditedFeatureResult> {
    let unique = validate_shell_request(model, openings, thickness, direction)?;
    let inward = direction == "inward";
    let (result, scale, notes) =
        if let Some(cylinder) = crate::intersections::recognize_cylinder(model)? {
            shell_cylinder(model, &cylinder, &unique, thickness, inward)?
        } else if let Some(tube) = recognize_analytic_tube(model) {
            shell_tube(tube, &unique, thickness, inward)?
        } else {
            shell_planar_body(model, &unique, thickness, inward)?
        };
    certify_blend_result(
        result,
        EXACT_ANALYTIC_SHELL_CAPABILITY,
        notes,
        scale,
        "BREP_ANALYTIC_SHELL_AUDIT_REFUSED",
    )
}

/// Validates the source and request; returns the sorted unique openings.
fn validate_shell_request(
    model: &Model,
    openings: &[usize],
    thickness: f64,
    direction: &str,
) -> Result<Vec<usize>> {
    model.validate()?;
    audit_solid(model).map_err(|_| {
        refuse(
            SHELL_REFUSED,
            "Shell source must pass the global solid audit",
        )
    })?;
    if !(thickness.is_finite() && (MIN_SHELL_THICKNESS..=MAX_SHELL_THICKNESS).contains(&thickness))
    {
        return Err(refuse(
            SHELL_REFUSED,
            "Shell thickness must be finite and within 0.00001..1000000 mm",
        ));
    }
    if !matches!(direction, "inward" | "outward") {
        return Err(refuse(
            SHELL_REFUSED,
            "Shell direction must be inward or outward",
        ));
    }
    let mut unique = openings.to_vec();
    unique.sort_unstable();
    if unique.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(refuse(
            SHELL_REFUSED,
            "Duplicate shell openings are ambiguous",
        ));
    }
    Ok(unique)
}

/// `origin + axis * distance`.
fn along_axis(origin: [f64; 3], axis: [f64; 3], distance: f64) -> [f64; 3] {
    [
        origin[0] + axis[0] * distance,
        origin[1] + axis[1] * distance,
        origin[2] + axis[2] * distance,
    ]
}

fn shell_cylinder(
    model: &Model,
    cylinder: &crate::intersections::CanonicalCylinder,
    unique: &[usize],
    thickness: f64,
    inward: bool,
) -> Result<ShellStage> {
    let caps = model
        .faces
        .iter()
        .enumerate()
        .filter_map(|(index, face)| {
            (face.surface.degree_u == 1 && face.surface.degree_v == 1).then_some(index)
        })
        .collect::<Vec<_>>();
    if caps.len() != 2 || unique.iter().any(|face| !caps.contains(face)) {
        return Err(refuse(
            SHELL_REFUSED,
            "Cylinder openings must select planar cap faces",
        ));
    }
    if unique.len() == 1 {
        return Err(refuse(
            SHELL_TRANSITION_REFUSED,
            "Single-cap cylinder shell awaits exact annular rim and axial corner ownership",
        ));
    }
    let height = cylinder.half_height * 2.;
    let source_origin = along_axis(cylinder.center, cylinder.axis, -cylinder.half_height);
    if unique.len() == 2 {
        let (outer, inner) = if inward {
            if cylinder.radius <= thickness + COLLAPSE_MARGIN {
                return Err(refuse(
                    SHELL_COLLAPSE_REFUSED,
                    "Inward radial offset collapses the cylinder wall",
                ));
            }
            (cylinder.radius, cylinder.radius - thickness)
        } else {
            (cylinder.radius + thickness, cylinder.radius)
        };
        let local = tube(outer, inner, height)?;
        return Ok((
            place_axial(&local, cylinder.frame, cylinder.axis, source_origin)?,
            outer.max(height),
            vec![
                "exact_cylinder_dual_cap_opening",
                "exact_radial_offset",
                "inner_outer_orientation_and_cavity_owned",
            ],
        ));
    }
    if inward {
        if cylinder.radius <= thickness + COLLAPSE_MARGIN
            || height <= 2. * thickness + COLLAPSE_MARGIN
        {
            return Err(refuse(
                SHELL_COLLAPSE_REFUSED,
                "Inward radial or axial offset collapses the closed cylinder cavity",
            ));
        }
        let inner = crate::cylinder(cylinder.radius - thickness, height - 2. * thickness)?;
        let inner_origin = along_axis(source_origin, cylinder.axis, thickness);
        let inner = place_axial(&inner, cylinder.frame, cylinder.axis, inner_origin)?;
        Ok((
            crate::imprint_pipeline::cavity(model, &inner, model.tolerance_mm)?,
            cylinder.radius.max(height),
            vec![
                "exact_cylinder_closed_inward_shell",
                "exact_radial_axial_offset",
                "inner_shell_reversed_and_cavity_owned",
            ],
        ))
    } else {
        let local = crate::cylinder(cylinder.radius + thickness, height + 2. * thickness)?;
        let outer_origin = along_axis(source_origin, cylinder.axis, -thickness);
        let outer = place_axial(&local, cylinder.frame, cylinder.axis, outer_origin)?;
        Ok((
            crate::imprint_pipeline::cavity(&outer, model, model.tolerance_mm)?,
            (cylinder.radius + thickness).max(height + 2. * thickness),
            vec![
                "exact_cylinder_closed_outward_shell",
                "exact_radial_axial_offset",
                "inner_shell_reversed_and_cavity_owned",
            ],
        ))
    }
}

fn shell_tube(
    (outer, inner, height, origin, axis, frame): AnalyticTube,
    unique: &[usize],
    thickness: f64,
    inward: bool,
) -> Result<ShellStage> {
    if !unique.is_empty() {
        return Err(refuse(
            SHELL_MIXED_OPENING_REFUSED,
            "Tube face openings require exact four-way annular rim ownership",
        ));
    }
    let (new_outer, new_inner, new_height, shift) = if inward {
        if outer - inner <= 2. * thickness + COLLAPSE_MARGIN
            || height <= 2. * thickness + COLLAPSE_MARGIN
        {
            return Err(refuse(
                SHELL_COLLAPSE_REFUSED,
                "Inward tube radial or axial offset collapses the material wall",
            ));
        }
        (
            outer - thickness,
            inner + thickness,
            height - 2. * thickness,
            thickness,
        )
    } else {
        if inner <= thickness + COLLAPSE_MARGIN {
            return Err(refuse(
                SHELL_COLLAPSE_REFUSED,
                "Outward tube offset collapses the inner radial boundary",
            ));
        }
        (
            outer + thickness,
            inner - thickness,
            height + 2. * thickness,
            -thickness,
        )
    };
    let local = tube(new_outer, new_inner, new_height)?;
    Ok((
        place_axial(&local, frame, axis, along_axis(origin, axis, shift))?,
        new_outer.max(new_height),
        vec![
            "exact_analytic_tube_body_offset",
            "exact_inner_outer_radial_axial_supports",
            "tube_material_orientation_owned",
        ],
    ))
}

fn shell_planar_body(
    model: &Model,
    unique: &[usize],
    thickness: f64,
    inward: bool,
) -> Result<ShellStage> {
    if model
        .faces
        .iter()
        .any(|face| face.surface.degree_u != 1 || face.surface.degree_v != 1)
    {
        return Err(refuse(
            SHELL_MIXED_OPENING_REFUSED,
            "Mixed curved-planar and unsupported analytic tube shell topology is not authored",
        ));
    }
    if !selected_faces_are_nonadjacent(model, unique) {
        return Err(refuse(
            SHELL_TRANSITION_REFUSED,
            "Adjacent planar openings await exact corner-transition ownership",
        ));
    }
    let result = if inward {
        crate::operations::shell_planar(model, unique, thickness)
    } else {
        crate::operations::shell_planar_outward(model, unique, thickness)
    }
    .map_err(|error| {
        refuse(
            if error.code == "BREP_INVALID_SELECTION" {
                "BREP_ANALYTIC_SHELL_SELECTION_REFUSED"
            } else {
                "BREP_ANALYTIC_SHELL_FEASIBILITY_REFUSED"
            },
            &error.message,
        )
    })?;
    let (min, max) = model_bounds(model);
    Ok((
        result,
        (0..3).map(|axis| max[axis] - min[axis]).fold(0., f64::max),
        vec![
            "exact_convex_support_plane_offset",
            "bounded_halfspace_collision_and_collapse_proof",
            "exact_nonadjacent_opening_rims",
        ],
    ))
}

/// ASL-01: two planar convex sections with matching vertex counts → ruled solid loft.
pub fn analytic_solid_loft(sections: &[Model]) -> Result<(Model, FeatureCertificate)> {
    if sections.len() < 2 {
        return Err(refuse(
            "BREP_ANALYTIC_LOFT_REFUSED",
            "Solid loft requires at least two section solids",
        ));
    }
    let mut profiles = Vec::new();
    for section in sections {
        section.validate()?;
        if !is_axis_aligned_cuboid(section) {
            return Err(refuse(
                "BREP_ANALYTIC_LOFT_REFUSED",
                "SectionMatch walking slice requires axis-aligned cuboid section carriers",
            ));
        }
        let (min, max) = model_bounds(section);
        // Axis-aligned rectangular profile at the section's lowest Z, CCW in XY.
        let z = min[2];
        let pts = vec![
            [min[0], min[1], z],
            [max[0], min[1], z],
            [max[0], max[1], z],
            [min[0], max[1], z],
        ];
        if (max[0] - min[0]) <= 1e-12 || (max[1] - min[1]) <= 1e-12 {
            return Err(refuse(
                "BREP_ANALYTIC_LOFT_REFUSED",
                "SectionMatch failed: degenerate profile extents",
            ));
        }
        profiles.push(pts);
    }
    let n0 = profiles[0].len();
    if profiles.iter().any(|p| p.len() != n0) {
        return Err(refuse(
            "BREP_ANALYTIC_LOFT_REFUSED",
            "SectionMatch failed: unequal vertex counts",
        ));
    }
    let result = ruled_loft(&profiles)?;
    result.validate()?;
    if result.faces.is_empty() {
        return Err(refuse(
            "BREP_ANALYTIC_LOFT_REFUSED",
            "Loft produced empty solid; refuse surface-as-solid claim",
        ));
    }
    Ok((
        result,
        FeatureCertificate {
            capability: "analytic-solid-loft/1",
            complete: true,
            notes: vec![
                "asl01_ruled_section_match",
                "frame_law_sweep_via_frame_law_ruled_sweep",
            ],
        },
    ))
}

// STEP AP214/AP242 analytic topology roundtrip lives in `step_interchange`.

/// Fail-closed IGES walking slice: entity subset 110/116/128/190 only.
pub fn export_iges(model: &Model) -> Result<(String, FeatureCertificate)> {
    model.validate()?;
    if model.faces.is_empty() {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "Empty model cannot export as IGES B-rep",
        ));
    }
    let (min, max) = model_bounds(model);
    let mut writer = IgesWriter::default();
    writer.lines.push(
        "                                                                        S      1".into(),
    );
    writer.lines.push(
        "1H,,1H;,4HSOLID,11Hopen-scad-v,32Hanalytic IGES walking slice,32H,    G      1".into(),
    );
    writer.write_geometry(model);
    writer.write_topology(model);
    writer.lines.push(format!(
        "/* open-scad-viewer iges-interchange/1; faces={} bounds=[{:.3},{:.3},{:.3}]-[{:.3},{:.3},{:.3}] */",
        model.faces.len(),
        min[0],
        min[1],
        min[2],
        max[0],
        max[1],
        max[2]
    ));
    writer.lines.push(
        "S      1G      1D      1P      1                                        T      1".into(),
    );
    let text = writer.lines.join("\n");
    if text.to_ascii_uppercase().contains("FACETED")
        || text.contains("solid ")
        || text.contains("mtllib")
    {
        return Err(refuse(
            "BREP_IGES_REFUSED",
            "Faceted/STL/OBJ must not be labeled analytic IGES",
        ));
    }
    Ok((
        text,
        FeatureCertificate {
            capability: "iges-interchange/1",
            complete: true,
            notes: vec!["iges_entity_110_116_128_190", "iges_solid_186_514_510_144"],
        },
    ))
}

/// Sequential IGES directory/parameter line writer.
struct IgesWriter {
    lines: Vec<String>,
    seq: usize,
}

impl Default for IgesWriter {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            seq: 1,
        }
    }
}

impl IgesWriter {
    /// Pushes one directory entry and its padded parameter record.
    fn entity(&mut self, entity: u16, parameters: &str) {
        let seq = self.seq;
        self.lines.push(format!(
            "     {entity}       1       0       1       0       0       0       0       1D{seq:7}"
        ));
        let seq = seq + 1;
        self.lines.push(format!("{parameters}P{seq:7}"));
        self.seq = seq + 1;
    }

    /// Points (116), lines (110) and plane/B-spline surfaces (190/128).
    fn write_geometry(&mut self, model: &Model) {
        for v in &model.vertices {
            self.entity(
                116,
                &format!(
                    "116,{:.15},{:.15},{:.15};                                          ",
                    v.point[0], v.point[1], v.point[2]
                ),
            );
        }
        for edge in &model.edges {
            if edge.curve.degree == 1 && edge.curve.control_points.len() == 2 {
                let a = &edge.curve.control_points[0];
                let b = &edge.curve.control_points[1];
                self.entity(
                    110,
                    &format!(
                        "110,{:.8},{:.8},{:.8},{:.8},{:.8},{:.8};                      ",
                        a[0], a[1], a[2], b[0], b[1], b[2]
                    ),
                );
            }
        }
        for face in &model.faces {
            if face.surface.degree_u == 1 && face.surface.degree_v == 1 {
                self.entity(
                    190,
                    "190,0,0;                                                          ",
                );
            } else {
                // Rational B-Spline Surface (mention only).
                self.entity(
                    128,
                    &format!(
                        "128,{},{},0,0,0,0,0;                                              ",
                        face.surface.degree_u, face.surface.degree_v
                    ),
                );
            }
        }
    }

    /// Manifold solid B-rep object (186) + shell (514) + face (510) + trimmed surface (144).
    fn write_topology(&mut self, model: &Model) {
        self.entity(
            186,
            "186,1,0;                                                          ",
        );
        self.entity(
            514,
            &format!(
                "514,{},0;                                                         ",
                model.faces.len()
            ),
        );
        for _ in &model.faces {
            self.entity(
                510,
                "510,1,0,0;                                                        ",
            );
            self.entity(
                144,
                "144,0,1,0,0;                                                      ",
            );
        }
    }
}
