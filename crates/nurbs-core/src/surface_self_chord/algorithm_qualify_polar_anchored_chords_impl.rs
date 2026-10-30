//! surface self chord algorithm: qualify_polar_anchored_chords_impl.
use super::*;
pub fn qualify_polar_anchored_chords_impl<'a>(
    anchors:[&SurfaceAnchorCertificate<'a>;2],rectangles:[[[f64;2];2];2],
    differential:[Option<&crate::rational_pole_differential::Certificate<'_>>;2],
    normals:[Option<&crate::factored_surface_normals::Certificate<'_>>;2],
    projections:&[[f64;3]],sine:f64,max_spans:usize,adaptive:bool,
)->Result<(Option<CornerChordCertificate<'a>>,usize)> {
    check(anchors[0].world==anchors[1].world&&(1..=100000).contains(&max_spans)
        &&(1..=26).contains(&projections.len())&&projections.iter().flatten().all(|x|x.is_finite())&&sine.is_finite()&&(0. ..1.).contains(&sine),
        "Use identical exact anchors and bounded original polar path work")?;
    let sources=anchors.map(|a|a.surface);
    for side in 0..2 {
        normal_alignment::validate_rectangle(sources[side],rectangles[side],max_spans)?;
        if let Some(d)=differential[side] {
            check(std::ptr::eq(d.surface(),sources[side]),"Use original polar path source")?;
            let s=sources[side];let pole=if d.end(){s.knots_u[s.control_points.len()]}else{s.knots_u[s.degree_u]};
            check(anchors[side].uv[0]==pole,"Exact anchor must lie on the prepared pole")?;
        }
        if let Some(n)=normals[side] {
            check(std::ptr::eq(n.surface(),sources[side])&&differential[side].is_none_or(|d|d.end()==n.end()),
                "Use original matching pole normal factors")?;
        }
    }
    let mut spans=0;let mut generators=Vec::new();
    for side in 0..2 {
        let mut signs=[0.;2];
        let matrix=if let Some(d)=differential[side] {
            if max_spans-spans<6 {return Ok((None,spans));}
            let mut domain=rectangles[side];
            domain[1]=[domain[1][0].min(anchors[side].uv[1]),domain[1][1].max(anchors[side].uv[1])];
            let (matrix,used)=d.coordinate_path_report(domain)?;spans+=used;
            let Some(matrix)=matrix else{return Ok((None,spans));};
            let box_=crate::surface_distance::rectangle_bounds(sources[side],rectangles[side])?;spans+=1;
            let axis=d.anchor();let world=anchors[side].world[axis];
            signs[0]=if box_[axis][0]>=world {1.}else if box_[axis][1]<=world {-1.}else{return Ok((None,spans));};
            signs[1]=if rectangles[side][1][0]>=anchors[side].uv[1] {1.}
                else if rectangles[side][1][1]<=anchors[side].uv[1] {-1.}else{return Ok((None,spans));};
            matrix
        }else{
            for axis in 0..2 {
                signs[axis]=if rectangles[side][axis][0]>=anchors[side].uv[axis] {1.}
                    else if rectangles[side][axis][1]<=anchors[side].uv[axis] {-1.}else{return Ok((None,spans));};
            }
            if spans==max_spans{return Ok((None,spans));}
            let domain=std::array::from_fn(|a|[anchors[side].uv[a].min(rectangles[side][a][0]),anchors[side].uv[a].max(rectangles[side][a][1])]);
            let report=qualify_rectangle(sources[side],domain,sine,max_spans-spans)?;spans+=report.spans;
            let Some(matrix)=report.mean_matrix else{return Ok((None,spans));};matrix.jacobian
        };
        for axis in 0..2 {
            let sign=(if side==0{-1.}else{1.})*signs[axis];let mut g=[[0.;2];3];
            for k in 0..3 {let value=matrix[k][axis].mul(I::point(sign))?;g[k]=[value.lo,value.hi];}
            generators.push(g);
        }
    }
    let proposal=if adaptive {normal_alignment::chord_cone_projection_proposal(&generators)}else{None};
    for side in 0..2 {
        if spans==max_spans {break;}
        let normal=if let Some(n)=normals[side] {spans+=1;Some(n.bounds(rectangles[side])?)}else{
            let (n,used)=normal_alignment::normal_bounds(sources[side],rectangles[side],max_spans-spans)?;spans+=used;n
        };
        if let Some(n)=normal {for projection in proposal.iter().chain(projections.iter()) {
            if normal_alignment::positive_chord_cone_excluded(&generators,n,*projection,sine)? {
                return Ok((Some(CornerChordCertificate{sources,rectangles,max_sine_squared:sine}),spans));
            }
        }}
    }
    Ok((None,spans))
}
