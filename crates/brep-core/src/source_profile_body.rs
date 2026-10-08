//! Boundary-preserving exact weight-profile proposal on an immutable original Body.
//! Closed topology is inherited only after every retained contour is checked;
//! embedding, normals, volume and wall qualification must be rebuilt separately.
use crate::source_volume::Body;
use nurbs_core::{Error, Result, normal_alignment::WeightProfile, surface::Surface};
#[derive(Clone, Copy)]
pub struct FaceSpec {
    pub face: usize,
    pub anchor: [usize; 2],
}
pub struct Limits {
    pub controls: usize,
    pub fragments: usize,
    pub position_tolerance: f64,
}
struct Face<'a> {
    spec: FaceSpec,
    profile: WeightProfile<'a>,
}
pub struct Proposal<'a> {
    body: &'a Body,
    faces: Vec<Face<'a>>,
    axis_deviation_upper: [f64; 3],
    position_tolerance: f64,
}
/// Tangent-plane certificate attached to this exact proposal and an owned
/// original shared carrier. It does not admit volume or shell embedding.
pub struct Seam<'p, 'b> {
    proposal: &'p Proposal<'b>,
    faces: [usize; 2],
    edge: usize,
    threshold: f64,
    upper: f64,
}
impl Seam<'_, '_> {
    pub fn proposal(&self) -> &Proposal<'_> {
        self.proposal
    }
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn threshold(&self) -> f64 {
        self.threshold
    }
    pub fn upper_sine_squared(&self) -> f64 {
        self.upper
    }
}
pub struct SeamReport<'p, 'b> {
    pub seam: Option<Seam<'p, 'b>>,
    pub angular: Option<nurbs_core::normal_alignment::ProfilePathReport>,
    pub reason: &'static str,
}
/// Tangent-plane qualification for exactly one canonical carrier interval.
/// Does not qualify the rest of the original edge or admit a sewn Body.
pub struct RestrictedSeam<'p,'b> {
    proposal:&'p Proposal<'b>, faces:[usize;2], edge:usize,
    canonical:[f64;2], threshold:f64, upper:f64,
}
impl RestrictedSeam<'_,'_> {
    pub fn proposal(&self)->&Proposal<'_> {self.proposal}
    pub fn faces(&self)->[usize;2] {self.faces}
    pub fn edge(&self)->usize {self.edge}
    pub fn canonical_interval(&self)->[f64;2] {self.canonical}
    pub fn threshold(&self)->f64 {self.threshold}
    pub fn upper_sine_squared(&self)->f64 {self.upper}
}
pub struct RestrictedSeamReport<'p,'b> {
    pub seam:Option<RestrictedSeam<'p,'b>>,
    pub angular:nurbs_core::normal_alignment::ProfilePathReport,
}
/// Restricted seam retaining both freshly admitted original carrier roots.
pub struct RootEndedSeam<'p,'b> {
    restricted:RestrictedSeam<'p,'b>, points:[crate::source_carrier_point::Point;2],
    face:usize,axis:usize,parameters:[f64;2],high:bool,
}
impl<'p,'b> RootEndedSeam<'p,'b> {
    pub fn restricted(&self)->&RestrictedSeam<'_,'_> {&self.restricted}
    pub fn points(&self)->&[crate::source_carrier_point::Point;2] {&self.points}
    /// Bind only the full Hermite-corrected expression with this exact recipe.
    pub fn qualify_transition_side<'s>(
        &'s self,bridge:&nurbs_core::normal_alignment::RailConstrainedBridge<'_, '_, '_, '_, '_>,cells:usize,spans:usize,
    )->Result<TransitionSideReport<'s,'p,'b>> {
        let mut out=TransitionSideReport{side:None,regularity:None,reason:"profile-transition-side-recipe-unproven"};
        let owned=self.restricted.proposal.profile(self.face).unwrap();
        for high in [false,true] {
            let section=bridge.section(high);
            if !std::ptr::eq(section.profile(),owned) || section.axis()!=self.axis
                || section.parameter()!=self.parameters[usize::from(high)] {return Ok(out);}
        }
        let report=bridge.inspect_transverse_corrected_side_join(self.high,[0.,1.],cells,spans)?;
        out.reason=report.reason;
        if report.complete_regular {out.side=Some(TransitionSide{rooted:self});out.reason="profile-transition-side-qualified";}
        out.regularity=Some(report);Ok(out)
    }

    /// Retain exactly the original interval between both roots on each use.
    /// Traversal follows the original incident use, preserving opposite sides.
    pub fn incident_fragments(&self)->Result<[crate::source_boundary_fragment::Fragment;2]> {
        use crate::source_boundary_fragment::{Fragment,Endpoint};
        let mut fragments=Vec::new();
        for side in 0..2 {
            let source=&self.points[0].edge().uses()[side];
            let ranges=[self.points[0].parameter_bounds()[side],self.points[1].parameter_bounds()[side]];
            let order=if ranges[0][1]<ranges[1][0] {[0,1]}else if ranges[1][1]<ranges[0][0] {[1,0]}else {
                return Err(Error::new("BREP_PROFILE_SEAM","Require strictly ordered carrier ends"));
            };
            let order=if source.reversed() {[order[1],order[0]]}else{order};
            fragments.push(Fragment::new(source.surface(),source.curve(),
                Endpoint::Carrier{point:Box::new(self.points[order[0]].clone()),side},
                Endpoint::Carrier{point:Box::new(self.points[order[1]].clone()),side})?);
        }
        Ok([fragments.remove(0),fragments.remove(0)])
    }
    /// Split both original incident contours at the same carrier roots.
    /// The middle use is ready for sewing to the exact transition rail.
    pub fn propose_incident_contours(&self,max_fragments:usize)->Result<IncidentContourReport> {
        use crate::source_boundary_fragment::Fragment;
        if max_fragments>100000 {return Err(Error::new("BREP_PROFILE_CONTOUR","Bound incident contour work"));}
        let shell=self.restricted.proposal.body.geometry().shell();
        let addresses=shell.uses()[self.restricted.edge];
        let regions=shell.regions().unwrap();
        let cost:usize=addresses.iter().map(|a|regions[a.face].loops().iter().map(Vec::len).sum::<usize>()+2).sum();
        let mut out=IncidentContourReport{proposal:None,fragments:0,reason:"profile-incident-contour-work-limit"};
        if cost>max_fragments {return Ok(out);}
        let middle=self.incident_fragments()?;
        let mut contours=Vec::new();
        for side in 0..2 {
            let address=addresses[side];
            let mut loops=regions[address.face].loops().to_vec();
            let source=&loops[address.wire][address.edge];
            let owned=&self.points[0].edge().uses()[side];
            if source.definition()!=owned.definition() {
                out.reason="profile-incident-contour-source-unproven";return Ok(out);
            }
            let head=Fragment::new(source.surface(),source.curve(),source.endpoints()[0].clone(),middle[side].endpoints()[0].clone())?;
            let tail=Fragment::new(source.surface(),source.curve(),middle[side].endpoints()[1].clone(),source.endpoints()[1].clone())?;
            loops[address.wire].splice(address.edge..=address.edge,[head,middle[side].clone(),tail]);
            if loops.iter().any(|wire|(0..wire.len()).any(|i|!wire[i].joins(&wire[(i+1)%wire.len()]))) {
                out.reason="profile-incident-contour-joins-unproven";return Ok(out);
            }
            contours.push(IncidentContour{face:address.face,loops,middle:[address.wire,address.edge+1]});
        }
        out.fragments=cost;
        out.proposal=Some(IncidentContours{contours:[contours.remove(0),contours.remove(0)]});
        out.reason="profile-incident-contours-source-joins-qualified";Ok(out)
    }
    /// Rebuild shared-edge authority over both retained original uses.
    pub fn qualify_incident_edge(&self,max_work:u64,max_driver:usize)->Result<crate::source_shared_edge::Report> {
        let fragments=self.incident_fragments()?;
        let original=self.points[0].edge();
        crate::source_mapped_edge::qualify(original.world(),[&fragments[0],&fragments[1]],
            crate::source_seam_tangency::ranges(original),[None,None],max_work,max_driver)
    }


}
/// Original face contours subdivided at exact shared roots, preserving holes.
pub struct IncidentContour {face:usize,loops:Vec<Vec<crate::source_boundary_fragment::Fragment>>,middle:[usize;2]}
impl IncidentContour {
    pub fn face(&self)->usize {self.face}
    pub fn loops(&self)->&[Vec<crate::source_boundary_fragment::Fragment>] {&self.loops}
    pub fn middle(&self)->[usize;2] {self.middle}
}
pub struct IncidentContours {contours:[IncidentContour;2]}
impl IncidentContours {pub fn contours(&self)->&[IncidentContour;2] {&self.contours}}
pub struct IncidentContourReport {pub proposal:Option<IncidentContours>,pub fragments:usize,pub reason:&'static str}
/// Exact full-Hermite side recipe bound to the retained root-ended B-rep seam.
/// Does not admit a closed transition face or certify the bridge interior.
pub struct TransitionSide<'s,'p,'b> {rooted:&'s RootEndedSeam<'p,'b>}
impl TransitionSide<'_,'_,'_> {
    pub fn rooted(&self)->&RootEndedSeam<'_,'_> {self.rooted}
    pub fn face(&self)->usize {self.rooted.face}
    pub fn axis(&self)->usize {self.rooted.axis}
    pub fn parameters(&self)->[f64;2] {self.rooted.parameters}
    pub fn high_free_end(&self)->bool {self.rooted.high}
}
pub struct TransitionSideReport<'s,'p,'b> {
    pub side:Option<TransitionSide<'s,'p,'b>>,
    pub regularity:Option<nurbs_core::normal_alignment::TransitionRegularityReport>,
    pub reason:&'static str,
}
/// Four exact boundary expression views with root-owned corners. This is a
/// contour proposal; it does not admit a transition face or a closed Body.
pub struct TransitionBoundary<'s,'p,'b> {rooted:[&'s RootEndedSeam<'p,'b>;2]}
impl<'s,'p,'b> TransitionBoundary<'s,'p,'b> {
    /// Bind a terminal/entry section cut to this exact transition's carrier roots.
    pub fn propose_section_contours<'a>(&'a self,high:bool,parts:[&CutPartition<'_, 'p,'b>;2],max_fragments:usize)->Result<TransitionSectionReport<'a,'s,'p,'b>> {
        let mut out=TransitionSectionReport{section:None,fragments:0,reason:"profile-transition-section-ownership-unproven"};
        let index=usize::from(high);let recipe=self.rooted[0];
        for rail in 0..2 {
            let part=parts[rail];let endpoint=part.endpoint;
            if !std::ptr::eq(endpoint.proposal,recipe.restricted.proposal) || endpoint.face!=recipe.face
                || endpoint.axis!=recipe.axis || endpoint.parameter!=recipe.parameters[index]
                || endpoint.high!=(rail==1) || endpoint.edge!=self.rooted[rail].restricted.edge
                || part.point.definition()!=self.rooted[rail].points[index].root().definition() {return Ok(out);}
        }
        let contours=propose_cut_contours(parts[0],parts[1],max_fragments)?;
        out.fragments=contours.fragments;out.reason=contours.reason;
        let Some(contours)=contours.proposal else {return Ok(out);};
        out.section=Some(TransitionSection{boundary:self,high,contours});
        out.reason="profile-transition-section-contours-bound";Ok(out)
    }
    /// Fresh whole-domain injectivity also proves immersion. Authority remains
    /// attached to this boundary and its exact full-Hermite recipe.
    pub fn qualify_face<'a>(&'a self,projection:[[f64;3];2],controls:usize,cells:usize,spans:usize)->Result<TransitionFaceReport<'a,'s,'p,'b>> {
        let root=self.rooted[0];
        let profile=root.restricted.proposal.profile(root.face).unwrap();
        let sections=[profile.section(root.axis,root.parameters[0])?,profile.section(root.axis,root.parameters[1])?];
        let range=sections[0].range();
        let width=root.parameters[1]-root.parameters[0];
        let [first,last]=sections;
        let bridge=nurbs_core::normal_alignment::ProfileSectionBridge::new(first,last,[width;2])?;
        let (bridge,_)=bridge.constrain_original_rails(controls)?;
        let Some(bridge)=bridge else {return Ok(TransitionFaceReport{face:None,interior:None,reason:"profile-transition-face-recipe-unproven"});};
        let interior=bridge.inspect_transverse_corrected_injectivity(range,[0.,1.],projection,cells,spans)?;
        let face=if interior.complete_regular {Some(TransitionFace{boundary:self,projection})}else{None};
        let reason=interior.reason;
        Ok(TransitionFaceReport{face,interior:Some(interior),reason})
    }
    pub fn corners(&self)->[&crate::source_carrier_point::Point;4] {
        [&self.rooted[0].points[0],&self.rooted[0].points[1],&self.rooted[1].points[1],&self.rooted[1].points[0]]
    }
    pub fn sides(&self)->[&RootEndedSeam<'_,'_>;2] {self.rooted}
    /// Low rail, terminal section, reversed high rail, reversed entry section.
    /// Each segment uses the exact owned profile boundary specialization.
    pub fn enclose_edge(&self,index:usize,parameter:[f64;2],spans:usize)->Result<(Option<[[f64;2];3]>,usize)> {
        use nurbs_core::interval_eval::Interval as I;
        if index>3 || !parameter.iter().all(|x|x.is_finite()) || parameter[0]<0. || parameter[1]>1. || parameter[0]>parameter[1] {
            return Err(Error::new("BREP_PROFILE_TRANSITION_BOUNDARY","Bound edge index and normalized parameter"));
        }
        let root=self.rooted[0];let profile=root.restricted.proposal.profile(root.face).unwrap();
        let range=profile.section(root.axis,root.parameters[0])?.range();
        let affine=|ends:[f64;2]|->Result<[f64;2]> {
            if parameter==[0.,0.] {return Ok([ends[0];2]);}
            if parameter==[1.,1.] {return Ok([ends[1];2]);}
            let value=I::point(ends[0]).add(I::new(parameter[0],parameter[1])?.mul(I::point(ends[1]).sub(I::point(ends[0]))?)?)?
                .intersect(ends[0].min(ends[1]),ends[0].max(ends[1]))?;
            Ok([value.lo,value.hi])
        };
        let mut domain=[[0.;2];2];
        match index {
            0=>{domain[root.axis]=affine(root.parameters)?;domain[1-root.axis]=[range[0];2];},
            1=>{domain[root.axis]=[root.parameters[1];2];domain[1-root.axis]=affine(range)?;},
            2=>{domain[root.axis]=affine([root.parameters[1],root.parameters[0]])?;domain[1-root.axis]=[range[1];2];},
            _=>{domain[root.axis]=[root.parameters[0];2];domain[1-root.axis]=affine([range[1],range[0]])?;},
        }
        profile.enclose(domain,spans)
    }
}
/// Two complementary original source-chart contours at one owned section.
/// Selection of the retained region and profile-face admission remain separate.
pub struct TransitionSection<'a,'s,'p,'b> {boundary:&'a TransitionBoundary<'s,'p,'b>,high:bool,contours:CutContourProposal}
impl<'a,'s,'p,'b> TransitionSection<'a,'s,'p,'b> {
    /// Whole-source convex hulls and monotonic implicit-root side proofs.
    pub fn qualify_exterior<'r>(&'r self,max_controls:usize,max_driver_cells:usize)->Result<ExteriorSectionReport<'r,'a,'s,'p,'b>> {
        use crate::source_boundary_fragment::{Endpoint,Role};
        if max_controls>100000 || max_driver_cells>100000 {return Err(Error::new("BREP_PROFILE_SECTION_REGION","Bound exterior section work"));}
        let mut out=ExteriorSectionReport{region:None,controls:0,driver_cells:0,reason:"profile-section-exterior-side-unproven"};
        let recipe=self.boundary.rooted[0];let axis=recipe.axis;let parameter=recipe.parameters[usize::from(self.high)];
        let mut sides=Vec::new();
        for wire in &self.contours.loops {
            let contact=wire.last().unwrap().curve();
            if contact.degree!=1 || contact.control_points.len()!=2 || contact.control_points.iter().any(|p|p[axis]!=parameter) {return Ok(out);}
            let a=&contact.control_points[0];let b=&contact.control_points[1];
            if a[1-axis]==b[1-axis] {return Ok(out);}
            let coefficient_positive=if axis==1 {b[0]>a[0]}else{b[1]<a[1]};
            let mut admitted=None;
            for above in [false,true] {
                let mut proven=true;
                for fragment in &wire[..wire.len()-1] {
                    let crossing=fragment.endpoints().iter().any(|e|matches!(e,Endpoint::Crossing{point,role:Role::Boundary} if point.contact()==contact));
                    if crossing {
                        let report=crate::source_halfplane_side::prove(fragment,contact,1,above==coefficient_positive,max_driver_cells-out.driver_cells)?;
                        out.driver_cells+=report.driver_cells;
                        if !report.proven {proven=false;break;}
                    } else {
                        for p in &fragment.curve().control_points {
                            if out.controls==max_controls {out.reason="profile-section-exterior-work-limit";return Ok(out);}
                            out.controls+=1;
                            if !(if above {p[axis]>parameter}else{p[axis]<parameter}) {proven=false;break;}
                        }
                        if !proven {break;}
                    }
                }
                if proven {admitted=Some(above);break;}
            }
            let Some(side)=admitted else {return Ok(out);};sides.push(side);
        }
        if sides[0]==sides[1] {return Ok(out);}
        let kept=usize::from(sides[1]==self.high);
        out.region=Some(ExteriorSection{section:self,kept});
        out.reason="profile-section-exterior-side-qualified";Ok(out)
    }
    pub fn boundary(&self)->&TransitionBoundary<'_,'_,'_> {self.boundary}
    pub fn edge_index(&self)->usize {if self.high {1}else{3}}
    pub fn contours(&self)->&CutContourProposal {&self.contours}
}
/// Both complementary arcs classified, with the exterior side selected.
pub struct ExteriorSection<'r,'a,'s,'p,'b> {section:&'r TransitionSection<'a,'s,'p,'b>,kept:usize}
impl ExteriorSection<'_,'_,'_,'_,'_> {
    /// Inspect a rectangle containing the complete qualified exterior region.
    /// No original-surface immersion certificate is inherited by the profile.
    pub fn inspect_regularity(&self,cells:usize,spans:usize)->Result<nurbs_core::normal_alignment::TransitionRegularityReport> {
        let (profile,domain)=self.profile_domain();profile.inspect_regularity(domain,cells,spans)
    }
    pub fn inspect_factored_pole_regularity(&self,high:bool,cells:usize,spans:usize)->Result<nurbs_core::normal_alignment::ProfilePoleCover> {
        let (profile,domain)=self.profile_domain();profile.inspect_factored_pole_regularity(domain,high,cells,spans)
    }
    fn profile_domain(&self)->(&WeightProfile<'_>,[[f64;2];2]) {
        let recipe=self.section.boundary.rooted[0];
        let profile=recipe.restricted.proposal.profile(recipe.face).unwrap();let source=profile.source();
        let mut domain=[[source.knots_u[source.degree_u],source.knots_u[source.control_points.len()]],
            [source.knots_v[source.degree_v],source.knots_v[source.control_points[0].len()]]];
        domain[recipe.axis][usize::from(!self.section.high)]=recipe.parameters[usize::from(self.section.high)];
        (profile,domain)
    }
    pub fn section(&self)->&TransitionSection<'_,'_,'_,'_> {self.section}
    pub fn kept_loop(&self)->&[crate::source_boundary_fragment::Fragment] {&self.section.contours.loops[self.kept]}
    pub fn removed_loop(&self)->&[crate::source_boundary_fragment::Fragment] {&self.section.contours.loops[1-self.kept]}
}
pub struct ExteriorSectionReport<'r,'a,'s,'p,'b> {pub region:Option<ExteriorSection<'r,'a,'s,'p,'b>>,pub controls:usize,pub driver_cells:usize,pub reason:&'static str}
pub struct TransitionSectionReport<'a,'s,'p,'b> {pub section:Option<TransitionSection<'a,'s,'p,'b>>,pub fragments:usize,pub reason:&'static str}
/// Admitted embedded transition expression with its root-owned contour.
/// This does not establish closure or embedding of the surrounding Body.
pub struct TransitionFace<'a,'s,'p,'b> {boundary:&'a TransitionBoundary<'s,'p,'b>,projection:[[f64;3];2]}
impl<'a,'s,'p,'b> TransitionFace<'a,'s,'p,'b> {
    /// Fresh retained-edge admission followed by opposite directed incidence.
    pub fn qualify_rail_incidence<'f>(&'f self,work_per_rail:u64,drivers_per_rail:usize,fragments_per_rail:usize)->Result<TransitionRailReport<'f,'a,'s,'p,'b>> {
        let mut out=TransitionRailReport{incidence:None,work:0,reason:"profile-transition-rail-incidence-unproven"};
        let mut rails=Vec::new();let mut orientation=None;
        for (rail,root) in self.boundary.rooted.iter().enumerate() {
            let report=root.qualify_incident_edge(work_per_rail,drivers_per_rail)?;
            out.work+=report.work_used;out.reason=report.reason;
            let Some(edge)=report.edge else {return Ok(out);};
            let contours=root.propose_incident_contours(fragments_per_rail)?;out.reason=contours.reason;
            let Some(contours)=contours.proposal else {return Ok(out);};
            let selected=root.points[0].source_side();let neighbor=1-selected;
            let ranges=root.points.each_ref().map(|p|p.canonical_bounds());
            let forward=if ranges[0][1]<ranges[1][0] {true}else if ranges[1][1]<ranges[0][0] {false}else {return Ok(out);};
            let transition_reversed=(!forward)^(rail==1);
            let reverse=transition_reversed==edge.reversed()[neighbor];
            if orientation.is_some_and(|value|value!=reverse) {out.reason="profile-transition-rail-orientation-conflict";return Ok(out);}
            orientation=Some(reverse);
            let neighbor_contour=&contours.contours[neighbor];
            let [wire,index]=neighbor_contour.middle;
            if neighbor_contour.loops[wire][index].definition()!=edge.uses()[neighbor].definition() {return Ok(out);}
            rails.push(TransitionRail{edge,neighbor_contour:contours.contours.into_iter().nth(neighbor).unwrap(),neighbor,transition_reversed});
        }
        out.incidence=Some(TransitionRailIncidence{face:self,rails:[rails.remove(0),rails.remove(0)],reverse_face:orientation.unwrap()});
        out.reason="profile-transition-rail-incidence-qualified";Ok(out)
    }
    pub fn boundary(&self)->&TransitionBoundary<'_,'_,'_> {self.boundary}
    pub fn projection(&self)->[[f64;3];2] {self.projection}
}
pub struct TransitionRail {edge:crate::source_shared_edge::SharedEdge,neighbor_contour:IncidentContour,neighbor:usize,transition_reversed:bool}
impl TransitionRail {
    pub fn edge(&self)->&crate::source_shared_edge::SharedEdge {&self.edge}
    pub fn neighbor_contour(&self)->&IncidentContour {&self.neighbor_contour}
    pub fn neighbor_side(&self)->usize {self.neighbor}
    pub fn transition_reversed(&self)->bool {self.transition_reversed}
}
/// Two sewn rail incidences; entry and terminal edges remain open.
pub struct TransitionRailIncidence<'f,'a,'s,'p,'b> {face:&'f TransitionFace<'a,'s,'p,'b>,rails:[TransitionRail;2],reverse_face:bool}
impl<'f,'a,'s,'p,'b> TransitionRailIncidence<'f,'a,'s,'p,'b> {
    pub fn qualify_sections<'i>(&'i self,sections:[&ExteriorSection<'_, '_, 's,'p,'b>;2])->Result<TransitionPatchReport<'i,'f,'a,'s,'p,'b>> {
        use crate::source_boundary_fragment::{Endpoint,Role};
        let mut out=TransitionPatchReport{patch:None,reason:"profile-transition-section-incidence-owner-unproven"};
        if sections[0].section.high==sections[1].section.high || sections.iter().any(|r|!std::ptr::eq(r.section.boundary,self.face.boundary)) {return Ok(out);}
        let sections=if sections[0].section.high {[sections[1],sections[0]]}else{sections};
        let mut kept=Vec::new();
        for (index,region) in sections.iter().enumerate() {
            let wire=region.kept_loop();let contact=wire.last().unwrap();
            let axis=self.face.boundary.rooted[0].axis;
            let descending=contact.curve().control_points[1][1-axis]<contact.curve().control_points[0][1-axis];
            let kept_descending=descending^contact.reversed();
            let transition_descending=(index==0)^self.reverse_face;
            if kept_descending==transition_descending {out.reason="profile-transition-section-incidence-orientation-conflict";return Ok(out);}
            for end in 0..2 {
                let rail=if kept_descending {1-end}else{end};
                let expected=self.face.boundary.rooted[rail].points[index].root().definition();
                if !matches!(&contact.endpoints()[end],Endpoint::Crossing{point,role:Role::Contact} if point.definition()==expected) {
                    out.reason="profile-transition-section-incidence-root-unproven";return Ok(out);
                }
            }
            kept.push(wire.to_vec());
        }
        out.patch=Some(TransitionPatchIncidence{rails:self,retained:[kept.remove(0),kept.remove(0)]});
        out.reason="profile-transition-four-edge-incidence-qualified";Ok(out)
    }
    pub fn face(&self)->&TransitionFace<'_,'_,'_,'_> {self.face}
    pub fn rails(&self)->&[TransitionRail;2] {&self.rails}
    pub fn reverse_face(&self)->bool {self.reverse_face}
}
/// All four patch edges have opposite neighboring uses. Surrounding shell
/// admission, retained-profile immersion and global embedding remain separate.
pub struct TransitionPatchIncidence<'i,'f,'a,'s,'p,'b> {rails:&'i TransitionRailIncidence<'f,'a,'s,'p,'b>,retained:[Vec<crate::source_boundary_fragment::Fragment>;2]}
impl TransitionPatchIncidence<'_,'_,'_,'_,'_,'_> {
    pub fn rails(&self)->&TransitionRailIncidence<'_,'_,'_,'_,'_> {self.rails}
    pub fn retained(&self)->&[Vec<crate::source_boundary_fragment::Fragment>;2] {&self.retained}
}
pub struct TransitionPatchReport<'i,'f,'a,'s,'p,'b> {pub patch:Option<TransitionPatchIncidence<'i,'f,'a,'s,'p,'b>>,pub reason:&'static str}
pub struct TransitionRailReport<'f,'a,'s,'p,'b> {pub incidence:Option<TransitionRailIncidence<'f,'a,'s,'p,'b>>,pub work:u64,pub reason:&'static str}
pub struct TransitionFaceReport<'a,'s,'p,'b> {
    pub face:Option<TransitionFace<'a,'s,'p,'b>>,
    pub interior:Option<nurbs_core::normal_alignment::TransitionRegularityReport>,
    pub reason:&'static str,
}
pub fn propose_transition_boundary<'s,'p,'b>(
    sides:[&TransitionSide<'s,'p,'b>;2],
)->Result<TransitionBoundary<'s,'p,'b>> {
    let a=sides[0].rooted;let b=sides[1].rooted;
    if !std::ptr::eq(a.restricted.proposal,b.restricted.proposal) || a.face!=b.face
        || a.axis!=b.axis || a.parameters!=b.parameters || a.high==b.high {
        return Err(Error::new("BREP_PROFILE_TRANSITION_BOUNDARY","Require opposite sides of one owned transition recipe"));
    }
    Ok(TransitionBoundary{rooted:if a.high {[b,a]}else{[a,b]}})
}
pub struct RootEndedSeamReport<'p,'b> {
    pub seam:Option<RootEndedSeam<'p,'b>>,pub carrier_spans:usize,
    pub angular:Option<nurbs_core::normal_alignment::ProfilePathReport>,pub reason:&'static str,
}
/// An owned new section end with an exact source-parameter recipe. Interval
/// coordinates enclose that recipe; they are not rounded replacement vertices.
pub struct CutEndpoint<'p, 'b> {
    proposal: &'p Proposal<'b>,
    face: usize,
    neighbor: usize,
    edge: usize,
    axis: usize,
    parameter: f64,
    high: bool,
    canonical: [f64; 2],
    neighbor_uv: [[f64; 2]; 2],
    world: [[f64; 2]; 3],
}
/// Two unchanged selected-face fragments sharing one certified crossing root.
/// Neighbor ranges are transported through the same private canonical carrier;
/// they remain expression views until neighbor contour admission is rebuilt.
pub struct CutPartition<'c, 'p, 'b> {
    endpoint: &'c CutEndpoint<'p, 'b>,
    point: crate::source_contact_point::SourcePoint,
    selected: [crate::source_boundary_fragment::Fragment; 2],
    neighbor_ranges: [[[f64; 2]; 2]; 2],
}
/// Freshly admitted original shared-edge halves, retaining both root definitions.
/// This is not a closed replacement contour or a new corrected Body.
pub struct NeighborPartition {
    fragments: [crate::source_boundary_fragment::Fragment; 2],
    edges: [crate::source_shared_edge::SharedEdge; 2],
}
impl NeighborPartition {
    pub fn fragments(&self) -> &[crate::source_boundary_fragment::Fragment; 2] {
        &self.fragments
    }
    pub fn edges(&self) -> &[crate::source_shared_edge::SharedEdge; 2] {
        &self.edges
    }
}
pub struct NeighborPartitionReport {
    pub carrier_spans: usize,
    pub partition: Option<NeighborPartition>,
    pub mapping_cells: usize,
    pub exact_work: u64,
    pub root_checks: usize,
    pub driver_cells: usize,
    pub reason: &'static str,
}
impl CutPartition<'_, '_, '_> {
    /// Transport the existing selected root through its admitted original
    /// carrier, then freshly prove both restricted compositions and endpoints.
    /// No independently rounded neighbor cutter or root parameter is authored.
    pub fn qualify_transported_neighbor(&self, max_spans: usize, max_work: u64,
        max_driver: usize) -> Result<NeighborPartitionReport> {
        if !(1..=100000000).contains(&max_work) || max_driver > 100000 {
            return Err(Error::new("BREP_PROFILE_CUT", "Bound transported neighbor work"));
        }
        let shell = self.endpoint.proposal.body.geometry().shell();
        let edge = &shell.edges()[self.endpoint.edge];
        let addresses = &shell.uses()[self.endpoint.edge];
        let side = usize::from(addresses[0].face != self.endpoint.face);
        let transport = crate::source_carrier_point::qualify(edge,&self.point,side,max_spans)?;
        let mut out = NeighborPartitionReport { partition:None, carrier_spans:transport.spans,
            mapping_cells:0, exact_work:0, root_checks:0, driver_cells:0, reason:transport.reason };
        let Some(point) = transport.point else { return Ok(out); };
        let fragments = edge.uses()[1-side].split_at_carrier(&point,1-side)?;
        let ranges = crate::source_seam_tangency::ranges(edge);
        let mut halves = Vec::new();
        for piece in 0..2 {
            if out.exact_work == max_work {
                out.reason = "profile-cut-neighbor-work-limit"; return Ok(out);
            }
            let checked = crate::source_mapped_edge::qualify(edge.world(),
                [&self.selected[piece],&fragments[1-piece]], [ranges[side],ranges[1-side]],
                [None,None], max_work-out.exact_work, max_driver-out.driver_cells)?;
            out.exact_work += checked.work_used;
            out.root_checks += checked.root_checks;
            out.driver_cells += checked.driver_cells;
            out.reason = checked.reason;
            let Some(half) = checked.edge else { return Ok(out); };
            halves.push(half);
        }
        out.partition = Some(NeighborPartition {fragments,edges:[halves.remove(0),halves.remove(0)]});
        out.reason = "profile-cut-transported-neighbor-qualified";
        Ok(out)
    }

    /// Treat the neighbor cutter as a proposal: requalify its root and both
    /// restricted world-edge halves. Parameter enclosure overlap is not identity.
    pub fn qualify_neighbor(
        &self,
        contact: &nurbs_core::curve::Curve,
        selector: [[f64; 2]; 2],
        mapping_cells: usize,
        max_work: u64,
        max_driver: usize,
    ) -> Result<NeighborPartitionReport> {
        if !(1..=100000000).contains(&max_work) || max_driver > 100000 {
            return Err(Error::new(
                "BREP_PROFILE_CUT",
                "Bound fresh neighbor edge work",
            ));
        }
        let shell = self.endpoint.proposal.body.geometry().shell();
        let index = self.endpoint.edge;
        let edge = &shell.edges()[index];
        let addresses = &shell.uses()[index];
        let side = usize::from(addresses[0].face != self.endpoint.face);
        let other = &edge.uses()[1 - side];
        let root = crate::source_contact_point::qualify(
            other.surface(),
            other.curve(),
            contact,
            selector,
            mapping_cells,
        )?;
        let mut out = NeighborPartitionReport {
            partition: None,
            carrier_spans: 0,
            mapping_cells: root.mapping_cells,
            exact_work: 0,
            root_checks: 0,
            driver_cells: 0,
            reason: root.reason,
        };
        let Some(point) = root.point else {
            return Ok(out);
        };
        let fragments = other.split_at(&point, crate::source_boundary_fragment::Role::Boundary)?;
        let ranges = crate::source_seam_tangency::ranges(edge);
        let mut halves = Vec::new();
        for piece in 0..2 {
            if out.exact_work == max_work {
                out.reason = "profile-cut-neighbor-work-limit";
                return Ok(out);
            }
            let checked = crate::source_mapped_edge::qualify(
                edge.world(),
                [&self.selected[piece], &fragments[1 - piece]],
                [ranges[side], ranges[1 - side]],
                [None, None],
                max_work - out.exact_work,
                max_driver - out.driver_cells,
            )?;
            out.exact_work += checked.work_used;
            out.root_checks += checked.root_checks;
            out.driver_cells += checked.driver_cells;
            out.reason = checked.reason;
            let Some(half) = checked.edge else {
                return Ok(out);
            };
            halves.push(half);
        }
        out.partition = Some(NeighborPartition {
            fragments,
            edges: [halves.remove(0), halves.remove(0)],
        });
        out.reason = "profile-cut-neighbor-partition-qualified";
        Ok(out)
    }
    pub fn endpoint(&self) -> &CutEndpoint<'_, '_> {
        self.endpoint
    }
    pub fn point(&self) -> &crate::source_contact_point::SourcePoint {
        &self.point
    }
    pub fn selected_fragments(&self) -> &[crate::source_boundary_fragment::Fragment; 2] {
        &self.selected
    }
    pub fn neighbor_ranges(&self) -> [[[f64; 2]; 2]; 2] {
        self.neighbor_ranges
    }
}
/// Two source-owned closed contour candidates separated by one original UV
/// section. Closure alone does not qualify region containment or a corrected face.
pub struct CutContourProposal {
    loops: [Vec<crate::source_boundary_fragment::Fragment>; 2],
}
impl CutContourProposal {
    pub fn loops(&self) -> &[Vec<crate::source_boundary_fragment::Fragment>; 2] { &self.loops }
}
pub struct CutContourReport {
    pub proposal: Option<CutContourProposal>,
    pub fragments: usize,
    pub reason: &'static str,
}
pub fn propose_cut_contours(a: &CutPartition<'_, '_, '_>, b: &CutPartition<'_, '_, '_>,
    max_fragments: usize) -> Result<CutContourReport> {
    use crate::source_boundary_fragment::{Endpoint, Fragment, Role};
    if max_fragments > 100000 {
        return Err(Error::new("BREP_PROFILE_CUT", "Bound cut contour fragments"));
    }
    let mut out=CutContourReport { proposal:None,fragments:0,reason:"profile-cut-contour-source-unproven" };
    if !std::ptr::eq(a.endpoint.proposal,b.endpoint.proposal)
        || a.endpoint.face != b.endpoint.face || a.endpoint.axis != b.endpoint.axis
        || a.endpoint.parameter.to_bits() != b.endpoint.parameter.to_bits()
        || a.endpoint.high == b.endpoint.high || a.point.contact() != b.point.contact()
        || a.point.surface() != b.point.surface() { return Ok(out); }
    let regions=a.endpoint.proposal.body.geometry().shell().regions().unwrap();
    let region=&regions[a.endpoint.face];
    if region.loops().len()!=1 { out.reason="profile-cut-contour-holes-unclassified";return Ok(out); }
    let wire=&region.loops()[0];
    let locate=|p:&CutPartition<'_, '_, '_>| {
        let matches=wire.iter().enumerate().filter(|(_,f)|
        f.surface()==p.point.surface() && f.curve()==p.point.boundary()
            && f.endpoints().iter().zip(p.selected[0].endpoints().iter().take(1)
                .chain(p.selected[1].endpoints().iter().skip(1))).all(|(x,y)| {
                    // Original retained endpoints must agree as definitions.
                    match (x,y) { (Endpoint::Parameter(x),Endpoint::Parameter(y))=>x.to_bits()==y.to_bits(), _=>false }
                })).map(|(i,_)|i).collect::<Vec<_>>();
        if matches.len()==1 {Some(matches[0])} else {None}
    };
    let (Some(ia),Some(ib))=(locate(a),locate(b)) else {return Ok(out);};
    if ia==ib {return Ok(out);}
    let cost=wire.len()+4;
    if cost>max_fragments {out.reason="profile-cut-contour-work-limit";return Ok(out);}
    let bridge=|start:&CutPartition<'_, '_, '_>,end:&CutPartition<'_, '_, '_>| {
        Fragment::new(start.point.surface(),start.point.contact(),
            Endpoint::Crossing {point:start.point.clone(),role:Role::Contact},
            Endpoint::Crossing {point:end.point.clone(),role:Role::Contact})
    };
    let arc=|start:usize,end:usize,head:&CutPartition<'_, '_, '_>,tail:&CutPartition<'_, '_, '_>| -> Result<Vec<Fragment>> {
        let mut fragments=vec![head.selected[1].clone()];
        let mut index=(start+1)%wire.len();
        while index!=end {fragments.push(wire[index].clone());index=(index+1)%wire.len();}
        fragments.push(tail.selected[0].clone());
        fragments.push(bridge(tail,head)?);
        Ok(fragments)
    };
    let loops=[arc(ia,ib,a,b)?,arc(ib,ia,b,a)?];
    out.fragments=cost;
    if loops.iter().any(|wire| (0..wire.len()).any(|i| !wire[i].joins(&wire[(i+1)%wire.len()]))) {
        out.reason="profile-cut-contour-joins-unproven";return Ok(out);
    }
    out.proposal=Some(CutContourProposal {loops});out.reason="profile-cut-contour-source-joins-qualified";
    Ok(out)
}
pub struct CutPartitionReport<'c, 'p, 'b> {
    pub partition: Option<CutPartition<'c, 'p, 'b>>,
    pub mapping_cells: usize,
    pub reason: &'static str,
}
impl<'p, 'b> CutEndpoint<'p, 'b> {
    /// Split by an original UV crossing expression, preserving its root on both
    /// directed fragments. No rounded world point or parameter becomes an end.
    pub fn split_source_use<'c>(
        &'c self,
        max_mapping_cells: usize,
    ) -> Result<CutPartitionReport<'c, 'p, 'b>> {
        use nurbs_core::{curve::Curve, interval_eval::Interval as I};
        let shell = self.proposal.body.geometry().shell();
        let edge = &shell.edges()[self.edge];
        let addresses = &shell.uses()[self.edge];
        let side = usize::from(addresses[0].face != self.face);
        let source = &edge.uses()[side];
        let other = &edge.uses()[1 - side];
        let profile = self.proposal.profile(self.face).unwrap();
        let section = profile.section(self.axis, self.parameter)?;
        let [lo, hi] = section.range();
        let width = hi - lo;
        let ends = [lo - width, hi + width];
        if !ends.iter().all(|x| x.is_finite()) || ends[0] >= lo || ends[1] <= hi {
            return Err(Error::new(
                "BREP_PROFILE_CUT",
                "Bound the original UV cut line",
            ));
        }
        let contact = Curve::from_polyline(
            ends.map(|free| {
                let mut p = vec![self.parameter; 2];
                p[1 - self.axis] = free;
                p
            })
            .to_vec(),
        )?;
        let t = crate::source_seam_tangency::inverse(
            edge,
            side,
            I::new(self.canonical[0], self.canonical[1])?,
        )?;
        let contact_t = I::point(section.range()[usize::from(self.high)])
            .sub(I::point(ends[0]))?
            .div(I::point(ends[1]).sub(I::point(ends[0]))?)?;
        let d = source.curve().domain();
        let margin = (d[1] - d[0]) * 0.05;
        let selector = [
            [(t.lo - margin).max(d[0]), (t.hi + margin).min(d[1])],
            [(contact_t.lo - 0.05).max(0.), (contact_t.hi + 0.05).min(1.)],
        ];
        let report = crate::source_contact_point::qualify(
            source.surface(),
            source.curve(),
            &contact,
            selector,
            max_mapping_cells,
        )?;
        let mut out = CutPartitionReport {
            partition: None,
            mapping_cells: report.mapping_cells,
            reason: report.reason,
        };
        let Some(point) = report.point else {
            return Ok(out);
        };
        let selected = source.split_at(&point, crate::source_boundary_fragment::Role::Boundary)?;
        let root = point.root_parameters()[0];
        let canonical = crate::source_seam_tangency::map(edge, side, I::new(root[0], root[1])?)?
            .intersect(0., 1.)?;
        let transported = crate::source_seam_tangency::inverse(edge, 1 - side, canonical)?;
        let bounds = other.parameter_bounds();
        let cut = [transported.lo, transported.hi];
        if cut[0] <= bounds[0][1].min(bounds[1][1]) || cut[1] >= bounds[0][0].max(bounds[1][0]) {
            out.reason = "profile-cut-neighbor-order-unproven";
            return Ok(out);
        }
        out.partition = Some(CutPartition {
            endpoint: self,
            point,
            selected,
            neighbor_ranges: [[bounds[0], cut], [cut, bounds[1]]],
        });
        out.reason = "profile-cut-source-partition-qualified";
        Ok(out)
    }
    pub fn proposal(&self) -> &Proposal<'_> {
        self.proposal
    }
    pub fn faces(&self) -> [usize; 2] {
        [self.face, self.neighbor]
    }
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn canonical_bounds(&self) -> [f64; 2] {
        self.canonical
    }
    pub fn neighbor_uv_bounds(&self) -> [[f64; 2]; 2] {
        self.neighbor_uv
    }
    pub fn world_bounds(&self) -> [[f64; 2]; 3] {
        self.world
    }
    pub fn definition(&self) -> value_codec::Value {
        value_codec::json!({"version":1,"kind":"owned-profile-section-end","face":self.face,
            "edge":self.edge,"axis":self.axis,"parameter":self.parameter,"highFreeEnd":self.high})
    }
}
pub struct CutEndpointReport<'p, 'b> {
    pub endpoint: Option<CutEndpoint<'p, 'b>>,
    pub spans: usize,
    pub reason: &'static str,
}
impl<'a> Proposal<'a> {
    /// Bind a new section end to both retained original edge uses. The selected
    /// use must be affine on a preserved natural boundary; its neighbor may
    /// have a nonlinear pcurve. No world-distance test grants ownership.
    pub fn cut_endpoint<'p>(
        &'p self,
        face: usize,
        axis: usize,
        parameter: f64,
        high: bool,
        index: usize,
        max_spans: usize,
    ) -> Result<CutEndpointReport<'p, 'a>> {
        use nurbs_core::interval_eval::Interval as I;
        let profile = self
            .profile(face)
            .ok_or_else(|| Error::new("BREP_PROFILE_CUT", "Choose a proposed face"))?;
        let section = profile.section(axis, parameter)?;
        let shell = self.body.geometry().shell();
        let addresses = shell
            .uses()
            .get(index)
            .ok_or_else(|| Error::new("BREP_PROFILE_CUT", "Choose an owned edge"))?;
        if max_spans > 100000 {
            return Err(Error::new("BREP_PROFILE_CUT", "Bound cut parameter work"));
        }
        let mut out = CutEndpointReport {
            endpoint: None,
            spans: 0,
            reason: "profile-cut-carrier-unproven",
        };
        let Some(side) = addresses.iter().position(|u| u.face == face) else {
            return Ok(out);
        };
        let edge = &shell.edges()[index];
        let regions = shell.regions().unwrap();
        if (0..2).any(|i| {
            !same(
                edge.uses()[i].surface(),
                regions[addresses[i].face].loops()[0][0].surface(),
            )
        }) {
            return Ok(out);
        }
        let use_ = &edge.uses()[side];
        let c = use_.curve();
        let boundary = section.range()[usize::from(high)];
        if !edge.maps_complete_canonical_source()
            || !same(use_.surface(), profile.source())
            || c.periodic
            || c.degree != 1
            || c.control_points.len() != 2
            || c.weights[0] != c.weights[1]
            || c.control_points
                .iter()
                .any(|p| p.len() != 2 || p[1 - axis] != boundary)
            || c.control_points[0][axis] == c.control_points[1][axis]
        {
            return Ok(out);
        }
        let ends = [c.control_points[0][axis], c.control_points[1][axis]];
        if parameter < ends[0].min(ends[1]) || parameter > ends[0].max(ends[1]) {
            return Ok(out);
        }
        let normalized = I::point(parameter)
            .sub(I::point(ends[0]))?
            .div_signed(I::point(ends[1]).sub(I::point(ends[0]))?)?
            .intersect(0., 1.)?;
        let d = c.domain();
        let t = I::point(d[0])
            .add(normalized.mul(I::point(d[1]).sub(I::point(d[0]))?)?)?
            .intersect(d[0], d[1])?;
        let canonical = crate::source_seam_tangency::map(edge, side, t)?.intersect(0., 1.)?;
        for current in 0..2 {
            let t = crate::source_seam_tangency::inverse(edge, current, canonical)?;
            let ends = edge.uses()[current].parameter_bounds();
            if t.lo < ends[0][1].min(ends[1][1]) || t.hi > ends[0][0].max(ends[1][0]) {
                out.reason = "profile-cut-retained-parameter-unproven";
                return Ok(out);
            }
        }
        let neighbor = 1 - side;
        let nc = edge.uses()[neighbor].curve();
        let t = crate::source_seam_tangency::inverse(edge, neighbor, canonical)?;
        let world = edge.world();
        let d = world.domain();
        let w = I::point(d[0])
            .add(canonical.mul(I::point(d[1]).sub(I::point(d[0]))?)?)?
            .intersect(d[0], d[1])?;
        for (curve, t) in [(nc, t), (world, w)] {
            let spent = (curve.degree..curve.control_points.len())
                .filter(|&i| {
                    curve.knots[i] < curve.knots[i + 1]
                        && curve.knots[i] <= t.hi
                        && curve.knots[i + 1] >= t.lo
                })
                .count();
            if spent > max_spans - out.spans {
                out.reason = "profile-cut-work-limit";
                return Ok(out);
            }
            out.spans += spent;
        }
        let uv = nurbs_core::interval_eval::evaluate_interval(nc, t)?;
        let point = nurbs_core::interval_eval::evaluate_interval(world, w)?;
        out.endpoint = Some(CutEndpoint {
            proposal: self,
            face,
            neighbor: addresses[neighbor].face,
            edge: index,
            axis,
            parameter,
            high,
            canonical: [canonical.lo, canonical.hi],
            neighbor_uv: [[uv[0].lo, uv[0].hi], [uv[1].lo, uv[1].hi]],
            world: std::array::from_fn(|k| [point[k].lo, point[k].hi]),
        });
        out.reason = "profile-cut-endpoint-owned";
        Ok(out)
    }
    /// Derive both UV paths from private original carrier ownership and compare
    /// each incident face's actual original or proposed weight representation.
    pub fn qualify_seam<'p>(
        &'p self,
        face: usize,
        neighbor: usize,
        max_edges: usize,
        threshold: f64,
        cells: usize,
        spans: usize,
    ) -> Result<SeamReport<'p, 'a>> {
        self.qualify_selected_seam(face, neighbor, None, max_edges, threshold, cells, spans)
    }
    /// Qualify a single owned edge; never reuse another edge between its faces.
    pub fn qualify_edge<'p>(
        &'p self,
        edge: usize,
        max_edges: usize,
        threshold: f64,
        cells: usize,
        spans: usize,
    ) -> Result<SeamReport<'p, 'a>> {
        let uses = self
            .body
            .geometry()
            .shell()
            .uses()
            .get(edge)
            .ok_or_else(|| Error::new("BREP_PROFILE_SEAM", "Choose an owned edge"))?;
        let mut faces = [uses[0].face, uses[1].face];
        if self.profile(faces[0]).is_none() {
            faces.reverse();
        }
        self.qualify_selected_seam(
            faces[0],
            faces[1],
            Some(edge),
            max_edges,
            threshold,
            cells,
            spans,
        )
    }
    fn qualify_selected_seam<'p>(
        &'p self,
        face: usize,
        neighbor: usize,
        edge: Option<usize>,
        max_edges: usize,
        threshold: f64,
        cells: usize,
        spans: usize,
    ) -> Result<SeamReport<'p, 'a>> {
        let mut out = SeamReport {
            seam: None,
            angular: None,
            reason: "profile-seam-carrier-unproven",
        };
        let profile = self
            .profile(face)
            .ok_or_else(|| Error::new("BREP_PROFILE_SEAM", "Choose a proposed face"))?;
        let map = match edge {
            Some(edge) => crate::source_shared_chord_map::prepare_edge(
                self.body,
                [face, neighbor],
                edge,
                max_edges,
            )?,
            None => {
                crate::source_shared_chord_map::prepare(self.body, [face, neighbor], max_edges)?
            }
        };
        let Some(map) = map else {
            if let Some(index) = edge.filter(|index| *index < max_edges) {
                let angular =
                    self.inspect_curved_edge(face, neighbor, index, threshold, cells, spans)?;
                out.reason = angular.reason;
                if angular.aligned == Some(true) {
                    out.seam = Some(Seam {
                        proposal: self,
                        faces: [face, neighbor],
                        edge: index,
                        threshold,
                        upper: angular.upper_sine_squared.unwrap(),
                    });
                }
                out.angular = Some(angular);
            }
            return Ok(out);
        };
        let axes = map.fixed_axes();
        let fixed = map.fixed_values();
        let free = map.free_endpoints();
        let lines = std::array::from_fn(|side| {
            std::array::from_fn(|end| {
                std::array::from_fn(|axis| {
                    if axis == axes[side] {
                        fixed[side]
                    } else {
                        free[side][end]
                    }
                })
            })
        });
        if !(1..=100000).contains(&cells) || !(1..=100000).contains(&spans) {
            return Err(Error::new(
                "BREP_PROFILE_SEAM",
                "Bound endpoint and path work",
            ));
        }
        let mut pre = nurbs_core::normal_alignment::ProfilePathReport {
            aligned: None,
            cells: 0,
            spans: 0,
            accepted_cells: 0,
            upper_sine_squared: None,
            pending: vec![[0., 1.]],
            reason: "profile-path-work-limit",
        };
        let mut endpoint_upper = 0_f64;
        for end in 0..2 {
            if pre.cells == cells || pre.spans == spans {
                out.reason = pre.reason;
                out.angular = Some(pre);
                return Ok(out);
            }
            let rectangles = std::array::from_fn(|side| lines[side][end].map(|x| [x, x]));
            pre.cells += 1;
            let r = match self.profile(neighbor) {
                Some(other) => profile.inspect_profile_normals(
                    other,
                    rectangles,
                    threshold,
                    spans - pre.spans,
                )?,
                None => profile.inspect_original_normals(
                    map.surfaces()[1],
                    rectangles,
                    threshold,
                    spans - pre.spans,
                )?,
            };
            pre.spans += r.spans;
            if r.aligned != Some(true) {
                pre.pending.push([end as f64; 2]);
                pre.reason = if r.reason == "span-limit" {
                    "profile-path-work-limit"
                } else if r.aligned == Some(false) {
                    pre.aligned = Some(false);
                    "profile-path-angular-mismatch"
                } else {
                    "profile-seam-endpoint-normal-unresolved"
                };
                out.reason = pre.reason;
                out.angular = Some(pre);
                return Ok(out);
            }
            endpoint_upper = endpoint_upper.max(r.sine_squared_interval.unwrap()[1]);
            pre.accepted_cells += 1;
        }
        if pre.cells == cells || pre.spans == spans {
            out.reason = pre.reason;
            out.angular = Some(pre);
            return Ok(out);
        }
        let mut angular = match self.profile(neighbor) {
            Some(other) => profile.inspect_profile_path(
                other,
                lines,
                threshold,
                cells - pre.cells,
                spans - pre.spans,
            )?,
            None => profile.inspect_original_path(
                map.surfaces()[1],
                lines,
                threshold,
                cells - pre.cells,
                spans - pre.spans,
            )?,
        };
        angular.cells += pre.cells;
        angular.spans += pre.spans;
        angular.accepted_cells += pre.accepted_cells;
        if let Some(upper) = angular.upper_sine_squared.as_mut() {
            *upper = upper.max(endpoint_upper);
        }

        out.reason = angular.reason;
        if angular.aligned == Some(true) {
            out.seam = Some(Seam {
                proposal: self,
                faces: [face, neighbor],
                edge: map.edge(),
                threshold,
                upper: angular.upper_sine_squared.unwrap(),
            });
        }
        out.angular = Some(angular);
        Ok(out)
    }
    /// Bind angular qualification to two owned section roots on one carrier.
    /// The interval covers their exact transported enclosures; it is not made
    /// from independently rounded canonical endpoint scalars.
    pub fn qualify_root_ended_seam<'p>(
        &'p self,parts:[&CutPartition<'_, 'p,'a>;2],threshold:f64,cells:usize,spans:usize,
    )->Result<RootEndedSeamReport<'p,'a>> {
        if spans>100000 || !(1..=100000).contains(&cells) || !threshold.is_finite() || !(0. ..1.).contains(&threshold) {
            return Err(Error::new("BREP_PROFILE_SEAM","Bound root-ended seam angular and carrier work"));
        }
        let mut out=RootEndedSeamReport{seam:None,carrier_spans:0,angular:None,reason:"profile-root-seam-ownership-unproven"};
        let ends=[parts[0].endpoint,parts[1].endpoint];
        if ends.iter().any(|e|!std::ptr::eq(e.proposal,self))
            || ends[0].edge!=ends[1].edge || ends[0].face!=ends[1].face
            || ends[0].neighbor!=ends[1].neighbor || ends[0].axis!=ends[1].axis
            || ends[0].high!=ends[1].high || ends[0].parameter==ends[1].parameter {
            return Ok(out);
        }
        let shell=self.body.geometry().shell();let edge=&shell.edges()[ends[0].edge];
        let side=usize::from(shell.uses()[ends[0].edge][0].face!=ends[0].face);
        let mut points=Vec::new();
        for part in parts {
            if out.carrier_spans==spans {out.reason="profile-root-seam-carrier-work-limit";return Ok(out);}
            let checked=crate::source_carrier_point::qualify(edge,&part.point,side,spans-out.carrier_spans)?;
            out.carrier_spans+=checked.spans;out.reason=checked.reason;
            let Some(point)=checked.point else {return Ok(out);};points.push(point);
        }
        let ranges=[points[0].canonical_bounds(),points[1].canonical_bounds()];
        if !(ranges[0][1]<ranges[1][0] || ranges[1][1]<ranges[0][0]) {
            out.reason="profile-root-seam-order-unproven";return Ok(out);
        }
        let canonical=[ranges[0][0].min(ranges[1][0]),ranges[0][1].max(ranges[1][1])];
        if out.carrier_spans==spans {out.reason="profile-root-seam-angular-work-limit";return Ok(out);}
        let angular=self.qualify_edge_interval(ends[0].edge,canonical,threshold,cells,spans-out.carrier_spans)?;
        out.reason=angular.angular.reason;
        if let Some(restricted)=angular.seam {
            out.seam=Some(RootEndedSeam{restricted,points:[points.remove(0),points.remove(0)],face:ends[0].face,axis:ends[0].axis,parameters:[ends[0].parameter,ends[1].parameter],high:ends[0].high});
            out.reason="profile-root-ended-seam-qualified";
        }
        out.angular=Some(angular.angular);Ok(out)
    }
    /// Qualify only the requested interval of an owned original carrier.
    /// UVs are freshly enclosed from both original pcurves and canonical maps.
    pub fn qualify_edge_interval<'p>(
        &'p self,edge:usize,canonical:[f64;2],threshold:f64,cells:usize,spans:usize,
    )->Result<RestrictedSeamReport<'p,'a>> {
        let uses=self.body.geometry().shell().uses().get(edge)
            .ok_or_else(||Error::new("BREP_PROFILE_SEAM","Choose an owned interval edge"))?;
        let mut faces=[uses[0].face,uses[1].face];
        if self.profile(faces[0]).is_none() {faces.reverse();}
        if self.profile(faces[0]).is_none() {
            return Err(Error::new("BREP_PROFILE_SEAM","Require an owned corrected face"));
        }
        let angular=self.inspect_curved_edge_interval(faces[0],faces[1],edge,threshold,cells,spans,canonical)?;
        let seam=if angular.aligned==Some(true) {
            Some(RestrictedSeam{proposal:self,faces,edge,canonical,threshold,upper:angular.upper_sine_squared.unwrap()})
        }else{None};
        Ok(RestrictedSeamReport{seam,angular})
    }
    /// Original nonlinear pcurves remain source-owned; inverse canonical maps
    /// and complete curve-span enclosures replace manually supplied UV lines.
    fn inspect_curved_edge(
        &self,
        face: usize,
        neighbor: usize,
        index: usize,
        threshold: f64,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<nurbs_core::normal_alignment::ProfilePathReport> {
        self.inspect_curved_edge_interval(face,neighbor,index,threshold,max_cells,max_spans,[0.,1.])
    }
    fn inspect_curved_edge_interval(
        &self,face:usize,neighbor:usize,index:usize,threshold:f64,max_cells:usize,max_spans:usize,canonical:[f64;2],
    )->Result<nurbs_core::normal_alignment::ProfilePathReport> {
        use nurbs_core::{interval_eval::Interval as I, normal_alignment::ProfilePathReport};
        if !canonical.iter().all(|x|x.is_finite()) || canonical[0]<0. || canonical[1]>1. || canonical[0]>=canonical[1]
            || !(1..=100000).contains(&max_cells)
            || !(1..=100000).contains(&max_spans)
            || !threshold.is_finite()
            || !(0. ..1.).contains(&threshold)
        {
            return Err(Error::new(
                "BREP_PROFILE_SEAM",
                "Bound original pcurve and normal work",
            ));
        }
        let shell = self.body.geometry().shell();
        let edge = &shell.edges()[index];
        let addresses = &shell.uses()[index];
        let order = if addresses[0].face == face {
            [0, 1]
        } else {
            [1, 0]
        };
        let mut out = ProfilePathReport {
            aligned: None,
            cells: 0,
            spans: 0,
            accepted_cells: 0,
            upper_sine_squared: None,
            pending: vec![canonical],
            reason: "profile-seam-carrier-unproven",
        };
        let regions = shell.regions().unwrap();
        if !edge.maps_complete_canonical_source()
            || [face, neighbor].iter().enumerate().any(|(side, &f)| {
                !same(
                    edge.uses()[order[side]].surface(),
                    regions[f].loops()[0][0].surface(),
                ) || edge.uses()[order[side]].curve().periodic
            })
        {
            return Ok(out);
        }
        let profile = self.profile(face).unwrap();
        // Singular literal source endpoints are explicitly inspected before
        // repeatedly subdividing an interval that contains a zero normal.
        out.pending.push([canonical[1];2]);
        out.pending.push([canonical[0];2]);
        out.reason = "profile-path-work-limit";
        let mut upper = 0_f64;
        while let Some(q) = out.pending.pop() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(q);
                return Ok(out);
            }
            out.cells += 1;
            let mut rectangles = [[[0.; 2]; 2]; 2];
            for side in 0..2 {
                let use_ = &edge.uses()[order[side]];
                let c = use_.curve();
                let t =
                    crate::source_seam_tangency::inverse(edge, order[side], I::new(q[0], q[1])?)?;
                let spent = (c.degree..c.control_points.len())
                    .filter(|&i| {
                        c.knots[i] < c.knots[i + 1] && c.knots[i] <= t.hi && c.knots[i + 1] >= t.lo
                    })
                    .count();
                if spent > max_spans - out.spans {
                    out.pending.push(q);
                    return Ok(out);
                }
                out.spans += spent;
                let uv = nurbs_core::interval_eval::evaluate_interval(c, t)?;
                let s = use_.surface();
                let natural = [
                    [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                    [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
                ];
                for axis in 0..2 {
                    let r = uv[axis].intersect(natural[axis][0], natural[axis][1])?;
                    rectangles[side][axis] = [r.lo, r.hi];
                }
            }
            if out.spans == max_spans {
                out.pending.push(q);
                return Ok(out);
            }
            let r = match self.profile(neighbor) {
                Some(other) => profile.inspect_profile_normals(
                    other,
                    rectangles,
                    threshold,
                    max_spans - out.spans,
                )?,
                None => profile.inspect_original_normals(
                    edge.uses()[order[1]].surface(),
                    rectangles,
                    threshold,
                    max_spans - out.spans,
                )?,
            };
            out.spans += r.spans;
            if r.aligned == Some(true) {
                upper = upper.max(r.sine_squared_interval.unwrap()[1]);
                out.accepted_cells += 1;
                continue;
            }
            if r.aligned == Some(false) {
                out.pending.push(q);
                out.aligned = Some(false);
                out.reason = "profile-path-angular-mismatch";
                return Ok(out);
            }
            if q[0] == q[1] {
                out.pending.push(q);
                out.reason = "profile-seam-endpoint-normal-unresolved";
                return Ok(out);
            }
            if r.reason == "span-limit" {
                out.pending.push(q);
                return Ok(out);
            }
            let mid = q[0] + (q[1] - q[0]) * 0.5;
            if mid <= q[0] || mid >= q[1] {
                out.pending.push(q);
                out.reason = "profile-path-resolution-limit";
                return Ok(out);
            }
            out.pending.push([mid, q[1]]);
            out.pending.push([q[0], mid]);
        }
        out.aligned = Some(true);
        out.upper_sine_squared = Some(upper);
        out.reason = "profile-path-tangent-planes-qualified";
        Ok(out)
    }
    pub fn original_body(&self) -> &'a Body {
        self.body
    }
    pub fn profile(&self, face: usize) -> Option<&WeightProfile<'a>> {
        self.faces
            .iter()
            .find(|f| f.spec.face == face)
            .map(|f| &f.profile)
    }
    pub fn specs(&self) -> Vec<FaceSpec> {
        self.faces.iter().map(|f| f.spec).collect()
    }
    pub fn axis_deviation_upper(&self) -> [f64; 3] {
        self.axis_deviation_upper
    }
    /// Persist source and exact factor recipes, never flattened target weights or
    /// claimed geometry certificates. Restoration must rerun all admission gates.
    pub fn definition(&self) -> Result<value_codec::Value> {
        Ok(
            value_codec::json!({"version":1,"kind":"boundary-preserving-weight-profiles",
            "originalBody":self.body.definition()?,"positionTolerance":self.position_tolerance,
            "profiles":self.faces.iter().map(|f|value_codec::json!({"face":f.spec.face,"anchor":f.spec.anchor})).collect::<Vec<_>>()}),
        )
    }
}
pub struct Report<'a> {
    pub proposal: Option<Proposal<'a>>,
    pub controls: usize,
    pub fragments: usize,
    pub reason: &'static str,
}
fn same(a: &Surface, b: &Surface) -> bool {
    a.degree_u == b.degree_u
        && a.degree_v == b.degree_v
        && a.periodic_u == b.periodic_u
        && a.periodic_v == b.periodic_v
        && a.knots_u == b.knots_u
        && a.knots_v == b.knots_v
        && a.control_points == b.control_points
        && a.weights == b.weights
}
pub fn prepare<'a>(body: &'a Body, specs: &[FaceSpec], limits: Limits) -> Result<Report<'a>> {
    let regions = body
        .geometry()
        .shell()
        .regions()
        .ok_or_else(|| Error::new("BREP_PROFILE_BODY", "Require original retained regions"))?;
    if specs.is_empty()
        || specs.len() > regions.len()
        || limits.controls > 1_000_000
        || limits.fragments > 100000
        || !limits.position_tolerance.is_finite()
        || limits.position_tolerance < 0.
        || specs.iter().any(|f| f.face >= regions.len())
        || specs
            .iter()
            .enumerate()
            .any(|(i, f)| specs[..i].iter().any(|a| a.face == f.face))
    {
        return Err(Error::new(
            "BREP_PROFILE_BODY",
            "Choose distinct owned faces and finite bounded proposal limits",
        ));
    }
    let mut out = Report {
        proposal: None,
        controls: 0,
        fragments: 0,
        reason: "profile-body-work-limit",
    };
    let mut faces = Vec::new();
    let mut deviation = [0_f64; 3];
    for spec in specs {
        let region = &regions[spec.face];
        let source = region.loops()[0][0].surface();
        let (profile, used) = nurbs_core::normal_alignment::qualify_weight_profile(
            source,
            spec.anchor,
            limits.controls - out.controls,
        )?;
        out.controls += used;
        let Some(profile) = profile else {
            return Ok(out);
        };
        if !profile.within_position_tolerance(limits.position_tolerance)? {
            out.reason = "profile-body-position-tolerance-unproven";
            return Ok(out);
        }
        // Check all natural boundaries, including collapsed rails absent from
        // the retained wire. This preserves original pole and edge positions.
        for axis in 0..2 {
            for high in [false, true] {
                let (boundary, used) =
                    profile.preserved_boundary(axis, high, limits.controls - out.controls)?;
                out.controls += used;
                if boundary.is_none() {
                    out.reason = "profile-body-boundary-identity-unproven";
                    return Ok(out);
                }
            }
        }
        let domains = [
            [
                source.knots_u[source.degree_u],
                source.knots_u[source.control_points.len()],
            ],
            [
                source.knots_v[source.degree_v],
                source.knots_v[source.control_points[0].len()],
            ],
        ];
        for wire in region.loops() {
            for fragment in wire {
                if out.fragments == limits.fragments {
                    return Ok(out);
                }
                out.fragments += 1;
                if !same(fragment.surface(), source) || fragment.curve().periodic {
                    out.reason = "profile-body-contour-source-unproven";
                    return Ok(out);
                }
                let mut fixed = [true; 4];
                for point in &fragment.curve().control_points {
                    if out.controls == limits.controls {
                        return Ok(out);
                    }
                    out.controls += 1;
                    if point.len() != 2
                        || (0..2).any(|a| point[a] < domains[a][0] || point[a] > domains[a][1])
                    {
                        out.reason = "profile-body-contour-boundary-unproven";
                        return Ok(out);
                    }
                    for axis in 0..2 {
                        for high in 0..2 {
                            fixed[2 * axis + high] &= point[axis] == domains[axis][high];
                        }
                    }
                }
                if !fixed.iter().any(|x| *x) {
                    out.reason = "profile-body-interior-trim-unproven";
                    return Ok(out);
                }
            }
        }
        for (k, error) in profile.axis_deviation_upper().into_iter().enumerate() {
            deviation[k] = deviation[k].max(error);
        }
        faces.push(Face {
            spec: *spec,
            profile,
        });
    }
    out.reason = "profile-body-boundaries-preserved";
    out.proposal = Some(Proposal {
        body,
        faces,
        axis_deviation_upper: deviation,
        position_tolerance: limits.position_tolerance,
    });
    Ok(out)
}
