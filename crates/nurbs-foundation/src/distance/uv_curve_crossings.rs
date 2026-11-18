//! Original-definition planar crossing isolation. No tolerance root admission,
//! endpoint welding, tangency classification or partition parameter rounding.
use crate::{check, curve::Curve, curve_jets, distance_bounds::Interval as I, Result};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Excluded,
    Unique,
    Unresolved,
}
pub struct Cell {
    pub parameters: [[f64; 2]; 2],
    pub root: Option<[[f64; 2]; 2]>,
    pub state: State,
}
pub struct Report {
    pub cells: Vec<Cell>,
    pub visited: usize,
    pub complete: bool,
    /// Requested widths use each original curve parameter, not normalized cells.
    pub target_width: Option<[f64; 2]>,
    pub precision_proven: bool,
}
fn point(c: &Curve, span: usize, t: f64) -> Result<Vec<I>> {
    let net = crate::curve_distance::restricted_controls(c, span, I::point(t))?;
    (0..2)
        .map(|k| {
            net[0][k]
                .div(net[0][2])?
                .add(I::point(c.control_points[span - c.degree][k]))
        })
        .collect()
}
fn classify(
    a: &Curve,
    b: &Curve,
    sa: usize,
    sb: usize,
    d: [[f64; 2]; 2],
) -> Result<(State, Option<[[f64; 2]; 2]>)> {
    let ja = curve_jets::enclose(a, sa, d[0])?;
    let jb = curve_jets::enclose(b, sb, d[1])?;
    let t=d.map(|x|x[0]*0.5+x[1]*0.5);
    let derivatives=[vec![ja[1].clone()],vec![jb[1].clone()]];
    classify_enclosed(ja,jb,point(a,sa,t[0])?,point(b,sb,t[1])?,d,derivatives)
}
fn classify_enclosed(ja:Vec<Vec<I>>,jb:Vec<Vec<I>>,pa:Vec<I>,pb:Vec<I>,d:[[f64;2];2],
    derivatives:[Vec<Vec<I>>;2])
 ->Result<(State,Option<[[f64;2];2]>)> {
    if (0..2).any(|k| ja[0][k].hi < jb[0][k].lo || jb[0][k].hi < ja[0][k].lo) {
        return Ok((State::Excluded, None));
    }
    let j = [
        [ja[1][0], I::new(-jb[1][0].hi, -jb[1][0].lo)?],
        [ja[1][1], I::new(-jb[1][1].hi, -jb[1][1].lo)?],
    ];
    let m = j.map(|row| row.map(|x| x.lo * 0.5 + x.hi * 0.5));
    let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
    if !det.is_finite() || det == 0. {
        return Ok((State::Unresolved, None));
    }
    let y = [
        [m[1][1] / det, -m[0][1] / det],
        [-m[1][0] / det, m[0][0] / det],
    ];
    if y.iter().flatten().any(|x| !x.is_finite()) {
        return Ok((State::Unresolved, None));
    }
    let t = d.map(|x| x[0] * 0.5 + x[1] * 0.5);
    let f = [pa[0].sub(pb[0])?, pa[1].sub(pb[1])?];
    let widths = [
        I::point(d[0][1]).sub(I::point(d[0][0]))?,
        I::point(d[1][1]).sub(I::point(d[1][0]))?,
    ];
    let x = [
        I::point(t[0]).sub(I::point(d[0][0]))?.div(widths[0])?,
        I::point(t[1]).sub(I::point(d[1][0]))?.div(widths[1])?,
    ];
    let mut k = [I::point(0.); 2];
    for i in 0..2 {
        k[i] = x[i];
        let mut norm = I::point(0.);
        for r in 0..2 {
            k[i] = k[i].sub(I::point(y[i][r]).mul(f[r])?)?;
        }
        for col in 0..2 {
            // Apply the constant preconditioner before combining original
            // spans. Hulling coordinate derivatives first loses correlation
            // between the two tangent components across a C0 seam.
            let mut entry:Option<I>=None;
            for derivative in &derivatives[col] {
                let mut part=I::point(if i==col {1.}else{0.});
                for r in 0..2 {
                    let signed=if col==0 {derivative[r]}
                        else {I::new(-derivative[r].hi,-derivative[r].lo)?};
                    part=part.sub(I::point(y[i][r]).mul(signed)?)?;
                }
                entry=Some(match entry {None=>part,Some(old)=>
                    I::new(old.lo.min(part.lo),old.hi.max(part.hi))?});
            }
            let entry=entry.ok_or_else(||crate::Error::new("NURBS_CROSSING_BOX","Missing original derivative spans"))?;
            norm = norm.add(I::point(entry.lo.abs().max(entry.hi.abs())))?;
            k[i] = k[i].add(entry.mul(I::new(0., 1.)?.sub(x[col])?)?)?;
        }
        if norm.hi >= 1. || k[i].lo <= 0. || k[i].hi >= 1. {
            return Ok((State::Unresolved, None));
        }
    }
    let mut root = [[0.; 2]; 2];
    for i in 0..2 {
        let r = I::point(d[i][0]).add(widths[i].mul(k[i])?)?;
        root[i] = [r.lo.max(d[i][0]), r.hi.min(d[i][1])];
    }
    Ok((State::Unique, Some(root)))
}
/// Freshly certify an original box, including continuous original knot seams.
/// Strict Krawczyk inclusion/contraction uses the convex hull of every
/// piecewise derivative in the same normalized box coordinates. Validated
/// curves are continuous; the resulting Lipschitz bound includes C0 seams.
pub fn certify_box(a: &Curve, b: &Curve, parameters: [[f64; 2]; 2]) -> Result<Cell> {
    let mut spans=[Vec::new(),Vec::new()];
    for (i,c) in [a,b].iter().enumerate() {
        c.validate()?;
        check(c.control_points[0].len()==2 && !c.periodic,
            "Crossing certification needs original nonperiodic UV curves")?;
        let d=c.domain();let p=parameters[i];
        check(p.iter().all(|x|x.is_finite()) && p[0]<p[1] && d[0]<=p[0] && p[1]<=d[1],
            "Crossing box must have positive finite contained widths")?;
        spans[i]=(c.degree..c.control_points.len()).filter(|&j|
            c.knots[j]<c.knots[j+1] && p[0]<c.knots[j+1] && c.knots[j]<p[1]).collect();
    }
    let (state,root)=if spans.iter().all(|s|s.len()==1) {
        classify(a,b,spans[0][0],spans[1][0],parameters)?
    } else if spans.iter().map(Vec::len).sum::<usize>()>256 {
        (State::Unresolved,None)
    } else {
        let jets=|c:&Curve,spans:&[usize],domain:[f64;2]|->Result<(Vec<Vec<I>>,Vec<Vec<I>>)> {
            let mut hull:Option<Vec<Vec<I>>>=None;
            let mut derivatives=Vec::with_capacity(spans.len());
            let full_width=I::point(domain[1]).sub(I::point(domain[0]))?;
            for &span in spans {
                let part=[domain[0].max(c.knots[span]),domain[1].min(c.knots[span+1])];
                let mut jet=curve_jets::enclose(c,span,part)?;
                // Original span jets differentiate local [0,1]. Convert to
                // the whole selector's [0,1] before taking the derivative hull.
                let scale=full_width.div(I::point(part[1]).sub(I::point(part[0]))?)?;
                jet.truncate(2);
                for axis in 0..2 {jet[1][axis]=jet[1][axis].mul(scale)?;}
                derivatives.push(jet[1].clone());
                if let Some(h)=&mut hull {
                    for order in 0..2 {for axis in 0..2 {
                        h[order][axis]=I::new(h[order][axis].lo.min(jet[order][axis].lo),
                            h[order][axis].hi.max(jet[order][axis].hi))?;
                    }}
                }else{hull=Some(jet);}
            }
            Ok((hull.ok_or_else(||crate::Error::new("NURBS_CROSSING_BOX","Crossing box misses original spans"))?,derivatives))
        };
        let (ja,da)=jets(a,&spans[0],parameters[0])?;
        let (jb,db)=jets(b,&spans[1],parameters[1])?;
        let t=parameters.map(|p|p[0]*0.5+p[1]*0.5);
        let pa=crate::interval_eval::evaluate_interval(a,I::point(t[0]))?;
        let pb=crate::interval_eval::evaluate_interval(b,I::point(t[1]))?;
        classify_enclosed(ja,jb,pa,pb,parameters,[da,db])?
    };
    Ok(Cell{parameters,root,state})
}
/// Search the complete original span product. Every unvisited cell is retained.
/// Only strict Krawczyk inclusion plus contraction admits a unique crossing.
pub fn isolate(a: &Curve, b: &Curve, max_cells: usize) -> Result<Report> {
    isolate_impl(a, b, max_cells, None)
}
/// Tighten isolated roots under the same global work budget. Stopping refinement
/// preserves its last valid root enclosure and marks unmet precision explicitly.
pub fn isolate_refined(
    a: &Curve,
    b: &Curve,
    max_cells: usize,
    target_width: [f64; 2],
) -> Result<Report> {
    check(
        target_width.iter().all(|x| x.is_finite() && *x > 0.),
        "Requested root widths must be finite and positive",
    )?;
    isolate_impl(a, b, max_cells, Some(target_width))
}
fn isolate_impl(
    a: &Curve,
    b: &Curve,
    max_cells: usize,
    target_width: Option<[f64; 2]>,
) -> Result<Report> {
    for c in [a, b] {
        c.validate()?;
        check(
            c.control_points[0].len() == 2 && !c.periodic,
            "Crossing isolation needs original nonperiodic UV curves",
        )?;
    }
    check(
        (1..=100000).contains(&max_cells),
        "Crossing work must be bounded",
    )?;
    let spans = |c: &Curve| {
        (c.degree..c.control_points.len())
            .filter(|&s| c.knots[s] < c.knots[s + 1])
            .collect::<Vec<_>>()
    };
    let aa = spans(a);
    let bb = spans(b);
    check(
        aa.len().checked_mul(bb.len()).is_some_and(|n| n <= 100000),
        "Crossing span product exceeds resource bound",
    )?;
    let mut queue = std::collections::VecDeque::new();
    let enqueue_spans=|queue:&mut std::collections::VecDeque<_>| {
        for &sa in &aa {for &sb in &bb {
            queue.push_back((Some((sa,sb)),[
                [a.knots[sa],a.knots[sa+1]],
                [b.knots[sb],b.knots[sb+1]],
            ],0usize));
        }}
    };
    // A root on an original knot lies on the boundary of both neighboring
    // span cells. Try the complete continuous domain before partitioning it.
    // A global certificate covers the whole product and needs no deduplication.
    if aa.len()>1 || bb.len()>1 {
        queue.push_back((None,[a.domain(),b.domain()],0usize));
    }else{enqueue_spans(&mut queue);}
    let mut out = Report {
        cells: vec![],
        visited: 0,
        complete: true,
        target_width,
        precision_proven: true,
    };
    while let Some((span, d, depth)) = queue.pop_front() {
        let (state, mut root) = if out.visited == max_cells {
            (State::Unresolved, None)
        } else {
            out.visited += 1;
            if let Some((sa,sb))=span {classify(a,b,sa,sb,d)?}
            else {let c=certify_box(a,b,d)?;(c.state,c.root)}
        };
        if span.is_none() && state==State::Unresolved && out.visited<max_cells {
            enqueue_spans(&mut queue);continue;
        }
        if state == State::Unresolved && out.visited < max_cells && depth < 32 {
            let axis = depth % 2;
            let mid = d[axis][0] * 0.5 + d[axis][1] * 0.5;
            if mid > d[axis][0] && mid < d[axis][1] {
                let mut left = d;
                left[axis][1] = mid;
                let mut right = d;
                right[axis][0] = mid;
                queue.push_back((span, left, depth + 1));
                queue.push_back((span, right, depth + 1));
                continue;
            }
        }
        if let (State::Unique, Some(target), Some(mut current)) = (state, target_width, root) {
            let mut owner = d;
            while (0..2).any(|i| (current[i][1] - current[i][0]).next_up() > target[i])
                && out.visited < max_cells
            {
                let mut candidate = current;
                for i in 0..2 {
                    let pad = (current[i][1] - current[i][0]) * 0.5;
                    candidate[i] = [
                        (current[i][0] - pad).next_down().max(owner[i][0]),
                        (current[i][1] + pad).next_up().min(owner[i][1]),
                    ];
                }
                if (0..2).any(|i| candidate[i][0] >= candidate[i][1]) || candidate == owner {
                    break;
                }
                out.visited += 1;
                let (next,enclosure)=if let Some((sa,sb))=span {classify(a,b,sa,sb,candidate)?}
                    else {let c=certify_box(a,b,candidate)?;(c.state,c.root)};
                crate::numeric(
                    next != State::Excluded,
                    "Refinement excluded a previously certified crossing",
                )?;
                if let (State::Unique, Some(narrower)) = (next, enclosure) {
                    let mut intersection = narrower;
                    for i in 0..2 {
                        intersection[i] = [
                            current[i][0].max(narrower[i][0]),
                            current[i][1].min(narrower[i][1]),
                        ];
                        crate::numeric(
                            intersection[i][0] <= intersection[i][1],
                            "Certified crossing enclosures became disjoint",
                        )?;
                    }
                    if intersection == current {
                        break;
                    }
                    current = intersection;
                    owner = candidate;
                } else {
                    break;
                }
            }
            root = Some(current);
            out.precision_proven &=
                (0..2).all(|i| (current[i][1] - current[i][0]).next_up() <= target[i]);
        }
        out.complete &= state != State::Unresolved;
        out.cells.push(Cell {
            parameters: d,
            root,
            state,
        });
    }
    out.precision_proven &= out.complete;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn continuous_original_knot_crossing_has_one_global_contraction() {
        let a=Curve{degree:1,knots:vec![0.,0.,0.5,1.,1.],
            control_points:vec![vec![0.,0.],vec![0.5,0.125],vec![1.,0.]],
            weights:vec![1.;3],periodic:false};
        let b=Curve{degree:1,knots:vec![2.,2.,3.5,6.5,8.,8.],
            control_points:vec![vec![0.5,-1.],vec![0.5,-0.5],vec![0.5,0.5],vec![0.5,1.]],
            weights:vec![1.;4],periodic:false};
        let result=certify_box(&a,&b,[[0.,1.],[2.,8.]]).unwrap();
        assert_eq!(result.state,State::Unique);
        let root=result.root.unwrap();
        assert!(root[0][0]<=0.5 && 0.5<=root[0][1]);
        assert!(root[1][0]<=5.375 && 5.375<=root[1][1]);
        let isolated=isolate(&a,&b,64).unwrap();
        assert!(isolated.complete && isolated.precision_proven);
        assert_eq!(isolated.cells.len(),1);
        assert_eq!(isolated.cells[0].state,State::Unique);
        assert_eq!(isolated.visited,1);
        let refined=isolate_refined(&a,&b,64,[1e-8,1e-8]).unwrap();
        assert!(refined.complete && refined.precision_proven);
        assert_eq!(refined.cells.len(),1);
        assert!(refined.cells[0].root.unwrap().iter().all(|r|r[1]-r[0]<=1e-8));
        let swapped=certify_box(&b,&a,[[2.,8.],[0.,1.]]).unwrap();
        assert_eq!(swapped.state,State::Unique);
        let mut turning=a.clone();turning.control_points=vec![vec![0.,0.],vec![1.,0.],vec![0.,0.]];
        assert_eq!(certify_box(&turning,&b,[[0.,1.],[2.,8.]]).unwrap().state,State::Unresolved);
        let stopped=isolate(&turning,&b,1).unwrap();
        assert!(!stopped.complete && !stopped.precision_proven);
        assert_eq!(stopped.visited,1);
        assert_eq!(stopped.cells.len(),1);
        assert_eq!(stopped.cells[0].parameters,[[0.,1.],[2.,8.]]);
        assert_eq!(stopped.cells[0].state,State::Unresolved);
        let mut disconnected=a.clone();disconnected.knots=vec![0.,0.,0.5,0.5,1.,1.];
        disconnected.control_points.insert(2,vec![0.75,0.]);disconnected.weights.push(1.);
        assert!(certify_box(&disconnected,&b,[[0.,1.],[2.,8.]]).is_err());
    }
    #[test]
    fn rational_quadratic_and_cubic_c0_seams_keep_original_root_boxes() {
        let vertical=Curve{degree:1,knots:vec![10.,10.,14.,14.],
            control_points:vec![vec![0.5,-1.],vec![0.5,1.]],
            weights:vec![1.,1.],periodic:false};
        for (degree,x,weights) in [
            (2,vec![0.,0.25,0.5,0.75,1.],vec![1.,1.125,1.,1.125,1.]),
            (3,vec![0.,0.125,0.375,0.5,0.625,0.875,1.],
                vec![1.,1.125,1.0625,1.,1.0625,1.125,1.]),
        ] {
            let mut knots=vec![2.;degree+1];knots.extend(vec![5.;degree]);knots.extend(vec![8.;degree+1]);
            let curve=Curve{degree,knots,control_points:x.into_iter()
                .map(|x|vec![x,0.25*f64::min(x,1.-x)]).collect(),weights,periodic:false};
            curve.validate().unwrap();
            let result=certify_box(&curve,&vertical,[[4.75,5.25],[12.,12.5]]).unwrap();
            assert_eq!(result.state,State::Unique,"degree {degree}");
            let root=result.root.unwrap();
            assert!(root[0][0]<=5. && 5.<=root[0][1]);
            assert!(root[1][0]<=12.25 && 12.25<=root[1][1]);
            let reverse=certify_box(&vertical,&curve,[[12.,12.5],[4.75,5.25]]).unwrap();
            assert_eq!(reverse.state,State::Unique);
            // The original rational curves are also checked after an exact
            // invertible affine map; this is no longer an axis-aligned crossing.
            let affine=|source:&Curve| {
                let mut mapped=source.clone();
                for p in &mut mapped.control_points {
                    let (x,y)=(p[0],p[1]);p[0]=1048576.+x+y;p[1]=-1048576.+x+2.*y;
                }
                mapped
            };
            let transformed=certify_box(&affine(&curve),&affine(&vertical),
                [[4.75,5.25],[12.,12.5]]).unwrap();
            assert_eq!(transformed.state,State::Unique);
            let transformed_root=transformed.root.unwrap();
            assert!(transformed_root[0][0]<=5. && 5.<=transformed_root[0][1]);
            assert!(transformed_root[1][0]<=12.25 && 12.25<=transformed_root[1][1]);
            assert_eq!(certify_box(&curve,&curve,[[4.75,5.25];2]).unwrap().state,State::Unresolved);
            assert_eq!(certify_box(&curve,&vertical,[[2.,2.25],[12.,12.5]]).unwrap().state,State::Excluded);
        }
    }
    #[test]
    fn crossing_nonunit_domains_and_excluded_pair() {
        let mut a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let mut b = Curve::from_polyline(vec![vec![0., 1.], vec![1., 0.]]).unwrap();
        a.knots = vec![2., 2., 8., 8.];
        b.knots = vec![10., 10., 14., 14.];
        let r = isolate(&a, &b, 64).unwrap();
        assert!(r.complete);
        let roots = r
            .cells
            .iter()
            .filter(|c| c.state == State::Unique)
            .collect::<Vec<_>>();
        assert_eq!(roots.len(), 1);
        let x = roots[0].root.unwrap();
        assert!(x[0][0] <= 5. && x[0][1] >= 5. && x[1][0] <= 12. && x[1][1] >= 12.);
        for p in &mut b.control_points {
            p[1] += 3.;
        }
        let r = isolate(&a, &b, 64).unwrap();
        assert!(r.complete && r.cells.iter().all(|c| c.state == State::Excluded));
    }
    #[test]
    fn coincidence_and_budget_stop_never_admit_crossings() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let r = isolate(&a, &a, 3).unwrap();
        assert!(!r.complete);
        assert_eq!(r.visited, 3);
        assert!(r.cells.iter().all(|c| c.state != State::Unique));
    }
    #[test]
    fn rational_arc_crossing_preserves_original_parameters() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: vec![1., 0.5f64.sqrt(), 1.],
            periodic: false,
        };
        let b = Curve::from_polyline(vec![vec![0., 0.3], vec![1., 0.3]]).unwrap();
        let r = isolate(&a, &b, 4096).unwrap();
        assert!(r.complete, "{} visited", r.visited);
        let roots = r
            .cells
            .iter()
            .filter(|c| c.state == State::Unique)
            .collect::<Vec<_>>();
        assert_eq!(roots.len(), 1);
        let x = roots[0].root.unwrap();
        let expected = (1_f64 - 0.3 * 0.3).sqrt();
        assert!(x[1][0] <= expected && expected <= x[1][1]);
        let mut near = b;
        for p in &mut near.control_points {
            p[1] = 1.000000000001;
        }
        let r = isolate(&a, &near, 4096).unwrap();
        assert!(r.cells.iter().all(|c| c.state != State::Unique));
    }
    #[test]
    fn refinement_preserves_certified_root_when_requested_precision_cannot_be_reached() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![0., 1.], vec![1., 0.]]).unwrap();
        let r = isolate_refined(&a, &b, 1, [1e-30, 1e-30]).unwrap();
        assert!(r.complete && !r.precision_proven);
        assert_eq!(r.visited, 1);
        assert_eq!(r.cells[0].state, State::Unique);
        assert!(r.cells[0]
            .root
            .unwrap()
            .iter()
            .all(|x| x[0] <= 0.5 && x[1] >= 0.5));
    }
    #[test]
    fn rational_crossing_refines_to_requested_original_parameter_widths() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: vec![1., 0.5f64.sqrt(), 1.],
            periodic: false,
        };
        let b = Curve::from_polyline(vec![vec![0., 0.3], vec![1., 0.3]]).unwrap();
        let r = isolate_refined(&a, &b, 4096, [1e-8, 1e-8]).unwrap();
        assert!(r.complete && r.precision_proven);
        let roots = r.cells.iter().filter_map(|c| c.root).collect::<Vec<_>>();
        assert_eq!(roots.len(), 1);
        assert!(roots[0].iter().all(|x| x[1] - x[0] <= 1e-8));
        let expected = (1_f64 - 0.3 * 0.3).sqrt();
        assert!(roots[0][1][0] <= expected && expected <= roots[0][1][1]);
    }
    #[test]
    fn supplied_boxes_are_rechecked_against_original_spans() {
        let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let b = Curve::from_polyline(vec![vec![0., 1.], vec![1., 0.]]).unwrap();
        assert_eq!(
            certify_box(&a, &b, [[0., 1.]; 2]).unwrap().state,
            State::Unique
        );
        assert_eq!(
            certify_box(&a, &b, [[0., 0.25]; 2]).unwrap().state,
            State::Excluded
        );
        assert!(certify_box(&a, &b, [[-0.1, 1.], [0., 1.]]).is_err());
        assert!(certify_box(&a, &b, [[0.5, 0.5], [0., 1.]]).is_err());
    }
}
