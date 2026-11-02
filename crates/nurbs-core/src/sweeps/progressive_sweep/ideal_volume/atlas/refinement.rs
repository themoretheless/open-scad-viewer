//! Source-paid refinement proposals. Neither point probes nor width scores
//! authorize geometry; every selected subcell still needs the same full SPD
//! proof under ONE fixed metric over the complete joint hull.
use super::*;
pub(super) fn promising(
    pieces: &[Box3],
    projection: [[f64; 3]; 3],
    scale: [f64; 3],
    metric: [[f64; 3]; 3],
    predicate: fn([V3;3],[[f64;3];3])->Result<bool>,
    query: &mut impl FnMut(Box3, &mut IdealVolumeReport) -> Result<Enclosed>,
    out: &mut IdealVolumeReport,
    max: usize,
) -> Result<bool> {
    for b in pieces {
        // Keep cheap corner rejection first. Rational carrier jets can be
        // benign at every corner but violate this candidate metric on a face
        // or in the interior. Extra original-point queries ONLY reject a
        // proposal; all successful candidates still need the complete cover.
        let corners = (0..8).map(|mask| std::array::from_fn(|i| [b[i][(mask >> i) & 1]; 2]));
        let mids = (0usize..27).filter(|&code|
            (0..3).any(|i| (code / 3usize.pow(i)) % 3 == 1)
        ).map(|code| std::array::from_fn(|i| {
            let position = (code / 3usize.pow(i as u32)) % 3;
            let value = match position {
                0 => b[i][0], 2 => b[i][1],
                _ => crate::distance_bounds::midpoint(b[i][0], b[i][1]),
            };
            [value; 2]
        }));
        for point in corners.chain(mids) {
            if out.cells == max {
                return Ok(false);
            }
            out.cells += 1;
            let Enclosed::Value(e) = query(point, out)? else {
                return Ok(false);
            };
            let positive = projected(e.columns, projection)
                .and_then(|c| scaled(c, scale))
                .and_then(|c| predicate(c, metric))
                .unwrap_or(false);
            if !positive {
                #[cfg(test)]
                if std::env::var_os("OSV_TRACE_ACTUAL_METRIC").is_some() {
                    eprintln!("METRIC rejected point={point:?} columns={:?}",e.columns);
                }
                return Ok(false);
            }
        }
    }
    Ok(true)
}
#[derive(Clone,Copy)]
struct Score {proved:bool,gap:f64,width:f64}
fn score(e:Enclosed,projection:[[f64;3];3],scale:[f64;3],metric:[[f64;3];3],predicate:fn([V3;3],[[f64;3];3])->Result<bool>)->Score {
    let unknown=Score{proved:false,gap:f64::INFINITY,width:f64::INFINITY};
    let Enclosed::Value(e)=e else{return unknown;};
    let Ok(c)=projected(e.columns,projection).and_then(|c|scaled(c,scale)) else{return unknown;};
    if predicate(c,metric).unwrap_or(false) {
        return Score{proved:true,gap:0.,width:0.};
    }
    let gap=margin(c,metric).ok().map(|m|(-m).max(0.)).filter(|x|x.is_finite()).unwrap_or(f64::INFINITY);
    let width=projected(c,metric).ok().map(|c|c.iter().flatten().map(|x|x.hi-x.lo).sum::<f64>())
        .filter(|x|x.is_finite()).unwrap_or(f64::INFINITY);
    Score{proved:false,gap,width}
}
/// All children marked complete have already passed the SAME whole-interval
/// SPD predicate under the parent's ONE fixed metric and coordinate scale.
pub(super) enum Choice {Covered, Split{axis:usize,complete:[bool;2]}, Unavailable}
pub(super) fn axis(
    b:Box3,projection:[[f64;3];3],scale:[f64;3],metric:[[f64;3];3],
    predicate:fn([V3;3],[[f64;3];3])->Result<bool>,
    query:&mut impl FnMut(Box3,&mut IdealVolumeReport)->Result<Enclosed>,
    out:&mut IdealVolumeReport,max:usize,
)->Result<Choice> {
    let Some((fallback,_))=widest(b) else{return Ok(Choice::Unavailable);};
    let mut best=(usize::MAX,f64::INFINITY,f64::INFINITY);
    let mut choice=Choice::Split{axis:fallback,complete:[false;2]};
    for axis in 0..3 {
        let Some(children)=split_axis(b,axis) else{continue;};
        if out.cells==max {return Ok(Choice::Unavailable);}out.cells+=1;
        let a=score(query(children[0],out)?,projection,scale,metric,predicate);
        if out.cells==max {return Ok(Choice::Unavailable);}
        let z=score(query(children[1],out)?,projection,scale,metric,predicate);
        if a.proved && z.proved {return Ok(Choice::Covered);}
        let candidate=(usize::from(!a.proved)+usize::from(!z.proved),a.gap.max(z.gap),a.width.max(z.width));
        if (candidate.1.is_finite() || candidate.0<2) && candidate<best {
            best=candidate;choice=Choice::Split{axis,complete:[a.proved,z.proved]};
        }
    }
    Ok(choice)
}
