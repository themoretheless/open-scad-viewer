//! Fresh native admission of an embedded closed source shell, before volume.
use crate::{
    source_face_contacts,
    source_shell_incidence::{ChartReport, Shell},
    source_vertex_links,
};
use nurbs_core::{Error, Result};
pub struct Limits {
    pub tolerance_uv: f64,
    pub corners: usize,
    pub spans: usize,
    pub linear_cells: usize,
    pub pairs: crate::face_contacts::Limits,
    pub exact_work: u64,
    pub driver_cells: usize,
}
pub struct Geometry {
    shell: Shell,
    topology: source_vertex_links::Report,
    charts: ChartReport,
    contacts: source_face_contacts::ShellReport,
}
impl Geometry {
    pub fn shell(&self) -> &Shell {
        &self.shell
    }
    pub fn topology(&self) -> &source_vertex_links::Report {
        &self.topology
    }
    pub fn charts(&self) -> &ChartReport {
        &self.charts
    }
    pub fn contacts(&self) -> &source_face_contacts::ShellReport {
        &self.contacts
    }
}
pub struct Report {
    pub geometry: Option<Geometry>,
    pub corners: usize,
    pub spans: usize,
    pub linear_cells: usize,
    pub exact_work: u64,
    pub driver_cells: usize,
    pub pairs: usize,
    pub reason: &'static str,
    pub uncertain_vertex: Option<usize>,
    pub uncertain_face: Option<usize>,
    pub next_pair: Option<[usize; 2]>,
}
/// Own the immutable source payload and recompute all admission gates. Public
/// diagnostic reports or caller certificates never authorize this geometry.
/// Material orientation, signed volume and Model conversion remain separate.
pub fn qualify(shell: Shell, limits: Limits) -> Result<Report> {
    if !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
        || shell.regions().is_none()
        || !(1..=100000).contains(&limits.corners)
        || !(1..=100000).contains(&limits.spans)
        || limits.linear_cells > 100000
        || !(1..=100_000_000).contains(&limits.exact_work)
        || !(1..=100000).contains(&limits.driver_cells)
    {
        return Err(Error::new(
            "BREP_SOURCE_SHELL_GEOMETRY",
            "Choose qualified source regions and bounded geometry work",
        ));
    }
    let mut out = Report {
        geometry: None,
        corners: 0,
        spans: 0,
        linear_cells: 0,
        exact_work: 0,
        driver_cells: 0,
        pairs: 0,
        reason: "source-shell-topology-unproven",
        uncertain_vertex: None,
        uncertain_face: None,
        next_pair: None,
    };
    let topology = source_vertex_links::inspect(&shell, limits.corners)?;
    out.corners = topology.corners;
    if !topology.all_manifold || topology.genus.is_none() {
        out.reason = topology.reason;
        out.uncertain_vertex = topology.uncertain_vertex;
        return Ok(out);
    }
    let charts = shell.inspect_face_charts(limits.spans, limits.linear_cells)?;
    out.spans = charts.spans;
    out.linear_cells = charts.linear_cells;
    out.reason = "source-shell-chart-injectivity-unproven";
    if !charts.all_injective {
        out.uncertain_face = charts
            .faces
            .iter()
            .find(|f| !f.injectivity_proven())
            .map(|f| f.face);
        return Ok(out);
    }
    out.reason = "source-shell-contact-work-limit";
    if out.spans == limits.spans {
        out.next_pair = Some([0, 1]);
        return Ok(out);
    }
    let contacts = source_face_contacts::inspect_shell_with_boundary_fibers_and_chart_work(
        &shell,
        limits.tolerance_uv,
        limits.pairs,
        limits.exact_work,
        limits.spans - out.spans,
        limits.driver_cells,
        limits.linear_cells - out.linear_cells,
    )?;
    out.spans += contacts.spans;
    out.linear_cells += contacts.linear_cells;
    out.exact_work = contacts.exact_work;
    out.driver_cells = contacts.driver_cells;
    out.pairs = contacts.pairs.len();
    out.reason = "source-shell-different-face-contacts-unproven";
    out.next_pair = contacts
        .pairs
        .iter()
        .find(|p| {
            p.allowed.is_none()
                && p.fiber.is_none()
                && p.interior_fiber.is_none()
                && p.paired_fiber.is_none()
                && p.vertex_contact.is_none()
                && p.disjoint_hull.is_none()
                && p.pole_paired.is_none()
                && !p.result.as_ref().is_some_and(|r| r.absence_proven)
        })
        .map(|p| p.faces)
        .or(contacts.next_pair);
    if !contacts.all_pairs_qualified
        || contacts.next_pair.is_some()
        || contacts.pairs.len() != contacts.total_pairs
    {
        return Ok(out);
    }
    out.geometry = Some(Geometry {
        shell,
        topology,
        charts,
        contacts,
    });
    out.reason = "source-shell-embedded-geometry-qualified";
    Ok(out)
}
