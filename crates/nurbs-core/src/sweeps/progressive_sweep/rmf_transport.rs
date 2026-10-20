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
    planar_basis: Option<PlanarBasis>,
}
#[derive(Clone, Debug)]
struct PlanarBasis {
    axis: Option<usize>,
    binormal: Vec3,
    parallel: I,
    transverse: I,
    longitudinal: I,
    radius: f64,
}
#[derive(Clone, Debug)]
pub struct OriginalRmfTransportReport {
    source_jets: Option<super::source_jets::SourceJets>,
    correction_phases: Option<Vec<[f64; 2]>>,
    correction_rate: Option<I>,
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
pub(super) fn unit(v: Vec3) -> Result<Vec3> {
    let mut result = div(v, norm(v)?)?;
    for axis in 0..3 {
        let mut minimum = I::point(0.);
        let mut maximum = I::point(0.);
        for k in 0..3 {
            if k != axis {
                let lo = if v[k].lo <= 0. && v[k].hi >= 0. {
                    0.
                } else {
                    v[k].lo.abs().min(v[k].hi.abs())
                };
                let hi = v[k].lo.abs().max(v[k].hi.abs());
                minimum = minimum.add(I::point(lo).mul(I::point(lo))?)?;
                maximum = maximum.add(I::point(hi).mul(I::point(hi))?)?;
            }
        }
        // x/sqrt(x²+s) increases with x>=0 and decreases with s>=0.
        // Keep the shared x in numerator and denominator, rather than
        // dividing independently boxed velocity and speed intervals.
        let ratio = |x: f64, s: f64| -> Result<I> {
            let q = I::point(x).mul(I::point(x))?.add(I::point(s))?;
            let denominator = I::new(
                q.lo.max(0.).sqrt().next_down().max(0.),
                q.hi.sqrt().next_up(),
            )?;
            I::point(x).div(denominator)
        };
        let bound = (|| -> Result<(f64, f64)> {
            let s_min = minimum.lo.max(0.);
            let s_max = maximum.hi.max(0.);
            if v[axis].lo >= 0. {
                Ok((ratio(v[axis].lo, s_max)?.lo, ratio(v[axis].hi, s_min)?.hi))
            } else if v[axis].hi <= 0. {
                Ok((
                    -ratio(-v[axis].lo, s_min)?.hi,
                    -ratio(-v[axis].hi, s_max)?.lo,
                ))
            } else {
                Ok((-ratio(-v[axis].lo, s_min)?.hi, ratio(v[axis].hi, s_min)?.hi))
            }
        })();
        if let Ok((lo, hi)) = bound {
            result[axis] = I::new(
                result[axis].lo.max(lo).max(-1.),
                result[axis].hi.min(hi).min(1.),
            )?;
        }
    }
    Ok(result)
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

/// Continuous source position/unit tangent and whole-cell velocity/acceleration are
/// mandatory. A dyadic traversal grid is refined around original source knots; domain mapping
/// and derivative factors are outward rounded. Partial covers are discarded.
/// Closed mode additionally proves original position/tangent identity and encloses holonomy;
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
        source_jets: None,
        correction_phases: None,
        correction_rate: None,
        status: Status::Unresolved,
        cells: 0,
        exact_work: 0,
        cover: None,
        closing_angle: None,
        arc_corrected_normals: None,
        arc_corrected_binormals: None,
        reason: Some("rmf-original-tangent-unproved"),
        continuous_bound: false,
        solid_certified: false,
    };
    // Bishop transport requires a continuous unit tangent, not equal
    // parameter speeds. Exact C0 position/positive tangent identity suffices
    // for piecewise regular sources; the ODE bounds hold on every span.
    let knots = if path.periodic {
        endpoint_jets::certify_knots(path, 1, max_exact_work)?
    } else {
        endpoint_jets::certify_direction(path, 0, false, false, max_exact_work)?
    };
    out.exact_work = knots.exact_work;
    if !knots.certified {
        return Ok(out);
    }
    if closed {
        let seam = if path.periodic {
            endpoint_jets::certify(path, 1, max_exact_work - out.exact_work)?
        } else {
            endpoint_jets::certify_direction(path, 0, true, false, max_exact_work - out.exact_work)?
        };
        out.exact_work += seam.exact_work;
        if !seam.certified {
            out.reason = Some("rmf-original-closed-tangent-unproved");
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
    let mut cuts = (0..=steps)
        .map(|i| i as f64 / steps as f64)
        .collect::<Vec<_>>();
    // Isolate non-grid source knots so a speed/curvature jump does not inflate
    // the angular enclosure over an entire dyadic propagation cell. Outward
    // brackets remain real source ownership; no knot is snapped to the grid.
    for &k in &path.knots {
        if k <= a || k >= b {
            continue;
        }
        if out.cells == max_cells {
            return Ok(out);
        }
        out.cells += 1;
        let normalized = if a == 0. && b == 1. {
            I::point(k)
        } else {
            I::point(k).sub(I::point(a))?.div(width)?
        };
        if normalized.lo == normalized.hi
            && cuts[..=steps]
                .binary_search_by(|v| v.total_cmp(&normalized.lo))
                .is_ok()
        {
            continue;
        }
        let mut lo = normalized.lo;
        let mut hi = normalized.hi;
        for _ in 0..8 {
            lo = lo.next_down();
            hi = hi.next_up();
        }
        if lo > 0. && hi < 1. {
            cuts.extend([lo, hi]);
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    let mut planes = Vec::with_capacity(path.control_points.len().saturating_sub(path.degree));
    for span in path.degree..path.control_points.len() {
        if path.knots[span] >= path.knots[span + 1] {
            continue;
        }
        let mut flat = None;
        for axis in 0..3 {
            let mut same = true;
            for pole in &path.control_points[span - path.degree..=span] {
                if out.cells == max_cells {
                    return Ok(out);
                }
                out.cells += 1;
                same &= pole[axis] == path.control_points[span - path.degree][axis];
            }
            if same {
                let mut b = [0.; 3];
                b[axis] = 1.;
                flat = Some((Some(axis), point(b)));
                break;
            }
        }
        if flat.is_none() && path.degree == 2 {
            // Every rational quadratic span belongs to the affine plane of
            // its three original active poles. Enclose its exact cross-product
            // normal, rather than fitting a plane or rounding its coefficients.
            if out.cells == max_cells {
                return Ok(out);
            }
            out.cells += 1;
            let poles = &path.control_points[span - 2..=span];
            let p = |j: usize| point(std::array::from_fn(|k| poles[j][k]));
            let original =
                (|| -> Result<Vec3> { unit(cross(sub(p(1), p(0))?, sub(p(2), p(0))?)?) })();
            if let Ok(binormal) = original {
                flat = Some((None, binormal));
            }
        }
        if flat.is_none() && path.degree > 2 {
            let poles = &path.control_points[span - path.degree..=span];
            let (work, plane) = exact_pole_plane(poles, max_cells - out.cells)?;
            out.cells += work;
            flat = plane.map(|binormal| (None, binormal));
        }
        planes.push((path.knots[span], path.knots[span + 1], flat));
    }
    let mut center = midpoint(n0);
    let mut radius = norm(sub(n0, point(center))?)?.hi;
    let mut cover = Vec::with_capacity(cuts.len().saturating_sub(1));
    let mut last_endpoint: Option<(u64, [[f64; 2]; 3])> = None;
    let (work, source_jets) =
        super::source_jets::SourceJets::new_order(path, 2, max_cells - out.cells)?;
    out.cells += work;
    let Some(source_jets) = source_jets else {
        return Ok(out);
    };
    for pair in cuts.windows(2) {
        out.reason = Some("rmf-original-jet-work-unproved");
        let traversal = [pair[0], pair[1]];
        let h = I::point(pair[1]).sub(I::point(pair[0]))?;
        let original = source_jets.query(traversal, max_cells - out.cells)?;
        out.cells += original.cells;
        debug_assert_eq!(original.order, 2);
        let single_span = original.single_span;
        let Some(original) = original.jets else {
            return Ok(out);
        };
        let jets = vector_certificate::Report {
            status: Status::Certified,
            cells: 0,
            value: Some(original[0]),
            first: Some(original[1]),
            second: Some(original[2]),
            single_span,
            reason: None,
        };
        if out.cells == max_cells {
            return Ok(out);
        }
        out.cells += 1; // whole rotation/error propagation cell, under same owner
        out.reason = Some("rmf-original-speed-or-propagation-unproved");
        let source = I::point(a)
            .add(width.mul(I::new(traversal[0], traversal[1])?)?)?
            .intersect(a, b)?;
        let mut flat = planes
            .iter()
            .find(|(lo, hi, _)| source.lo >= *lo && source.hi <= *hi)
            .and_then(|(_, _, axis)| *axis);
        if flat.is_none() {
            for &(lo, hi, plane) in &planes {
                if plane.is_none() || source.hi < lo || source.lo > hi {
                    continue;
                }
                // Outward mapping may cross an original knot at an exact
                // cell endpoint. Prove the represented domain comparison,
                // rather than sending a whole curved cell to the ODE because
                // its floating image extends one ulp past that endpoint.
                let mut contained = true;
                for (t, k, lower, already) in [
                    (traversal[0], lo, true, source.lo >= lo),
                    (traversal[1], hi, false, source.hi <= hi),
                ] {
                    if already {
                        continue;
                    }
                    if max_cells - out.cells < 4 {
                        return Ok(out);
                    }
                    out.cells += 4;
                    let sign = if lower {
                        crate::exact_products::sum_sign(&[(1., a), (b, t), (-a, t), (-1., k)])
                    } else {
                        crate::exact_products::sum_sign(&[(1., k), (-1., a), (-b, t), (a, t)])
                    };
                    if sign == std::cmp::Ordering::Less {
                        contained = false;
                        break;
                    }
                }
                if contained {
                    flat = plane;
                    break;
                }
            }
        }
        if let Some((axis, binormal)) = flat {
            // On an original coordinate plane, Bishop rotation is determined
            // by the source unit tangents. Rotate the numerical center only;
            // inherited ball uncertainty is preserved by the exact isometry.
            let start_first = if let Some((_, first)) =
                last_endpoint.filter(|(key, _)| *key == traversal[0].to_bits())
            {
                first
            } else {
                let start = source_jets.query([traversal[0]; 2], max_cells - out.cells)?;
                out.cells += start.cells;
                let Some(jets) = start.jets else {
                    return Ok(out);
                };
                jets[1]
            };
            let end = source_jets.query([traversal[1]; 2], max_cells - out.cells)?;
            out.cells += end.cells;
            let Some(end) = end.jets else {
                return Ok(out);
            };
            last_endpoint = Some((traversal[1].to_bits(), end[1]));
            let planar_step = (|| -> Result<(RmfNormalCell, [f64; 3], f64)> {
                let mut v = decode(jets.first.unwrap())?;
                if let Some(axis) = axis {
                    v[axis] = I::point(0.);
                }
                let speed = norm(scale(v, width)?)?;
                let t = unit(v)?;
                let tangent = |first| -> Result<Vec3> {
                    let mut v = decode(first)?;
                    if let Some(axis) = axis {
                        v[axis] = I::point(0.);
                    }
                    unit(v)
                };
                let t_start = tangent(start_first)?;
                let t_end = tangent(end[1])?;
                let c = point(center);
                let parallel = dot_tight(c, binormal)?;
                let transverse = dot_tight(c, cross(binormal, t_start)?)?;
                let longitudinal = dot_tight(c, t_start)?;
                let image = |t| -> Result<Vec3> {
                    add(
                        add(
                            scale(binormal, parallel)?,
                            scale(cross(binormal, t)?, transverse)?,
                        )?,
                        scale(t, longitudinal)?,
                    )
                };
                let endpoint = image(t_end)?;
                let next = midpoint(endpoint);
                let next_radius = I::point(radius).add(norm(sub(endpoint, point(next))?)?)?.hi;
                let tube = image(t)?;
                let tube_center = midpoint(tube);
                let tube_radius = I::point(radius)
                    .add(norm(sub(tube, point(tube_center))?)?)?
                    .hi;
                Ok((
                    RmfNormalCell {
                        traversal,
                        normal: encode(ball(tube_center, tube_radius)?),
                        tangent: encode(t),
                        endpoint_normal: encode(ball(next, next_radius)?),
                        endpoint_radius_upper: next_radius,
                        speed: [speed.lo, speed.hi],
                        planar_basis: Some(PlanarBasis {
                            axis,
                            binormal,
                            parallel,
                            transverse,
                            longitudinal,
                            radius,
                        }),
                    },
                    next,
                    next_radius,
                ))
            })();
            if let Ok((cell, next, next_radius)) = planar_step {
                center = next;
                radius = next_radius;
                cover.push(cell);
                continue;
            }
        }
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
                    tangent: encode(unit(v)?),
                    endpoint_normal: encode(ball(next, next_radius)?),
                    endpoint_radius_upper: next_radius,
                    speed: [speed.lo, speed.hi],
                    planar_basis: None,
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
            let h = I::point(cell.traversal[1]).sub(I::point(cell.traversal[0]))?;
            total = total.add(I::new(cell.speed[0], cell.speed[1])?.mul(h)?)?;
        }
        let mut prefix = I::point(0.);
        let mut corrected = Vec::with_capacity(cover.len());
        let mut phases = Vec::with_capacity(cover.len());
        let mut binormals = Vec::with_capacity(cover.len());
        for cell in &cover {
            if out.cells == max_cells {
                return Ok(out);
            }
            out.cells += 1;
            let h = I::point(cell.traversal[1]).sub(I::point(cell.traversal[0]))?;
            let length = I::new(cell.speed[0], cell.speed[1])?.mul(h)?;
            let next_prefix = prefix.add(length)?;
            // Every partial length in this cell lies between the proved
            // previous prefix lower bound and next prefix upper bound.
            let fraction = I::new(prefix.lo.max(0.), next_prefix.hi)?
                .div(total)?
                .intersect(0., 1.)?;
            let phase = I::new(angle[0], angle[1])?.mul(fraction)?;
            phases.push([phase.lo, phase.hi]);
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
        out.correction_phases = Some(phases);
        out.correction_rate = Some(I::new(angle[0], angle[1])?.div(total)?);
        out.arc_corrected_binormals = Some(binormals);
    }
    out.status = Status::Certified;
    out.reason = None;
    out.source_jets = Some(source_jets);
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
    let mut out = frame_axes(transport, traversal, max_cells, None)?;
    if out.status != Status::Certified {
        return Ok(out);
    }
    let charge = (twist.degree..twist.control_points.len())
        .filter(|&i| twist.knots[i] < twist.knots[i + 1])
        .count();
    let theta = if charge <= max_cells - out.cells {
        out.cells += charge;
        scalar_certificate::value_traversal(twist, station, charge)?
    } else {
        None
    };
    let Some(theta) = theta else {
        return Ok(frame::ValuesReport {
            status: Status::Unresolved,
            cells: out.cells,
            longitudinal: None,
            transverse: None,
            binormal: None,
        });
    };
    let trig = trigonometric_certificate::certify(theta)?;
    let c = I::new(trig.cos[0], trig.cos[1])?;
    let s = I::new(trig.sin[0], trig.sin[1])?;
    let n = decode(out.transverse.unwrap())?;
    let b = decode(out.binormal.unwrap())?;
    out.transverse = Some(encode(add(scale(n, c)?, scale(b, s)?)?));
    out.binormal = Some(encode(sub(scale(b, c)?, scale(n, s)?)?));
    Ok(out)
}

/// `prepared_first` comes from this same immutable source query, already
/// charged by the owning caller. It encloses every intersected source cell.
/// Exact original pole coplanarity; a small nonzero determinant still refuses.
fn exact_pole_plane(poles: &[Vec<f64>], budget: usize) -> Result<(usize, Option<Vec3>)> {
    use cad_predicates::{
        AuthoredScalar, Limits, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
    };
    let mut cells = 0;
    let point_at = |i: usize| point(std::array::from_fn(|k| poles[i][k]));
    let mut basis = None;
    for i in 1..poles.len() {
        for j in i + 1..poles.len() {
            if cells == budget {
                return Ok((cells, None));
            }
            cells += 1;
            if let Ok(normal) = unit(cross(
                sub(point_at(i), point_at(0))?,
                sub(point_at(j), point_at(0))?,
            )?) {
                basis = Some((i, j, normal));
                break;
            }
        }
        if basis.is_some() {
            break;
        }
    }
    let Some((i, j, normal)) = basis else {
        return Ok((cells, None));
    };
    // A rounded candidate is only a proposal. Exact dot identities on every
    // original pole certify that represented plane before normalization.
    let u = std::array::from_fn::<_, 3, _>(|k| poles[i][k] - poles[0][k]);
    let v = std::array::from_fn::<_, 3, _>(|k| poles[j][k] - poles[0][k]);
    let candidate = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    if candidate.iter().all(|x| x.is_finite()) && candidate.iter().any(|x| *x != 0.) {
        let mut exact = true;
        for pole in poles.iter().skip(1) {
            if budget - cells < 6 {
                return Ok((cells, None));
            }
            cells += 6;
            let terms = std::array::from_fn::<_, 6, _>(|k| {
                if k < 3 {
                    (candidate[k], pole[k])
                } else {
                    (-candidate[k - 3], poles[0][k - 3])
                }
            });
            if crate::exact_products::sum_sign(&terms) != std::cmp::Ordering::Equal {
                exact = false;
                break;
            }
        }
        if exact {
            if let Ok(normal) = unit(point(candidate)) {
                return Ok((cells, Some(normal)));
            }
        }
    }
    let values = poles
        .iter()
        .flatten()
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let Ok(source) = SourceArena::authored("rmf-original-span-plane", 1, values) else {
        return Ok((cells, None));
    };
    let refs = |i: usize| std::array::from_fn(|k| source.leaf(3 * i + k).unwrap());
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work: (budget - cells) as u64,
            ..Limits::default()
        },
        None,
    );
    for k in 1..poles.len() {
        if k == i || k == j {
            continue;
        }
        let result = cad_predicates::orient3d(&mut ctx, refs(0), refs(i), refs(j), refs(k));
        if !matches!(result.map(|d| d.outcome), Ok(Outcome::Sign(Sign::Zero))) {
            return Ok((cells + ctx.work_used() as usize, None));
        }
    }
    Ok((cells + ctx.work_used() as usize, Some(normal)))
}

pub(super) fn frame_axes(
    transport: &OriginalRmfTransportReport,
    traversal: [f64; 2],
    max_cells: usize,
    prepared_first: Option<[[f64; 2]; 3]>,
) -> Result<super::super::progressive_miter::authored_frame_certificate::ValuesReport> {
    frame_axes_impl(transport, traversal, max_cells, prepared_first, true, None)
}
#[cfg(test)]
pub(super) fn original_frame_axes(
    transport: &OriginalRmfTransportReport,
    traversal: [f64; 2],
    max_cells: usize,
    prepared_first: Option<[[f64; 2]; 3]>,
) -> Result<super::super::progressive_miter::authored_frame_certificate::ValuesReport> {
    frame_axes_impl(transport, traversal, max_cells, prepared_first, false, None)
}
/// Source axes and original correction share the same charged cover traversal.
/// Partial axes never publish a partial phase enclosure.
pub(super) fn original_frame_with_correction(
    transport: &OriginalRmfTransportReport,
    traversal: [f64; 2],
    budget: usize,
    first: Option<[[f64; 2]; 3]>,
) -> Result<(
    super::super::progressive_miter::authored_frame_certificate::ValuesReport,
    Option<(I, I)>,
)> {
    let mut phase = None;
    let frame = frame_axes_impl(transport, traversal, budget, first, false, Some(&mut phase))?;
    let correction = if frame.status != Status::Certified {
        None
    } else if transport.arc_corrected_normals.is_none() {
        Some((I::point(0.), I::point(0.)))
    } else {
        phase.zip(transport.correction_rate)
    };
    Ok((frame, correction))
}

fn frame_axes_impl(
    transport: &OriginalRmfTransportReport,
    traversal: [f64; 2],
    max_cells: usize,
    prepared_first: Option<[[f64; 2]; 3]>,
    corrected: bool,
    mut phase_image: Option<&mut Option<I>>,
) -> Result<super::super::progressive_miter::authored_frame_certificate::ValuesReport> {
    use super::super::progressive_miter::authored_frame_certificate as frame;
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
        let mut t = decode(cell.tangent)?;
        let fresh_correction = transport
            .correction_phases
            .as_ref()
            .filter(|_| corrected)
            .map(|phases| phases[i]);
        let mut n = decode(
            transport
                .arc_corrected_normals
                .as_ref()
                .filter(|_| corrected && fresh_correction.is_none())
                .map_or(cell.normal, |v| v[i]),
        )?;
        // Restrict the original planar Bishop identity to this query. Closed
        // holonomy images remain owned by the separate corrected cover.
        if !corrected || transport.arc_corrected_normals.is_none() || fresh_correction.is_some() {
            if let (Some(basis), Some(source)) = (&cell.planar_basis, &transport.source_jets) {
                let interval = [
                    traversal[0].max(cell.traversal[0]),
                    traversal[1].min(cell.traversal[1]),
                ];
                if interval != cell.traversal {
                    let first = if let Some(first) = prepared_first {
                        first
                    } else {
                        let original = source.query(interval, max_cells - cells)?;
                        cells += original.cells;
                        let Some(jets) = original.jets else {
                            return Ok(unresolved(cells));
                        };
                        jets[1]
                    };
                    let restricted = (|| -> Result<(Vec3, Vec3)> {
                        let mut velocity = decode(first)?;
                        if let Some(axis) = basis.axis {
                            velocity[axis] = I::point(0.);
                        }
                        let t = unit(velocity)?;
                        let axis = basis.binormal;
                        let n = add(
                            add(
                                add(
                                    scale(axis, basis.parallel)?,
                                    scale(cross(axis, t)?, basis.transverse)?,
                                )?,
                                scale(t, basis.longitudinal)?,
                            )?,
                            [I::new(-basis.radius, basis.radius)?; 3],
                        )?;
                        Ok((t, n))
                    })();
                    if let Ok((restricted_t, restricted_n)) = restricted {
                        t = restricted_t;
                        n = restricted_n;
                    }
                }
            }
        }
        let b = if let Some(phase) = fresh_correction {
            // Rotate the restricted original source image by the already
            // charged whole-cell original arc phase. No sampled chord phase
            // or exact orthogonality of interval boxes is assumed.
            let trig = trigonometric_certificate::certify(phase)?;
            let c = I::new(trig.cos[0], trig.cos[1])?;
            let s = I::new(trig.sin[0], trig.sin[1])?;
            n = add(
                add(scale(n, c)?, scale(cross(t, n)?, s)?)?,
                scale(t, dot_tight(t, n)?.mul(I::point(1.).sub(c)?)?)?,
            )?;
            cross(t, n)?
        } else if let Some(v) = transport
            .arc_corrected_binormals
            .as_ref()
            .filter(|_| corrected)
        {
            decode(v[i])?
        } else {
            cross(t, n)?
        };
        if let (Some(image), Some(phases)) = (
            phase_image.as_deref_mut(),
            transport.correction_phases.as_ref(),
        ) {
            let phase = I::new(phases[i][0], phases[i][1])?;
            *image = Some(if let Some(old) = *image {
                I::new(old.lo.min(phase.lo), old.hi.max(phase.hi))?
            } else {
                phase
            });
        }
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
    Ok(frame::ValuesReport {
        status: Status::Certified,
        cells,
        longitudinal: Some(encode(t)),
        transverse: Some(encode(n)),
        binormal: Some(encode(b)),
    })
}

/// Original holonomy phase and its derivative per original arc length.
/// A query is complete only after every intersecting cover cell is charged.
#[cfg(test)]
pub(super) fn correction_image(
    transport: &OriginalRmfTransportReport,
    traversal: [f64; 2],
    budget: usize,
) -> Result<(usize, Option<(I, I)>)> {
    if transport.status != Status::Certified {
        return Ok((0, None));
    }
    if transport.arc_corrected_normals.is_none() {
        return Ok((0, Some((I::point(0.), I::point(0.)))));
    }
    let (Some(cover), Some(phases), Some(rate)) = (
        &transport.cover,
        &transport.correction_phases,
        transport.correction_rate,
    ) else {
        return Ok((0, None));
    };
    let mut cells = 0;
    let mut image: Option<I> = None;
    for (cell, phase) in cover.iter().zip(phases) {
        if cell.traversal[1] < traversal[0] || cell.traversal[0] > traversal[1] {
            continue;
        }
        if cells == budget {
            return Ok((cells, None));
        }
        cells += 1;
        let phase = I::new(phase[0], phase[1])?;
        image = Some(if let Some(old) = image {
            I::new(old.lo.min(phase.lo), old.hi.max(phase.hi))?
        } else {
            phase
        });
    }
    Ok((cells, image.map(|phase| (phase, rate))))
}

pub(super) fn source_jets(
    transport: &OriginalRmfTransportReport,
    traversal: [f64; 2],
    budget: usize,
) -> Result<Option<super::source_jets::Report>> {
    transport
        .source_jets
        .as_ref()
        .map(|source| source.query(traversal, budget))
        .transpose()
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
pub(super) mod tests {
    use super::*;
    #[test]
    fn joint_original_frame_phase_preserves_complete_images_and_refuses_partial_work() {
        let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 1.).unwrap();
        let proof =
            certify_original_rmf_transport_shared(&path, [1., 0., 1.], 512, 10000, true).unwrap();
        for interval in [[0.3125; 2], [0.2, 0.3], [0., 1.]] {
            let old = original_frame_axes(&proof, interval, 10000, None).unwrap();
            let (phase_work, old_phase) = correction_image(&proof, interval, 10000).unwrap();
            let (joint, phase) =
                original_frame_with_correction(&proof, interval, 10000, None).unwrap();
            assert_eq!(joint.status, Status::Certified);
            assert_eq!(joint.transverse, old.transverse);
            assert_eq!(joint.longitudinal, old.longitudinal);
            let (a, b) = phase.unwrap();
            let (c, d) = old_phase.unwrap();
            assert_eq!([a.lo, a.hi, b.lo, b.hi], [c.lo, c.hi, d.lo, d.hi]);
            assert!(joint.cells < old.cells + phase_work);
            for budget in [0, joint.cells - 1] {
                let (short, phase) =
                    original_frame_with_correction(&proof, interval, budget, None).unwrap();
                assert_eq!(short.status, Status::Unresolved);
                assert!(phase.is_none() && short.cells <= budget);
            }
        }
    }

    #[test]
    fn higher_degree_original_plane_requires_exact_coplanarity_and_complete_work() {
        // The six authored poles lie on y=z. This premise is independent of
        // their degree, rational weights and subsequent transport sampling.
        let poles = vec![
            vec![0., 0., 0.],
            vec![0.2, 0., 0.],
            vec![0.4, 0.1, 0.1],
            vec![0.6, 0.3, 0.3],
            vec![0.8, 0.6, 0.6],
            vec![1., 1., 1.],
        ];
        let (work, plane) = exact_pole_plane(&poles, 10000).unwrap();
        let normal = plane.unwrap();
        let r = 2_f64.sqrt();
        assert!(normal[0].lo <= 0. && normal[0].hi >= 0.);
        for (k, value) in [(1, -1. / r), (2, 1. / r)] {
            assert!(normal[k].lo <= value && normal[k].hi >= value);
        }
        for budget in [0, work - 1] {
            let (used, plane) = exact_pole_plane(&poles, budget).unwrap();
            assert!(plane.is_none() && used <= budget);
        }
        let mut perturbed = poles;
        perturbed[5][2] = 1_f64.next_up();
        assert!(exact_pole_plane(&perturbed, 10000).unwrap().1.is_none());
    }

    #[test]
    fn corrected_original_circle_queries_restrict_source_frame_before_rotation() {
        let path = crate::primitives::circle([0.; 3], [0., 0., 1.], 1.).unwrap();
        let proof =
            certify_original_rmf_transport_shared(&path, [1., 0., 1.], 512, 10000, true).unwrap();
        assert_eq!(proof.status, Status::Certified, "{proof:?}");
        assert!(proof.correction_phases.is_some());
        let twist = super::super::constant_vector_law([0.; 3]).unwrap();
        for i in 0..=64 {
            let t = i as f64 / 64.;
            let span = (2..path.control_points.len())
                .find(|&j| {
                    path.knots[j] < path.knots[j + 1]
                        && t >= path.knots[j]
                        && t <= path.knots[j + 1]
                })
                .unwrap();
            let f = (t - path.knots[span]) / (path.knots[span + 1] - path.knots[span]);
            let b = [(1. - f).powi(2), 2. * f * (1. - f), f * f];
            let db = [-2. * (1. - f), 2. - 4. * f, 2. * f];
            let weights = &path.weights[span - 2..=span];
            let poles = &path.control_points[span - 2..=span];
            let w = (0..3).map(|j| b[j] * weights[j]).sum::<f64>();
            let dw = (0..3).map(|j| db[j] * weights[j]).sum::<f64>();
            let v = std::array::from_fn::<_, 3, _>(|k| {
                (0..3)
                    .map(|j| db[j] * weights[j] * poles[j][k])
                    .sum::<f64>()
                    * w
                    - (0..3).map(|j| b[j] * weights[j] * poles[j][k]).sum::<f64>() * dw
            });
            let length = math_core::norm(v);
            let root = 2_f64.sqrt();
            let expected = [v[1] / length / root, -v[0] / length / root, 1. / root];
            let image = frame_values(&proof, &twist, [t; 2], [t; 2], 10000).unwrap();
            assert_eq!(image.status, Status::Certified);
            let normal = image.transverse.unwrap();
            for k in 0..3 {
                assert!(normal[k][0] <= expected[k] && expected[k] <= normal[k][1]);
                assert!(
                    normal[k][1] - normal[k][0] < 1e-7,
                    "station={i} component={k} normal={normal:?} angle={:?} end_radius={}",
                    proof.closing_angle,
                    proof
                        .cover
                        .as_ref()
                        .unwrap()
                        .last()
                        .unwrap()
                        .endpoint_radius_upper
                );
            }
            for budget in [0, image.cells - 1] {
                let short = frame_values(&proof, &twist, [t; 2], [t; 2], budget).unwrap();
                assert_eq!(short.status, Status::Unresolved);
                assert!(short.transverse.is_none() && short.cells <= budget);
            }
        }
    }
    #[test]
    fn tilted_original_quadratic_plane_has_independent_bishop_normal() {
        let path =
            crate::paths::bezier(vec![vec![0.; 3], vec![0.5, 0., 0.], vec![1., 1., 1.]], None)
                .unwrap();
        let proof =
            certify_original_rmf_transport_shared(&path, [0., 0., 1.], 512, 10000, false).unwrap();
        assert_eq!(proof.status, Status::Certified, "{proof:?}");
        assert!(proof.cover.as_ref().unwrap().iter().any(|cell| {
            cell.planar_basis
                .as_ref()
                .is_some_and(|basis| basis.axis.is_none())
        }));
        let end = proof.cover.as_ref().unwrap().last().unwrap();
        assert!(end.endpoint_radius_upper < 1e-8);
        let twist = super::super::constant_vector_law([0.; 3]).unwrap();
        // P=(t,t²,t²), fixed original plane normal (0,-1,1)/sqrt(2).
        // Bishop N=(B+B x T)/sqrt(2), from the original seed N(0)=Z.
        for i in 0..=64 {
            let t = i as f64 / 64.;
            let r = (1. + 8. * t * t).sqrt();
            let expected = [-2. * t / r, (-1. + 1. / r) / 2., (1. + 1. / r) / 2.];
            let image = frame_values(&proof, &twist, [t; 2], [t; 2], 10000).unwrap();
            assert_eq!(image.status, Status::Certified);
            let n = image.transverse.unwrap();
            for k in 0..3 {
                assert!(n[k][0] <= expected[k] && expected[k] <= n[k][1]);
                assert!(n[k][1] - n[k][0] < 1e-8);
            }
        }
        let work = proof.cells + proof.exact_work as usize;
        for budget in [0, work - 1] {
            let short =
                certify_original_rmf_transport_shared(&path, [0., 0., 1.], 512, budget, false)
                    .unwrap();
            assert_eq!(short.status, Status::Unresolved);
            assert!(short.cover.is_none());
            assert!(short.cells + short.exact_work as usize <= budget);
        }
    }
    #[test]
    fn closed_original_plane_speed_uses_normalized_nonunit_domain() {
        let mut path = crate::primitives::circle([0.; 3], [0., 0., 1.], 1.).unwrap();
        for knot in &mut path.knots {
            *knot = 2. + 3. * *knot;
        }
        let proof =
            certify_original_rmf_transport_shared(&path, [0., 0., 1.], 512, 10000, true).unwrap();
        assert_eq!(proof.status, Status::Certified, "{proof:?}");
        let work = proof.cells + proof.exact_work as usize;
        assert!(work <= 10000 && proof.arc_corrected_normals.is_some());
        for cell in proof.cover.as_ref().unwrap() {
            let t = (cell.traversal[0] + cell.traversal[1]) / 2.;
            let u = 2. + 3. * t;
            let span = (2..path.control_points.len())
                .find(|&i| {
                    path.knots[i] < path.knots[i + 1]
                        && u >= path.knots[i]
                        && u <= path.knots[i + 1]
                })
                .unwrap();
            let h = path.knots[span + 1] - path.knots[span];
            let f = (u - path.knots[span]) / h;
            let b = [(1. - f).powi(2), 2. * f * (1. - f), f * f];
            let db = [-2. * (1. - f), 2. - 4. * f, 2. * f];
            let weights = &path.weights[span - 2..=span];
            let poles = &path.control_points[span - 2..=span];
            let w = (0..3).map(|j| b[j] * weights[j]).sum::<f64>();
            let dw = (0..3).map(|j| db[j] * weights[j]).sum::<f64>();
            let velocity = std::array::from_fn(|k| {
                let p = (0..3).map(|j| b[j] * weights[j] * poles[j][k]).sum::<f64>();
                let dp = (0..3)
                    .map(|j| db[j] * weights[j] * poles[j][k])
                    .sum::<f64>();
                // Independent original rational Bernstein derivative per
                // normalized traversal, not per the [2,5] source parameter.
                3. * (dp * w - p * dw) / (h * w * w)
            });
            let speed = math_core::norm(velocity);
            assert!(
                cell.speed[0] <= speed + 1e-12 && speed <= cell.speed[1] + 1e-12,
                "{:?} misses {speed}",
                cell.speed
            );
        }
        for budget in [0, work - 1] {
            let short =
                certify_original_rmf_transport_shared(&path, [0., 0., 1.], 512, budget, true)
                    .unwrap();
            assert_eq!(short.status, Status::Unresolved);
            assert!(short.cover.is_none());
            assert!(short.cells + short.exact_work as usize <= budget);
        }
    }
    #[test]
    fn normalized_component_bounds_keep_shared_speed_and_signed_boxes() {
        for bounds in [
            [[1., 1000.], [0., 0.], [0., 0.]],
            [[-1000., -1.], [0., 0.], [0., 0.]],
            [[0.5, 2.], [-3., 4.], [-0.1, 0.2]],
            [[-2., -0.5], [-4., -3.], [0.1, 0.2]],
            [[-1., 1.], [2., 3.], [-4., 4.]],
        ] {
            let box_value = bounds.map(|v| I::new(v[0], v[1]).unwrap());
            let proved = unit(box_value).unwrap();
            let old = div(box_value, norm(box_value).unwrap()).unwrap();
            for k in 0..3 {
                assert!(proved[k].lo >= old[k].lo && proved[k].hi <= old[k].hi);
            }
            for x in [0., 0.125, 0.5, 0.875, 1.] {
                for y in [0., 0.5, 1.] {
                    for z in [0., 0.5, 1.] {
                        let actual = std::array::from_fn(|k| {
                            bounds[k][0] + [x, y, z][k] * (bounds[k][1] - bounds[k][0])
                        });
                        let length = math_core::norm(actual);
                        for k in 0..3 {
                            let value = actual[k] / length;
                            assert!(proved[k].lo - 1e-15 <= value && value <= proved[k].hi + 1e-15);
                        }
                    }
                }
            }
        }
        let axis = unit([I::new(1., 1000.).unwrap(), I::point(0.), I::point(0.)]).unwrap();
        assert!(axis[0].lo > 0.999999999999 && axis[0].hi <= 1.);
        assert!(unit([I::point(0.); 3]).is_err());
    }
    #[test]
    fn piecewise_planar_source_transport_preserves_independent_quarter_turn_frame() {
        let path = crate::paths::round_polyline(
            &[[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 10., 10.]],
            2.,
        )
        .unwrap();
        let proof =
            certify_original_rmf_transport_shared(&path, [1., 0., 0.], 512, 10000, false).unwrap();
        assert_eq!(proof.status, Status::Certified, "{proof:?}");
        assert!(proof.cells + proof.exact_work as usize <= 10000);
        let end = proof.cover.as_ref().unwrap().last().unwrap();
        // Integer quarter turns: Z->X about +Y maps initial X to -Z;
        // X->Y about +Z then preserves -Z, independently of the ODE cover.
        for (axis, expected) in [0., 0., -1.].into_iter().enumerate() {
            assert!(
                end.endpoint_normal[axis][0] <= expected
                    && expected <= end.endpoint_normal[axis][1]
            );
        }
        assert!(
            end.endpoint_radius_upper < 1e-8,
            "{}",
            end.endpoint_radius_upper
        );
        let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        let scale = super::super::constant_vector_law([1., 0., 0.]).unwrap();
        let twist = super::super::constant_vector_law([0.; 3]).unwrap();
        let sweep = super::super::Sweep::new(
            &profile,
            &path,
            &scale,
            &twist,
            super::super::Options {
                normal: [1., 0., 0.],
                orientation: super::super::Orientation::RotationMinimizing,
                spacing: super::super::Spacing::Parameter,
                initial_sections: 5,
                max_sections: 1025,
                max_deviation: 0.01,
            },
        )
        .unwrap();
        assert_eq!(path.degree, 2);
        let reference = |t: f64| {
            let domain = path.domain();
            let u = domain[0] + (domain[1] - domain[0]) * t;
            let span = (2..path.control_points.len())
                .find(|&i| {
                    path.knots[i] < path.knots[i + 1]
                        && u >= path.knots[i]
                        && u <= path.knots[i + 1]
                })
                .unwrap();
            let f = (u - path.knots[span]) / (path.knots[span + 1] - path.knots[span]);
            let bernstein = [(1. - f) * (1. - f), 2. * f * (1. - f), f * f];
            let derivative = [-2. * (1. - f), 2. - 4. * f, 2. * f];
            let weights = &path.weights[span - 2..=span];
            let poles = &path.control_points[span - 2..=span];
            let w = (0..3).map(|j| bernstein[j] * weights[j]).sum::<f64>();
            let dw = (0..3).map(|j| derivative[j] * weights[j]).sum::<f64>();
            let numerator: [f64; 3] = std::array::from_fn(|k| {
                (0..3)
                    .map(|j| bernstein[j] * weights[j] * poles[j][k])
                    .sum()
            });
            let dn: [f64; 3] = std::array::from_fn(|k| {
                (0..3)
                    .map(|j| derivative[j] * weights[j] * poles[j][k])
                    .sum()
            });
            let velocity: [f64; 3] = std::array::from_fn(|k| dn[k] * w - numerator[k] * dw);
            let speed = math_core::norm(velocity);
            let arc_end = (path.knots[5] - domain[0]) / (domain[1] - domain[0]);
            let normal = if t <= arc_end {
                [velocity[2] / speed, 0., -velocity[0] / speed]
            } else {
                [0., 0., -1.]
            };
            (numerator.map(|x| x / w), normal)
        };
        let sections = sweep.sections_at(1025).unwrap();
        let mut deviation = 0_f64;
        for (i, section) in sections.iter().enumerate() {
            let t = i as f64 / 1024.;
            let (source, normal) = reference(t);
            for (j, q) in [1., 2.].into_iter().enumerate() {
                let delta = std::array::from_fn(|k| {
                    section.control_points[j][k] - source[k] - q * normal[k]
                });
                deviation = deviation.max(math_core::norm(delta));
            }
        }
        assert!(
            deviation < 0.01,
            "Original quarter-turn versus retained frame deviation={deviation}"
        );
        let bound = sweep
            .rmf_spatial_patch_error_bound(1025, 512, 10000, 1000000)
            .unwrap();
        assert_eq!(bound.status, Status::Certified, "{bound:?}");
        assert!(
            bound.within_budget && bound.error_upper.unwrap() <= 0.01,
            "{bound:?}"
        );
        for i in 0..1024 {
            for fraction in [0.125, 0.375, 0.5, 0.625, 0.875] {
                let (source, normal) = reference((i as f64 + fraction) / 1024.);
                for (j, q) in [1., 2.].into_iter().enumerate() {
                    let delta = std::array::from_fn(|k| {
                        (1. - fraction) * sections[i].control_points[j][k]
                            + fraction * sections[i + 1].control_points[j][k]
                            - source[k]
                            - q * normal[k]
                    });
                    assert!(math_core::norm(delta) <= bound.error_upper.unwrap());
                }
            }
        }
        for budget in [0, bound.cells - 1] {
            let short = sweep
                .rmf_spatial_patch_error_bound(1025, 512, budget, 1000000)
                .unwrap();
            assert!(
                !short.within_budget,
                "Short original wall budget unexpectedly admitted: {short:?}"
            );
            assert!(short.cells <= budget);
        }
        eprintln!(
            "Rounded continuous wall: upper={:?} cells={}",
            bound.error_upper, bound.cells
        );
        let mut worst = (0_f64, 0_usize);
        for i in 0..129 {
            let t = i as f64 / 128.;
            let (_, expected) = reference(t);
            let frame = frame_values(&proof, &twist, [t; 2], [t; 2], 10000).unwrap();
            assert_eq!(frame.status, Status::Certified);
            for budget in [0, frame.cells - 1] {
                let short = frame_values(&proof, &twist, [t; 2], [t; 2], budget).unwrap();
                assert_eq!(short.status, Status::Unresolved);
                assert!(short.cells <= budget && short.transverse.is_none());
            }
            let normal = frame.transverse.unwrap();
            for k in 0..3 {
                assert!(normal[k][0] <= expected[k] && expected[k] <= normal[k][1]);
            }
            let radius = normal
                .iter()
                .zip(expected)
                .map(|(v, x)| (v[0] - x).abs().max((v[1] - x).abs()).powi(2))
                .sum::<f64>()
                .sqrt();
            if radius > worst.0 {
                worst = (radius, i);
            }
        }
        assert!(
            worst.0 < 1e-8,
            "Restricted original station frame {worst:?}"
        );
        for i in 1..128 {
            let interval = [(i as f64 - 0.125) / 128., (i as f64 + 0.125) / 128.];
            let image = frame_values(&proof, &twist, interval, interval, 10000).unwrap();
            assert_eq!(image.status, Status::Certified);
            let normal = image.transverse.unwrap();
            for j in 0..=8 {
                let t = interval[0] + (interval[1] - interval[0]) * j as f64 / 8.;
                let (_, expected) = reference(t);
                for k in 0..3 {
                    assert!(normal[k][0] <= expected[k] && expected[k] <= normal[k][1]);
                }
            }
        }
        eprintln!("Rounded frame diagnostic: retained1025={deviation} worst129normal={worst:?}");
        for cell in proof.cover.as_ref().unwrap() {
            for fraction in [0.125, 0.5, 0.875] {
                let t = cell.traversal[0] + fraction * (cell.traversal[1] - cell.traversal[0]);
                let (_, expected) = reference(t);
                for k in 0..3 {
                    assert!(
                        cell.normal[k][0] - 1e-9 <= expected[k]
                            && expected[k] <= cell.normal[k][1] + 1e-9,
                        "t={t} axis={k}"
                    );
                }
            }
        }
        for budget in [0, (proof.cells + proof.exact_work as usize) - 1] {
            let refused =
                certify_original_rmf_transport_shared(&path, [1., 0., 0.], 512, budget, false)
                    .unwrap();
            assert_eq!(refused.status, Status::Unresolved);
            assert!(refused.cover.is_none());
            assert!(refused.cells + refused.exact_work as usize <= budget);
        }
    }
    #[test]
    fn original_non_grid_knots_keep_complete_cover_and_actual_arc_cell_widths() {
        let mut path = spatial_loop();
        path.knots = vec![
            0., 0., 0., 0., 0.2, 0.2, 0.2, 0.45, 0.45, 0.45, 0.7, 0.7, 0.7, 1., 1., 1., 1.,
        ];
        let proof = certify_original_rmf_transport(&path, [1., 0., 0.], 512, 100000, 1000000, true)
            .unwrap();
        assert_eq!(proof.status, Status::Certified, "{proof:?}");
        let cover = proof.cover.as_ref().unwrap();
        assert!(cover.len() > 512);
        assert_eq!(cover.first().unwrap().traversal[0], 0.);
        assert_eq!(cover.last().unwrap().traversal[1], 1.);
        assert!(cover.iter().all(|c| c.traversal[0] < c.traversal[1]));
        assert!(
            cover
                .windows(2)
                .all(|c| c[0].traversal[1] == c[1].traversal[0])
        );
        assert_eq!(
            proof.arc_corrected_normals.as_ref().unwrap().len(),
            cover.len()
        );
        assert_eq!(
            proof.arc_corrected_binormals.as_ref().unwrap().len(),
            cover.len()
        );
        for budget in [0, proof.cells - 1] {
            let refused =
                certify_original_rmf_transport(&path, [1., 0., 0.], 512, budget, 1000000, true)
                    .unwrap();
            assert_eq!(refused.status, Status::Unresolved);
            assert!(refused.cover.is_none() && refused.closing_angle.is_none());
            assert!(refused.cells <= budget);
        }
    }
    fn spatial_loop() -> Curve {
        let vertices = [[1., 0., 0.], [0., 1., 1.], [-1., 0., 0.], [0., -1., 0.5]];
        let tangents = [
            [0., 0.25, 0.25],
            [-0.25, 0., -0.25],
            [0., -0.25, 0.25],
            [0.25, 0.125, -0.25],
        ];
        let mut poles = vec![vertices[0].to_vec()];
        for i in 0..4 {
            let j = (i + 1) % 4;
            poles.push((0..3).map(|k| vertices[i][k] + tangents[i][k]).collect());
            poles.push((0..3).map(|k| vertices[j][k] - tangents[j][k]).collect());
            poles.push(vertices[j].to_vec());
        }
        Curve {
            degree: 3,
            knots: vec![
                0., 0., 0., 0., 0.25, 0.25, 0.25, 0.5, 0.5, 0.5, 0.75, 0.75, 0.75, 1., 1., 1., 1.,
            ],
            control_points: poles,
            weights: vec![1.; 13],
            periodic: false,
        }
    }
    #[test]
    fn closed_rational_nonuniform_spans_require_original_c1_and_complete_transport() {
        let mut path = spatial_loop();
        let fractions = [0., 0.125, 0.375, 0.75, 1.];
        // Equal interior/end weight ratios and offsets proportional to each
        // span preserve the original Cartesian first jet across every knot.
        for segment in 0..4 {
            let first = 3 * segment;
            let ratio = 4. * (fractions[segment + 1] - fractions[segment]);
            for k in 0..3 {
                path.control_points[first + 1][k] = path.control_points[first][k]
                    + ratio * (path.control_points[first + 1][k] - path.control_points[first][k]);
                path.control_points[first + 2][k] = path.control_points[first + 3][k]
                    + ratio
                        * (path.control_points[first + 2][k] - path.control_points[first + 3][k]);
            }
            path.weights[first + 1] = 1.25;
            path.weights[first + 2] = 1.25;
        }
        path.knots = vec![
            0., 0., 0., 0., 0.125, 0.125, 0.125, 0.375, 0.375, 0.375, 0.75, 0.75, 0.75, 1., 1., 1.,
            1.,
        ];
        let before = path.clone();
        let report =
            certify_original_rmf_transport(&path, [1., 0., 0.], 4096, 100000, 1000000, true)
                .unwrap();
        assert_eq!(report.status, Status::Certified, "{report:?}");
        assert_eq!(report.cover.as_ref().unwrap().len(), 4096);
        let angle = report.closing_angle.unwrap();
        assert!(angle.iter().all(|x| x.is_finite()) && angle[0] <= angle[1]);
        assert!(report.arc_corrected_normals.is_some() && report.arc_corrected_binormals.is_some());
        assert_eq!(path, before);
        use crate::progressive_sweep::{Options, Orientation, Spacing, Sweep, constant_vector_law};
        let total = report.cells + report.exact_work as usize;
        let shared =
            certify_original_rmf_transport_shared(&path, [1., 0., 0.], 4096, total, true).unwrap();
        assert_eq!(shared.status, Status::Certified);
        assert!(shared.cells + shared.exact_work as usize <= total);
        for budget in [0, total - 1] {
            let limited =
                certify_original_rmf_transport_shared(&path, [1., 0., 0.], 4096, budget, true)
                    .unwrap();
            assert_eq!(limited.status, Status::Unresolved);
            assert!(limited.cells + limited.exact_work as usize <= budget);
            assert!(
                limited.cover.is_none()
                    && limited.closing_angle.is_none()
                    && limited.arc_corrected_normals.is_none()
                    && limited.arc_corrected_binormals.is_none()
            );
        }
        let profile = crate::primitives::line([1.1, 0., 0.], [1.2, 0., 0.]).unwrap();
        let scale = constant_vector_law([1.1, 0., 0.]).unwrap();
        let twist = constant_vector_law([0.125, 0., 0.]).unwrap();
        let axes = constant_vector_law([1., 1.25, 0.75]).unwrap();
        let center = constant_vector_law([0.01, -0.02, 0.03]).unwrap();
        let contours = [
            crate::primitives::circle([1., 0., 0.], [0., 1., 1.], 0.05).unwrap(),
            crate::primitives::circle([1., 0., 0.], [0., -1., -1.], 0.02).unwrap(),
        ];
        let multi = crate::progressive_sweep::MultiSweep::new(
            &contours,
            &path,
            &scale,
            &twist,
            Options {
                normal: [1., 0., 0.],
                orientation: Orientation::RotationMinimizing,
                spacing: Spacing::ArcLength {
                    tolerance: 0.001,
                    max_cells: 100000,
                },
                initial_sections: 65,
                max_sections: 65,
                max_deviation: 0.4,
            },
        )
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap()
        .with_spatial_rmf_error_limits(4096, 100000, 1000000)
        .unwrap()
        .preview_at(65)
        .unwrap();
        assert!(
            multi.report.accepted && multi.report.continuous_bound,
            "{:?}",
            multi.report
        );
        assert!(multi.report.error_certificate_cells <= 100000);
        let refused_budget = crate::progressive_sweep::MultiSweep::new(
            &contours,
            &path,
            &scale,
            &twist,
            Options {
                normal: [1., 0., 0.],
                orientation: Orientation::RotationMinimizing,
                spacing: Spacing::ArcLength {
                    tolerance: 0.001,
                    max_cells: 100000,
                },
                initial_sections: 65,
                max_sections: 65,
                max_deviation: 0.25,
            },
        )
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap()
        .with_spatial_rmf_error_limits(4096, 100000, 1000000)
        .unwrap()
        .preview_at(65)
        .unwrap();
        assert!(refused_budget.report.continuous_bound && !refused_budget.report.accepted);
        assert_eq!(
            refused_budget.report.continuous_error_upper,
            multi.report.continuous_error_upper
        );
        eprintln!(
            "rational shared contours cells={} upper={:?}",
            multi.report.error_certificate_cells, multi.report.continuous_error_upper
        );
        for spacing in [
            Spacing::Parameter,
            Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
        ] {
            let sweep = Sweep::new(
                &profile,
                &path,
                &scale,
                &twist,
                Options {
                    normal: [1., 0., 0.],
                    orientation: Orientation::RotationMinimizing,
                    spacing,
                    initial_sections: 65,
                    max_sections: 65,
                    max_deviation: 2.,
                },
            )
            .unwrap()
            .with_affine_laws(&axes, &center)
            .unwrap();
            let retained = sweep
                .rmf_spatial_patch_error_bound(65, 4096, 100000, 1000000)
                .unwrap();
            assert_eq!(
                retained.status,
                Status::Certified,
                "{spacing:?}: {retained:?}"
            );
            assert!(retained.within_budget && retained.error_upper.is_some());
        }
        let short = certify_original_rmf_transport(
            &path,
            [1., 0., 0.],
            4096,
            report.cells - 1,
            1000000,
            true,
        )
        .unwrap();
        assert_eq!(short.status, Status::Unresolved);
        assert!(
            short.cover.is_none()
                && short.closing_angle.is_none()
                && short.arc_corrected_normals.is_none()
        );
        let mut changed_speed = path.clone();
        changed_speed.weights[1] = f64::from_bits(1.25_f64.to_bits() + 1);
        assert!(
            !endpoint_jets::certify(&changed_speed, 1, 1000000)
                .unwrap()
                .certified
        );
        assert!(
            endpoint_jets::certify_direction(&changed_speed, 0, true, false, 1000000)
                .unwrap()
                .certified
        );
        let changed = certify_original_rmf_transport(
            &changed_speed,
            [1., 0., 0.],
            4096,
            100000,
            1000000,
            true,
        )
        .unwrap();
        assert_eq!(changed.status, Status::Certified);
        // A speed jump preserves unit tangent; an actual tangent perturbation
        // must still discard the complete transport and holonomy certificate.
        let mut broken = path;
        broken.control_points[1][2] = broken.control_points[1][2].next_up();
        let refused =
            certify_original_rmf_transport(&broken, [1., 0., 0.], 4096, 100000, 1000000, true)
                .unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert!(refused.cover.is_none() && refused.closing_angle.is_none());
        eprintln!(
            "rational nonuniform closed RMF cells={} angle={angle:?}",
            report.cells
        );
    }
    #[test]
    fn antipodal_closed_bishop_transport_encloses_both_principal_branches() {
        let directions = [
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [1., 0., 0.],
            [0., 1., 0.],
            [-1., 0., 0.],
            [0., 0., -1.],
            [0., -1., 0.],
            [-1., 0., 0.],
            [0., -1., 0.],
            [1., 0., 0.],
        ];
        let mut points = vec![vec![0.; 3]];
        let mut position = [0.; 3];
        let mut independent_normal = [0., 0., 1.];
        for pair in directions.windows(2) {
            points.push((0..3).map(|k| position[k] + pair[0][k]).collect());
            position = std::array::from_fn(|k| position[k] + pair[0][k] + pair[1][k]);
            points.push(position.to_vec());
            // Each planar quadratic has positive curvature and turns its
            // tangent by exactly a quarter turn. This integer Rodrigues
            // calculation is independent of interval integration.
            let axis = math_core::cross(pair[0], pair[1]);
            let cross = math_core::cross(axis, independent_normal);
            let parallel = math_core::dot(axis, independent_normal);
            independent_normal = std::array::from_fn(|k| cross[k] + axis[k] * parallel);
        }
        assert_eq!(position, [0.; 3]);
        assert_eq!(independent_normal, [0., 0., -1.]);
        let mut knots = vec![0.; 3];
        for i in 1..10 {
            knots.extend([i as f64; 2]);
        }
        knots.extend([10.; 3]);
        let path = Curve {
            degree: 2,
            knots,
            weights: vec![1.; points.len()],
            control_points: points,
            periodic: false,
        };
        let report =
            certify_original_rmf_transport(&path, [0., 0., 1.], 4096, 100000, 1000000, true)
                .unwrap();
        assert_eq!(report.status, Status::Certified);
        let angle = report.closing_angle.unwrap();
        assert!(angle[0] < -std::f64::consts::PI && angle[1] > std::f64::consts::PI);
        let cover = report.cover.as_ref().unwrap();
        assert!(cover.len() >= 4096);
        assert_eq!(cover.first().unwrap().traversal[0], 0.);
        assert_eq!(cover.last().unwrap().traversal[1], 1.);
        assert!(
            cover
                .windows(2)
                .all(|c| c[0].traversal[1] == c[1].traversal[0])
        );
        assert_eq!(
            report.arc_corrected_normals.as_ref().unwrap().len(),
            cover.len()
        );
        assert_eq!(
            report.arc_corrected_binormals.as_ref().unwrap().len(),
            cover.len()
        );
        // The branch-cut enclosure does not bypass the correction work budget
        // or publish a partial normal/binormal image.
        let refused = certify_original_rmf_transport(
            &path,
            [0., 0., 1.],
            4096,
            report.cells - 1,
            1000000,
            true,
        )
        .unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert!(refused.cover.is_none() && refused.closing_angle.is_none());
        assert!(
            refused.arc_corrected_normals.is_none() && refused.arc_corrected_binormals.is_none()
        );
        assert!(!report.continuous_bound && !report.solid_certified);
        use crate::progressive_sweep::{Options, Orientation, Spacing, Sweep, constant_vector_law};
        let profile = crate::primitives::line([0.01, 0., 0.], [0.02, 0., 0.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0., 0., 0.]).unwrap();
        for spacing in [
            Spacing::Parameter,
            Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100000,
            },
        ] {
            let sweep = Sweep::new(
                &profile,
                &path,
                &scale,
                &twist,
                Options {
                    normal: [0., 0., 1.],
                    orientation: Orientation::RotationMinimizing,
                    spacing,
                    initial_sections: 65,
                    max_sections: 65,
                    max_deviation: 1.,
                },
            )
            .unwrap();
            let bound = sweep
                .rmf_spatial_patch_error_bound(65, 4096, 100000, 1000000)
                .unwrap();
            assert_eq!(bound.status, Status::Certified, "{spacing:?}: {bound:?}");
            assert!(bound.within_budget && bound.error_upper.is_some());
            // A finite source enclosure is not permission to exceed tolerance.
            let strict = Sweep::new(
                &profile,
                &path,
                &scale,
                &twist,
                Options {
                    normal: [0., 0., 1.],
                    orientation: Orientation::RotationMinimizing,
                    spacing,
                    initial_sections: 65,
                    max_sections: 65,
                    max_deviation: 1e-6,
                },
            )
            .unwrap()
            .rmf_spatial_patch_error_bound(65, 4096, 100000, 1000000)
            .unwrap();
            assert_eq!(strict.status, Status::Certified);
            assert!(!strict.within_budget);
            // The first-defect owner optionally refines against the requested
            // tolerance; a stricter request may tighten its complete enclosure.
            assert!(strict.error_upper.unwrap() <= bound.error_upper.unwrap());
            assert!(strict.error_upper.unwrap() > 1e-6);
        }
        let mut broken = path;
        broken.control_points[1][0] = f64::from_bits(1f64.to_bits() + 1);
        let refused =
            certify_original_rmf_transport(&broken, [0., 0., 1.], 4096, 100000, 1000000, true)
                .unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert!(refused.cover.is_none() && refused.closing_angle.is_none());
    }
    // Independent numerical comparison only: explicit cubic Bernstein jets
    // and RK4 do not participate in the production enclosure/admission.
    pub(crate) fn oracle(path: &Curve, steps: usize) -> Vec<[f64; 3]> {
        let mut n = [1., 0., 0.];
        let mut result = vec![n];
        let per_segment = 16384usize;
        let h = 1. / per_segment as f64;
        let per_cell = per_segment / (steps / 4);
        let rhs = |segment: usize, t: f64, n: [f64; 3]| {
            let p = &path.control_points[3 * segment..3 * segment + 4];
            let v = std::array::from_fn(|k| {
                3. * (1. - t).powi(2) * (p[1][k] - p[0][k])
                    + 6. * t * (1. - t) * (p[2][k] - p[1][k])
                    + 3. * t * t * (p[3][k] - p[2][k])
            });
            let a = std::array::from_fn(|k| {
                6. * (1. - t) * (p[2][k] - 2. * p[1][k] + p[0][k])
                    + 6. * t * (p[3][k] - 2. * p[2][k] + p[1][k])
            });
            let omega = math_core::cross(v, a).map(|x| x / math_core::dot(v, v));
            math_core::cross(omega, n)
        };
        for segment in 0..4 {
            for i in 0..per_segment {
                let t = i as f64 * h;
                let k1 = rhs(segment, t, n);
                let k2 = rhs(
                    segment,
                    t + h * 0.5,
                    std::array::from_fn(|k| n[k] + h * 0.5 * k1[k]),
                );
                let k3 = rhs(
                    segment,
                    t + h * 0.5,
                    std::array::from_fn(|k| n[k] + h * 0.5 * k2[k]),
                );
                let k4 = rhs(segment, t + h, std::array::from_fn(|k| n[k] + h * k3[k]));
                n = std::array::from_fn(|k| {
                    n[k] + h * (k1[k] + 2. * k2[k] + 2. * k3[k] + k4[k]) / 6.
                });
                if (i + 1) % per_cell == 0 {
                    result.push(n);
                }
            }
        }
        result
    }
    #[test]
    fn closed_spatial_source_encloses_nonzero_holonomy_and_discards_partial_work() {
        let path = spatial_loop();
        let before = path.clone();
        let r = certify_original_rmf_transport(&path, [1., 0., 0.], 4096, 100000, 1000000, true)
            .unwrap();
        assert_eq!(r.status, Status::Certified, "{r:?}");
        let coarse =
            certify_original_rmf_transport(&path, [1., 0., 0.], 512, 100000, 1000000, true)
                .unwrap();
        assert_eq!(coarse.status, Status::Certified);
        let coarse_radius = coarse
            .cover
            .as_ref()
            .unwrap()
            .last()
            .unwrap()
            .endpoint_radius_upper;
        let fine_radius = r
            .cover
            .as_ref()
            .unwrap()
            .last()
            .unwrap()
            .endpoint_radius_upper;
        assert!(
            fine_radius < coarse_radius * 0.5,
            "source refinement must reduce certified accumulated uncertainty"
        );
        eprintln!("spatial RMF endpoint radii: 512={coarse_radius}, 4096={fine_radius}");
        let cover = r.cover.as_ref().unwrap();
        assert_eq!(cover.len(), 4096);
        let independent = oracle(&path, 4096);
        for (i, cell) in cover.iter().enumerate() {
            for k in 0..3 {
                let x = independent[i + 1][k];
                assert!(
                    cell.endpoint_normal[k][0] - 1e-10 <= x
                        && x <= cell.endpoint_normal[k][1] + 1e-10
                );
                assert!(
                    cell.normal[k][0] - 1e-10 <= independent[i][k]
                        && independent[i][k] <= cell.normal[k][1] + 1e-10
                );
            }
        }
        let end = independent.last().unwrap();
        let angle = ((end[2] - end[1]) / 2f64.sqrt()).atan2(end[0]);
        assert!(
            angle.abs() > 1e-3,
            "spatial fixture must have nonzero holonomy: {angle}"
        );
        let section = certify_original_rmf_section_images(
            &path,
            [1., 0., 0.],
            &[[[2., 2.], [-3., -3.], [0.5, 0.5]]],
            4096,
            100000,
            1000000,
            true,
        )
        .unwrap();
        assert_eq!(section.transport.status, Status::Certified);
        let corrected = r.arc_corrected_normals.as_ref().unwrap();
        assert_eq!(corrected.len(), cover.len());
        assert_eq!(
            r.arc_corrected_binormals.as_ref().unwrap().len(),
            cover.len()
        );
        // Independent numerical arc quadrature and Rodrigues comparison.
        // These sampled checks do not participate in production admission.
        let derivative = |u: f64| {
            let segment = ((u * 4.).floor() as usize).min(3);
            let t = u * 4. - segment as f64;
            let p = &path.control_points[3 * segment..3 * segment + 4];
            std::array::from_fn::<_, 3, _>(|k| {
                4. * (3. * (1. - t).powi(2) * (p[1][k] - p[0][k])
                    + 6. * t * (1. - t) * (p[2][k] - p[1][k])
                    + 3. * t * t * (p[3][k] - p[2][k]))
            })
        };
        let speeds: Vec<_> = (0..4096)
            .map(|i| {
                let u = (i as f64 + 0.5) / 4096.;
                let d = derivative(u);
                d.iter().map(|x| x * x).sum::<f64>().sqrt()
            })
            .collect();
        let total: f64 = speeds.iter().sum();
        let mut prefix = 0.;
        for i in 0..4096 {
            prefix += speeds[i];
            let phase = angle * prefix / total;
            let d = derivative((i + 1) as f64 / 4096.);
            let speed = d.iter().map(|x| x * x).sum::<f64>().sqrt();
            let t = [d[0] / speed, d[1] / speed, d[2] / speed];
            let n = independent[i + 1];
            let cross = math_core::cross(t, n);
            let dot = math_core::dot(t, n);
            let rotated: [f64; 3] = std::array::from_fn(|k| {
                n[k] * phase.cos() + cross[k] * phase.sin() + t[k] * dot * (1. - phase.cos())
            });
            let binormal = math_core::cross(t, rotated);
            for k in 0..3 {
                let image = 2. * rotated[k] - 3. * binormal[k] + 0.5 * t[k];
                let enclosed = section.images.as_ref().unwrap()[i][0][k];
                assert!(enclosed[0] - 1e-8 <= image && image <= enclosed[1] + 1e-8);
                let value =
                    n[k] * phase.cos() + cross[k] * phase.sin() + t[k] * dot * (1. - phase.cos());
                assert!(corrected[i][k][0] - 1e-8 <= value && value <= corrected[i][k][1] + 1e-8);
            }
        }
        let bound = r.closing_angle.unwrap();
        assert!(bound[0] <= angle && angle <= bound[1]);
        assert!(bound[1] - bound[0] < 1.);
        assert!(!r.continuous_bound && !r.solid_certified);
        let short =
            certify_original_rmf_transport(&path, [1., 0., 0.], 4096, r.cells - 1, 1000000, true)
                .unwrap();
        assert!(
            short.cover.is_none()
                && short.closing_angle.is_none()
                && short.arc_corrected_normals.is_none()
                && short.cells <= r.cells - 1
        );
        assert!(
            certify_original_rmf_transport(
                &path,
                [1., 0., 0.],
                4096,
                100000,
                r.exact_work - 1,
                true
            )
            .unwrap()
            .cover
            .is_none()
        );
        let mut broken = path.clone();
        broken.control_points[1][0] = f64::from_bits(broken.control_points[1][0].to_bits() + 1);
        assert!(
            certify_original_rmf_transport(&broken, [1., 0., 0.], 4096, 100000, 1000000, true)
                .unwrap()
                .cover
                .is_none()
        );
        assert_eq!(path, before);
    }
    #[test]
    fn section_images_cover_all_axes_and_refuse_partial_coordinates() {
        let path = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![0., 0., 3.]],
            weights: vec![1., 1.],
            periodic: false,
        };
        let coordinates = [
            [[2., 2.], [-3., -3.], [4., 4.]],
            [[0., 1.], [2., 3.], [-1., 0.]],
        ];
        let r = certify_original_rmf_section_images(
            &path,
            [1., 0., 0.],
            &coordinates,
            8,
            1000,
            1000000,
            false,
        )
        .unwrap();
        assert_eq!(r.transport.status, Status::Certified);
        for cell in r.images.as_ref().unwrap() {
            for (k, value) in [2., -3., 4.].into_iter().enumerate() {
                assert!(cell[0][k][0] <= value && value <= cell[0][k][1]);
                assert!(
                    cell[1][k][0] <= coordinates[1][k][0] && coordinates[1][k][1] <= cell[1][k][1]
                );
            }
        }
        let short = certify_original_rmf_section_images(
            &path,
            [1., 0., 0.],
            &coordinates,
            8,
            r.transport.cells - 1,
            1000000,
            false,
        )
        .unwrap();
        assert!(short.images.is_none() && short.transport.cover.is_none());
        assert!(!r.transport.continuous_bound && !r.transport.solid_certified);
    }
    #[test]
    fn straight_original_cover_keeps_full_domain_budget_and_seed_scope() {
        let path = crate::primitives::line([0., 0., 0.], [0., 0., 4.]).unwrap();
        let r =
            certify_original_rmf_transport(&path, [1., 0., 0.], 16, 10000, 100000, false).unwrap();
        assert_eq!(r.status, Status::Certified, "{r:?}");
        assert!(!r.continuous_bound && !r.solid_certified && r.closing_angle.is_none());
        for c in r.cover.as_ref().unwrap() {
            for k in 0..3 {
                let x = if k == 0 { 1. } else { 0. };
                assert!(c.normal[k][0] <= x && x <= c.normal[k][1]);
            }
            assert!(c.endpoint_radius_upper < 1e-8);
        }
        let short =
            certify_original_rmf_transport(&path, [1., 0., 0.], 16, r.cells - 1, 100000, false)
                .unwrap();
        assert!(short.cover.is_none() && short.cells <= r.cells - 1);
        assert!(
            certify_original_rmf_transport(&path, [1., 0., 0.], 16, 10000, r.exact_work - 1, false)
                .unwrap()
                .cover
                .is_none()
        );
        assert!(
            certify_original_rmf_transport(&path, [0., 0., 1.], 16, 10000, 100000, false)
                .unwrap()
                .cover
                .is_none()
        );
    }
}
