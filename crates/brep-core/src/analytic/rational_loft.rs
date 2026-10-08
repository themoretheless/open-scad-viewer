//! Retained rational section loft with authored incidence and audited cap trims.
//! A closed manifold model is not a proof of global embedding/self-separation.
use super::*;
mod smooth_stations;
pub use smooth_stations::{SmoothStationWalls,smooth_station_walls};
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
fn certify_cap_chart(surface: &Surface, max_cells: usize, max_spans: usize) -> Result<()> {
    if max_cells == 0 || max_spans == 0 {
        return Err(err("Cap chart certificate budget exhausted"));
    }
    if !nurbs_core::surface_regularity::inspect(surface, max_cells)?.spanwise_regular {
        return Err(err("Cap chart regularity unproved"));
    }
    let injectivity = nurbs_core::surface_injectivity::certify(surface, max_spans)?;
    if !injectivity.proven {
        return Err(err(format!("Cap chart injectivity unproved: {}", injectivity.reason)));
    }
    Ok(())
}
fn cap(loops: &[Vec<Curve>]) -> Result<Cap> {
    cap_with_boundary_budget(loops, 1e-9, 100000)
}
fn cap_with_boundary_budget(loops: &[Vec<Curve>], tolerance: f64, max_products: usize) -> Result<Cap> {
    if let Some(cap)=coordinate_cap(loops,tolerance,max_products)? {return Ok(cap);}
    let origin = point(&loops[0][0], true);
    let u = unit(minus(point(&loops[0][0], false), origin))?;
    let candidate = loops[0]
        .iter()
        .flat_map(|c| &c.control_points)
        .map(|p| cross(u, minus([p[0], p[1], p[2]], origin)))
        .max_by(|a, b| dot(*a, *a).total_cmp(&dot(*b, *b)))
        .unwrap();
    let n = unit(candidate)?;
    let v = cross(n, u);
    let mut projected = loops.to_vec();
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for p in projected
        .iter_mut()
        .flatten()
        .flat_map(|c| &mut c.control_points)
    {
        let delta = minus([p[0], p[1], p[2]], origin);
        if dot(delta, n).abs() > 1e-9 {
            return Err(err("Cap section controls must be coplanar"));
        }
        *p = vec![dot(delta, u), dot(delta, v)];
        for k in 0..2 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }
    let width = [max[0] - min[0], max[1] - min[1]];
    if width.iter().any(|x| !x.is_finite() || *x <= 1e-8) {
        return Err(err("Collapsed cap bounds"));
    }
    for p in projected
        .iter_mut()
        .flatten()
        .flat_map(|c| &mut c.control_points)
    {
        for k in 0..2 {
            p[k] = (p[k] - min[k]) / width[k];
        }
    }
    let audit = nurbs_core::trim_region_audit::inspect(&projected, 1e-10, 100000, 100000, 1000000)?;
    if audit.valid != Some(true) {
        return Err(err(format!(
            "Cap trim region unresolved or invalid: {}",
            audit.reason
        )));
    }
    let world = |a: f64, b: f64| {
        (0..3)
            .map(|k| origin[k] + a * u[k] + b * v[k])
            .collect::<Vec<_>>()
    };
    let mut result = Cap {
        normal: n,
        surface: Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![world(min[0], min[1]), world(min[0], max[1])],
                vec![world(max[0], min[1]), world(max[0], max[1])],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        },
        loops: projected,
    };
    if audit.winding[0] == Some(-1) {
        result.normal = result.normal.map(|x| -x);
        for row in &mut result.surface.control_points {
            row.reverse();
        }
        for p in result
            .loops
            .iter_mut()
            .flatten()
            .flat_map(|c| &mut c.control_points)
        {
            p[1] = 1. - p[1];
        }
    }
    certify_cap_chart(&result.surface, 1000, 1)?;
    // Verify the actual stored surface/UV data after orientation changes.
    // Coplanar controls alone do not bound projection and reconstruction error.
    let mut products = 0;
    for (world_loop, uv_loop) in loops.iter().zip(&result.loops) {
        for (world_curve, uv_curve) in world_loop.iter().zip(uv_loop) {
            let report = nurbs_core::sweep_cap_boundary::inspect(
                &result.surface, world_curve, uv_curve, tolerance, max_products - products,
            )?;
            products += report.products;
            if !report.within_budget {
                return Err(err(format!("Cap boundary composition unproved: {}",
                    report.reason.unwrap_or("unresolved certificate"))));
            }
        }
    }
    Ok(result)
}

/// Keep natural UV coordinates on a coordinate-plane cap. No rounded inverse
/// frame or UV normalization is needed, and orientation swaps coordinates only.
fn coordinate_cap(loops:&[Vec<Curve>],tolerance:f64,max_products:usize)->Result<Option<Cap>> {
    let first=&loops[0][0].control_points[0];
    // Prefer natural coordinate graphs over rounded orthonormal projection.
    // These are candidates only: exact composition below is mandatory.
    let mut graph=None;
    'search: for axis in 0..3 {
        let axes:Vec<_>=(0..3).filter(|k|*k!=axis).collect();
        for a in [0.,0.5,-0.5,1.,-1.,2.,-2.] {
            for b in [0.,0.5,-0.5,1.,-1.,2.,-2.] {
                let offset=first[axis]-a*first[axes[0]]-b*first[axes[1]];
                if loops.iter().flatten().flat_map(|c|&c.control_points).all(|p|p[axis]==offset+a*p[axes[0]]+b*p[axes[1]]) {
                    graph=Some((axis,a,b,offset));break 'search;
                }
            }
        }
    }
    let Some((axis,a,b,offset))=graph else {return Ok(None);};
    let axes:Vec<_>=(0..3).filter(|k|*k!=axis).collect();
    let mut projected=loops.to_vec();
    let mut min=[f64::INFINITY;2];let mut max=[f64::NEG_INFINITY;2];
    for p in projected.iter_mut().flatten().flat_map(|c|&mut c.control_points) {
        *p=vec![p[axes[0]],p[axes[1]]];
        for k in 0..2 {min[k]=min[k].min(p[k]);max[k]=max[k].max(p[k]);}
    }
    if (0..2).any(|k|!max[k].is_finite() || max[k]-min[k]<=1e-8) {return Err(err("Collapsed cap bounds"));}
    let audit=nurbs_core::trim_region_audit::inspect(&projected,1e-10,100000,100000,1000000)?;
    if audit.valid!=Some(true) {return Err(err(format!("Cap trim region unresolved or invalid: {}",audit.reason)));}
    let world=|u:f64,v:f64|{let mut p=first.clone();p[axes[0]]=u;p[axes[1]]=v;p[axis]=offset+a*u+b*v;p};
    let mut normal=[0.;3];normal[axis]=1.;normal[axes[0]]=-a;normal[axes[1]]=-b;
    if axis==1 {normal=normal.map(|x|-x);}
    normal=unit(normal)?;
    let mut result=Cap{normal,surface:Surface{degree_u:1,degree_v:1,
        knots_u:vec![min[0],min[0],max[0],max[0]],knots_v:vec![min[1],min[1],max[1],max[1]],
        control_points:vec![vec![world(min[0],min[1]),world(min[0],max[1])],vec![world(max[0],min[1]),world(max[0],max[1])]],
        weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false},loops:projected};
    if audit.winding[0]==Some(-1) {
        result.normal=result.normal.map(|x|-x);
        std::mem::swap(&mut result.surface.knots_u,&mut result.surface.knots_v);
        result.surface.control_points=(0..2).map(|i|(0..2).map(|j|result.surface.control_points[j][i].clone()).collect()).collect();
        for p in result.loops.iter_mut().flatten().flat_map(|c|&mut c.control_points) {p.swap(0,1);}
    }
    certify_cap_chart(&result.surface,1000,1)?;
    let mut products=0;let mut work=0;
    for (world_loop,uv_loop) in loops.iter().zip(&result.loops) {for (world,uv) in world_loop.iter().zip(uv_loop) {
        let report=nurbs_core::sweep_cap_boundary::inspect(&result.surface,world,uv,tolerance,max_products-products)?;
        products+=report.products;
        if !report.within_budget {return Err(err("Cap boundary composition unproved"));}
        let exact=nurbs_core::curve_surface_agreement::verify_exact(world,uv,&result.surface,false,1000000-work)?;
        let Some(exact)=exact else {return Ok(None);};
        work+=exact.work_used;
        if exact.outcome!=cad_predicates::BezierIdentity::Equal {return Ok(None);}
    }}
    Ok(Some(result))
}

/// Preserve stored coefficients when every active span already is Bezier.
/// Renaming each knot domain to [0,1] is an exact parameter correspondence;
/// there is no homogeneous re-evaluation or weight division on this path.
fn retained_bezier_pieces(curve: &Curve) -> Result<Vec<Curve>> {
    let [a, b] = curve.domain();
    let segmented = curve
        .knots
        .iter()
        .copied()
        .filter(|k| *k > a && *k < b)
        .all(|k| curve.knots.iter().filter(|v| **v == k).count() >= curve.degree);
    let clamped = curve.knots.iter().filter(|k| **k == a).count() == curve.degree + 1
        && curve.knots.iter().filter(|k| **k == b).count() == curve.degree + 1;
    if segmented && clamped && !curve.periodic {
        return Ok((curve.degree..curve.control_points.len())
            .filter(|i| curve.knots[*i] < curve.knots[*i + 1])
            .map(|i| Curve {
                degree: curve.degree,
                knots: std::iter::repeat_n(0., curve.degree + 1)
                    .chain(std::iter::repeat_n(1., curve.degree + 1))
                    .collect(),
                control_points: curve.control_points[i - curve.degree..=i].to_vec(),
                weights: curve.weights[i - curve.degree..=i].to_vec(),
                periodic: false,
            })
            .collect());
    }
    curve
        .decompose()?
        .into_iter()
        .map(|span| {
            let mut c = span.definition().clone();
            c.knots = std::iter::repeat_n(0., c.degree + 1)
                .chain(std::iter::repeat_n(1., c.degree + 1))
                .collect();
            Ok(c)
        })
        .collect()
}

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

fn section_loft(
    sections: &[Vec<Vec<Curve>>],
    parameters: Option<&[f64]>,
    sides: Option<&[Vec<Surface>]>,
    periodic: bool,
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
                || a.iter()
                    .zip(b)
                    .any(|(a, b)| a.degree != b.degree || a.control_points.len() != b.control_points.len())
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
    let lower = cap(&prepared[0])?;
    let upper = if periodic {
        None
    } else {
        Some(cap(prepared.last().unwrap())?)
    };
    let travel = minus(
        point(&prepared[1][0][0], true),
        point(&prepared[0][0][0], true),
    );
    let direction = dot(travel, lower.normal);
    if !direction.is_finite() || direction.abs() <= 1e-10 {
        return Err(err("Initial loft ruling must cross the cap plane"));
    }

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
    let caps = if let Some(upper) = upper {
        vec![(0, lower), (prepared.len() - 1, upper)]
    } else {
        Vec::new()
    };
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
    use nurbs_core::progressive_sweep::{Sweep, approximate_profiles};
    if options.max_sections > 1025
        || loops.is_empty()
        || loops.len() > 16
        || loops.iter().any(Vec::is_empty)
    {
        return Err(err(
            "Progressive body needs 1..16 nonempty loops and at most1025 sections",
        ));
    }
    let profiles = loops.iter().flatten().cloned().collect::<Vec<_>>();
    let spans = profiles
        .iter()
        .map(|c| c.decompose().map(|s| s.len()))
        .collect::<nurbs_core::Result<Vec<_>>>()?
        .into_iter()
        .sum::<usize>();
    if spans == 0 || spans > 64 {
        return Err(err("Progressive body exceeds64 section spans"));
    }
    let cap_faces = if nurbs_core::progressive_sweep::path_is_closed(path)? {
        0
    } else {
        2
    };
    let options = nurbs_core::progressive_sweep::Options {
        max_sections: options
            .max_sections
            .min((MAX_FACES - cap_faces) / spans + 1),
        ..options
    };
    let approximation = if let Some((guide, anchor)) = guidance {
        let (axes, center) = affine.unwrap();
        if let Some((index, parameter)) = anchor {
            nurbs_core::progressive_sweep::approximate_contact_profiles(
                &profiles, path, scale, twist, guide, index, parameter, axes, center, options,
            )?
        } else {
            nurbs_core::progressive_sweep::approximate_guided_profiles(
                &profiles, path, scale, twist, guide, axes, center, options,
            )?
        }
    } else if let Some((longitudinal, transverse)) = frames {
        let (axes, center) = affine.unwrap();
        nurbs_core::progressive_sweep::approximate_authored_profiles(
            &profiles,
            path,
            scale,
            twist,
            longitudinal,
            transverse,
            axes,
            center,
            options,
        )?
    } else if let Some((axes, center)) = affine {
        nurbs_core::progressive_sweep::approximate_affine_profiles(
            &profiles, path, scale, twist, axes, center, options,
        )?
    } else {
        approximate_profiles(&profiles, path, scale, twist, options)?
    };
    let report = approximation.levels.last().unwrap();
    if !report.accepted {
        return Err(err("Progressive body misses sampled refinement budget"));
    }
    let transported = if let Some((guide, anchor)) = guidance {
        let sweep =
            nurbs_core::progressive_sweep::MultiSweep::new(&profiles, path, scale, twist, options)?;
        let sweep = if let Some((index, parameter)) = anchor {
            sweep.with_contact_guide(guide, index, parameter)?
        } else {
            sweep.with_orientation_guide(guide)?
        };
        let (axes, center) = affine.unwrap();
        sweep
            .with_affine_laws(axes, center)?
            .sections_at(report.sections)?
    } else {
        profiles
            .iter()
            .map(|p| {
                let sweep = Sweep::new(p, path, scale, twist, options)?;
                let sweep = if let Some((axes, center)) = affine {
                    sweep.with_affine_laws(axes, center)?
                } else {
                    sweep
                };
                let sweep = if let Some((longitudinal, transverse)) = frames {
                    sweep.with_frame_laws(longitudinal, transverse)?
                } else {
                    sweep
                };
                sweep.sections_at(report.sections)
            })
            .collect::<nurbs_core::Result<Vec<_>>>()?
    };
    let sections = (0..report.sections)
        .map(|station| {
            let mut first = 0;
            loops
                .iter()
                .map(|ring| {
                    let section = transported[first..first + ring.len()]
                        .iter()
                        .map(|p| p[station].clone())
                        .collect::<Vec<_>>();
                    first += ring.len();
                    section
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let model = if report.closed_path {
        periodic_section_loft(&sections)?
    } else {
        rational_section_loft(&sections)?
    };
    Ok((model, approximation))
}

#[cfg(test)]
#[path="tests/rational_loft_retained_tests.rs"]
mod retained_tests;

#[cfg(test)]
#[path="tests/rational_loft_cap_certificate_tests.rs"]
mod cap_certificate_tests;

#[cfg(test)]
#[path="tests/rational_loft_supplied_multispan_tests.rs"]
mod supplied_multispan_tests;

#[cfg(test)]
#[path="tests/rational_loft_nonuniform_wall_regression.rs"]
mod nonuniform_wall_regression;
