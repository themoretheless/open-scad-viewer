//! Original-curve Bishop transport, independent of discrete reflections.
//! For v=P', a=P'', omega=(v x a)/|v|^2 and N'=omega x N.
//! Skew flows are isometries: transported uncertainty adds, without a
//! spurious exp(total curvature) multiplier. Each cell encloses the original
//! omega, then charges variation and a constant-skew quadratic remainder.
//! This is an uncorrected source-normal cover, not a sweep/Solid certificate.
use super::endpoint_jets;
use crate::numerics::interval_vec3::{Vec3, add, cross, div, dot_tight, norm, scale, sub};
use crate::sweeps::progressive_miter::{
    scalar_certificate::Status, trigonometric_certificate, vector_certificate,
};
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I};

#[derive(Clone, Debug)]
pub struct RmfNormalCell {
    pub traversal: [f64; 2],
    /// Every original normal value on this whole parameter interval.
    pub normal: [[f64; 2]; 3],
    pub tangent: [[f64; 2]; 3],
    pub endpoint_normal: [[f64; 2]; 3],
    pub endpoint_radius_upper: f64,
    /// Original normalized-traversal speed, used for fresh arc-length bounds.
    pub speed: [f64; 2],
}
#[derive(Clone, Debug)]
pub struct OriginalRmfTransportReport {
    pub status: Status,
    pub cells: usize,
    pub exact_work: u64,
    pub cover: Option<Vec<RmfNormalCell>>,
    /// Original closing-to-initial Bishop rotation, before correction.
    pub closing_angle: Option<[f64; 2]>,
    /// Source Bishop normals corrected by original arc-length fraction.
    /// This does not certify the constructor's sampled-chord correction.
    pub arc_corrected_normals: Option<Vec<[[f64; 2]; 3]>>,
    pub arc_corrected_binormals: Option<Vec<[[f64; 2]; 3]>>,
    pub reason: Option<&'static str>,
    pub continuous_bound: bool,
    pub solid_certified: bool,
}
fn decode(v: [[f64; 2]; 3]) -> Result<Vec3> {
    Ok([
        I::new(v[0][0], v[0][1])?,
        I::new(v[1][0], v[1][1])?,
        I::new(v[2][0], v[2][1])?,
    ])
}
fn encode(v: Vec3) -> [[f64; 2]; 3] {
    v.map(|x| [x.lo, x.hi])
}
fn point(v: [f64; 3]) -> Vec3 {
    v.map(I::point)
}
fn unit(v: Vec3) -> Result<Vec3> {
    div(v, norm(v)?)
}
fn midpoint(v: Vec3) -> [f64; 3] {
    v.map(|x| x.lo * 0.5 + x.hi * 0.5)
}
fn ball(c: [f64; 3], r: f64) -> Result<Vec3> {
    add(point(c), [I::new(-r, r)?; 3])
}
fn initial(
    path: &Curve,
    normal: [f64; 3],
    remaining: usize,
) -> Result<(usize, Option<(Vec3, Vec3)>)> {
    let first = vector_certificate::certify_first_point(path, 0., remaining)?;
    let Some(v) = first.first else {
        return Ok((first.cells, None));
    };
    let value: Result<(Vec3, Vec3)> = (|| {
        let t = unit(decode(v)?)?;
        let n = point(normal);
        Ok((t, unit(sub(n, scale(t, dot_tight(n, t)?)?)?)?))
    })();
    Ok((first.cells, value.ok()))
}

/// C1 Cartesian source knots and fresh whole-cell velocity/acceleration are
/// mandatory. Steps are dyadic in normalized traversal; source domain mapping
/// and derivative factors are outward rounded. Partial covers are discarded.
/// Closed mode additionally proves original endpoint C1 and encloses holonomy;
/// distribution of that correction over the retained sweep is still separate.
pub fn certify_original_rmf_transport(
    path: &Curve,
    normal: [f64; 3],
    steps: usize,
    max_cells: usize,
    max_exact_work: u64,
    closed: bool,
) -> Result<OriginalRmfTransportReport> {
    certify_original_rmf_transport_impl(
        path,
        normal,
        steps,
        max_cells,
        max_exact_work,
        closed,
        None,
    )
}

/// Exact source premises and interval transport spend one aggregate allowance.
/// Premises run first; only their actual expenditure is reserved from cells.
pub(crate) fn certify_original_rmf_transport_shared(
    path: &Curve,
    normal: [f64; 3],
    steps: usize,
    max_work: usize,
    closed: bool,
) -> Result<OriginalRmfTransportReport> {
    certify_original_rmf_transport_impl(
        path,
        normal,
        steps,
        max_work,
        max_work as u64,
        closed,
        Some(max_work),
    )
}

fn certify_original_rmf_transport_impl(
    path: &Curve,
    normal: [f64; 3],
    steps: usize,
    max_cells: usize,
    max_exact_work: u64,
    closed: bool,
    shared_work: Option<usize>,
) -> Result<OriginalRmfTransportReport> {
    path.validate()?;
    check(
        path.control_points.iter().all(|p| p.len() == 3)
            && normal.iter().all(|v| v.is_finite())
            && steps.is_power_of_two()
            && (2..=16384).contains(&steps)
            && max_cells <= 100000
            && max_exact_work <= 1000000,
        "Invalid original RMF transport limits",
    )?;
    let mut out = OriginalRmfTransportReport {
        status: Status::Unresolved,
        cells: 0,
        exact_work: 0,
        cover: None,
        closing_angle: None,
        arc_corrected_normals: None,
        arc_corrected_binormals: None,
        reason: Some("rmf-original-C1-unproved"),
        continuous_bound: false,
        solid_certified: false,
    };
    let knots = endpoint_jets::certify_knots(path, 1, max_exact_work)?;
    out.exact_work = knots.exact_work;
    if !knots.certified {
        return Ok(out);
    }
    if closed {
        let seam = endpoint_jets::certify(path, 1, max_exact_work - out.exact_work)?;
        out.exact_work += seam.exact_work;
        if !seam.certified {
            out.reason = Some("rmf-original-closed-C1-unproved");
            return Ok(out);
        }
    }
    let max_cells = match shared_work {
        Some(total) => {
            max_cells.min(total.checked_sub(out.exact_work as usize).ok_or_else(|| {
                crate::resource("RMF exact source work exceeded shared allowance")
            })?)
        }
        None => max_cells,
    };
    out.reason = Some("rmf-initial-normal-unproved");
    let (work, seed) = initial(path, normal, max_cells)?;
    out.cells += work;
    let Some((t0, n0)) = seed else {
        return Ok(out);
    };
    let [a, b] = path.domain();
    let width = I::point(b).sub(I::point(a))?;
    let h = I::point(1. / steps as f64);
    let mut center = midpoint(n0);
    let mut radius = norm(sub(n0, point(center))?)?.hi;
    let mut cover = Vec::with_capacity(steps);
    for index in 0..steps {
        out.reason = Some("rmf-original-jet-work-unproved");
        let traversal = [
            index as f64 / steps as f64,
            (index + 1) as f64 / steps as f64,
        ];
        let jets =
            vector_certificate::certify_traversal(path, traversal, max_cells - out.cells, false)?;
        out.cells += jets.cells;
        if jets.status != Status::Certified {
            return Ok(out);
        }
        if out.cells == max_cells {
            return Ok(out);
        }
        out.cells += 1; // whole rotation/error propagation cell, under same owner
        out.reason = Some("rmf-original-speed-or-propagation-unproved");
        let step = (|| -> Result<(RmfNormalCell, [f64; 3], f64)> {
            let v = scale(decode(jets.first.unwrap())?, width)?;
            let acceleration = scale(decode(jets.second.unwrap())?, width.mul(width)?)?;
            let speed = norm(v)?;
            let speed_square = speed.mul(speed)?;
            let omega = div(cross(v, acceleration)?, speed_square)?;
            let reference = point(midpoint(omega));
            let c = point(center);
            let c_length = norm(c)?;
            // Duhamel: ||U_variable c-U_reference c|| <= h*delta_omega*||c||.
            let variation = h.mul(norm(sub(omega, reference)?)?)?.mul(c_length)?;
            // Constant skew flow: retain the quadratic Taylor term.
            // Every derivative of exp(t A)c has norm <= |omega0|^k |c|
            // because A is skew and exp(t A) is an isometry. The integral
            // third-order remainder needs no exponential inflation.
            let w = norm(reference)?;
            let h2 = h.mul(h)?;
            let w2 = w.mul(w)?;
            let remainder = h2
                .mul(h)?
                .mul(w2.mul(w)?)?
                .mul(c_length)?
                .div(I::point(6.))?;
            let first = cross(reference, c)?;
            let second = cross(reference, first)?;
            let exact_quadratic = add(
                add(c, scale(first, h)?)?,
                scale(second, h2.mul(I::point(0.5))?)?,
            )?;
            let next = midpoint(exact_quadratic);
            let rounding = norm(sub(exact_quadratic, point(next))?)?;
            let next_radius = I::point(radius)
                .add(variation)?
                .add(remainder)?
                .add(rounding)?
                .hi;
            // Whole-cell motion from c: ||U(t)c-c|| <= h*sup|omega|*||c||.
            let tube_radius = I::point(radius)
                .add(h.mul(norm(omega)?)?.mul(c_length)?)?
                .hi;
            Ok((
                RmfNormalCell {
                    traversal,
                    normal: encode(ball(center, tube_radius)?),
                    tangent: encode(div(v, speed)?),
                    endpoint_normal: encode(ball(next, next_radius)?),
                    endpoint_radius_upper: next_radius,
                    speed: [speed.lo, speed.hi],
                },
                next,
                next_radius,
            ))
        })();
        let Ok((cell, next, next_radius)) = step else {
            return Ok(out);
        };
        center = next;
        radius = next_radius;
        cover.push(cell);
    }
    if closed {
        out.reason = Some("rmf-closing-angle-work-unproved");
        if out.cells == max_cells {
            return Ok(out);
        }
        out.cells += 1;
        let end = ball(center, radius)?;
        let sine = dot_tight(t0, cross(end, n0)?)?;
        let cosine = dot_tight(end, n0)?;
        let angle =
            trigonometric_certificate::certify_atan2([sine.lo, sine.hi], [cosine.lo, cosine.hi])?
                .unwrap_or_else(|| {
                    // The proved original C1 closure identifies the endpoint
                    // tangents. Bishop transport preserves the unit normal
                    // perpendicular to that tangent, so the true sine/cosine
                    // pair lies on the unit circle and atan2 is defined.
                    // Its principal value always belongs to [-pi, pi], even
                    // when this enclosure crosses the negative-axis cut.
                    // Preserve both branches; downstream error admission must
                    // account for the whole conservative correction image.
                    let pi = std::f64::consts::PI.next_up();
                    [-pi, pi]
                });
        // Original arc-length distribution, independently of sampled chords.
        // No partial corrected cover survives insufficient work.
        out.reason = Some("rmf-arc-correction-work-unproved");
        let mut total = I::point(0.);
        for cell in &cover {
            if out.cells == max_cells {
                return Ok(out);
            }
            out.cells += 1;
            total = total.add(I::new(cell.speed[0], cell.speed[1])?.mul(h)?)?;
        }
        let mut prefix = I::point(0.);
        let mut corrected = Vec::with_capacity(steps);
        let mut binormals = Vec::with_capacity(steps);
        for cell in &cover {
            if out.cells == max_cells {
                return Ok(out);
            }
            out.cells += 1;
            let length = I::new(cell.speed[0], cell.speed[1])?.mul(h)?;
            let next_prefix = prefix.add(length)?;
            // Every partial length in this cell lies between the proved
            // previous prefix lower bound and next prefix upper bound.
            let fraction = I::new(prefix.lo.max(0.), next_prefix.hi)?
                .div(total)?
                .intersect(0., 1.)?;
            let phase = I::new(angle[0], angle[1])?.mul(fraction)?;
            let trig = trigonometric_certificate::certify([phase.lo, phase.hi])?;
            let n = decode(cell.normal)?;
            let t = decode(cell.tangent)?;
            let cosine = I::new(trig.cos[0], trig.cos[1])?;
            let sine = I::new(trig.sin[0], trig.sin[1])?;
            // Full Rodrigues formula: interval N and T must not silently
            // assume an exact dot product from their rounded enclosures.
            let rotated = add(
                add(scale(n, cosine)?, scale(cross(t, n)?, sine)?)?,
                scale(t, dot_tight(t, n)?.mul(I::point(1.).sub(cosine)?)?)?,
            )?;
            binormals.push(encode(cross(t, rotated)?));
            corrected.push(encode(rotated));
            prefix = next_prefix;
        }
        out.closing_angle = Some(angle);
        out.arc_corrected_normals = Some(corrected);
        out.arc_corrected_binormals = Some(binormals);
    }
    out.status = Status::Certified;
    out.reason = None;
    out.cover = Some(cover);
    Ok(out)
}
/// Fresh source relative section images in (normal, binormal, tangent)
/// coordinates. No position, scalar/affine law, retained-wall or cap proof is
/// implied. A caller must account for those separately before admission.
#[derive(Clone, Debug)]
pub struct OriginalRmfSectionImagesReport {
    pub transport: OriginalRmfTransportReport,
    pub images: Option<Vec<Vec<[[f64; 2]; 3]>>>,
}
pub fn certify_original_rmf_section_images(
    path: &Curve,
    normal: [f64; 3],
    coordinates: &[[[f64; 2]; 3]],
    steps: usize,
    max_cells: usize,
    max_exact_work: u64,
    closed: bool,
) -> Result<OriginalRmfSectionImagesReport> {
    check(
        !coordinates.is_empty() && coordinates.len() <= 100000,
        "Invalid RMF section coordinate count",
    )?;
    let coordinates: Vec<_> = coordinates
        .iter()
        .copied()
        .map(decode)
        .collect::<Result<_>>()?;
    let mut transport =
        certify_original_rmf_transport(path, normal, steps, max_cells, max_exact_work, closed)?;
    let mut images = Vec::with_capacity(steps);
    if transport.status != Status::Certified {
        return Ok(OriginalRmfSectionImagesReport {
            transport,
            images: None,
        });
    }
    for (i, cell) in transport.cover.as_ref().unwrap().iter().enumerate() {
        let t = decode(cell.tangent)?;
        let n = decode(if closed {
            transport.arc_corrected_normals.as_ref().unwrap()[i]
        } else {
            cell.normal
        })?;
        let b = if closed {
            decode(transport.arc_corrected_binormals.as_ref().unwrap()[i])?
        } else {
            cross(t, n)?
        };
        let mut values = Vec::with_capacity(coordinates.len());
        for q in &coordinates {
            if transport.cells == max_cells {
                transport.status = Status::Unresolved;
                transport.reason = Some("rmf-source-section-image-work-unproved");
                transport.cover = None;
                transport.closing_angle = None;
                transport.arc_corrected_normals = None;
                transport.arc_corrected_binormals = None;
                return Ok(OriginalRmfSectionImagesReport {
                    transport,
                    images: None,
                });
            }
            transport.cells += 1;
            values.push(encode(add(
                add(scale(n, q[0])?, scale(b, q[1])?)?,
                scale(t, q[2])?,
            )?));
        }
        images.push(values);
    }
    Ok(OriginalRmfSectionImagesReport {
        transport,
        images: Some(images),
    })
}

/// Internal query over a freshly owned transport proof. The complete source
/// cover is charged by its caller; this query charges each inspected cell and
/// the original twist law before existing affine/scale image machinery.
pub(super) fn frame_values(
    transport: &OriginalRmfTransportReport,
    twist: &Curve,
    traversal: [f64; 2],
    station: [f64; 2],
    max_cells: usize,
) -> Result<super::super::progressive_miter::authored_frame_certificate::ValuesReport> {
    use super::super::progressive_miter::{
        authored_frame_certificate as frame, scalar_certificate,
    };
    let mut axes: Option<[Vec3; 3]> = None;
    let mut cells = 0;
    let unresolved = |cells| frame::ValuesReport {
        status: Status::Unresolved,
        cells,
        longitudinal: None,
        transverse: None,
        binormal: None,
    };
    let Some(cover) = transport
        .cover
        .as_ref()
        .filter(|_| transport.status == Status::Certified)
    else {
        return Ok(unresolved(0));
    };
    for (i, cell) in cover.iter().enumerate() {
        if cell.traversal[1] < traversal[0] || cell.traversal[0] > traversal[1] {
            continue;
        }
        if cells == max_cells {
            return Ok(unresolved(cells));
        }
        cells += 1;
        let t = decode(cell.tangent)?;
        let n = decode(
            transport
                .arc_corrected_normals
                .as_ref()
                .map_or(cell.normal, |v| v[i]),
        )?;
        let b = if let Some(v) = transport.arc_corrected_binormals.as_ref() {
            decode(v[i])?
        } else {
            cross(t, n)?
        };
        let value = [t, n, b];
        if let Some(old) = axes.as_mut() {
            for a in 0..3 {
                for k in 0..3 {
                    old[a][k] = I::new(
                        old[a][k].lo.min(value[a][k].lo),
                        old[a][k].hi.max(value[a][k].hi),
                    )?;
                }
            }
        } else {
            axes = Some(value);
        }
    }
    let Some([t, n, b]) = axes else {
        return Ok(unresolved(cells));
    };
    let charge = (twist.degree..twist.control_points.len())
        .filter(|&i| twist.knots[i] < twist.knots[i + 1])
        .count();
    if charge > max_cells - cells {
        return Ok(unresolved(cells));
    }
    let theta = scalar_certificate::value_traversal(twist, station, charge)?;
    cells += charge;
    let Some(theta) = theta else {
        return Ok(unresolved(cells));
    };
    let trig = trigonometric_certificate::certify(theta)?;
    let c = I::new(trig.cos[0], trig.cos[1])?;
    let sin = I::new(trig.sin[0], trig.sin[1])?;
    let rotated_n = add(scale(n, c)?, scale(b, sin)?)?;
    let rotated_b = sub(scale(b, c)?, scale(n, sin)?)?;
    Ok(frame::ValuesReport {
        status: Status::Certified,
        cells,
        longitudinal: Some(encode(t)),
        transverse: Some(encode(rotated_n)),
        binormal: Some(encode(rotated_b)),
    })
}

pub(super) fn relative_values(
    transport: &OriginalRmfTransportReport,
    scale_law: &Curve,
    twist: &Curve,
    affine: Option<(&Curve, &Curve)>,
    qs: &[[[f64; 2]; 3]],
    traversal: [f64; 2],
    station: [f64; 2],
    max_cells: usize,
) -> Result<super::super::progressive_miter::authored_frame_certificate::ControlValuesReport> {
    use super::super::progressive_miter::authored_frame_certificate as frame;
    let values = frame_values(transport, twist, traversal, station, max_cells)?;
    if values.status != Status::Certified {
        return Ok(frame::ControlValuesReport {
            status: Status::Unresolved,
            cells: values.cells,
            values: None,
            reason: Some("spatial-rmf-relative-image-unproved"),
        });
    }
    frame::relative_control_values_from_frame(scale_law, affine, qs, station, max_cells, values)
}

#[cfg(test)]
#[path="tests/rmf_transport.rs"]
pub(super) mod tests;
