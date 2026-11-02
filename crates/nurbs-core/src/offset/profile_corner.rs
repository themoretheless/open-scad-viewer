//! Trim two original rational planar curves and author a convex round/chamfer.
//! Numeric contact generation is separate from whole B-rep/radius admission.
use crate::{Error, Result, curve::Curve, curve_measure};
pub mod full_round;
fn refused(message: &str) -> Error {
    Error::new("NURBS_PROFILE_CORNER_REFUSED", message)
}
fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn norm(a: [f64; 2]) -> f64 {
    a[0].hypot(a[1])
}
fn cross(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
struct Jet {
    point: [f64; 2],
    d1: [f64; 2],
    d2: [f64; 2],
}
fn jet(c: &Curve, t: f64) -> Result<Jet> {
    let e = c.evaluate(t)?;
    let d1 =
        e.d1.ok_or_else(|| refused("Original curve first derivative unresolved"))?;
    let d2 =
        e.d2.ok_or_else(|| refused("Original curve second derivative unresolved"))?;
    let speed = d1[0].hypot(d1[1]);
    if !speed.is_finite() || speed <= 0. {
        return Err(refused("Singular original contact curve"));
    }
    Ok(Jet {
        point: [e.point[0], e.point[1]],
        d1: [d1[0], d1[1]],
        d2: [d2[0], d2[1]],
    })
}
fn validate(a: &Curve, b: &Curve, tol: f64) -> Result<(Jet, Jet)> {
    a.validate()?;
    b.validate()?;
    if !tol.is_finite()
        || tol <= 0.
        || a.periodic
        || b.periodic
        || a.control_points[0].len() != 2
        || b.control_points[0].len() != 2
    {
        return Err(refused(
            "Need nonperiodic 2D curves and positive finite tolerance",
        ));
    }
    let ja = jet(a, a.domain()[1])?;
    let jb = jet(b, b.domain()[0])?;
    if norm(sub(ja.point, jb.point)) > tol
        || cross(ja.d1, jb.d1) <= 1e-10 * norm(ja.d1) * norm(jb.d1)
    {
        return Err(refused(
            "Original curves must meet at a strictly convex material-left junction",
        ));
    }
    Ok((ja, jb))
}
pub struct Corner {
    incoming: Curve,
    bridge: Curve,
    outgoing: Curve,
    parameters: [f64; 2],
    contacts: [[f64; 2]; 2],
    center: Option<[f64; 2]>,
    radius: Option<f64>,
    contact_residual: f64,
    setback_errors: Option<[f64; 2]>,
    length_cells: usize,
    source_error_upper: [f64; 2],
}
impl Corner {
    pub fn incoming(&self) -> &Curve {
        &self.incoming
    }
    pub fn bridge(&self) -> &Curve {
        &self.bridge
    }
    pub fn outgoing(&self) -> &Curve {
        &self.outgoing
    }
    pub fn parameters(&self) -> [f64; 2] {
        self.parameters
    }
    pub fn contacts(&self) -> [[f64; 2]; 2] {
        self.contacts
    }
    pub fn center(&self) -> Option<[f64; 2]> {
        self.center
    }
    pub fn radius(&self) -> Option<f64> {
        self.radius
    }
    pub fn contact_residual(&self) -> f64 {
        self.contact_residual
    }
    pub fn setback_errors(&self) -> Option<[f64; 2]> {
        self.setback_errors
    }
    pub fn length_cells(&self) -> usize {
        self.length_cells
    }
    pub fn source_error_upper(&self) -> [f64; 2] {
        self.source_error_upper
    }
}
fn restriction_error(original: &Curve, retained: &Curve) -> Result<f64> {
    let [a, b] = retained.domain();
    let mut nodes = vec![a, b];
    nodes.extend(
        original
            .knots
            .iter()
            .chain(&retained.knots)
            .copied()
            .filter(|t| a < *t && *t < b),
    );
    nodes.sort_by(f64::total_cmp);
    nodes.dedup();
    if nodes.len() > 257 {
        return Err(refused("Original restriction span budget exceeded"));
    }
    let mut error = 0f64;
    for w in nodes.windows(2) {
        let span = |c: &Curve| {
            (c.degree..c.control_points.len()).find(|&i| {
                c.knots[i] < c.knots[i + 1] && c.knots[i] <= w[0] && w[1] <= c.knots[i + 1]
            })
        };
        let ia = span(original).ok_or_else(|| refused("Original restriction span missing"))?;
        let ib = span(retained).ok_or_else(|| refused("Retained restriction span missing"))?;
        let bound = crate::curve_deviation::translated_cell_upper(
            original,
            ia,
            retained,
            ib,
            [w[0], w[1]],
            &[0.; 2],
        )?;
        error = error.max(bound);
    }
    Ok(error)
}
fn offset(c: &Curve, t: f64, r: f64) -> Result<([f64; 2], [f64; 2])> {
    let j = jet(c, t)?;
    let q = norm(j.d1);
    let projection = (j.d1[0] * j.d2[0] + j.d1[1] * j.d2[1]) / (q * q * q);
    let tangent = j.d1.map(|v| v / q);
    let derivative = std::array::from_fn::<_, 2, _>(|i| j.d2[i] / q - j.d1[i] * projection);
    let normal = [-tangent[1], tangent[0]];
    let dn = [-derivative[1], derivative[0]];
    let p = std::array::from_fn(|i| j.point[i] + r * normal[i]);
    let d = std::array::from_fn(|i| j.d1[i] + r * dn[i]);
    if p.iter().chain(&d).any(|v| !v.is_finite()) {
        return Err(refused("Offset contact overflow"));
    }
    Ok((p, d))
}
/// Solve the actual two original offset-curve equations with analytic Jacobian
/// and bounded Newton descent. The seed selects a local convex junction branch;
/// global profile simplicity and whole circular radius are downstream gates.
pub fn round(a: &Curve, b: &Curve, radius: f64, tolerance: f64) -> Result<Corner> {
    let (ja, jb) = validate(a, b, tolerance)?;
    if !radius.is_finite() || radius <= 0. || radius <= tolerance * 4. {
        return Err(refused("Round radius must exceed the contact tolerance"));
    }
    let cosine = (ja.d1[0] * jb.d1[0] + ja.d1[1] * jb.d1[1]) / (norm(ja.d1) * norm(jb.d1));
    let sine = cross(ja.d1, jb.d1) / (norm(ja.d1) * norm(jb.d1));
    let setback = radius * sine / (1. + cosine);
    let domains = [a.domain(), b.domain()];
    let widths = domains.map(|d| d[1] - d[0]);
    let mut x = [
        1. - (setback / (norm(ja.d1) * widths[0])).min(0.45),
        (setback / (norm(jb.d1) * widths[1])).min(0.45),
    ];
    let residual = |x: [f64; 2]| -> Result<([f64; 2], [[f64; 2]; 2], [[f64; 2]; 2])> {
        let (pa, da) = offset(a, domains[0][0] + x[0] * widths[0], radius)?;
        let (pb, db) = offset(b, domains[1][0] + x[1] * widths[1], radius)?;
        Ok((
            sub(pa, pb),
            [da.map(|v| v * widths[0]), db.map(|v| -v * widths[1])],
            [pa, pb],
        ))
    };
    let mut solved = false;
    for _ in 0..40 {
        let (f, j, _) = residual(x)?;
        let determinant = cross(j[0], j[1]);
        if !determinant.is_finite() || determinant.abs() <= 1e-12 * norm(j[0]) * norm(j[1]) {
            return Err(refused("Convex offset contact Jacobian singular"));
        }
        let old = norm(f);
        if old <= tolerance {
            solved = true;
            break;
        }
        let negative = f.map(|v| -v);
        let step = [
            cross(negative, j[1]) / determinant,
            cross(j[0], negative) / determinant,
        ];
        let mut scale = 1.;
        let mut accepted = false;
        for _ in 0..24 {
            let next = std::array::from_fn(|i| x[i] + scale * step[i]);
            if next.iter().all(|v| v.is_finite() && *v > 0. && *v < 1.) {
                if let Ok((f, _, _)) = residual(next) {
                    if norm(f) < old {
                        x = next;
                        accepted = true;
                        break;
                    }
                }
            }
            scale *= 0.5;
        }
        if !accepted {
            return Err(refused(
                "Round does not fit original curves or local descent failed",
            ));
        }
    }
    let (f, _, offsets) = residual(x)?;
    let contact_residual = norm(f);
    if !solved && contact_residual > tolerance {
        return Err(refused("Round contact work exhausted"));
    }
    let parameters = std::array::from_fn(|i| domains[i][0] + x[i] * widths[i]);
    let incoming = a.trim(domains[0][0], parameters[0])?;
    let outgoing = b.trim(parameters[1], domains[1][1])?;
    let left = incoming.control_points.last().unwrap();
    let right = &outgoing.control_points[0];
    let contacts = [[left[0], left[1]], [right[0], right[1]]];
    let source_error_upper = [
        restriction_error(a, &incoming)?,
        restriction_error(b, &outgoing)?,
    ];
    if source_error_upper.iter().any(|e| *e > tolerance) {
        return Err(refused(
            "Actual retained curve disagrees with the original restriction",
        ));
    }
    let center = std::array::from_fn(|i| offsets[0][i] / 2. + offsets[1][i] / 2.);
    let radial = contacts.map(|p| sub(p, center));
    let cosine = (radial[0][0] * radial[1][0] + radial[0][1] * radial[1][1])
        / (norm(radial[0]) * norm(radial[1]));
    let sine = cross(radial[0], radial[1]) / (norm(radial[0]) * norm(radial[1]));
    if sine <= 0. || cosine <= -1. + 1e-8 {
        return Err(refused(
            "Round contact angle is nonconvex or near a semicircle",
        ));
    }
    let weight = ((1. + cosine.clamp(-1., 1.)) / 2.).sqrt();
    let middle = std::array::from_fn::<_, 2, _>(|i| {
        center[i] + (radial[0][i] + radial[1][i]) / (2. * weight * weight)
    });
    let bridge = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![contacts[0].to_vec(), middle.to_vec(), contacts[1].to_vec()],
        weights: vec![1., weight, 1.],
        periodic: false,
    };
    bridge.validate()?;
    Ok(Corner {
        incoming,
        outgoing,
        bridge,
        parameters,
        contacts,
        center: Some(center),
        radius: Some(radius),
        contact_residual,
        setback_errors: None,
        length_cells: 0,
        source_error_upper,
    })
}
/// Distance/distance chamfer measured along the two original rational curves.
/// Prefix inversion only proposes contacts; final ORIGINAL tail/head lengths
/// are freshly bounded against requested distances before returning the edit.
pub fn chamfer(
    a: &Curve,
    b: &Curve,
    distances: [f64; 2],
    tolerance: f64,
    max_cells: usize,
) -> Result<Corner> {
    validate(a, b, tolerance)?;
    if distances
        .iter()
        .any(|d| !d.is_finite() || *d <= tolerance * 4.)
        || !(32..=100000).contains(&max_cells)
    {
        return Err(refused(
            "Chamfer needs positive setbacks and bounded length work",
        ));
    }
    let reverse = a.reverse()?;
    let mut remaining = max_cells;
    let first =
        curve_measure::point_at_length(&reverse, distances[0], tolerance / 4., remaining / 3)?;
    remaining = remaining.saturating_sub(first.cells);
    let second = curve_measure::point_at_length(b, distances[1], tolerance / 4., remaining / 3)?;
    remaining = remaining.saturating_sub(second.cells);
    if !first.within_tolerance || !second.within_tolerance {
        return Err(refused("Chamfer length inverse unresolved"));
    }
    let da = a.domain();
    let db = b.domain();
    let parameters = [da[0] + da[1] - first.parameter, second.parameter];
    if parameters[0] <= da[0]
        || parameters[0] >= da[1]
        || parameters[1] <= db[0]
        || parameters[1] >= db[1]
    {
        return Err(refused("Chamfer consumes a complete source curve"));
    }
    let tail = a.trim(parameters[0], da[1])?;
    let head = b.trim(db[0], parameters[1])?;
    let mut errors = [0.; 2];
    for (i, curve) in [&tail, &head].into_iter().enumerate() {
        let report = curve_measure::length(curve, tolerance / 4., remaining / (2 - i))?;
        remaining = remaining.saturating_sub(report.cells);
        errors[i] = (report.bounds[0] - distances[i])
            .abs()
            .max((report.bounds[1] - distances[i]).abs());
        if !report.within_tolerance || errors[i] > tolerance {
            return Err(refused("Actual original chamfer setback exceeds tolerance"));
        }
    }
    let incoming = a.trim(da[0], parameters[0])?;
    let outgoing = b.trim(parameters[1], db[1])?;
    let left = incoming.control_points.last().unwrap();
    let right = &outgoing.control_points[0];
    let contacts = [[left[0], left[1]], [right[0], right[1]]];
    let source_error_upper = [
        restriction_error(a, &incoming)?,
        restriction_error(b, &outgoing)?,
    ];
    if source_error_upper.iter().any(|e| *e > tolerance) {
        return Err(refused(
            "Actual chamfer source restriction disagrees with original curve",
        ));
    }
    let bridge = Curve::from_polyline(contacts.iter().map(|p| p.to_vec()).collect())?;
    Ok(Corner {
        incoming,
        outgoing,
        bridge,
        parameters,
        contacts,
        center: None,
        radius: None,
        contact_residual: 0.,
        setback_errors: Some(errors),
        length_cells: max_cells - remaining,
        source_error_upper,
    })
}
