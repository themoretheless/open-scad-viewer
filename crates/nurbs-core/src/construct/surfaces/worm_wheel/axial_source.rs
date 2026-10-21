//! Uniform original side-source ownership for compatible grids and assembly.
use super::*;

#[derive(Clone, Debug)]
pub enum AxialSource {
    Root(AxialRootChart),
    Working(AxialStockIntersection),
    Tip(AxialTipChart),
}
impl AxialSource {
    pub fn specification(&self) -> Spec {
        match self {
            Self::Root(s) => s.specification(),
            Self::Working(s) => s.specification(),
            Self::Tip(s) => s.specification(),
        }
    }
    pub fn domain(&self) -> AxialDomain {
        match self {
            Self::Root(s) => s.domain(),
            Self::Working(s) => s.domain(),
            Self::Tip(s) => s.domain(),
        }
    }
    pub fn reverify(&self) -> Result<()> {
        match self {
            Self::Root(s) => s.reverify(),
            Self::Working(s) => s.reverify(),
            Self::Tip(s) => s.reverify(),
        }
    }
}

/// Original source at exact nominal tooth-period angle -2*pi*period/teeth.
/// Does not replace the original axial phase with a rounded multiple of pi.
#[derive(Clone, Debug)]
pub struct AxialInstance {
    source: AxialSource,
    period: usize,
}
impl AxialInstance {
    pub fn source(&self) -> &AxialSource {
        &self.source
    }
    pub fn period(&self) -> usize {
        self.period
    }
    pub fn domain(&self) -> AxialDomain {
        self.source.domain()
    }
    pub fn reverify(&self) -> Result<()> {
        check(
            self.period < self.source.specification().teeth,
            "Tooth instance period is outside its original count",
        )?;
        self.source.reverify()
    }
}
pub fn axial_instance(source: &AxialSource, period: usize) -> Result<AxialInstance> {
    check(
        period < source.specification().teeth,
        "Tooth instance period is outside its original count",
    )?;
    Ok(AxialInstance {
        source: source.clone(),
        period,
    })
}
