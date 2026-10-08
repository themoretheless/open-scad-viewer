//! Progressive profile transport with simultaneous scale and angular twist.
//!
//! Each level is a retained piecewise-linear NURBS patch set. Refinement compares
//! profile controls against fourfold finer transport. Open parameter-spaced
//! authored frames also carry original-law continuous retained-patch error
//! certificates. Other transport, global injectivity and capped-body admission
//! remain separate obligations; sampled acceptance is not their proof.
use crate::{Result, check, core::vec3_ext::norm, curve::Curve, surface::Surface};
use math_core::{cross, dot, sub};
use std::f64::consts::TAU;

type V = [f64; 3];
// Constructor station reuse is separate from certificate work accounting.
// Keys retain original curve identity, station count and exact work policy.
type StationCache = std::sync::Arc<std::sync::Mutex<std::collections::BTreeMap<
    (usize, usize, usize, u64), (Vec<f64>, Option<f64>)>>>;
mod authored_error;
mod arc_guide;
mod contact_anchor;
mod source_plane;
mod source_line;
mod retained_regularity;
mod retained_smoothness;
mod original_smoothness;
mod endpoint_jets;
mod rmf_transport;
pub use rmf_transport::{OriginalRmfTransportReport,OriginalRmfSectionImagesReport,RmfNormalCell,certify_original_rmf_transport,certify_original_rmf_section_images};
pub use original_smoothness::{OriginalFrameSmoothnessReport,ClosedAuthoredFrameSmoothnessReport,ClosedPathFrameSmoothnessReport};
pub use retained_smoothness::{RetainedSeamReport,RetainedSmoothnessReport,RetainedDecompositionReport,RetainedDecompositionSmoothnessReport};
mod profile_domain;
pub use profile_domain::{IdealEndpointDomainsReport,IdealEndpointPlanesReport,EndpointCapProjectionReport};
pub use retained_regularity::{RetainedRegularityReport, inspect as inspect_retained_regularity};
pub use contact_anchor::{ContactAnchorReport, ContactFitReport};
pub use authored_error::{InitialCoordinatesReport, SectionInterpolationReport, PatchErrorReport};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    RotationMinimizing,
    /// Preserve the initial world orientation; twist uses the initial tangent.
    Fixed,
    /// Project the authored normal perpendicular to each path tangent.
    FixedNormal,
    /// Principal normal. Zero curvature and undefined second derivatives fail.
    Frenet,
    /// Sign-continuous sampled principal normal; RMF transport at zero curvature.
    CorrectedFrenet,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Spacing {
    Parameter,
    /// Certified inverse arc-length residual, in model length units.
    ArcLength {
        tolerance: f64,
        max_cells: usize,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub normal: V,
    pub orientation: Orientation,
    pub spacing: Spacing,
    pub initial_sections: usize,
    pub max_sections: usize,
    pub max_deviation: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LevelReport {
    pub accepted: bool,
    pub sections: usize,
    pub stations: usize,
    pub sampled_control_deviation: f64,
    pub budget: f64,
    pub closed_path: bool,
    pub length_residual_upper: Option<f64>,
    pub continuous_error_upper: Option<f64>,
    /// Bounds both original end-section contours, before decomposition/correction.
    pub original_section_endpoint_error_upper: Option<f64>,
    /// Retained end contours, including decomposition, before correction/caps.
    pub endpoint_contour_error_upper: Option<[f64;2]>,
    /// Maximum among certified profiles, even when the complete union is unresolved.
    /// This controls conservative admission but is not a bound for every profile.
    pub known_profile_error_upper: Option<f64>,
    pub error_certificate_cells: usize,
    pub decomposition_products: usize,
    pub error_certificate_reason: Option<&'static str>,
    pub continuous_bound: bool,
}

#[derive(Clone, Debug)]
pub struct Level {
    /// Preview is always available, even when this level misses the budget.
    pub patches: Vec<Surface>,
    pub report: LevelReport,
}

#[derive(Clone, Debug)]
pub struct Approximation {
    /// Only an accepted final level is promoted to a construction result.
    pub patches: Option<Vec<Surface>>,
    pub levels: Vec<LevelReport>,
}

/// Caller-controlled progression: consume one level for preview, stop or resume.
/// Scale is positive [value,0,0]; twist is signed [radians,0,0]. Each law domain
/// independently maps to normalized traversal (parameter or arc length).
pub struct Sweep<'a> {
    station_cache: StationCache,
    profile: &'a Curve,
    path: &'a Curve,
    scale: &'a Curve,
    twist: &'a Curve,
    affine_laws: Option<(&'a Curve, &'a Curve)>,
    frame_laws: Option<(&'a Curve, &'a Curve)>,
    orientation_guide: Option<&'a Curve>,
    contact_point: Option<V>,
    /// Original anchor ownership, including the reference contour in MultiSweep.
    contact_source: Option<(&'a Curve, f64)>,
    spatial_rmf_error_limits: Option<[usize;3]>,
    options: Options,
    next_sections: Option<usize>,
}

fn unit(a: V) -> Result<V> {
    let largest = a.iter().fold(0_f64, |m, x| m.max(x.abs()));
    check(
        largest.is_finite() && largest > 0.,
        "Sweep direction must be finite and nonzero",
    )?;
    let scaled = a.map(|x| x / largest);
    let length = scaled[0].hypot(scaled[1]).hypot(scaled[2]);
    Ok(scaled.map(|x| x / length))
}
fn project(normal: V, tangent: V) -> Result<V> {
    let n = unit(normal)?;
    let transverse = sub(n, tangent.map(|x| x * dot(n, tangent)));
    check(
        norm(transverse) > 1e-12,
        "Sweep normal is parallel or numerically inseparable from tangent",
    )?;
    unit(transverse)
}
fn reflect(a: V, direction: V) -> Result<V> {
    let n = unit(direction)?;
    Ok(sub(a, n.map(|x| 2. * dot(a, n) * x)))
}
fn transported_normal(normal: V, previous_tangent: V, tangent: V, chord: V) -> Result<V> {
    let r = reflect(normal, chord)?;
    let t = reflect(previous_tangent, chord)?;
    let difference = sub(tangent, t);
    let r = if norm(difference) <= 1e-14 { r } else { reflect(r, difference)? };
    project(r, tangent)
}
fn rotate(a: V, axis: V, angle: f64) -> V {
    let (s, c) = angle.sin_cos();
    let perpendicular = cross(axis, a);
    let parallel = dot(axis, a) * (1. - c);
    std::array::from_fn(|k| c * a[k] + s * perpendicular[k] + parallel * axis[k])
}
fn law(curve: &Curve, fraction: f64) -> Result<f64> {
    let [a, b] = curve.domain();
    Ok(curve
        .evaluate(if fraction == 1. {
            b
        } else {
            a + (b - a) * fraction
        })?
        .point[0])
}
/// A retained constant 3-vector law without a degenerate geometric line.
pub fn constant_vector_law(value: V) -> Result<Curve> {
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![value.to_vec(); 2],
        weights: vec![1., 1.],
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}

fn vector_law(curve: &Curve, fraction: f64) -> Result<V> {
    let [a, b] = curve.domain();
    // At clamped endpoints the real rational point is the authored control,
    // independent of its weight; avoid a spurious rounding-induced seam gap.
    if fraction == 0. && curve.knots[..=curve.degree].iter().all(|k| *k == a) {
        let p = &curve.control_points[0];
        return Ok([p[0], p[1], p[2]]);
    }
    if fraction == 1.
        && curve.knots[curve.control_points.len()..]
            .iter()
            .all(|k| *k == b)
    {
        let p = curve.control_points.last().unwrap();
        return Ok([p[0], p[1], p[2]]);
    }
    let p = curve
        .evaluate(if fraction == 1. {
            b
        } else {
            a + (b - a) * fraction
        })?
        .point;
    Ok([p[0], p[1], p[2]])
}

fn validate_law(curve: &Curve, positive: bool) -> Result<()> {
    curve.validate()?;
    check(
        curve
            .control_points
            .iter()
            .all(|p| p.len() == 3 && p[1] == 0. && p[2] == 0. && (!positive || p[0] > 0.)),
        "Sweep laws require [value,0,0] controls; scale must be positive",
    )
}
fn sample(path: &Curve, t: f64) -> Result<(V, V)> {
    let e = path.evaluate(t)?;
    let derivative = match e.d1 {
        Some(d) => d,
        None => {
            let [a, b] = path.domain();
            let side = if t > a {
                path.trim(a, t)?
            } else {
                path.trim(t, b)?
            };
            side.evaluate(t)?
                .d1
                .ok_or_else(|| crate::input("Sweep path has no unique regular tangent"))?
        }
    };
    Ok((
        [e.point[0], e.point[1], e.point[2]],
        unit([derivative[0], derivative[1], derivative[2]])?,
    ))
}

impl<'a> Sweep<'a> {
    pub fn new(
        profile: &'a Curve,
        path: &'a Curve,
        scale: &'a Curve,
        twist: &'a Curve,
        options: Options,
    ) -> Result<Self> {
        profile.validate()?;
        path.validate()?;
        validate_law(scale, true)?;
        validate_law(twist, false)?;
        check(
            profile.control_points[0].len() == 3 && path.control_points[0].len() == 3,
            "Progressive sweep requires 3D profile and path",
        )?;
        check(
            (2..=1025).contains(&options.initial_sections)
                && options.initial_sections <= options.max_sections
                && options.max_sections <= 1025,
            "Progressive sweep needs 2..1025 sections with initial <= maximum",
        )?;
        check(
            options.max_deviation.is_finite() && options.max_deviation > 0.,
            "Progressive sweep deviation must be positive and finite",
        )?;
        unit(options.normal)?;
        if let Spacing::ArcLength {
            tolerance,
            max_cells,
        } = options.spacing
        {
            check(
                tolerance.is_finite() && tolerance > 0. && (1..=100_000).contains(&max_cells),
                "Sweep arc-length tolerance/work budget is invalid",
            )?;
            // Fourfold station inversion uses the curve-measure 1024-segment limit.
            check(
                options.max_sections <= 257,
                "Arc-length sweep supports at most 257 sections",
            )?;
        }
        let [a, b] = path.domain();
        for &k in &path.knots {
            if options.orientation != Orientation::Fixed
                && k > a
                && k < b
                && path.knots.iter().filter(|&&v| v == k).count() >= path.degree
            {
                let left = sample(&path.trim(a, k)?, k)?.1;
                let right = sample(&path.trim(k, b)?, k)?.1;
                check(
                    norm(sub(left, right)) < 1e-10,
                    "Split the sweep path at tangent discontinuities",
                )?;
            }
        }
        Ok(Self {
            station_cache: Default::default(),
            profile,
            path,
            scale,
            twist,
            affine_laws: None,
            frame_laws: None,
            orientation_guide: None,
            contact_point: None,
            contact_source: None,
            spatial_rmf_error_limits: None,
            options,
            next_sections: Some(options.initial_sections),
        })
    }

    /// Orient the normal toward a second spatial rail, projected into the
    /// guide-tangent normal plane. Correspondence uses normalized traversal:
    /// parameter fraction, or independent arc-length fraction on both rails.
    /// This controls orientation only; it does not scale a profile onto the rail.
    pub fn with_orientation_guide(mut self, guide: &'a Curve) -> Result<Self> {
        guide.validate()?;
        check(
            guide.control_points[0].len() == 3,
            "Sweep orientation guide must be 3D",
        )?;
        check(
            self.frame_laws.is_none() && self.options.orientation != Orientation::Fixed,
            "Orientation guide cannot be combined with fixed or authored frames",
        )?;
        self.orientation_guide = Some(guide);
        self.contact_point = None;
        self.contact_source = None;
        if let Ok(mut cache) = self.station_cache.lock() { cache.clear(); }
        self.next_sections = Some(self.options.initial_sections);
        Ok(self)
    }

    /// Fit the profile transverse extent so its selected point follows the rail
    /// at retained stations. Initial anchor must coincide with the initial rail,
    /// and corresponding rail points must lie in the main tangent normal plane.
    /// Contact fixes normal orientation, so nonzero twist is incompatible.
    /// Only the transverse coordinate is fitted; other scale axes are retained.
    /// This is sampled contact, not a continuous interpolated-rail certificate.
    pub fn with_contact_guide(self, guide: &'a Curve, profile_parameter: f64) -> Result<Self> {
        let [a, b] = self.profile.domain();
        check(
            profile_parameter.is_finite() && profile_parameter >= a && profile_parameter <= b,
            "Contact anchor parameter is outside profile domain",
        )?;
        let p = self.profile.evaluate(profile_parameter)?.point;
        let profile = self.profile;
        let mut result = self.with_contact_point(guide, [p[0], p[1], p[2]])?;
        result.contact_source = Some((profile, profile_parameter));
        Ok(result)
    }

    fn with_contact_point(self, guide: &'a Curve, point: V) -> Result<Self> {
        check(
            self.twist.control_points.iter().all(|p| p[0] == 0.),
            "Contact guide fixes orientation and requires zero twist",
        )?;
        let mut result = self.with_orientation_guide(guide)?;
        result.contact_point = Some(point);
        Ok(result)
    }

    /// Complete authored orientation: normalize the longitudinal axis and project
    /// the transverse direction perpendicular to it; the third axis is their
    /// right-handed cross product. Both rational vector-law domains map to the
    /// normalized traversal. Unlike RMF these axes need not follow the guide tangent.
    /// Zero/parallel evaluated directions refuse; sampling is not a continuous
    /// regularity certificate. Twist rotates around the authored longitudinal axis.
    pub fn new_authored(
        profile: &'a Curve,
        path: &'a Curve,
        scale: &'a Curve,
        twist: &'a Curve,
        longitudinal: &'a Curve,
        transverse: &'a Curve,
        mut options: Options,
    ) -> Result<Self> {
        options.orientation = Orientation::Fixed;
        Self::new(profile, path, scale, twist, options)?.with_frame_laws(longitudinal, transverse)
    }

    /// Replace orientation and restart refinement; applies to a Fixed sweep.
    /// Use new_authored when constructing a sweep with these laws directly.
    pub fn with_frame_laws(
        mut self,
        longitudinal: &'a Curve,
        transverse: &'a Curve,
    ) -> Result<Self> {
        check(
            self.options.orientation == Orientation::Fixed,
            "Authored frame laws require Fixed orientation or new_authored",
        )?;
        for curve in [longitudinal, transverse] {
            curve.validate()?;
            check(
                curve.control_points[0].len() == 3,
                "Authored frame laws require three coordinates",
            )?;
        }
        check(
            self.orientation_guide.is_none(),
            "Authored frames cannot be combined with an orientation guide",
        )?;
        self.frame_laws = Some((longitudinal, transverse));
        if let Ok(mut cache) = self.station_cache.lock() { cache.clear(); }
        self.next_sections = Some(self.options.initial_sections);
        Ok(self)
    }

    fn authored_frame(&self, fraction: f64) -> Result<Option<(V, V)>> {
        self.frame_laws
            .map(|(longitudinal, transverse)| {
                let axis = unit(vector_law(longitudinal, fraction)?)?;
                Ok((axis, project(vector_law(transverse, fraction)?, axis)?))
            })
            .transpose()
    }

    /// Certify authored direction regularity over the entire normalized law
    /// traversal, independently of the preview station grid. No report is
    /// returned for transport modes without authored direction laws. This is
    /// one premise of a continuous sweep bound, not that complete bound.
    pub fn authored_frame_regularity(
        &self,
        max_cells: usize,
    ) -> Result<Option<super::progressive_miter::authored_frame_certificate::RegularityReport>> {
        self.frame_laws
            .map(|(axis, normal)| {
                super::progressive_miter::authored_frame_certificate::certify_regularity(
                    axis, normal, max_cells,
                )
            })
            .transpose()
    }

    /// Full twisted-frame jets for later retained-wall error composition.
    /// Publishing this premise does not certify the complete sweep boundary.
    pub fn authored_frame_jet_cover(
        &self,
        max_cells: usize,
    ) -> Result<Option<super::progressive_miter::authored_frame_certificate::RegularityReport>> {
        self.frame_laws
            .map(|(axis, normal)| {
                super::progressive_miter::authored_frame_certificate::certify_twisted_cover(
                    axis, normal, self.twist, max_cells,
                )
            })
            .transpose()
    }

    /// Original-law initial profile coordinates including outward arithmetic.
    /// A separate premise, not a retained-wall or complete boundary bound.
    pub fn authored_initial_coordinates(&self, max_cells: usize) -> Result<InitialCoordinatesReport> {
        authored_error::initial_coordinates(self, max_cells)
    }

    pub fn fixed_initial_coordinates(&self,max_cells:usize)->Result<InitialCoordinatesReport>{
        authored_error::fixed_initial_coordinates(self,max_cells)
    }
    pub fn fixed_patch_error_bound(&self,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
        authored_error::fixed_patch_error(self,count,max_cells,max_products)
    }
    /// Original straight-axis RMF transport only; general RMF remains unresolved.
    pub fn rmf_straight_patch_error_bound(&self,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
        authored_error::rmf_straight_patch_error(self,count,max_cells,max_products)
    }
    /// Explicit original spatial RMF integration resolution and shared work.
    /// This proves retained patch error only; global/cap/Solid obligations stay separate.
    pub fn with_spatial_rmf_error_limits(mut self,steps:usize,max_cells:usize,max_products:usize)->Result<Self>{
        check(steps.is_power_of_two() && (2..=16384).contains(&steps)
            && max_cells<=100000 && max_products<=1000000,
            "Invalid spatial RMF proof limits")?;
        check(self.options.orientation==Orientation::RotationMinimizing && self.frame_laws.is_none()
            && self.orientation_guide.is_none() && self.contact_point.is_none(),
            "Spatial RMF proof limits require an unguided RMF source")?;
        self.spatial_rmf_error_limits=Some([steps,max_cells,max_products]);Ok(self)
    }
    pub fn rmf_spatial_patch_error_bound(&self,count:usize,transport_steps:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
        authored_error::rmf_spatial_patch_error(self,count,transport_steps,max_cells,max_products)
    }
    pub fn rmf_planar_patch_error_bound(&self,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
        authored_error::rmf_planar_patch_error(self,count,max_cells,max_products)
    }
    pub fn fixed_normal_patch_error_bound(&self,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
        authored_error::fixed_normal_patch_error(self,count,max_cells,max_products)
    }
    pub fn frenet_patch_error_bound(&self,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
        authored_error::frenet_patch_error(self,count,max_cells,max_products)
    }

    /// Guided initial basis before twist/affine laws; not a contact/error proof.
    pub fn guided_initial_coordinates(&self,max_cells:usize)->Result<InitialCoordinatesReport> {
        authored_error::guided_initial_coordinates(self,max_cells)
    }

    /// Original selected profile point in the initial guided basis.
    /// This premise does not establish continuous rail contact or retained error.
    pub fn contact_anchor_bound(&self, max_cells: usize) -> Result<ContactAnchorReport> {
        contact_anchor::certify(self, max_cells)
    }

    /// Constructor-owned contact-width quotient jets; not retained geometry E.
    pub fn contact_fit_jet(&self, traversal: [f64; 2], max_cells: usize) -> Result<ContactFitReport> {
        contact_anchor::fit(self, traversal, max_cells)
    }

    /// Original contact control jets with constructor-owned coordinates/anchor.
    /// This does not certify retained interpolation or continuous rail contact.
    pub fn contact_control_trajectory(&self, control: usize, traversal: [f64; 2], max_cells: usize) -> Result<super::progressive_miter::authored_frame_certificate::TrajectoryReport> {
        contact_anchor::control_trajectory(self, control, traversal, max_cells)
    }

    /// Original contact control value, including point-safe station evaluation.
    pub fn contact_control_value(&self, control: usize, traversal: [f64; 2], max_cells: usize) -> Result<super::progressive_miter::authored_frame_certificate::ControlValueReport> {
        contact_anchor::control_value(self, control, traversal, max_cells)
    }

    /// Original guided control image; contact/closed/arc-length proofs remain separate.
    pub fn guided_control_value(&self,profile_control:usize,traversal:[f64;2],max_cells:usize)->Result<super::progressive_miter::authored_frame_certificate::ControlValueReport> {
        authored_error::guided_control_value(self,profile_control,traversal,max_cells)
    }

    /// Constructor-owned ideal control trajectory, including initial-coordinate
    /// enclosures. Retained interpolation and endpoint displacement are separate.
    pub fn authored_control_trajectory(
        &self,
        profile_control: usize,
        traversal: [f64; 2],
        max_cells: usize,
    ) -> Result<super::progressive_miter::authored_frame_certificate::TrajectoryReport> {
        authored_error::control_trajectory(self, profile_control, traversal, max_cells)
    }

    /// Complete original-profile section interpolation bound before retained
    /// profile decomposition. This is not the complete patch/boundary bound.
    pub fn authored_section_interpolation_bound(&self, count: usize, max_cells: usize) -> Result<SectionInterpolationReport> {
        authored_error::section_interpolation(self, count, max_cells)
    }
    /// Whole original-profile guided interpolation, before decomposition/caps.
    pub fn guided_section_interpolation_bound(&self,count:usize,max_cells:usize)->Result<SectionInterpolationReport> {
        authored_error::guided_section_interpolation(self,count,max_cells)
    }
    /// Contact fitted original-profile interpolation; caps/contact identity separate.
    pub fn contact_section_interpolation_bound(&self,count:usize,max_cells:usize)->Result<SectionInterpolationReport> {
        authored_error::contact_section_interpolation(self,count,max_cells)
    }
    pub fn authored_patch_error_bound(&self, count: usize, max_cells: usize, max_products: usize) -> Result<PatchErrorReport> {
        authored_error::patch_error(self,count,max_cells,max_products)
    }

    /// Guided retained-patch bound including source-profile decomposition.
    pub fn guided_patch_error_bound(&self,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport> {
        authored_error::guided_patch_error(self,count,max_cells,max_products)
    }
    /// Contact fitted retained patches including original profile decomposition.
    pub fn contact_patch_error_bound(&self,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport> {
        authored_error::contact_patch_error(self,count,max_cells,max_products)
    }

    /// Axis scale is a positive dimensionless 3-vector law; center is a local
    /// frame offset in model units. q' = uniform_scale * axis_scale * q + center,
    /// then twist/frame transport. Laws use normalized traversal independently.
    pub fn with_affine_laws(mut self, axis_scale: &'a Curve, center: &'a Curve) -> Result<Self> {
        for curve in [axis_scale, center] {
            curve.validate()?;
            check(
                curve.control_points[0].len() == 3,
                "Affine sweep laws require three coordinates",
            )?;
        }
        check(
            axis_scale.control_points.iter().flatten().all(|x| *x > 0.),
            "Every axis-scale control must be positive",
        )?;
        // Equal rational poles define the exact identity for every parameter,
        // regardless of knots or positive weights. Keep generated default laws
        // from consuming certificate work for an absent affine transform.
        let identity=axis_scale.control_points.iter().all(|p|p.iter().all(|x|*x==1.))
            && center.control_points.iter().all(|p|p.iter().all(|x|*x==0.));
        self.affine_laws = if identity {None} else {Some((axis_scale, center))};
        if let Ok(mut cache) = self.station_cache.lock() { cache.clear(); }
        self.next_sections = Some(self.options.initial_sections);
        Ok(self)
    }

    fn parameters(&self, count: usize) -> Result<(Vec<f64>, Option<f64>)> {
        if let (Some(guide),Some(_),Spacing::ArcLength {tolerance,max_cells})=(self.orientation_guide,self.contact_point,self.options.spacing) {
            let phase=arc_guide::certify(self.path,guide,max_cells)?;
            let tolerance=phase.scale_upper.filter(|scale|*scale>1.).map_or(tolerance,|scale|(tolerance/scale).next_down());
            return self.curve_parameters_with_budget(self.path,count,Some(max_cells-phase.cells),Some(tolerance));
        }
        self.curve_parameters(self.path, count)
    }

    fn curve_parameters(&self, curve: &Curve, count: usize) -> Result<(Vec<f64>, Option<f64>)> {
        self.curve_parameters_with_budget(curve,count,None,None)
    }

    fn curve_parameters_with_budget(&self,curve:&Curve,count:usize,remaining:Option<usize>,tolerance_override:Option<f64>)->Result<(Vec<f64>,Option<f64>)> {
        let [a, b] = curve.domain();
        match self.options.spacing {
            Spacing::Parameter => Ok((
                (0..count)
                    .map(|i| {
                        if i + 1 == count {
                            b
                        } else {
                            a + (b - a) * i as f64 / (count - 1) as f64
                        }
                    })
                    .collect(),
                None,
            )),
            Spacing::ArcLength {
                tolerance,
                max_cells,
            } => {
                let tolerance = tolerance_override.unwrap_or(tolerance);
                let max_cells = remaining.map_or(max_cells, |r| r.min(max_cells));
                let key = (curve as *const Curve as usize, count, max_cells, tolerance.to_bits());
                if let Ok(cache) = self.station_cache.lock() {
                    if let Some(stations) = cache.get(&key) { return Ok(stations.clone()); }
                }
                let division =
                    crate::curve_measure::divide_by_length(curve, count - 1, tolerance, max_cells)?;
                check(
                    division.within_tolerance && division.points.len() == count,
                    &format!(
                        "Sweep inverse arc-length budget was not met: {:?}, {} cells, {}/{} stations",
                        division.stop_reason,
                        division.cells,
                        division.points.len(),
                        count
                    ),
                )?;
                let residual = division
                    .points
                    .iter()
                    .map(|p| p.residual_upper)
                    .fold(0_f64, f64::max);
                let stations = (division.points.into_iter().map(|p| p.parameter).collect(), Some(residual));
                if let Ok(mut cache) = self.station_cache.lock() {
                    if cache.len() < 32 { cache.insert(key, stations.clone()); }
                }
                Ok(stations)
            }
        }
    }

    fn sections(&self, count: usize) -> Result<(Vec<Curve>, bool, Option<f64>)> {
        let (curves,closed,residual,_)=self.sections_with_frame_identity(count)?;
        Ok((curves,closed,residual))
    }

    /// Identity evidence belongs to the normals/correction actually used by
    /// this constructor, before twisting and the retained copied seam.
    fn sections_with_frame_identity(&self,count:usize)->Result<(Vec<Curve>,bool,Option<f64>,bool)> {
        let (parameters, mut residual) = self.parameters(count)?;
        let samples = parameters
            .iter()
            .map(|&t| sample(self.path, t))
            .collect::<Result<Vec<_>>>()?;
        let (start, t0) = samples[0];
        let (end, t1) = samples[count - 1];
        // On an exactly proved source plane the Bishop basis is B, B x T.
        // Carry its coefficients directly: a zero second reflection at an
        // antiparallel endpoint step must not reverse the binormal. This is
        // constructor transport only, not retained-error/smoothness evidence.
        let corrected_plane = if self.options.orientation == Orientation::CorrectedFrenet
            && self.frame_laws.is_none() && self.orientation_guide.is_none()
            && (authored_error::original_planar_rmf(self.path,self.options.normal)
                || source_plane::certify(self.path,self.options.normal,1000000)?.proved)
        { Some(unit(self.options.normal)?) } else { None };
        let corrected_transport = |normal, previous_tangent, tangent, chord| {
            if let Some(binormal) = corrected_plane {
                let previous_in_plane = cross(binormal,previous_tangent);
                let in_plane = cross(binormal,tangent);
                let a = dot(normal,previous_in_plane);
                let b = dot(normal,binormal);
                project(std::array::from_fn(|k|a*in_plane[k]+b*binormal[k]),tangent)
            } else { transported_normal(normal,previous_tangent,tangent,chord) }
        };
        let closed = closed_extent(self.path, start, end)?;
        check(
            !self.path.periodic || closed,
            "Periodic sweep endpoints must coincide",
        )?;
        if closed {
            check(
                count >= 4 && self.options.initial_sections >= 4,
                "Closed progressive sweep needs at least four sections",
            )?;
            check(
                self.options.orientation == Orientation::Fixed || norm(sub(t0, t1)) < 1e-10,
                "Closed sweep tangent endpoints must agree",
            )?;
            check(
                law(self.scale, 0.)? == law(self.scale, 1.)?,
                "Closed sweep scale endpoints must agree",
            )?;
            if let Some((axis_scale, center)) = self.affine_laws {
                for curve in [axis_scale, center] {
                    check(
                        vector_law(curve, 0.)? == vector_law(curve, 1.)?,
                        "Closed sweep axis scale and center endpoints must agree",
                    )?;
                }
            }
            let twist = law(self.twist, 1.)? - law(self.twist, 0.)?;
            check(
                twist.is_finite() && (twist / TAU - (twist / TAU).round()).abs() <= 1e-12,
                "Closed sweep twist endpoints must differ by whole turns",
            )?;
        }
        let principal_normal = |i: usize| -> Result<Option<V>> {
            let e = self.path.evaluate(parameters[i])?;
            let Some(d2) = e.d2 else {
                // C1 rational joins may lack a common second derivative while
                // their principal direction agrees. Require a tight original
                // one-sided frame enclosure; never infer it from nearby samples.
                if !matches!(e.continuity,Some(0|1)) {return Ok(None);}
                let [a,b]=self.path.domain();let f=(parameters[i]-a)/(b-a);
                let zero=constant_vector_law([0.;3])?;
                let frame=crate::sweeps::progressive_miter::authored_frame_certificate::certify_frenet_path_values(
                    self.path,&zero,[f,f],10000,
                )?;
                let Some(n)=frame.transverse else {return Ok(None);};
                let Some(t)=frame.longitudinal else {return Ok(None);};
                let diameter=norm(n.map(|v|v[1]-v[0]));
                let tangent_diameter=norm(t.map(|v|v[1]-v[0]));
                if !diameter.is_finite() || diameter>1e-10 || !tangent_diameter.is_finite() || tangent_diameter>1e-10 {return Ok(None);}
                let midpoint=n.map(|v|v[0]+(v[1]-v[0])*0.5);
                return unit(midpoint).map(Some);
            };
            let acceleration = [d2[0], d2[1], d2[2]];
            check(acceleration.iter().all(|x| x.is_finite()), "Sweep curvature must be finite")?;
            if acceleration.iter().all(|x| *x == 0.) { return Ok(None); }
            let n = unit(acceleration)?;
            let tangent = samples[i].1;
            let transverse = sub(n, tangent.map(|x| x * dot(n, tangent)));
            if norm(transverse) <= 1e-12 { Ok(None) } else { unit(transverse).map(Some) }
        };
        let frenet = |i: usize| -> Result<V> {
            principal_normal(i)?.ok_or_else(|| crate::input("Frenet sweep needs a unique nonzero principal normal"))
        };
        let mut contact_widths = Vec::new();
        let guide_normals = if let Some(guide) = self.orientation_guide {
            let (guide_parameters, guide_residual) = if let Spacing::ArcLength {tolerance,max_cells}=self.options.spacing {
                if self.contact_point.is_some() {
                    let phase=arc_guide::certify(self.path,guide,max_cells)?;
                    let shared_residual=phase.scale_upper.zip(residual).map(|(scale,r)|(scale*r).next_up());
                    if let Some(r)=shared_residual.filter(|r|*r<=tolerance) {
                        (parameters.clone(),Some(r))
                    }else{
                        self.curve_parameters_with_budget(guide,count,Some(max_cells-phase.cells),None)?
                    }
                }else{self.curve_parameters(guide,count)?}
            }else{self.curve_parameters(guide,count)?};
            residual = match (residual, guide_residual) {
                (Some(a), Some(b)) => Some(a.max(b)),
                (a, b) => a.or(b),
            };
            let normals = guide_parameters
                .iter()
                .zip(&samples)
                .map(|(&t, &(position, tangent))| {
                    let p = guide.evaluate(t)?.point;
                    let offset = sub([p[0], p[1], p[2]], position);
                    if self.contact_point.is_some() {
                        let width = norm(offset);
                        check(
                            width.is_finite()
                                && width > 0.
                                && dot(offset, tangent).abs() <= 1e-10 * width,
                            "Contact rail must lie in the path normal plane",
                        )?;
                        contact_widths.push(width);
                    }
                    project(offset, tangent)
                })
                .collect::<Result<Vec<_>>>()?;
            if closed && self.contact_point.is_some() {
                check(
                    (contact_widths[0] - contact_widths[count - 1]).abs()
                        <= 1e-10 * contact_widths[0],
                    "Closed contact rail widths must agree",
                )?;
            }
            if closed {
                check(
                    norm(sub(normals[0], *normals.last().unwrap())) < 1e-10,
                    "Closed sweep orientation-guide endpoints must agree",
                )?;
            }
            Some(normals)
        } else {
            None
        };
        let initial_frame = self.authored_frame(0.)?;
        let coordinate_axis = initial_frame.map_or(t0, |f| f.0);
        let initial = if let Some(normals) = &guide_normals {
            normals[0]
        } else if let Some((_, normal)) = initial_frame {
            normal
        } else if self.options.orientation == Orientation::Frenet {
            frenet(0)?
        } else if self.options.orientation == Orientation::CorrectedFrenet {
            let source_initial = if !closed && principal_normal(0)?.is_none() {
                if let Some(plane)=corrected_plane {
                    let (_,phase)=crate::sweeps::progressive_miter::authored_frame_certificate::certify_initial_planar_principal_phase(
                        self.path,self.options.normal,10000,
                    )?;
                    phase.map(|phase|cross(t0,plane).map(|x|x*phase)).map(|normal|project(normal,t0)).transpose()?
                }else{None}
            }else{None};
            if let Some(normal)=source_initial {normal}else{
            let mut first = None;
            for i in 0..count {
                if let Some(normal) = principal_normal(i)? {
                    first = Some((i, normal));
                    break;
                }
            }
            if let Some((index, mut normal)) = first {
                // Resolve an initially straight interval from the first available
                // principal normal rather than introducing a seed-normal jump.
                for i in (1..=index).rev() {
                    normal = corrected_transport(normal, samples[i].1, samples[i - 1].1,
                        sub(samples[i - 1].0, samples[i].0))?;
                }
                normal
            } else { project(self.options.normal, t0)? }
            }
        } else {
            project(self.options.normal, t0)?
        };
        let side = cross(coordinate_axis, initial);
        let contact_anchor = if let Some(p) = self.contact_point {
            let q = sub([p[0], p[1], p[2]], start);
            let width = dot(q, initial);
            check(
                width > 0.
                    && norm(sub(q, initial.map(|v| v * contact_widths[0])))
                        <= 1e-10 * contact_widths[0],
                "Initial contact anchor must coincide with orientation rail",
            )?;
            Some(width)
        } else {
            None
        };
        let coordinates: Vec<V> = self
            .profile
            .control_points
            .iter()
            .map(|p| {
                let q = sub([p[0], p[1], p[2]], start);
                [dot(q, initial), dot(q, side), dot(q, coordinate_axis)]
            })
            .collect();
        let mut normals = vec![initial];
        let mut lengths = vec![0.];
        for i in 1..count {
            let (previous, previous_tangent) = samples[i - 1];
            let (position, tangent) = samples[i];
            let chord = sub(position, previous);
            lengths.push(lengths[i - 1] + norm(chord));
            let next = if let Some(normals) = &guide_normals {
                normals[i]
            } else if let Some((_, normal)) = self.authored_frame(i as f64 / (count - 1) as f64)? {
                normal
            } else {
                match self.options.orientation {
                    Orientation::Fixed => initial,
                    Orientation::FixedNormal => project(self.options.normal, tangent)?,
                    Orientation::Frenet => frenet(i)?,
                    Orientation::CorrectedFrenet => {
                        let transported = corrected_transport(normals[i - 1], previous_tangent, tangent, chord)?;
                        match principal_normal(i)? {
                            Some(n) => if dot(n, transported) < 0. { n.map(|x| -x) } else { n },
                            None => transported,
                        }
                    }
                    Orientation::RotationMinimizing => transported_normal(normals[i - 1], previous_tangent, tangent, chord)?,
                }
            };
            normals.push(next);
        }
        let correction = if closed
            && self.orientation_guide.is_none()
            && self.options.orientation == Orientation::RotationMinimizing
        {
            dot(t0, cross(normals[count - 1], initial)).atan2(dot(normals[count - 1], initial))
        } else {
            0.
        };
        let closed_planar_identity=if closed
            && self.options.orientation==Orientation::RotationMinimizing
            && self.frame_laws.is_none() && self.orientation_guide.is_none()
            && authored_error::original_planar_rmf(self.path,self.options.normal) {
            let axis=(0..3).find(|&k|self.options.normal[k]!=0.).unwrap();
            let mut expected=[0.;3];
            expected[axis]=if self.options.normal[axis]>0. {1.}else {-1.};
            // Both reflections act in the source plane and preserve this
            // exact axial normal. Check the actual numerical realization,
            // including every tangent/normal and the computed holonomy.
            correction==0. && normals.iter().all(|&n|n==expected)
                && samples.iter().all(|(p,t)|p[axis]==start[axis]&&t[axis]==0.)
        }else {false};
        if closed
            && matches!(
                self.options.orientation,
                Orientation::Frenet | Orientation::CorrectedFrenet | Orientation::FixedNormal
            )
        {
            check(
                norm(sub(normals[count - 1], initial)) < 1e-10,
                "Closed sweep orientation endpoints must agree",
            )?;
        }
        if closed && self.frame_laws.is_some() {
            let first = self.authored_frame(0.)?.unwrap();
            let last = self.authored_frame(1.)?.unwrap();
            check(
                norm(sub(first.0, last.0)) < 1e-10 && norm(sub(first.1, last.1)) < 1e-10,
                "Closed sweep authored frame endpoints must agree",
            )?;
        }
        let total = lengths[count - 1];
        check(
            total.is_finite() && total > 0.,
            "Sweep path traversal has zero or nonfinite sampled length",
        )?;
        let mut curves = Vec::with_capacity(count);
        for i in 0..count {
            let fraction = i as f64 / (count - 1) as f64;
            let scale = law(self.scale, fraction)?;
            let twist = law(self.twist, fraction)?;
            check(
                scale.is_finite() && scale > 0. && twist.is_finite(),
                "Sweep law evaluation is invalid",
            )?;
            let (axes, center) = if let Some((axes, center)) = self.affine_laws {
                (vector_law(axes, fraction)?, vector_law(center, fraction)?)
            } else {
                ([1.; 3], [0.; 3])
            };
            check(
                axes.iter().all(|x| x.is_finite() && *x > 0.)
                    && center.iter().all(|x| x.is_finite()),
                "Affine sweep law evaluation is invalid",
            )?;
            let contact_fit = if let Some(anchor) = contact_anchor {
                check(
                    center[1] == 0. && center[2] == 0.,
                    "Contact guide requires zero side/longitudinal center offset",
                )?;
                let transformed = scale * axes[0] * anchor + center[0];
                check(
                    transformed.is_finite() && transformed > 0.,
                    "Contact anchor scale must remain positive",
                )?;
                let fit = contact_widths[i] / transformed;
                check(fit.is_finite() && fit > 0., "Contact width fit overflowed")?;
                fit
            } else {
                1.
            };
            let tangent = if let Some((axis, _)) = self.authored_frame(fraction)? {
                axis
            } else if self.options.orientation == Orientation::Fixed {
                t0
            } else {
                samples[i].1
            };
            let normal = rotate(normals[i], tangent, twist + correction * lengths[i] / total);
            let side = cross(tangent, normal);
            let mut curve = self.profile.clone();
            curve.control_points = coordinates
                .iter()
                .map(|q| {
                    (0..3)
                        .map(|k| {
                            if self.affine_laws.is_none() && contact_anchor.is_none() {
                                return samples[i].0[k]
                                    + scale
                                        * (q[0] * normal[k] + q[1] * side[k] + q[2] * tangent[k]);
                            }
                            samples[i].0[k]
                                + contact_fit * (scale * axes[0] * q[0] + center[0]) * normal[k]
                                + (scale * axes[1] * q[1] + center[1]) * side[k]
                                + (scale * axes[2] * q[2] + center[2]) * tangent[k]
                        })
                        .collect()
                })
                .collect();
            curve.validate()?;
            curves.push(curve);
        }
        if closed {
            let first = curves[0].clone();
            *curves.last_mut().unwrap() = first;
        }
        Ok((curves, closed, residual,closed_planar_identity))
    }

    /// Retained section curves for downstream authored topology. These are
    /// previews; admission still requires a level report satisfying the budget.
    pub fn sections_at(&self, count: usize) -> Result<Vec<Curve>> {
        check(
            count >= self.options.initial_sections && count <= self.options.max_sections,
            "Section preview count outside configured sweep budget",
        )?;
        Ok(self.sections(count)?.0)
    }

    /// Compute one bounded preview without advancing the iterator. This allows
    /// hosts to request levels independently and cancel between requests.
    /// Unaccepted patches remain previews and must not be promoted to a body.
    pub fn preview_at(&self, count: usize) -> Result<Level> {
        check(
            count >= self.options.initial_sections && count <= self.options.max_sections,
            "Preview section count outside configured sweep budget",
        )?;
        self.level(count)
    }

    fn level(&self, count: usize) -> Result<Level> {
        let [_,cells,products]=self.spatial_rmf_error_limits.unwrap_or([512,10000,1000000]);
        self.level_with_error_budget(count,cells,products)
    }
    fn level_with_error_budget(&self, count: usize, max_cells: usize, max_products: usize) -> Result<Level> {
        self.level_with_error_budget_and_length(count,max_cells,max_products,None,None,None,None)
    }
    fn level_with_error_budget_and_length(&self,count:usize,max_cells:usize,max_products:usize,
        shared_length:Option<&crate::curve_measure::DivisionReport>,shared_guide:Option<&crate::curve_measure::DivisionReport>,
        shared_rmf:Option<&rmf_transport::OriginalRmfTransportReport>,shared_pose:Option<&mut authored_error::SharedArcPose>)->Result<Level>{
        let (coarse, closed, coarse_residual) = self.sections(count)?;
        let (fine, _, fine_residual) = self.sections(4 * (count - 1) + 1)?;
        let mut error = 0_f64;
        for (i, section) in fine.iter().enumerate() {
            let j = (i / 4).min(count - 2);
            let f = (i - 4 * j) as f64 / 4.;
            for (k, p) in section.control_points.iter().enumerate() {
                let delta: V = std::array::from_fn(|axis| {
                    p[axis]
                        - ((1. - f) * coarse[j].control_points[k][axis]
                            + f * coarse[j + 1].control_points[k][axis])
                });
                error = error.max(norm(delta));
            }
        }
        check(error.is_finite(), "Progressive sweep refinement overflowed")?;
        let patches = patches(&coarse)?;
        let length_residual_upper = coarse_residual.map(|r| r.max(fine_residual.unwrap()));
        let certificate = if let Some([steps,_,_])=self.spatial_rmf_error_limits {
            Some(authored_error::rmf_spatial_patch_error_with_transport(self,count,steps,max_cells,max_products,shared_length,shared_rmf)?)
        }else if self.frame_laws.is_some() {
            Some(authored_error::patch_error_with_length(self,count,max_cells,max_products,shared_length)?)
        } else if self.contact_source.is_some() {
            Some(authored_error::contact_patch_error_with_lengths(self,count,max_cells,max_products,shared_length,shared_guide)?)
        } else if self.orientation_guide.is_some() {
            Some(authored_error::guided_patch_error_with_lengths(self,count,max_cells,max_products,shared_length,shared_guide)?)
        } else if self.options.orientation==Orientation::Fixed {
            Some(authored_error::fixed_patch_error_with_length(self,count,max_cells,max_products,shared_length)?)
        } else if self.options.orientation==Orientation::FixedNormal {
            Some(authored_error::fixed_normal_patch_error_with_length(self,count,max_cells,max_products,shared_length)?)
        } else if self.options.orientation==Orientation::Frenet {
            Some(authored_error::frenet_patch_error_with_length(self,count,max_cells,max_products,shared_length)?)
        } else if self.options.orientation==Orientation::RotationMinimizing {
            Some(if !authored_error::original_line(self.path) {
                authored_error::rmf_planar_patch_error_with_length(self,count,max_cells,max_products,shared_length)?
            } else {self.rmf_straight_patch_error_bound(count,max_cells,max_products)?})
        } else if self.options.orientation==Orientation::CorrectedFrenet {
            Some(authored_error::corrected_patch_error_with_shared_arc_pose(self,count,max_cells,max_products,shared_length,shared_pose)?)
        } else {None};
        Ok(Level {
            patches,
            report: LevelReport {
                accepted: error <= self.options.max_deviation
                    && certificate.as_ref().and_then(|r|r.error_upper).is_none_or(|upper|upper<=self.options.max_deviation)
                    && (self.spatial_rmf_error_limits.is_none() || certificate.as_ref().is_some_and(|r|
                        r.status==super::progressive_miter::scalar_certificate::Status::Certified && r.within_budget)),
                sections: count,
                stations: fine.len(),
                sampled_control_deviation: error,
                budget: self.options.max_deviation,
                closed_path: closed,
                length_residual_upper,
                continuous_error_upper: certificate.as_ref().and_then(|r|r.error_upper),
                original_section_endpoint_error_upper: certificate.as_ref().and_then(|r|r.original_section_endpoint_error_upper),
                endpoint_contour_error_upper: certificate.as_ref().and_then(|r|r.endpoint_contour_error_upper),
                known_profile_error_upper: certificate.as_ref().and_then(|r|r.error_upper),
                error_certificate_cells: certificate.as_ref().map_or(0,|r|r.cells),
                decomposition_products: certificate.as_ref().map_or(0,|r|r.products),
                error_certificate_reason: certificate.as_ref().and_then(|r|r.reason).or(if certificate.is_none() {Some("transport-mode-error-unproved")} else {None}),
                continuous_bound: certificate.as_ref().is_some_and(|r|r.status==super::progressive_miter::scalar_certificate::Status::Certified),
            },
        })
    }
}

impl Iterator for Sweep<'_> {
    type Item = Result<Level>;
    fn next(&mut self) -> Option<Self::Item> {
        let count = self.next_sections.take()?;
        let level = self.level(count);
        if let Ok(result) = &level {
            if !result.report.accepted && count < self.options.max_sections {
                self.next_sections = Some((2 * (count - 1) + 1).min(self.options.max_sections));
            }
        }
        Some(level)
    }
}

pub fn approximate(
    profile: &Curve,
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    options: Options,
) -> Result<Approximation> {
    let sweep = Sweep::new(profile, path, scale, twist, options)?;
    let mut levels = Vec::new();
    let mut patches = None;
    for level in sweep {
        let level = level?;
        if level.report.accepted {
            patches = Some(level.patches);
        }
        levels.push(level.report);
    }
    Ok(Approximation { patches, levels })
}

/// Multiple authored profile curves transported on one shared refinement grid.
/// Curves may describe boundary pieces or separate contours. This operation does
/// not infer nesting, sew adjacent boundaries, add caps, or certify a solid.
#[derive(Clone, Debug)]
pub struct MultiLevel {
    pub patches: Vec<Surface>,
    /// Half-open patch ranges, one per input curve, in authored order.
    pub profile_patch_ranges: Vec<[usize; 2]>,
    pub report: LevelReport,
}

#[derive(Clone, Debug)]
pub struct MultiApproximation {
    pub patches: Option<Vec<Surface>>,
    pub profile_patch_ranges: Option<Vec<[usize; 2]>>,
    pub levels: Vec<LevelReport>,
}

pub struct MultiSweep<'a> {
    sweeps: Vec<Sweep<'a>>,
    options: Options,
    next_sections: Option<usize>,
}

impl<'a> MultiSweep<'a> {
    pub fn new(
        profiles: &'a [Curve],
        path: &'a Curve,
        scale: &'a Curve,
        twist: &'a Curve,
        options: Options,
    ) -> Result<Self> {
        check(
            (1..=64).contains(&profiles.len()),
            "Multi-profile sweep needs 1..64 curves",
        )?;
        let station_cache: StationCache = Default::default();
        let sweeps = profiles
            .iter()
            .map(|profile| {
                let mut sweep = Sweep::new(profile, path, scale, twist, options)?;
                sweep.station_cache = station_cache.clone();
                Ok(sweep)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            sweeps,
            options,
            next_sections: Some(options.initial_sections),
        })
    }
}

impl<'a> MultiSweep<'a> {
    pub fn with_spatial_rmf_error_limits(mut self,steps:usize,cells:usize,products:usize)->Result<Self>{
        self.sweeps=self.sweeps.into_iter().map(|s|s.with_spatial_rmf_error_limits(steps,cells,products)).collect::<Result<_>>()?;
        Ok(self)
    }
    /// Retained preview sections in authored profile order, each on the same grid.
    /// Body admission still requires an accepted aggregate level report.
    pub fn sections_at(&self, count: usize) -> Result<Vec<Vec<Curve>>> {
        self.sweeps.iter().map(|s| s.sections_at(count)).collect()
    }
    /// One selected profile anchor defines the transverse fit for every contour.
    /// Hole boundaries therefore retain their relative width and shared joins.
    pub fn with_contact_guide(
        mut self,
        guide: &'a Curve,
        profile_index: usize,
        parameter: f64,
    ) -> Result<Self> {
        check(
            profile_index < self.sweeps.len(),
            "Contact profile index outside authored profiles",
        )?;
        let reference = self
            .sweeps
            .remove(profile_index)
            .with_contact_guide(guide, parameter)?;
        let point = reference.contact_point.unwrap();
        let source = reference.contact_source.unwrap();
        self.sweeps.insert(profile_index, reference);
        self.sweeps = self
            .sweeps
            .into_iter()
            .map(|s| {
                let mut result = s.with_contact_point(guide, point)?;
                result.contact_source = Some(source);
                Ok(result)
            })
            .collect::<Result<_>>()?;
        self.next_sections = Some(self.options.initial_sections);
        Ok(self)
    }
    /// Shared correspondence/orientation for all contours on the aggregate grid.
    pub fn with_orientation_guide(mut self, guide: &'a Curve) -> Result<Self> {
        self.sweeps = self
            .sweeps
            .into_iter()
            .map(|s| s.with_orientation_guide(guide))
            .collect::<Result<_>>()?;
        self.next_sections = Some(self.options.initial_sections);
        Ok(self)
    }
    /// Shared complete orientation for every profile on the aggregate grid.
    pub fn with_frame_laws(
        mut self,
        longitudinal: &'a Curve,
        transverse: &'a Curve,
    ) -> Result<Self> {
        self.sweeps = self
            .sweeps
            .into_iter()
            .map(|s| s.with_frame_laws(longitudinal, transverse))
            .collect::<Result<_>>()?;
        self.next_sections = Some(self.options.initial_sections);
        Ok(self)
    }
    pub fn with_affine_laws(mut self, axis_scale: &'a Curve, center: &'a Curve) -> Result<Self> {
        self.sweeps = self
            .sweeps
            .into_iter()
            .map(|s| s.with_affine_laws(axis_scale, center))
            .collect::<Result<_>>()?;
        self.next_sections = Some(self.options.initial_sections);
        Ok(self)
    }
}

impl MultiSweep<'_> {
    /// One aggregate preview; does not consume or alter iterator progression.
    pub fn preview_at(&self, count: usize) -> Result<MultiLevel> {
        check(
            count >= self.options.initial_sections && count <= self.options.max_sections,
            "Preview section count outside configured sweep budget",
        )?;
        let mut patches = Vec::new();
        let mut ranges = Vec::new();
        let mut report: Option<LevelReport> = None;
        // One invocation owns the original path and all profile certificates.
        // Measure that shared source once; per-profile pose and decomposition
        // work still consume the remaining aggregate budget.
        let first=&self.sweeps[0];
        let [_,cell_limit,product_limit]=first.spatial_rmf_error_limits.unwrap_or([512,10000,1000000]);
        let shared_guided=first.frame_laws.is_none()&&first.orientation_guide.is_some()
            &&first.orientation_guide.is_some_and(|g|!g.periodic)
            &&match first.orientation_guide{Some(g)=>!path_is_closed(g)?,None=>false};
        let shared_length=if cell_limit>0 && (first.spatial_rmf_error_limits.is_some() || ((shared_guided || first.frame_laws.is_some() || (first.options.orientation==Orientation::Fixed||first.options.orientation==Orientation::FixedNormal||first.options.orientation==Orientation::Frenet||first.options.orientation==Orientation::RotationMinimizing||first.options.orientation==Orientation::CorrectedFrenet)
            && first.orientation_guide.is_none() && first.contact_point.is_none())&&(!authored_error::original_line(first.path)||shared_guided&&(first.contact_source.is_some()||first.orientation_guide.is_some_and(|g|!authored_error::original_line(g))))
            && (!first.path.periodic||(path_is_closed(first.path)?
                &&first.options.orientation==Orientation::CorrectedFrenet
                &&first.frame_laws.is_none()&&first.orientation_guide.is_none()&&first.contact_point.is_none()))&&(!path_is_closed(first.path)?
                || first.options.orientation==Orientation::CorrectedFrenet
                    &&first.frame_laws.is_none()&&first.orientation_guide.is_none()&&first.contact_point.is_none()))) {
            if let Spacing::ArcLength {tolerance,max_cells}=first.options.spacing {
                check(self.sweeps.iter().all(|s|std::ptr::eq(s.path,first.path)&&s.options.spacing==first.options.spacing),
                    "Shared length certificate requires identical original source and spacing")?;
                let phase=if first.contact_point.is_some() {first.orientation_guide.map(|guide|arc_guide::certify(first.path,guide,max_cells.min(cell_limit))).transpose()?}else{None};
                let phase_cells=phase.as_ref().map_or(0,|p|p.cells);
                let tolerance=phase.as_ref().and_then(|p|p.scale_upper).filter(|scale|*scale>1.).map_or(tolerance,|scale|(tolerance/scale).next_down());
                // The shared source division owns both the exact phase premise
                // and the inverse-length cells, preserving the aggregate ceiling.
                let mut division=crate::curve_measure::divide_by_length(first.path,count-1,tolerance,max_cells.min(cell_limit)-phase_cells)?;
                division.cells+=phase_cells;
                Some(division)
            }else{None}
        }else{None};
        let mut cells=shared_length.as_ref().map_or(0,|report|report.cells);
        let shared_guide=if shared_guided&&shared_length.is_some()&&cells<cell_limit {
            let guide=first.orientation_guide.unwrap();
            check(self.sweeps.iter().all(|s|s.orientation_guide.is_some_and(|g|std::ptr::eq(g,guide))),
                "Shared guide length certificate requires identical original guide")?;
            let Spacing::ArcLength {tolerance,max_cells}=first.options.spacing else{unreachable!()};
            let report=crate::curve_measure::divide_by_length(guide,count-1,tolerance,max_cells.min(cell_limit-cells))?;
            cells+=report.cells;Some(report)
        }else{None};
        // Both station policies share precisely the same original Bishop
        // transport and closing phase. Source arc division is also shared.
        // Every profile query/decomposition consumes the remaining allowance.
        let shared_rmf=if let Some([steps,_,_])=first.spatial_rmf_error_limits {
                check(self.sweeps.iter().all(|s|std::ptr::eq(s.path,first.path)
                    &&s.options.normal==first.options.normal&&s.options.spacing==first.options.spacing
                    &&s.spatial_rmf_error_limits==first.spatial_rmf_error_limits),
                    "Shared RMF certificate requires identical original source and policy")?;
                let remaining=cell_limit-cells;
                let proof=rmf_transport::certify_original_rmf_transport_shared(first.path,first.options.normal,
                    steps,remaining,path_is_closed(first.path)?)?;
                cells+=proof.cells+proof.exact_work as usize;Some(proof)
        }else{None};
        let mut shared_pose=if first.options.orientation==Orientation::CorrectedFrenet
            &&first.options.spacing!=Spacing::Parameter&&first.frame_laws.is_none()
            &&first.orientation_guide.is_none()&&first.contact_point.is_none() {
            check(self.sweeps.iter().all(|s|std::ptr::eq(s.path,first.path)
                &&std::ptr::eq(s.scale,first.scale)&&std::ptr::eq(s.twist,first.twist)
                &&s.affine_laws.map(|(a,c)|(a as *const _,c as *const _))==first.affine_laws.map(|(a,c)|(a as *const _,c as *const _))
                &&s.options.orientation==first.options.orientation&&s.options.normal==first.options.normal
                &&s.options.spacing==first.options.spacing&&s.options.initial_sections==first.options.initial_sections
                &&s.options.max_sections==first.options.max_sections&&s.options.max_deviation==first.options.max_deviation&&s.frame_laws.is_none()&&s.orientation_guide.is_none()&&s.contact_point.is_none()),
                "Shared corrected pose requires identical original source, laws and policy")?;
            Some(authored_error::SharedArcPose::default())
        }else{None};
        let mut products=0;
        for sweep in &self.sweeps {
            let level = sweep.level_with_error_budget_and_length(count,cell_limit-cells,product_limit-products,shared_length.as_ref(),shared_guide.as_ref(),shared_rmf.as_ref(),shared_pose.as_mut())?;
            cells+=level.report.error_certificate_cells;
            products+=level.report.decomposition_products;
            check(
                patches.len() + level.patches.len() <= 4096,
                "Multi-profile sweep exceeds 4096 total retained patches",
            )?;
            let first = patches.len();
            patches.extend(level.patches);
            ranges.push([first, patches.len()]);
            if let Some(total) = &mut report {
                total.accepted &= level.report.accepted;
                total.sampled_control_deviation = total
                    .sampled_control_deviation
                    .max(level.report.sampled_control_deviation);
                total.continuous_bound &= level.report.continuous_bound;
                total.known_profile_error_upper = match (total.known_profile_error_upper,level.report.known_profile_error_upper) {
                    (Some(a),Some(b))=>Some(a.max(b)),(a,b)=>a.or(b),
                };
                total.continuous_error_upper = match (total.continuous_error_upper,level.report.continuous_error_upper) {
                    (Some(a),Some(b))=>Some(a.max(b)),_=>None,
                };
                total.original_section_endpoint_error_upper = match (total.original_section_endpoint_error_upper,level.report.original_section_endpoint_error_upper) {
                    (Some(a),Some(b))=>Some(a.max(b)),_=>None,
                };
                total.endpoint_contour_error_upper = match (total.endpoint_contour_error_upper,level.report.endpoint_contour_error_upper) {
                    (Some(a),Some(b))=>Some([a[0].max(b[0]),a[1].max(b[1])]),_=>None,
                };
                total.error_certificate_cells += level.report.error_certificate_cells;
                total.decomposition_products += level.report.decomposition_products;
                total.error_certificate_reason = total.error_certificate_reason.or(level.report.error_certificate_reason);
            } else {
                report = Some(level.report);
            }
        }
        let mut report=report.unwrap();
        report.error_certificate_cells=cells;
        Ok(MultiLevel {
            patches,
            profile_patch_ranges: ranges,
            report,
        })
    }
}

impl Iterator for MultiSweep<'_> {
    type Item = Result<MultiLevel>;
    fn next(&mut self) -> Option<Self::Item> {
        let count = self.next_sections.take()?;
        let level = self.preview_at(count);
        if let Ok(result) = &level {
            if !result.report.accepted && count < self.options.max_sections {
                self.next_sections = Some((2 * (count - 1) + 1).min(self.options.max_sections));
            }
        }
        Some(level)
    }
}

pub fn approximate_profiles(
    profiles: &[Curve],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    options: Options,
) -> Result<MultiApproximation> {
    collect_profiles(MultiSweep::new(profiles, path, scale, twist, options)?)
}

pub fn approximate_affine_profiles(
    profiles: &[Curve],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    axis_scale: &Curve,
    center: &Curve,
    options: Options,
) -> Result<MultiApproximation> {
    collect_profiles(
        MultiSweep::new(profiles, path, scale, twist, options)?
            .with_affine_laws(axis_scale, center)?,
    )
}

/// Original closed spatial RMF error policy, shared across all profiles.
pub fn approximate_spatial_rmf_profiles(
    profiles: &[Curve], path: &Curve, scale: &Curve, twist: &Curve,
    affine: Option<(&Curve, &Curve)>, options: Options,
    transport_steps: usize, max_cells: usize, max_products: usize,
) -> Result<MultiApproximation> {
    let sweep = MultiSweep::new(profiles, path, scale, twist, options)?;
    let sweep = if let Some((axes, center)) = affine {
        sweep.with_affine_laws(axes, center)?
    } else { sweep };
    collect_profiles(sweep.with_spatial_rmf_error_limits(transport_steps, max_cells, max_products)?)
}

/// Complete authored frames, simultaneous affine laws and aggregate admission.
pub fn approximate_authored_profiles(
    profiles: &[Curve],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    longitudinal: &Curve,
    transverse: &Curve,
    axis_scale: &Curve,
    center: &Curve,
    mut options: Options,
) -> Result<MultiApproximation> {
    options.orientation = Orientation::Fixed;
    collect_profiles(
        MultiSweep::new(profiles, path, scale, twist, options)?
            .with_frame_laws(longitudinal, transverse)?
            .with_affine_laws(axis_scale, center)?,
    )
}

/// Shared guide-normal correspondence with simultaneous affine laws.
pub fn approximate_guided_profiles(
    profiles: &[Curve],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    guide: &Curve,
    axis_scale: &Curve,
    center: &Curve,
    options: Options,
) -> Result<MultiApproximation> {
    collect_profiles(
        MultiSweep::new(profiles, path, scale, twist, options)?
            .with_orientation_guide(guide)?
            .with_affine_laws(axis_scale, center)?,
    )
}

/// Contact fit shared by every profile using one authored reference anchor.
pub fn approximate_contact_profiles(
    profiles: &[Curve],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    guide: &Curve,
    profile_index: usize,
    parameter: f64,
    axis_scale: &Curve,
    center: &Curve,
    options: Options,
) -> Result<MultiApproximation> {
    collect_profiles(
        MultiSweep::new(profiles, path, scale, twist, options)?
            .with_contact_guide(guide, profile_index, parameter)?
            .with_affine_laws(axis_scale, center)?,
    )
}

pub(crate) fn collect_profiles(sweep: MultiSweep<'_>) -> Result<MultiApproximation> {
    let mut result = MultiApproximation {
        patches: None,
        profile_patch_ranges: None,
        levels: Vec::new(),
    };
    for level in sweep {
        let level = level?;
        if level.report.accepted {
            result.patches = Some(level.patches);
            result.profile_patch_ranges = Some(level.profile_patch_ranges);
        }
        result.levels.push(level.report);
    }
    Ok(result)
}

/// Geometric closure classification shared with topology resource admission.
/// Tangent/law seam conditions are checked by the sweep itself.
pub fn path_is_closed(path: &Curve) -> Result<bool> {
    path.validate()?;
    check(path.control_points[0].len() == 3, "Sweep path must be3D")?;
    let [a, b] = path.domain();
    let start = path.evaluate(a)?.point;
    let end = path.evaluate(b)?.point;
    closed_extent(
        path,
        [start[0], start[1], start[2]],
        [end[0], end[1], end[2]],
    )
}
fn closed_extent(path: &Curve, start: V, end: V) -> Result<bool> {
    let extent = path
        .control_points
        .iter()
        .map(|p| norm(sub([p[0], p[1], p[2]], start)))
        .fold(0_f64, f64::max);
    check(
        extent.is_finite() && extent > 0.,
        "Sweep path extent is zero or unrepresentable",
    )?;
    Ok(start == end || norm(sub(end, start)) / extent <= 64. * f64::EPSILON)
}

fn profile_parts(curve: &Curve) -> Result<Vec<Curve>> {
        if curve.control_points.len() <= 32 {
            return Ok(vec![curve.clone()]);
        }
        check(
            !curve.periodic,
            "Dense periodic sweep profiles need explicit clamped encoding",
        )?;
        Ok(curve
            .decompose()?
            .iter()
            .map(|s| s.definition().clone())
            .collect())
}
fn patches(sections: &[Curve]) -> Result<Vec<Surface>> {
    let rows = sections.iter().map(profile_parts).collect::<Result<Vec<_>>>()?;
    let count = sections.len();
    let columns = (count - 1).div_ceil(31);
    check(
        rows[0].len() * columns <= 4096,
        "Progressive sweep exceeds 4096 retained patches",
    )?;
    let mut result = Vec::new();
    for u in 0..rows[0].len() {
        let mut first = 0;
        while first + 1 < count {
            let last = (first + 31).min(count - 1);
            let curves = (first..=last)
                .map(|v| rows[v][u].clone())
                .collect::<Vec<_>>();
            let mut surface = crate::surface::loft(&curves)?;
            for k in &mut surface.knots_v {
                *k = (*k + first as f64) / (count - 1) as f64;
            }
            surface.validate()?;
            result.push(surface);
            first = last;
        }
    }
    Ok(result)
}

#[cfg(feature = "codec")]
#[path = "progressive_sweep/serialization.rs"]
mod serialization;
#[cfg(test)]
#[path = "progressive_sweep/tests.rs"]
mod tests;
