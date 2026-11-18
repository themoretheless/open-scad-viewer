//! Exhaustive symmetric endpoint-product coverage for one owned expression.
use super::ProfileSectionBridge;
use crate::{check,Result};
type Rect=[[f64;2];2];
type Pair=[Rect;2];
#[derive(Debug)]
pub struct ProfileBridgeChordCoverage {
    pub cells:usize,
    pub spans:usize,
    pub exclusions:usize,
    /// Each representative also covers its endpoint-exchanged product.
    pub unresolved_pairs:Vec<Pair>,
    /// Excludes normal-aligned chords only; never admits a shell or thickness.
    pub complete_exclusion:bool,
}
impl ProfileSectionBridge<'_,'_,'_,'_> {
    pub fn inspect_self_chord_coverage(&self,domain:Rect,sine:f64,max_cells:usize,max_spans:usize)
        ->Result<ProfileBridgeChordCoverage> {
        // Validation is mandatory even when no work can be spent.
        self.inspect_self_chords(domain,sine,0)?;
        check(max_cells<=100000 && max_spans<=100000,"Bound whole bridge endpoint coverage")?;
        let mut out=ProfileBridgeChordCoverage{cells:0,spans:0,exclusions:0,
            unresolved_pairs:Vec::new(),complete_exclusion:false};
        let mut stack=vec![[domain,domain]];
        while let Some(pair)=stack.pop() {
            if out.cells==max_cells || out.spans==max_spans {
                out.unresolved_pairs.push(pair);out.unresolved_pairs.extend(stack.drain(..));break;
            }
            out.cells+=1;
            let (excluded,spent)=if pair[0]==pair[1] {
                self.inspect_self_chords(pair[0],sine,max_spans-out.spans)?
            }else{self.inspect_self_chord_pair(pair,sine,max_spans-out.spans)?};
            out.spans+=spent;
            if excluded==Some(true){out.exclusions+=1;continue;}
            // Unsupported continuity or incomplete derivative work gives no
            // exclusion authority. Retain the entire endpoint obligation.
            if excluded.is_none(){out.unresolved_pairs.push(pair);continue;}
            let mut widest=(0,0,0.);
            for endpoint in 0..if pair[0]==pair[1]{1}else{2} {for axis in 0..2 {
                let width=(pair[endpoint][axis][1]-pair[endpoint][axis][0])/(domain[axis][1]-domain[axis][0]);
                if width>widest.2 {widest=(endpoint,axis,width);}
            }}
            let (endpoint,axis,_)=widest;
            let [lo,hi]=pair[endpoint][axis];let mid=lo*0.5+hi*0.5;
            if !(lo<mid && mid<hi){out.unresolved_pairs.push(pair);continue;}
            let mut left=pair[endpoint];let mut right=left;left[axis][1]=mid;right[axis][0]=mid;
            if pair[0]==pair[1] {
                // LL/LR/RR and the implicit swapped LR cover A×A, including
                // the shared split boundary. No near-diagonal products drop.
                stack.extend([[right,right],[left,right],[left,left]]);
            }else{
                let mut l=pair;let mut r=pair;l[endpoint]=left;r[endpoint]=right;stack.extend([r,l]);
            }
        }
        out.complete_exclusion=out.unresolved_pairs.is_empty();Ok(out)
    }
}
