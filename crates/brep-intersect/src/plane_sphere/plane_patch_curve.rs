use super::*;

/// One plane patch's share of an intersection curve in that patch's UV. The
/// canonical plane operand has exactly one face, so `patch` is always 0.
#[derive(Clone, Debug)]
pub struct PlanePatchCurve {
    /// Face index in the source plane model.
    pub patch: usize,
    /// Exact 2D rational quadratic arcs (or degree-1 segments) in patch UV.
    pub arcs: Vec<Curve>,
}

/// A 2D half-plane constraint `normal . x <= offset` with its own outward
/// classification band (in the 2D coordinate units of the clipped space).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Halfplane2 {
    pub(crate) normal: [f64; 2],
    pub(crate) offset: f64,
    pub(crate) band: f64,
}

/// Result of clipping an ellipse by a set of half-planes.
#[derive(Clone, Debug)]
pub(crate) enum EllipseClip {
    /// Provably outside at least one half-plane beyond its band.
    Empty,
    /// No cuts: strictly inside every half-plane.
    Full,
    /// Swept eccentric-angle intervals (start, end), `end > start`, each
    /// strictly inside all half-planes.
    Arcs(Vec<(f64, f64)>),
}

/// Exact ellipse/half-plane clipping by eccentric angle. The ellipse is
/// `p(phi) = center + cos(phi) w1 + sin(phi) w2`. Each half-plane cuts the
/// ellipse where `f0 + P cos(phi) + Q sin(phi) = 0`, solved exactly as
/// `phi = atan2(Q, P) +- acos(-f0 / hypot(P, Q))`. A maximum or minimum of
/// the constraint function within the half-plane's band of the boundary is
/// a tangency and is reported through the flag, never clipped through.
/// Surviving spans are decided by exact midpoint evaluation; nothing is
/// merged across gaps.
pub(crate) fn clip_ellipse(
    center: [f64; 2],
    w1: [f64; 2],
    w2: [f64; 2],
    halfplanes: &[Halfplane2],
) -> (EllipseClip, bool) {
    let point = |phi: f64| {
        [
            center[0] + phi.cos() * w1[0] + phi.sin() * w2[0],
            center[1] + phi.cos() * w1[1] + phi.sin() * w2[1],
        ]
    };
    let mut cuts: Vec<f64> = Vec::new();
    let mut tangent = false;
    for hp in halfplanes {
        let f0 = hp.normal[0] * center[0] + hp.normal[1] * center[1] - hp.offset;
        let p = hp.normal[0] * w1[0] + hp.normal[1] * w1[1];
        let q = hp.normal[0] * w2[0] + hp.normal[1] * w2[1];
        let s = p.hypot(q);
        if !f0.is_finite() || !s.is_finite() {
            return (EllipseClip::Empty, true);
        }
        if s <= 0. {
            // The constraint is constant on the ellipse (degenerate input).
            if f0 > hp.band {
                return (EllipseClip::Empty, tangent);
            }
            if f0.abs() <= hp.band {
                tangent = true;
            }
            continue;
        }
        if f0 - s > hp.band {
            // Provably violated everywhere: the ellipse misses this region.
            return (EllipseClip::Empty, tangent);
        }
        if f0 + s <= -hp.band {
            // Provably inside everywhere: the constraint never binds.
            continue;
        }
        if (f0 + s).abs() <= hp.band || (f0 - s).abs() <= hp.band {
            // An extremum of the constraint function sits within the band of
            // the boundary line: a tangency, never clipped through.
            tangent = true;
            continue;
        }
        let delta = q.atan2(p);
        let alpha = (-f0 / s).clamp(-1., 1.).acos();
        cuts.push((delta + alpha).rem_euclid(TAU));
        cuts.push((delta - alpha).rem_euclid(TAU));
    }
    if cuts.is_empty() {
        return (EllipseClip::Full, tangent);
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() <= 1e-12);
    let inside = |phi: f64| {
        let q = point(phi);
        halfplanes.iter().all(|hp| {
            let scale = hp.normal[0].abs() * (center[0].abs() + w1[0].abs() + w2[0].abs())
                + hp.normal[1].abs() * (center[1].abs() + w1[1].abs() + w2[1].abs())
                + hp.offset.abs()
                + 1.;
            hp.normal[0] * q[0] + hp.normal[1] * q[1] - hp.offset <= 1e-12 * scale
        })
    };
    let mut arcs: Vec<(f64, f64)> = Vec::new();
    for i in 0..cuts.len() {
        let start = cuts[i];
        let end = if i + 1 < cuts.len() {
            cuts[i + 1]
        } else {
            cuts[0] + TAU
        };
        if end - start > 1e-14 && inside((start + end) / 2.) {
            arcs.push((start, end));
        }
    }
    if arcs.is_empty() {
        (EllipseClip::Empty, tangent)
    } else {
        (EllipseClip::Arcs(arcs), tangent)
    }
}

/// Exact rational multi-span sweep of the conic
/// `p(phi) = center + cos(phi) w1 + sin(phi) w2` over `[start, end]`, split
/// into arcs of at most 90 degrees with weights cos(half-angle). Knots are
/// `0..=pieces` with doubled interior knots, matching `circle_curve` when
/// w1/w2 are perpendicular equal-length vectors and the sweep is 0..TAU.
pub(crate) fn conic_sweep(
    center: [f64; 3],
    w1: [f64; 3],
    w2: [f64; 3],
    start: f64,
    end: f64,
) -> Curve {
    let pieces = ((end - start) / QUARTER).ceil().max(1.) as usize;
    let mut knots = vec![0., 0., 0.];
    let mut control_points = Vec::with_capacity(2 * pieces + 1);
    let mut weights = Vec::with_capacity(2 * pieces + 1);
    let point = |phi: f64| {
        std::array::from_fn::<f64, 3, _>(|k| center[k] + phi.cos() * w1[k] + phi.sin() * w2[k])
            .to_vec()
    };
    for i in 0..pieces {
        let a0 = start + (end - start) * i as f64 / pieces as f64;
        let a1 = start + (end - start) * (i + 1) as f64 / pieces as f64;
        let half = (a1 - a0) / 2.;
        let weight = half.cos();
        let middle = (a0 + a1) / 2.;
        let shoulder = std::array::from_fn::<f64, 3, _>(|k| {
            center[k] + (middle.cos() / weight) * w1[k] + (middle.sin() / weight) * w2[k]
        })
        .to_vec();
        if i == 0 {
            control_points.push(point(a0));
            weights.push(1.);
        }
        control_points.push(shoulder);
        weights.push(weight);
        control_points.push(point(a1));
        weights.push(1.);
        if i + 1 < pieces {
            knots.extend_from_slice(&[(i + 1) as f64, (i + 1) as f64]);
        }
    }
    knots.extend_from_slice(&[pieces as f64, pieces as f64, pieces as f64]);
    Curve {
        degree: 2,
        knots,
        control_points,
        weights,
        periodic: false,
    }
}

/// Exact rational quadratic arcs (single-span, knots [0,0,0,1,1,1]) covering
/// the 2D conic sweep over `[start, end]`, matching `circle_arcs` for a
/// circular w1/w2 pair.
pub(crate) fn conic_arcs_2d(
    center: [f64; 2],
    w1: [f64; 2],
    w2: [f64; 2],
    start: f64,
    end: f64,
) -> Vec<Curve> {
    let pieces = ((end - start) / QUARTER).ceil().max(1.) as usize;
    (0..pieces)
        .map(|i| {
            let a0 = start + (end - start) * i as f64 / pieces as f64;
            let a1 = start + (end - start) * (i + 1) as f64 / pieces as f64;
            let half = (a1 - a0) / 2.;
            let weight = half.cos();
            let point = |phi: f64| {
                [
                    center[0] + phi.cos() * w1[0] + phi.sin() * w2[0],
                    center[1] + phi.cos() * w1[1] + phi.sin() * w2[1],
                ]
                .to_vec()
            };
            let middle = (a0 + a1) / 2.;
            let shoulder = [
                center[0] + (middle.cos() / weight) * w1[0] + (middle.sin() / weight) * w2[0],
                center[1] + (middle.cos() / weight) * w1[1] + (middle.sin() / weight) * w2[1],
            ]
            .to_vec();
            Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![point(a0), shoulder, point(a1)],
                weights: vec![1., weight, 1.],
                periodic: false,
            }
        })
        .collect()
}

#[derive(Clone, Debug)]
pub enum PlaneSphereComponent {
    /// Clear crossing: the exact circle (full, or clipped to the finite
    /// plane domain with exact trim parameters) plus both UV lifts.
    Circle {
        /// Exact rational quadratic sweep: the full circle (four 90-degree
        /// arcs, weights cos(pi/4), knots 0..=4) when `full`, else the
        /// clipped arc pieces over the swept interval.
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit plane normal (the circle plane normal).
        normal: [f64; 3],
        /// True when the circle lies fully inside the plane patch domain.
        full: bool,
        plane_uv: Vec<PlanePatchCurve>,
        sphere_uv: Vec<SpherePatchCircle>,
        /// Worst residual over curve samples against the sphere equation and
        /// the plane equation.
        max_sample_residual: f64,
    },
}

/// Worst |.|p|-r|. and |n.(p-o)| over `count` samples of the curve domain.
fn circle_residuals(
    curve: &Curve,
    sphere: &CanonicalSphere,
    plane: &CanonicalPlane,
    count: usize,
) -> Result<f64> {
    let [lo, hi] = curve.domain();
    let mut worst = 0_f64;
    for i in 0..count {
        let t = lo + (hi - lo) * i as f64 / (count - 1) as f64;
        let p = curve.evaluate(t)?.point;
        let p = [p[0], p[1], p[2]];
        let d = sub(p, sphere.center);
        let sphere_residual = (d[0].hypot(d[1]).hypot(d[2]) - sphere.radius).abs();
        let plane_residual = dot(plane.normal, sub(p, plane.origin)).abs();
        worst = worst.max(sphere_residual).max(plane_residual);
    }
    Ok(worst)
}

/// Analytic plane/sphere intersection of a canonical planar patch and a
/// canonical sphere solid. Non-canonical operands are explicit
/// `UnsupportedSurface` regions, never a numerical fallback; the tangency
/// band (|d| == r) and any tangency to the patch's rectangular domain
/// boundary stay unresolved — tangent contacts are never guessed.
pub fn intersect_plane_sphere(
    plane_model: &Model,
    sphere_model: &Model,
    options: Options,
) -> Result<Report<PlaneSphereComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(plane), Some(sphere)) = (
        recognize_plane(plane_model)?,
        sphere_sphere::recognize_any(sphere_model)?,
    ) else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let d = dot(plane.normal, sub(sphere.center, plane.origin));
    let scale = plane.u_len
        + plane.v_len
        + sphere.radius
        + sub(sphere.center, plane.origin)
            .iter()
            .map(|v| v.abs())
            .fold(0., f64::max)
        + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the distance arithmetic and the radii.
    let band = plane.error + sphere.error + 16. * f64::EPSILON * scale;
    let ad = d.abs();
    let d_lo = (ad - band).max(0.);
    let d_hi = ad + band;
    let r = sphere.radius;
    // Miss: the whole distance band clears the radius.
    if d_lo > r + band {
        return Ok(report);
    }
    // Tangency, or a band straddling it: never a point component.
    if d_hi >= (r - band).max(0.) {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let rho2 = r * r - d * d;
    if !rho2.is_finite() || rho2 <= 0. {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let rho = rho2.sqrt();
    let middle = std::array::from_fn(|k| sphere.center[k] - d * plane.normal[k]);
    // In-plane orthonormal basis aligned with the patch edges, so the circle
    // is an axis-aligned ellipse in patch UV.
    let e1 = plane.u.map(|x| x / plane.u_len);
    let e2 = plane.v.map(|x| x / plane.v_len);
    let rel = sub(middle, plane.origin);
    let uc = dot(rel, plane.u) / (plane.u_len * plane.u_len);
    let vc = dot(rel, plane.v) / (plane.v_len * plane.v_len);
    let w1 = [rho / plane.u_len, 0.];
    let w2 = [0., rho / plane.v_len];
    let rect = [
        Halfplane2 {
            normal: [-1., 0.],
            offset: 0.,
            band: band / plane.u_len,
        },
        Halfplane2 {
            normal: [1., 0.],
            offset: 1.,
            band: band / plane.u_len,
        },
        Halfplane2 {
            normal: [0., -1.],
            offset: 0.,
            band: band / plane.v_len,
        },
        Halfplane2 {
            normal: [0., 1.],
            offset: 1.,
            band: band / plane.v_len,
        },
    ];
    let (clip, tangent) = clip_ellipse([uc, vc], w1, w2, &rect);
    if tangent {
        // The circle touches the patch boundary within the band: unresolved,
        // alongside any resolved arc components (house tangency discipline).
        report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
    }
    let point_at = |phi: f64| {
        std::array::from_fn::<f64, 3, _>(|k| {
            middle[k] + rho * (phi.cos() * e1[k] + phi.sin() * e2[k])
        })
    };
    let we1 = e1.map(|x| x * rho);
    let we2 = e2.map(|x| x * rho);
    match clip {
        EllipseClip::Empty => {}
        EllipseClip::Full if !tangent => {
            let curve = conic_sweep(middle, we1, we2, 0., TAU);
            let sphere_uv = sphere_sphere::lift(&sphere, plane.normal, middle);
            let plane_uv = vec![PlanePatchCurve {
                patch: 0,
                arcs: conic_arcs_2d([uc, vc], w1, w2, 0., TAU),
            }];
            let max_sample_residual = circle_residuals(&curve, &sphere, &plane, 16)?;
            report.components.push(PlaneSphereComponent::Circle {
                curve,
                center: middle,
                radius: rho,
                normal: plane.normal,
                full: true,
                plane_uv,
                sphere_uv,
                max_sample_residual,
            });
        }
        EllipseClip::Arcs(intervals) => {
            for (a, b) in intervals {
                let curve = conic_sweep(middle, we1, we2, a, b);
                let sphere_uv =
                    sphere_sphere::lift_clipped(&sphere, plane.normal, middle, &[(a, b)], point_at);
                let plane_uv = vec![PlanePatchCurve {
                    patch: 0,
                    arcs: conic_arcs_2d([uc, vc], w1, w2, a, b),
                }];
                let max_sample_residual = circle_residuals(&curve, &sphere, &plane, 9)?;
                report.components.push(PlaneSphereComponent::Circle {
                    curve,
                    center: middle,
                    radius: rho,
                    normal: plane.normal,
                    full: false,
                    plane_uv,
                    sphere_uv,
                    max_sample_residual,
                });
            }
        }
        // Tangency within the band: the full circle is withheld; the
        // unresolved region above carries the contact.
        EllipseClip::Full => {}
    }
    Ok(report)
}

impl value_codec::Serialize for PlanePatchCurve {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"patch":self.patch,"arcs":self.arcs})
    }
}
impl value_codec::Serialize for PlaneSphereComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                full,
                plane_uv,
                sphere_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"full":full,"planeUv":plane_uv,"sphereUv":sphere_uv,
                "maxSampleResidual":max_sample_residual}),
        }
    }
}

/// Builds the canonical planar patch operand: a one-face open model with an
/// exact bilinear affine surface over the unit square, unit-square boundary
/// trims and four corner vertices (the cuboid face construction, freestanding).
#[cfg(test)]
pub(crate) fn plane_patch(origin: [f64; 3], u: [f64; 3], v: [f64; 3]) -> Model {
    use brep_core::{Coedge, Edge, Face, FaceUse, Loop, Shell, TopologyIds, Vertex};
    let add = |a: [f64; 3], b: [f64; 3]| [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
    let corners = [
        origin,
        add(origin, u),
        add(add(origin, u), v),
        add(origin, v),
    ];
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let mut edges = Vec::new();
    let mut coedges = Vec::new();
    for i in 0..4 {
        let j = (i + 1) % 4;
        edges.push(Edge {
            degenerate: false,
            vertices: [i, j],
            curve: super::test_utils::line(corners[i].to_vec(), corners[j].to_vec()),
        });
        coedges.push(Coedge {
            edge: i,
            reversed: false,
            pcurve: super::test_utils::line(uv[i].to_vec(), uv[j].to_vec()),
        });
    }
    let mut model = Model(
        brep_topology::Model {
            vertices: corners.map(|point| Vertex { point }).to_vec(),
            edges,
            loops: vec![Loop { coedges }],
            faces: vec![Face {
                surface: nurbs_core::surface::Surface {
                    degree_u: 1,
                    degree_v: 1,
                    knots_u: vec![0., 0., 1., 1.],
                    knots_v: vec![0., 0., 1., 1.],
                    control_points: vec![
                        vec![corners[0].to_vec(), corners[3].to_vec()],
                        vec![corners[1].to_vec(), corners[2].to_vec()],
                    ],
                    weights: vec![vec![1., 1.], vec![1., 1.]],
                    periodic_u: false,
                    periodic_v: false,
                },
                outer: 0,
                holes: vec![],
            }],
            shells: vec![Shell {
                faces: vec![FaceUse {
                    face: 0,
                    reversed: false,
                }],
                closed: false,
            }],
            // A freestanding open patch: bodies require closed shells.
            bodies: vec![],
            tolerance_mm: 1e-7,
        },
        TopologyIds::default(),
    );
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}
