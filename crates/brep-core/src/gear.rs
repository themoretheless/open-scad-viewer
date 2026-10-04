//! Involute gears as trimmed NURBS solids: spur, helical and herringbone,
//! external with an optional bore or internal (ring) with a rim.
//!
//! The tooth outline is authored per tooth as exact rational arcs (tip and
//! root circles), radial lines below the base circle, and involute flanks
//! approximated by quintic Hermite spans with an analytic real-arithmetic
//! remainder at most `FLANK_FIT`. Binary64 rounding is not certified. Each outline
//! curve becomes one wall face: a ruled surface for a spur gear, or for a
//! helical gear a quintic Hermite sweep with a continuous real-arithmetic
//! remainder at most `SWEEP_FIT`; binary64 rounding is not certified.
//! A herringbone gear is two mirrored helical halves sharing the mid-plane
//! outline. Caps are planar faces bounded by the outline and its rotated
//! image, with affine pcurves. All edges are iso-curves of the wall
//! surfaces, so the body is self-consistent to the kernel tolerance; the
//! stated fit bounds describe how far the surfaces sit from the ideal
//! involute helicoid, which no polynomial surface represents exactly.
use crate::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopologyIds, Vertex};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::f64::consts::{PI, TAU};

/// Requested real-arithmetic Hermite remainder; excludes binary64 rounding.
const FLANK_FIT: f64 = 1e-6;
/// Largest deviation of a lofted helical wall from the exact helical sweep.
const SWEEP_FIT: f64 = 1e-4;
const ARC_WEIGHT_MAX_SWEEP: f64 = PI - 1e-6;

fn invalid(message: impl Into<String>) -> Error {
    Error::new("BREP_INVALID_SIZE", message)
}

#[derive(Clone, Debug)]
pub struct GearSpec {
    pub module: f64,
    pub teeth: usize,
    pub pressure_angle_deg: f64,
    pub height: f64,
    /// Helix angle at the pitch circle, signed (positive twists
    /// counter-clockwise with height); zero is a spur gear.
    pub helix_angle_deg: f64,
    /// Two mirrored helical halves.
    pub herringbone: bool,
    /// Central bore diameter of an external gear (zero for none).
    pub bore: f64,
    /// Internal (ring) gear: teeth point inward, `rim_width` of material
    /// outside the root circle.
    pub internal: bool,
    pub rim_width: f64,
    pub clearance: f64,
    pub backlash: f64,
}

impl Default for GearSpec {
    fn default() -> Self {
        Self {
            module: 1.,
            teeth: 20,
            pressure_angle_deg: 20.,
            height: 5.,
            helix_angle_deg: 0.,
            herringbone: false,
            bore: 0.,
            internal: false,
            rim_width: 2.,
            clearance: 0.25,
            backlash: 0.,
        }
    }
}

/// Derived radii and angles of a spec, mirroring the sampled generator.
#[derive(Clone, Debug)]
pub struct GearGeometry {
    pub pitch_radius: f64,
    pub base_radius: f64,
    pub tip_radius: f64,
    pub root_radius: f64,
    pub outside_radius: f64,
    /// Continuous real-arithmetic remainder for the Hermite involute flanks.
    /// Excludes binary64 trig, control placement and evaluation rounding.
    pub flank_deviation: f64,
    /// Whether the flank error report also covers binary64 rounding.
    pub flank_rounding_certified: bool,
    /// Continuous real-arithmetic remainder for the helical walls (zero for spur).
    /// Binary64 rounding is excluded.
    pub sweep_deviation: f64,
    pub sweep_rounding_certified: bool,
    pub face_count: usize,
}

// ---------------------------------------------------------------------------
// 2D outline
// ---------------------------------------------------------------------------

fn polar(r: f64, a: f64) -> [f64; 2] {
    [r * a.cos(), r * a.sin()]
}

fn line2(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    }
}

/// Exact circular arc about the origin from `from` to `to` (sweep below a
/// half turn, either sense).
fn arc2(r: f64, from: f64, to: f64) -> Result<Curve> {
    let sweep = to - from;
    if !(sweep.abs() > 1e-12 && sweep.abs() < ARC_WEIGHT_MAX_SWEEP) {
        return Err(invalid("Gear arc must sweep less than a half turn"));
    }
    let mid = (from + to) / 2.;
    let w = (sweep / 2.).cos();
    Ok(Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            polar(r, from).to_vec(),
            polar(r / w, mid).to_vec(),
            polar(r, to).to_vec(),
        ],
        weights: vec![1., w, 1.],
        periodic: false,
    })
}

/// Full circle as `arcs` exact arcs (at least four), counter-clockwise
/// from +x.
fn circle2(r: f64, arcs: usize) -> Result<Vec<Curve>> {
    let arcs = arcs.max(4);
    (0..arcs)
        .map(|q| {
            arc2(
                r,
                q as f64 * TAU / arcs as f64,
                (q + 1) as f64 * TAU / arcs as f64,
            )
        })
        .collect()
}

/// Arc count for a circle of radius `r` that must stay outside radius
/// `inner` even when each arc is displayed as two chords: the inscribed
/// radius of the chord polygon must clear `inner`.
fn arcs_clearing(r: f64, inner: f64) -> usize {
    if inner <= 0. || inner >= r {
        return 4;
    }
    let half = (inner / r).acos();
    ((PI / (2. * half)).ceil() as usize).clamp(4, 64)
}

/// Arcs per full circle on a gear: one arc for every two teeth, so the rim
/// and the bore are displayed as round as the teeth at any tessellation
/// level, while the arc count stays exact and bounded.
pub fn circle_arcs(teeth: usize) -> usize {
    (teeth / 2).clamp(4, 64)
}

/// Involute of the base circle `base` between roll parameters `t0..t1`,
/// placed at the tooth centre with half-angle function `half`; `sign` picks
/// the flank side. Quintic Hermite approximation with an ideal continuous bound.
fn involute(
    base: f64,
    t0: f64,
    t1: f64,
    center: f64,
    sign: f64,
    half: &dyn Fn(f64) -> f64,
) -> Result<(Curve, f64)> {
    // Reuse the native involute Hermite constructor and its continuous
    // real-arithmetic remainder instead of a finite sample acceptance test.
    let approximation = nurbs_core::involute::approximate_quintic(
        [0.; 3],
        base,
        t0.min(t1),
        t0.max(t1),
        FLANK_FIT,
    )?;
    let mut curve = approximation.curve;
    if curve.control_points.len() > 32 {
        return Err(invalid(
            "Gear flank needs more than 32 surface controls at the requested tolerance; reduce module or increase teeth",
        ));
    }
    let angle = center + sign * half(base);
    let (sin, cos) = angle.sin_cos();
    for point in &mut curve.control_points {
        let (x, y) = (point[0], point[1]);
        // The canonical involute has polar angle t-atan(t). Reflection
        // followed by rotation yields center+sign*(half(base)-t+atan(t)).
        *point = vec![cos * x + sign * sin * y, sin * x - sign * cos * y];
    }
    if t1 < t0 {
        curve = reverse2(&curve);
    }
    curve.validate()?;
    // Orthogonal placement preserves the ideal Euclidean remainder.
    // Binary64 trigonometry, placement and evaluation rounding are excluded.
    Ok((curve, approximation.real_arithmetic_error_estimate))
}

fn reverse2(c: &Curve) -> Curve {
    let mut r = c.clone();
    r.control_points.reverse();
    r.weights.reverse();
    let [a, b] = c.domain();
    r.knots = c.knots.iter().rev().map(|k| a + b - k).collect();
    r
}

/// Closed loops of the outline (each a connected CCW or CW chain of 2D
/// curves, material on the left), the geometry report, and the fit.
fn outline(spec: &GearSpec) -> Result<(Vec<Vec<Curve>>, GearGeometry)> {
    let teeth = spec.teeth as f64;
    if !(3..=256).contains(&spec.teeth) {
        return Err(invalid("Gear teeth must be 3..256"));
    }
    if !(0.05..=100.).contains(&spec.module) {
        return Err(invalid("Gear module must be 0.05..100 mm"));
    }
    if !(10.0..=35.).contains(&spec.pressure_angle_deg) {
        return Err(invalid("Gear pressure angle must be 10..35 degrees"));
    }
    if !(0.05..=1000.).contains(&spec.height) {
        return Err(invalid("Gear height must be 0.05..1000 mm"));
    }
    if !spec.helix_angle_deg.is_finite() || spec.helix_angle_deg.abs() >= 80. {
        return Err(invalid("Gear helix angle must be within -80..80 degrees"));
    }
    if spec.backlash < 0.
        || spec.backlash > spec.module / 2.
        || spec.clearance < 0.
        || spec.clearance > spec.module
    {
        return Err(invalid(
            "Gear backlash must be 0..module/2 and clearance 0..module",
        ));
    }
    let alpha = spec.pressure_angle_deg.to_radians();
    let pitch = spec.module * teeth / 2.;
    let base = pitch * alpha.cos();
    let (tip, root) = if spec.internal {
        (pitch - spec.module, pitch + spec.module + spec.clearance)
    } else {
        (pitch + spec.module, pitch - spec.module - spec.clearance)
    };
    let outside = if spec.internal {
        root + spec.rim_width
    } else {
        tip
    };
    if root <= 0. || outside > 10000. {
        return Err(invalid(
            "Gear root radius must be positive and outside radius at most 10000 mm",
        ));
    }
    if spec.internal {
        if tip <= base + 1e-9 {
            return Err(invalid(
                "Internal gear tip must stay outside the base circle",
            ));
        }
        if spec.bore != 0. {
            return Err(invalid(
                "Internal gear has a toothed opening; bore must be zero",
            ));
        }
        if spec.rim_width < spec.module / 4. {
            return Err(invalid(
                "Internal gear rim must be at least a quarter module",
            ));
        }
    } else if spec.bore < 0. || (spec.bore > 0. && spec.bore > 2. * root - spec.module / 2.) {
        return Err(invalid(
            "Gear bore must leave a quarter module inside the root circle",
        ));
    }
    let low = if spec.internal { tip } else { root };
    let high = if spec.internal { root } else { tip };
    let start = base.max(low);
    let inv_pitch = alpha.tan() - alpha;
    let half_pitch = PI / (2. * teeth)
        + if spec.internal {
            spec.backlash / (2. * pitch)
        } else {
            -spec.backlash / (2. * pitch)
        };
    let half = move |r: f64| -> f64 {
        let t = ((r / base).powi(2) - 1.).max(0.).sqrt();
        half_pitch + inv_pitch - (t - t.atan())
    };
    let low_half = half(start);
    let high_half = half(high);
    let step = TAU / teeth;
    if high_half <= 1e-6 || low_half >= step / 2. - 1e-6 {
        return Err(invalid(
            "Gear tooth profile collapses or overlaps; change teeth, pressure angle, backlash or clearance",
        ));
    }
    let t0 = ((start / base).powi(2) - 1.).max(0.).sqrt();
    let t1 = ((high / base).powi(2) - 1.).max(0.).sqrt();
    let mut flank_deviation: f64 = 0.;
    // One tooth outline, counter-clockwise, starting at the low radius on
    // the leading side.
    let mut tooth: Vec<Curve> = Vec::new();
    for i in 0..spec.teeth {
        let center = i as f64 * step + if spec.internal { step / 2. } else { 0. };
        if start > low + 1e-12 {
            tooth.push(line2(
                polar(low, center - low_half),
                polar(start, center - low_half),
            ));
        }
        let (rising, d1) = involute(base, t0, t1, center, -1., &half)?;
        let (falling, d2) = involute(base, t1, t0, center, 1., &half)?;
        flank_deviation = flank_deviation.max(d1).max(d2);
        tooth.push(rising);
        tooth.push(arc2(high, center - high_half, center + high_half)?);
        tooth.push(falling);
        if start > low + 1e-12 {
            tooth.push(line2(
                polar(start, center + low_half),
                polar(low, center + low_half),
            ));
        }
        tooth.push(arc2(low, center + low_half, center + step - low_half)?);
    }
    let mut loops = Vec::new();
    if spec.internal {
        // Material outside: the outer circle is the CCW boundary, the tooth
        // outline runs clockwise as a hole.
        loops.push(circle2(
            outside,
            arcs_clearing(outside, root).max(circle_arcs(spec.teeth)),
        )?);
        let mut hole: Vec<Curve> = tooth.iter().rev().map(reverse2).collect();
        hole.iter_mut().for_each(|_| {});
        loops.push(hole);
    } else {
        loops.push(tooth);
        if spec.bore > 0. {
            let mut hole: Vec<Curve> = circle2(spec.bore / 2., circle_arcs(spec.teeth))?
                .iter()
                .rev()
                .map(reverse2)
                .collect();
            hole.iter_mut().for_each(|_| {});
            loops.push(hole);
        }
    }
    let geometry = GearGeometry {
        pitch_radius: pitch,
        base_radius: base,
        tip_radius: tip,
        root_radius: root,
        outside_radius: outside,
        flank_deviation,
        flank_rounding_certified: false,
        sweep_deviation: 0.,
        sweep_rounding_certified: false,
        face_count: 0,
    };
    Ok((loops, geometry))
}

// ---------------------------------------------------------------------------
// Solid
// ---------------------------------------------------------------------------

fn rotate2(c: &Curve, theta: f64) -> Curve {
    let (s, co) = theta.sin_cos();
    let mut r = c.clone();
    for p in &mut r.control_points {
        let (x, y) = (p[0], p[1]);
        p[0] = co * x - s * y;
        p[1] = s * x + co * y;
    }
    r
}

fn lift(c: &Curve, z: f64) -> Curve {
    let mut r = c.clone();
    for p in &mut r.control_points {
        p.push(z);
    }
    r
}

/// Wall surface of one outline curve between `z0` and `z1`, rotated by
/// `twist` radians over the height: ruled for zero twist, else quintic
/// endpoint-Hermite spans with an ideal continuous bound at most SWEEP_FIT.
fn wall(c: &Curve, z0: f64, z1: f64, theta0: f64, twist: f64) -> Result<(Surface, f64)> {
    let approximation =
        nurbs_core::helical_sweep::approximate(&lift(c, z0), z1 - z0, theta0, twist, SWEEP_FIT)?;
    Ok((
        approximation.surface,
        approximation.real_arithmetic_error_estimate,
    ))
}

/// Iso-curve of a surface at u = 0 or u = 1 (the shared vertical edge).
fn column(surface: &Surface, last: bool) -> Curve {
    let i = if last {
        surface.control_points.len() - 1
    } else {
        0
    };
    Curve {
        degree: surface.degree_v,
        knots: surface.knots_v.clone(),
        control_points: surface.control_points[i].clone(),
        weights: surface.weights[i].clone(),
        periodic: false,
    }
}

/// Planar cap over the bounding box of the outline curves at height `z`:
/// the surface and the affine pcurve map.
fn cap(curves: &[&Curve], z: f64) -> (Surface, [f64; 4]) {
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for c in curves {
        for p in &c.control_points {
            for k in 0..2 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
    }
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![lo[0], lo[1], z], vec![lo[0], hi[1], z]],
            vec![vec![hi[0], lo[1], z], vec![hi[0], hi[1], z]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    (surface, [lo[0], lo[1], hi[0] - lo[0], hi[1] - lo[1]])
}

fn cap_pcurve(c: &Curve, map: [f64; 4]) -> Curve {
    let mut p = c.clone();
    for point in &mut p.control_points {
        let (x, y) = (point[0], point[1]);
        *point = vec![(x - map[0]) / map[2], (y - map[1]) / map[3]];
    }
    p
}

fn unit_line(a: [f64; 2], b: [f64; 2]) -> Curve {
    line2(a, b)
}

/// Builds the gear solid. Returns the model and its geometry report.
pub fn gear_with_report(spec: &GearSpec) -> Result<(Model, GearGeometry)> {
    let (loops, mut geometry) = outline(spec)?;
    let beta = spec.helix_angle_deg.to_radians();
    let total_twist = spec.height * beta.tan() / geometry.pitch_radius;
    // Axial layers: (z0, z1, theta0, twist).
    let layers: Vec<(f64, f64, f64, f64)> = if spec.herringbone && beta != 0. {
        let half = spec.height / 2.;
        vec![
            (0., half, 0., total_twist / 2.),
            (half, spec.height, total_twist / 2., -total_twist / 2.),
        ]
    } else {
        vec![(0., spec.height, 0., total_twist)]
    };
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    let mut loops_out: Vec<Loop> = Vec::new();
    let mut faces: Vec<Face> = Vec::new();
    let mut uses: Vec<FaceUse> = Vec::new();
    let mut sweep_deviation: f64 = 0.;
    // Per level (layer boundaries) and per loop: the level edges (one per
    // outline curve) and the level vertices (one per curve start).
    let level_z: Vec<f64> = std::iter::once(layers[0].0)
        .chain(layers.iter().map(|l| l.1))
        .collect();
    let level_theta: Vec<f64> = std::iter::once(layers[0].2)
        .chain(layers.iter().map(|l| l.2 + l.3))
        .collect();
    let mut level_vertices: Vec<Vec<Vec<usize>>> = Vec::new();
    let mut level_edges: Vec<Vec<Vec<usize>>> = Vec::new();
    for (li, &z) in level_z.iter().enumerate() {
        let theta = level_theta[li];
        let mut vl = Vec::new();
        let mut el = Vec::new();
        for outline_loop in &loops {
            let mut vs = Vec::new();
            for c in outline_loop {
                let rotated = lift(&rotate2(c, theta), z);
                let p = rotated.control_points[0].clone();
                vs.push(vertices.len());
                vertices.push(Vertex {
                    point: [p[0], p[1], p[2]],
                });
            }
            let n = outline_loop.len();
            let mut es = Vec::new();
            for (k, c) in outline_loop.iter().enumerate() {
                let rotated = lift(&rotate2(c, theta), z);
                es.push(edges.len());
                edges.push(Edge {
                    degenerate: false,
                    vertices: [vs[k], vs[(k + 1) % n]],
                    curve: rotated,
                });
            }
            vl.push(vs);
            el.push(es);
        }
        level_vertices.push(vl);
        level_edges.push(el);
    }
    // Walls.
    for (layer, &(z0, z1, theta0, twist)) in layers.iter().enumerate() {
        for (loop_index, outline_loop) in loops.iter().enumerate() {
            let n = outline_loop.len();
            // Vertical edges at every curve start of this layer.
            let mut vertical: Vec<usize> = Vec::with_capacity(n);
            let mut surfaces: Vec<Surface> = Vec::with_capacity(n);
            for c in outline_loop {
                let (surface, dev) = wall(c, z0, z1, theta0, twist)?;
                sweep_deviation = sweep_deviation.max(dev);
                surfaces.push(surface);
            }
            for k in 0..n {
                let curve = column(&surfaces[k], false);
                vertical.push(edges.len());
                edges.push(Edge {
                    degenerate: false,
                    vertices: [
                        level_vertices[layer][loop_index][k],
                        level_vertices[layer + 1][loop_index][k],
                    ],
                    curve,
                });
            }
            for k in 0..n {
                let next = (k + 1) % n;
                let bottom = level_edges[layer][loop_index][k];
                let top = level_edges[layer + 1][loop_index][k];
                let coedges = vec![
                    Coedge {
                        edge: bottom,
                        reversed: false,
                        pcurve: unit_line([0., 0.], [1., 0.]),
                    },
                    Coedge {
                        edge: vertical[next],
                        reversed: false,
                        pcurve: unit_line([1., 0.], [1., 1.]),
                    },
                    Coedge {
                        edge: top,
                        reversed: true,
                        pcurve: unit_line([1., 1.], [0., 1.]),
                    },
                    Coedge {
                        edge: vertical[k],
                        reversed: true,
                        pcurve: unit_line([0., 1.], [0., 0.]),
                    },
                ];
                loops_out.push(Loop { coedges });
                faces.push(Face {
                    surface: surfaces[k].clone(),
                    outer: loops_out.len() - 1,
                    holes: vec![],
                });
                uses.push(FaceUse {
                    face: faces.len() - 1,
                    reversed: false,
                });
            }
        }
    }
    // Caps: bottom (reversed use, normal -z) and top.
    for (li, reversed) in [(0usize, true), (level_z.len() - 1, false)] {
        let z = level_z[li];
        let theta = level_theta[li];
        let rotated: Vec<Vec<Curve>> = loops
            .iter()
            .map(|l| l.iter().map(|c| rotate2(c, theta)).collect())
            .collect();
        let all: Vec<&Curve> = rotated.iter().flatten().collect();
        let (surface, map) = cap(&all, z);
        let mut loop_ids = Vec::new();
        for (loop_index, curves) in rotated.iter().enumerate() {
            let coedges = curves
                .iter()
                .enumerate()
                .map(|(k, c)| Coedge {
                    edge: level_edges[li][loop_index][k],
                    reversed: false,
                    pcurve: cap_pcurve(c, map),
                })
                .collect();
            loops_out.push(Loop { coedges });
            loop_ids.push(loops_out.len() - 1);
        }
        faces.push(Face {
            surface,
            outer: loop_ids[0],
            holes: loop_ids[1..].to_vec(),
        });
        uses.push(FaceUse {
            face: faces.len() - 1,
            reversed,
        });
    }
    geometry.sweep_deviation = sweep_deviation;
    geometry.face_count = faces.len();
    let mut model = Model(
        brep_topology::Model {
            vertices,
            edges,
            loops: loops_out,
            faces,
            shells: vec![Shell {
                faces: uses,
                closed: true,
            }],
            bodies: vec![Body {
                outer_shell: 0,
                inner_shells: vec![],
            }],
            tolerance_mm: 1e-7,
        },
        TopologyIds::default(),
    );
    model.rebuild_topology_ids();
    model.validate()?;
    Ok((model, geometry))
}

pub fn gear(spec: &GearSpec) -> Result<Model> {
    gear_with_report(spec).map(|(model, _)| model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::mass_properties;

    fn volume(model: &Model) -> f64 {
        // Hundreds of faces: the coarsest admitted relative tolerance keeps
        // the divergence integration inside its evaluation budget.
        mass_properties(model, 1e-3, 2_000_000)
            .unwrap()
            .signed_volume_mm3
    }

    #[test]
    fn involute_flanks_follow_analytic_geometry_and_never_relax_remainder() {
        for base in [2., 20., 100.] {
            let half = |r: f64| {
                let t = ((r / base).powi(2) - 1.).max(0.).sqrt();
                0.18 - (t - t.atan())
            };
            for sign in [-1., 1.] {
                for (start, end) in [(0., 0.7), (0.7, 0.), (0.2, 0.7)] {
                    let (curve, bound) = involute(base, start, end, 0.4, sign, &half).unwrap();
                    assert!(bound > 0. && bound <= FLANK_FIT);
                    for i in 0..=200 {
                        let s = i as f64 / 200.;
                        let t = start + (end - start) * s;
                        // Independent roll-radius/polar-angle definition.
                        let radius = base * t.hypot(1.);
                        let angle = 0.4 + sign * (0.18 - t + t.atan());
                        let point = curve.evaluate(s).unwrap().point;
                        let distance = (point[0] - radius * angle.cos())
                            .hypot(point[1] - radius * angle.sin());
                        assert!(distance <= bound + 1e-11, "{distance} > {bound}");
                    }
                }
            }
        }
        let half = |_: f64| 0.;
        assert!(involute(2., 0., 100., 0., 1., &half).is_err());
    }

    #[test]
    fn spur_gear_is_a_closed_exact_solid_with_the_expected_volume() {
        let spec = GearSpec {
            module: 2.,
            teeth: 20,
            height: 6.,
            ..GearSpec::default()
        };
        let (model, geometry) = gear_with_report(&spec).unwrap();
        assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
        assert_eq!(geometry.face_count, 20 * 6 + 2);
        assert!(geometry.flank_deviation <= FLANK_FIT);
        assert!(!geometry.flank_rounding_certified);
        assert_eq!(geometry.sweep_deviation, 0.);
        // The volume lies between the root and tip cylinders and close to
        // the pitch cylinder (teeth fill about half the band).
        let v = volume(&model);
        let root = PI * geometry.root_radius.powi(2) * 6.;
        let tip = PI * geometry.tip_radius.powi(2) * 6.;
        assert!(v > root && v < tip, "{v} not in ({root}, {tip})");
        let pitch = PI * geometry.pitch_radius.powi(2) * 6.;
        assert!((v - pitch).abs() < 0.08 * pitch, "{v} vs pitch {pitch}");
    }

    #[test]
    fn helical_and_herringbone_gears_match_the_sweep_within_tolerance() {
        for herringbone in [false, true] {
            let spec = GearSpec {
                module: 1.5,
                teeth: 16,
                height: 8.,
                helix_angle_deg: 25.,
                herringbone,
                bore: 6.,
                ..GearSpec::default()
            };
            let (model, geometry) = gear_with_report(&spec).unwrap();
            assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
            assert!(geometry.sweep_deviation > 0. && geometry.sweep_deviation <= SWEEP_FIT);
            assert!(!geometry.sweep_rounding_certified);
            let expected_faces = (16 * 6 + 8) * if herringbone { 2 } else { 1 } + 2;
            assert_eq!(geometry.face_count, expected_faces);
            let spur = gear(&GearSpec {
                helix_angle_deg: 0.,
                herringbone: false,
                ..spec.clone()
            })
            .unwrap();
            // Twisting does not change the volume.
            let (vh, vs) = (volume(&model), volume(&spur));
            assert!((vh - vs).abs() < 2e-3 * vs, "helical {vh} vs spur {vs}");
        }
    }

    #[test]
    fn internal_gear_has_a_rim_and_a_toothed_opening() {
        let spec = GearSpec {
            module: 1.5,
            teeth: 40,
            height: 10.,
            internal: true,
            rim_width: 2.,
            helix_angle_deg: 15.,
            herringbone: true,
            ..GearSpec::default()
        };
        let (model, geometry) = gear_with_report(&spec).unwrap();
        assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
        assert!(geometry.outside_radius > geometry.root_radius);
        let v = volume(&model);
        let outer = PI * geometry.outside_radius.powi(2) * 10.;
        let pitch = PI * geometry.pitch_radius.powi(2) * 10.;
        assert!(v > 0. && v < outer - pitch * 0.9, "{v}");
    }

    #[test]
    fn few_teeth_planet_and_refusals() {
        // A four-tooth planet as in the spinner: undercut is not modelled,
        // the outline is still a valid solid.
        let planet = gear(&GearSpec {
            module: 1.5,
            teeth: 4,
            height: 10.,
            pressure_angle_deg: 20.,
            ..GearSpec::default()
        });
        assert!(planet.is_ok(), "{:?}", planet.err());
        assert!(
            gear(&GearSpec {
                teeth: 2,
                ..GearSpec::default()
            })
            .is_err()
        );
        assert!(
            gear(&GearSpec {
                bore: 100.,
                ..GearSpec::default()
            })
            .is_err()
        );
        assert!(
            gear(&GearSpec {
                internal: true,
                rim_width: 0.,
                ..GearSpec::default()
            })
            .is_err()
        );
    }
}
