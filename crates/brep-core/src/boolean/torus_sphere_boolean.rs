//! Axial torus × sphere: the `Mate` of `sphere_mate` for the canonical ring
//! torus (sixteen rational biquadratic patches, four profile quadrants by
//! four revolution quadrants, rigid placement admitted) against a sphere.
//!
//! A sphere centred on the torus axis meets the torus in parallels: in the
//! meridian half-plane the torus is the circle `(rho - R)^2 + z^2 = r_t^2`
//! and the sphere the circle `rho^2 + (z - h)^2 = r_s^2`, and each of their
//! (at most two) crossings revolves into an exact circle. A parallel is an
//! iso-v curve of the four patches of its profile row, split at the four
//! meridian edges, so its pcurves are straight lines.
//!
//! A sphere off the axis meets the torus in a quartic and is refused, unless
//! it provably misses the torus surface: the torus is the level set of the
//! 1-Lipschitz distance to its core circle, so its points lie between
//! `|dmin - r_t|` and `dmax + r_t` from the sphere centre. Such a pair has no
//! section and the engine decides containment or disjointness by sampling.
//! Tangencies and parallels on a patch seam are refused rather than guessed.
use crate::Model;
use crate::imprint_pipeline::SpatialRelation;
use crate::intersections::sphere_sphere::{self, CanonicalSphere};
use crate::intersections::{CanonicalTorus, recognize_torus};
use crate::sphere_mate::{
    self, Circle, Imprint, Mate, TAU, dot, norm, point3, quarter_arc_parameter, sub, unsupported,
};
use nurbs_core::{Result, curve::Curve};

const QUARTER: f64 = std::f64::consts::FRAC_PI_2;

struct AxialTorus<'m> {
    model: &'m Model,
    canon: CanonicalTorus,
    /// (profile quadrant, revolution quadrant) of every face.
    quadrant_of: Vec<(usize, usize)>,
    /// Meridian edge at revolution angle `i * 90°` spanning profile quadrant
    /// `j`: `meridians[j][i]`.
    meridians: [[usize; 4]; 4],
}

/// Angle from `from` to `point` on a circle of centre `center`, in
/// `[0, 90°]` for points of a quarter arc starting at `from`.
fn quarter_angle(center: [f64; 3], from: [f64; 3], point: [f64; 3]) -> (f64, f64) {
    let a = sub(from, center);
    let b = sub(point, center);
    let scale = norm(a) * norm(b);
    let cos = dot(a, b) / scale;
    let c = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    (cos, norm(c) / scale)
}

impl<'m> AxialTorus<'m> {
    fn recognize(model: &'m Model) -> Result<Option<Self>> {
        let Some(canon) = recognize_torus(model)? else {
            return Ok(None);
        };
        let mut quadrant_of = vec![(usize::MAX, usize::MAX); model.faces.len()];
        let mut meridians = [[usize::MAX; 4]; 4];
        for (j, row) in canon.patches.iter().enumerate() {
            for (i, &face) in row.iter().enumerate() {
                quadrant_of[face] = (j, i);
                let wire = &model.loops[model.faces[face].outer];
                let Some(coedge) = wire.coedges.iter().find(|c| {
                    c.pcurve.degree == 1 && c.pcurve.control_points.iter().all(|p| p[0] == 0.)
                }) else {
                    return Ok(None);
                };
                meridians[j][i] = coedge.edge;
            }
        }
        Ok(Some(Self {
            model,
            canon,
            quadrant_of,
            meridians,
        }))
    }

    /// Axial coordinate and radial offset from the torus centre.
    fn local(&self, p: [f64; 3]) -> (f64, [f64; 3]) {
        let d = sub(p, self.canon.center);
        let axial = dot(d, self.canon.axis);
        let perp: [f64; 3] = std::array::from_fn(|k| d[k] - axial * self.canon.axis[k]);
        (axial, perp)
    }

    /// Unit radial direction at revolution angle `angle`.
    fn radial(&self, angle: f64) -> [f64; 3] {
        let [x, y] = self.canon.frame;
        let (sin, cos) = angle.sin_cos();
        std::array::from_fn(|k| cos * x[k] + sin * y[k])
    }

    /// Profile quadrant and local profile angle of a point, and the
    /// revolution angle.
    fn angles(&self, p: [f64; 3]) -> (f64, f64) {
        let (axial, perp) = self.local(p);
        let [x, y] = self.canon.frame;
        let revolution = dot(perp, y).atan2(dot(perp, x)).rem_euclid(TAU);
        let profile = axial.atan2(norm(perp) - self.canon.major).rem_euclid(TAU);
        (profile, revolution)
    }
}

/// Local quarter-arc parameter of `angle` within quadrant `q`, refusing
/// angles outside it.
fn local_parameter(angle: f64, q: usize, what: &str) -> Result<f64> {
    let delta =
        (angle - q as f64 * QUARTER + std::f64::consts::PI).rem_euclid(TAU) - std::f64::consts::PI;
    if !(-1e-9..=QUARTER + 1e-9).contains(&delta) {
        return Err(unsupported(format!(
            "Torus/sphere Boolean: network vertex outside its patch ({what})"
        )));
    }
    let (sin, cos) = delta.clamp(0., QUARTER).sin_cos();
    Ok(quarter_arc_parameter(cos, sin).clamp(0., 1.))
}

impl Mate for AxialTorus<'_> {
    fn inside(&self, point: [f64; 3], band: f64) -> Result<bool> {
        let (axial, perp) = self.local(point);
        let distance = (norm(perp) - self.canon.major).hypot(axial) - self.canon.minor;
        if distance < -band {
            Ok(true)
        } else if distance > band {
            Ok(false)
        } else {
            Err(unsupported(
                "Torus/sphere Boolean: region classification falls within the error band of the \
                 torus surface",
            ))
        }
    }

    fn uv_of(&self, face: usize, point: [f64; 3]) -> Result<[f64; 2]> {
        let (j, i) = self.quadrant_of[face];
        if j == usize::MAX {
            return Err(unsupported("Torus/sphere Boolean: face outside the torus"));
        }
        let (profile, revolution) = self.angles(point);
        Ok([
            local_parameter(revolution, i, "revolution")?,
            local_parameter(profile, j, "profile")?,
        ])
    }

    fn arc_pcurve(&self, face: usize, arc: &Curve) -> Result<Curve> {
        // A parallel piece between two meridians is the patch's own u-arc at
        // constant v: a straight pcurve, verified by the engine.
        let a = self.uv_of(face, point3(&arc.control_points[0]))?;
        let b = self.uv_of(face, point3(&arc.control_points[2]))?;
        Ok(Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1., 1.],
            periodic: false,
        })
    }

    fn network(&self, imprint: &mut Imprint<'_, Self>) -> Result<Option<SpatialRelation>> {
        let band = imprint.band;
        let sphere = imprint.sphere.clone();
        let t = &self.canon;
        let (h, perp) = self.local(sphere.center);
        let offset = norm(perp);
        let (big_r, r_t, r_s) = (t.major, t.minor, sphere.radius);
        if offset > band {
            let dmin = (offset - big_r).hypot(h);
            let dmax = (offset + big_r).hypot(h);
            if r_s + band < (dmin - r_t).abs() || r_s > dmax + r_t + band {
                // No section: the engine samples containment.
                return Ok(None);
            }
            return Err(unsupported(
                "Torus/sphere Boolean: only a sphere centred on the torus axis meets the torus \
                 in exact circles; a general contact needs a quartic and is refused",
            ));
        }
        // Meridian circle/circle: torus profile centre (R, 0), sphere (0, h).
        let d = big_r.hypot(h);
        let (sum, diff) = (r_t + r_s, (r_t - r_s).abs());
        if d > sum + band || d + band < diff {
            return Ok(None);
        }
        if (d - sum).abs() <= band || (d - diff).abs() <= band {
            return Err(unsupported(
                "Torus/sphere Boolean: the sphere is tangent to the torus",
            ));
        }
        let a = (d * d + r_t * r_t - r_s * r_s) / (2. * d);
        let l2 = r_t * r_t - a * a;
        if !(l2 > 0.) {
            return Err(unsupported(
                "Torus/sphere Boolean: the sphere is tangent to the torus",
            ));
        }
        let l = l2.sqrt();
        for sign in [1., -1.] {
            let rho = big_r - (a * big_r + sign * l * h) / d;
            let z = (a * h - sign * l * big_r) / d;
            self.parallel(imprint, z, rho)?;
        }
        Ok(None)
    }
}

impl AxialTorus<'_> {
    /// One parallel at axial height `z` with radius `rho`: four hits on the
    /// meridians of its profile row and one quarter interval per patch.
    fn parallel(&self, imprint: &mut Imprint<'_, Self>, z: f64, rho: f64) -> Result<()> {
        let t = &self.canon;
        let profile = z.atan2(rho - t.major).rem_euclid(TAU);
        let row = (profile / QUARTER).floor() as usize % 4;
        let local = profile - row as f64 * QUARTER;
        // A parallel on a profile seam would run along torus edges.
        let seam_band = (imprint.band / t.minor).max(1e-9);
        if local < seam_band || local > QUARTER - seam_band {
            return Err(unsupported(
                "Torus/sphere Boolean: a section circle runs along a torus patch seam",
            ));
        }
        let center: [f64; 3] = std::array::from_fn(|k| t.center[k] + z * t.axis[k]);
        let [x, y] = t.frame;
        let mut hits = [usize::MAX; 4];
        for i in 0..4 {
            let edge = self.meridians[row][i];
            let radial = self.radial(i as f64 * QUARTER);
            let point: [f64; 3] = std::array::from_fn(|k| center[k] + rho * radial[k]);
            let profile_center: [f64; 3] =
                std::array::from_fn(|k| t.center[k] + t.major * radial[k]);
            let curve = &self.model.edges[edge].curve;
            let start = point3(&curve.control_points[0]);
            let (cos, sin) = quarter_angle(profile_center, start, point);
            let s = quarter_arc_parameter(cos, sin);
            if !(1e-9..=1. - 1e-9).contains(&s) {
                return Err(unsupported(
                    "Torus/sphere Boolean: parallel hit outside its meridian edge",
                ));
            }
            hits[i] = imprint.hit(point, edge, s);
        }
        for i in 0..4 {
            let mut circle = Circle::new(t.patches[row][i], center, rho, x, y);
            circle.hits.push((i as f64 * QUARTER, hits[i]));
            circle.push_interval(
                i as f64 * QUARTER,
                (i + 1) as f64 * QUARTER,
                hits[i],
                hits[(i + 1) % 4],
            );
            imprint.circles.push(circle);
        }
        Ok(())
    }
}

/// Torus/sphere Boolean entry: `Ok(None)` unless one operand is the canonical
/// torus and the other the canonical sphere.
pub(crate) fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Option<Model>> {
    if !matches!(operation, "union" | "difference" | "intersection") {
        return Ok(None);
    }
    let (mate_index, torus, sphere) = match (
        AxialTorus::recognize(a)?,
        sphere_sphere::recognize(b)?,
        AxialTorus::recognize(b)?,
        sphere_sphere::recognize(a)?,
    ) {
        (Some(torus), Some(sphere), _, _) => (0usize, torus, sphere),
        (_, _, Some(torus), Some(sphere)) => (1usize, torus, sphere),
        _ => return Ok(None),
    };
    let band = shared_band(&torus, &sphere);
    sphere_mate::run([a, b], mate_index, &torus, sphere, band, operation).map(Some)
}

fn shared_band(torus: &AxialTorus<'_>, sphere: &CanonicalSphere) -> f64 {
    let t = &torus.canon;
    let scale = t.major + t.minor + sphere.radius + norm(sub(sphere.center, t.center)) + 1.;
    t.error + sphere.error + 1e-9 * scale + 64. * f64::EPSILON * scale
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{sphere, torus, transform};

    fn placed_sphere(radius: f64, at: [f64; 3]) -> Model {
        transform::affine(
            &sphere(radius).unwrap(),
            [
                [1., 0., 0., at[0]],
                [0., 1., 0., at[1]],
                [0., 0., 1., at[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    }
    fn all_ops(t: &Model, s: &Model) -> [Model; 3] {
        let run = |op: &str| {
            let model = boolean(t, s, op)
                .unwrap_or_else(|e| panic!("{op}: {e:?}"))
                .unwrap();
            model.validate().unwrap();
            assert!(!model.bodies.is_empty(), "{op}");
            model
        };
        [run("union"), run("difference"), run("intersection")]
    }
    fn on_torus(p: [f64; 3]) -> bool {
        ((p[0].hypot(p[1]) - 3.).hypot(p[2]) - 1.).abs() < 1e-9
    }
    fn on_sphere(p: [f64; 3], c: [f64; 3], r: f64) -> bool {
        (norm(sub(p, c)) - r).abs() < 1e-9
    }

    #[test]
    fn sphere_on_the_axis_cuts_two_parallels() {
        // Torus R 3, r 1; sphere radius 3 centred 0.5 above the torus centre.
        let t = torus(3., 1.).unwrap();
        let center = [0., 0., 0.5];
        let s = placed_sphere(3., center);
        let [union, difference, intersection] = all_ops(&t, &s);
        for model in [&union, &difference, &intersection] {
            let section: Vec<_> = model
                .vertices
                .iter()
                .filter(|v| on_torus(v.point) && on_sphere(v.point, center, 3.))
                .collect();
            assert_eq!(section.len(), 8, "four hits on each of two parallels");
        }
        // The inner part of the ring lies inside the sphere: the difference
        // keeps only the outer part, still one connected ring.
        assert_eq!(difference.bodies.len(), 1);
        assert_eq!(intersection.bodies.len(), 1);
    }

    #[test]
    fn misses_and_containment_off_the_axis() {
        let t = torus(3., 1.).unwrap();
        // Inside the tube, off the axis: the torus contains the sphere.
        let cavity = boolean(&t, &placed_sphere(0.3, [3.1, 0., 0.]), "difference")
            .unwrap()
            .unwrap();
        assert_eq!(cavity.bodies[0].inner_shells.len(), 1);
        // Beside the torus.
        let far = boolean(&t, &placed_sphere(1., [7., 1., 0.5]), "union")
            .unwrap()
            .unwrap();
        assert_eq!(far.bodies.len(), 2);
        // Swallowing the whole torus from off the axis.
        let swallowed = boolean(&t, &placed_sphere(9., [0.7, -0.4, 1.]), "intersection")
            .unwrap()
            .unwrap();
        assert_eq!(swallowed.faces.len(), 16);
    }

    #[test]
    fn off_axis_contacts_tangencies_and_seams_are_refused() {
        let t = torus(3., 1.).unwrap();
        for s in [
            placed_sphere(1., [4.5, 0., 0.]),
            // Externally tangent on the axis: d = r_t + r_s.
            placed_sphere(10_f64.sqrt() - 1., [0., 0., 1.]),
            // Centred on the torus centre with radius sqrt(R^2 + r^2): the
            // parallels are the top and bottom circles, which are seams.
            placed_sphere(10_f64.sqrt(), [0., 0., 0.]),
        ] {
            assert_eq!(
                boolean(&t, &s, "union").unwrap_err().code,
                "BREP_UNSUPPORTED_OPERATION"
            );
        }
    }
}
