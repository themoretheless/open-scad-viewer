//! profile expression radius operations: inspect_self_chord_pair.
use super::*;
impl crate::surface_weight_profile::WeightProfile<'_> {
/// Endpoint-specific bound on one continuous exact profile chart.
    pub fn inspect_self_chord_pair(
        &self,pair:[[[f64;2];2];2],max_sine_squared:f64,max_spans:usize,
    ) -> Result<(Option<bool>,usize)> {
        self.inspect_self_chord_pair_cached(pair,max_sine_squared,max_spans,&mut ProfileDerivativeCache::default())
    }
pub(super) fn inspect_self_chord_pair_cached(
        &self,pair:[[[f64;2];2];2],max_sine_squared:f64,max_spans:usize,
        cache:&mut ProfileDerivativeCache,
    ) -> Result<(Option<bool>,usize)> {
        check(max_spans<=100000 && pair.iter().all(|uv|uv.iter().all(|d|
            d.iter().all(|x|x.is_finite()) && d[0]<d[1])),"Bound profile chord endpoints and work")?;
        let union=std::array::from_fn(|k|[pair[0][k][0].min(pair[1][k][0]),pair[0][k][1].max(pair[1][k][1])]);
        self.inspect_self_chords(union,max_sine_squared,0)?;
        let source=self.source();
        for (knots,degree,count,domain) in [(&source.knots_u,source.degree_u,source.control_points.len(),union[0]),
            (&source.knots_v,source.degree_v,source.control_points[0].len(),union[1])] {
            if degree==0 || !continuous_knots(knots,degree,count,domain) {return Ok((None,0));}
        }
        if max_spans==0 {return Ok((None,0));}
        let (jacobian,mut spent)=cache.enclose(self,union,max_spans)?;
        let Some(mut jacobian)=jacobian else {return Ok((None,spent));};
        if !crate::surface_self_chord::inspect_jacobian_rank_bound(jacobian)? {
            // Connect A to B through (B.u,A.v): U varies only on A's
            // V strip, then V only on B's U strip. Continuity above covers
            // both paths. Each integrated column bounds its own full strip.
            let strips=[[union[0],pair[0][1]],[pair[1][0],union[1]]];
            let mut columns=jacobian;
            for axis in 0..2 {
                if spent==max_spans {return Ok((None,spent));}
                let (bounds,used)=cache.enclose(self,strips[axis],max_spans-spent)?;
                spent+=used;
                let Some(bounds)=bounds else {return Ok((None,spent));};
                for k in 0..3 {columns[k][axis]=bounds[k][axis];}
            }
            jacobian=columns;
        }
        // Direct M*(B-A) interval exclusion proves its own nonzero chord;
        // only the mean-matrix theorem requires uniform rank admission.
        let ranked=crate::surface_self_chord::inspect_jacobian_rank_bound(jacobian)?;
        let mut normals=[[[0.;2];3];2];
        for (index,endpoint) in pair.into_iter().enumerate() {
            if spent==max_spans {return Ok((None,spent));}
            // Only chord/normal angles are used here; strictly interior pole
            // factors preserve direction without changing Jacobian magnitudes.
            let (normal,used)=cache.profile_direction(self,endpoint,max_spans-spent)?;
            spent+=used;
            let Some(normal)=normal else {return Ok((None,spent));};
            normals[index]=normal;
            if crate::surface_self_chord::inspect_jacobian_endpoint_pair_chords(jacobian,pair,normal,max_sine_squared)?
                || (ranked && crate::surface_self_chord::inspect_jacobian_endpoint_chords(jacobian,normal,max_sine_squared)?.0) {
                return Ok((Some(true),spent));
            }
        }
        // Both coordinate paths bound the same exact profile chord, even
        // when the wider union already has full rank. Only complete paths
        // participate in this independent chord-box intersection.
        let mut chord=[[f64::NEG_INFINITY,f64::INFINITY];3];
        let mut delta=[I::point(0.);2];
        for axis in 0..2 {
            delta[axis]=I::new(pair[1][axis][0],pair[1][axis][1])?
                .sub(I::new(pair[0][axis][0],pair[0][axis][1])?)?;
        }
        for reversed in [false,true] {
            let strips=if reversed {[[union[0],pair[1][1]],[pair[0][0],union[1]]]}
                else {[[union[0],pair[0][1]],[pair[1][0],union[1]]]};
            let mut columns=jacobian;
            for axis in 0..2 {
                if spent==max_spans {return Ok((None,spent));}
                let (jet,used)=cache.enclose(self,strips[axis],max_spans-spent)?;spent+=used;
                let Some(jet)=jet else {return Ok((None,spent));};
                for k in 0..3 {columns[k][axis]=jet[k][axis];}
            }
            for k in 0..3 {
                let value=I::new(columns[k][0][0],columns[k][0][1])?.mul(delta[0])?
                    .add(I::new(columns[k][1][0],columns[k][1][1])?.mul(delta[1])?)?;
                chord[k]=[chord[k][0].max(value.lo),chord[k][1].min(value.hi)];
                I::new(chord[k][0],chord[k][1])?;
            }
            // The first complete path is sufficient when its chord box
            // already excludes alignment. Otherwise the reverse path tightens
            // the same exact chord by interval intersection as before.
            for normal in normals {
                if crate::normal_alignment::inspect_pair_bounds([normal,chord],max_sine_squared)?.aligned==Some(false) {
                    return Ok((Some(true),spent));
                }
            }
        }
        let mut positions=[[[0.;2];3];2];
        for (index,endpoint) in pair.into_iter().enumerate() {
            if spent==max_spans {return Ok((None,spent));}
            let (position,used)=cache.position(self,endpoint,max_spans-spent)?;spent+=used;
            let Some(position)=position else {return Ok((None,spent));};positions[index]=position;
        }
        for k in 0..3 {
            let value=I::new(positions[1][k][0],positions[1][k][1])?
                .sub(I::new(positions[0][k][0],positions[0][k][1])?)?;
            let overlap=I::new(chord[k][0],chord[k][1])?.intersect(value.lo,value.hi)?;
            chord[k]=[overlap.lo,overlap.hi];
        }
        for normal in normals {
            if crate::normal_alignment::inspect_pair_bounds([normal,chord],max_sine_squared)?.aligned==Some(false) {
                return Ok((Some(true),spent));
            }
        }
        Ok((Some(false),spent))
    }
/// Inspect normal-aligned self chords of the exact separable-weight profile.
    /// Positive recipe weights preserve chart continuity; derivative bounds are
    /// recomputed from this profile rather than inherited from its source.
    pub fn inspect_self_chords(
        &self, domain: [[f64; 2]; 2], max_sine_squared: f64, max_spans: usize,
    ) -> Result<(Option<bool>, usize)> {
        self.inspect_self_chords_cached(domain,max_sine_squared,max_spans,&mut ProfileDerivativeCache::default())
    }
pub(super) fn inspect_self_chords_cached(
        &self,domain:[[f64;2];2],max_sine_squared:f64,max_spans:usize,
        cache:&mut ProfileDerivativeCache,
    ) -> Result<(Option<bool>,usize)> {
        let source = self.source();
        let knots = [&source.knots_u, &source.knots_v];
        let degrees = [source.degree_u, source.degree_v];
        let counts = [source.control_points.len(), source.control_points[0].len()];
        check(max_spans <= 100000 && max_sine_squared.is_finite()
            && (0. ..1.).contains(&max_sine_squared)
            && (0..2).all(|axis| domain[axis].iter().all(|x| x.is_finite())
                && domain[axis][0] < domain[axis][1]
                && domain[axis][0] >= knots[axis][degrees[axis]]
                && domain[axis][1] <= knots[axis][counts[axis]]),
            "Bound profile self-chord domain, angle and work")?;
        if (0..2).any(|axis| degrees[axis] == 0
            || !continuous_knots(knots[axis], degrees[axis], counts[axis], domain[axis]))
            || max_spans == 0 {
            return Ok((None, 0));
        }
        let (jacobian, spent) = cache.enclose(self,domain,max_spans)?;
        let Some(jacobian) = jacobian else { return Ok((None, spent)); };
        let (excluded, _) = crate::surface_self_chord::inspect_jacobian_self_chords(
            jacobian, max_sine_squared)?;
        Ok((Some(excluded), spent))
    }
}
