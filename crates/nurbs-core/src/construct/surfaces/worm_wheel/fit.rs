//! Source-bound C0 quintic tensor patches with sixth-derivative interpolation bounds.
use super::*;
use super::{
    chart::validate,
    interpolation::{NODES, Operator, operator, transform},
};
use crate::{distance_bounds::box_distance, surface::Surface};
#[derive(Clone, Debug, PartialEq)]
struct Yaw {
    period: usize,
    teeth: usize,
    cos: I,
    sin: I,
}
impl Yaw {
    fn new(period: usize, teeth: usize) -> Result<Self> {
        check(
            (12..=256).contains(&teeth) && period < teeth,
            "Nominal wheel yaw requires a valid original period/count",
        )?;
        let (sin, cos) = if period == 0 {
            (I::point(0.), I::point(1.))
        } else {
            let theta = crate::involute::certified_pi()?
                .mul(I::point(-2. * period as f64))?
                .div(I::point(teeth as f64))?;
            crate::involute::certified_trig(theta)?
        };
        Ok(Self {
            period,
            teeth,
            cos,
            sin,
        })
    }
    fn rotate(&self, p: [I; 3]) -> Result<[I; 3]> {
        Ok([
            self.cos.mul(p[0])?.sub(self.sin.mul(p[2])?)?,
            p[1],
            self.sin.mul(p[0])?.add(self.cos.mul(p[2])?)?,
        ])
    }
}
#[derive(Clone)]
enum Source {
    Original(Spec, Flank),
    Stock(StockIntersection),
    Root(RootChart),
    AxialRoot(AxialRootChart),
    AxialStock(AxialStockIntersection),
    AxialTip(AxialTipChart),
    Placed(Box<Source>, Yaw),
}
impl Source {
    fn physical_y(&self) -> bool {
        match self {
            Self::AxialRoot(_) | Self::AxialStock(_) | Self::AxialTip(_) => true,
            Self::Placed(s, _) => s.physical_y(),
            _ => false,
        }
    }

    fn spec(&self) -> Spec {
        match self {
            Self::Original(s, _) => *s,
            Self::Stock(s) => s.specification(),
            Self::Root(s) => s.specification(),
            Self::AxialRoot(s) => s.specification(),
            Self::AxialStock(s) => s.specification(),
            Self::AxialTip(s) => s.specification(),
            Self::Placed(s, _) => s.spec(),
        }
    }
    fn working_flank(&self) -> Option<Flank> {
        match self {
            Self::Original(_, f) => Some(*f),
            Self::Stock(s) => Some(s.flank()),
            Self::Root(_) | Self::AxialRoot(_) | Self::AxialTip(_) | Self::Placed(_, _) => None,
            Self::AxialStock(s) => Some(s.flank()),
        }
    }
    fn validate(&self, domain: ChartDomain) -> Result<()> {
        match self {
            Self::Original(s, f) => validate(*s, *f, domain),
            Self::Root(s) => s.validate_query(domain),
            Self::AxialRoot(s) => s.validate_query(AxialDomain {
                t: domain.u,
                y: domain.beta,
            }),
            Self::AxialStock(s) => s.validate_query(AxialDomain {
                t: domain.u,
                y: domain.beta,
            }),
            Self::AxialTip(s) => s.validate_query(AxialDomain {
                t: domain.u,
                y: domain.beta,
            }),
            Self::Placed(s, _) => s.validate(domain),
            Self::Stock(s) => {
                check(
                    s.domain().u == [0., 1.]
                        && domain.u.iter().all(|x| x.is_finite())
                        && 0. <= domain.u[0]
                        && domain.u[0] <= domain.u[1]
                        && domain.u[1] <= 1.
                        && s.domain().beta[0] <= domain.beta[0]
                        && domain.beta[0] <= domain.beta[1]
                        && domain.beta[1] <= s.domain().beta[1],
                    "Stock fit requires the full original flank, local U in [0,1] and original angles",
                )?;
                validate(
                    s.specification(),
                    s.flank(),
                    ChartDomain {
                        u: [0., 1.],
                        beta: domain.beta,
                    },
                )
            }
        }
    }
    fn replay(&self) -> Result<()> {
        match self {
            Self::Original(_, _) => Ok(()),
            Self::Stock(s) => s.reverify(),
            Self::Root(s) => s.reverify(),
            Self::AxialRoot(s) => s.reverify(),
            Self::AxialStock(s) => s.reverify(),
            Self::AxialTip(s) => s.reverify(),
            Self::Placed(s, yaw) => {
                check(
                    *yaw == Yaw::new(yaw.period, s.spec().teeth)?,
                    "Original nominal instance yaw changed",
                )?;
                s.replay()
            }
        }
    }
    fn at_beta(&self, beta: [f64; 2]) -> Result<Self> {
        match self {
            Self::Original(s, f) => Ok(Self::Original(*s, *f)),
            Self::Stock(s) => Ok(Self::Stock(s.restrict_beta(beta)?)),
            Self::AxialStock(s) => Ok(Self::AxialStock(s.restrict_y(beta)?)),
            Self::AxialTip(s) => Ok(Self::AxialTip(s.sample_y(beta)?)),
            Self::Placed(s, yaw) => Ok(Self::Placed(Box::new(s.at_beta(beta)?), yaw.clone())),
            Self::AxialRoot(s) => {
                s.validate_query(AxialDomain {
                    t: [0., 1.],
                    y: beta,
                })?;
                Ok(self.clone())
            }
            Self::Root(s) => {
                s.validate_query(ChartDomain { u: [0., 1.], beta })?;
                Ok(self.clone())
            }
        }
    }
    fn point(&self, domain: ChartDomain) -> Result<[[f64; 2]; 3]> {
        self.validate(domain)?;
        match self {
            Self::Original(s, f) => Ok(enclose_chart(*s, *f, domain)?.point()),
            Self::Root(s) => s.position_bounds(domain),
            Self::AxialRoot(s) => s.position_bounds(AxialDomain {
                t: domain.u,
                y: domain.beta,
            }),
            Self::AxialStock(s) => s.position_bounds(AxialDomain {
                t: domain.u,
                y: domain.beta,
            }),
            Self::AxialTip(s) => s.position_bounds(AxialDomain {
                t: domain.u,
                y: domain.beta,
            }),
            Self::Placed(s, yaw) => {
                let p = s.point(domain)?;
                let mut values = [I::point(0.); 3];
                for i in 0..3 {
                    values[i] = iv(p[i])?;
                }
                Ok(yaw.rotate(values)?.map(pair))
            }
            Self::Stock(s) => {
                let root = if s.domain().beta == domain.beta {
                    s.clone()
                } else {
                    s.restrict_beta(domain.beta)?
                };
                let crest = I::point(if s.flank() == Flank::Descending {
                    0.
                } else {
                    1.
                });
                let u = crest.add(iv(root.u_bracket())?.sub(crest)?.mul(iv(domain.u)?)?)?;
                Ok(enclose_chart(
                    s.specification(),
                    s.flank(),
                    ChartDomain {
                        u: [u.lo.max(0.), u.hi.min(1.)],
                        beta: domain.beta,
                    },
                )?
                .point())
            }
        }
    }
    fn cell(&self, domain: ChartDomain) -> Result<(bool, [[I; 3]; 2])> {
        self.validate(domain)?;
        match self {
            Self::Original(s, f) => Ok((
                enclose_chart(*s, *f, domain)?.regularity_proven(),
                [
                    super::taylor::sixth(*s, *f, domain, 0)?,
                    super::taylor::sixth(*s, *f, domain, 1)?,
                ],
            )),
            Self::Root(s) => {
                let r = s.enclose(domain)?;
                let mut sixth = [[I::point(0.); 3]; 2];
                for i in 0..2 {
                    for k in 0..3 {
                        sixth[i][k] = iv(r.sixth[i][k])?;
                    }
                }
                Ok((r.regularity_proven, sixth))
            }
            Self::AxialRoot(s) => {
                let r = s.enclose(AxialDomain {
                    t: domain.u,
                    y: domain.beta,
                })?;
                let mut sixth = [[I::point(0.); 3]; 2];
                for i in 0..2 {
                    for k in 0..3 {
                        sixth[i][k] = iv(r.sixth[i][k])?;
                    }
                }
                Ok((r.regularity_proven, sixth))
            }
            Self::AxialStock(s) => {
                let r = s.enclose(AxialDomain {
                    t: domain.u,
                    y: domain.beta,
                })?;
                let mut sixth = [[I::point(0.); 3]; 2];
                for i in 0..2 {
                    for k in 0..3 {
                        sixth[i][k] = iv(r.sixth[i][k])?;
                    }
                }
                Ok((r.regularity_proven, sixth))
            }
            Self::AxialTip(s) => {
                let r = s.enclose(AxialDomain {
                    t: domain.u,
                    y: domain.beta,
                })?;
                let mut sixth = [[I::point(0.); 3]; 2];
                for i in 0..2 {
                    for k in 0..3 {
                        sixth[i][k] = iv(r.sixth[i][k])?;
                    }
                }
                Ok((r.regularity_proven, sixth))
            }
            Self::Placed(s, yaw) => {
                let (regular, mut sixth) = s.cell(domain)?;
                for axis in &mut sixth {
                    *axis = yaw.rotate(*axis)?;
                }
                // Exact nominal rotation preserves ORIGINAL rank. Actual
                // saved-patch rank is still checked from actual world poles.
                Ok((regular, sixth))
            }
            Self::Stock(s) => {
                let b = s
                    .restrict_beta(domain.beta)?
                    .enclose_trimmed_chart(domain.u)?;
                let mut sixth = [[I::point(0.); 3]; 2];
                for i in 0..2 {
                    for k in 0..3 {
                        sixth[i][k] = iv(b.sixth[i][k])?;
                    }
                }
                Ok((b.regularity_proven, sixth))
            }
        }
    }
}






#[derive(Clone, Debug)]
pub struct GridCertificate {
    pub error_upper: f64,
    pub interpolation_error_upper: f64,
    pub control_difference_upper: f64,
    pub directional_error_upper: [f64; 2],
    pub source_regular: bool,
    pub saved_regular: bool,
    pub patches: usize,
}


/// Independent continuous original-envelope error and actual C0 grid replay.
/// Source and saved-patch regularity are separate, independently recomputed flags.
pub fn certify_envelope_grid(
    spec: Spec,
    flank: Flank,
    domain: ChartDomain,
    u: &[f64],
    beta: &[f64],
    surfaces: &[Surface],
    max_patches: usize,
) -> Result<GridCertificate> {
    verify(
        &Source::Original(spec, flank),
        domain,
        u,
        beta,
        surfaces,
        max_patches,
        &operator()?,
    )
}
fn breaks(range: [f64; 2], n: usize) -> Vec<f64> {
    (0..=n)
        .map(|i| {
            if i == 0 {
                range[0]
            } else if i == n {
                range[1]
            } else {
                range[0] + (range[1] - range[0]) * (i as f64 / n as f64)
            }
        })
        .collect()
}
// Nominal parameter V is physical Y. The saved polynomial may differ from
// exact identity by rounding of the interior 1/5 controls; that error is
// included in the ordinary actual-control/source bound. End planes are exact.
fn physical_y_controls(y: [f64; 2]) -> Result<[f64; 6]> {
    let mut values = [0.; 6];
    values[0] = y[0];
    values[5] = y[1];
    for i in 1..5 {
        values[i] = mid(I::point(y[0]).add(
            I::point(y[1])
                .sub(I::point(y[0]))?
                .mul(I::point(i as f64).div(I::point(5.))?)?,
        )?);
    }
    check(
        values.windows(2).all(|w| w[0] < w[1]),
        "Axial Y controls lost strict parameter progress",
    )?;
    Ok(values)
}



struct QualifiedGrid {
    source: Source,
    domain: ChartDomain,
    u: Vec<f64>,
    beta: Vec<f64>,
    surfaces: Vec<Surface>,
    packed: Vec<Surface>,
    certificate: GridCertificate,
    tolerance: f64,
}



impl QualifiedGrid {
    fn reverify_against(&self, source: &Source) -> Result<()> {
        check(
            same_packed_definitions(&pack(&self.surfaces, &self.u, &self.beta)?, &self.packed),
            "Owned wheel packed control/weight/knot definitions changed",
        )?;
        let fresh = verify(
            source,
            self.domain,
            &self.u,
            &self.beta,
            &self.surfaces,
            self.surfaces.len(),
            &operator()?,
        )?;
        check(
            fresh.error_upper <= self.tolerance
                && fresh.source_regular
                && fresh.saved_regular
                && fresh.error_upper == self.certificate.error_upper,
            "Owned wheel source/grid error receipt changed",
        )
    }
}
macro_rules! grid_accessors {
    () => {
        /// Exact C0 packing retains original saved control bits and domains.
        pub fn packed_surfaces(&self) -> &[Surface] {
            &self.grid.packed
        }
        pub fn surfaces(&self) -> &[Surface] {
            &self.grid.surfaces
        }
        pub fn u_breaks(&self) -> &[f64] {
            &self.grid.u
        }
        pub fn beta_breaks(&self) -> &[f64] {
            &self.grid.beta
        }
        pub fn domain(&self) -> ChartDomain {
            self.grid.domain
        }
        pub fn certificate(&self) -> &GridCertificate {
            &self.grid.certificate
        }
    };
}
/// A working flank grid: original profile U or explicitly owned stock-chart t.
pub struct QualifiedEnvelopeGrid {
    grid: QualifiedGrid,
    flank: Flank,
}
impl QualifiedEnvelopeGrid {
    grid_accessors!();
    pub fn specification(&self) -> Spec {
        self.grid.source.spec()
    }
    pub fn flank(&self) -> Flank {
        self.flank
    }
    pub fn stock_source(&self) -> Option<&StockIntersection> {
        match &self.grid.source {
            Source::Stock(s) => Some(s),
            _ => None,
        }
    }
    pub fn reverify(&self) -> Result<()> {
        check(
            self.grid.source.working_flank() == Some(self.flank),
            "Working flank source role changed",
        )?;
        self.grid.reverify_against(&self.grid.source)
    }
}
/// A root transition/cylinder grid with its own original source role.
/// It cannot be mistaken for a working-flank grid or given a fictitious flank.
pub struct QualifiedRootGrid {
    grid: QualifiedGrid,
    root: RootChart,
}
impl QualifiedRootGrid {
    grid_accessors!();
    pub fn source(&self) -> &RootChart {
        &self.root
    }
    pub fn reverify(&self) -> Result<()> {
        self.grid.reverify_against(&Source::Root(self.root.clone()))
    }
}





/// Fits the true original implicit crest-to-stock chart, including root-isolation
/// roundoff and sixth-order implicit derivative bounds. U is local t in [0,1].
pub fn fit_stock_chart(
    stock: &StockIntersection,
    tolerance: f64,
    max_patches: usize,
) -> Result<QualifiedEnvelopeGrid> {
    let grid = fit(
        Source::Stock(stock.clone()),
        ChartDomain {
            u: [0., 1.],
            beta: stock.domain().beta,
        },
        tolerance,
        max_patches,
    )?;
    Ok(QualifiedEnvelopeGrid {
        grid,
        flank: stock.flank(),
    })
}
/// Recomputes original-stock error and actual grid definitions without trusting a receipt.
pub fn certify_stock_grid(
    stock: &StockIntersection,
    domain: ChartDomain,
    u: &[f64],
    beta: &[f64],
    surfaces: &[Surface],
    max_patches: usize,
) -> Result<GridCertificate> {
    verify(
        &Source::Stock(stock.clone()),
        domain,
        u,
        beta,
        surfaces,
        max_patches,
        &operator()?,
    )
}

/// Converts a fully qualified original root-contact chart to quintic NURBS.
pub fn fit_root_chart(
    root: &RootChart,
    tolerance: f64,
    max_patches: usize,
) -> Result<QualifiedRootGrid> {
    let grid = fit(
        Source::Root(root.clone()),
        root.domain(),
        tolerance,
        max_patches,
    )?;
    Ok(QualifiedRootGrid {
        grid,
        root: root.clone(),
    })
}
/// Continuous original-root error and actual control/weight/knot grid replay.
pub fn certify_root_grid(
    root: &RootChart,
    domain: ChartDomain,
    u: &[f64],
    beta: &[f64],
    surfaces: &[Surface],
    max_patches: usize,
) -> Result<GridCertificate> {
    verify(
        &Source::Root(root.clone()),
        domain,
        u,
        beta,
        surfaces,
        max_patches,
        &operator()?,
    )
}
pub fn verify_packed_root_grid(source: &QualifiedRootGrid, packed: &[Surface]) -> Result<()> {
    source.reverify()?;
    check(
        same_packed_definitions(
            &pack(&source.grid.surfaces, &source.grid.u, &source.grid.beta)?,
            packed,
        ),
        "Packed root surfaces do not retain the qualified original cells",
    )
}

#[cfg(test)]
#[path = "fit/tests/root_ownership_tests_integration.rs"]
mod root_ownership_tests;

/// Root grid with physical-Y coordinates and actual planar end boundaries.
pub struct QualifiedAxialRootGrid {
    grid: QualifiedGrid,
    root: AxialRootChart,
}
impl QualifiedAxialRootGrid {
    pub fn source(&self) -> &AxialRootChart {
        &self.root
    }
    pub fn domain(&self) -> AxialDomain {
        AxialDomain {
            t: self.grid.domain.u,
            y: self.grid.domain.beta,
        }
    }
    pub fn surfaces(&self) -> &[Surface] {
        &self.grid.surfaces
    }
    pub fn packed_surfaces(&self) -> &[Surface] {
        &self.grid.packed
    }
    pub fn t_breaks(&self) -> &[f64] {
        &self.grid.u
    }
    pub fn y_breaks(&self) -> &[f64] {
        &self.grid.beta
    }
    pub fn certificate(&self) -> &GridCertificate {
        &self.grid.certificate
    }
    pub fn reverify(&self) -> Result<()> {
        self.grid
            .reverify_against(&Source::AxialRoot(self.root.clone()))
    }
}




/// Original crest-to-stock working flank between actual physical Y planes.
pub struct QualifiedAxialStockGrid {
    grid: QualifiedGrid,
    stock: AxialStockIntersection,
}
impl QualifiedAxialStockGrid {
    pub fn source(&self) -> &AxialStockIntersection {
        &self.stock
    }
    pub fn domain(&self) -> AxialDomain {
        AxialDomain {
            t: self.grid.domain.u,
            y: self.grid.domain.beta,
        }
    }
    pub fn surfaces(&self) -> &[Surface] {
        &self.grid.surfaces
    }
    pub fn packed_surfaces(&self) -> &[Surface] {
        &self.grid.packed
    }
    pub fn t_breaks(&self) -> &[f64] {
        &self.grid.u
    }
    pub fn y_breaks(&self) -> &[f64] {
        &self.grid.beta
    }
    pub fn certificate(&self) -> &GridCertificate {
        &self.grid.certificate
    }
    pub fn reverify(&self) -> Result<()> {
        self.grid
            .reverify_against(&Source::AxialStock(self.stock.clone()))
    }
}




pub struct QualifiedAxialTipGrid {
    grid: QualifiedGrid,
    tip: AxialTipChart,
}
impl QualifiedAxialTipGrid {
    pub fn source(&self) -> &AxialTipChart {
        &self.tip
    }
    pub fn domain(&self) -> AxialDomain {
        AxialDomain {
            t: self.grid.domain.u,
            y: self.grid.domain.beta,
        }
    }
    pub fn surfaces(&self) -> &[Surface] {
        &self.grid.surfaces
    }
    pub fn packed_surfaces(&self) -> &[Surface] {
        &self.grid.packed
    }
    pub fn t_breaks(&self) -> &[f64] {
        &self.grid.u
    }
    pub fn y_breaks(&self) -> &[f64] {
        &self.grid.beta
    }
    pub fn certificate(&self) -> &GridCertificate {
        &self.grid.certificate
    }
    pub fn reverify(&self) -> Result<()> {
        self.grid
            .reverify_against(&Source::AxialTip(self.tip.clone()))
    }
}




#[cfg(test)]
#[path = "fit/tests/axial_source_ownership_tests_integration.rs"]
mod axial_source_ownership_tests;


/// Original-source owner for a compatible or subsequently edited axial grid.
pub struct QualifiedAxialGrid {
    grid: QualifiedGrid,
    instance: AxialInstance,
}
impl QualifiedAxialGrid {
    pub fn instance(&self) -> &AxialInstance {
        &self.instance
    }
    pub fn source(&self) -> &AxialSource {
        self.instance.source()
    }
    pub fn domain(&self) -> AxialDomain {
        self.instance.domain()
    }
    pub fn surfaces(&self) -> &[Surface] {
        &self.grid.surfaces
    }
    pub fn packed_surfaces(&self) -> &[Surface] {
        &self.grid.packed
    }
    pub fn t_breaks(&self) -> &[f64] {
        &self.grid.u
    }
    pub fn y_breaks(&self) -> &[f64] {
        &self.grid.beta
    }
    pub fn certificate(&self) -> &GridCertificate {
        &self.grid.certificate
    }
    pub fn reverify(&self) -> Result<()> {
        self.grid
            .reverify_against(&instance_source(&self.instance)?)
    }
}
/// Admits ACTUAL saved poles after editing/stitching, including exact Y-plane
/// controls, complete original-domain coverage and unchanged world tolerance.








#[path = "fit/axial_fitting.rs"]
mod axial_fitting;
pub use axial_fitting::fit_axial_root;
pub use axial_fitting::certify_axial_root_grid;
pub use axial_fitting::verify_packed_axial_root;
pub use axial_fitting::fit_axial_stock;
pub use axial_fitting::certify_axial_stock_grid;
pub use axial_fitting::verify_packed_axial_stock;
pub use axial_fitting::fit_axial_tip;
pub use axial_fitting::certify_axial_tip_grid;
pub use axial_fitting::verify_packed_axial_tip;
use axial_fitting::axial_source;
pub use axial_fitting::qualify_axial_grid;
pub use axial_fitting::regrid_axial_source;
use axial_fitting::instance_source;
pub use axial_fitting::qualify_axial_instance;
pub use axial_fitting::regrid_axial_instance;
pub use axial_fitting::place_axial_surfaces;

#[path = "fit/grid_packing.rs"]
mod grid_packing;
use grid_packing::build;
use grid_packing::pack;
pub use grid_packing::verify_packed_grid;
use grid_packing::same_scalar_bits;
use grid_packing::same_row_bits;
use grid_packing::same_packed_definitions;

#[path = "fit/grid_verification.rs"]
mod grid_verification;
use grid_verification::verify;
use grid_verification::validate_layout;
use grid_verification::saved_regular;
use grid_verification::interpolation_error;
use grid_verification::node;
use grid_verification::ideal_net;
use grid_verification::norm;
use grid_verification::sixth_power;
pub use grid_verification::fit_chart;
use grid_verification::fit;
