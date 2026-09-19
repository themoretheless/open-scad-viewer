//! Regularized curved Boolean closure for canonical sphere/sphere pairs, as
//! the `Mate` of `sphere_mate`: the second sphere authors the section
//! network (one exact circle, cut into per-patch intervals at its own
//! seams) and the shared engine splits, arranges, classifies and assembles.
//!
//! Both operands must be canonical stereographic sphere solids as certified
//! by `intersections::sphere_sphere::recognize`. A provably disjoint or
//! strictly contained pair resolves by regularized empty algebra (a strict
//! containment difference keeps the outer body with the inner sphere as an
//! inverted cavity shell). Anything outside the certificate is an explicit
//! `BREP_UNSUPPORTED_OPERATION`, never a numerical fallback: tangency and
//! coincidence bands, a section plane through a patch pole of the second
//! sphere, and sections through its vertices or grazing its seams.
use crate::Model;
use crate::imprint_pipeline::SpatialRelation;
use crate::intersections::Options;
use crate::intersections::sphere_sphere::{
    self, CanonicalSphere, PatchUvSection, SphereSphereComponent,
};
use crate::sphere_mate::{
    self, BOUNDARY_UV, CORNER, Circle, Imprint, Mate, TAU, cross, dist, norm, piece_of, piece_tau,
    point3, snap_to_piece, sphere_arc_pcurve, strictly_inside_quarter_disk, sub, unsupported,
};
use nurbs_core::{Result, curve::Curve};

/// Sphere/sphere Boolean entry: `Ok(None)` unless both operands are canonical
/// spheres and the operation is union, difference or intersection.
pub(crate) fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Option<Model>> {
    if !matches!(operation, "union" | "difference" | "intersection") {
        return Ok(None);
    }
    let (Some(sa), Some(sb)) = (sphere_sphere::recognize(a)?, sphere_sphere::recognize(b)?) else {
        return Ok(None);
    };
    let report = sphere_sphere::intersect_sphere_sphere(a, b, Options::default())?;
    if let Some(unresolved) = report.unresolved.first() {
        return Err(unsupported(format!(
            "Sphere/sphere Boolean cannot certify the intersection ({:?}); \
             tangency, coincidence and vertex/grazing contacts are not regularized",
            unresolved.reason
        )));
    }
    let tolerance = a.tolerance_mm.max(b.tolerance_mm);
    match &report.components[..] {
        [] => Ok(Some(separated_or_contained(
            a, b, operation, &sa, &sb, tolerance,
        )?)),
        [
            SphereSphereComponent::Circle {
                center,
                radius,
                normal,
                ..
            },
        ] => {
            let scale = sa.radius + sb.radius + dist(sa.center, sb.center) + 1.;
            let band = sa.error + sb.error + 1e-9 * scale + 64. * f64::EPSILON * scale;
            let mate = SphereMate {
                model: b,
                canon: sb,
                pieces: Imprint::<SphereMate>::sphere_boundary_pieces(b),
                circle: (*center, *radius, *normal),
            };
            sphere_mate::run(a, b, 1, &mate, sa, band, operation).map(Some)
        }
        _ => Err(unsupported(
            "Sphere/sphere Boolean: unexpected multiple intersection components",
        )),
    }
}

/// Empty-component resolution: provably disjoint or strictly contained pairs.
fn separated_or_contained(
    a: &Model,
    b: &Model,
    operation: &str,
    sa: &CanonicalSphere,
    sb: &CanonicalSphere,
    tolerance: f64,
) -> Result<Model> {
    // Same outward classification band as the intersection query.
    let delta = sub(sb.center, sa.center);
    let scale = delta.iter().map(|v| v.abs()).fold(0., f64::max);
    let distance = if scale == 0. {
        0.
    } else {
        let scaled = delta.map(|x| x / scale);
        scaled[0].hypot(scaled[1]).hypot(scaled[2]) * scale
    };
    let band =
        sa.error + sb.error + 16. * f64::EPSILON * (distance + scale + sa.radius + sb.radius + 1.);
    let d_lo = (distance - band).max(0.);
    let d_hi = distance + band;
    let sum = sa.radius + sb.radius;
    let diff = (sa.radius - sb.radius).abs();
    if d_lo > sum + band {
        return Ok(match operation {
            "intersection" => Model::empty(tolerance)?,
            "difference" => a.clone(),
            _ => crate::boolean_support::separated_union(a, b)?,
        });
    }
    if d_hi < (diff - band).max(0.) {
        let (outer, inner, a_is_outer) = if sa.radius >= sb.radius {
            (a, b, true)
        } else {
            (b, a, false)
        };
        return Ok(match operation {
            "union" => outer.clone(),
            "intersection" => inner.clone(),
            _ => {
                if a_is_outer {
                    crate::imprint_pipeline::cavity(outer, inner, tolerance)?
                } else {
                    Model::empty(tolerance)?
                }
            }
        });
    }
    Err(unsupported(
        "Sphere/sphere Boolean: separation classification falls inside the error band",
    ))
}

/// The second sphere as the mate of the engine.
struct SphereMate<'m> {
    model: &'m Model,
    canon: CanonicalSphere,
    /// Per face, coedge index of each quarter-disk boundary piece.
    pieces: [[usize; 3]; 8],
    /// Certified section circle: (center, radius, unit normal).
    circle: ([f64; 3], f64, [f64; 3]),
}

impl Mate for SphereMate<'_> {
    fn model(&self) -> &Model {
        self.model
    }

    fn inside(&self, point: [f64; 3], band: f64) -> Result<bool> {
        let d = dist(point, self.canon.center) - self.canon.radius;
        if d < -band {
            Ok(true)
        } else if d > band {
            Ok(false)
        } else {
            Err(unsupported(
                "Sphere/sphere Boolean: region classification falls within the error band \
                 of the second sphere",
            ))
        }
    }

    /// Interior points invert exactly; points on the patch rim snap to the
    /// boundary piece they lie on.
    fn uv_of(&self, face: usize, point: [f64; 3]) -> Result<[f64; 2]> {
        let uv = self.canon.invert_uv(face, point).ok_or_else(|| {
            unsupported("Sphere/sphere Boolean: network vertex escapes its patch chart")
        })?;
        if strictly_inside_quarter_disk(uv, CORNER) {
            return Ok(uv);
        }
        let piece = piece_of(uv)?;
        Ok(snap_to_piece(piece, uv))
    }

    fn arc_pcurve(&self, face: usize, arc: &Curve) -> Result<Curve> {
        sphere_arc_pcurve(&self.canon, face, arc)
    }

    fn network(&self, imprint: &mut Imprint<'_, Self>) -> Result<Option<SpatialRelation>> {
        let (center, radius, normal) = self.circle;
        // Any orthonormal basis of the circle plane; +phi runs counter-
        // clockwise about the normal.
        let seed = if normal[0].abs() < 0.9 {
            [1., 0., 0.]
        } else {
            [0., 1., 0.]
        };
        let e1 = {
            let proj = normal
                .map(|n| n * (seed[0] * normal[0] + seed[1] * normal[1] + seed[2] * normal[2]));
            let raw = sub(seed, proj);
            raw.map(|x| x / norm(raw))
        };
        let e2 = cross(normal, e1);
        let model = self.model;
        let sections = sphere_sphere::patch_sections(&self.canon, normal, center);
        for (patch, section) in sections {
            let PatchUvSection::Circle {
                center: cc,
                radius: ruv,
                start,
                end,
            } = section
            else {
                return Err(unsupported(
                    "Sphere/sphere Boolean: section plane contains a patch pole of the \
                     second sphere (great-circle section through original vertices)",
                ));
            };
            let mut circle = Circle::new(patch, center, radius, e1, e2);
            if end - start >= TAU - 1e-9 {
                let q = cc[0].hypot(cc[1]);
                if cc[0] - ruv <= CORNER || cc[1] - ruv <= CORNER || q + ruv >= 1. - CORNER {
                    return Err(unsupported(
                        "Sphere/sphere Boolean: section circle is tangent to a patch boundary \
                         of the second sphere",
                    ));
                }
                circle.push_ring();
                imprint.circles.push(circle);
                continue;
            }
            // Chord: both ends are hits on this sphere's seams.
            let mut ends = [(0usize, 0.); 2];
            for (which, angle) in [(0usize, start), (1, end)] {
                let raw = [cc[0] + ruv * angle.cos(), cc[1] + ruv * angle.sin()];
                let piece = piece_of(raw)?;
                let w = snap_to_piece(piece, raw);
                let tau = piece_tau(piece, w);
                let coedge =
                    &model.loops[model.faces[patch].outer].coedges[self.pieces[patch][piece]];
                let t = if coedge.reversed { 1. - tau } else { tau };
                if !(1e-9..=1. - 1e-9).contains(&t) {
                    return Err(unsupported(
                        "Sphere/sphere Boolean: section passes through a vertex of the \
                         second sphere",
                    ));
                }
                let point = point3(&model.faces[patch].surface.evaluate(w[0], w[1])?.point);
                let id = imprint.hit(point, coedge.edge, t);
                ends[which] = (id, circle.phi_of(point));
            }
            // The interval inside this patch is whichever way round the
            // circle keeps its midpoint strictly inside the quarter disk.
            let (ida, pa) = ends[0];
            let (idb, pb) = ends[1];
            let forward_hi = pa + (pb - pa).rem_euclid(TAU);
            let mid = circle.point_at((pa + forward_hi) / 2.);
            let inside = self
                .canon
                .invert_uv(patch, mid)
                .map(|uv| strictly_inside_quarter_disk(uv, BOUNDARY_UV))
                .unwrap_or(false);
            if inside {
                circle.hits.push((pa, ida));
                circle.push_interval(pa, forward_hi, ida, idb);
            } else {
                let backward_hi = pb + (pa - pb).rem_euclid(TAU);
                let mid = circle.point_at((pb + backward_hi) / 2.);
                let inside = self
                    .canon
                    .invert_uv(patch, mid)
                    .map(|uv| strictly_inside_quarter_disk(uv, BOUNDARY_UV))
                    .unwrap_or(false);
                if !inside {
                    return Err(unsupported(
                        "Sphere/sphere Boolean: neither arc between the seam crossings lies \
                         inside the patch",
                    ));
                }
                circle.hits.push((pb, idb));
                circle.push_interval(pb, backward_hi, idb, ida);
            }
            imprint.circles.push(circle);
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{sphere, transform};

    fn placed(radius: f64, at: [f64; 3]) -> Model {
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
    fn x_range(model: &Model) -> (f64, f64) {
        model
            .vertices
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v.point[0]), hi.max(v.point[0]))
            })
    }
    fn all_ops(a: &Model, b: &Model) -> [Model; 3] {
        let run = |op: &str| {
            let model = boolean(a, b, op).unwrap().unwrap();
            model.validate().unwrap();
            assert_eq!(model.bodies.len(), 1, "{op}");
            model
        };
        [run("union"), run("difference"), run("intersection")]
    }

    #[test]
    fn overlapping_equal_spheres() {
        let a = sphere(3.).unwrap();
        let b = placed(3., [4.1, 0.3, -0.2]);
        let [union, difference, intersection] = all_ops(&a, &b);
        assert!((x_range(&union).0 + 3.).abs() < 1e-9);
        assert!((x_range(&union).1 - 7.1).abs() < 1e-9);
        assert!((x_range(&difference).0 + 3.).abs() < 1e-9);
        assert!(x_range(&difference).1 < 3. + 1e-9);
        assert!(
            (x_range(&intersection).0 - 1.1).abs() < 1e-9,
            "b's -x vertex"
        );
        assert!(
            (x_range(&intersection).1 - 3.).abs() < 1e-9,
            "a's +x vertex"
        );
        let swapped = boolean(&b, &a, "difference").unwrap().unwrap();
        swapped.validate().unwrap();
    }

    #[test]
    fn small_sphere_bulging_out_of_a_big_one() {
        let a = sphere(5.).unwrap();
        let b = placed(1.5, [4.6, 0.7, 0.9]);
        let [union, difference, intersection] = all_ops(&a, &b);
        assert!(union.faces.len() > 8);
        assert!(difference.faces.len() > 8);
        assert!((x_range(&union).1 - 6.1).abs() < 1e-9);
        assert!((x_range(&intersection).0 - 3.1).abs() < 1e-9);
    }

    #[test]
    fn contained_and_disjoint_pairs_use_empty_algebra() {
        let a = sphere(5.).unwrap();
        let inside = placed(1., [1., 0.5, -0.5]);
        let cavity = boolean(&a, &inside, "difference").unwrap().unwrap();
        assert_eq!(cavity.bodies[0].inner_shells.len(), 1);
        assert!(
            boolean(&inside, &a, "difference")
                .unwrap()
                .unwrap()
                .is_empty()
        );
        let far = placed(1., [20., 0., 0.]);
        assert_eq!(boolean(&a, &far, "union").unwrap().unwrap().bodies.len(), 2);
        assert!(
            boolean(&a, &far, "intersection")
                .unwrap()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn tangent_and_seam_aligned_pairs_are_refused() {
        let a = sphere(3.).unwrap();
        let tangent = placed(2., [5., 0., 0.]);
        assert_eq!(
            boolean(&a, &tangent, "union").unwrap_err().code,
            "BREP_UNSUPPORTED_OPERATION"
        );
        // Radical plane x = 4 passes through the second sphere's centre and
        // so contains its poles: refused, never approximated.
        let big = sphere(5.).unwrap();
        let axial = placed(3., [4., 0., 0.]);
        assert_eq!(
            boolean(&big, &axial, "union").unwrap_err().code,
            "BREP_UNSUPPORTED_OPERATION"
        );
    }
}
