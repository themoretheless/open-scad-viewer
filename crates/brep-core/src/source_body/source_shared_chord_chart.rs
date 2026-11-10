//! Outward affine conversion on a privately admitted original pcurve map.
use nurbs_core::{Result,interval_eval::Interval as I};
#[derive(Clone,Copy)]
pub(super) struct Chart {
    pub free:[f64;2],
    pub canonical:[[f64;2];2],
}
impl Chart {
    pub fn ends(self)->Result<[I;2]> {
        Ok([I::point(self.canonical[0][0]).div(I::point(self.canonical[0][1]))?,
            I::point(self.canonical[1][0]).div(I::point(self.canonical[1][1]))?])
    }
    pub fn to_canonical(self,free:I)->Result<I> {
        let [a,b]=self.ends()?;
        let q=free.sub(I::point(self.free[0]))?
            .div_signed(I::point(self.free[1]).sub(I::point(self.free[0]))?)?;
        a.add(q.mul(b.sub(a)?)?)
    }
    pub fn to_free(self,canonical:I)->Result<I> {
        let [a,b]=self.ends()?;
        I::point(self.free[0]).add(canonical.sub(a)?.div_signed(b.sub(a)?)?
            .mul(I::point(self.free[1]).sub(I::point(self.free[0]))?)?)
    }
    pub fn derivative_scale(self)->Result<I> {
        let [a,b]=self.ends()?;
        I::point(self.free[1]).sub(I::point(self.free[0]))?.div_signed(b.sub(a)?)
    }
    /// An inner common domain ensures both original charts cover every chord
    /// anchor; uncertain rational endpoints may conservatively refuse queries.
    pub fn common(a:Self,b:Self)->Result<Option<[f64;2]>> {
        let aa=a.ends()?;let bb=b.ends()?;
        let lo=aa[0].hi.min(aa[1].hi).max(bb[0].hi.min(bb[1].hi));
        let hi=aa[0].lo.max(aa[1].lo).min(bb[0].lo.max(bb[1].lo));
        Ok((lo<hi).then_some([lo,hi]))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_and_reversed_rational_charts_keep_outward_maps_and_derivatives() {
        for reverse in [false,true] {
            let canonical=if reverse {[[2.,3.],[1.,3.]]}else{[[1.,3.],[2.,3.]]};
            let chart=Chart {free:[10.,18.],canonical};
            let q=chart.to_canonical(I::point(14.)).unwrap();
            assert!(q.lo<=0.5 && 0.5<=q.hi);
            let free=chart.to_free(I::point(0.5)).unwrap();
            assert!(free.lo<=14. && 14.<=free.hi);
            let scale=chart.derivative_scale().unwrap();
            let exact=if reverse {-24.}else{24.};
            assert!(scale.lo<=exact && exact<=scale.hi);
            let common=Chart::common(chart,Chart {free:[0.,1.],canonical:[[0.,1.],[1.,1.]]}).unwrap().unwrap();
            assert!(common[0]>=1./3. && common[1]<=2./3.);
        }
        assert!(Chart::common(Chart{free:[0.,1.],canonical:[[0.,1.],[1.,4.]]},
            Chart{free:[0.,1.],canonical:[[3.,4.],[1.,1.]]}).unwrap().is_none());
    }
}
