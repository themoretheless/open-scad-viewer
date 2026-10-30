//! surface self chord operations: coordinate_path.
use super::*;
impl MeanMatrix<'_> {
/// Integrate U on A's V strip, then V on B's U strip. Each
    /// column bounds its entire path for all endpoints in A x B.
    pub(crate) fn coordinate_path(&self, pair:[[[f64;2];2];2],
        u_path:&MeanMatrix<'_>,v_path:&MeanMatrix<'_>)->Result<CoordinatePath<'_>> {
        let union=std::array::from_fn(|a|[pair[0][a][0].min(pair[1][a][0]),pair[0][a][1].max(pair[1][a][1])]);
        check(pair.iter().all(|rect|rect.iter().all(|axis|axis[0].is_finite()&&axis[1].is_finite()&&axis[0]<=axis[1]))
            && std::ptr::eq(self.surface,u_path.surface)&&std::ptr::eq(self.surface,v_path.surface)
            && (0..2).all(|a|self.domain[a][0]<=union[a][0]&&union[a][1]<=self.domain[a][1])
            && u_path.domain[0][0]<=union[0][0]&&union[0][1]<=u_path.domain[0][1]
            && u_path.domain[1][0]<=pair[0][1][0]&&pair[0][1][1]<=u_path.domain[1][1]
            && v_path.domain[0][0]<=pair[1][0][0]&&pair[1][0][1]<=v_path.domain[0][1]
            && v_path.domain[1][0]<=union[1][0]&&union[1][1]<=v_path.domain[1][1],
            "Coordinate path matrices must cover original endpoint strips")?;
        Ok(CoordinatePath{pair,matrix:MeanMatrix{surface:self.surface,domain:union,
            jacobian:std::array::from_fn(|k|[u_path.jacobian[k][0],v_path.jacobian[k][1]])}})
    }
pub(crate) fn exclude_pair(
        &self, pair: [[[f64; 2]; 2]; 2], sine: f64, max_spans: usize,
        cached: Option<(&crate::factored_surface_normals::Certificate<'_>,
            &crate::factored_surface_normals::PairBounds<'_>)>,
        original: Option<&normal_alignment::OriginalPairBounds<'_>>,
    ) -> Result<(bool,usize)> {
        self.exclude_pair_with_normal(pair,sine,max_spans,cached,original,
            &mut |domain,budget|normal_alignment::normal_bounds(self.surface,domain,budget))
    }
pub(super) fn exclude_pair_with_normal(
        &self, pair: [[[f64; 2]; 2]; 2], sine: f64, max_spans: usize,
        cached: Option<(&crate::factored_surface_normals::Certificate<'_>,
            &crate::factored_surface_normals::PairBounds<'_>)>,
        original: Option<&normal_alignment::OriginalPairBounds<'_>>,
        normal: &mut impl FnMut([[f64;2];2],usize)->Result<(Option<[[f64;2];3]>,usize)>,
    ) -> Result<(bool,usize)> {
        check(sine.is_finite() && (0. ..1.).contains(&sine)
            && pair.iter().all(|rect| (0..2).all(|a| rect[a][0].is_finite()
                && rect[a][1].is_finite() && self.domain[a][0]<=rect[a][0]
                && rect[a][0]<=rect[a][1] && rect[a][1]<=self.domain[a][1])),
            "Mean-path endpoints must lie in their original continuous rectangle")?;
        let components = if let Some((proof,bounds))=cached {
            check(std::ptr::eq(self.surface,proof.surface()),"Use original mean-path normal source")?;
            Some(bounds.components(proof,pair)?)
        } else if let Some(bounds)=original {
            Some(bounds.components(self.surface,pair)?)
        } else { None };
        let mut j=self.jacobian;
        for a in 0..2 {
            let scale=j.iter().map(|row|row[a].lo.abs().max(row[a].hi.abs())).fold(0_f64,f64::max);
            if scale>=f64::MIN_POSITIVE && scale.is_finite() {
                for row in &mut j { row[a]=row[a].div(I::point(scale))?; }
            }
        }
        // Rank is required only for the mean-matrix cosine theorem. A direct
        // endpoint chord enclosure can establish nonzero direction independently.
        let ranked=mean_matrix_lower(j)?>0.;
        let threshold=I::point(1.).sub(I::point(sine))?.lo;
        let mut used=0;
        for (index,rect) in pair.into_iter().enumerate() {
            let (bounds,spent)=if let Some(values)=components {(Some(values[index]),0)}
                else {
                    if used==max_spans { break; }
                    normal(rect,max_spans-used)?
                };
            used+=spent;
            let Some(bounds)=bounds else {continue};
            if inspect_jacobian_endpoint_pair_chords(
                self.jacobian.map(|row|row.map(|d|[d.lo,d.hi])),pair,bounds,sine)? {
                return Ok((true,used));
            }
            let scale=bounds.iter().flatten().map(|x|x.abs()).fold(0_f64,f64::max);
            if scale<f64::MIN_POSITIVE {continue;}
            let mut n=[I::point(0.);3];
            for k in 0..3 {n[k]=I::new(bounds[k][0],bounds[k][1])?.div(I::point(scale))?;}
            if ranked && endpoint_matrix_cosine(j,n)?<threshold {return Ok((true,used));}
        }
        Ok((false,used))
    }
}
