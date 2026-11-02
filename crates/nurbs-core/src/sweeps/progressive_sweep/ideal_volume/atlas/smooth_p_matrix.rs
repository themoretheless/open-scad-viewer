//! Gale--Nikaido, Math. Ann. 159 (1965), section 4: a C1 map whose
//! Jacobian is a P-matrix on a rectangle is injective there. This route is
//! ONLY called after the source factory proves one smooth rational chart.
//! A fixed left metric, all seven strict principal minors and a complete
//! original field cover are mandatory. Point proposals never grant credit.
use super::*;
fn minors(columns: [V3; 3], metric: [[f64; 3]; 3]) -> Result<[I; 7]> {
    let a = projected(columns, metric)?;
    Ok([
        a[0][0],
        a[1][1],
        a[2][2],
        a[0][0].mul(a[1][1])?.sub(a[1][0].mul(a[0][1])?)?,
        a[0][0].mul(a[2][2])?.sub(a[2][0].mul(a[0][2])?)?,
        a[1][1].mul(a[2][2])?.sub(a[2][1].mul(a[1][2])?)?,
        iv::dot_tight(a[0], iv::cross(a[1], a[2])?)?,
    ])
}
pub(crate) fn certify(
    mut query: impl FnMut(Box3, &mut IdealVolumeReport, usize) -> Result<Enclosed>,
    projection: [[f64; 3]; 3],
    out: &mut IdealVolumeReport,
    max: usize,
) -> Result<bool> {
    if out.cells == max {
        return Ok(false);
    }
    out.cells += 1;
    let Enclosed::Value(seed) = query([[0.5; 2]; 3], out, max)? else {
        return Ok(false);
    };
    let Some(metric) = inverse_proposal(projected(seed.columns, projection)?) else {
        return Ok(false);
    };
    let determinant = iv::dot_tight(
        metric[0].map(I::point),
        iv::cross(metric[1].map(I::point), metric[2].map(I::point))?,
    )?;
    if determinant.lo <= 0. && determinant.hi >= 0. {
        return Ok(false);
    }
    // Necessary rejection checks only. A full cover below is still required.
    for bits in 0..8 {
        if out.cells == max {
            return Ok(false);
        }
        out.cells += 1;
        let point = std::array::from_fn(|axis| [f64::from((bits >> axis) & 1); 2]);
        let Enclosed::Value(e) = query(point, out, max)? else {
            return Ok(false);
        };
        if minors(projected(e.columns, projection)?, metric)?
            .iter()
            .any(|m| m.hi <= 0.)
        {
            return Ok(false);
        }
    }
    let mut pending = vec![[[0., 1.]; 3]];
    while let Some(b) = pending.pop() {
        if out.cells == max {
            return Ok(false);
        }
        out.cells += 1;
        let e = query(b, out, max)?;
        if let Enclosed::Value(e) = e {
            if minors(projected(e.columns, projection)?, metric)?
                .iter()
                .all(|m| m.lo > 0.)
            {
                continue;
            }
        }
        let axis = match e {
            Enclosed::Unsupported(_) => return Ok(false),
            Enclosed::Unavailable(Some(axis)) => axis,
            _ => (0..3)
                .max_by(|&a, &c| (b[a][1] - b[a][0]).total_cmp(&(b[c][1] - b[c][0])))
                .unwrap(),
        };
        let Some(children) = split_axis(b, axis) else {
            return Ok(false);
        };
        pending.extend(children);
    }
    out.embedding_certified = true;
    out.regularity_certified = true;
    out.parameter_topology_preserved = true;
    out.reason = None;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_cover_replays_exact_budget_and_refuses_an_interior_fold() {
        let projection = [[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]];
        let run = |max, fold| {
            let mut out=IdealVolumeReport{embedding_certified:false,regularity_certified:false,
                parameter_topology_preserved:false,cells:0,exact_work:0,boxes:0,
                monotonicity_lower:None,projection,reason:None};
            let accepted=certify(|b,_,_| {
                let u=I::new(b[0][0],b[0][1])?;
                let slope=I::point(3.).mul(u.mul(u)?)?
                    .sub(I::point(3.).mul(u)?)?.add(I::point(if fold {0.7} else {1.}))?;
                let mut columns=[[I::point(0.);3];3];
                columns[0][0]=slope;columns[1][1]=I::point(1.);columns[2][2]=I::point(1.);
                Ok(Enclosed::Value(cell::Enclosure{value:[I::point(0.);3],columns}))
            },projection,&mut out,max).unwrap();
            (accepted,out)
        };
        let (ok,full)=run(10000,false);assert!(ok);assert!(full.cells<10000);
        let (ok,replay)=run(full.cells,false);assert!(ok);assert_eq!(replay.cells,full.cells);
        let (ok,short)=run(full.cells-1,false);assert!(!ok);
        assert!(!short.embedding_certified && !short.regularity_certified && !short.parameter_topology_preserved);
        // The folded polynomial has positive endpoint derivatives but negative
        // derivative near the middle; an inverse proposal is never admission.
        let (ok,fold)=run(10000,true);assert!(!ok);
        assert!(!fold.embedding_certified && !fold.regularity_certified && !fold.parameter_topology_preserved);
    }
    #[test]
    fn all_principal_minors_are_required_not_symmetry_or_positive_determinant_alone() {
        let identity = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        let a = [[1., 4., 0.], [-4., 1., 0.], [20., 20., 1.]].map(|c| c.map(I::point));
        assert!(!symmetric_positive(a, identity).unwrap());
        assert!(minors(a, identity).unwrap().iter().all(|m| m.lo > 0.));
        let mut bad = a;
        bad[0][0] = I::point(-1.);
        bad[1][1] = I::point(-1.);
        assert!(
            iv::dot_tight(bad[0], iv::cross(bad[1], bad[2]).unwrap())
                .unwrap()
                .lo
                > 0.
        );
        assert!(!minors(bad, identity).unwrap().iter().all(|m| m.lo > 0.));
    }
}
