use super::*;

/// Source-relative uniqueness on a closed original UV4 window. This proves
/// membership of any original SS root in the window, not existence of a root
/// at every point of the window, nor simplicity of a wire.
#[derive(Clone, Debug)]
pub struct QualifiedOriginalSsGraphWindow {
    curve: Arc<OriginalSsRootCurve>,
    original: [[Q; 2]; 4],
    local: [[Q; 2]; 4],
    subarc: Option<(OriginalSsRootCurve, [f64; 2], OriginalSsCurveLimits)>,
}
impl QualifiedOriginalSsGraphWindow {
    pub fn curve(&self) -> &Arc<OriginalSsRootCurve> {
        &self.curve
    }
    pub fn original_window(&self) -> &[[Q; 2]; 4] {
        &self.original
    }
    pub fn local_window(&self) -> &[[Q; 2]; 4] {
        &self.local
    }
    pub fn reverify(&self) -> Result<()> {
        self.curve.reverify()?;
        if let Some((source, parameter, limits)) = &self.subarc {
            source.reverify()?;
            let fresh = self
                .curve
                .qualify_subarc_window(source, *parameter, *limits)?;
            check(
                fresh.original == self.original,
                "Source subarc window changed",
            )?;
        }
        let fresh = self.curve.qualify_graph_window(self.original.clone())?;
        check(fresh.local == self.local, "Original graph window changed")
    }
}
impl OriginalSsRootCurve {
    /// Prove that a complete source subarc lies in this graph. The source
    /// carrier supplies root existence; a box alone makes no such assertion.
    pub fn qualify_subarc_window(
        self: &Arc<Self>,
        source: &OriginalSsRootCurve,
        parameter: [f64; 2],
        limits: OriginalSsCurveLimits,
    ) -> Result<QualifiedOriginalSsGraphWindow> {
        check(
            self.source().first() == source.source().first()
                && self.source().second() == source.source().second(),
            "Graph-window subarc has different ordered original carriers",
        )?;
        let bounds = source.enclose(parameter, limits)?;
        let original = bounds.uv.map(|d| d.map(|x| Q::from_float(x).unwrap()));
        let mut proof = self.qualify_graph_window(original)?;
        proof.subarc = Some((source.clone(), parameter, limits));
        Ok(proof)
    }

    /// The sealed carrier supplies source-equivalent graph uniqueness. Exact
    /// rational frame containment transfers that authority to original UV4.
    /// No periodic shift or nonlinear blowdown is silently identified here.
    pub fn qualify_graph_window(
        self: &Arc<Self>,
        original: [[Q; 2]; 4],
    ) -> Result<QualifiedOriginalSsGraphWindow> {
        let frame = frames::frame(self)?;
        let domain = self.coordinate_domain();
        let mut local = original.clone();
        for k in 0..4 {
            check(
                original[k][0] <= original[k][1] && frame[k][1] > Q::zero(),
                "Invalid original graph window or affine frame",
            )?;
            for j in 0..2 {
                local[k][j] = (&original[k][j] - &frame[k][0]) / &frame[k][1];
            }
            let lo = Q::from_float(domain[k][0])
                .ok_or_else(|| crate::input("Nonfinite graph domain"))?;
            let hi = Q::from_float(domain[k][1])
                .ok_or_else(|| crate::input("Nonfinite graph domain"))?;
            check(
                lo <= local[k][0] && local[k][1] <= hi,
                "Original window leaves the qualified SS graph",
            )?;
        }
        if let Carrier::Pole(f) = &self.carrier {
            let coordinate = 2 * f.source().pole().support() + f.source().pole().axis();
            let shore = Q::from_integer(f.source().pole().shore().into());
            check(
                local[coordinate][1] < shore || shore < local[coordinate][0],
                "Original graph window touches an excluded pole stratum",
            )?;
        }
        Ok(QualifiedOriginalSsGraphWindow {
            curve: self.clone(),
            original,
            local,
            subarc: None,
        })
    }
}
