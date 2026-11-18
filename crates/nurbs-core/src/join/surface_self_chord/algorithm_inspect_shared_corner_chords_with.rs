//! surface self chord algorithm: inspect_shared_corner_chords_with.
use super::*;
pub fn inspect_shared_corner_chords_with<'a>(sources:[&'a Surface;2],rectangles:[[[f64;2];2];2],
    projection:[f64;3],sine:f64,max_spans:usize,mut cache:Option<&mut [OriginalChordWorkspace<'a>;2]>,
)->Result<(bool,usize)> {
    check(max_spans<=100000 && sine.is_finite() && (0. ..1.).contains(&sine)
        && projection.iter().all(|x|x.is_finite())
        && rectangles.iter().flatten().all(|d|d.iter().all(|x|x.is_finite()) && d[0]<d[1]),
        "Bound shared-corner angle, projection, positive endpoint rectangles and work")?;
    for side in 0..2 {
        sources[side].validate()?;
        normal_alignment::validate_rectangle(sources[side],rectangles[side],max_spans.max(1))?;
    }
    if max_spans==0 || sources.iter().any(|s|s.periodic_u||s.periodic_v) {return Ok((false,0));}
    let corners=|s:&Surface| {
        let counts=[s.control_points.len(),s.control_points[0].len()];
        let degrees=[s.degree_u,s.degree_v];let knots=[&s.knots_u,&s.knots_v];
        let mut out=Vec::new();
        for u in [false,true] {for v in [false,true] {
            let high=[u,v];
            if !(0..2).all(|a| {
                let k=knots[a];let value=k[if high[a]{counts[a]}else{degrees[a]}];
                let end=if high[a]{&k[counts[a]..]}else{&k[..=degrees[a]]};
                end.iter().all(|x|*x==value)
            }) {continue;}
            let p=&s.control_points[if u{counts[0]-1}else{0}][if v{counts[1]-1}else{0}];
            out.push((high,[p[0],p[1],p[2]]));
        }}
        out
    };
    let a=corners(sources[0]);let b=corners(sources[1]);let mut used=0;
    for (ah,ap) in a {for &(bh,bp) in &b {
        if ap!=bp {continue;}
        if used==max_spans {return Ok((false,used));}
        let (proof,spent)=qualify_corner_chords_with(sources,[ah,bh],rectangles,projection,sine,max_spans-used,cache.as_deref_mut())?;
        used+=spent;
        if proof.is_some() {return Ok((true,used));}
    }}
    Ok((false,used))
}
