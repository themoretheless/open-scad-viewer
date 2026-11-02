//! Closed circular-edge chamfers. Nearest-distance lower bounds are independent
//! of the matched ORIGINAL curve correspondence that supplies the upper bound.
use super::*;
use nurbs_core::{interval_eval::Interval as I, moving_radius::curves as law};
pub enum CircularSetbackLower {
    /// Reverse triangle inequality bounds EVERY original/actual point pair.
    Radial {
        source_radius: law::Certificate,
        actual_radius: law::Certificate,
    },
    /// Original/actual projected control hulls bound EVERY point pair.
    Axial {
        origin: [f64; 3],
        normal: [f64; 3],
        source_height: [f64; 2],
        actual_height: [f64; 2],
        normal_length: [f64; 2],
    },
}
/// Whole nearest distance from the ACTUAL saved rail to the complete ORIGINAL
/// edge image. The full matched clock gives an upper bound only; the retained
/// radial/plane separation gives a separate universal lower bound.
pub struct CircularSetbackCertificate {
    original: Curve,
    actual: Curve,
    edge: usize,
    requested: f64,
    distance: [f64; 2],
    error: f64,
    matched: law::Certificate,
    lower: CircularSetbackLower,
}
impl CircularSetbackCertificate {
    pub fn original_curve(&self) -> &Curve {
        &self.original
    }
    pub fn actual_curve(&self) -> &Curve {
        &self.actual
    }
    pub fn edge_index(&self) -> usize {
        self.edge
    }
    pub fn requested(&self) -> f64 {
        self.requested
    }
    pub fn distance_interval(&self) -> [f64; 2] {
        self.distance
    }
    pub fn error_upper(&self) -> f64 {
        self.error
    }
    pub fn matched_distance(&self) -> &law::Certificate {
        &self.matched
    }
    pub fn lower_bound(&self) -> &CircularSetbackLower {
        &self.lower
    }
}
pub struct CircularChamferTransition {
    topology: TopologyTransition,
    setbacks: [Vec<CircularSetbackCertificate>; 2],
}
impl std::ops::Deref for CircularChamferTransition {
    type Target = TopologyTransition;
    fn deref(&self) -> &Self::Target {
        &self.topology
    }
}
impl CircularChamferTransition {
    pub fn into_model(self) -> Model {
        self.topology.into_model()
    }
    /// Order is planar support, nonplanar wall; source_supports() has this order.
    pub fn setbacks(&self) -> &[Vec<CircularSetbackCertificate>; 2] {
        &self.setbacks
    }
}
fn certify(
    a: &Curve,
    b: &Curve,
    reverse: bool,
    requested: f64,
    tolerance: f64,
    work: &mut Ledger,
    parts: usize,
) -> Result<law::Certificate> {
    let mut report = law::qualify(
        [a, b],
        reverse,
        requested,
        tolerance,
        work.allowance(parts)?,
        work.exact_remaining() / parts as u64,
    )?;
    work.charge(report.cells, report.work)?;
    report.certificate.take().ok_or_else(|| {
        refused(&format!(
            "Whole original circular setback distance remains unproven: {}; {:?}",
            report.reason, report.uncertain_t
        ))
    })
}
fn projected(c: &Curve, origin: [f64; 3], n: [f64; 3], work: &mut Ledger) -> Result<I> {
    c.validate()?;
    if c.periodic || c.control_points[0].len() != 3 || c.weights.iter().any(|w| *w <= 0.) {
        return Err(refused(
            "Circular projected setback needs positive original 3D curves",
        ));
    }
    work.charge(c.control_points.len(), 0)?;
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for p in &c.control_points {
        let mut v = I::point(0.);
        for k in 0..3 {
            v = v.add(
                I::point(p[k])
                    .sub(I::point(origin[k]))?
                    .mul(I::point(n[k]))?,
            )?;
        }
        lo = lo.min(v.lo);
        hi = hi.max(v.hi);
    }
    Ok(I::new(lo, hi)?)
}
fn length(n: [f64; 3]) -> Result<I> {
    let mut sum = I::point(0.);
    for x in n {
        sum = sum.add(I::point(x).mul(I::point(x))?)?;
    }
    Ok(I::new(
        sum.lo.max(0.).sqrt().next_down().max(0.),
        sum.hi.sqrt().next_up(),
    )?)
}
fn reverse_proposal(a: &Curve, b: &Curve, requested: f64) -> Result<bool> {
    let x = a.evaluate(a.domain()[0])?.point;
    let d = b.domain();
    let mut errors = [0.; 2];
    for i in 0..2 {
        let y = b.evaluate(d[i])?.point;
        errors[i] = ((0..3).map(|k| (x[k] - y[k]).powi(2)).sum::<f64>().sqrt() - requested).abs();
    }
    Ok(errors[1] < errors[0])
}
fn qualify(
    source: &Model,
    selected: usize,
    mut topology: TopologyTransition,
    frame: circular::Frame,
    requested: [f64; 2],
) -> Result<CircularChamferTransition> {
    let original = &source.edges[selected].curve;
    let center = Curve::from_polyline(vec![frame.origin.to_vec(); 2])?;
    let mut certificates = [Vec::new(), Vec::new()];
    for side in 0..2 {
        for &edge in &topology.rails[side] {
            let actual = topology.model().edges[edge].curve.clone();
            let reverse = reverse_proposal(&actual, original, requested[side])?;
            let matched = certify(
                &actual,
                original,
                reverse,
                requested[side],
                source.tolerance_mm / 8.,
                &mut topology.work,
                4,
            )?;
            let (lower, proof) = if side == 0 {
                let source_radius = certify(
                    original,
                    &center,
                    false,
                    frame.radius,
                    source.tolerance_mm / 16.,
                    &mut topology.work,
                    3,
                )?;
                let radius = frame.radius - frame.sign * requested[0];
                let actual_radius = certify(
                    &actual,
                    &center,
                    false,
                    radius,
                    source.tolerance_mm / 16.,
                    &mut topology.work,
                    2,
                )?;
                let error = I::point(source_radius.error_upper())
                    .add(I::point(actual_radius.error_upper()))?;
                let gap = I::point(frame.radius).sub(I::point(radius))?;
                let minimum = if gap.lo > 0. {
                    gap.lo
                } else if gap.hi < 0. {
                    -gap.hi
                } else {
                    0.
                };
                let lower = I::point(minimum).sub(error)?.lo.max(0.);
                (
                    lower,
                    CircularSetbackLower::Radial {
                        source_radius,
                        actual_radius,
                    },
                )
            } else {
                let a = projected(original, frame.origin, frame.normal, &mut topology.work)?;
                let b = projected(&actual, frame.origin, frame.normal, &mut topology.work)?;
                let n = length(frame.normal)?;
                if n.lo <= 0. {
                    return Err(refused("Original circular plane direction unresolved"));
                }
                let gap = b.sub(a)?;
                let minimum = if gap.lo > 0. {
                    gap.lo
                } else if gap.hi < 0. {
                    -gap.hi
                } else {
                    0.
                };
                let lower = I::point(minimum).div(n)?.lo.max(0.);
                (
                    lower,
                    CircularSetbackLower::Axial {
                        origin: frame.origin,
                        normal: frame.normal,
                        source_height: [a.lo, a.hi],
                        actual_height: [b.lo, b.hi],
                        normal_length: [n.lo, n.hi],
                    },
                )
            };
            let upper = I::point(requested[side])
                .add(I::point(matched.error_upper()))?
                .hi;
            let distance = I::new(lower, upper)?;
            let residual = distance.sub(I::point(requested[side]))?;
            let error = residual.lo.abs().max(residual.hi.abs());
            if error > source.tolerance_mm {
                return Err(refused(
                    "Actual saved rail fails complete nearest circular setback",
                ));
            }
            certificates[side].push(CircularSetbackCertificate {
                original: original.clone(),
                actual,
                edge,
                requested: requested[side],
                distance: [distance.lo, distance.hi],
                error,
                matched,
                lower: proof,
            });
        }
    }
    Ok(CircularChamferTransition {
        topology,
        setbacks: certificates,
    })
}
/// Asymmetric closed chamfer of a circular original edge. Distances are ordered
/// [planar radial setback, wall axial setback], including inner walls. Native
/// original four-face cuts/closure and nearest-distance bounds on every ACTUAL
/// saved contact rail are mandatory. No G1/fillet/radius-envelope claim applies
/// to the sharp chamfer. General spines/end discovery and corner networks remain
/// separate; partial or exhausted construction publishes no body.
pub fn chamfer_circular_edge(
    source: &Model,
    edge: usize,
    setbacks: [f64; 2],
    limits: Limits,
) -> Result<CircularChamferTransition> {
    source.validate()?;
    if setbacks
        .iter()
        .any(|v| !v.is_finite() || *v <= source.tolerance_mm)
        || !(256..=100000).contains(&limits.cells)
        || !(1..=1000000).contains(&limits.exact_work)
    {
        return Err(refused(
            "Choose two resolved circular setbacks and bounded original work",
        ));
    }
    let frame = circular::frame(source, edge)?;
    let radius = frame.radius - frame.sign * setbacks[0];
    if radius <= source.tolerance_mm * 16. {
        return Err(refused("Circular chamfer collapses its planar contact"));
    }
    let original = &source.edges[edge].curve;
    let factor = radius / frame.radius;
    let plane = original
        .control_points
        .iter()
        .map(|p| {
            (0..3)
                .map(|k| frame.origin[k] + factor * (p[k] - frame.origin[k]))
                .collect()
        })
        .collect();
    let wall = original
        .control_points
        .iter()
        .map(|p| {
            (0..3)
                .map(|k| p[k] - setbacks[1] * frame.normal[k])
                .collect()
        })
        .collect();
    let surface = Surface {
        degree_u: 1,
        degree_v: original.degree,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: original.knots.clone(),
        control_points: vec![plane, wall],
        weights: vec![original.weights.clone(); 2],
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    let (contours, curves) = circular::boundaries(&surface, original.domain())?;
    let topology = insert_four_faces(
        source,
        edge,
        &surface,
        contours.each_ref(),
        Some(&curves),
        limits,
    )?;
    qualify(source, edge, topology, frame, setbacks)
}
