//! normal alignment operations: new.
use super::*;
impl<'a> OriginalNormalCache<'a> {
pub fn new(surface:&'a Surface)->Self {
        Self{surface,rectangles:std::collections::HashMap::new(),factored_rectangles:std::collections::HashMap::new()}
    }
pub fn inspect_product(&mut self, other:&mut Self,
        domains:[[[f64;2];2];2], sine:f64, max_cells:usize, max_spans:usize)
        ->Result<ProductReport> {
        self.inspect_product_with_factors(other,domains,sine,max_cells,max_spans,[None,None],0)
    }
/// Closed normal-direction coverage using exact source-bound limiting factors.
    /// Factored directions do not establish a regular tangent plane at a pole.
    pub fn inspect_product_with_factors(&mut self, other:&mut Self,
        domains:[[[f64;2];2];2], sine:f64, max_cells:usize, max_spans:usize,
        factors:[Option<&crate::factored_surface_normals::Certificate<'_>>;2],
        max_factored_queries:usize)->Result<ProductReport> {
        check(max_factored_queries<=100000,"Factored normal query budget")?;
        for (surface,factor) in [self.surface,other.surface].into_iter().zip(factors) {
            if let Some(c)=factor {check(std::ptr::eq(surface,c.surface()),"Use original factored normal source")?;}
        }
        check((1..=100000).contains(&max_cells),"Normal product cell budget")?;
        validate_input(self.surface,domains[0],[1.,0.,0.],sine,max_spans.max(1))?;
        validate_input(other.surface,domains[1],[1.,0.,0.],sine,max_spans.max(1))?;
        let mut out=ProductReport{excluded:false,cells:0,spans:0,factored_queries:0,
            unresolved:vec![domains],reason:"product-cell-limit"};
        while out.cells<max_cells && !out.unresolved.is_empty() {
            let pair=out.unresolved.pop().unwrap();out.cells+=1;
            let (r,queries)=self.inspect_other_with_factors(other,pair,sine,max_spans-out.spans,factors,max_factored_queries-out.factored_queries)?;
            out.factored_queries+=queries;
            out.spans+=r.spans;
            if r.aligned==Some(false) {continue;}
            if r.aligned==Some(true) {
                out.unresolved.push(pair);out.reason="product-aligned-region";break;
            }
            if (factors.iter().any(Option::is_none) && out.spans==max_spans) ||
                (factors.iter().any(Option::is_some) && out.factored_queries==max_factored_queries && r.normal_components.iter().any(Option::is_none)) {
                out.unresolved.push(pair);out.reason="product-span-limit";break;
            }
            let mut choice=None;let mut largest=0.;
            for side in 0..2 {for axis in 0..2 {
                let full=domains[side][axis][1]-domains[side][axis][0];
                if full<=0. {continue;}
                let width=(pair[side][axis][1]-pair[side][axis][0])/full;
                if width>largest {largest=width;choice=Some((side,axis));}
            }}
            let Some((side,axis))=choice else {
                out.unresolved.push(pair);out.reason="product-resolution-limit";break;
            };
            let d=pair[side][axis];let mid=d[0]*0.5+d[1]*0.5;
            if !(d[0]<mid && mid<d[1]) {
                out.unresolved.push(pair);out.reason="product-resolution-limit";break;
            }
            let mut left=pair;let mut right=pair;
            left[side][axis][1]=mid;right[side][axis][0]=mid;
            out.unresolved.push(right);out.unresolved.push(left);
        }
        if out.unresolved.is_empty() {
            out.excluded=true;out.reason="product-normal-excluded";
        }
        Ok(out)
    }
/// Compare complete original rectangles, charging only new source queries.
    /// Private source references and cache keys prevent rebinding paid hulls.
    pub fn inspect_other(&mut self, other: &mut Self,
        domains: [[[f64; 2]; 2]; 2], sine: f64, budget: usize) -> Result<PairReport> {
        Ok(self.inspect_other_with_factors(other,domains,sine,budget,[None,None],0)?.0)
    }
pub fn inspect_other_with_factors(&mut self, other:&mut Self,
        domains:[[[f64;2];2];2], sine:f64, budget:usize,
        factors:[Option<&crate::factored_surface_normals::Certificate<'_>>;2],
        query_budget:usize)->Result<(PairReport,usize)> {
        check(query_budget<=100000,"Bound original factored normal queries")?;
        for (surface,factor) in [self.surface,other.surface].into_iter().zip(factors) {
            if let Some(c)=factor {check(std::ptr::eq(surface,c.surface()),"Use original factored normal source")?;}
        }
        validate_input(self.surface,domains[0],[1.,0.,0.],sine,budget.max(1))?;
        validate_input(other.surface,domains[1],[1.,0.,0.],sine,budget.max(1))?;
        let mut spent=0; let mut queries=0; let mut components=[None;2];
        for (side,cache) in [self,other].into_iter().enumerate() {
            let rect=domains[side];let key=rect.map(|axis|axis.map(f64::to_bits));
            if let Some(c)=factors[side] {
                let factor_key=(c.end(),key);
                if let Some(hull)=cache.factored_rectangles.get(&factor_key) {components[side]=Some(*hull);}
                else if queries<query_budget {
                    let hull=c.bounds(rect)?;queries+=1;
                    cache.factored_rectangles.insert(factor_key,hull);components[side]=Some(hull);
                }
            } else if let Some(hull)=cache.rectangles.get(&key) {components[side]=Some(*hull);}
            else if spent<budget {
                let (hull,used)=normal_bounds(cache.surface,rect,budget-spent)?;
                spent+=used;components[side]=hull;
                if let Some(hull)=hull {cache.rectangles.insert(key,hull);}
            }
        }
        let [Some(a),Some(b)]=components else {
            return Ok((PairReport{aligned:None,sine_squared_interval:None,
                normal_components:components,spans:spent,reason:"span-limit"},queries));
        };
        let mut report=inspect_pair_bounds([a,b],sine)?;report.spans=spent;Ok((report,queries))
    }
pub(crate) fn inspect_pair(&mut self,pair:[[[f64;2];2];2],sine:f64,budget:usize)
        ->Result<(PairReport,Option<OriginalPairBounds<'a>>,usize)> {
        // Validate both original rectangles even if no new query is needed.
        for rect in pair {validate_input(self.surface,rect,[1.,0.,0.],sine,budget.max(1))?;}
        let mut components=[None;2];let mut spent=0;let mut hits=0;
        for (index,rect) in pair.into_iter().enumerate() {
            let key=rect.map(|axis|axis.map(f64::to_bits));
            if let Some(bounds)=self.rectangles.get(&key) {
                components[index]=Some(*bounds);hits+=1;
            } else if spent<budget {
                let (bounds,used)=normal_bounds(self.surface,rect,budget-spent)?;
                spent+=used;components[index]=bounds;
                if let Some(bounds)=bounds {self.rectangles.insert(key,bounds);}
            }
        }
        let [Some(a),Some(b)]=components else {
            return Ok((PairReport{aligned:None,sine_squared_interval:None,
                normal_components:components,spans:spent,reason:"span-limit"},None,hits));
        };
        let mut report=inspect_pair_bounds([a,b],sine)?;report.spans=spent;
        Ok((report,Some(OriginalPairBounds{surface:self.surface,pair,components:[a,b]}),hits))
    }
}
