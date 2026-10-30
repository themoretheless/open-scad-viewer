//! surface self chord algorithm: inspect_original_endpoint_pair_with.
use super::*;
pub fn inspect_original_endpoint_pair_with<'a>(
    s:&'a Surface,pair:[[[f64;2];2];2],sine:f64,max_spans:usize,
    qualify:&mut impl FnMut([[f64;2];2],usize)->Result<Report<'a>>,
    normal:&mut impl FnMut([[f64;2];2],usize)->Result<(Option<[[f64;2];3]>,usize)>,
)->Result<(bool,usize)> {
    s.validate()?;
    check(sine.is_finite() && (0. ..1.).contains(&sine) && max_spans<=100000,
        "Bound original endpoint angle and work")?;
    for rect in pair {
        check(rect.iter().all(|a|a[0]<a[1]),"Endpoint rectangles require positive widths")?;
        normal_alignment::validate_rectangle(s,rect,max_spans.max(1))?;
    }
    if max_spans==0 {return Ok((false,0));}
    let union=std::array::from_fn(|a|[pair[0][a][0].min(pair[1][a][0]),pair[0][a][1].max(pair[1][a][1])]);
    let report=qualify(union,max_spans)?;
    let mut used=report.spans;
    let Some(mut matrix)=report.mean_matrix else {return Ok((false,used));};
    let raw=matrix.jacobian.map(|row|row.map(|d|[d.lo,d.hi]));
    if !inspect_jacobian_rank_bound(raw)? {
        let strips=[[union[0],pair[0][1]],[pair[1][0],union[1]]];
        if used==max_spans {return Ok((false,used));}
        let u=qualify(strips[0],max_spans-used)?;
        used+=u.spans;
        let Some(u)=u.mean_matrix else {return Ok((false,used));};
        if used==max_spans {return Ok((false,used));}
        let v=qualify(strips[1],max_spans-used)?;
        used+=v.spans;
        let Some(v)=v.mean_matrix else {return Ok((false,used));};
        let tightened=matrix.coordinate_path(pair,&u,&v)?.matrix.jacobian;
        matrix.jacobian=tightened;
    }
    // Share the immutable endpoint normal hulls with the later coordinate-path
    // exclusion instead of paying for the same source rectangle twice.
    let (excluded,spent)=matrix.exclude_pair_with_normal(pair,sine,max_spans-used,None,None,normal)?;used+=spent;
    if excluded {return Ok((true,used));}
    // Intersect the two exact original coordinate-path chords. Each path is
    // source-bound and all connecting strip coverage must complete first.
    let mut chord=[[f64::NEG_INFINITY,f64::INFINITY];3];
    for reverse in [false,true] {
        let endpoints=if reverse {[pair[1],pair[0]]}else{pair};
        let strips=[[union[0],endpoints[0][1]],[endpoints[1][0],union[1]]];
        if used==max_spans {return Ok((false,used));}
        let u=qualify(strips[0],max_spans-used)?;used+=u.spans;
        let Some(u)=u.mean_matrix else {return Ok((false,used));};
        if used==max_spans {return Ok((false,used));}
        let v=qualify(strips[1],max_spans-used)?;used+=v.spans;
        let Some(v)=v.mean_matrix else {return Ok((false,used));};
        let bounds=matrix.coordinate_path(endpoints,&u,&v)?.chord_bounds()?;
        for k in 0..3 {
            let value=if reverse {[-bounds[k][1],-bounds[k][0]]}else{bounds[k]};
            chord[k]=[chord[k][0].max(value[0]),chord[k][1].min(value[1])];
            I::new(chord[k][0],chord[k][1])?;
        }
    }
    for endpoint in pair {
        if used==max_spans {return Ok((false,used));}
        let (normal,spent)=normal(endpoint,max_spans-used)?;used+=spent;
        let Some(normal)=normal else {return Ok((false,used));};
        if normal_alignment::inspect_pair_bounds([normal,chord],sine)?.aligned==Some(false) {
            return Ok((true,used));
        }
    }
    Ok((false,used))
}
