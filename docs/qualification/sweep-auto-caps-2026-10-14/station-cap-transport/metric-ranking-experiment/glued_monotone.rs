//! One fixed linear projection on two complete parameter half-rectangles.
//! The source Bezier poles and weights remain unchanged. Midpoint derivatives
//! propose a projection and transverse parameter lengths only; existing interval
//! symmetric-Jacobian certificates establish the actual whole-domain result.
use super::{Result, Surface};

#[derive(Clone,Copy,Debug)]
enum Parametrization { Extents, GeometricMean, Balanced }

fn virtual_face(
    source: &Surface,
    end: bool,
    left: bool,
    policy: Parametrization,
) -> Result<Option<Surface>> {
    let u = [
        source.knots_u[source.degree_u],
        source.knots_u[source.degree_u + 1],
    ];
    let v = [
        source.knots_v[source.degree_v],
        source.knots_v[source.degree_v + 1],
    ];
    let sample = source.evaluate(u[0] * 0.5 + u[1] * 0.5, v[0] * 0.5 + v[1] * 0.5)?;
    let Some((du, dv)) = sample.first_derivatives() else {
        return Ok(None);
    };
    let transverse = du[0].hypot(du[1]).hypot(du[2]) * (u[1] - u[0]);
    let along = dv[0].hypot(dv[1]).hypot(dv[2]);
    let extents=transverse.max(along*(v[1]-v[0]));
    let length = match policy {
        // Equalize proposed parameter derivative magnitudes, not physical
        // rectangle extents. A thin cap next to a long wall otherwise makes
        // the symmetric cross terms dominate despite a valid shared chart.
        // This is only a choice of positive clamped knot endpoints: its
        // rounded value is not a premise of any derivative/positivity proof.
        Parametrization::Balanced=>transverse/along,
        Parametrization::Extents=>extents,
        // An intermediate positive parameter metric avoids making either
        // the long wall or the thin cap dominate the common dual projection.
        // Only original coefficients and outward Jacobian bounds certify it.
        Parametrization::GeometricMean=>extents.sqrt()*(transverse/along).sqrt(),
    };
    if !length.is_finite() || length <= 0. {
        return Ok(None);
    }
    let mut out = source.clone();
    if left != end {
        out.control_points.reverse();
        out.weights.reverse();
    }
    let domain = if left { [-length, 0.] } else { [0., length] };
    out.knots_u = [
        vec![domain[0]; source.degree_u + 1],
        vec![domain[1]; source.degree_u + 1],
    ]
    .concat();
    out.validate()?;
    Ok(Some(out))
}
fn proposal(a: &Surface, b: &Surface) -> Result<Option<[[i8; 3]; 2]>> {
    let mut columns = [[0.; 3]; 2];
    for s in [a, b] {
        let sample = s.evaluate(
            (s.knots_u[s.degree_u] + s.knots_u[s.degree_u + 1]) * 0.5,
            (s.knots_v[s.degree_v] + s.knots_v[s.degree_v + 1]) * 0.5,
        )?;
        let Some((du, dv)) = sample.first_derivatives() else {
            return Ok(None);
        };
        for k in 0..3 {
            columns[0][k] += du[k] * 0.5;
            columns[1][k] += dv[k] * 0.5;
        }
    }
    let dot = |a: [f64; 3], b: [f64; 3]| (0..3).map(|k| a[k] * b[k]).sum::<f64>();
    let g = [
        [dot(columns[0], columns[0]), dot(columns[0], columns[1])],
        [dot(columns[1], columns[0]), dot(columns[1], columns[1])],
    ];
    let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
    if !det.is_finite() || det <= 0. {
        return Ok(None);
    }
    let inverse = [
        [g[1][1] / det, -g[0][1] / det],
        [-g[1][0] / det, g[0][0] / det],
    ];
    let y = std::array::from_fn::<_, 2, _>(|i| {
        std::array::from_fn::<_, 3, _>(|k| {
            inverse[i][0] * columns[0][k] + inverse[i][1] * columns[1][k]
        })
    });
    if y.iter().flatten().any(|v| !v.is_finite()) {
        return Ok(None);
    }
    let scale = y.iter().flatten().map(|v| v.abs()).fold(0., f64::max);
    if scale <= 0. {
        return Ok(None);
    }
    let p = y.map(|r| r.map(|v| (v / scale * 63.).round() as i8));
    if !(0..3).any(|k| {
        p[0][k] as i16 * p[1][(k + 1) % 3] as i16 != p[1][k] as i16 * p[0][(k + 1) % 3] as i16
    }) {
        return Ok(None);
    }
    Ok(Some(p))
}
fn certify_policy(
    a: &Surface,
    b: &Surface,
    ends: [bool; 2],
    max_cells: usize,
    policy: Parametrization,
) -> Result<(bool, usize)> {
    let (Some(a), Some(b)) = (
        virtual_face(a, ends[0], true, policy)?,
        virtual_face(b, ends[1], false, policy)?,
    ) else {
        return Ok((false, 0));
    };
    let Some(p) = proposal(&a, &b)? else {
        return Ok((false, 0));
    };
    // Both maps use the same V interval and exact common boundary. Their
    // rectangles glue to [-La,Lb] x [v0,v1], which is convex. Positive symmetric
    // projected Jacobians on every cell imply strict monotonicity along every
    // segment in that union, including segments crossing the shared boundary.
    let first = crate::surface_linear_monotonicity::inspect(&a, p, max_cells)?;
    if !first.certified {
        return Ok((false, first.cells));
    }
    let second = crate::surface_linear_monotonicity::inspect(&b, p, max_cells - first.cells)?;
    Ok((second.certified, first.cells + second.cells))
}

// Corners and the midpoint rank proposals only. Even a positive score does
// not admit any cell: certify_policy still proves both complete rectangles.
fn proposal_score(a:&Surface,b:&Surface,ends:[bool;2],policy:Parametrization)->Result<f64> {
    let (Some(a),Some(b))=(virtual_face(a,ends[0],true,policy)?,virtual_face(b,ends[1],false,policy)?)
        else{return Ok(f64::NEG_INFINITY);};
    let Some(p)=proposal(&a,&b)? else{return Ok(f64::NEG_INFINITY);};
    let mut score=1_f64;
    for s in [&a,&b] {
        let u=[s.knots_u[s.degree_u],s.knots_u[s.degree_u+1]];
        let v=[s.knots_v[s.degree_v],s.knots_v[s.degree_v+1]];
        for [x,y] in [[u[0],v[0]],[u[0],v[1]],[u[1],v[0]],[u[1],v[1]],
            [u[0]*0.5+u[1]*0.5,v[0]*0.5+v[1]*0.5]] {
            let sample=s.evaluate(x,y)?;
            let Some((du,dv))=sample.first_derivatives() else{return Ok(f64::NEG_INFINITY);};
            let dot=|row:usize,d:[f64;3]| (0..3).map(|k|p[row][k] as f64*d[k]).sum::<f64>();
            let uu=dot(0,du);let vv=dot(1,dv);let off=(dot(0,dv)+dot(1,du))*0.5;
            let product=uu*vv;let radius=off*off;
            let margin=(product-radius)/(product+radius);
            if uu<=0.||vv<=0.||!margin.is_finite() {return Ok(f64::NEG_INFINITY);}
            score=score.min(margin);
        }
    }
    Ok(score)
}

pub(super) fn certify(
    a: &Surface,
    b: &Surface,
    ends: [bool; 2],
    max_cells: usize,
) -> Result<(bool, usize)> {
    // Preserve the existing successful fast path and account for its failed
    // attempt before trying other parametrizations. No attempt gets a fresh
    // cell allowance. All keep the same original poles and V curve.
    let first = certify_policy(a, b, ends, max_cells, Parametrization::Extents)?;
    if first.0 || first.1 == max_cells {
        return Ok(first);
    }
    // A failing intermediate metric must not starve an already useful
    // balanced metric on outer walls. Rank the two bounded proposals before
    // spending the same remaining proof allowance; never accept samples.
    let mut policies=[Parametrization::Balanced,Parametrization::GeometricMean];
    if proposal_score(a,b,ends,policies[1])?>proposal_score(a,b,ends,policies[0])? {policies.swap(0,1);}
    let mut used=first.1;
    for policy in policies {
        let result=certify_policy(a,b,ends,max_cells-used,policy)?;
        used+=result.1;
        if result.0||used==max_cells {return Ok((result.0,used));}
    }
    Ok((false,used))
}

#[cfg(all(test, feature = "codec"))]
mod tests {
    use super::*;

    fn natural(mut surface: Surface, pcurve: crate::curve::Curve) -> (Surface, bool) {
        let axis = (0..2)
            .find(|&k| {
                pcurve
                    .control_points
                    .iter()
                    .all(|p| p[k] == pcurve.control_points[0][k])
            })
            .unwrap();
        if axis == 1 {
            surface = crate::surface_edit::transpose(&surface).unwrap();
        }
        let position = pcurve.control_points[0][axis];
        let end = position == surface.knots_u[surface.control_points.len()];
        assert!(end || position == surface.knots_u[surface.degree_u]);
        (surface, end)
    }

    #[test]
    fn thin_oblique_caps_and_long_walls_share_a_complete_monotone_chart() {
        let data: value_codec::Value =
            value_codec::from_str(include_str!("fixtures/oblique-annular-wall-cap.json")).unwrap();
        for pair in data["pairs"].as_array().unwrap() {
            let (a, ea) = natural(
                value_codec::from_value(pair["surfaces"][0].clone()).unwrap(),
                value_codec::from_value(pair["pcurves"][0].clone()).unwrap(),
            );
            let (b, eb) = natural(
                value_codec::from_value(pair["surfaces"][1].clone()).unwrap(),
                value_codec::from_value(pair["pcurves"][1].clone()).unwrap(),
            );
            let proof =
                super::super::certify_shared_natural_boundary(&a, &b, [ea, eb], 1000).unwrap();
            assert!(
                proof.proven,
                "pair {}: {}, cells={}",
                pair["faces"], proof.reason, proof.spans
            );
            assert!(proof.spans <= 1000);
            let empty = super::super::certify_shared_natural_boundary(&a, &b, [ea, eb], 0).unwrap();
            assert!(!empty.proven && empty.spans == 0);
            let overlap =
                super::super::certify_shared_natural_boundary(&a, &a, [ea, ea], 100).unwrap();
            assert!(!overlap.proven && overlap.spans <= 100);
            let mut changed = b.clone();
            let row = if eb { changed.degree_u } else { 0 };
            changed.control_points[row][0][0] = changed.control_points[row][0][0].next_up();
            let changed =
                super::super::certify_shared_natural_boundary(&a, &changed, [ea, eb], 1000)
                    .unwrap();
            assert!(
                !changed.proven,
                "A changed common curve cannot inherit the glued certificate"
            );
        }
    }

    #[test]
    fn reconstructed_oblique_station_and_cap_share_the_original_pair_budget() {
        let data:value_codec::Value=value_codec::from_str(include_str!(
            "fixtures/oblique-reconstructed-station-wall-cap.json"
        )).unwrap();
        for pair in data["pairs"].as_array().unwrap() {
            let (a,ea)=natural(value_codec::from_value(pair["surfaces"][0].clone()).unwrap(),
                value_codec::from_value(pair["pcurves"][0].clone()).unwrap());
            let (mut b,eb)=natural(value_codec::from_value(pair["surfaces"][1].clone()).unwrap(),
                value_codec::from_value(pair["pcurves"][1].clone()).unwrap());
            let ia=if ea{a.degree_u}else{0};let ib=if eb{b.degree_u}else{0};
            if a.control_points[ia]!=b.control_points[ib] {
                for row in &mut b.control_points {row.reverse();}
                for row in &mut b.weights {row.reverse();}
            }
            assert_eq!(a.control_points[ia],b.control_points[ib]);
            assert_eq!(a.weights[ia],b.weights[ib]);
            let original=(a.clone(),b.clone());
            let proof=super::super::certify_shared_natural_boundary(&a,&b,[ea,eb],1000).unwrap();
            eprintln!("pair {}: total={proof:?}, extent={:?}, middle={:?}, balanced={:?}, scores=({}, {})",pair["faces"],
                certify_policy(&a,&b,[ea,eb],1000,Parametrization::Extents).unwrap(),
                certify_policy(&a,&b,[ea,eb],1000,Parametrization::GeometricMean).unwrap(),
                certify_policy(&a,&b,[ea,eb],1000,Parametrization::Balanced).unwrap(),
                proposal_score(&a,&b,[ea,eb],Parametrization::GeometricMean).unwrap(),
                proposal_score(&a,&b,[ea,eb],Parametrization::Balanced).unwrap());
            assert!(proof.proven,"pair {}: {}, cells={}",pair["faces"],proof.reason,proof.spans);
            assert!(proof.spans>6&&proof.spans<=1000);
            assert!(super::super::certify_shared_natural_boundary(&a,&b,[ea,eb],proof.spans).unwrap().proven);
            for budget in [0,5,6,proof.spans-1] {
                let partial=super::super::certify_shared_natural_boundary(&a,&b,[ea,eb],budget).unwrap();
                assert!(!partial.proven&&partial.spans<=budget);
            }
            let overlap=super::super::certify_shared_natural_boundary(&a,&a,[ea,ea],1000).unwrap();
            assert!(!overlap.proven&&overlap.spans<=1000);
            assert_eq!((a.clone(),b.clone()),original);
            b.control_points[ib][0][0]=b.control_points[ib][0][0].next_up();
            assert!(!super::super::certify_shared_natural_boundary(&a,&b,[ea,eb],1000).unwrap().proven);
        }
    }
}
