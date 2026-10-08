//! Original affine boundary charts on one privately admitted shared carrier.
//! Parameter correspondence alone proves neither chord direction nor wall thickness.
use crate::source_volume::Body;
use nurbs_core::{Error,Result,surface::Surface,interval_eval::Interval as I};

pub struct SharedBoundary<'a> {
    body:&'a Body,faces:[usize;2],edge:usize,
    surfaces:[&'a Surface;2],fixed_axes:[usize;2],fixed:[f64;2],free_ends:[[f64;2];2],
}
impl<'a> SharedBoundary<'a> {
    pub fn body(&self)->&'a Body{self.body}
    pub fn faces(&self)->[usize;2]{self.faces}
    pub fn edge(&self)->usize{self.edge}
    pub fn surfaces(&self)->[&'a Surface;2]{self.surfaces}
    pub fn fixed_axes(&self)->[usize;2]{self.fixed_axes}
    pub fn fixed_values(&self)->[f64;2]{self.fixed}
    pub fn free_endpoints(&self)->[[f64;2];2]{self.free_ends}
    /// Enclose the derivative of the admitted common carrier over both endpoint
    /// parameter intervals. Source identity comes from this private Body map.
    pub fn tangent_bounds(&self, rectangles:[[[f64;2];2];2], max_spans:usize)
        ->Result<(Option<[[f64;2];3]>,usize)> {
        let bounds=[self.canonical_bounds(0,rectangles[0])?,
            self.canonical_bounds(1,rectangles[1])?];
        let parameter=I::new(bounds[0][0].min(bounds[1][0]),bounds[0][1].max(bounds[1][1]))?;
        let [first,last]=self.free_ends[0];
        let scale=I::point(last).sub(I::point(first))?;
        let free=I::point(first).add(scale.mul(parameter)?)?;
        let axis=1-self.fixed_axes[0];
        let low=first.min(last);let high=first.max(last);
        let free=free.intersect(low,high)?;
        let mut domain=[[self.fixed[0];2];2];domain[axis]=[free.lo,free.hi];
        let (proof,spent)=nurbs_core::normal_alignment::derivative_path_bounds(self.surfaces[0],domain,axis,max_spans)?;
        let Some(proof)=proof else{return Ok((None,spent));};
        let mut result=[[0.;2];3];
        for (k,r) in proof.components().into_iter().enumerate(){
            let derivative=I::new(r[0],r[1])?.mul(scale)?;
            result[k]=[derivative.lo,derivative.hi];
        }
        Ok((Some(result),spent))
    }
    /// Exclude a normal-aligned chord on the full endpoint product. A common
    /// carrier coefficient stays unrestricted; transverse displacements have
    /// known signs because both paths start at natural chart boundaries.
    /// Returns exclusion, original derivative spans, and normal spans consumed.
    pub fn exclude_normal_chords(&self, rectangles:[[[f64;2];2];2], sine:f64,
        max_spans:usize,max_normal_spans:usize)->Result<(bool,usize,usize)> {
        self.exclude_normal_chords_impl(rectangles,sine,max_spans,max_normal_spans,0)
    }
    /// Optional interval mean integration. It spends up to eight path pieces
    /// per transverse column and is not selected by automatic wall coverage.
    pub fn exclude_normal_chords_with_mean(&self, rectangles:[[[f64;2];2];2], sine:f64,
        max_spans:usize,max_normal_spans:usize)->Result<(bool,usize,usize)> {
        self.exclude_normal_chords_impl(rectangles,sine,max_spans,max_normal_spans,1)
    }
    /// Correlated original rational secants where the source layout is supported.
    pub fn exclude_normal_chords_with_secant(&self, rectangles:[[[f64;2];2];2], sine:f64,
        max_spans:usize,max_normal_spans:usize)->Result<(bool,usize,usize)> {
        self.exclude_normal_chords_impl(rectangles,sine,max_spans,max_normal_spans,2)
    }
    fn exclude_normal_chords_impl(&self, rectangles:[[[f64;2];2];2], sine:f64,
        max_spans:usize,max_normal_spans:usize,method:usize)->Result<(bool,usize,usize)> {
        if !sine.is_finite()||!(0. ..1.).contains(&sine) {
            return Err(Error::new("BREP_SHARED_CHORD_MAP","Require squared sine in [0,1)"));
        }
        // Validate both endpoint products before any partial geometric work.
        for side in 0..2 {self.canonical_bounds(side,rectangles[side])?;}
        if max_spans==0||max_normal_spans==0 {return Ok((false,0,0));}
        let (tangent,mut spent)=self.tangent_bounds(rectangles,max_spans)?;
        let Some(tangent)=tangent else{return Ok((false,spent,0));};
        let mut transverse=[[[0.;2];3];2];
        for side in 0..2 {
            if spent==max_spans {return Ok((false,spent,0));}
            let axis=self.fixed_axes[side];let s=self.surfaces[side];
            let knots=[&s.knots_u,&s.knots_v];let degrees=[s.degree_u,s.degree_v];
            let direction=if self.fixed[side]==knots[axis][degrees[axis]] {1.}else{-1.};
            let sign=direction*if side==0 {-1.}else{1.};
            let (components,used)=if method==2 {
                let (proof,used)=nurbs_core::normal_alignment::boundary_secant_bounds(s,rectangles[side],axis,self.fixed[side],max_spans-spent)?;
                (proof.map(|p|p.components()),used)
            } else if method==1 {
                let (proof,used)=nurbs_core::normal_alignment::boundary_mean_bounds(s,rectangles[side],axis,self.fixed[side],8,max_spans-spent)?;
                (proof.map(|p|p.components()),used)
            } else {
                let mut path=rectangles[side];
                path[axis]=[path[axis][0].min(self.fixed[side]),path[axis][1].max(self.fixed[side])];
                let (proof,used)=nurbs_core::normal_alignment::derivative_path_bounds(s,path,axis,max_spans-spent)?;
                (proof.map(|p|p.components()),used)
            };
            spent+=used;
            let Some(components)=components else{return Ok((false,spent,0));};
            for (k,r) in components.into_iter().enumerate(){
                let g=I::new(r[0],r[1])?.mul(I::point(sign))?;
                transverse[side][k]=[g.lo,g.hi];
            }
        }
        let mut normal_spent=0;
        for side in 0..2 {
            if normal_spent==max_normal_spans {break;}
            let (normal,used)=nurbs_core::normal_alignment::normal_bounds(self.surfaces[side],rectangles[side],max_normal_spans-normal_spent)?;
            normal_spent+=used;
            if let Some(normal)=normal {
                if nurbs_core::normal_alignment::shared_chord_cone_excluded(tangent,transverse,normal,sine)? {
                    return Ok((true,spent,normal_spent));
                }
            }
        }
        Ok((false,spent,normal_spent))
    }
    /// Outward arithmetic applies the original affine map; no rounded inverse
    /// or evaluated world coincidence is admitted as a parameter identity.
    pub fn canonical_bounds(&self,side:usize,rectangle:[[f64;2];2])->Result<[f64;2]> {
        if side>1 {return Err(Error::new("BREP_SHARED_CHORD_MAP","Choose an owned boundary side"));}
        let s=self.surfaces[side];let counts=[s.control_points.len(),s.control_points[0].len()];
        let knots=[&s.knots_u,&s.knots_v];let degrees=[s.degree_u,s.degree_v];
        for a in 0..2 {
            let r=rectangle[a];
            if !r[0].is_finite()||!r[1].is_finite()||r[0]>r[1]
                ||r[0]<knots[a][degrees[a]]||r[1]>knots[a][counts[a]] {
                return Err(Error::new("BREP_SHARED_CHORD_MAP","Keep endpoints inside their original source chart"));
            }
        }
        let [first,last]=self.free_ends[side];let r=rectangle[1-self.fixed_axes[side]];
        if r[0]<first.min(last)||r[1]>first.max(last) {
            return Err(Error::new("BREP_SHARED_CHORD_MAP","The original carrier map must cover the free coordinate"));
        }
        let t=I::new(r[0],r[1])?.sub(I::point(first))?
            .div_signed(I::point(last).sub(I::point(first))?)?.intersect(0.,1.)?;
        Ok([t.lo,t.hi])
    }
}
fn same_surface(a:&Surface,b:&Surface)->bool {
    a.degree_u==b.degree_u&&a.degree_v==b.degree_v&&a.periodic_u==b.periodic_u&&a.periodic_v==b.periodic_v
        &&a.knots_u==b.knots_u&&a.knots_v==b.knots_v&&a.control_points==b.control_points&&a.weights==b.weights
}
/// Inspect a bounded set of already admitted Body edges. Only an exact,
/// constant-weight linear pcurve on a natural chart boundary is supported.
/// Root-ended retained uses may be supported when their full source maps are
/// already admitted; this does not promote those retained ends to full coverage.
pub fn prepare<'a>(body:&'a Body,faces:[usize;2],max_edges:usize)->Result<Option<SharedBoundary<'a>>> {
    let shell=body.geometry().shell();let regions=shell.regions().ok_or_else(||Error::new("BREP_SHARED_CHORD_MAP","Require retained original regions"))?;
    if faces[0]==faces[1]||faces.iter().any(|&f|f>=regions.len())||max_edges>100000 {
        return Err(Error::new("BREP_SHARED_CHORD_MAP","Choose distinct owned faces and bounded edge inspection"));
    }
    let surfaces=faces.map(|f|regions[f].loops()[0][0].surface());
    for (index,addresses) in shell.uses().iter().enumerate().take(max_edges) {
        let order=if [addresses[0].face,addresses[1].face]==faces {[0,1]}
            else if [addresses[1].face,addresses[0].face]==faces {[1,0]}else{continue;};
        let edge=&shell.edges()[index];if !edge.maps_complete_canonical_source(){continue;}
        let mut axes=[0;2];let mut fixed=[0.;2];let mut ends=[[0.;2];2];let mut supported=true;
        for side in 0..2 {
            let use_=&edge.uses()[order[side]];let p=use_.curve();let s=surfaces[side];
            if !same_surface(use_.surface(),s)||p.periodic||p.degree!=1||p.control_points.len()!=2
                ||p.weights[0]!=p.weights[1]||p.control_points.iter().any(|v|v.len()!=2) {supported=false;break;}
            let count=[s.control_points.len(),s.control_points[0].len()];let degree=[s.degree_u,s.degree_v];let knots=[&s.knots_u,&s.knots_v];
            let axis=(0..2).find(|&a|p.control_points[0][a]==p.control_points[1][a]
                &&[knots[a][degree[a]],knots[a][count[a]]].contains(&p.control_points[0][a]));
            let Some(axis)=axis else{supported=false;break;};
            let mut range=[p.control_points[0][1-axis],p.control_points[1][1-axis]];
            if range[0]==range[1] {supported=false;break;}
            if edge.reversed()[order[side]]^use_.reversed(){range.reverse();}
            axes[side]=axis;fixed[side]=p.control_points[0][axis];ends[side]=range;
        }
        if supported {return Ok(Some(SharedBoundary{body,faces,edge:index,surfaces,fixed_axes:axes,fixed,free_ends:ends}));}
    }
    Ok(None)
}
