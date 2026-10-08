//! Boundary-preserving exact weight-profile proposal on an immutable original Body.
//! Closed topology is inherited only after every retained contour is checked;
//! embedding, normals, volume and wall qualification must be rebuilt separately.
use crate::source_volume::Body;
use nurbs_core::{Error, Result, normal_alignment::WeightProfile, surface::Surface};
#[derive(Clone, Copy)]
pub struct FaceSpec {
    pub face: usize,
    pub anchor: [usize; 2],
}
pub struct Limits {
    pub controls: usize,
    pub fragments: usize,
    pub position_tolerance: f64,
}
struct Face<'a> {
    spec: FaceSpec,
    profile: WeightProfile<'a>,
}
pub struct Proposal<'a> {
    body: &'a Body,
    faces: Vec<Face<'a>>,
    axis_deviation_upper: [f64; 3],
    position_tolerance: f64,
}
/// Tangent-plane certificate attached to this exact proposal and an owned
/// original shared carrier. It does not admit volume or shell embedding.
pub struct Seam<'p, 'b> {
    proposal: &'p Proposal<'b>,
    faces: [usize; 2],
    edge: usize,
    threshold: f64,
    upper: f64,
}
impl Seam<'_, '_> {
    pub fn proposal(&self) -> &Proposal<'_> {
        self.proposal
    }
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn threshold(&self) -> f64 {
        self.threshold
    }
    pub fn upper_sine_squared(&self) -> f64 {
        self.upper
    }
}
pub struct SeamReport<'p, 'b> {
    pub seam: Option<Seam<'p, 'b>>,
    pub angular: Option<nurbs_core::normal_alignment::ProfilePathReport>,
    pub reason: &'static str,
}
impl<'a> Proposal<'a> {
    /// Derive both UV paths from private original carrier ownership and compare
    /// each incident face's actual original or proposed weight representation.
    pub fn qualify_seam<'p>(
        &'p self,
        face: usize,
        neighbor: usize,
        max_edges: usize,
        threshold: f64,
        cells: usize,
        spans: usize,
    ) -> Result<SeamReport<'p, 'a>> {
        let mut out = SeamReport {
            seam: None,
            angular: None,
            reason: "profile-seam-carrier-unproven",
        };
        let profile = self
            .profile(face)
            .ok_or_else(|| Error::new("BREP_PROFILE_SEAM", "Choose a proposed face"))?;
        let Some(map) =
            crate::source_shared_chord_map::prepare(self.body, [face, neighbor], max_edges)?
        else {
            return Ok(out);
        };
        let axes = map.fixed_axes();
        let fixed = map.fixed_values();
        let free = map.free_endpoints();
        let lines = std::array::from_fn(|side| {
            std::array::from_fn(|end| {
                std::array::from_fn(|axis| {
                    if axis == axes[side] {
                        fixed[side]
                    } else {
                        free[side][end]
                    }
                })
            })
        });
        let angular = match self.profile(neighbor) {
            Some(other) => profile.inspect_profile_path(other, lines, threshold, cells, spans)?,
            None => {
                profile.inspect_original_path(map.surfaces()[1], lines, threshold, cells, spans)?
            }
        };
        out.reason = angular.reason;
        if angular.aligned == Some(true) {
            out.seam = Some(Seam {
                proposal: self,
                faces: [face, neighbor],
                edge: map.edge(),
                threshold,
                upper: angular.upper_sine_squared.unwrap(),
            });
        }
        out.angular = Some(angular);
        Ok(out)
    }
    pub fn original_body(&self) -> &'a Body {
        self.body
    }
    pub fn profile(&self, face: usize) -> Option<&WeightProfile<'a>> {
        self.faces
            .iter()
            .find(|f| f.spec.face == face)
            .map(|f| &f.profile)
    }
    pub fn specs(&self) -> Vec<FaceSpec> {
        self.faces.iter().map(|f| f.spec).collect()
    }
    pub fn axis_deviation_upper(&self) -> [f64; 3] {
        self.axis_deviation_upper
    }
    /// Persist source and exact factor recipes, never flattened target weights or
    /// claimed geometry certificates. Restoration must rerun all admission gates.
    pub fn definition(&self) -> Result<value_codec::Value> {
        Ok(
            value_codec::json!({"version":1,"kind":"boundary-preserving-weight-profiles",
            "originalBody":self.body.definition()?,"positionTolerance":self.position_tolerance,
            "profiles":self.faces.iter().map(|f|value_codec::json!({"face":f.spec.face,"anchor":f.spec.anchor})).collect::<Vec<_>>()}),
        )
    }
}
pub struct Report<'a> {
    pub proposal: Option<Proposal<'a>>,
    pub controls: usize,
    pub fragments: usize,
    pub reason: &'static str,
}
fn same(a: &Surface, b: &Surface) -> bool {
    a.degree_u == b.degree_u
        && a.degree_v == b.degree_v
        && a.periodic_u == b.periodic_u
        && a.periodic_v == b.periodic_v
        && a.knots_u == b.knots_u
        && a.knots_v == b.knots_v
        && a.control_points == b.control_points
        && a.weights == b.weights
}
pub fn prepare<'a>(body: &'a Body, specs: &[FaceSpec], limits: Limits) -> Result<Report<'a>> {
    let regions = body
        .geometry()
        .shell()
        .regions()
        .ok_or_else(|| Error::new("BREP_PROFILE_BODY", "Require original retained regions"))?;
    if specs.is_empty()
        || specs.len() > regions.len()
        || limits.controls > 1_000_000
        || limits.fragments > 100000
        || !limits.position_tolerance.is_finite()
        || limits.position_tolerance < 0.
        || specs.iter().any(|f| f.face >= regions.len())
        || specs
            .iter()
            .enumerate()
            .any(|(i, f)| specs[..i].iter().any(|a| a.face == f.face))
    {
        return Err(Error::new(
            "BREP_PROFILE_BODY",
            "Choose distinct owned faces and finite bounded proposal limits",
        ));
    }
    let mut out = Report {
        proposal: None,
        controls: 0,
        fragments: 0,
        reason: "profile-body-work-limit",
    };
    let mut faces = Vec::new();
    let mut deviation = [0_f64; 3];
    for spec in specs {
        let region = &regions[spec.face];
        let source = region.loops()[0][0].surface();
        let (profile, used) = nurbs_core::normal_alignment::qualify_weight_profile(
            source,
            spec.anchor,
            limits.controls - out.controls,
        )?;
        out.controls += used;
        let Some(profile) = profile else {
            return Ok(out);
        };
        if !profile.within_position_tolerance(limits.position_tolerance)? {
            out.reason = "profile-body-position-tolerance-unproven";
            return Ok(out);
        }
        // Check all natural boundaries, including collapsed rails absent from
        // the retained wire. This preserves original pole and edge positions.
        for axis in 0..2 {
            for high in [false, true] {
                let (boundary, used) =
                    profile.preserved_boundary(axis, high, limits.controls - out.controls)?;
                out.controls += used;
                if boundary.is_none() {
                    out.reason = "profile-body-boundary-identity-unproven";
                    return Ok(out);
                }
            }
        }
        let domains = [
            [
                source.knots_u[source.degree_u],
                source.knots_u[source.control_points.len()],
            ],
            [
                source.knots_v[source.degree_v],
                source.knots_v[source.control_points[0].len()],
            ],
        ];
        for wire in region.loops() {
            for fragment in wire {
                if out.fragments == limits.fragments {
                    return Ok(out);
                }
                out.fragments += 1;
                if !same(fragment.surface(), source) || fragment.curve().periodic {
                    out.reason = "profile-body-contour-source-unproven";
                    return Ok(out);
                }
                let mut fixed = [true; 4];
                for point in &fragment.curve().control_points {
                    if out.controls == limits.controls {
                        return Ok(out);
                    }
                    out.controls += 1;
                    if point.len() != 2
                        || (0..2).any(|a| point[a] < domains[a][0] || point[a] > domains[a][1])
                    {
                        out.reason = "profile-body-contour-boundary-unproven";
                        return Ok(out);
                    }
                    for axis in 0..2 {
                        for high in 0..2 {
                            fixed[2 * axis + high] &= point[axis] == domains[axis][high];
                        }
                    }
                }
                if !fixed.iter().any(|x| *x) {
                    out.reason = "profile-body-interior-trim-unproven";
                    return Ok(out);
                }
            }
        }
        for (k, error) in profile.axis_deviation_upper().into_iter().enumerate() {
            deviation[k] = deviation[k].max(error);
        }
        faces.push(Face {
            spec: *spec,
            profile,
        });
    }
    out.reason = "profile-body-boundaries-preserved";
    out.proposal = Some(Proposal {
        body,
        faces,
        axis_deviation_upper: deviation,
        position_tolerance: limits.position_tolerance,
    });
    Ok(out)
}
