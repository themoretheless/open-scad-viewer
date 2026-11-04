//! Complete third-surface census excludes additional face-boundary contacts.
use super::*;
#[derive(Clone,Copy,Debug)]
pub struct RootChordBoundaryLimits {
    pub max_censuses:usize,pub max_entries:usize,pub max_identity_queries:usize,
    pub algebra:ss::ExactSsGroebnerLimits,pub branch:ss::SsThirdSurfaceBranchLimits,
    pub clock:ss::SsAlgebraicClockLimits,pub geometry:ss::OriginalSsCurveLimits,
}
impl Default for RootChordBoundaryLimits {
    fn default()->Self {Self {max_censuses:4096,max_entries:8192,max_identity_queries:16384,
        algebra:Default::default(),branch:Default::default(),clock:Default::default(),geometry:Default::default()}}
}
#[derive(Clone,Debug)]
pub struct QualifiedRootChordBoundary {
    split:RootFaceChordSplit,limits:RootChordBoundaryLimits,
    censuses:Vec<Arc<ss::QualifiedSsThirdSurfaceBranchCensus>>,entries:usize,identities:usize,
}
impl QualifiedRootChordBoundary {
    /// Sufficient proof: the chord's other original surface meets the complete
    /// finite face boundary only at its two explicitly UV-bound endpoints.
    /// Extra contacts outside the chord image may conservatively be refused.
    pub fn qualify(split:RootFaceChordSplit,limits:RootChordBoundaryLimits)->Result<Self> {
        if limits.max_censuses==0 || limits.max_censuses>16384 || limits.max_entries==0
            || limits.max_entries>65536 || limits.max_identity_queries==0 || limits.max_identity_queries>131072 {
            return Err(unproved("Invalid chord-boundary census budget"));
        }
        let chord=split.chord();let sources=chord.curve().source();
        let third=if split.support()==0 {sources.second()} else {sources.first()};
        let spans:Vec<_>=(third.degree_u..third.control_points.len()).flat_map(|u|
            (third.degree_v..third.control_points[0].len()).filter_map(move |v|
                (third.knots_u[u]<third.knots_u[u+1] && third.knots_v[v]<third.knots_v[v+1]).then_some([u,v]))).collect();
        let source=split.source();let t=source.topology();let f=&t.faces[split.face_children()[0]];
        let count=std::iter::once(f.outer).chain(f.holes.iter().copied()).map(|i|t.loops[i].coedges.len()).sum::<usize>();
        if spans.is_empty() || count.checked_mul(spans.len()).is_none_or(|n|n>limits.max_censuses) {
            return Err(unproved("Complete chord-boundary span census exceeds budget"));
        }
        let mut censuses=Vec::new();let mut entries=0;let mut identities=0;
        for id in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for c in &t.loops[id].coedges {
                let edge=&t.edges[c.edge].curve;let range=edge.parameter_bounds();
                for &span in &spans {
                    let candidates=Arc::new(edge.curve().third_surface_candidates(third,span,limits.algebra)?);
                    let census=Arc::new(ss::QualifiedSsThirdSurfaceBranchCensus::qualify(candidates,edge.curve().clone(),limits.branch)?);
                    for entry in census.entries() {
                        if entries==limits.max_entries {return Err(unproved("Chord-boundary root-entry budget exhausted"));}entries+=1;
                        if !entry.is_inside() {continue;}
                        let bounds=entry.point().clock_bounds()?;
                        if bounds[1]<range[0] || range[1]<bounds[0] {continue;}
                        let mut allowed=false;
                        for endpoint in [chord.start(),chord.end()] {
                            if identities==limits.max_identity_queries {return Err(unproved("Chord-boundary endpoint identity budget exhausted"));}identities+=1;
                            if entry.point().same_source_point_in_charts(endpoint.point(),limits.clock,limits.geometry)? {
                                let port=ss::bind_original_ss_port(endpoint.point().clone(),edge.curve().clone(),[0;4],limits.geometry)?;
                                qualify_root_uv_joint(&port,c.pcurve.support,endpoint,split.support())?;
                                allowed=true;break;
                            }
                        }
                        if !allowed {return Err(unproved("Additional or unresolved original face-boundary contact with chord surface"));}
                    }
                    censuses.push(census);
                }
            }
        }
        Ok(Self {split,limits,censuses,entries,identities})
    }
    pub fn split(&self)->&RootFaceChordSplit {&self.split}
    pub fn censuses(&self)->&[Arc<ss::QualifiedSsThirdSurfaceBranchCensus>] {&self.censuses}
    pub fn entries(&self)->usize {self.entries}
    pub fn identity_queries(&self)->usize {self.identities}
    pub fn reverify(&self)->Result<()> {
        self.split.reverify()?;let fresh=Self::qualify(self.split.clone(),self.limits)?;
        if self.censuses.len()!=fresh.censuses.len() || self.entries!=fresh.entries || self.identities!=fresh.identities {
            return Err(unproved("Chord-boundary census coverage changed"));
        }Ok(())
    }
}
