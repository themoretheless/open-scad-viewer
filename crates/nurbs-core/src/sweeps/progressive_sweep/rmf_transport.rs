//! Original-curve Bishop transport, independent of discrete reflections.
//! For v=P', a=P'', omega=(v x a)/|v|^2 and N'=omega x N.
//! Skew flows are isometries: transported uncertainty adds, without a
//! spurious exp(total curvature) multiplier. Each cell encloses the original
//! omega, then charges variation and a constant-skew quadratic remainder.
//! This is an uncorrected source-normal cover, not a sweep/Solid certificate.
mod frame_jets;
mod closed_frame_jets;
pub(in crate::sweeps::progressive_sweep) use closed_frame_jets::{closed_frame_jets,closed_phase_jets};
pub(in crate::sweeps::progressive_sweep) use frame_jets::{frame_jets,phase_jets};
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
    /// Complete connected-position/forward-tangent arc premises were needed
    /// instead of original Cartesian C1. This is clock-policy metadata only;
    /// frame Gk, boundary error, global embedding and Solid remain separate.
    pub used_geometric_arc_join_premises: bool,
    pub cover: Option<Vec<RmfNormalCell>>,
    /// Original closing-to-initial Bishop rotation, before correction.
    pub closing_angle: Option<[f64; 2]>,
    /// SAME original total arc length paid for by the corrected cover owner.
    /// Present only with a complete closed arc-holonomy correction.
    pub original_arc_length: Option<[f64; 2]>,
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

/// Original piecewise positive-speed path with exact C0 position and common
/// forward tangent values. Parameter speeds may jump. The Bishop ODE is
/// integrated in the original traversal on each span; bounded measurable
/// angular fields and continuous tangent values preserve the normal globally.
/// Arc phase jets and all source/retained Gk remain separate obligations.
pub fn certify_original_arc_rmf_transport(path:&Curve,normal:[f64;3],steps:usize,max_cells:usize,max_exact_work:u64,closed:bool)->Result<OriginalRmfTransportReport>{
    certify_original_rmf_transport_impl(path,normal,steps,max_cells,max_exact_work,closed,None)
}
pub(super) fn certify_original_rmf_transport_for_spacing(path:&Curve,normal:[f64;3],steps:usize,max_cells:usize,max_exact_work:u64,closed:bool,_spacing:super::Spacing)->Result<OriginalRmfTransportReport>{
    certify_original_rmf_transport_impl(path,normal,steps,max_cells,max_exact_work,closed,None)
}
pub(crate) fn certify_original_rmf_transport_shared_for_spacing(path:&Curve,normal:[f64;3],steps:usize,max_work:usize,closed:bool,_spacing:super::Spacing)->Result<OriginalRmfTransportReport>{
    certify_original_rmf_transport_impl(path,normal,steps,max_work,max_work as u64,closed,Some(max_work))
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
                transport.original_arc_length = None;
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
#[path = "rmf_transport/tests/tests_fixtures.rs"]
pub(super) mod tests ;

#[path = "rmf_transport/bishop_integration.rs"]
mod bishop_integration;
use bishop_integration::certify_original_rmf_transport_impl;

#[cfg(test)]
mod arc_clock_tests;
