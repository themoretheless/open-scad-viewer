//! Exact column semantics for an admitted original rectangular graph Body.
//! This is source material evidence, not SS sewing or general Boolean authority.
use crate::{Error, Model, Result, source_body_restore};
use nurbs_core::ss_intersection::{
    QuadraticGraphSSLimits, QualifiedGraphSupport, qualify_graph_support,
};
#[cfg(test)]
pub(crate) mod tests;
mod topology;
pub struct Limits {
    /// Shared structural/coefficient work, with each core graph query <=4096.
    pub operations: usize,
    pub body: source_body_restore::Limits,
}
pub struct QualifiedGraphBody<'a> {
    source: &'a Model,
    limits: &'a Limits,
    roof: usize,
    floor: usize,
    walls: [usize; 4],
    roof_support: QualifiedGraphSupport,
    floor_support: QualifiedGraphSupport,
    floor_height: f64,
    xy: [[f64; 2]; 2],
    body: crate::source_volume::Body,
    equivalences: Vec<nurbs_core::ss_intersection::QualifiedPolynomialSurface>,
    operations: usize,
}
impl QualifiedGraphBody<'_> {
    pub fn source(&self) -> &Model {
        self.source
    }
    pub fn roof(&self) -> usize {
        self.roof
    }
    pub fn floor(&self) -> usize {
        self.floor
    }
    /// xmin,xmax,ymin,ymax actual source faces.
    pub fn walls(&self) -> [usize; 4] {
        self.walls
    }
    pub fn roof_support(&self) -> &QualifiedGraphSupport {
        &self.roof_support
    }
    pub fn floor_support(&self) -> &QualifiedGraphSupport {
        &self.floor_support
    }
    pub fn footprint(&self) -> [[f64; 2]; 2] {
        self.xy
    }
    pub fn floor_height(&self) -> f64 {
        self.floor_height
    }
    pub fn equivalent_body(&self) -> &crate::source_volume::Body {
        &self.body
    }
    /// Exact same native parameters and face/edge topology; original source is retained.
    pub fn equivalences(&self) -> &[nurbs_core::ss_intersection::QualifiedPolynomialSurface] {
        &self.equivalences
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = qualify(self.source, self.roof, self.floor, self.limits)?;
        require(
            self.walls == fresh.walls
                && self.xy == fresh.xy
                && self.floor_height == fresh.floor_height
                && self.operations == fresh.operations
                && self.body.reverse_orientation() == fresh.body.reverse_orientation(),
            "Original graph body ownership changed",
        )
    }
}
fn error(message: &str) -> Error {
    Error::new("BREP_GRAPH_BODY", message)
}
fn require(ok: bool, message: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(error(message)) }
}
struct Work {
    used: usize,
    max: usize,
}
impl Work {
    fn charge(&mut self, n: usize) -> Result<()> {
        if n > self.max - self.used {
            return Err(error("Original graph body work exhausted"));
        }
        self.used += n;
        Ok(())
    }
    fn core(&self) -> Result<usize> {
        require(self.used < self.max, "No original graph work remains")?;
        Ok((self.max - self.used).min(4096))
    }
}
fn footprint(g: &QualifiedGraphSupport) -> [[f64; 2]; 2] {
    let [x, dx, y, dy] = g.xy_frame();
    [
        [x.min(x + dx), x.max(x + dx)],
        [y.min(y + dy), y.max(y + dy)],
    ]
}
/// Prove the actual source solid occupies the complete column between the flat
/// floor and original graph roof over one common rectangular XY footprint.
/// Source coefficient identities, complete natural charts, embedded closed
/// incidence and outward material senses are all required. No sample point or
/// `closed` flag assigns volume roles, and input geometry is never rewritten.
pub fn qualify<'a>(
    source: &'a Model,
    roof: usize,
    floor: usize,
    limits: &'a Limits,
) -> Result<QualifiedGraphBody<'a>> {
    require(
        (1..=1000000).contains(&limits.operations),
        "Bound original graph body operations",
    )?;
    let mut work = Work {
        used: 0,
        max: limits.operations,
    };
    topology::layout(source, roof, floor, &mut work)?;
    let roof_support = qualify_graph_support(
        &source.faces[roof].surface,
        QuadraticGraphSSLimits {
            max_operations: work.core()?,
        },
    )?;
    work.charge(roof_support.operations())?;
    let floor_support = qualify_graph_support(
        &source.faces[floor].surface,
        QuadraticGraphSSLimits {
            max_operations: work.core()?,
        },
    )?;
    work.charge(floor_support.operations())?;
    let xy = footprint(&roof_support);
    require(
        xy == footprint(&floor_support),
        "Original roof and floor footprints differ",
    )?;
    let floor_height = floor_support.source().control_points[0][0][2];
    for p in floor_support.source().control_points.iter().flatten() {
        work.charge(1)?;
        require(
            p[2] == floor_height,
            "Original floor is not exactly horizontal",
        )?;
    }
    let separated = roof_support.prove_above_floor(floor_height, work.core()?)?;
    work.charge(separated)?;
    let walls = topology::walls(source, roof, floor, xy, &mut work)?;
    let mut equivalent = source.clone();
    let mut equivalences = Vec::new();
    for face in 0..6 {
        let proof = nurbs_core::ss_intersection::qualify_polynomial_surface(
            &source.faces[face].surface,
            QuadraticGraphSSLimits {
                max_operations: work.core()?,
            },
        )?;
        work.charge(proof.operations())?;
        equivalent.0.faces[face].surface = proof.polynomial().clone();
        equivalences.push(proof);
    }
    equivalent.rebuild_topology_ids();
    let incidence = crate::source_support_shell::prepare(
        &equivalent,
        crate::source_support_shell::Limits {
            tolerance_uv: limits.body.embedding.tolerance_uv,
            faces: 6,
            uses: 24,
            regions: limits.body.shell.regions.region,
            exact_work: limits.body.shell.exact_work,
        },
    )?;
    let shell = incidence
        .shell
        .as_ref()
        .ok_or_else(|| error(incidence.reason))?;
    require(
        shell.regions().is_some_and(|regions| {
            regions
                .iter()
                .all(|r| r.whole_chart_material() && r.chart_winding() == 1)
        }),
        "Graph body requires all six complete original natural rectangles",
    )?;
    let embedded = crate::source_shell_geometry::qualify(
        incidence.shell.unwrap(),
        topology::embedding(&limits.body.embedding),
    )?;
    let geometry = embedded.geometry.ok_or_else(|| {
        error(&format!(
            "{} at face {:?}",
            embedded.reason, embedded.uncertain_face
        ))
    })?;
    let volume = crate::source_volume::qualify(geometry, topology::volume(&limits.body.volume))?;
    let body = volume.body.ok_or_else(|| error(volume.reason))?;
    require(
        source.shells[0]
            .faces
            .iter()
            .all(|u| u.reversed == body.reverse_orientation()),
        "Actual Model face senses disagree with requalified material orientation",
    )?;
    let positive = |g: &QualifiedGraphSupport| {
        let [_, dx, _, dy] = g.xy_frame();
        (dx > 0.) == (dy > 0.)
    };
    let reversed = body.reverse_orientation();
    require(
        positive(&roof_support) != reversed && positive(&floor_support) == reversed,
        "Original roof/floor normals do not bound the positive material column",
    )?;
    Ok(QualifiedGraphBody {
        source,
        limits,
        roof,
        floor,
        walls,
        roof_support,
        floor_support,
        floor_height,
        xy,
        body,
        equivalences,
        operations: work.used,
    })
}
