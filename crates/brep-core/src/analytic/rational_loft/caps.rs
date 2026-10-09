use super::*;

pub(super) fn certify_cap_chart(surface: &Surface, max_cells: usize, max_spans: usize) -> Result<()> {
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
pub(super) fn cap(loops: &[Vec<Curve>]) -> Result<Cap> {
    cap_with_boundary_budget(loops, 1e-9, 100000)
}
pub(super) fn cap_with_boundary_budget(loops: &[Vec<Curve>], tolerance: f64, max_products: usize) -> Result<Cap> {
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
pub(super) fn coordinate_cap(loops:&[Vec<Curve>],tolerance:f64,max_products:usize)->Result<Option<Cap>> {
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
pub(super) fn retained_bezier_pieces(curve: &Curve) -> Result<Vec<Curve>> {
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
