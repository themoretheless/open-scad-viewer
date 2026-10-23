use super::*;

/// Nonzero analytic winding. An isolated clockwise loop still has an inside.
pub fn locate_point(loops: &[Vec<Curve>], point: Point, tolerance: f64) -> Result<PointLocation> {
    if point.iter().any(|x| !x.is_finite()) {
        return Err(invalid("Planar query point must be finite"));
    }
    let region = Region::parse(loops, tolerance)?;
    region.location(sub(point, region.origin))
}

/// Give adjacent analytic cells the same retained edge parameterization. All
/// endpoints come from the common arrangement; no control point is fitted or
/// moved. Coincident spans select one source trim (possibly reversed).
pub(crate) fn conform_profiles(
    profiles: &[Vec<Vec<Curve>>],
    tolerance: f64,
) -> Result<Vec<Vec<Vec<Curve>>>> {
    let origin = profiles
        .iter()
        .find(|p| !p.is_empty())
        .map_or([0., 0.], |p| region_origin(p));
    let regions = profiles
        .iter()
        .map(|p| Region::parse_at(p, tolerance, origin))
        .collect::<Result<Vec<_>>>()?;
    for region in &regions {
        region.validate()?;
    }
    let spans: Vec<_> = regions.iter().flat_map(|r| &r.spans).collect();
    if spans.len() > MAX_SPANS {
        return Err(limit());
    }
    let epsilon = regions.iter().map(|r| r.epsilon).fold(0., f64::max);
    let mut cuts: Vec<_> = spans.iter().map(|s| s.curve.domain().to_vec()).collect();
    for i in 0..spans.len() {
        for j in i + 1..spans.len() {
            for p in intersections(spans[i], spans[j], epsilon, tolerance)? {
                cuts[i].push(spans[i].parameter(p, epsilon * 8.)?);
                cuts[j].push(spans[j].parameter(p, epsilon * 8.)?);
            }
        }
    }
    let mut canonical = Vec::<Span>::new();
    let mut split = Vec::new();
    let mut work = 0;
    for (span, mut cut) in spans.into_iter().zip(cuts) {
        cut.sort_by(f64::total_cmp);
        cut.dedup();
        let mut pieces = Vec::new();
        for pair in cut.windows(2) {
            work += 1;
            if work > MAX_FRAGMENTS {
                return Err(limit());
            }
            if pair[1] - pair[0] <= 64. * f64::EPSILON * (1. + pair[0].abs()) {
                if distance(
                    point(&span.local_curve, pair[0])?,
                    point(&span.local_curve, pair[1])?,
                ) > epsilon * 8.
                {
                    return Err(ambiguous(
                        "Common cell events exceed source parameter precision",
                    ));
                }
                continue;
            }
            let mut curve = span.curve.trim(pair[0], pair[1])?;
            let [low, high] = curve.domain();
            for k in &mut curve.knots {
                *k = (*k - low) / (high - low);
            }
            let piece = Span::from_curve(curve, 0, epsilon, origin)?;
            let shared = canonical.iter().find_map(|c| {
                if !same_carrier(c, &piece, epsilon) {
                    return None;
                }
                if distance(c.p0, piece.p0) <= epsilon * 16.
                    && distance(c.p1, piece.p1) <= epsilon * 16.
                {
                    Some((c, false))
                } else if distance(c.p0, piece.p1) <= epsilon * 16.
                    && distance(c.p1, piece.p0) <= epsilon * 16.
                {
                    Some((c, true))
                } else {
                    None
                }
            });
            pieces.push(if let Some((c, reverse)) = shared {
                if reverse {
                    c.curve.reverse()?
                } else {
                    c.curve.clone()
                }
            } else {
                let curve = piece.curve.clone();
                canonical.push(piece);
                curve
            });
        }
        split.push(pieces);
    }
    let mut offset = 0;
    let mut output = Vec::new();
    for region in regions {
        let profile: Vec<Vec<Curve>> = region
            .loops
            .iter()
            .map(|wire| {
                wire.iter()
                    .flat_map(|i| split[offset + i].clone())
                    .collect()
            })
            .collect();
        offset += region.spans.len();
        validate(&profile, tolerance)?;
        output.push(profile);
    }
    Ok(output)
}

#[derive(Clone)]
struct Fragment {
    span: Span,
    source_loop: usize,
}
fn material(operation: &str, a: bool, b: bool) -> bool {
    match operation {
        "union" => a || b,
        "intersection" => a && b,
        "difference" => a && !b,
        "xor" => a != b,
        _ => false,
    }
}

/// Regularized Boolean of material-left planar loops. Empty output is valid.
/// A point contact contributes no area; separate tangent components remain
/// separate loops. Near-coincidence/ambiguous sewing fails without fallback.
pub fn boolean(
    a: &[Vec<Curve>],
    b: &[Vec<Curve>],
    operation: &str,
    tolerance: f64,
) -> Result<Vec<Vec<Curve>>> {
    if !matches!(operation, "union" | "intersection" | "difference" | "xor") {
        return Err(invalid(
            "Planar Boolean must be union, intersection, difference or xor",
        ));
    }
    let origin = if a.iter().any(|wire| !wire.is_empty()) {
        region_origin(a)
    } else {
        region_origin(b)
    };
    let a = Region::parse_at(a, tolerance, origin)?;
    let b = Region::parse_at(b, tolerance, origin)?;
    a.validate()?;
    b.validate()?;
    if a.spans.len() + b.spans.len() > MAX_SPANS {
        return Err(limit());
    }
    let epsilon = a.epsilon.max(b.epsilon);
    let mut acuts: Vec<Vec<f64>> = a.spans.iter().map(|s| s.curve.domain().to_vec()).collect();
    let mut bcuts: Vec<Vec<f64>> = b.spans.iter().map(|s| s.curve.domain().to_vec()).collect();
    for (i, sa) in a.spans.iter().enumerate() {
        for (j, sb) in b.spans.iter().enumerate() {
            for p in intersections(sa, sb, epsilon, tolerance)? {
                acuts[i].push(sa.parameter(p, epsilon * 8.)?);
                bcuts[j].push(sb.parameter(p, epsilon * 8.)?);
            }
        }
    }
    let mut fragments: Vec<Fragment> = Vec::new();
    let mut fragment_work = 0usize;
    for (from_a, source, other, cuts) in [(true, &a, &b, &mut acuts), (false, &b, &a, &mut bcuts)] {
        for (span, cut) in source.spans.iter().zip(cuts) {
            cut.sort_by(f64::total_cmp);
            cut.dedup_by(|a, b| *a == *b);
            for parameters in cut.windows(2) {
                fragment_work += 1;
                if fragment_work > MAX_FRAGMENTS {
                    return Err(limit());
                }
                if parameters[1] - parameters[0] <= 64. * f64::EPSILON * (1. + parameters[0].abs())
                {
                    if distance(
                        point(&span.local_curve, parameters[0])?,
                        point(&span.local_curve, parameters[1])?,
                    ) > epsilon * 8.
                    {
                        return Err(ambiguous(
                            "Distinct geometric events cannot be isolated in the source parameter precision",
                        ));
                    }
                    continue;
                }
                let curve = span.curve.trim(parameters[0], parameters[1])?;
                let piece = Span::from_curve(curve, span.loop_id, epsilon, source.origin)?;
                let midpoint = piece.position(0.5)?;
                let (other_left, other_right) = match other.location(midpoint)? {
                    PointLocation::Inside => (true, true),
                    PointLocation::Outside => (false, false),
                    PointLocation::Boundary => {
                        let coincident: Vec<_> = other
                            .spans
                            .iter()
                            .filter(|s| {
                                same_carrier(&piece, s, epsilon)
                                    && s.contains(midpoint, epsilon * 16.)
                            })
                            .collect();
                        if coincident.is_empty() {
                            return Err(ambiguous(
                                "Fragment classification falls in a noncoincident boundary tolerance band",
                            ));
                        }
                        let tangent = piece.tangent(0.5)?;
                        let mut direction = None;
                        for neighbor in coincident {
                            let domain = neighbor.curve.domain();
                            let t = (neighbor.parameter(midpoint, epsilon * 8.)? - domain[0])
                                / (domain[1] - domain[0]);
                            let same = dot(tangent, neighbor.tangent(t)?) > 0.;
                            if direction.is_some_and(|d| d != same) {
                                return Err(ambiguous(
                                    "Conflicting coincident boundary material sides",
                                ));
                            }
                            direction = Some(same);
                        }
                        let same = direction.unwrap();
                        (same, !same)
                    }
                };
                let left = if from_a {
                    material(operation, true, other_left)
                } else {
                    material(operation, other_left, true)
                };
                let right = if from_a {
                    material(operation, false, other_right)
                } else {
                    material(operation, other_right, false)
                };
                if left == right {
                    continue;
                }
                let selected = if left {
                    piece
                } else {
                    Span::from_curve(
                        piece.curve.reverse()?,
                        piece.loop_id,
                        epsilon,
                        source.origin,
                    )?
                };
                let duplicate = fragments.iter().any(|f| {
                    same_carrier(&selected, &f.span, epsilon)
                        && distance(selected.p0, f.span.p0) <= epsilon * 16.
                        && distance(selected.p1, f.span.p1) <= epsilon * 16.
                });
                if !duplicate {
                    fragments.push(Fragment {
                        span: selected,
                        source_loop: span.loop_id + if from_a { 0 } else { a.loops.len() },
                    });
                }
            }
        }
    }
    let mut used = vec![false; fragments.len()];
    let mut loops = Vec::new();
    while let Some(first) = used.iter().position(|v| !*v) {
        let mut current = first;
        let mut chain = Vec::new();
        loop {
            if used[current] {
                return Err(ambiguous("Planar sewing reused an oriented fragment"));
            }
            used[current] = true;
            chain.push(fragments[current].span.curve.clone());
            let end = fragments[current].span.p1;
            if distance(end, fragments[first].span.p0) <= epsilon * 16. {
                break;
            }
            let mut candidates: Vec<_> = (0..fragments.len())
                .filter(|i| !used[*i] && distance(end, fragments[*i].span.p0) <= epsilon * 16.)
                .collect();
            if candidates.is_empty() {
                return Err(ambiguous(
                    "Planar arrangement has an unmatched boundary endpoint",
                ));
            }
            if candidates.len() > 1 {
                let same: Vec<_> = candidates
                    .iter()
                    .copied()
                    .filter(|i| fragments[*i].source_loop == fragments[current].source_loop)
                    .collect();
                if same.len() == 1 {
                    candidates = same;
                } else {
                    let incoming = fragments[current].span.tangent(1.)?;
                    let mut angles = candidates
                        .iter()
                        .map(|i| {
                            let outgoing = fragments[*i].span.tangent(0.)?;
                            Ok((
                                *i,
                                cross(incoming, outgoing)
                                    .atan2(dot(incoming, outgoing))
                                    .rem_euclid(TAU),
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?;
                    angles.sort_by(|a, b| a.1.total_cmp(&b.1));
                    if angles.len() > 1 && (angles[0].1 - angles[1].1).abs() < 1e-10 {
                        return Err(ambiguous("Unresolved tangent junction in planar sewing"));
                    }
                    candidates = vec![angles[0].0];
                }
            }
            current = candidates[0];
        }
        if signed_area(&chain, tolerance)?.abs() <= tolerance * tolerance {
            return Err(ambiguous("Selected boundary has unresolved area"));
        }
        loops.push(chain);
    }
    validate(&loops, tolerance)?;
    Ok(loops)
}

/// Euclidean parallel region with round joins. Boundary tubes are analytic
/// line strips / circular bands plus endpoint disks. Union dilates material;
/// subtraction erodes it, including all holes and disconnected components.
/// Uses the same numerical admission and resource limits as planar Boolean.
pub fn offset(loops:&[Vec<Curve>],amount:f64,tolerance:f64)->Result<Vec<Vec<Curve>>>{
 let region=Region::parse(loops,tolerance)?;region.validate()?;
 if !amount.is_finite()||amount.abs()>1e6{return Err(invalid("Invalid profile offset distance"));}
 if amount==0.||region.spans.is_empty(){return Ok(loops.to_vec());}
 let width=amount.abs();
 if width<=region.epsilon*16.{return Err(ambiguous("Offset distance is below coordinate resolution"));}
 let operation=if amount>0.{"union"}else{"difference"};
 let world=|p:Point|add(p,region.origin);
 let line=|a:Point,b:Point|Curve::from_polyline(vec![world(a).to_vec(),world(b).to_vec()]);
 let mut result=loops.to_vec();let mut endpoints=Vec::new();
 for span in &region.spans{
  let band=match span.carrier{
   Carrier::Line=>{
    let n=mul(left(sub(span.p1,span.p0)),width/distance(span.p0,span.p1));
    crate::sketch::polygon_wire(vec![world(add(span.p0,n)),world(sub(span.p0,n)),world(sub(span.p1,n)),world(add(span.p1,n))])?
   }
   Carrier::Circle{center,radius,..}=>{
    let scaled=|r:f64|->Result<Curve>{let mut c=span.local_curve.clone();for p in &mut c.control_points{let q=world(add(center,mul(sub([p[0],p[1]],center),r/radius)));p[0]=q[0];p[1]=q[1];}c.validate()?;Ok(c)};
    let outer=scaled(radius+width)?;
    let out0=add(center,mul(sub(span.p0,center),(radius+width)/radius));
    let out1=add(center,mul(sub(span.p1,center),(radius+width)/radius));
    let inner_radius=radius-width;
    if inner_radius>0.{
     if inner_radius<=region.epsilon*16.{return Err(ambiguous("Offset approaches a circular cusp below resolution"));}
     let inner=scaled(inner_radius)?;
     let in0=add(center,mul(sub(span.p0,center),inner_radius/radius));
     let in1=add(center,mul(sub(span.p1,center),inner_radius/radius));
     vec![outer,line(out1,in1)?,inner.reverse()?,line(in0,out0)?]
    }else{vec![outer,line(out1,center)?,line(center,out0)?]}
   }
  };
  let band=orient_even_odd(&[band],tolerance)?;
  result=boolean(&result,&band,operation,tolerance)?;
  if result.is_empty(){return Ok(result);}
  for p in [span.p0,span.p1]{if !endpoints.contains(&p){endpoints.push(p);}}
 }
 for endpoint in endpoints{
  let center=world(endpoint);let mut disk=crate::sketch::circle_wire(width)?;
  for curve in &mut disk{for p in &mut curve.control_points{p[0]+=center[0];p[1]+=center[1];}}
  result=boolean(&result,&[disk],operation,tolerance)?;
  if result.is_empty(){return Ok(result);}
 }
 validate(&result,tolerance)?;Ok(result)
}
#[cfg(test)]mod offset_tests{
 use super::*;
 fn area(loops:&[Vec<Curve>])->f64{loops.iter().map(|w|signed_area(w,1e-7).unwrap()).sum()}
 fn rectangle(w:f64,h:f64)->Vec<Curve>{crate::sketch::polygon_wire(vec![[0.,0.],[w,0.],[w,h],[0.,h]]).unwrap()}
 #[test]fn rectangle_round_expansion_and_sharp_erosion(){
  let source=vec![rectangle(6.,4.)];
  for (d,expected) in [(1.,24.+20.+PI),(-1.,8.)]{let r=offset(&source,d,1e-7).unwrap();assert!((area(&r)-expected).abs()<1e-7,"{d}: {}",area(&r));}
 }
 #[test]fn circle_radius_changes_and_empty_erosion(){
  let source=vec![crate::sketch::circle_wire(2.).unwrap()];
  for (d,expected) in [(1.,9.*PI),(-0.5,2.25*PI)]{let r=offset(&source,d,1e-7).unwrap();assert!((area(&r)-expected).abs()<1e-7,"{d}: {}",area(&r));}
  assert!(offset(&source,-3.,1e-7).unwrap().is_empty());
 }
 #[test]fn mixed_semicircle_rounds_outer_corners_and_erodes_to_a_circular_segment(){
  let mut wire=crate::sketch::circle_wire(2.).unwrap();wire.truncate(2);wire.push(Curve::from_polyline(vec![vec![-2.,0.],vec![2.,0.]]).unwrap());
  let grown=offset(&[wire.clone()],0.5,1e-7).unwrap();assert!((area(&grown)-(3.25*PI+2.)).abs()<1e-7);
  let eroded=offset(&[wire],-0.5,1e-7).unwrap();let expected=2.25*(1_f64/3.).acos()-0.5*2_f64.sqrt();assert!((area(&eroded)-expected).abs()<1e-7);
 }
 #[test]fn holes_shrink_or_grow_with_material(){
  let mut hole=crate::sketch::circle_wire(1.).unwrap();for c in &mut hole{for p in &mut c.control_points{p[0]+=4.;p[1]+=3.;}}
  let source=orient_even_odd(&[rectangle(8.,6.),hole],1e-7).unwrap();
  for (d,expected) in [(0.5,62.),(-0.5,35.-2.25*PI)]{let r=offset(&source,d,1e-7).unwrap();assert_eq!(r.len(),2);assert!((area(&r)-expected).abs()<1e-7,"{d}: {}",area(&r));}
 }
}

#[cfg(test)]mod offset_topology_tests{
 use super::*;
 fn disk(x:f64,r:f64)->Vec<Curve>{let mut wire=crate::sketch::circle_wire(r).unwrap();for c in &mut wire{for p in &mut c.control_points{p[0]+=x;}}wire}
 fn area(loops:&[Vec<Curve>])->f64{loops.iter().map(|w|signed_area(w,1e-7).unwrap()).sum()}
 #[test]fn expansion_merges_separate_islands(){
  let source=vec![disk(-1.5,1.),disk(1.5,1.)];
  let r=offset(&source,0.6,1e-7).unwrap();assert_eq!(r.len(),1);
  let radius:f64=1.6;let lens=2.*radius*radius*(3./(2.*radius)).acos()-1.5*(4.*radius*radius-9.).sqrt();
  assert!((area(&r)-(2.*PI*radius*radius-lens)).abs()<1e-7);
 }
 #[test]fn expansion_removes_small_holes(){
  let source=orient_even_odd(&[disk(0.,4.),disk(0.,1.)],1e-7).unwrap();
  let r=offset(&source,1.5,1e-7).unwrap();assert_eq!(r.len(),1);assert!((area(&r)-PI*5.5*5.5).abs()<1e-7);
 }
 #[test]fn erosion_can_split_a_narrow_neck(){
  let ring=crate::sketch::polygon_wire(vec![[-3.,-2.],[-1.,-2.],[-1.,-0.2],[1.,-0.2],[1.,-2.],[3.,-2.],[3.,2.],[1.,2.],[1.,0.2],[-1.,0.2],[-1.,2.],[-3.,2.]]).unwrap();
  let r=offset(&[ring],-0.3,1e-7).unwrap();assert_eq!(r.len(),2);validate(&r,1e-7).unwrap();
  let extra=0.12-0.2*0.05_f64.sqrt()-0.09*(2_f64/3.).asin();assert!((area(&r)-2.*(4.76+extra)).abs()<1e-7);
 }
 #[test]fn twenty_successive_offsets_preserve_the_circle(){
  let mut result=vec![disk(0.,2.)];
  for _ in 0..20{result=offset(&result,0.01,1e-7).unwrap();}
  assert!((area(&result)-PI*2.2*2.2).abs()<1e-7);
  assert!(result.iter().flatten().count()<=16);
 }
 #[test]fn refuses_unresolved_and_nonfinite_offsets(){
  let source=vec![disk(0.,2.)];assert!(offset(&source,f64::NAN,1e-7).is_err());assert!(offset(&source,1e-20,1e-7).is_err());
 }
}
