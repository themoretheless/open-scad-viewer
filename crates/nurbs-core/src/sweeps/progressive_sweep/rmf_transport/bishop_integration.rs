//! Bishop integration: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn certify_original_rmf_transport_impl(
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
        used_geometric_arc_join_premises: false,
        cover: None,
        closing_angle: None,
        original_arc_length: None,
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
        super::super::source_jets::SourceJets::new_order(path, 2, max_cells - out.cells)?;
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
