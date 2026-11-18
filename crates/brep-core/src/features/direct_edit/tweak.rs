use super::movement::CanonicalPolygon;
use super::*;
use crate::analysis::draft::{DraftReport, draft_angle};

/// Outcome of one successful face tweak.
#[derive(Clone, Debug)]
pub struct TweakFaceReport {
    /// The staged, validated and audited result model.
    pub model: Model,
    /// Input-model index of the tweaked face.
    pub face: usize,
    /// Coplanar shard faces rotated together with the selected face.
    pub driven_faces: Vec<usize>,
    /// The (normalized) rotation axis actually used.
    pub axis: Axis,
    /// Signed rotation applied, right-hand rule about `axis.direction`.
    pub angle_rad: f64,
    /// The pre-computed draft limit for the rotation direction (radians),
    /// when the planar prediction applies.
    pub max_angle_rad: Option<f64>,
    /// Vertices re-positioned by the re-trim (rigidly rotated or slid along
    /// the neighbor-plane intersection lines).
    pub moved_vertices: usize,
    /// Faces that collapsed to zero area and were absorbed by the re-sew.
    pub absorbed_faces: Vec<usize>,
    pub volume_before_mm3: f64,
    pub volume_after_mm3: f64,
    /// First-order estimate: −θ · ∫_R (r·m) dA with m = k×n (exact triangle
    /// moments of the driven region about the axis).
    pub delta_volume_estimate_mm3: f64,
    /// Measured delta from `mass_properties` (after − before).
    pub delta_volume_actual_mm3: f64,
    /// The pre-operation draft analysis (acceptance evidence).
    pub draft: DraftReport,
    /// Post-operation solid audit certificate (acceptance gate).
    pub audit: SolidAuditCertificate,
}

/// Rotate `face` about `axis` by `angle_rad` (checklist 873).
///
/// Iteration 1 admits a *planar* face of a *planar polygonal* body with the
/// axis lying in the face plane (the classic draft tilt of a wall). The
/// whole coplanar shard region rotates as one rigid plane; every adjacent
/// edge is re-trimmed by re-positioning the shared vertices, not only the
/// edges bounding the tweaked face: a vertex with two non-driven incident
/// faces slides to `P'(θ) ∩ N1 ∩ N2`, a vertex interior to the driven
/// region rotates rigidly. Curved faces, curved neighbors (cylinders
/// included), off-plane axes and 1/≥3-face corners fail with typed errors
/// — plane∩cylinder re-trims are future work, never guessed.
///
/// Before any mutation, [`draft_angle`] predicts the maximum admissible
/// angle for the rotation direction; exceeding it returns
/// [`TWEAK_LIMIT`] carrying the limit and leaves the model untouched.
/// Transaction contour as in move/offset: snapshot → staged rebuild →
/// validate → audit → measure; the input is never mutated.
pub fn tweak_face(
    model: &Model,
    face: usize,
    axis: Axis,
    angle_rad: f64,
    budget: &Budget,
) -> Result<TweakFaceReport> {
    let snapshot = ModelSnapshot::new(model.clone())?;
    let source = snapshot.model();
    let axis = validated_tweak_axis(source, face, axis, angle_rad)?;
    let mut guard = budget.guard("tweak-face");
    guard.check()?;
    check_planar_polygonal_body(source, &mut guard)?;

    let polygons = canonical_polygons(source, &BTreeMap::new(), &mut guard)?;
    let plane = polygons[face].plane;
    let linear = source.tolerance_mm * 8.;
    check_axis_in_plane(&axis, plane, linear)?;

    let adjacency = edge_face_adjacency(source, &mut guard)?;
    let driven = coplanar_component(
        source,
        face,
        plane,
        &BTreeSet::new(),
        &adjacency,
        &mut guard,
    )?;

    // Pre-operation draft analysis: the angle limit is enforced BEFORE any
    // mutation attempt; the error carries the predicted maximum.
    let draft = draft_angle(source, face, axis, budget)?;
    let limit = draft_limit(&draft, angle_rad)?;

    // Rotated plane: the axis point is fixed and lies on the plane.
    let rotated_normal = rotate_vector(&axis, angle_rad, plane.normal);
    let tweak = PlaneTweak {
        source,
        axis,
        angle_rad,
        rotated: PlaneEq {
            normal: rotated_normal,
            offset: dot(rotated_normal, axis.point),
        },
        limit,
        linear,
        driven: &driven,
        polygons: &polygons,
    };
    let substitute = tweak.retrim_vertices(&mut guard)?;
    let moved = canonical_polygons(source, &substitute, &mut guard)?;
    let absorbed = tweak.absorbed_faces(&moved, &mut guard)?;
    let staged = tweak.rebuild(&moved, &absorbed, &mut guard)?;
    staged.validate()?;
    let audit = crate::solid_audit::audit_solid(&staged)?;
    let volume_before = volume_of(source)?;
    let volume_after = volume_of(&staged)?;
    let moment = tweak.driven_moment(plane.normal);
    guard.check()?;
    Ok(TweakFaceReport {
        model: staged,
        face,
        driven_faces: driven.iter().copied().filter(|&d| d != face).collect(),
        axis,
        angle_rad,
        max_angle_rad: limit,
        moved_vertices: substitute.len(),
        absorbed_faces: absorbed.iter().copied().collect(),
        volume_before_mm3: volume_before,
        volume_after_mm3: volume_after,
        delta_volume_estimate_mm3: -angle_rad * moment,
        delta_volume_actual_mm3: volume_after - volume_before,
        draft,
        audit,
    })
}

/// Angles at or below this magnitude are a no-op and refused.
const MIN_TWEAK_ANGLE_RAD: f64 = 1e-12;
/// `|cos|` above which the axis is not parallel to the face plane.
const AXIS_IN_PLANE_COS: f64 = 1e-9;
/// Neighbor planes closer to parallel than this have no slide line.
const SLIDE_LINE_PARALLEL: f64 = 1e-9;
/// Slide lines closer to parallel to the rotated plane are refused.
const SLIDE_PLANE_PARALLEL: f64 = 1e-12;
/// Fewest faces a rebuilt closed polyhedron can have.
const MIN_SOLID_FACES: usize = 4;

/// Input gates (face range, angle, axis, single body); returns the axis with
/// a unit direction.
fn validated_tweak_axis(source: &Model, face: usize, axis: Axis, angle_rad: f64) -> Result<Axis> {
    if face >= source.faces.len() {
        return Err(error(TWEAK_INVALID, format!("Face {face} is out of range")));
    }
    if !angle_rad.is_finite() || angle_rad.abs() > std::f64::consts::PI {
        return Err(error(
            TWEAK_INVALID,
            "Angle must be finite and within ±π radians",
        ));
    }
    if angle_rad.abs() <= MIN_TWEAK_ANGLE_RAD {
        return Err(error(TWEAK_INVALID, "Angle must be nonzero"));
    }
    if axis
        .point
        .iter()
        .chain(&axis.direction)
        .any(|v| !v.is_finite())
    {
        return Err(error(TWEAK_INVALID, "Axis point/direction must be finite"));
    }
    let length = norm(axis.direction);
    if length <= 0. {
        return Err(error(TWEAK_INVALID, "Axis direction must be nonzero"));
    }
    if source.bodies.len() > 1 {
        return Err(error(
            TWEAK_UNSUPPORTED,
            "Multi-body models are outside tweak-face/1",
        ));
    }
    Ok(Axis {
        point: axis.point,
        direction: mul(axis.direction, 1. / length),
    })
}

/// Planar polygonal body gate (same subset as move-face/1): curved faces or
/// curved boundary edges anywhere fail closed with the face named.
fn check_planar_polygonal_body(source: &Model, guard: &mut BudgetGuard) -> Result<()> {
    for (i, f) in source.faces.iter().enumerate() {
        guard.tick()?;
        face_plane_fit(source, i).map_err(|_| {
            error(
                TWEAK_UNSUPPORTED,
                format!("Face {i} is not planar; tweak-face/1 admits planar bodies only"),
            )
        })?;
        for &ring in std::iter::once(&f.outer).chain(&f.holes) {
            if source.loops[ring]
                .coedges
                .iter()
                .any(|c| source.edges[c.edge].curve.degree != 1)
            {
                return Err(error(
                    TWEAK_UNSUPPORTED,
                    format!(
                        "Face {i} has curved boundary edges; tweak-face/1 admits straight edges only"
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// The axis must lie in the face plane (draft tilt, not a free hinge).
fn check_axis_in_plane(axis: &Axis, plane: PlaneEq, linear: f64) -> Result<()> {
    if (dot(plane.normal, axis.point) - plane.offset).abs() > linear {
        return Err(error(
            TWEAK_INVALID,
            "Axis point is not on the tweaked face plane",
        ));
    }
    if dot(plane.normal, axis.direction).abs() > AXIS_IN_PLANE_COS {
        return Err(error(
            TWEAK_INVALID,
            "Axis direction is not parallel to the tweaked face plane",
        ));
    }
    Ok(())
}

/// Draft limit for the rotation direction; refuses angles reaching it.
fn draft_limit(draft: &DraftReport, angle_rad: f64) -> Result<Option<f64>> {
    let limit = if angle_rad > 0. {
        draft.max_angle_positive_rad
    } else {
        draft.max_angle_negative_rad
    };
    if let Some(max) = limit
        && angle_rad.abs() >= max
    {
        return Err(error(
            TWEAK_LIMIT,
            format!(
                "Angle {angle_rad} rad exceeds the draft limit {max} rad ({} deg) predicted from the neighbor geometry; the edit is blocked before mutation",
                max.to_degrees()
            ),
        ));
    }
    Ok(limit)
}

/// One validated plane rotation of a driven coplanar region.
struct PlaneTweak<'a> {
    source: &'a Model,
    axis: Axis,
    angle_rad: f64,
    rotated: PlaneEq,
    limit: Option<f64>,
    linear: f64,
    driven: &'a BTreeSet<usize>,
    polygons: &'a [CanonicalPolygon],
}

impl PlaneTweak<'_> {
    /// Re-trim: every vertex of the driven region is re-positioned against
    /// the *neighbor* planes, which re-trims all adjacent edges (including
    /// edges between two neighbors that meet at a driven vertex).
    fn retrim_vertices(&self, guard: &mut BudgetGuard) -> Result<BTreeMap<usize, [f64; 3]>> {
        let source = self.source;
        let incidence = vertex_face_incidence(source);
        let mut substitute: BTreeMap<usize, [f64; 3]> = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for &d in self.driven {
            for &ring in std::iter::once(&source.faces[d].outer).chain(&source.faces[d].holes) {
                for v in loop_vertices(source, ring)? {
                    if !seen.insert(v) {
                        continue;
                    }
                    guard.tick()?;
                    if let Some(moved) = self.retrim_vertex(v, &incidence[v])? {
                        substitute.insert(v, moved);
                    }
                }
            }
        }
        Ok(substitute)
    }

    /// New position of one driven vertex; `None` for a pivot on the axis.
    fn retrim_vertex(&self, v: usize, incident: &BTreeSet<usize>) -> Result<Option<[f64; 3]>> {
        let axis = &self.axis;
        let p = self.source.vertices[v].point;
        let r = sub(p, axis.point);
        let radial = sub(r, mul(axis.direction, dot(r, axis.direction)));
        if norm(radial) <= self.linear {
            return Ok(None); // pivot vertex on the axis: unchanged
        }
        let non_driven: Vec<usize> = incident
            .iter()
            .copied()
            .filter(|f| !self.driven.contains(f))
            .collect();
        let moved = match non_driven[..] {
            [] => rotate_point(axis, self.angle_rad, p),
            [a, b] => self.slide(v, a, b)?,
            _ => {
                return Err(error(
                    TWEAK_UNSUPPORTED,
                    format!(
                        "Vertex {v} touches {} non-driven faces; tweak-face/1 admits 0 (rigid) or 2 (slide) only",
                        non_driven.len()
                    ),
                ));
            }
        };
        if moved.iter().any(|c| !c.is_finite()) {
            return Err(error(
                TWEAK_UNSUPPORTED,
                format!("Vertex {v} re-trim produced a non-finite point"),
            ));
        }
        Ok(Some(moved))
    }

    /// Slide vertex `v` along the line of neighbor planes `a`, `b` onto the
    /// rotated plane.
    fn slide(&self, v: usize, a: usize, b: usize) -> Result<[f64; 3]> {
        let (n1, n2) = (self.polygons[a].plane, self.polygons[b].plane);
        let Some((lp, ld)) = intersect_planes(n1, n2, SLIDE_LINE_PARALLEL) else {
            return Err(error(
                TWEAK_UNSUPPORTED,
                format!("Vertex {v}: neighbor planes are parallel; the slide line is undefined"),
            ));
        };
        let limit = self.limit;
        line_plane(lp, ld, self.rotated, SLIDE_PLANE_PARALLEL).ok_or_else(|| {
            error(
                TWEAK_LIMIT,
                format!(
                    "Vertex {v}: the slide line is parallel to the rotated plane at this angle; the draft limit is {limit:?} rad"
                ),
            )
        })
    }

    /// Absorption / inversion gate, as in move-face: collapsed neighbors are
    /// re-sewn, inverted geometry is refused (the draft limit should already
    /// have blocked it; this is the defensive second gate).
    fn absorbed_faces(
        &self,
        moved: &[CanonicalPolygon],
        guard: &mut BudgetGuard,
    ) -> Result<BTreeSet<usize>> {
        let area_floor = self.linear * self.linear;
        let mut absorbed: BTreeSet<usize> = BTreeSet::new();
        for (i, (poly, before)) in moved.iter().zip(self.polygons).enumerate() {
            guard.tick()?;
            if self.driven.contains(&i) {
                continue;
            }
            if norm(poly.area_vec) <= area_floor {
                absorbed.insert(i);
                continue;
            }
            if norm(before.area_vec) > area_floor && dot(poly.area_vec, before.area_vec) < 0. {
                return Err(error(
                    TWEAK_UNSUPPORTED,
                    format!("Tweak inverts face {i}; partial crossing is refused"),
                ));
            }
        }
        Ok(absorbed)
    }

    /// Re-sew the surviving polygons into the staged model.
    fn rebuild(
        &self,
        moved: &[CanonicalPolygon],
        absorbed: &BTreeSet<usize>,
        guard: &mut BudgetGuard,
    ) -> Result<Model> {
        let kept: Vec<PlanarBoundary> = moved
            .iter()
            .enumerate()
            .filter(|(i, _)| !absorbed.contains(i))
            .map(|(_, p)| p.boundary.clone())
            .collect();
        if kept.len() < MIN_SOLID_FACES {
            return Err(error(
                TWEAK_UNSUPPORTED,
                "Tweak consumes the whole body; refused instead of returning an empty solid",
            ));
        }
        let mut staged =
            model_from_trimmed_polygons(kept, self.source.tolerance_mm).map_err(|e| {
                Error::new(
                    TWEAK_UNSUPPORTED,
                    format!("Tweak rebuild failed: {}", e.message),
                )
            })?;
        restore_unchanged_ids(&mut staged, self.source, guard)?;
        staged.refresh_change_set(&[self.source]);
        Ok(staged)
    }

    /// First-order ΔV moment ∫_R (r·m) dA over the driven region, m = k×n.
    /// r·m is linear across each triangle, so the fan moments are exact.
    fn driven_moment(&self, plane_normal: [f64; 3]) -> f64 {
        let axis = &self.axis;
        let m = cross(axis.direction, plane_normal);
        let mut moment = 0.;
        for &d in self.driven {
            let polygon = &self.polygons[d];
            let n = polygon.plane.normal;
            for ring in std::iter::once(&polygon.boundary.outer).chain(&polygon.boundary.holes) {
                for i in 1..ring.len().saturating_sub(1) {
                    let (a, b, c) = (ring[0], ring[i], ring[i + 1]);
                    let signed_area = 0.5 * dot(cross(sub(b, a), sub(c, a)), n);
                    let mean = (dot(sub(a, axis.point), m)
                        + dot(sub(b, axis.point), m)
                        + dot(sub(c, axis.point), m))
                        / 3.;
                    moment += signed_area * mean;
                }
            }
        }
        moment
    }
}
