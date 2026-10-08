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
    // Derive additional dyadic graph candidates from the stored control hull.
    // Floating fitting selects candidates only; the exact coedge identity gate
    // below remains mandatory and rejects rounded or nonplanar constructions.
    if graph.is_none() {
        let points: Vec<_> = loops.iter().flatten().flat_map(|c| &c.control_points).collect();
        let delta = |p: &Vec<f64>| [p[0]-first[0],p[1]-first[1],p[2]-first[2]];
        let norm = |v: [f64;3]| v.iter().map(|x| x*x).sum::<f64>();
        let direction = points.iter().map(|p| delta(p))
            .filter(|v| norm(*v).is_finite())
            .max_by(|a,b| norm(*a).total_cmp(&norm(*b)));
        if let Some(d) = direction {
            let normal = points.iter().map(|p| {
                let v=delta(p);
                [d[1]*v[2]-d[2]*v[1],d[2]*v[0]-d[0]*v[2],d[0]*v[1]-d[1]*v[0]]
            }).filter(|v| norm(*v).is_finite())
                .max_by(|a,b| norm(*a).total_cmp(&norm(*b)));
            if let Some(n) = normal {
                let mut graph_axes=[0,1,2];
                graph_axes.sort_by(|a,b| n[*b].abs().total_cmp(&n[*a].abs()));
                'derived: for axis in graph_axes {
                    if n[axis]==0. {continue;}
                    let axes:Vec<_>=(0..3).filter(|k|*k!=axis).collect();
                    let coefficients=[-n[axes[0]]/n[axis],-n[axes[1]]/n[axis]];
                    if !coefficients.iter().all(|x|x.is_finite()) {continue;}
                    for bits in 0..=52 {
                        let scale=2f64.powi(bits);
                        let a=(coefficients[0]*scale).round()/scale;
                        let b=(coefficients[1]*scale).round()/scale;
                        let offset=first[axis]-a*first[axes[0]]-b*first[axes[1]];
                        if [a,b,offset].iter().all(|x|x.is_finite()) && points.iter().all(|p|
                            p[axis]==offset+a*p[axes[0]]+b*p[axes[1]]) {
                            graph=Some((axis,a,b,offset));break 'derived;
                        }
                    }
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
/// corresponding spans retain degree and weights. Planar end caps must pass
/// interval trim-region audit. Side regularity/global self-intersections remain
/// unproven and are reported by the model's separate solid audit.
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
                    .any(|(a, b)| a.degree != b.degree || a.weights != b.weights)
            {
                return Err(err(
                    "Rational section span correspondence/weights must match",
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
    pub retained_walls:crate::sweep_retained_walls::Report,
    pub boundary_error_upper:Option<f64>,
    pub boundary_error_within_budget:Option<bool>,
    pub body_decomposition_error_upper:Option<f64>,
    pub body_decomposition_products:usize,
}

/// Reproduce the constructor's retained sections and certify their complete
/// correspondence to each original transported section. The error can be
/// interpolated along a wall only when rational bases agree at every station.
fn retained_section_partition(sections:&[Vec<Vec<Curve>>],max_products:usize)
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
    if let Some(first)=retained.first() {
        if retained.iter().any(|station|station.len()!=first.len() || station.iter().zip(first).any(|(a,b)|
            a.len()!=b.len() || a.iter().zip(b).any(|(a,b)|a.degree!=b.degree || a.knots!=b.knots || a.weights!=b.weights || a.periodic!=b.periodic))) {
            upper=None;
        }
    } else {upper=None;}
    Ok((retained,upper,products))
}

#[cfg(test)]
mod body_partition_tests {
    use super::*;
    fn profile()->Curve {Curve {degree:2,knots:vec![0.,0.,0.,0.5,1.,1.,1.],
        control_points:vec![vec![0.,0.,0.],vec![0.5,-0.25,0.],vec![1.5,-0.25,0.],vec![2.,0.,0.]],
        weights:vec![1.,0.75,1.25,1.],periodic:false}}
    #[test]
    fn complete_body_partition_bounds_extraction_and_refuses_partial_or_variable_basis(){
        let a=profile();let mut b=a.clone();for p in &mut b.control_points {p[2]=10.;}
        let sections=vec![vec![vec![a]],vec![vec![b]]];
        let (retained,bound,products)=retained_section_partition(&sections,36).unwrap();
        assert_eq!(products,36);assert!(bound.unwrap()>0. && bound.unwrap()<1e-10);
        assert_eq!(retained[0][0].len(),2);
        let (_,partial,products)=retained_section_partition(&sections,35).unwrap();
        assert!(partial.is_none());assert_eq!(products,27);
        let mut changed=sections.clone();changed[1][0][0].weights[1]=0.8;
        assert!(retained_section_partition(&changed,36).unwrap().1.is_none());
        let mut source=sections.clone();source[1][0][0].control_points[1][2]+=0.125;
        assert_ne!(retained_section_partition(&source,36).unwrap().0,retained);
    }
    #[test]
    fn rational_line_arc_length_body_composes_actual_walls_and_filled_caps(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing}};
        let corners=[[0.,0.,0.],[1.,0.,0.],[1.,1.,0.],[0.,1.,0.]];
        let loops=vec![(0..4).map(|i|line(corners[i],corners[(i+1)%4]).unwrap()).collect()];
        let path=Curve {weights:vec![1.,4.],..line([0.;3],[0.,0.,10.]).unwrap()};
        let scale=nurbs_core::progressive_sweep::constant_vector_law([1.,0.,0.]).unwrap();
        let twist=nurbs_core::progressive_sweep::constant_vector_law([0.;3]).unwrap();
        for orientation in [Orientation::Fixed,Orientation::RotationMinimizing] {
            let result=progressive_profile_body_with_evidence(&loops,&path,&scale,&twist,None,None,None,
                Options {normal:[1.,0.,0.],orientation,
                    spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                    initial_sections:3,max_sections:3,max_deviation:0.01}).unwrap();
            assert!(result.approximation.levels.last().unwrap().continuous_bound);
            assert!(result.retained_walls.certified);
            assert!(result.retained_caps.as_ref().unwrap().exact);
            assert!(result.filled_cap_error_upper.is_some());
            assert_eq!(result.boundary_error_within_budget,Some(true));
            assert!(result.boundary_error_upper.unwrap()<=0.01);
        }
    }
    #[test]
    fn curved_arc_length_frame_modes_hollow_body_composes_walls_caps_and_source_domain(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[0.,0.,0.],[2.,0.,0.],[2.,2.,0.],[0.,2.,0.]]),
            ring([[0.5,0.5,0.],[0.5,1.5,0.],[1.5,1.5,0.],[1.5,0.5,0.]])];
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        for (orientation,authored) in [(Orientation::Fixed,false),(Orientation::Fixed,true),(Orientation::FixedNormal,false)] {
        let budget=if orientation==Orientation::FixedNormal {2.}else{0.2};
        let result=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),if authored {Some((&axis,&normal))}else{None},None,
            Options {normal:[1.,0.,0.],orientation,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                initial_sections:3,max_sections:if orientation==Orientation::FixedNormal {17}else{9},max_deviation:budget},if orientation==Orientation::FixedNormal {Some(EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})}else{None}).unwrap();
        assert!(result.approximation.levels.last().unwrap().continuous_bound,"{:?}",result.approximation.levels);
        assert!(result.retained_walls.certified);assert!(result.retained_caps.as_ref().unwrap().exact,"{:?}",result.retained_caps);
        assert!(result.filled_cap_error_upper.is_some());
        assert_eq!(result.boundary_error_within_budget,Some(true));
        assert!(result.boundary_error_upper.unwrap()<=budget);
        assert!(result.model.faces.iter().rev().take(2).all(|face|face.holes.len()==1));
        }
    }
    #[test]
    fn small_fixed_normal_arc_length_hollow_body_requires_actual_native_volume_admission(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[0.,0.,0.],[0.1,0.,0.],[0.1,0.1,0.],[0.,0.1,0.]]),
            ring([[0.025,0.025,0.],[0.025,0.075,0.],[0.075,0.075,0.],[0.075,0.025,0.]])];
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        for (orientation,authored) in [(Orientation::FixedNormal,false)] {
        let budget=if orientation==Orientation::FixedNormal {2.}else{0.2};
        let result=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),if authored {Some((&axis,&normal))}else{None},None,
            Options {normal:[1.,0.,0.],orientation,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                initial_sections:3,max_sections:if orientation==Orientation::FixedNormal {17}else{9},max_deviation:budget},if orientation==Orientation::FixedNormal {Some(EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})}else{None}).unwrap();
        assert!(result.approximation.levels.last().unwrap().continuous_bound,"{:?}",result.approximation.levels);
        assert!(result.retained_walls.certified);assert!(result.retained_caps.as_ref().unwrap().exact,"{:?}",result.retained_caps);
        assert!(result.filled_cap_error_upper.is_some());
        let correction=result.cap_correction_error_upper.unwrap();assert!(correction>0.&&correction<=1e-9);
        assert!(result.boundary_error_upper.unwrap()>=correction);
        assert_eq!(result.boundary_error_within_budget,Some(true));
        assert!(result.boundary_error_upper.unwrap()<=budget);
        assert!(result.model.faces.iter().rev().take(2).all(|face|face.holes.len()==1));
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,
            crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000},
            20000,&[result.model.faces.len()-2,result.model.faces.len()-1],crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,
                max_chart_cells:100000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        assert!(volume.proven,"boundary={}, nesting={:?}, outward={:?}",volume.boundary.proven,volume.nesting.as_ref().map(|n|n.roles_consistent),volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        }
    }
    #[test]
    fn small_planar_rmf_arc_length_hollow_body_requires_actual_native_volume_admission(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[0.,0.,0.],[0.1,0.,0.],[0.1,0.1,0.],[0.,0.1,0.]]),
            ring([[0.025,0.025,0.],[0.025,0.075,0.],[0.075,0.075,0.],[0.075,0.025,0.]])];
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        for (orientation,authored) in [(Orientation::RotationMinimizing,false)] {
        let budget=if orientation==Orientation::RotationMinimizing {2.}else{0.2};
        let result=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),if authored {Some((&axis,&normal))}else{None},None,
            Options {normal:[1.,0.,0.],orientation,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                initial_sections:3,max_sections:if orientation==Orientation::RotationMinimizing {17}else{9},max_deviation:budget},if orientation==Orientation::RotationMinimizing {Some(EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})}else{None}).unwrap();
        assert!(result.approximation.levels.last().unwrap().continuous_bound,"{:?}",result.approximation.levels);
        assert!(result.retained_walls.certified);assert!(result.retained_caps.as_ref().unwrap().exact,"{:?}",result.retained_caps);
        assert!(result.filled_cap_error_upper.is_some());
        let correction=result.cap_correction_error_upper.unwrap();assert!(correction>0.&&correction<=1e-9);
        assert!(result.boundary_error_upper.unwrap()>=correction);
        assert_eq!(result.boundary_error_within_budget,Some(true));
        assert!(result.boundary_error_upper.unwrap()<=budget);
        assert!(result.model.faces.iter().rev().take(2).all(|face|face.holes.len()==1));
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,
            crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000},
            20000,&[result.model.faces.len()-2,result.model.faces.len()-1],crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,
                max_chart_cells:100000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        assert!(volume.proven,"boundary={}, nesting={:?}, outward={:?}",volume.boundary.proven,volume.nesting.as_ref().map(|n|n.roles_consistent),volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        }
    }
    #[test]
    fn small_nonaxial_planar_rmf_arc_length_hollow_body_requires_native_volume_admission(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let mut loops:Vec<Vec<Curve>>=vec![ring([[0.,0.,0.],[0.1,0.,0.],[0.1,0.1,0.],[0.,0.1,0.]]),
            ring([[0.025,0.025,0.],[0.025,0.075,0.],[0.075,0.075,0.],[0.075,0.025,0.]])];
        for ring in &mut loops {for curve in ring {for pole in &mut curve.control_points {*pole=vec![pole[0],pole[0],pole[1]];}}}
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.5,-0.5,0.],vec![1.,-1.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        for (orientation,authored) in [(Orientation::RotationMinimizing,false)] {
        let budget=if orientation==Orientation::RotationMinimizing {2.}else{0.2};
        let result=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),if authored {Some((&axis,&normal))}else{None},None,
            Options {normal:[1.,1.,0.],orientation,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                initial_sections:3,max_sections:if orientation==Orientation::RotationMinimizing {17}else{9},max_deviation:budget},if orientation==Orientation::RotationMinimizing {Some(EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})}else{None}).unwrap();
        assert!(result.approximation.levels.last().unwrap().continuous_bound,"{:?}",result.approximation.levels);
        assert!(result.retained_walls.certified);assert!(result.retained_caps.as_ref().unwrap().exact,"{:?}",result.retained_caps);
        assert!(result.filled_cap_error_upper.is_some());
        let correction=result.cap_correction_error_upper.unwrap();assert!(correction>0.&&correction<=1e-9);
        assert!(result.boundary_error_upper.unwrap()>=correction);
        assert_eq!(result.boundary_error_within_budget,Some(true));
        assert!(result.boundary_error_upper.unwrap()<=budget);
        eprintln!("nonaxial RMF body boundary={:?}, filled_caps={:?}, correction={correction}, faces={}",result.boundary_error_upper,result.filled_cap_error_upper,result.model.faces.len());
        assert!(result.model.faces.iter().rev().take(2).all(|face|face.holes.len()==1));
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,
            crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000},
            20000,&[result.model.faces.len()-2,result.model.faces.len()-1],crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,
                max_chart_cells:100000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        assert!(volume.proven,"boundary={}, nesting={:?}, outward={:?}",volume.boundary.proven,volume.nesting.as_ref().map(|n|n.roles_consistent),volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        }
    }
    #[test]
    fn small_guided_curved_arc_length_hollow_body_requires_actual_native_volume_admission(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[0.,0.,0.],[0.1,0.,0.],[0.1,0.1,0.],[0.,0.1,0.]]),
            ring([[0.025,0.025,0.],[0.025,0.075,0.],[0.075,0.075,0.],[0.075,0.025,0.]])];
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
        let guide=Curve {degree:2,knots:vec![-3.,-3.,-3.,7.,7.,7.],control_points:vec![vec![1.,0.,0.],vec![1.,0.,0.5],vec![1.,1.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        for (orientation,authored) in [(Orientation::RotationMinimizing,false)] {
        let budget=if orientation==Orientation::RotationMinimizing {2.}else{0.2};
        let result=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),if authored {Some((&axis,&normal))}else{None},Some((&guide,None)),
            Options {normal:[1.,0.,0.],orientation,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                initial_sections:3,max_sections:if orientation==Orientation::RotationMinimizing {17}else{9},max_deviation:budget},if orientation==Orientation::RotationMinimizing {Some(EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})}else{None}).unwrap();
        assert!(result.approximation.levels.last().unwrap().continuous_bound,"{:?}",result.approximation.levels);
        assert!(result.retained_walls.certified);assert!(result.retained_caps.as_ref().unwrap().exact,"{:?}",result.retained_caps);
        assert!(result.filled_cap_error_upper.is_some());
        let correction=result.cap_correction_error_upper.unwrap();assert!(correction>0.&&correction<=1e-9);
        assert!(result.boundary_error_upper.unwrap()>=correction);
        assert_eq!(result.boundary_error_within_budget,Some(true));
        assert!(result.boundary_error_upper.unwrap()<=budget);
        assert!(result.model.faces.iter().rev().take(2).all(|face|face.holes.len()==1));
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,
            crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000},
            20000,&[result.model.faces.len()-2,result.model.faces.len()-1],crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,
                max_chart_cells:100000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        assert!(volume.proven,"boundary={}, nesting={:?}, outward={:?}",volume.boundary.proven,volume.nesting.as_ref().map(|n|n.roles_consistent),volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        }
    }
    #[test]
    fn small_contact_curved_arc_length_hollow_body_requires_actual_native_volume_admission(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[1.,0.,0.],[1.,0.1,0.],[0.9,0.1,0.],[0.9,0.,0.]]),
            ring([[0.925,0.025,0.],[0.925,0.075,0.],[0.975,0.075,0.],[0.975,0.025,0.]])];
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
        let guide=Curve {degree:2,knots:vec![-3.,-3.,-3.,7.,7.,7.],control_points:vec![vec![1.,0.,0.],vec![1.,0.,0.5],vec![1.,1.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        for (orientation,authored) in [(Orientation::RotationMinimizing,false)] {
        let budget=if orientation==Orientation::RotationMinimizing {2.}else{0.2};
        let result=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),if authored {Some((&axis,&normal))}else{None},Some((&guide,Some((0,0.)))),
            Options {normal:[1.,0.,0.],orientation,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                initial_sections:3,max_sections:if orientation==Orientation::RotationMinimizing {17}else{9},max_deviation:budget},if orientation==Orientation::RotationMinimizing {Some(EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})}else{None}).unwrap();
        assert!(result.approximation.levels.last().unwrap().continuous_bound,"{:?}",result.approximation.levels);
        assert!(result.retained_walls.certified);assert!(result.retained_caps.as_ref().unwrap().exact,"{:?}",result.retained_caps);
        assert!(result.filled_cap_error_upper.is_some());
        let correction=result.cap_correction_error_upper.unwrap();assert!(correction>0.&&correction<=1e-9);
        assert!(result.boundary_error_upper.unwrap()>=correction);
        assert_eq!(result.boundary_error_within_budget,Some(true),"wall={:?},filled={:?},boundary={:?},projection={:?}",result.approximation.levels.last().unwrap().continuous_error_upper,result.filled_cap_error_upper,result.boundary_error_upper,result.cap_projection.as_ref().and_then(|p|p.normal_dots));
        assert!(result.boundary_error_upper.unwrap()<=budget);
        assert!(result.model.faces.iter().rev().take(2).all(|face|face.holes.len()==1));
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,
            crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000},
            20000,&[result.model.faces.len()-2,result.model.faces.len()-1],crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,
                max_chart_cells:100000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        assert!(volume.proven,"boundary={}, nesting={:?}, outward={:?}",volume.boundary.proven,volume.nesting.as_ref().map(|n|n.roles_consistent),volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        }
    }
    #[test]
    fn closed_concentric_contact_hollow_body_requires_original_complete_boundary() {
        use nurbs_core::{primitives::circle,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let outer=circle([4.,0.,0.],[0.,1.,0.],0.25).unwrap();
        let hole=circle([4.,0.,0.],[0.,-1.,0.],0.125).unwrap();
        let path=circle([0.;3],[0.,0.,1.],4.).unwrap();
        let guide=circle([0.;3],[0.,0.,1.],4.25).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();
        let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([1.;3]).unwrap();
        let center=constant_vector_law([0.;3]).unwrap();
        for spacing in [Spacing::Parameter,Spacing::ArcLength {tolerance:0.001,max_cells:100000}] {
        let result=progressive_profile_body_with_evidence_and_correction(&[vec![outer.clone()],vec![hole.clone()]],&path,&scale,&twist,
            Some((&axes,&center)),None,Some((&guide,Some((0,0.)))),Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,
            spacing,initial_sections:17,max_sections:65,max_deviation:2.},None).unwrap();
        assert!(result.approximation.levels.last().unwrap().continuous_bound,"{:?}",result.approximation.levels);
        assert!(result.retained_walls.certified);
        assert!(result.retained_caps.is_none() && result.filled_cap_error_upper.is_none());
        assert_eq!(result.boundary_error_within_budget,Some(true));
        assert!(result.boundary_error_upper.unwrap()<=2.);
        assert_eq!(result.model.shells.len(),2);
        let limits=crate::volume_validity::Limits {
            boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                    pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
            nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
            orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000};
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,limits,20000,&[],
            crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,max_chart_cells:100000,
                max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        assert!(volume.proven,"boundary={}, nesting={:?}, outward={:?}",volume.boundary.proven,
            volume.nesting.as_ref().map(|n|n.roles_consistent),volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        assert_eq!(volume.nesting.as_ref().unwrap().parents,Some(vec![None,Some(0)]));
        assert_eq!(volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>(),vec![Some(true),Some(false)]);
        }
    }
    #[test]
    fn small_frenet_arc_length_hollow_body_requires_actual_native_volume_admission(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[0.,0.,0.],[0.,0.1,0.],[0.,0.1,0.1],[0.,0.,0.1]]),
            ring([[0.,0.025,0.025],[0.,0.025,0.075],[0.,0.075,0.075],[0.,0.075,0.025]])];
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.5,0.,0.],vec![1.,1.,0.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        for (orientation,authored) in [(Orientation::Frenet,false)] {
        let budget=if orientation==Orientation::Frenet {2.}else{0.2};
        let result=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),if authored {Some((&axis,&normal))}else{None},None,
            Options {normal:[1.,0.,0.],orientation,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                initial_sections:3,max_sections:if orientation==Orientation::Frenet {17}else{9},max_deviation:budget},if orientation==Orientation::Frenet {Some(EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})}else{None}).unwrap();
        assert!(result.approximation.levels.last().unwrap().continuous_bound,"{:?}",result.approximation.levels);
        assert!(result.retained_walls.certified);assert!(result.retained_caps.as_ref().unwrap().exact,"{:?}",result.retained_caps);
        assert!(result.filled_cap_error_upper.is_some());
        let correction=result.cap_correction_error_upper.unwrap();assert!(correction>0.&&correction<=1e-9);
        assert!(result.boundary_error_upper.unwrap()>=correction);
        assert_eq!(result.boundary_error_within_budget,Some(true));
        assert!(result.boundary_error_upper.unwrap()<=budget);
        assert!(result.model.faces.iter().rev().take(2).all(|face|face.holes.len()==1));
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,
            crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000},
            20000,&[result.model.faces.len()-2,result.model.faces.len()-1],crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,
                max_chart_cells:100000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        assert!(volume.proven,"boundary={}, nesting={:?}, outward={:?}",volume.boundary.proven,volume.nesting.as_ref().map(|n|n.roles_consistent),volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        }
    }
    #[test]
    fn fixed_normal_cap_correction_refuses_exhausted_work_and_excess_displacement(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing,constant_vector_law}};
        let ring=|points:[[f64;3];4]|(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect();
        let loops=vec![ring([[0.,0.,0.],[0.1,0.,0.],[0.1,0.1,0.],[0.,0.1,0.]]),
            ring([[0.025,0.025,0.],[0.025,0.075,0.],[0.075,0.075,0.],[0.075,0.025,0.]])];
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        for (quantum,max_work) in [(2_f64.powi(-40),0),(1.,1000000)] {
        let orientation=Orientation::FixedNormal;let authored=false;
        let budget=if orientation==Orientation::FixedNormal {2.}else{0.2};
        let outcome=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,Some((&axes,&center)),if authored {Some((&axis,&normal))}else{None},None,
            Options {normal:[1.,0.,0.],orientation,
                spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
                initial_sections:3,max_sections:if orientation==Orientation::FixedNormal {17}else{9},max_deviation:budget},if orientation==Orientation::FixedNormal {Some(EndpointCapCorrection {quantum,tolerance:1e-9,max_work})}else{None});
        let message=match outcome {Ok(_)=>panic!("Unproved correction was published"),Err(e)=>e.to_string()};
        assert!(message.contains("Progressive cap correction refused"),"{message}");
        }
    }

    #[test]
    fn progressive_body_binds_small_unsegmented_profile_to_actual_walls_and_caps(){
        use nurbs_core::{primitives::line,progressive_sweep::{Options,Orientation,Spacing}};
        let loops=vec![vec![profile(),line([2.,0.,0.],[2.,2.,0.]).unwrap(),
            line([2.,2.,0.],[0.,2.,0.]).unwrap(),line([0.,2.,0.],[0.,0.,0.]).unwrap()]];
        let path=line([0.;3],[0.,0.,10.]).unwrap();
        let mut scale=path.clone();scale.control_points=vec![vec![1.,0.,0.];2];
        let mut twist=path.clone();twist.control_points=vec![vec![0.;3];2];
        let result=progressive_profile_body_with_evidence(&loops,&path,&scale,&twist,None,None,None,
            Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,
                initial_sections:3,max_sections:3,max_deviation:0.01}).unwrap();
        assert_eq!(result.body_decomposition_products,54);
        assert!(result.body_decomposition_error_upper.unwrap()>0.);
        assert!(result.retained_walls.certified);
        assert!(result.retained_caps.as_ref().unwrap().exact);
        assert!(result.boundary_error_upper.unwrap()>=result.body_decomposition_error_upper.unwrap());
        assert_eq!(result.boundary_error_within_budget,Some(true));
    }
}

pub fn progressive_profile_body_with_evidence(
    loops:&[Vec<Curve>],path:&Curve,scale:&Curve,twist:&Curve,
    affine:Option<(&Curve,&Curve)>,frames:Option<(&Curve,&Curve)>,
    guidance:Option<(&Curve,Option<(usize,f64)>)>,
    options:nurbs_core::progressive_sweep::Options,
)->Result<ProgressiveBodyEvidence>{
    progressive_profile_body_with_evidence_and_correction(loops,path,scale,twist,affine,frames,guidance,options,None)
}
pub fn progressive_profile_body_with_evidence_and_correction(
    loops:&[Vec<Curve>],path:&Curve,scale:&Curve,twist:&Curve,
    affine:Option<(&Curve,&Curve)>,frames:Option<(&Curve,&Curve)>,
    guidance:Option<(&Curve,Option<(usize,f64)>)>,options:nurbs_core::progressive_sweep::Options,
    correction:Option<EndpointCapCorrection>,
)->Result<ProgressiveBodyEvidence>{
    progressive_profile_body_with_rmf_policy(loops,path,scale,twist,affine,frames,guidance,options,correction,None)
}

/// Optional bounded original spatial RMF proof; existing callers retain their policy.
pub fn progressive_profile_body_with_rmf_policy(
    loops:&[Vec<Curve>],path:&Curve,scale:&Curve,twist:&Curve,
    affine:Option<(&Curve,&Curve)>,frames:Option<(&Curve,&Curve)>,
    guidance:Option<(&Curve,Option<(usize,f64)>)>,options:nurbs_core::progressive_sweep::Options,
    correction:Option<EndpointCapCorrection>,rmf_policy:Option<(usize,usize,usize)>,
)->Result<ProgressiveBodyEvidence>{
    use nurbs_core::progressive_sweep::{Sweep, approximate_profiles};
    if rmf_policy.is_some() && (frames.is_some() || guidance.is_some()) {
        return Err(err("Spatial RMF policy excludes authored frames and guides"));
    }
    if (frames.is_some() || guidance.is_some()) && affine.is_none() || frames.is_some() && guidance.is_some() {
        return Err(err("Progressive body frame/guide configuration requires affine laws and one frame source"));
    }
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
    let approximation = if let Some((steps,cells,products)) = rmf_policy {
        nurbs_core::progressive_sweep::approximate_spatial_rmf_profiles(
            &profiles,path,scale,twist,affine,options,steps,cells,products,
        )?
    } else if let Some((guide, anchor)) = guidance {
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
        if rmf_policy.is_some() {
            return Err(err(&format!("Progressive body continuous retained-patch error is unproved or exceeds budget: {}",
                report.error_certificate_reason.unwrap_or("configured RMF error budget"))));
        }
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
    let mut sections = (0..report.sections)
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
    let cap_correction_error_upper=if let Some(correction)=correction {
        if report.closed_path{return Err(err("Closed progressive body has no caps to correct"));}
        let mut work=0_u64;let mut displacement=0_f64;
        for end in [false,true] {
            let station=if end {sections.len()-1}else{0};
            let normal=if let Some((axis,_))=frames {
                let domain=axis.domain();axis.evaluate(if end {domain[1]}else{domain[0]})?.point
            }else{
                let domain=path.domain();let parameter=if end&&options.orientation!=nurbs_core::progressive_sweep::Orientation::Fixed {domain[1]}else{domain[0]};
                path.evaluate(parameter)?.d1.ok_or_else(||err("Cap correction needs an endpoint tangent"))?
            };
            let axis=nurbs_core::progressive_sweep::constant_vector_law([normal[0],normal[1],normal[2]])?;
            let curves=sections[station].iter().flatten().cloned().collect::<Vec<_>>();
            let projected=nurbs_core::section_projection::project_authored_axis(&curves,&axis,0.,correction.quantum,
                correction.tolerance,correction.max_work.checked_sub(work).ok_or_else(||err("Cap correction work exhausted"))?)?;
            work+=projected.work;
            if !projected.exact_planar{return Err(err(&format!("Progressive cap correction refused: {}",projected.reason)));}
            let corrected=projected.curves.ok_or_else(||err("Progressive cap correction has no geometry"))?;
            displacement=displacement.max(projected.displacement_upper.ok_or_else(||err("Cap correction displacement unproved"))?);
            let mut at=0;
            for ring in &mut sections[station]{for curve in ring{*curve=corrected[at].clone();at+=1;}}
        }
        Some(displacement)
    }else{None};
    let model = if report.closed_path {
        periodic_section_loft(&sections)?
    } else {
        rational_section_loft(&sections)?
    };
    let (retained_sections,body_decomposition_error_upper,body_decomposition_products)=retained_section_partition(&sections,1000000)?;
    let retained_caps=if report.closed_path {None} else {
        Some(crate::sweep_retained_caps::inspect(&model,&[retained_sections[0].clone(),retained_sections.last().unwrap().clone()],
            crate::sweep_cap_contacts::Budgets {max_walls:1024,max_exact_work:1000000,max_chart_cells:1000,
                max_trim_pairs:100000,max_trim_cells:100000,max_trim_domain_cells:1000000},1024)?)
    };
    let cap_projection=if report.closed_path {None} else {
        let mut source=nurbs_core::progressive_sweep::MultiSweep::new(&profiles,path,scale,twist,options)?;
        if let Some((axes,center))=affine {source=source.with_affine_laws(axes,center)?;}
        if let Some((axis,normal))=frames {source=source.with_frame_laws(axis,normal)?;}
        if let Some((guide,anchor))=guidance {
            source=if let Some((index,parameter))=anchor {source.with_contact_guide(guide,index,parameter)?}else{source.with_orientation_guide(guide)?};
        }
        if let Some((steps,cells,products))=rmf_policy {source=source.with_spatial_rmf_error_limits(steps,cells,products)?;}
        let sizes=loops.iter().map(Vec::len).collect::<Vec<_>>();
        let caps=[model.faces[model.faces.len()-2].surface.clone(),model.faces.last().unwrap().surface.clone()];
        Some(source.certify_endpoint_cap_projection(&sizes,&caps,1e-9,10000,100000,1000000)?)
    };
    let filled_cap_error_upper=cap_projection.as_ref().and_then(|projection|nurbs_core::sweeps::filled_cap_error::filled_caps(
        &nurbs_core::sweeps::filled_cap_error::Premises {
            ideal_domains_certified:projection.original.domains.ideal_endpoint_domains_certified,
            retained_regions_exact:retained_caps.as_ref().is_some_and(|r|r.exact),
            projection_normal_dots:projection.normal_dots,endpoint_error:report.endpoint_contour_error_upper,
            correction:Some(cap_correction_error_upper.unwrap_or(0.)),decomposition:body_decomposition_error_upper.map(|e|[e;2]),parallel_planes:[false;2],
        }));
    let retained_walls=crate::sweep_retained_walls::inspect(&model,&retained_sections,report.closed_path,1024,1000000)?;
    // Preview may itself have been decomposed; keeping its full error and adding
    // constructor extraction is conservative even when both include that term.
    let original_wall_error=if retained_walls.certified {report.continuous_error_upper.zip(body_decomposition_error_upper)
        .and_then(|(a,b)|nurbs_core::numerics::error_upper::add(a,b))}else{None};
    let wall_error=if let Some(correction)=cap_correction_error_upper {original_wall_error.and_then(|e|nurbs_core::numerics::error_upper::add(e,correction))}else{original_wall_error};
    let boundary_error_upper=nurbs_core::sweeps::filled_cap_error::boundary(wall_error,filled_cap_error_upper,report.closed_path);
    let boundary_error_within_budget=boundary_error_upper.map(|upper|upper<=options.max_deviation);
    Ok(ProgressiveBodyEvidence {model,approximation,retained_caps,cap_projection,filled_cap_error_upper,cap_correction_error_upper,retained_walls,boundary_error_upper,boundary_error_within_budget,body_decomposition_error_upper,body_decomposition_products})
}

#[cfg(test)]
mod retained_tests {
    use super::*;
    #[test]
    fn periodic_hollow_sections_decompose_without_bypassing_join_or_orientation_checks() {
        let outer = Curve { degree: 2, knots: (0..9).map(|i| i as f64).collect(),
            control_points: vec![vec![1.,0.,0.],vec![0.,1.,0.],vec![-1.,0.,0.],
                vec![0.,-1.,0.],vec![1.,0.,0.],vec![0.,1.,0.]],
            weights: vec![1.;6], periodic: true };
        let mut hole = outer.clone();
        hole.control_points.reverse();
        for point in &mut hole.control_points { for x in point { *x *= 0.25; } }
        let start = vec![vec![outer], vec![hole]];
        let mut end = start.clone();
        for curve in end.iter_mut().flatten() { for point in &mut curve.control_points { point[2] += 10.; } }
        let sections = vec![start,end];
        let before = sections.clone();
        let model = rational_section_loft(&sections).unwrap();
        model.validate().unwrap();
        assert_eq!(model.faces.len(),10);
        assert_eq!(model.edges.len(),24);
        assert_eq!(model.faces.iter().filter(|f| !f.holes.is_empty()).count(),2);
        let mut wrong_orientation = sections.clone();
        for section in &mut wrong_orientation { section[1][0].control_points.reverse(); }
        assert!(rational_section_loft(&wrong_orientation).is_err());
        let mut open = sections.clone();
        open[1][0][0].periodic = false;
        open[1][0][0].control_points[5][0] += 0.125;
        assert!(rational_section_loft(&open).is_err());
        assert_eq!(sections,before);
    }
    #[test]
    fn segmented_curve_retains_coefficients_without_homogeneous_rounding() {
        let c = Curve {
            degree: 2,
            knots: vec![3., 3., 3., 5., 5., 7., 7., 7.],
            control_points: vec![
                vec![0.13, 0., 0.],
                vec![0.27, 1., 0.],
                vec![0.39, 2., 0.],
                vec![0.51, 1., 0.],
                vec![0.67, 0., 0.],
            ],
            weights: vec![0.7, 1.3, 0.9, 1.7, 0.8],
            periodic: false,
        };
        let pieces = retained_bezier_pieces(&c).unwrap();
        assert_eq!(pieces.len(), 2);
        for (j, p) in pieces.iter().enumerate() {
            assert_eq!(p.control_points, c.control_points[2 * j..2 * j + 3]);
            assert_eq!(p.weights, c.weights[2 * j..2 * j + 3]);
            for t in [0.13, 0.37, 0.81] {
                let expected = c.evaluate(3. + 2. * j as f64 + 2. * t).unwrap().point;
                let actual = p.evaluate(t).unwrap().point;
                for k in 0..3 {
                    assert!((expected[k] - actual[k]).abs() < 1e-12);
                }
            }
        }
    }
}

#[cfg(test)]
mod cap_certificate_tests {
    use super::*;
    #[test]
    fn oblique_graph_cap_preserves_exact_edges_in_both_senses() {
        for reverse in [false,true] {
            let mut points=vec![[-0.5,-0.5],[0.5,-0.5],[0.5,0.5],[-0.5,0.5]];
            if reverse {points.reverse();}
            let world=|p:[f64;2]|vec![p[0],10.-0.5*p[1],p[1]];
            let loops=vec![(0..4).map(|i|Curve{degree:1,knots:vec![0.,0.,1.,1.],
                control_points:vec![world(points[i]),world(points[(i+1)%4])],weights:vec![1.;2],periodic:false}).collect()];
            let cap=coordinate_cap(&loops,1e-9,100000).unwrap().expect("exact oblique graph");
            for (world,uv) in loops[0].iter().zip(&cap.loops[0]) {
                assert_eq!(nurbs_core::curve_surface_agreement::verify_exact(world,uv,&cap.surface,false,1000000).unwrap().unwrap().outcome,cad_predicates::BezierIdentity::Equal);
            }
        }
    }
    #[test]
    fn corrected_oblique_rmf_hollow_body_certifies_filled_caps_and_volume() {
        use nurbs_core::{progressive_sweep::{Options,Orientation,Spacing,constant_vector_law},primitives::{circle,line}};
        let outer=circle([0.;3],[3.,4.,0.],0.1).unwrap();
        let inner=circle([0.;3],[3.,4.,0.],0.05).unwrap().reverse().unwrap();
        let loops=vec![retained_bezier_pieces(&outer).unwrap(),retained_bezier_pieces(&inner).unwrap()];
        let path=line([0.;3],[3.,4.,0.]).unwrap();
        let scale=line([1.,0.,0.],[2.,0.,0.]).unwrap();
        let twist=constant_vector_law([0.;3]).unwrap();
        let result=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,None,None,None,
            Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,
                spacing:Spacing::Parameter,initial_sections:2,max_sections:2,max_deviation:0.01},
            Some(EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})).unwrap();
        assert!(result.retained_walls.certified);
        assert!(result.retained_caps.as_ref().unwrap().exact,"{:?}",result.retained_caps);
        assert!(result.filled_cap_error_upper.is_some(),"{:?}",result.cap_projection);
        assert_eq!(result.boundary_error_within_budget,Some(true));
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,
            crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000},
            20000,&[result.model.faces.len()-2,result.model.faces.len()-1],crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,
                max_chart_cells:100000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        assert!(volume.proven,"boundary={}, nesting={:?}, outward={:?}",volume.boundary.proven,volume.nesting.as_ref().map(|n|n.roles_consistent),volume.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
    }
    #[test]
    fn corrected_oblique_65_station_hollow_body_keeps_original_exact_work_limit() {
        use nurbs_core::{progressive_sweep::{Options,Orientation,Spacing,constant_vector_law},primitives::{circle,line}};
        let outer=circle([0.;3],[3.,4.,0.],0.1).unwrap();
        let inner=circle([0.;3],[3.,4.,0.],0.05).unwrap().reverse().unwrap();
        let loops=vec![retained_bezier_pieces(&outer).unwrap(),retained_bezier_pieces(&inner).unwrap()];
        let path=line([0.;3],[3.,4.,0.]).unwrap();
        let scale=line([1.,0.,0.],[2.,0.,0.]).unwrap();
        let twist=constant_vector_law([0.;3]).unwrap();
        let result=progressive_profile_body_with_evidence_and_correction(&loops,&path,&scale,&twist,None,None,None,
            Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,
                spacing:Spacing::Parameter,initial_sections:65,max_sections:65,max_deviation:0.01},
            Some(EndpointCapCorrection {quantum:2_f64.powi(-40),tolerance:1e-9,max_work:1000000})).unwrap();
        assert!(result.retained_walls.certified);
        assert!(result.retained_caps.as_ref().unwrap().exact,"{:?}",result.retained_caps);
        assert!(result.filled_cap_error_upper.is_some(),"{:?}",result.cap_projection);
        assert_eq!(result.boundary_error_within_budget,Some(true));
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,
            crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000},
            20000,&[result.model.faces.len()-2,result.model.faces.len()-1],crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,
                max_chart_cells:100000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        if !volume.proven {
            let pairs=&volume.boundary.intersections.pairs;
            eprintln!("pair-stage next={:?} individual={} grouped={} cells={} group_cells={} domains={} pending={:?}",
                pairs.next_pair,pairs.pairs.len(),pairs.grouped_pairs,pairs.cells,pairs.group_cells,pairs.domain_cells,
                pairs.pairs.iter().filter(|p|p.reason!="pair-disjoint"&&p.reason!="shared-boundary")
                    .take(12).map(|p|(p.faces,p.reason,p.result.as_ref().map(|r|(r.cells,r.domain_cells,r.unresolved.len(),r.contact.is_some())))).collect::<Vec<_>>());
        }
        assert!(volume.proven,"boundary={} agreement={} joins={} work={} trim={} injective={} pairs={} caps={:?} nesting={:?}",
            volume.boundary.proven,volume.boundary.agreement.all_equal,volume.boundary.agreement.all_joins_exact,
            volume.boundary.agreement.work,volume.boundary.trim.all_valid,
            volume.boundary.intersections.faces.all_faces_injective,volume.boundary.intersections.pairs.all_pairs_classified,
            volume.boundary.cap_contacts,volume.nesting.as_ref().map(|n|n.roles_consistent));
    }
    #[test]
    fn dyadic_oblique_hollow_caps_preserve_exact_planes_and_coedges() {
        for axis in 0..3 {for (a,b) in [(-0.75,0.),(0.375,-0.3125)] {for reversed in [false,true] {
            let axes:Vec<_>=(0..3).filter(|k|*k!=axis).collect();
            let ring=|r:f64| {
                let points=[[r,0.],[r,r],[0.,r],[-r,r],[-r,0.],[-r,-r],[0.,-r],[r,-r],[r,0.]];
                (0..4).map(|i|Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
                    control_points:points[2*i..=2*i+2].iter().map(|q| {
                        let mut p=vec![0.;3];p[axes[0]]=q[0];p[axes[1]]=q[1];p[axis]=6.25+a*q[0]+b*q[1];p
                    }).collect(),weights:vec![1.,std::f64::consts::FRAC_1_SQRT_2,1.],periodic:false}).collect::<Vec<_>>()
            };
            let mut loops=vec![ring(1.),ring(0.25).into_iter().rev().map(|c|c.reverse().unwrap()).collect()];
            if reversed {for ring in &mut loops {*ring=ring.iter().rev().map(|c|c.reverse().unwrap()).collect();}}
            let before=format!("{loops:?}");
            let cap=coordinate_cap(&loops,1e-9,100000).unwrap_or_else(|e|panic!("axis={axis} a={a} b={b} reversed={reversed}: {e:?}")).expect("exact dyadic plane requires a natural coordinate chart");
            assert!(nurbs_core::sweeps::progressive_miter::cap_retained_plane_certificate::inspect(&cap.surface,1000000).planar_control_hull_certified);
            let mut work=0;
            for (worlds,uvs) in loops.iter().zip(&cap.loops) {for (world,uv) in worlds.iter().zip(uvs) {
                let proof=nurbs_core::curve_surface_agreement::verify_exact(world,uv,&cap.surface,false,1000000-work).unwrap().unwrap();
                work+=proof.work_used;assert_eq!(proof.outcome,cad_predicates::BezierIdentity::Equal);
            }}
            assert_eq!(format!("{loops:?}"),before);
            let mut changed=loops.clone();changed[0][0].control_points[1][axis]=changed[0][0].control_points[1][axis].next_up();
            assert!(!matches!(coordinate_cap(&changed,1e-9,100000),Ok(Some(_))));
        }}}
    }
    #[test]
    fn coordinate_caps_retain_exact_natural_uv_in_all_axes_and_senses() {
        for axis in 0..3 {for reverse in [false,true] {
            let axes:Vec<_>=(0..3).filter(|k|*k!=axis).collect();
            let mut points=vec![[-0.5,-0.5],[0.5,-0.5],[0.5,0.5],[-0.5,0.5]];
            if reverse {points.reverse();}
            let world=|p:[f64;2]|{let mut q=vec![0.;3];q[axis]=3.;q[axes[0]]=p[0];q[axes[1]]=p[1];q};
            let loops=vec![(0..4).map(|i|Curve{degree:1,knots:vec![0.,0.,1.,1.],
                control_points:vec![world(points[i]),world(points[(i+1)%4])],weights:vec![1.;2],periodic:false}).collect()];
            let cap=cap(&loops).unwrap();
            assert_eq!(cap.surface.knots_u,vec![-0.5,-0.5,0.5,0.5]);
            for (world,uv) in loops[0].iter().zip(&cap.loops[0]) {
                assert_eq!(nurbs_core::curve_surface_agreement::verify_exact(world,uv,&cap.surface,false,1000000).unwrap().unwrap().outcome,cad_predicates::BezierIdentity::Equal);
            }
        }}
    }
    #[test]
    fn translated_hollow_coordinate_cap_fits_shared_exact_budget() {
        let outer=nurbs_core::primitives::circle([0.,0.,10.],[0.,0.,1.],0.5).unwrap();
        let inner=nurbs_core::primitives::circle([0.,0.,10.],[0.,0.,1.],0.2).unwrap().reverse().unwrap();
        let loops=vec![retained_bezier_pieces(&outer).unwrap(),retained_bezier_pieces(&inner).unwrap()];
        let result=coordinate_cap(&loops,1e-9,100000).unwrap().expect("translated cap must fit unchanged shared budget");
        assert!(result.surface.knots_u[0]<0.);
        let mut work=0;
        for (worlds,uvs) in loops.iter().zip(&result.loops) {for (world,uv) in worlds.iter().zip(uvs) {
            let proof=nurbs_core::curve_surface_agreement::verify_exact(world,uv,&result.surface,false,1000000-work).unwrap().unwrap();
            work+=proof.work_used;
            assert_eq!(proof.outcome,cad_predicates::BezierIdentity::Equal);
        }}
        assert!(work<1000000);
        let mut changed=loops.clone();changed[0][0].control_points[1][2]=10_f64.next_up();
        assert!(!matches!(coordinate_cap(&changed,1e-9,100000),Ok(Some(_))));
    }
    #[test]
    fn stored_cap_refuses_zero_product_budget_and_unproved_zero_tolerance() {
        let points=[[0.,0.,0.],[1.,0.,0.],[1.,1.,0.],[0.,1.,0.]];
        let loops=vec![(0..4).map(|i|Curve {degree:1,knots:vec![0.,0.,1.,1.],
            control_points:vec![points[i].to_vec(),points[(i+1)%4].to_vec()],
            weights:vec![1.,1.],periodic:false}).collect()];
        let stored = cap_with_boundary_budget(&loops,1e-9,100000).unwrap();
        assert!(certify_cap_chart(&stored.surface, 1000, 1).is_ok());
        assert!(certify_cap_chart(&stored.surface, 0, 1).is_err());
        assert!(certify_cap_chart(&stored.surface, 1000, 0).is_err());
        let mut collapsed = stored.surface.clone();
        collapsed.control_points[1] = collapsed.control_points[0].clone();
        assert!(certify_cap_chart(&collapsed, 100, 1).is_err());
        for (tolerance,budget) in [(1e-9,0),(0.,100000)] {
            let error=cap_with_boundary_budget(&loops,tolerance,budget).err().unwrap();
            assert!(error.message.contains("Cap boundary composition unproved"));
        }
    }
}

#[cfg(test)]
mod supplied_multispan_tests {
    use super::*;
    fn fixture()->(Vec<Vec<Vec<Curve>>>,Vec<Vec<Surface>>){
        let base=crate::sketch::polygon_wire(vec![[0.,0.],[1.,0.],[1.,1.],[0.,1.]]).unwrap();
        let sections=(0..3).map(|z|vec![base.iter().cloned().map(|mut c|{for p in &mut c.control_points{p.push(z as f64);}c}).collect::<Vec<_>>()]).collect::<Vec<_>>();
        let mut sides=vec![Vec::new()];
        for layer in 0..2{for c in &sections[layer][0]{
            let poles=c.control_points.iter().map(|p|vec![p.clone(),vec![p[0]+0.125,p[1],p[2]+0.25],vec![p[0]-0.125,p[1],p[2]+0.75],vec![p[0],p[1],p[2]+1.]]).collect::<Vec<_>>();
            sides[0].push(Surface{degree_u:1,degree_v:3,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,0.,0.,1.,1.,1.,1.],control_points:poles,weights:vec![vec![1.;4];2],periodic_u:false,periodic_v:false});
        }}
        (sections,sides)
    }
    #[test]
    fn retains_each_actual_cubic_wall_and_its_owned_edges(){
        let (sections,sides)=fixture();let model=section_loft_surfaces(&sections,&sides,false).unwrap();
        assert_eq!(model.faces.len(),10);
        for (index,face) in model.faces[..8].iter().enumerate(){assert_eq!(face.surface,sides[0][index]);}
        assert!(model.edges.iter().any(|edge|edge.curve.degree==3));
        model.validate().unwrap();
        let mut incomplete=sides.clone();incomplete[0].pop();assert!(section_loft_surfaces(&sections,&incomplete,false).is_err());
        assert!(section_loft_surfaces(&sections,&sides,true).is_err());
    }
}

#[cfg(test)]
mod closed_planar_rmf_boundary_tests {
    use super::*;
    #[test]
    fn closed_planar_rmf_full_turn_body_composes_owned_periodic_walls_without_caps() {
        use nurbs_core::{primitives::{line,circle},progressive_sweep::{Options,Orientation,Spacing}};
        let points=[[4.9,0.,-0.1],[5.1,0.,-0.1],[5.1,0.,0.1],[4.9,0.,0.1]];
        let loops=vec![(0..4).map(|i|line(points[i],points[(i+1)%4]).unwrap()).collect()];
        let path=circle([0.;3],[0.,0.,1.],5.).unwrap();
        let scale=nurbs_core::progressive_sweep::constant_vector_law([1.,0.,0.]).unwrap();
        let twist=line([0.;3],[std::f64::consts::TAU,0.,0.]).unwrap();
        let result=progressive_profile_body_with_evidence(&loops,&path,&scale,&twist,None,None,None,
            Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,
                spacing:Spacing::Parameter,initial_sections:5,max_sections:129,max_deviation:0.01}).unwrap();
        let report=result.approximation.levels.last().unwrap();
        assert!(report.closed_path);
        assert!(report.accepted&&report.continuous_bound);
        assert!(result.retained_caps.is_none()&&result.cap_projection.is_none());
        assert!(result.filled_cap_error_upper.is_none());
        assert!(result.retained_walls.certified);
        assert_eq!(result.model.faces.len(),4*(report.sections-1));
        assert_eq!(result.body_decomposition_error_upper,Some(0.));
        let upper=result.boundary_error_upper.unwrap();
        assert!(upper>0.&&upper<=0.01);
        assert_eq!(result.boundary_error_within_budget,Some(true));
        let agreement=crate::boundary_agreement::verify_exact(&result.model,1000000).unwrap();
        let trim=crate::face_domain::audit_trim_regions(&result.model,1e-8,10000,100000,1000000).unwrap();
        let volume=crate::volume_validity::inspect_sweep(&result.model,1e-8,
            crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits {exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits {
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:10000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:1000},
            20000,&[],crate::sweep_cap_contacts::Budgets {max_walls:1000,max_exact_work:1000000,
                max_chart_cells:100000,max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000}).unwrap();
        eprintln!("closed volume proven={}, orientation cells={}, outward={:?}",volume.proven,
            volume.orientation_cells,volume.orientations.iter().map(|s|s.outward).collect::<Vec<_>>());
        let joint=&volume.boundary;
        let unresolved=joint.intersections.pairs.pairs.iter().filter(|p|p.reason=="pair-unresolved")
            .map(|p|p.faces).collect::<Vec<_>>();
        eprintln!("joint proven={}, hulls={}, visited={}, next={:?}, cells={}, unresolved={:?}",
            joint.proven,joint.hull_contacts.len(),joint.intersections.pairs.pairs.len(),
            joint.intersections.pairs.next_pair,joint.intersections.pairs.cells,unresolved);
        for pair in &unresolved {
            eprintln!("unresolved hull {:?}: {:?}",pair,crate::boundary_hull_contact::certify(&result.model,*pair));
            for &face in pair {eprintln!("unresolved poles {face}: {:?}",result.model.faces[face].surface.control_points);}
        }
        assert!(joint.proven,"closed periodic boundary must classify every contact");
        for pair in [[125,130],[126,129],[190,195],[191,194]] {
            let contact=crate::boundary_hull_contact::certify(&result.model,pair)
                .expect("rotated station corner requires exact projected contact");
            let vertex=contact.vertex.expect("corner contact must own its shared vertex");
            let mut displaced=result.model.clone();
            displaced.vertices[vertex].point[2]=displaced.vertices[vertex].point[2].next_up();
            assert!(crate::boundary_hull_contact::certify(&displaced,pair).is_none(),
                "one ULP displacement cannot retain exact corner ownership: {pair:?}");
            let mut unowned=result.model.clone();
            let face=&result.model.faces[pair[1]];
            for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                for index in 0..unowned.loops[wire].coedges.len() {
                    let mut edge=unowned.edges[unowned.loops[wire].coedges[index].edge].clone();
                    for vertex in &mut edge.vertices {
                        let source=unowned.vertices[*vertex].clone();
                        *vertex=unowned.vertices.len();unowned.vertices.push(source);
                    }
                    unowned.loops[wire].coedges[index].edge=unowned.edges.len();unowned.edges.push(edge);
                }
            }
            assert!(crate::boundary_hull_contact::certify(&unowned,pair).is_none(),
                "coincident poles without shared topology cannot authorize corner contact: {pair:?}");
        }
        assert!(volume.proven,"closed periodic body requires native nesting and outward orientation");
        let winding=trim.faces.iter().filter(|r|r.as_ref().is_some_and(|r|r.winding.iter().all(|w|*w==Some(1)))).count();
        eprintln!("closed body exact={}, joins={}, trim={}, positive winding faces={}",
            agreement.all_equal,agreement.all_joins_exact,trim.all_valid,winding);
        for pair in [[0,5],[0,7],[0,result.model.faces.len()-3],[0,result.model.faces.len()-1]] {
            eprintln!("closed body hull {:?}: {:?}",pair,crate::boundary_hull_contact::certify(&result.model,pair));
        }
    }
}
