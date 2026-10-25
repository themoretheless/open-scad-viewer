use super::*;

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

/// Rational end-transition support with cubic smoothstep radius in rational arc
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
            // The adjacent radius-law control also equals zero. Its Euclidean
            // point is independent of meridian v, just like the pole row.
            // Repeated multiply/divide through different meridian weights
            // otherwise introduces one-ulp transverse changes in this row.
            let adjacent = if k == 0 { 1 } else { 4 };
            center_controls[adjacent][2] = height;
            for d in 0..2 {
                if point[d] == unit.control_points[1][d] {
                    center_controls[adjacent][d] = outer_radius * point[d];
                }
            }
            controls[adjacent] = vec![center_controls[adjacent].clone(); 3];
        }
    }
    // First align the two cylinder-side meridian control columns in XY.
    // Independently weighted accumulation/division can split them by an ulp.
    // Preserve the original contact rail and copy it into the tangent column.
    for row in &mut controls {
        row[1][0]=row[2][0];
        row[1][1]=row[2][1];
    }
    // Keep rounding-scale projective variation on the inward side of the
    // contact rail. Endpoint/seam and collapsed rows stay definition-exact.
    // This is a bounded authoring perturbation, not exact analytic tangency;
    // radius/normal qualification must still inspect the resulting surface.
    for k in 1..5 {
        if controls[k].iter().all(|p|p==&controls[k][0]) {continue;}
        for d in 0..2 {
            let x=controls[k][1][d];
            if x!=0. {
                let bits=x.to_bits();
                let moved=f64::from_bits(bits.checked_sub(16).ok_or_else(||invalid("Transition coordinate correction overflow"))?);
                if !moved.is_finite() {return Err(invalid("Transition coordinate correction is not finite"));}
                controls[k][1][d]=moved;
            }
        }
    }
    // The plane contact and its meridian tangent controls have authored
    // height H. Product-degree arithmetic must not introduce a Z residual.
    for row in &mut controls {row[0][2]=height;row[1][2]=height;}
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
    let q = sweep;
    let spans = circular_strip_spans(outer_radius, height, radius, 0., q / 4., q / 2., q / 4.)?;
    close_annular_spans(outer_radius, inner_radius, height, q, spans, tolerance_mm)
}

/// Closed annular prototype with independently sized entry/exit transitions
/// and two positive section radii. The middle span interpolates the radii by
/// cubic smoothstep in rational arc parameter. This does not certify a general
/// rolling-ball envelope, pole G1 continuity, or an edit of an existing body.
pub fn partial_annular_variable_arc(
    outer_radius: f64,
    inner_radius: f64,
    height: f64,
    radii: [f64; 2],
    sweeps: [f64; 3],
    tolerance_mm: f64,
) -> Result<crate::Model> {
    if radii.iter().any(|r| !r.is_finite() || *r <= 0.)
        || sweeps.iter().any(|s| {
            !s.is_finite()
                || s.abs() < 1e-12
                || s.abs() > std::f64::consts::FRAC_PI_2
                || s.signum() != sweeps[0].signum()
        })
    {
        return Err(invalid("Variable annular blend requires two positive radii and three consistently directed nonzero spans of at most pi/2"));
    }
    let middle_start = sweeps[0];
    let exit_start = middle_start + sweeps[1];
    let sweep = exit_start + sweeps[2];
    let entry = plane_cylinder_transition(outer_radius, height, 0., radii[0], 0., sweeps[0])?;
    let middle = if radii[0] == radii[1] {
        plane_cylinder_rim(outer_radius, height, radii[0], middle_start, sweeps[1])?.remove(0)
    } else {
        plane_cylinder_transition(outer_radius, height, radii[0], radii[1], middle_start, sweeps[1])?
    };
    let exit = plane_cylinder_transition(outer_radius, height, radii[1], 0., exit_start, sweeps[2])?;
    close_annular_spans(outer_radius, inner_radius, height, sweep, [entry, middle, exit], tolerance_mm)
}

fn close_annular_spans(
    outer_radius: f64,
    inner_radius: f64,
    height: f64,
    q: f64,
    spans: [CircularBlendSpan; 3],
    tolerance_mm: f64,
) -> Result<crate::Model> {
    let direction = q.signum();
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
