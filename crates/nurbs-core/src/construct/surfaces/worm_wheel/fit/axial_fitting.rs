//! Axial fitting: preserves the original arithmetic and proof budgets.
use super::*;

pub fn fit_axial_root(
    root: &AxialRootChart,
    tolerance: f64,
    max_patches: usize,
) -> Result<QualifiedAxialRootGrid> {
    let d = root.domain();
    let grid = fit(
        Source::AxialRoot(root.clone()),
        ChartDomain { u: d.t, beta: d.y },
        tolerance,
        max_patches,
    )?;
    Ok(QualifiedAxialRootGrid {
        grid,
        root: root.clone(),
    })
}

pub fn certify_axial_root_grid(
    root: &AxialRootChart,
    d: AxialDomain,
    t: &[f64],
    y: &[f64],
    surfaces: &[Surface],
    max_patches: usize,
) -> Result<GridCertificate> {
    verify(
        &Source::AxialRoot(root.clone()),
        ChartDomain { u: d.t, beta: d.y },
        t,
        y,
        surfaces,
        max_patches,
        &operator()?,
    )
}

pub fn verify_packed_axial_root(source: &QualifiedAxialRootGrid, packed: &[Surface]) -> Result<()> {
    source.reverify()?;
    check(
        same_packed_definitions(
            &pack(&source.grid.surfaces, &source.grid.u, &source.grid.beta)?,
            packed,
        ),
        "Packed axial root does not preserve actual saved definitions",
    )
}

pub fn fit_axial_stock(
    stock: &AxialStockIntersection,
    tolerance: f64,
    max_patches: usize,
) -> Result<QualifiedAxialStockGrid> {
    let d = stock.domain();
    let grid = fit(
        Source::AxialStock(stock.clone()),
        ChartDomain { u: d.t, beta: d.y },
        tolerance,
        max_patches,
    )?;
    Ok(QualifiedAxialStockGrid {
        grid,
        stock: stock.clone(),
    })
}

pub fn certify_axial_stock_grid(
    stock: &AxialStockIntersection,
    d: AxialDomain,
    t: &[f64],
    y: &[f64],
    surfaces: &[Surface],
    max_patches: usize,
) -> Result<GridCertificate> {
    verify(
        &Source::AxialStock(stock.clone()),
        ChartDomain { u: d.t, beta: d.y },
        t,
        y,
        surfaces,
        max_patches,
        &operator()?,
    )
}

pub fn verify_packed_axial_stock(
    source: &QualifiedAxialStockGrid,
    packed: &[Surface],
) -> Result<()> {
    source.reverify()?;
    check(
        same_packed_definitions(
            &pack(&source.grid.surfaces, &source.grid.u, &source.grid.beta)?,
            packed,
        ),
        "Packed axial stock does not preserve actual saved definitions",
    )
}

pub fn fit_axial_tip(
    tip: &AxialTipChart,
    tolerance: f64,
    max_patches: usize,
) -> Result<QualifiedAxialTipGrid> {
    let d = tip.domain();
    let grid = fit(
        Source::AxialTip(tip.clone()),
        ChartDomain { u: d.t, beta: d.y },
        tolerance,
        max_patches,
    )?;
    Ok(QualifiedAxialTipGrid {
        grid,
        tip: tip.clone(),
    })
}

pub fn certify_axial_tip_grid(
    tip: &AxialTipChart,
    d: AxialDomain,
    t: &[f64],
    y: &[f64],
    surfaces: &[Surface],
    max_patches: usize,
) -> Result<GridCertificate> {
    verify(
        &Source::AxialTip(tip.clone()),
        ChartDomain { u: d.t, beta: d.y },
        t,
        y,
        surfaces,
        max_patches,
        &operator()?,
    )
}

pub fn verify_packed_axial_tip(source: &QualifiedAxialTipGrid, packed: &[Surface]) -> Result<()> {
    source.reverify()?;
    check(
        same_packed_definitions(
            &pack(&source.grid.surfaces, &source.grid.u, &source.grid.beta)?,
            packed,
        ),
        "Packed axial tip does not preserve actual saved definitions",
    )
}

pub(super) fn axial_source(source: &AxialSource) -> Source {
    match source {
        AxialSource::Root(s) => Source::AxialRoot(s.clone()),
        AxialSource::Working(s) => Source::AxialStock(s.clone()),
        AxialSource::Tip(s) => Source::AxialTip(s.clone()),
    }
}

pub fn qualify_axial_grid(
    source: &AxialSource,
    t: Vec<f64>,
    y: Vec<f64>,
    surfaces: Vec<Surface>,
    tolerance: f64,
    max_patches: usize,
) -> Result<QualifiedAxialGrid> {
    qualify_axial_instance(
        &axial_instance(source, 0)?,
        t,
        y,
        surfaces,
        tolerance,
        max_patches,
    )
}

pub fn regrid_axial_source(
    source: &AxialSource,
    t: Vec<f64>,
    y: Vec<f64>,
    tolerance: f64,
    max_patches: usize,
) -> Result<QualifiedAxialGrid> {
    regrid_axial_instance(&axial_instance(source, 0)?, t, y, tolerance, max_patches)
}

pub(super) fn instance_source(instance: &AxialInstance) -> Result<Source> {
    Ok(Source::Placed(
        Box::new(axial_source(instance.source())),
        Yaw::new(instance.period(), instance.source().specification().teeth)?,
    ))
}

pub fn qualify_axial_instance(
    instance: &AxialInstance,
    t: Vec<f64>,
    y: Vec<f64>,
    surfaces: Vec<Surface>,
    tolerance: f64,
    max_patches: usize,
) -> Result<QualifiedAxialGrid> {
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Axial admission needs finite positive tolerance",
    )?;
    let d = instance.domain();
    let domain = ChartDomain { u: d.t, beta: d.y };
    let source = instance_source(instance)?;
    let certificate = verify(
        &source,
        domain,
        &t,
        &y,
        &surfaces,
        max_patches,
        &operator()?,
    )?;
    check(
        certificate.error_upper <= tolerance
            && certificate.source_regular
            && certificate.saved_regular,
        "Actual axial instance grid does not qualify original source error and source/saved rank",
    )?;
    let packed = pack(&surfaces, &t, &y)?;
    Ok(QualifiedAxialGrid {
        instance: instance.clone(),
        grid: QualifiedGrid {
            source,
            domain,
            u: t,
            beta: y,
            surfaces,
            packed,
            certificate,
            tolerance,
        },
    })
}

pub fn regrid_axial_instance(
    instance: &AxialInstance,
    t: Vec<f64>,
    y: Vec<f64>,
    tolerance: f64,
    max_patches: usize,
) -> Result<QualifiedAxialGrid> {
    let d = instance.domain();
    let domain = ChartDomain { u: d.t, beta: d.y };
    let count = t
        .len()
        .checked_sub(1)
        .and_then(|n| y.len().checked_sub(1).and_then(|m| n.checked_mul(m)))
        .ok_or_else(|| crate::input("Axial regrid has an invalid cell census"))?;
    validate_layout(domain, &t, &y, count, max_patches)?;
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Axial regrid needs finite positive tolerance",
    )?;
    let s = instance_source(instance)?;
    s.validate(domain)?;
    s.replay()?;
    let surfaces = build(&s, &t, &y, &operator()?)?;
    qualify_axial_instance(instance, t, y, surfaces, tolerance, max_patches)
}

/// Candidate placement of saved template definitions. The returned poles are
/// NOT admitted until qualify_axial_instance checks the exact nominal source.
pub fn place_axial_surfaces(
    instance: &AxialInstance,
    surfaces: &[Surface],
) -> Result<Vec<Surface>> {
    let yaw = Yaw::new(instance.period(), instance.source().specification().teeth)?;
    let c = mid(yaw.cos);
    let sn = mid(yaw.sin);
    let mut output = surfaces.to_vec();
    for s in &mut output {
        s.validate()?;
        for row in &mut s.control_points {
            for p in row {
                check(
                    p.len() == 3,
                    "Axial placement requires three-dimensional poles",
                )?;
                let x = c * p[0] - sn * p[2];
                let z = sn * p[0] + c * p[2];
                check(
                    x.is_finite() && z.is_finite(),
                    "Axial placed poles exceeded numeric range",
                )?;
                p[0] = x;
                p[2] = z;
            }
        }
    }
    Ok(output)
}
