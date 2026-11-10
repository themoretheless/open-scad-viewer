//! surface weight profile operations: section.
use super::*;
impl<'source> WeightProfile<'source> {
/// Exact new cut boundary for subsequent endpoint surface construction.
    /// Creating the carrier does not trim the face or sew a closed Body.
    pub fn section(&self, axis: usize, parameter: f64) -> Result<ProfileSection<'_, 'source>> {
        check(
            axis < 2 && parameter.is_finite(),
            "Choose a finite profile section coordinate",
        )?;
        let s = self.source;
        let knots = [&s.knots_u, &s.knots_v];
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let range = [
            knots[1 - axis][degrees[1 - axis]],
            knots[1 - axis][counts[1 - axis]],
        ];
        check(
            parameter >= knots[axis][degrees[axis]] && parameter <= knots[axis][counts[axis]],
            "Section coordinate lies inside the original natural chart",
        )?;
        Ok(ProfileSection {
            profile: self,
            axis,
            parameter,
            range,
        })
    }
pub fn source(&self) -> &Surface {
        self.source
    }
pub fn factors(&self) -> (&[f64], &[f64], f64) {
        (&self.row, &self.column, self.pivot)
    }
pub fn axis_deviation_upper(&self) -> [f64; 3] {
        self.axis_deviation_upper
    }
pub fn weight_deviation_upper(&self) -> f64 {
        self.weight_deviation_upper
    }
/// Certified enclosure of the exact proposed profile over a complete
    /// original parameter rectangle. No target weight is rounded to f64.
    pub fn enclose(
        &self,
        domain: [[f64; 2]; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        crate::normal_alignment::validate_rectangle(self.source, domain, max_spans)?;
        let s = self.source;
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let knots = [&s.knots_u, &s.knots_v];
        let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
        let mut spent = 0;
        for u in degrees[0]..counts[0] {
            for v in degrees[1]..counts[1] {
                let span = [
                    [knots[0][u], knots[0][u + 1]],
                    [knots[1][v], knots[1][v + 1]],
                ];
                if span.iter().any(|a| a[0] == a[1]) {
                    continue;
                }
                let section = std::array::from_fn(|a| {
                    [span[a][0].max(domain[a][0]), span[a][1].min(domain[a][1])]
                });
                if section.iter().any(|a| a[0] > a[1]) {
                    continue;
                }
                if spent == max_spans {
                    return Ok((None, spent));
                }
                let net = self.homogeneous_on([u, v], span)?;
                let values = crate::normal_alignment::restrict_hull(&net, degrees, span, section)?;
                for k in 0..3 {
                    let r = values[k].div(values[3])?;
                    hull[k][0] = hull[k][0].min(r.lo);
                    hull[k][1] = hull[k][1].max(r.hi);
                }
                spent += 1;
            }
        }
        if spent == 0 {
            return Ok((None, 0));
        }
        Ok((Some(hull), spent))
    }
/// Quotient derivatives of the exact factored homogeneous expression.
    /// Every original span intersecting the rectangle contributes; incomplete
    /// work returns no certificate. No replacement binary64 weights are made.
    pub fn derivative_bounds(
        &self,
        domain: [[f64; 2]; 2],
        max_spans: usize,
    ) -> Result<(Option<[[[f64; 2]; 2]; 3]>, usize)> {
        crate::normal_alignment::validate_rectangle(self.source, domain, max_spans)?;
        let s = self.source;
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let knots = [&s.knots_u, &s.knots_v];
        let mut hull = [[[f64::INFINITY, f64::NEG_INFINITY]; 2]; 3];
        let mut spent = 0;
        for u in degrees[0]..counts[0] {
            for v in degrees[1]..counts[1] {
                let span = [
                    [knots[0][u], knots[0][u + 1]],
                    [knots[1][v], knots[1][v + 1]],
                ];
                if span.iter().any(|r| r[0] == r[1]) {
                    continue;
                }
                let section = std::array::from_fn(|a| {
                    [span[a][0].max(domain[a][0]), span[a][1].min(domain[a][1])]
                });
                if section.iter().any(|r| r[0] > r[1]) {
                    continue;
                }
                if spent == max_spans {
                    return Ok((None, spent));
                }
                let values = crate::normal_alignment::jacobian_net_on(
                    self.homogeneous_centered_on([u,v],span,&s.control_points[u][v])?,
                    degrees,
                    span,
                    section,
                    &[0.;3],
                )?;
                for k in 0..3 {
                    for axis in 0..2 {
                        hull[k][axis][0] = hull[k][axis][0].min(values[k][axis].lo);
                        hull[k][axis][1] = hull[k][axis][1].max(values[k][axis].hi);
                    }
                }
                spent += 1;
            }
        }
        Ok((if spent == 0 { None } else { Some(hull) }, spent))
    }
/// Bound S(end,free)-S(start,free) by integrating one common derivative
    /// enclosure. Absolute position boxes are never subtracted. Continuity
    /// across the intervening knots is required explicitly.
    pub fn coordinate_difference_bounds(
        &self, axis:usize, parameters:[f64;2], free:[f64;2], max_spans:usize,
    ) -> Result<(Option<[[f64;2];3]>,usize)> {
        check(axis<2 && parameters.iter().all(|x|x.is_finite()),"Require finite source difference coordinates")?;
        let mut domain=[free;2];
        domain[axis]=[parameters[0].min(parameters[1]),parameters[0].max(parameters[1])];
        crate::normal_alignment::validate_rectangle(self.source,domain,max_spans)?;
        let knots=if axis==0 {&self.source.knots_u}else{&self.source.knots_v};
        let degree=if axis==0 {self.source.degree_u}else{self.source.degree_v};
        if knots.iter().any(|&k|k>=domain[axis][0] && k<=domain[axis][1] && k>knots[degree] && k<knots[knots.len()-degree-1] && knots.iter().filter(|&&v|v==k).count()>degree) {
            return Ok((None,0));
        }
        if parameters[0]==parameters[1] {return Ok((Some([[0.;2];3]),0));}
        let (jet,spent)=self.derivative_bounds(domain,max_spans)?;
        let Some(jet)=jet else {return Ok((None,spent));};
        let scale=I::point(parameters[1]).sub(I::point(parameters[0]))?;
        let mut out=[[0.;2];3];
        for k in 0..3 {
            let v=I::new(jet[k][axis][0],jet[k][axis][1])?.mul(scale)?;
            out[k]=[v.lo,v.hi];
        }
        Ok((Some(out),spent))
    }
/// Higher exact quotient partial for bounded Hermite remainder estimates.
    /// Includes all incident span sides; continuity must be checked separately.
    pub fn higher_derivative_bounds(
        &self, domain:[[f64;2];2], order:[usize;2], max_spans:usize,
    ) -> Result<(Option<[[f64;2];3]>,usize)> {
        check(order.iter().all(|&n|n<=4) && order[0]+order[1]<=5 && order!=[0,0],"Bound higher profile derivative order")?;
        crate::normal_alignment::validate_rectangle(self.source,domain,max_spans)?;
        let s=self.source;let degree=[s.degree_u,s.degree_v];
        let knots=[&s.knots_u,&s.knots_v];let counts=[s.control_points.len(),s.control_points[0].len()];
        let mut out=[[f64::INFINITY,f64::NEG_INFINITY];3];let mut spent=0;
        let choose=|n:usize,k:usize| -> f64 {
            let mut c=1usize;for j in 0..k {c=c*(n-j)/(j+1);}c as f64
        };
        for u in degree[0]..counts[0] {for v in degree[1]..counts[1] {
            let span=[[knots[0][u],knots[0][u+1]],[knots[1][v],knots[1][v+1]]];
            if span.iter().any(|r|r[0]==r[1]) {continue;}
            let part=std::array::from_fn(|a|[span[a][0].max(domain[a][0]),span[a][1].min(domain[a][1])]);
            if part.iter().any(|r|r[0]>r[1]) {continue;}
            if spent==max_spans {return Ok((None,spent));}
            let net=self.homogeneous_centered_on([u,v],span,&s.control_points[u][v])?;
            let mut h=vec![vec![[I::point(0.);4];order[1]+1];order[0]+1];
            let mut f=vec![vec![[I::point(0.);3];order[1]+1];order[0]+1];
            for a in 0..=order[0] {for b in 0..=order[1] {
                h[a][b]=crate::normal_alignment::homogeneous_jet_hull(&net,degree,span,part,[a,b])?;
                for k in 0..3 {
                    let mut value=h[a][b][k];
                    for i in 0..=a {for j in 0..=b {
                        if i==0 && j==0 {continue;}
                        value=value.sub(I::point(choose(a,i)*choose(b,j)).mul(h[i][j][3])?.mul(f[a-i][b-j][k])?)?;
                    }}
                    f[a][b][k]=value.div(h[0][0][3])?;
                }
            }}
            for k in 0..3 {out[k][0]=out[k][0].min(f[order[0]][order[1]][k].lo);out[k][1]=out[k][1].max(f[order[0]][order[1]][k].hi);}
            spent+=1;
        }}
        Ok((if spent==0 {None}else{Some(out)},spent))
    }
/// Mixed derivative of the exact profile, needed by transverse transition
    /// fields. Both one-sided knot limits are included in the complete hull.
    pub fn mixed_derivative_bounds(
        &self,
        domain: [[f64; 2]; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.second_derivative_bounds(domain,[0,1],max_spans)
    }
/// Exact profile second partial, including every incident span side.
    /// Axis pairs [0,0], [0,1], [1,0], [1,1] are supported. This enclosure
    /// alone does not establish continuity across knots.
    pub fn second_derivative_bounds(
        &self, domain:[[f64;2];2], axes:[usize;2], max_spans:usize,
    ) -> Result<(Option<[[f64;2];3]>,usize)> {
        check(axes.iter().all(|&a|a<2),"Require two valid second derivative axes")?;
        crate::normal_alignment::validate_rectangle(self.source, domain, max_spans)?;
        let s = self.source;
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let knots = [&s.knots_u, &s.knots_v];
        let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
        let mut spent = 0;
        for u in degrees[0]..counts[0] {
            for v in degrees[1]..counts[1] {
                let span = [
                    [knots[0][u], knots[0][u + 1]],
                    [knots[1][v], knots[1][v + 1]],
                ];
                if span.iter().any(|r| r[0] == r[1]) {
                    continue;
                }
                let section = std::array::from_fn(|a| {
                    [span[a][0].max(domain[a][0]), span[a][1].min(domain[a][1])]
                });
                if section.iter().any(|r| r[0] > r[1]) {
                    continue;
                }
                if spent == max_spans {
                    return Ok((None, spent));
                }
                let jet = crate::normal_alignment::jet_net_on(
                    self.homogeneous_centered_on([u,v],span,&s.control_points[u][v])?,
                    degrees,
                    span,
                    section,
                    &[0.;3],
                )?;
                for k in 0..3 {
                    hull[k][0] = hull[k][0].min(jet.second[axes[0]+axes[1]][k].lo);
                    hull[k][1] = hull[k][1].max(jet.second[axes[0]+axes[1]][k].hi);
                }
                spent += 1;
            }
        }
        Ok((if spent == 0 { None } else { Some(hull) }, spent))
    }
/// Complete rectangle enclosure of the exact profile's unnormalized normal.
    /// Includes both one-sided limits at knots; zero-containing bounds do not
    /// establish regularity. A missing span never returns a partial certificate.
    pub fn normal_bounds(
        &self,
        domain: [[f64; 2]; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        crate::normal_alignment::validate_rectangle(self.source, domain, max_spans)?;
        let s = self.source;
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        let knots = [&s.knots_u, &s.knots_v];
        let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
        let mut spent = 0;
        for u in degrees[0]..counts[0] {
            for v in degrees[1]..counts[1] {
                let span = [
                    [knots[0][u], knots[0][u + 1]],
                    [knots[1][v], knots[1][v + 1]],
                ];
                if span.iter().any(|a| a[0] == a[1]) {
                    continue;
                }
                let section = std::array::from_fn(|a| {
                    [span[a][0].max(domain[a][0]), span[a][1].min(domain[a][1])]
                });
                if section.iter().any(|a| a[0] > a[1]) {
                    continue;
                }
                if spent == max_spans {
                    return Ok((None, spent));
                }
                let net = self.homogeneous_centered_on([u,v],span,&s.control_points[u][v])?;
                let j = crate::normal_alignment::jacobian_net_on(
                    net,
                    degrees,
                    span,
                    section,
                    &[0.;3],
                )?;
                for k in 0..3 {
                    let a = (k + 1) % 3;
                    let b = (k + 2) % 3;
                    let n = j[a][0].mul(j[b][1])?.sub(j[b][0].mul(j[a][1])?)?;
                    hull[k][0] = hull[k][0].min(n.lo);
                    hull[k][1] = hull[k][1].max(n.hi);
                }
                spent += 1;
            }
        }
        Ok((if spent == 0 { None } else { Some(hull) }, spent))
    }
/// Direction enclosure on the exact profile. A pole factor may tighten
    /// uncertain normals only on rectangles strictly inside the U domain;
    /// no regular normal or unique tangent is supplied at collapsed endpoints.
    pub fn interior_normal_direction_bounds(&self,domain:[[f64;2];2],max_spans:usize)
        ->Result<(Option<[[f64;2];3]>,usize)> {
        check(max_spans<=100000,"Bound interior profile normal-direction work")?;
        crate::normal_alignment::validate_rectangle(self.source,domain,max_spans.max(1))?;
        if max_spans==0 {return Ok((None,0));}
        let (normal,mut spent)=self.normal_bounds(domain,max_spans)?;
        let Some(bounds)=normal else {return Ok((None,spent));};
        let s=self.source;
        if !bounds.iter().all(|d|d[0]<=0. && d[1]>=0.)
            || domain[0][0]<=s.knots_u[s.degree_u]
            || domain[0][1]>=s.knots_u[s.control_points.len()] {
            return Ok((normal,spent));
        }
        for high in [false,true] {
            if max_spans-spent<6 {break;}
            let (factored,used)=self.factored_pole_normal_bounds(domain,high,max_spans-spent)?;
            spent+=used;
            if let Some(factored)=factored {
                if factored.components.iter().any(|d|d[0]>0. || d[1]<0.) {
                    return Ok((Some(factored.components),spent));
                }
            }
        }
        Ok((normal,spent))
    }
/// Normal divided by exact longitudinal and transverse collapsed-U factors.
    /// This is a factored interior/pole-limit diagnostic, not immersion at the pole.
    pub fn factored_pole_normal_bounds(&self,domain:[[f64;2];2],high:bool,max_spans:usize)->Result<(Option<ProfilePoleNormal>,usize)> {
        crate::normal_alignment::validate_rectangle(self.source,domain,max_spans)?;
        let s=self.source;let n=s.degree_u;let q=s.degree_v;
        let span=[[s.knots_u[n],s.knots_u[s.control_points.len()]],
            [s.knots_v[q],s.knots_v[s.control_points[0].len()]]];
        if n>8 || q>8 || s.control_points.len()!=n+1 || s.control_points[0].len()!=q+1
            || ![(&s.knots_u,n),(&s.knots_v,q)].iter().enumerate().all(|(axis,(knots,degree))|
                knots[..=*degree].iter().all(|x|*x==span[axis][0]) && knots[degree+1..].iter().all(|x|*x==span[axis][1])) {return Ok((None,0));}
        let pole=&s.control_points[if high {n}else{0}][0];
        let power=(0..=n).take_while(|step|s.control_points[if high {n-step}else{*step}].iter().all(|p|p==pole)).count();
        if power==0 || power>n || max_spans<6 {return Ok((None,0));}
        // Separable profile weights make each constant control row independent
        // of V. More such rows can vanish transversely than coincide at the pole.
        let transverse=(0..=n).take_while(|step|{
            let row=&s.control_points[if high {n-step}else{*step}];row.iter().all(|p|p==&row[0])
        }).count().min(n);
        let queries=if transverse>power {7}else{6};
        if max_spans<queries {return Ok((None,0));}
        let choose=|n:usize,k:usize|->u64 {(0..k).fold(1,|value,i|value*(n-i) as u64/(i+1) as u64)};
        let m=n-power;let mut reduced=vec![vec![[I::point(0.);4];q+1];m+1];
        for i in 0..=m {for j in 0..=q {
            let original=if high {i}else{i+power};
            let ratio=I::point(choose(n,original) as f64).div(I::point(choose(m,i) as f64))?;
            let weight=I::point(self.row[original]).mul(I::point(self.column[j]))?.div(I::point(self.pivot))?;
            for k in 0..3 {
                let difference=if s.control_points[original][j][k]==pole[k] {I::point(0.)}else{I::point(s.control_points[original][j][k]).sub(I::point(pole[k]))?};
                reduced[i][j][k]=difference.mul(weight)?.mul(ratio)?;
            }
        }}
        let full=self.homogeneous_on([n,q],span)?;
        let hull=crate::normal_alignment::homogeneous_jet_hull;
        let a=hull(&reduced,[m,q],span,domain,[0,0])?;
        let au=hull(&reduced,[m,q],span,domain,[1,0])?;
        let (transverse_position,transverse_derivative)=if transverse==power {
            (a,hull(&reduced,[m,q],span,domain,[0,1])?)
        } else {
            // Subtract each row's own constant V baseline. Its rational blend
            // depends on U only, so its V derivative vanishes identically.
            let degree=n-transverse;let mut net=vec![vec![[I::point(0.);4];q+1];degree+1];
            for i in 0..=degree {for j in 0..=q {
                let original=if high {i}else{i+transverse};
                let ratio=I::point(choose(n,original) as f64).div(I::point(choose(degree,i) as f64))?;
                let weight=I::point(self.row[original]).mul(I::point(self.column[j]))?.div(I::point(self.pivot))?;
                for k in 0..3 {
                    let base=s.control_points[original][0][k];let value=s.control_points[original][j][k];
                    let difference=if value==base {I::point(0.)}else{I::point(value).sub(I::point(base))?};
                    net[i][j][k]=difference.mul(weight)?.mul(ratio)?;
                }
            }}
            (hull(&net,[degree,q],span,domain,[0,0])?,hull(&net,[degree,q],span,domain,[0,1])?)
        };
        let w=hull(&full,[n,q],span,domain,[0,0])?[3];
        let wu=hull(&full,[n,q],span,domain,[1,0])?[3];
        let wv=hull(&full,[n,q],span,domain,[0,1])?[3];
        let width=I::point(span[0][1]).sub(I::point(span[0][0]))?;
        let u=I::new(domain[0][0],domain[0][1])?;
        let factor=if high {I::point(span[0][1]).sub(u)?}else{u.sub(I::point(span[0][0]))?}.div(width)?.intersect(0.,1.)?;
        let derivative=I::point(if high {-(power as f64)}else{power as f64}).div(width)?;
        let mut first=[I::point(0.);3];let mut second=first;
        for k in 0..3 {
            first[k]=derivative.mul(a[k])?.mul(w)?.add(factor.mul(au[k].mul(w)?.sub(a[k].mul(wu)?)?)?)?;
            second[k]=transverse_derivative[k].mul(w)?.sub(transverse_position[k].mul(wv)?)?;
        }
        let denominator=w.mul(w)?.mul(w)?.mul(w)?;
        let mut components=[[0.;2];3];
        for k in 0..3 {let i=(k+1)%3;let j=(k+2)%3;
            let normal=first[i].mul(second[j])?.sub(first[j].mul(second[i])?)?.div(denominator)?;
            components[k]=[normal.lo,normal.hi];
        }
        Ok((Some(ProfilePoleNormal{components,high,power:power+transverse-1,point:[pole[0],pole[1],pole[2]]}),queries))
    }
/// Cover the exact normal after collapsed-U factor removal. Completion
    /// proves the interior and nonzero scaled limits, never a smooth pole.
    pub fn inspect_factored_pole_regularity(&self,domain:[[f64;2];2],high:bool,max_cells:usize,max_spans:usize)->Result<ProfilePoleCover> {
        crate::normal_alignment::validate_rectangle(self.source,domain,max_spans)?;
        check(max_cells<=100000,"Bound factored pole cell work")?;
        let mut out=ProfilePoleCover{regular:Vec::new(),pending:Vec::new(),cells:0,spans:0,complete_factored:false,high,power:None,point:None,reason:"profile-pole-factor-work-limit"};
        let mut queue=std::collections::VecDeque::from([domain]);
        while let Some(rect)=queue.pop_front() {
            if out.cells==max_cells || max_spans-out.spans<6 {out.pending.push(rect);out.pending.extend(queue);return Ok(out);}
            out.cells+=1;
            let (normal,spent)=self.factored_pole_normal_bounds(rect,high,max_spans-out.spans)?;out.spans+=spent;
            let Some(normal)=normal else {out.pending.push(rect);out.pending.extend(queue);out.reason=if max_spans-out.spans<7 {"profile-pole-factor-work-limit"}else{"profile-pole-factor-recipe-unproven"};return Ok(out);};
            out.power=Some(normal.power);out.point=Some(normal.point);
            if normal.components.iter().any(|r|r[0]>0. || r[1]<0.) {out.regular.push(rect);continue;}
            let width=std::array::from_fn::<_,2,_>(|axis|{
                let d=domain[axis][1]-domain[axis][0];if d==0. {0.}else{(rect[axis][1]-rect[axis][0])/d}
            });
            let axis=usize::from(width[1]>width[0]);let mid=rect[axis][0]*0.5+rect[axis][1]*0.5;
            if mid<=rect[axis][0] || mid>=rect[axis][1] {out.pending.push(rect);continue;}
            let mut left=rect;left[axis][1]=mid;let mut right=rect;right[axis][0]=mid;
            queue.push_front(right);queue.push_front(left);
        }
        out.complete_factored=out.pending.is_empty();
        out.reason=if out.complete_factored {"profile-pole-factor-cover-qualified"}else{"profile-pole-factor-resolution-limit"};Ok(out)
    }
/// Adaptive complete coverage of exact profile normals on a rectangle.
    /// A nonzero component proves immersion; zero-containing cells stay pending.
    pub fn inspect_regularity(&self,domain:[[f64;2];2],max_cells:usize,max_spans:usize)->Result<TransitionRegularityReport> {
        crate::normal_alignment::validate_rectangle(self.source,domain,max_spans)?;
        check(max_cells<=100000,"Bound profile regularity cell work")?;
        let mut out=TransitionRegularityReport{regular:Vec::new(),pending:Vec::new(),cells:0,spans:0,complete_regular:false,reason:"profile-regularity-work-limit"};
        let mut queue=std::collections::VecDeque::from([domain]);
        while let Some(rect)=queue.pop_front() {
            if out.cells==max_cells || out.spans==max_spans {out.pending.push(rect);out.pending.extend(queue);return Ok(out);}
            out.cells+=1;
            let (normal,spent)=self.normal_bounds(rect,max_spans-out.spans)?;out.spans+=spent;
            let Some(normal)=normal else {out.pending.push(rect);out.pending.extend(queue);return Ok(out);};
            if normal.iter().any(|r|r[0]>0. || r[1]<0.) {out.regular.push(rect);continue;}
            let normalized=std::array::from_fn::<_,2,_>(|axis|{
                let width=domain[axis][1]-domain[axis][0];
                if width==0. {0.}else{(rect[axis][1]-rect[axis][0])/width}
            });
            let axis=usize::from(normalized[1]>normalized[0]);
            let mid=rect[axis][0]*0.5+rect[axis][1]*0.5;
            if mid<=rect[axis][0] || mid>=rect[axis][1] {out.pending.push(rect);continue;}
            let mut left=rect;left[axis][1]=mid;let mut right=rect;right[axis][0]=mid;
            queue.push_front(right);queue.push_front(left);
        }
        out.complete_regular=out.pending.is_empty();
        out.reason=if out.complete_regular {"profile-regularity-qualified"}else{"profile-regularity-resolution-limit"};Ok(out)
    }
/// Compare complete proposed-profile and original-surface normal bounds.
    /// This proves only tangent-plane angle over the supplied rectangles;
    /// shared position, seam ownership and orientation require separate gates.
    pub fn inspect_original_normals(
        &self,
        original: &Surface,
        domains: [[[f64; 2]; 2]; 2],
        max_sine_squared: f64,
        max_spans: usize,
    ) -> Result<crate::normal_alignment::PairReport> {
        self.inspect_normal_pair(original, None, domains, max_sine_squared, max_spans)
    }
/// Compare exact factored profile normal limits with an unchanged original
    /// surface. Diagnostic angle only; a collapsed pole stays nonregular.
    pub fn inspect_original_factored_pole_normals(&self,original:&Surface,
        domains:[[[f64;2];2];2],high:bool,threshold:f64,max_spans:usize
    )->Result<crate::normal_alignment::PairReport> {
        check(threshold.is_finite() && (0. ..1.).contains(&threshold) && max_spans<=100000,
            "Bound factored normal comparison")?;
        crate::normal_alignment::validate_rectangle(original,domains[1],max_spans)?;
        let (a,used)=self.factored_pole_normal_bounds(domains[0],high,max_spans)?;
        let mut out=crate::normal_alignment::PairReport{aligned:None,sine_squared_interval:None,
            normal_components:[a.map(|normal|normal.components),None],spans:used,reason:"span-limit"};
        if out.normal_components[0].is_none() || used==max_spans {return Ok(out);}
        let (b,spent)=crate::normal_alignment::normal_bounds(original,domains[1],max_spans-used)?;
        out.spans+=spent;out.normal_components[1]=b;
        if let [Some(a),Some(b)]=out.normal_components {
            let mut compared=crate::normal_alignment::inspect_pair_bounds([a,b],threshold)?;
            compared.spans=out.spans;Ok(compared)
        } else {Ok(out)}
    }
/// Complete rectangle angle bounds for two exact proposed profiles.
    pub fn inspect_profile_normals(
        &self,
        other: &WeightProfile<'_>,
        domains: [[[f64; 2]; 2]; 2],
        threshold: f64,
        max_spans: usize,
    ) -> Result<crate::normal_alignment::PairReport> {
        self.inspect_normal_pair(other.source, Some(other), domains, threshold, max_spans)
    }
pub(super) fn inspect_normal_pair(
        &self,
        original: &Surface,
        profile: Option<&WeightProfile<'_>>,
        domains: [[[f64; 2]; 2]; 2],
        max_sine_squared: f64,
        max_spans: usize,
    ) -> Result<crate::normal_alignment::PairReport> {
        check(
            max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared),
            "Finite squared-sine threshold below one",
        )?;
        crate::normal_alignment::validate_rectangle(original, domains[1], max_spans)?;
        let (a, used) = self.normal_bounds(domains[0], max_spans)?;
        let mut out = crate::normal_alignment::PairReport {
            aligned: None,
            sine_squared_interval: None,
            normal_components: [a, None],
            spans: used,
            reason: "span-limit",
        };
        if a.is_none() || used == max_spans {
            return Ok(out);
        }
        let (b, spent) = match profile {
            Some(p) => p.normal_bounds(domains[1], max_spans - used)?,
            None => crate::normal_alignment::normal_bounds(original, domains[1], max_spans - used)?,
        };
        out.spans += spent;
        out.normal_components[1] = b;
        if let (Some(a), Some(b)) = (a, b) {
            let mut compared =
                crate::normal_alignment::inspect_pair_bounds([a, b], max_sine_squared)?;
            compared.spans = out.spans;
            Ok(compared)
        } else {
            Ok(out)
        }
    }
/// Adaptively cover the entire [0,1] correspondence of two affine UV lines.
    /// Endpoint coordinates are exact binary64 inputs; interval arithmetic
    /// encloses every mapped point. Both endpoints stay in interval coverage.
    pub fn inspect_original_path(
        &self,
        original: &Surface,
        endpoints: [[[f64; 2]; 2]; 2],
        threshold: f64,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<ProfilePathReport> {
        self.inspect_path(original, None, endpoints, threshold, max_cells, max_spans, None)
    }
/// Full affine-path comparison of factored profile limits against an
    /// original surface. Includes pole limits, but never certifies immersion
    /// at a collapsed point or positional agreement between the surfaces.
    pub fn inspect_original_factored_pole_path(&self,original:&Surface,
        endpoints:[[[f64;2];2];2],high:bool,threshold:f64,max_cells:usize,max_spans:usize
    )->Result<ProfilePathReport> {
        self.inspect_path(original,None,endpoints,threshold,max_cells,max_spans,Some(high))
    }
}
