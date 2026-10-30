//! profile expression radius operations: self_chord_continuous.
use super::*;
impl RailConstrainedBridge<'_, '_, '_, '_, '_> {
pub(super) fn self_chord_continuous(&self) -> bool {
        for high in [false,true] {
            let source=self.section(high).profile().source();
            for (knots,degree,count) in [(&source.knots_u,source.degree_u,source.control_points.len()),
                (&source.knots_v,source.degree_v,source.control_points[0].len())] {
                if degree==0 || knots.iter().any(|&k|k>knots[degree] && k<knots[count]
                    && knots.iter().filter(|&&x|x==k).count()>=degree) {return false;}
            }
        }
        true
    }
/// Chord from this bridge's own profile to its corrected support, through
    /// an exact owned section. No foreign profile or serialized proof is accepted.
    pub fn inspect_profile_endpoint_chord(
        &self,profile_domain:[[f64;2];2],bridge_domain:[[f64;2];2],
        high:bool,sine:f64,max_spans:usize,
    )->Result<(Option<bool>,usize)> {
        self.inspect_profile_endpoint_chord_cached(profile_domain,bridge_domain,high,sine,max_spans,&mut ProfileDerivativeCache::default())
    }
pub(super) fn inspect_profile_endpoint_chord_cached(
        &self,profile_domain:[[f64;2];2],bridge_domain:[[f64;2];2],
        high:bool,sine:f64,max_spans:usize,cache:&mut ProfileDerivativeCache,
    )->Result<(Option<bool>,usize)> {
        check(max_spans<=100000 && sine.is_finite() && (0. ..1.).contains(&sine)
            && [profile_domain,bridge_domain].iter().flatten().all(|d|
                d.iter().all(|x|x.is_finite()) && d[0]<d[1]),"Bound owned cross-chart chord query")?;
        let section=self.section(high);let profile=section.profile();let axis=section.axis();let free=1-axis;
        crate::normal_alignment::validate_rectangle(profile.source(),profile_domain,max_spans.max(1))?;
        self.inspect_self_chord_pair([bridge_domain;2],sine,0)?;
        let cut=section.parameter();let end=if high {1.}else{0.};
        let mut first=profile_domain;
        first[axis]=[profile_domain[axis][0].min(cut),profile_domain[axis][1].max(cut)];
        let mut second=first;second[axis]=[cut;2];
        second[free]=[profile_domain[free][0].min(bridge_domain[0][0]),profile_domain[free][1].max(bridge_domain[0][1])];
        let union=std::array::from_fn(|k|[first[k][0].min(second[k][0]),first[k][1].max(second[k][1])]);
        profile.inspect_self_chords(union,sine,0)?;
        if max_spans==0 || !self.self_chord_continuous() {return Ok((None,0));}
        let mut used=0;let mut columns=[[[0.;2];3];3];
        for (index,domain) in [first,second].into_iter().enumerate() {
            if used==max_spans {return Ok((None,used));}
            let (jet,spent)=cache.enclose(profile,domain,max_spans-used)?;used+=spent;
            let Some(jet)=jet else {return Ok((None,used));};
            for k in 0..3 {columns[index][k]=jet[k][if index==0 {axis}else{free}];}
        }
        if used==max_spans {return Ok((None,used));}
        let across=[bridge_domain[1][0].min(end),bridge_domain[1][1].max(end)];
        let (jet,spent)=cache.corrected_jet(self,[bridge_domain[0],across],max_spans-used)?;used+=spent;
        let Some(jet)=jet else {return Ok((None,used));};columns[2]=jet[1];
        let delta=[I::point(cut).sub(I::new(profile_domain[axis][0],profile_domain[axis][1])?)?,
            I::new(bridge_domain[0][0],bridge_domain[0][1])?.sub(I::new(profile_domain[free][0],profile_domain[free][1])?)?,
            I::new(bridge_domain[1][0],bridge_domain[1][1])?.sub(I::point(end))?];
        // Exact endpoint ordering establishes the signs before interval
        // subtraction rounds an exact zero slightly across the origin.
        let positive=profile_domain[axis][1]<=cut && bridge_domain[1][0]>=end;
        let negative=profile_domain[axis][0]>=cut && bridge_domain[1][1]<=end;
        let mut transverse=[[delta[0].lo,delta[0].hi],[delta[2].lo,delta[2].hi]];
        if positive {for d in &mut transverse {d[0]=d[0].max(0.);}}
        if negative {for d in &mut transverse {d[1]=d[1].min(0.);}}
        let source_scale=(self.section(true).parameter()-self.section(false).parameter()).abs();
        let mut chord=[[0.;2];3];
        for k in 0..3 {
            let mut value=I::point(0.);
            for i in 0..3 {value=value.add(I::new(columns[i][k][0],columns[i][k][1])?.mul(delta[i])?)?;}
            chord[k]=[value.lo,value.hi];
        }
        let mut normals=[[[0.;2];3];2];
        for corrected in [false,true] {
            if used==max_spans {return Ok((None,used));}
            let (normal,spent)=if corrected {
                cache.corrected_normal(self,bridge_domain,max_spans-used)?
            }else{cache.profile_direction(profile,profile_domain,max_spans-used)?};used+=spent;
            let Some(normal)=normal else {return Ok((None,used));};
            normals[usize::from(corrected)]=normal;

            if crate::normal_alignment::inspect_pair_bounds([normal,chord],sine)?.aligned==Some(false) {
                return Ok((Some(true),used));
            }
            if (positive || negative) && source_scale.is_finite() && source_scale>0.
                && crate::surface_self_chord::inspect_same_sign_transverse_chords(
                    columns,transverse,source_scale,normal,sine)? {
                return Ok((Some(true),used));
            }
        }
        // The other exact source path changes free first, then transverse.
        // Both paths reach the same section point; intersect only complete
        // independent enclosures, sharing the already paid corrected leg.
        let mut alternate=columns;let mut alternate_complete=true;
        let mut free_first=second;free_first[axis]=profile_domain[axis];
        let mut transverse_last=first;transverse_last[free]=bridge_domain[0];
        for (column,domain) in [(1,free_first),(0,transverse_last)] {
            if used==max_spans {alternate_complete=false;break;}
            let (jet,spent)=cache.enclose(profile,domain,max_spans-used)?;used+=spent;
            let Some(jet)=jet else {alternate_complete=false;break;};
            for k in 0..3 {alternate[column][k]=jet[k][if column==0 {axis}else{free}];}
        }
        if alternate_complete {
            for k in 0..3 {
                let mut value=I::point(0.);
                for i in 0..3 {value=value.add(I::new(alternate[i][k][0],alternate[i][k][1])?.mul(delta[i])?)?;}
                let overlap=I::new(chord[k][0].max(value.lo),chord[k][1].min(value.hi))?;
                chord[k]=[overlap.lo,overlap.hi];
            }
            // The alternate complete source path reaches the same owned
            // section point with the same signed parameter increments.
            // Its narrower column hulls may prove the transverse cone even
            // when the first path's hulls cannot. Never use partial paths.
            if (positive || negative) && source_scale.is_finite() && source_scale>0. {
                for normal in normals {
                    if crate::surface_self_chord::inspect_same_sign_transverse_chords(
                        alternate,transverse,source_scale,normal,sine)? {
                        return Ok((Some(true),used));
                    }
                }
            }
        }
        for normal in normals {
            if crate::normal_alignment::inspect_pair_bounds([normal,chord],sine)?.aligned==Some(false) {
                return Ok((Some(true),used));
            }
        }
        // Endpoint subtraction independently encloses the same exact chord.
        // Refine only unresolved paths, and never use an incomplete position hull.
        if used==max_spans {return Ok((None,used));}
        let (a,spent)=cache.position(profile,profile_domain,max_spans-used)?;used+=spent;
        let Some(a)=a else {return Ok((None,used));};
        if used==max_spans {return Ok((None,used));}
        let (b,spent)=cache.corrected_position(self,bridge_domain,max_spans-used)?;used+=spent;
        let Some(b)=b else {return Ok((None,used));};
        for k in 0..3 {
            let difference=I::new(b[k][0],b[k][1])?.sub(I::new(a[k][0],a[k][1])?)?;
            let overlap=I::new(chord[k][0].max(difference.lo),chord[k][1].min(difference.hi))?;
            chord[k]=[overlap.lo,overlap.hi];
        }
        for normal in normals {
            if crate::normal_alignment::inspect_pair_bounds([normal,chord],sine)?.aligned==Some(false) {
                return Ok((Some(true),used));
            }
        }
        Ok((Some(false),used))
    }
/// Fresh whole-path derivatives and local endpoint normals of the exact
    /// corrected expression, with continuous source derivative fields.
    pub fn inspect_self_chord_pair(
        &self,pair:[[[f64;2];2];2],max_sine_squared:f64,max_spans:usize,
    ) -> Result<(Option<bool>,usize)> {
        self.inspect_self_chord_pair_cached(pair,max_sine_squared,max_spans,&mut ProfileDerivativeCache::default())
    }
pub(super) fn inspect_self_chord_pair_cached(
        &self,pair:[[[f64;2];2];2],max_sine_squared:f64,max_spans:usize,cache:&mut ProfileDerivativeCache,
    ) -> Result<(Option<bool>,usize)> {
        check(max_spans<=100000 && pair.iter().all(|uv|uv.iter().all(|d|
            d.iter().all(|x|x.is_finite()) && d[0]<d[1])),"Bound corrected chord endpoints and work")?;
        let union=std::array::from_fn(|k|[pair[0][k][0].min(pair[1][k][0]),pair[0][k][1].max(pair[1][k][1])]);
        self.inspect_self_chords(union,max_sine_squared,0)?;
        if max_spans==0 || !self.self_chord_continuous() {return Ok((None,0));}
        let (jet,mut spent)=cache.corrected_jet(self,union,max_spans)?;
        let Some(jet)=jet else {return Ok((None,spent));};
        let mut jacobian=std::array::from_fn(|k|std::array::from_fn(|axis|jet[axis][k]));
        if !crate::surface_self_chord::inspect_jacobian_rank_bound(jacobian)? {
            // Same two-leg path as exact profiles; every column comes from
            // the corrected expression on its entire connecting strip.
            let strips=[[union[0],pair[0][1]],[pair[1][0],union[1]]];
            let mut columns=jacobian;
            for axis in 0..2 {
                if spent==max_spans {return Ok((None,spent));}
                let (bounds,used)=cache.corrected_jet(self,strips[axis],max_spans-spent)?;
                spent+=used;
                let Some(bounds)=bounds else {return Ok((None,spent));};
                for k in 0..3 {columns[k][axis]=bounds[axis][k];}
            }
            jacobian=columns;
        }
        // Direct M*(B-A) interval exclusion proves its own nonzero chord;
        // only the mean-matrix theorem requires uniform rank admission.
        let ranked=crate::surface_self_chord::inspect_jacobian_rank_bound(jacobian)?;
        let mut normals=[[[0.;2];3];2];
        for (index,endpoint) in pair.into_iter().enumerate() {
            if spent==max_spans {return Ok((None,spent));}
            let (normal,used)=cache.corrected_normal(self,endpoint,max_spans-spent)?;
            spent+=used;
            let Some(normal)=normal else {return Ok((None,spent));};
            normals[index]=normal;
            if crate::surface_self_chord::inspect_jacobian_endpoint_pair_chords(jacobian,pair,normal,max_sine_squared)?
                || (ranked && crate::surface_self_chord::inspect_jacobian_endpoint_chords(jacobian,normal,max_sine_squared)?.0) {
                return Ok((Some(true),spent));
            }
        }
        // Even a full-rank union can be wider than its coordinate strips.
        // Unresolved pairs therefore use the first path regardless of rank.
        let strips=[[union[0],pair[0][1]],[pair[1][0],union[1]]];
        for axis in 0..2 {
            if spent==max_spans {return Ok((None,spent));}
            let (jet,used)=cache.corrected_jet(self,strips[axis],max_spans-spent)?;spent+=used;
            let Some(jet)=jet else {return Ok((None,spent));};
            for k in 0..3 {jacobian[k][axis]=jet[axis][k];}
        }
        // A complete first path already encloses the exact endpoint chord.
        // Admit its exclusion before paying for the alternate path; uncertainty
        // still falls through to the intersection of both complete bounds.
        for normal in normals {
            if crate::surface_self_chord::inspect_jacobian_endpoint_pair_chords(
                jacobian,pair,normal,max_sine_squared)? {
                return Ok((Some(true),spent));
            }
        }
        // Reverse the coordinate path: across first at A.free, then free
        // at B.across. Both complete paths enclose the same exact chord.
        let strips=[[union[0],pair[1][1]],[pair[0][0],union[1]]];
        let mut alternate=jacobian;
        for axis in 0..2 {
            if spent==max_spans {return Ok((None,spent));}
            let (jet,used)=cache.corrected_jet(self,strips[axis],max_spans-spent)?;spent+=used;
            let Some(jet)=jet else {return Ok((None,spent));};
            for k in 0..3 {alternate[k][axis]=jet[axis][k];}
        }
        let mut delta=[I::point(0.);2];
        for axis in 0..2 {
            delta[axis]=I::new(pair[1][axis][0],pair[1][axis][1])?
                .sub(I::new(pair[0][axis][0],pair[0][axis][1])?)?;
        }
        let mut chord=[[0.;2];3];
        for k in 0..3 {
            let evaluate=|j:[[f64;2];2]| -> Result<I> {
                I::new(j[0][0],j[0][1])?.mul(delta[0])?
                    .add(I::new(j[1][0],j[1][1])?.mul(delta[1])?)
            };
            let first=evaluate(jacobian[k])?;let second=evaluate(alternate[k])?;
            let overlap=first.intersect(second.lo,second.hi)?;
            chord[k]=[overlap.lo,overlap.hi];
        }
        for normal in normals {
            if crate::normal_alignment::inspect_pair_bounds([normal,chord],max_sine_squared)?.aligned==Some(false) {
                return Ok((Some(true),spent));
            }
        }
        // Independently enclose both positions on the same corrected owner.
        // Only complete endpoint hulls may tighten the already bounded chord.
        let mut positions=[[[0.;2];3];2];
        for (index,endpoint) in pair.into_iter().enumerate() {
            if spent==max_spans {return Ok((None,spent));}
            let (position,used)=cache.corrected_position(self,endpoint,max_spans-spent)?;spent+=used;
            let Some(position)=position else {return Ok((None,spent));};
            positions[index]=position;
        }
        for k in 0..3 {
            let difference=I::new(positions[1][k][0],positions[1][k][1])?
                .sub(I::new(positions[0][k][0],positions[0][k][1])?)?;
            let overlap=I::new(chord[k][0],chord[k][1])?.intersect(difference.lo,difference.hi)?;
            chord[k]=[overlap.lo,overlap.hi];
        }
        for normal in normals {
            if crate::normal_alignment::inspect_pair_bounds([normal,chord],max_sine_squared)?.aligned==Some(false) {
                return Ok((Some(true),spent));
            }
        }
        Ok((Some(false),spent))
    }
/// Diagnostic exclusion on the exact transverse-corrected expression.
    /// Its source derivative fields must be continuous before a mean bound
    /// can connect arbitrary endpoints on the rectangle.
    pub fn inspect_self_chords(
        &self, domain: [[f64; 2]; 2], max_sine_squared: f64, max_spans: usize,
    ) -> Result<(Option<bool>, usize)> {
        self.inspect_self_chords_cached(domain,max_sine_squared,max_spans,&mut ProfileDerivativeCache::default())
    }
pub(super) fn inspect_self_chords_cached(
        &self, domain: [[f64; 2]; 2], max_sine_squared: f64, max_spans: usize,
        cache:&mut ProfileDerivativeCache,
    ) -> Result<(Option<bool>, usize)> {
        let range = self.section(false).range();
        check(max_spans <= 100000 && max_sine_squared.is_finite()
            && (0. ..1.).contains(&max_sine_squared)
            && domain.iter().all(|d|d.iter().all(|x|x.is_finite()) && d[0]<d[1])
            && domain[0][0]>=range[0] && domain[0][1]<=range[1]
            && domain[1][0]>=0. && domain[1][1]<=1.,
            "Bound corrected self-chord domain, angle and work")?;
        if !self.self_chord_continuous() {return Ok((None,0));}
        if max_spans==0 {return Ok((None,0));}
        let (jet,spent)=cache.corrected_jet(self,domain,max_spans)?;
        let Some(jet)=jet else {return Ok((None,spent));};
        let jacobian=std::array::from_fn(|k|std::array::from_fn(|axis|jet[axis][k]));
        Ok((Some(crate::surface_self_chord::inspect_jacobian_self_chords(jacobian,max_sine_squared)?.0),spent))
    }
pub(super) fn mean_radius_residual(
        &self,
        centers: &Curve,
        radius: &Curve,
        domains: [[f64; 2]; 2],
        rect: [[f64; 2]; 2],
        position: [[f64; 2]; 3],
        c: &[I],
        r: I,
        budget: usize,
    ) -> Result<(Option<([f64; 2], [f64; 2])>, usize)> {
        for (curve, domain) in [centers, radius].into_iter().zip(domains) {
            if !continuous_knots(
                &curve.knots,
                curve.degree,
                curve.control_points.len(),
                domain,
            ) {
                return Ok((None, 0));
            }
        }
        // The exact profile rails participate in the corrected expression.
        // A discontinuous source chart cannot use a whole-cell mean bound.
        for high in [false, true] {
            let section = self.section(high);
            let source = section.profile().source();
            let across = [
                self.section(false).parameter(),
                self.section(true).parameter(),
            ];
            let mut domain = [[across[0].min(across[1]), across[0].max(across[1])]; 2];
            domain[1 - section.axis()] = rect[0];
            for (axis, (knots, degree, count)) in [
                (
                    &source.knots_u,
                    source.degree_u,
                    source.control_points.len(),
                ),
                (
                    &source.knots_v,
                    source.degree_v,
                    source.control_points[0].len(),
                ),
            ]
            .into_iter()
            .enumerate()
            {
                if !continuous_knots(knots, degree, count, domain[axis]) {
                    return Ok((None, 0));
                }
            }
        }
        let midpoint = rect.map(|d| d[0] * 0.5 + d[1] * 0.5);
        let (p, mut spent) = self.enclose_with_transverse_correction(
            [midpoint[0]; 2],
            [midpoint[1]; 2],
            budget.min(100000),
        )?;
        let Some(p) = p else {
            return Ok((None, spent));
        };
        let ct = mapped(domains[0], [midpoint[1]; 2])?;
        let rt = mapped(domains[1], [midpoint[1]; 2])?;
        let used = spans(centers, ct) + spans(radius, rt);
        if used > budget - spent {
            return Ok((None, spent));
        }
        spent += used;
        let center = interval_eval::evaluate_interval(centers, ct)?;
        let law = interval_eval::evaluate_interval(radius, rt)?;
        let point = crate::moving_radius::bound_expression(
            p,
            std::array::from_fn(|k| [center[k].lo, center[k].hi]),
            [law[0].lo.max(0.), law[0].hi],
        )?;
        let (jet, used) = self.derivatives_with_transverse_correction(
            rect[0],
            rect[1],
            (budget - spent).min(100000),
        )?;
        spent += used;
        let Some(jet) = jet else {
            return Ok((None, spent));
        };
        let (cd, used) = law_derivative(
            centers,
            mapped(domains[0], rect[1])?,
            domains[0],
            budget - spent,
        )?;
        spent += used;
        let Some(cd) = cd else {
            return Ok((None, spent));
        };
        let (rd, used) = law_derivative(
            radius,
            mapped(domains[1], rect[1])?,
            domains[1],
            budget - spent,
        )?;
        spent += used;
        let Some(rd) = rd else {
            return Ok((None, spent));
        };
        let mut gradient = [I::point(0.); 2];
        for k in 0..3 {
            let delta = I::new(position[k][0], position[k][1])?.sub(c[k])?;
            gradient[0] = gradient[0].add(
                I::point(2.)
                    .mul(delta)?
                    .mul(I::new(jet[0][k][0], jet[0][k][1])?)?,
            )?;
            gradient[1] = gradient[1].add(
                I::point(2.)
                    .mul(delta)?
                    .mul(I::new(jet[1][k][0], jet[1][k][1])?.sub(cd[k])?)?,
            )?;
        }
        gradient[1] = gradient[1].sub(I::point(2.).mul(r)?.mul(rd[0])?)?;
        let mut residual = I::new(point.squared_residual[0], point.squared_residual[1])?;
        let mut contributions = [0.; 2];
        for axis in 0..2 {
            let contribution = gradient[axis]
                .mul(I::new(rect[axis][0], rect[axis][1])?.sub(I::point(midpoint[axis]))?)?;
            contributions[axis] = contribution.lo.abs().max(contribution.hi.abs());
            residual = residual.add(contribution)?;
        }
        Ok((Some(([residual.lo, residual.hi], contributions)), spent))
    }
/// Center and radius curves are mapped over their complete domains by
    /// normalized across coordinate. Radius uses coordinate zero. All free
    /// coordinates participate; no sample or unchanged source admits a cell.
    pub fn inspect_radius(
        &self,
        centers: &Curve,
        radius: &Curve,
        tolerance: f64,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<Report> {
        centers.validate()?;
        radius.validate()?;
        self.inspect_radius_on_domains(
            centers,
            radius,
            [centers.domain(), radius.domain()],
            tolerance,
            max_cells,
            max_spans,
        )
    }
}
