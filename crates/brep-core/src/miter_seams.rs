//! Natural retained-wall seam extraction from actual B-rep coedge ownership.
use crate::{Model, Result};
use nurbs_core::{curve::Curve, surface::Surface, sweep_cap_wall::{self, Boundary}, sweep_seam_set::Seam};
use std::collections::{BTreeSet, HashMap};

pub struct Extraction {
    pub seams: Vec<Seam>,
    pub edge_ids: Vec<usize>,
    pub cap_edges: Vec<usize>,
    pub unclassified_faces: Vec<usize>,
    pub unpaired_edges: Vec<usize>,
}
impl Extraction {
    pub fn complete(&self) -> bool { self.unclassified_faces.is_empty() && self.unpaired_edges.is_empty() }
}
fn boundary(s: &Surface, p: &Curve, station: bool) -> Option<&'static str> {
    if p.degree != 1 || p.control_points.len() != 2 || p.weights.len() != 2 || p.weights[0] != p.weights[1] || p.weights[0] <= 0. {return None;}
    let a = &p.control_points[0]; let b = &p.control_points[1];
    let u0 = *s.knots_u.get(s.degree_u)?; let u1 = *s.knots_u.get(s.control_points.len())?;
    let v0 = *s.knots_v.get(s.degree_v)?; let v1 = *s.knots_v.get(s.control_points.first()?.len())?;
    let label = if station {
        if a.get(1)? != b.get(1)? || !((a.first()? == &u0 && b.first()? == &u1) || (a.first()? == &u1 && b.first()? == &u0)) {return None;}
        if a[1] == v0 {"vMin"} else if a[1] == v1 {"vMax"} else {return None;}
    } else {
        if a.first()? != b.first()? || !((a.get(1)? == &v0 && b.get(1)? == &v1) || (a.get(1)? == &v1 && b.get(1)? == &v0)) {return None;}
        if a[0] == u0 {"uMin"} else if a[0] == u1 {"uMax"} else {return None;}
    };
    let axis = match label {"uMin" => Boundary::UMin, "uMax" => Boundary::UMax, "vMin" => Boundary::VMin, _ => Boundary::VMax};
    sweep_cap_wall::covers_boundary(s, p, axis).ok()?.then_some(label)
}
pub fn extract(model: &Model, caps: &[usize], station: bool) -> Result<Extraction> {
    // Inspect represented boundaries even on an unadmitted diagnostic model.
    // Broken pcurves remain unresolved; invalid topology references are input errors.
    if model.faces.iter().any(|f| std::iter::once(&f.outer).chain(&f.holes).any(|&w| w >= model.loops.len()))
        || model.loops.iter().flat_map(|w| &w.coedges).any(|c| c.edge >= model.edges.len()) {
        return Err(nurbs_core::Error::new("NURBS_INPUT", "Invalid retained seam topology reference"));
    }
    let caps_set: BTreeSet<_> = caps.iter().copied().collect();
    if caps_set.len() != caps.len() || caps.iter().any(|&f| f >= model.faces.len()) {
        return Err(nurbs_core::Error::new("NURBS_INPUT", "Invalid station smoothness cap scope"));
    }
    let mut cap_owned = BTreeSet::new();
    for &id in caps {let f = &model.faces[id]; for wire in std::iter::once(&f.outer).chain(&f.holes) {for c in &model.loops[*wire].coedges {cap_owned.insert(c.edge);}}}
    let mut uses: HashMap<usize, Vec<(usize, &'static str)>> = HashMap::new();
    let mut edges = Vec::new();
    let mut result = Extraction {seams: Vec::new(), edge_ids: Vec::new(), cap_edges: Vec::new(), unclassified_faces: Vec::new(), unpaired_edges: Vec::new()};
    for (id, f) in model.faces.iter().enumerate() {
        if caps_set.contains(&id) {continue;}
        let mut found = BTreeSet::new();
        for wire in std::iter::once(&f.outer).chain(&f.holes) {for c in &model.loops[*wire].coedges {
            if let Some(label) = boundary(&f.surface, &c.pcurve, station) {
                found.insert(label);
                if !uses.contains_key(&c.edge) {edges.push(c.edge);}
                uses.entry(c.edge).or_default().push((id, label));
            }
        }}
        let expected = if station {["vMin", "vMax"]} else {["uMin", "uMax"]};
        if expected.iter().any(|s| !found.contains(s)) {result.unclassified_faces.push(id);}
    }
    for edge in edges {
        let list = &uses[&edge];
        if station && list.len() == 1 && cap_owned.contains(&edge) {result.cap_edges.push(edge); continue;}
        if list.len() != 2 || (station && cap_owned.contains(&edge)) {result.unpaired_edges.push(edge); continue;}
        let (a, ab) = list[0]; let (b, bb) = list[1];
        let scale = if station {nurbs_core::continuity::propose_station_normal_scale(&model.faces[a].surface, &model.faces[b].surface, ab, bb)?} else {1.};
        result.seams.push(Seam {patches: [a,b], boundaries: [ab.into(),bb.into()], order: 2, normal_scale: scale});
        result.edge_ids.push(edge);
    }
    Ok(result)
}


pub struct Smoothness {
    pub extraction: Extraction,
    pub g2: nurbs_core::sweep_seam_set::Report,
    pub g1: Option<nurbs_core::sweep_seam_set::Report>,
    pub g1_certified: bool,
    pub work: u64,
}
pub fn inspect(model: &Model, caps: &[usize], max_work: u64, station: bool) -> Result<Smoothness> {
    let mut extraction = extract(model, caps, station)?;
    let patches: Vec<_> = model.faces.iter().map(|f| f.surface.clone()).collect();
    let mut g2 = nurbs_core::sweep_seam_set::inspect(&patches, &extraction.seams, max_work, true)?;
    if !extraction.complete() {g2.certified = false; g2.order = None;}
    let g1 = if g2.certified {None} else {
        for seam in &mut extraction.seams {seam.order = 1;}
        let mut report = nurbs_core::sweep_seam_set::inspect(&patches, &extraction.seams, max_work - g2.work, true)?;
        if !extraction.complete() {report.certified = false; report.order = None;}
        Some(report)
    };
    let g1_certified = extraction.complete() && (g2.certified || g1.as_ref().is_some_and(|r| r.certified));
    let work = g2.work + g1.as_ref().map_or(0, |r| r.work);
    Ok(Smoothness {extraction, g2, g1, g1_certified, work})
}
pub struct ProfileReport {pub profile: Smoothness, pub station: Smoothness, pub total_work: u64}
pub fn inspect_profile(model: &Model, caps: &[usize], max_work: u64) -> Result<ProfileReport> {
    let profile = inspect(model, caps, max_work, false)?;
    let station = inspect(model, caps, max_work - profile.work, true)?;
    let total_work = profile.work + station.work;
    Ok(ProfileReport {profile, station, total_work})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_circle_profile_and_station_share_budget_without_mutation() {
        let sections: Vec<_> = [0., 1., 2.].into_iter().map(|z| vec![vec![nurbs_core::primitives::ellipse_arc(
            [0.,0.,z], [1.,0.,0.], [0.,1.,0.], 0., 360.).unwrap()]]).collect();
        let model = crate::rational_section_loft(&sections).unwrap();
        let original = model.clone();
        let caps = vec![model.faces.len()-2, model.faces.len()-1];
        let report = inspect_profile(&model,&caps,2_000_000).unwrap();
        assert!(report.profile.extraction.complete());
        assert!(report.profile.g2.certified);
        assert!(report.station.extraction.complete());
        assert!(report.station.g2.certified);
        assert_eq!(report.total_work,report.profile.work+report.station.work);
        assert!(report.total_work > 0);
        let refused = inspect_profile(&model,&caps,0).unwrap();
        assert!(!refused.profile.g1_certified && !refused.station.g1_certified);
        assert_eq!(refused.total_work,0);
        let charts = inspect_charts(&model,&caps,100_000).unwrap();
        assert!(charts.certified);
        assert_eq!(charts.charts.len(),model.faces.len()-caps.len());
        assert_eq!(charts.cells,charts.charts.iter().map(|(_,r)| r.cells).sum());
        assert!(!inspect_charts(&model,&caps,0).unwrap().certified);
        assert!(inspect_charts(&model,&[caps[0],caps[0]],100_000).is_err());
        let limited=inspect_profile(&model,&caps,report.total_work-1).unwrap();
        assert!(limited.total_work<=report.total_work-1);
        assert!(!limited.station.g2.certified);
        let mut damaged=model.clone();
        let wire=damaged.faces[0].outer;
        let edge=damaged.loops[wire].coedges.iter_mut().find(|c| c.pcurve.control_points[0][1]==c.pcurve.control_points[1][1]).unwrap();
        edge.pcurve.control_points[1][0]=0.5;
        let diagnostic=inspect(&damaged,&caps,2_000_000,true).unwrap();
        assert!(!diagnostic.extraction.complete());
        assert!(!diagnostic.g1_certified && !diagnostic.g2.certified);
        assert_eq!(model,original);
        assert!(extract(&model,&[caps[0],caps[0]],true).is_err());
    }
}


pub struct Charts {
    pub certified: bool,
    pub cells: usize,
    pub charts: Vec<(usize, nurbs_core::surface_linear_monotonicity::Report)>,
    pub unresolved: Vec<usize>,
}
pub fn inspect_charts(model: &Model, caps: &[usize], max_cells: usize) -> Result<Charts> {
    let selected: BTreeSet<_> = caps.iter().copied().collect();
    if max_cells > 100_000 || model.faces.len() > 1024 || caps.len() > 16 || selected.len() != caps.len() || caps.iter().any(|&i| i >= model.faces.len()) {
        return Err(nurbs_core::Error::new("NURBS_INPUT", "Invalid retained wall chart selection or budget"));
    }
    let mut charts = Vec::new(); let mut cells = 0;
    for (face, value) in model.faces.iter().enumerate() {
        if selected.contains(&face) {continue;}
        let audit = nurbs_core::surface_linear_monotonicity::inspect_candidate(&value.surface,max_cells-cells)?;
        if audit.cells > max_cells-cells {return Err(nurbs_core::Error::new("NURBS_INPUT", "Invalid native chart work accounting"));}
        cells += audit.cells; charts.push((face,audit));
    }
    let unresolved: Vec<_> = charts.iter().filter_map(|(f,r)| (!r.certified).then_some(*f)).collect();
    Ok(Charts {certified: !charts.is_empty() && unresolved.is_empty(),cells,charts,unresolved})
}
