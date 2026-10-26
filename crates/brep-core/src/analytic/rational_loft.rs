//! Retained rational section loft with authored incidence and audited cap trims.
//! A closed manifold model is not a proof of global embedding/self-separation.
use super::*;
use nurbs_core::numerics::error_upper as error_bound;
mod affine_stations;
mod smooth_stations;
pub use affine_stations::{
    AffinePose, AffineStationLimits, AffineStationWalls, affine_station_walls,
};
pub use smooth_stations::{SmoothStationWalls, smooth_station_walls};
mod polygon_stations;
pub use polygon_stations::{PolygonStationWalls, smooth_polygon_station_walls};
type P = [f64; 3];
fn err(message: impl Into<String>) -> Error {
    Error::new("BREP_RATIONAL_SWEEP_REFUSED", message)
}
fn minus(a: P, b: P) -> P {
    std::array::from_fn(|k| a[k] - b[k])
}
fn dot(a: P, b: P) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
fn cross(a: P, b: P) -> P {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(p: P) -> Result<P> {
    let n = p[0].hypot(p[1]).hypot(p[2]);
    if !n.is_finite() || n <= 1e-10 {
        return Err(err("Singular cap frame"));
    }
    Ok(p.map(|x| x / n))
}
fn point(c: &Curve, start: bool) -> P {
    let p = if start {
        &c.control_points[0]
    } else {
        c.control_points.last().unwrap()
    };
    [p[0], p[1], p[2]]
}
struct Cap {
    normal: P,
    surface: Surface,
    loops: Vec<Vec<Curve>>,
}
mod caps;
use caps::*;

/// Sections contain one outer loop followed by clockwise holes in a common
/// authored correspondence. Curves are decomposed exactly into Bezier spans;
/// corresponding spans retain degree; rational weights may differ by section. Planar end caps must pass
/// interval trim-region audit. Side regularity/global self-intersections remain
/// unproven and are reported by the model's separate solid audit.
/// Rebuilding creates independent naming provenance. Match every retained
/// geometric/topological value; source mutation is checked by the proof owner.
pub fn section_loft_source_matches(source:&Model,sections:&[Vec<Vec<Curve>>],closed:bool)->Result<bool> {
    source.validate()?;
    let rebuilt=if closed {periodic_section_loft(sections)?} else {rational_section_loft(sections)?};
    Ok(source.0==rebuilt.0)
}
pub fn rational_section_loft(sections: &[Vec<Vec<Curve>>]) -> Result<Model> {
    section_loft(sections, None, None, false)
}

/// Cubic section interpolation with audited planar caps and shared side edges.
/// Requires authored span correspondence; global embedding is audited separately.
pub fn natural_section_loft(sections: &[Vec<Vec<Curve>>], parameters: &[f64]) -> Result<Model> {
    if !(2..=11).contains(&sections.len()) || parameters.len() != sections.len() {
        return Err(err(
            "Natural capped loft needs 2..11 sections and matching stations",
        ));
    }
    section_loft(sections, Some(parameters), None, false)
}

/// Caps complete authored side patches. Every patch corresponds to one Bezier
/// span of the endpoint loops; shared isocurves are checked by BRep validation.
pub fn capped_loft_surfaces(
    start: &[Vec<Curve>],
    end: &[Vec<Curve>],
    sides: &[Vec<Surface>],
) -> Result<Model> {
    section_loft(&[start.to_vec(), end.to_vec()], None, Some(sides), false)
}

/// Retain one supplied wall patch per section interval and Bezier profile span.
/// Owned longitudinal edges are extracted from the actual supplied surfaces.
/// Material/embedding and approximation certificates remain separate audits.
pub fn section_loft_surfaces(
    sections:&[Vec<Vec<Curve>>],sides:&[Vec<Surface>],periodic:bool,
)->Result<Model>{
    if periodic && sections.len()<4 {return Err(err("Periodic loft needs at least four sections including repeated seam"));}
    section_loft(sections,None,Some(sides),periodic)
}

/// Uncapped periodic topology: final section must be an exact repeat of the
/// first. Each authored contour owns a connected shell; holes become inner
/// shells of one body. Embedding and global shell containment remain unproven.
pub fn periodic_section_loft(sections: &[Vec<Vec<Curve>>]) -> Result<Model> {
    if sections.len() < 4 {
        return Err(err(
            "Periodic loft needs at least four sections including repeated seam",
        ));
    }
    section_loft(sections, None, None, true)
}

/// Native candidate for an independently owned closed material chart. No
/// physical cap or planar section premise is introduced. Orientation is a
/// proposal only; this operation supplies no Solid/embedding certificate.
pub(crate) fn periodic_material_section_loft(sections: &[Vec<Vec<Curve>>]) -> Result<Model> {
    if sections.len() < 4 {
        return Err(err("Periodic material loft needs a repeated seam and four sections"));
    }
    section_loft_impl(sections, None, None, true, true)
}
pub(crate) fn periodic_material_loft_surfaces(sections: &[Vec<Vec<Curve>>], sides: &[Vec<Surface>]) -> Result<Model> {
    if sections.len() < 4 { return Err(err("Periodic material loft needs a repeated seam and four sections")); }
    section_loft_impl(sections, None, Some(sides), true, true)
}
fn section_loft(
    sections: &[Vec<Vec<Curve>>],
    parameters: Option<&[f64]>,
    sides: Option<&[Vec<Surface>]>,
    periodic: bool,
) -> Result<Model> {
    section_loft_impl(sections, parameters, sides, periodic, false)
}
fn section_loft_impl(
    sections: &[Vec<Vec<Curve>>],
    parameters: Option<&[f64]>,
    sides: Option<&[Vec<Surface>]>,
    periodic: bool,
    material_chart: bool,
) -> Result<Model> {
    if !(2..=1025).contains(&sections.len()) {
        return Err(err("Need 2..1025 rational sections"));
    }
    let loop_count = sections[0].len();
    if !(1..=16).contains(&loop_count) || sections.iter().any(|s| s.len() != loop_count) {
        return Err(err("Need matching 1..16 loops per section"));
    }
    let mut prepared = Vec::new();
    for section in sections {
        let mut loops = Vec::new();
        let mut count = 0;
        for ring in section {
            if ring.is_empty() || ring.len() > 64 {
                return Err(err("Section loop needs 1..64 curves"));
            }
            let mut pieces = Vec::new();
            for curve in ring {
                curve.validate()?;
                if curve
                        .control_points
                        .iter()
                        .any(|p| p.len() != 3 || p.iter().any(|x| x.abs() > 1e6))
                {
                    return Err(err("Need bounded 3D section curves"));
                }
                for c in retained_bezier_pieces(curve)? {
                    pieces.push(c);
                    count += 1;
                    if count > 64 {
                        return Err(err("Section exceeds 64 Bezier spans"));
                    }
                }
            }
            if pieces.len() < 3 {
                return Err(err(
                    "Closed loop needs at least three distinct boundary spans",
                ));
            }
            for i in 0..pieces.len() {
                if point(&pieces[i], false) != point(&pieces[(i + 1) % pieces.len()], true) {
                    return Err(err("Section loops require exact authored endpoint joins"));
                }
            }
            loops.push(pieces);
        }
        if count * (sections.len() - 1) + if periodic { 0 } else { 2 } > MAX_FACES {
            return Err(err("Rational section loft exceeds B-rep face budget"));
        }
        prepared.push(loops);
    }
    for section in &prepared[1..] {
        for (a, b) in prepared[0].iter().zip(section) {
            if a.len() != b.len()
                || a.iter().zip(b).any(|(a, b)| {
                    a.degree != b.degree || a.control_points.len() != b.control_points.len()
                })
            {
                return Err(err(
                    "Rational section span correspondence/degrees must match",
                ));
            }
        }
    }
    if periodic && prepared.first() != prepared.last() {
        return Err(err(
            "Periodic section loft requires identical rational seam definitions",
        ));
    }
    if let Some(sides) = sides {
        if sides.len() != loop_count
            || sides
                .iter()
                .zip(&prepared[0])
                .any(|(s, r)| s.len() != r.len() * (prepared.len()-1))
        {
            return Err(err(
                "Capped loft side patches must match endpoint Bezier spans",
            ));
        }
        for surface in sides.iter().flatten() {
            surface.validate()?;
            if surface.knots_u[surface.degree_u] != 0.
                || surface.knots_u[surface.control_points.len()] != 1.
                || surface.knots_v[surface.degree_v] != 0.
                || surface.knots_v[surface.control_points[0].len()] != 1.
            {
                return Err(err("Capped loft patches require normalized U/V domains"));
            }
        }
    }
    let (caps, direction) = if material_chart {
        // A control-polygon area vector only PROPOSES orientation. It need not
        // be a cap normal or certify the nonplanar material region. The caller
        // must independently verify every retained shell orientation.
        let points = prepared[0][0].iter().flat_map(|c| &c.control_points)
            .map(|p| [p[0], p[1], p[2]]).collect::<Vec<_>>();
        let origin = points[0];
        let mut area = [0.; 3];
        for i in 0..points.len() {
            let c = cross(minus(points[i], origin), minus(points[(i + 1) % points.len()], origin));
            for k in 0..3 { area[k] += c[k]; }
        }
        let travel = minus(point(&prepared[1][0][0], true), point(&prepared[0][0][0], true));
        let proposed = dot(travel, area);
        (Vec::new(), if proposed < 0. { -1. } else { 1. })
    } else {
        let lower = cap(&prepared[0])?;
        let upper = if periodic { None } else { Some(cap(prepared.last().unwrap())?) };
        let travel = minus(point(&prepared[1][0][0], true), point(&prepared[0][0][0], true));
        let direction = dot(travel, lower.normal);
        if !direction.is_finite() || direction.abs() <= 1e-10 {
            return Err(err("Initial loft ruling must cross the cap plane"));
        }
        let caps = if let Some(upper) = upper {
            vec![(0, lower), (prepared.len() - 1, upper)]
        } else { Vec::new() };
        (caps, direction)
    };

    let mut build = Builder::new();
    let mut ids = prepared
        .iter()
        .enumerate()
        .map(|(station, s)| {
            if periodic && station == prepared.len() - 1 {
                return Vec::new();
            }
            if parameters.is_some() && station != 0 && station != prepared.len() - 1 {
                return Vec::new();
            }
            s.iter()
                .map(|ring| {
                    ring.iter()
                        .map(|c| {
                            let id = build.model.vertices.len();
                            build.model.vertices.push(Vertex {
                                point: point(c, true),
                            });
                            id
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if periodic {
        let last = ids.len() - 1;
        ids[last] = ids[0].clone();
    }
    let mut shell_faces = vec![Vec::new(); loop_count];
    let layers = if parameters.is_some() {
        1
    } else {
        prepared.len() - 1
    };
    for layer in 0..layers {
        let next = if parameters.is_some() {
            prepared.len() - 1
        } else {
            layer + 1
        };
        for ring in 0..loop_count {
            let count = prepared[layer][ring].len();
            for i in 0..count {
                let j = (i + 1) % count;
                let a = &prepared[layer][ring][i];
                let b = &prepared[next][ring][i];
                let side = if let Some(s) = sides {
                    s[ring][layer * count + i].clone()
                } else if let Some(p) = parameters {
                    let profiles = prepared
                        .iter()
                        .map(|s| s[ring][layer * count + i].clone())
                        .collect::<Vec<_>>();
                    nurbs_core::natural_loft::interpolate(&profiles, p)?
                } else {
                    nurbs_core::surface::loft(&[a.clone(), b.clone()])?
                };
                let end_edge = side.iso(nurbs_core::surface::Axis::U, 1.)?;
                let start_edge = side.iso(nurbs_core::surface::Axis::U, 0.)?.reverse()?;
                build.rectangular_patch(
                    side,
                    [
                        ids[layer][ring][i],
                        ids[layer][ring][j],
                        ids[next][ring][j],
                        ids[next][ring][i],
                    ],
                    [a.clone(), end_edge, b.reverse()?, start_edge],
                );
                shell_faces[ring].push(FaceUse {
                    face: build.model.faces.len() - 1,
                    reversed: false,
                });
            }
        }
    }
    for (layer, cap) in caps {
        let mut wires = Vec::new();
        for (ring, uv) in cap.loops.iter().enumerate() {
            let mut coedges = Vec::new();
            for i in 0..uv.len() {
                let j = (i + 1) % uv.len();
                coedges.push(build.coedge(
                    ids[layer][ring][i],
                    ids[layer][ring][j],
                    prepared[layer][ring][i].clone(),
                    uv[i].clone(),
                ));
            }
            wires.push(build.wire(coedges));
        }
        build.face(cap.surface, wires[0], wires[1..].to_vec(), layer == 0);
    }
    let mut model = if periodic {
        let mut model = build.model;
        model.shells = shell_faces
            .into_iter()
            .map(|faces| Shell {
                faces,
                closed: true,
            })
            .collect();
        model.bodies.push(Body {
            outer_shell: 0,
            inner_shells: (1..loop_count).collect(),
        });
        model.rebuild_topology_ids();
        model.validate()?;
        model
    } else {
        build.finish()?
    };
    if direction < 0. {
        for face in model.shells.iter_mut().flat_map(|s| &mut s.faces) {
            face.reversed = !face.reversed;
        }
        model.validate()?;
    }
    Ok(model)
}

/// Progressive transport of ordered planar section boundaries. Open paths use
/// rational_section_loft caps; closed paths use periodic contour shells. This returns a topology-validated model and sampled
/// approximation evidence, not a certified globally embedded CAD solid.
pub fn progressive_profile_body(
    loops: &[Vec<Curve>],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    options: nurbs_core::progressive_sweep::Options,
) -> Result<(Model, nurbs_core::progressive_sweep::MultiApproximation)> {
    progressive_body_laws(loops, path, scale, twist, None, None, None, options)
}

pub fn progressive_affine_profile_body(
    loops: &[Vec<Curve>],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    axis_scale: &Curve,
    center: &Curve,
    options: nurbs_core::progressive_sweep::Options,
) -> Result<(Model, nurbs_core::progressive_sweep::MultiApproximation)> {
    progressive_body_laws(
        loops,
        path,
        scale,
        twist,
        Some((axis_scale, center)),
        None,
        None,
        options,
    )
}

/// Complete authored frames with affine laws and retained rational body topology.
/// Endpoint caps still require planar audited contour regions; sampled admission
/// does not prove globally embedded walls or continuous frame regularity.
pub fn progressive_authored_profile_body(
    loops: &[Vec<Curve>],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    longitudinal: &Curve,
    transverse: &Curve,
    axis_scale: &Curve,
    center: &Curve,
    mut options: nurbs_core::progressive_sweep::Options,
) -> Result<(Model, nurbs_core::progressive_sweep::MultiApproximation)> {
    options.orientation = nurbs_core::progressive_sweep::Orientation::Fixed;
    progressive_body_laws(
        loops,
        path,
        scale,
        twist,
        Some((axis_scale, center)),
        Some((longitudinal, transverse)),
        None,
        options,
    )
}

/// Spatial orientation rail with optional shared contact anchor (flattened
/// authored profile index, parameter in that profile domain). Contact is sampled.
pub fn progressive_guided_profile_body(
    loops: &[Vec<Curve>],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    guide: &Curve,
    anchor: Option<(usize, f64)>,
    axis_scale: &Curve,
    center: &Curve,
    options: nurbs_core::progressive_sweep::Options,
) -> Result<(Model, nurbs_core::progressive_sweep::MultiApproximation)> {
    progressive_body_laws(
        loops,
        path,
        scale,
        twist,
        Some((axis_scale, center)),
        None,
        Some((guide, anchor)),
        options,
    )
}

fn progressive_body_laws(
    loops: &[Vec<Curve>],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    affine: Option<(&Curve, &Curve)>,
    frames: Option<(&Curve, &Curve)>,
    guidance: Option<(&Curve, Option<(usize, f64)>)>,
    options: nurbs_core::progressive_sweep::Options,
) -> Result<(Model, nurbs_core::progressive_sweep::MultiApproximation)> {
    let result=progressive_profile_body_with_evidence(loops,path,scale,twist,affine,frames,guidance,options)?;
    Ok((result.model,result.approximation))
}

/// Evidence belongs to the model and endpoints produced in this construction.
/// Cap material identity remains separate from original-domain and boundary E.
#[derive(Clone,Copy)]
pub struct EndpointCapCorrection {
    pub quantum:f64,
    pub tolerance:f64,
    pub max_work:u64,
}
pub struct ProgressiveBodyEvidence {
    pub model:Model,
    pub approximation:nurbs_core::progressive_sweep::MultiApproximation,
    pub retained_caps:Option<crate::sweep_retained_caps::Report>,
    pub cap_projection:Option<nurbs_core::progressive_sweep::EndpointCapProjectionReport>,
    pub filled_cap_error_upper:Option<[f64;2]>,
    pub cap_correction_error_upper:Option<f64>,
    pub profile_correction_error_upper:Option<f64>,
    pub retained_walls:crate::sweep_retained_walls::Report,
    pub boundary_error_upper:Option<f64>,
    pub boundary_error_within_budget:Option<bool>,
    pub body_decomposition_error_upper:Option<f64>,
    pub body_decomposition_products:usize,
    // Constructor-owned premises cannot be replaced by public receipt fields.
    station_sections:Vec<Vec<Vec<Curve>>>,
    station_boundary:nurbs_core::sweeps::filled_cap_error::BoundaryCertificate,
    station_closed:bool,
    station_budget:f64,
    annular_caps:Option<AnnularCaps>,
    cap_selection_work:u64,
}

mod annular_caps;
pub use annular_caps::{Boundary as AnnularProgressiveBody, CapEvidence as AnnularCaps};

impl ProgressiveBodyEvidence {
    /// Explicit retained-wall reconstruction, bounded against the original sweep.
    /// This does not certify actual jets, regularity, embedding or Solid admission.
    /// Mutating the public model cannot transfer the private source certificate.
    pub fn reconstruct_stations(
        &self,
        quantum:f64,
        tolerance:f64,
        max_work:u64,
    )->Result<crate::sweep_miter_owned::StationBoundary>{
        use nurbs_core::{numerics::error_upper,sweeps::filled_cap_error};
        let source=&self.station_boundary;
        let mut out=crate::sweep_miter_owned::StationBoundary {
            reconstruction:None,boundary:None,reason:"source-bound-unproved",
        };
        if !source.continuous_bound || source.within_budget!=Some(true) {return Ok(out);}
        let mut reconstruction=if self.annular_caps.is_some() {
            annular_caps::reconstruct(&self.model,&self.station_sections,quantum,tolerance,max_work)?
        } else {
            crate::sweep_station_reconstruction::reconstruct(
                &self.model,&self.station_sections,&[],self.station_closed,
                quantum,tolerance,max_work,
            )?
        };
        let Some(displacement)=reconstruction.candidate.wall_displacement_upper else {
            out.reason=reconstruction.candidate.reason;
            return Ok(out);
        };
        if reconstruction.model.is_none(){out.reason=reconstruction.candidate.reason;return Ok(out);}
        let wall=source.wall_error_upper.and_then(|upper|error_upper::add(upper,displacement));
        let boundary=filled_cap_error::compose_boundary(
            wall,source.filled_cap_error_upper,self.station_closed,Some(self.station_budget),
        );
        if !boundary.continuous_bound || boundary.within_budget!=Some(true) {
            reconstruction.model=None;
            out.reason="boundary-budget-unproved";
        }else{out.reason="owned-progressive-station-complete-boundary";}
        out.reconstruction=Some(reconstruction);
        out.boundary=Some(boundary);
        Ok(out)
    }
}

/// Reproduce the constructor's retained sections and certify their complete
/// correspondence to each original transported section. The error can be
/// interpolated along a wall only when rational bases agree at every station.
pub(crate) fn retained_section_partition(sections:&[Vec<Vec<Curve>>],max_products:usize)
    ->Result<(Vec<Vec<Vec<Curve>>>,Option<f64>,usize)>{
    if max_products>1000000 {return Err(err("Body decomposition product budget exceeds1000000"));}
    let mut retained=Vec::new();let mut upper=Some(0f64);let mut products=0;
    for station in sections {
        let mut rings=Vec::new();
        for ring in station {
            let mut pieces=Vec::new();
            for curve in ring {
                let parts=retained_bezier_pieces(curve)?;
                // Already segmented profiles are copied coefficient-for-
                // coefficient by the constructor, with no extraction rounding.
                let exact=nurbs_core::retained_wall_coefficients::segmented_bezier_controls(curve,1000000)
                    .is_some_and(|source|source==parts);
                if !exact && upper.is_some() {
                    let proof=nurbs_core::curve_decomposition_certificate::inspect_partition(curve,&parts,max_products-products)?;
                    products+=proof.products;
                    upper=upper.zip(proof.error_upper).map(|(a,b)|a.max(b));
                }
                pieces.extend(parts);
            }
            rings.push(pieces);
        }
        retained.push(rings);
    }
    if let Some(first) = retained.first() {
        if retained.iter().any(|station| {
            station.len() != first.len()
                || station.iter().zip(first).any(|(a, b)| {
                    a.len() != b.len()
                        || a.iter().zip(b).any(|(a, b)| {
                            a.degree != b.degree
                                || a.knots != b.knots
                                || a.weights != b.weights
                                || a.periodic != b.periodic
                        })
                })
        }) {
            upper = None;
        }
    } else {
        upper = None;
    }
    Ok((retained, upper, products))
}

#[cfg(test)]
#[path = "tests/rational_loft_body_partition_tests.rs"]
mod body_partition_tests;

pub fn progressive_profile_body_with_evidence(
    loops: &[Vec<Curve>],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    affine: Option<(&Curve, &Curve)>,
    frames: Option<(&Curve, &Curve)>,
    guidance: Option<(&Curve, Option<(usize, f64)>)>,
    options: nurbs_core::progressive_sweep::Options,
) -> Result<ProgressiveBodyEvidence> {
    progressive_profile_body_with_evidence_and_correction(
        loops, path, scale, twist, affine, frames, guidance, options, None,
    )
}
pub fn progressive_profile_body_with_evidence_and_correction(
    loops: &[Vec<Curve>],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    affine: Option<(&Curve, &Curve)>,
    frames: Option<(&Curve, &Curve)>,
    guidance: Option<(&Curve, Option<(usize, f64)>)>,
    options: nurbs_core::progressive_sweep::Options,
    correction: Option<EndpointCapCorrection>,
) -> Result<ProgressiveBodyEvidence> {
    progressive_profile_body_with_rmf_policy(
        loops, path, scale, twist, affine, frames, guidance, options, correction, None,
    )
}

/// Optional bounded original spatial RMF proof; existing callers retain their policy.
pub fn progressive_profile_body_with_rmf_policy(
    loops: &[Vec<Curve>],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    affine: Option<(&Curve, &Curve)>,
    frames: Option<(&Curve, &Curve)>,
    guidance: Option<(&Curve, Option<(usize, f64)>)>,
    options: nurbs_core::progressive_sweep::Options,
    correction: Option<EndpointCapCorrection>,
    rmf_policy: Option<(usize, usize, usize)>,
) -> Result<ProgressiveBodyEvidence> {
    progressive_profile_body_with_rmf_policy_and_corrections(
        loops, path, scale, twist, affine, frames, guidance, options, correction, None, rmf_policy,
    )
}

/// Explicit same-basis circle repair applies only to these retained ruled walls.
/// Its displacement is charged against the original source, including filled caps.
pub fn progressive_profile_body_with_rmf_policy_and_corrections(
    loops: &[Vec<Curve>],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    affine: Option<(&Curve, &Curve)>,
    frames: Option<(&Curve, &Curve)>,
    guidance: Option<(&Curve, Option<(usize, f64)>)>,
    options: nurbs_core::progressive_sweep::Options,
    correction: Option<EndpointCapCorrection>,
    profile_correction: Option<EndpointCapCorrection>,
    rmf_policy: Option<(usize, usize, usize)>,
) -> Result<ProgressiveBodyEvidence> {
    progressive_profile_body_internal(
        loops,
        path,
        scale,
        twist,
        affine,
        frames,
        guidance,
        options,
        correction,
        profile_correction,
        rmf_policy,
        None,
        false,
    )
}

/// Source-owned progressive body with complete radial annular end caps.
/// Legacy constructors keep their representation; this explicit mode shares
/// cap construction/filling work and preserves physical cap groups.
pub fn progressive_profile_body_with_annular_caps(
    loops: &[Vec<Curve>],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    affine: Option<(&Curve, &Curve)>,
    frames: Option<(&Curve, &Curve)>,
    guidance: Option<(&Curve, Option<(usize, f64)>)>,
    options: nurbs_core::progressive_sweep::Options,
    correction: Option<EndpointCapCorrection>,
    profile_correction: Option<EndpointCapCorrection>,
    rmf_policy: Option<(usize, usize, usize)>,
    caps: crate::retained_annular_cap::reference_filling::Limits,
) -> Result<AnnularProgressiveBody> {
    let body = progressive_profile_body_internal(
        loops,
        path,
        scale,
        twist,
        affine,
        frames,
        guidance,
        options,
        correction,
        profile_correction,
        rmf_policy,
        Some(caps),
        false,
    )?;
    Ok(annular_caps::Boundary::new(body))
}

/// Native selection preserves exact legacy caps. A failed terminal coedge
/// probe and annular construction/filling share one exact-work allowance.
pub fn progressive_profile_body_with_automatic_caps(
    loops: &[Vec<Curve>],
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    affine: Option<(&Curve, &Curve)>,
    frames: Option<(&Curve, &Curve)>,
    guidance: Option<(&Curve, Option<(usize, f64)>)>,
    options: nurbs_core::progressive_sweep::Options,
    correction: Option<EndpointCapCorrection>,
    profile_correction: Option<EndpointCapCorrection>,
    rmf_policy: Option<(usize, usize, usize)>,
    caps: crate::retained_annular_cap::reference_filling::Limits,
) -> Result<AnnularProgressiveBody> {
    let body = progressive_profile_body_internal(
        loops,
        path,
        scale,
        twist,
        affine,
        frames,
        guidance,
        options,
        correction,
        profile_correction,
        rmf_policy,
        Some(caps),
        true,
    )?;
    Ok(annular_caps::Boundary::new(body))
}

#[cfg(test)]
#[path = "tests/rational_loft_retained_tests.rs"]
mod retained_tests;

#[cfg(test)]
#[path = "tests/rational_loft_cap_certificate_tests.rs"]
mod cap_certificate_tests;

#[cfg(test)]
#[path = "tests/rational_loft_supplied_multispan_tests.rs"]
mod supplied_multispan_tests;

#[cfg(test)]
#[path = "tests/rational_loft_nonuniform_wall_regression.rs"]
mod nonuniform_wall_regression;

#[cfg(test)]
#[path = "tests/rational_loft_closed_planar_rmf_boundary_tests.rs"]
mod closed_planar_rmf_boundary_tests;

#[path = "rational_loft/owned_progressive_body.rs"]
mod owned_progressive_body;
use owned_progressive_body::progressive_profile_body_internal;
