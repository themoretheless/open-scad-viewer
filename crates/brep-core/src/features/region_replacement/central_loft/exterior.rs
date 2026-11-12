//! Fresh exterior two-jet comparisons on original rational supports.
use super::*;
impl CentralLoftCandidate<'_, '_, '_> {
    /// Cover all original boundary spans, including both owned flat poles.
    /// This is exterior G2 evidence only: no topology graft or global embedding
    /// is inferred. Internal U joins retain their separate native inspection.
    pub fn inspect_exterior_jets(
        &self,
        max_cells: usize,
        max_exact: u64,
    ) -> Result<Vec<nurbs_core::surface_g2_pcurves::ExtrusionBoundaryReport>> {
        self.inspect_surface_exterior_jets(&self.surface, max_cells, max_exact)
    }
    pub(super) fn inspect_surface_exterior_jets(
        &self,
        surface: &Surface,
        max_cells: usize,
        max_exact: u64,
    ) -> Result<Vec<nurbs_core::surface_g2_pcurves::ExtrusionBoundaryReport>> {
        require(
            (18..=100000).contains(&max_cells) && max_exact <= 1000000,
            "Bound central exterior jet work",
        )?;
        let poles = self.inspect_surface_pole_limits(surface, 1. / 64., max_cells / 3)?;
        require(
            poles.iter().all(|p| p.report.proven),
            "Central exterior endpoints need both original pole quotient proofs",
        )?;
        let certificates = [poles[0].certificate.as_ref(), poles[1].certificate.as_ref()];
        let mut used = poles.iter().map(|p| p.report.cells).sum::<usize>();
        let mut exact = 0;
        let mut reports = Vec::new();
        let s = surface;
        let p = s.degree_u;
        let q = s.degree_v;
        for i in 0..6 {
            let pc = &self.pcurves[i];
            let originals = &self.originals[i];
            let report = if i % 2 == 0 {
                let u = (pc.control_points[0][0] + pc.control_points[1][0]) * 0.5;
                let span = (p..s.control_points.len())
                    .find(|&j| s.knots_u[j] <= u && u < s.knots_u[j + 1])
                    .ok_or_else(|| refuse("Original exterior span missing"))?;
                let j = span - p + p / 2;
                let row = if pc.control_points[0][1] == s.knots_v[q] {
                    0
                } else {
                    q
                };
                let next = if row == 0 { 1 } else { q - 1 };
                let axes: Vec<_> = (0..3)
                    .filter(|&k| s.control_points[j][row][k] != s.control_points[j][next][k])
                    .collect();
                require(
                    axes.len() == 1,
                    "Central exterior axis proposal is not unique",
                )?;
                nurbs_core::surface_g2_pcurves::inspect_extrusion_boundary(
                    &self.curves[i],
                    [&originals.1, pc],
                    [&originals.0, s],
                    axes[0],
                    certificates,
                    nurbs_core::surface_g2_pcurves::G2Budget {
                        tolerances: self.options.tolerances,
                        max_cells: (max_cells - used) / (6 - i),
                        max_exact: (max_exact - exact) / (6 - i) as u64,
                    },
                )?
            } else {
                let source = &originals.0;
                let axes: Vec<_> = (0..3)
                    .filter(|&k| {
                        source
                            .control_points
                            .iter()
                            .flatten()
                            .all(|p| p[k] == source.control_points[0][0][k])
                    })
                    .collect();
                require(
                    axes.len() == 1,
                    "Original connector support plane is not unique",
                )?;
                nurbs_core::surface_g2_pcurves::inspect_planar_boundary(
                    &self.curves[i],
                    [&originals.1, pc],
                    [source, s],
                    axes[0],
                    certificates,
                    nurbs_core::surface_g2_pcurves::G2Budget {
                        tolerances: self.options.tolerances,
                        max_cells: (max_cells - used) / (6 - i),
                        max_exact: (max_exact - exact) / (6 - i) as u64,
                    },
                )?
            };
            used += report.cells;
            exact += report.positions.exact_work;
            require(
                used <= max_cells && exact <= max_exact,
                "Central exterior jets exceeded shared work",
            )?;
            reports.push(report);
        }
        Ok(reports)
    }
}

/// Original exterior subinterval owned by one central network patch. A report
/// applies only to this interval; it grants no topology or whole-body authority.
pub struct NetworkExteriorReport {
    pub boundary: usize,
    pub patch: usize,
    pub parameter: [f64; 2],
    pub report: nurbs_core::surface_g2_pcurves::Report,
}
impl CentralLoftCandidate<'_, '_, '_> {
    /// Partition all six original boundary clocks against the actual U cells.
    /// Trim the world curve and BOTH pcurves on the same original clock, then
    /// freshly check source agreement and G2. Never bridge an uncovered span.
    pub fn inspect_network_exterior_jets(
        &self,
        patches: &[Surface],
        max_cells: usize,
        max_exact: u64,
    ) -> Result<Vec<NetworkExteriorReport>> {
        require(
            !patches.is_empty()
                && patches.len() <= 32
                && (18..=100000).contains(&max_cells)
                && max_exact <= 1000000,
            "Bound network exterior work",
        )?;
        let pieces = self.network_exterior_pieces(patches, max_cells)?;
        let count = pieces.len();
        let mut out = Vec::new();
        for (boundary, patch, parameter) in pieces {
            let c = self.curves[boundary].trim(parameter[0], parameter[1])?;
            let source_pc = self.originals[boundary]
                .1
                .trim(parameter[0], parameter[1])?;
            let target_pc = self.pcurves[boundary].trim(parameter[0], parameter[1])?;
            let report = nurbs_core::surface_g2_pcurves::inspect(
                nurbs_core::surface_g2_pcurves::PcurveSeam { curve: &c, pcurves: [&source_pc, &target_pc], surfaces: [&self.originals[boundary].0, &patches[patch]], reversed: [false, false] },
                nurbs_core::surface_g2_pcurves::G2PcurveTolerance { position: self.options.tolerances[0], normal: self.options.tolerances[1], curvature: self.options.tolerances[2] },
                max_cells / count,
                max_exact / count as u64,
            )?;
            out.push(NetworkExteriorReport {
                boundary,
                patch,
                parameter,
                report,
            });
        }
        Ok(out)
    }
    pub(super) fn network_exterior_pieces(
        &self,
        patches: &[Surface],
        max_cells: usize,
    ) -> Result<Vec<(usize, usize, [f64; 2])>> {
        require(
            !patches.is_empty() && patches.len() <= 32,
            "Bound network exterior partition",
        )?;
        let mut pieces = Vec::new();
        for boundary in 0..6 {
            let pc = &self.pcurves[boundary];
            require(
                pc.degree == 1 && pc.control_points.len() == 2 && pc.weights[0] == pc.weights[1],
                "Network rim needs original affine clock",
            )?;
            let [a, b] = [pc.control_points[0][0], pc.control_points[1][0]];
            require(
                a != b && pc.control_points[0][1] == pc.control_points[1][1],
                "Network rim is not a horizontal owned interval",
            )?;
            let d = [pc.knots[pc.degree], pc.knots[pc.control_points.len()]];
            let mut covered = Vec::new();
            for (index, s) in patches.iter().enumerate() {
                s.validate()?;
                let lo = a.min(b).max(s.knots_u[s.degree_u]);
                let hi = a.max(b).min(s.knots_u[s.control_points.len()]);
                if lo >= hi {
                    continue;
                }
                let t0 = ((lo - a) / (b - a)).min((hi - a) / (b - a));
                let t1 = ((lo - a) / (b - a)).max((hi - a) / (b - a));
                let clock = [d[0] + (d[1] - d[0]) * t0, d[0] + (d[1] - d[0]) * t1];
                covered.push([lo, hi]);
                pieces.push((boundary, index, clock));
            }
            covered.sort_by(|x, y| x[0].total_cmp(&y[0]));
            require(
                !covered.is_empty()
                    && covered[0][0] == a.min(b)
                    && covered.last().unwrap()[1] == a.max(b)
                    && covered.windows(2).all(|p| p[0][1] == p[1][0]),
                "Network exterior partition has a gap or overlap",
            )?;
        }
        require(
            pieces.len() <= max_cells,
            "Network exterior cell allowance exhausted",
        )?;
        Ok(pieces)
    }
    /// Specialized exact support recognition on each actual owned interval.
    /// Pole receipts borrow the actual patch, never the old single surface.
    pub fn inspect_network_exterior_support_jets(
        &self,
        patches: &[Surface],
        max_cells: usize,
        max_exact: u64,
    ) -> Result<Vec<NetworkSupportReport>> {
        require(
            (18..=100000).contains(&max_cells) && max_exact <= 1000000,
            "Bound network support work",
        )?;
        let pieces = self.network_exterior_pieces(patches, max_cells)?;
        let count = pieces.len();
        let mut out = Vec::new();
        for (boundary, patch, parameter) in pieces {
            let result = (|| {
                let s = &patches[patch];
                let allowance = max_cells / count;
                let c = self.curves[boundary].trim(parameter[0], parameter[1])?;
                let source_pc = self.originals[boundary]
                    .1
                    .trim(parameter[0], parameter[1])?;
                let target_pc = self.pcurves[boundary].trim(parameter[0], parameter[1])?;
                self.inspect_network_support_piece(
                    s,
                    boundary,
                    &c,
                    &source_pc,
                    &target_pc,
                    allowance,
                    max_exact / count as u64,
                )
            })();
            out.push(NetworkSupportReport {
                boundary,
                patch,
                parameter,
                report: result,
            });
        }
        Ok(out)
    }
    /// Shared exact-support audit used both before sewing and on FINAL uses.
    pub(super) fn inspect_network_support_piece(
        &self,
        s: &Surface,
        boundary: usize,
        c: &Curve,
        source_pc: &Curve,
        target_pc: &Curve,
        allowance: usize,
        max_exact: u64,
    ) -> Result<nurbs_core::surface_g2_pcurves::ExtrusionBoundaryReport> {
        require(
            boundary < 6 && allowance <= 100000 && max_exact <= 1000000,
            "Bound final network support audit",
        )?;
        let mut poles = Vec::new();
        let mut used = 0;
        for end in [false, true] {
            let row = if end { s.control_points.len() - 1 } else { 0 };
            let point = &s.control_points[row][0];
            if !s.control_points[row].iter().all(|p| p == point) {
                continue;
            }
            let axis = (0..3)
                .min_by(|&a, &b| {
                    let d = |k: usize| {
                        (point[k] - self.stock[k][0])
                            .abs()
                            .min((point[k] - self.stock[k][1]).abs())
                    };
                    d(a).total_cmp(&d(b))
                })
                .unwrap();
            let low = s
                .weights
                .iter()
                .flatten()
                .copied()
                .fold(f64::INFINITY, f64::min);
            let high = s.weights.iter().flatten().copied().fold(0., f64::max);
            let mut band = (low / high) / 64.;
            let proof = loop {
                let proof = nurbs_core::surface_shape_operator::inspect_original_flat_pole(
                    s,
                    end,
                    axis,
                    band,
                    (allowance / 2 - used).min(512),
                )?;
                used += proof.report.cells;
                if proof.report.proven
                    || proof.report.cells == 0
                    || used == allowance / 2
                    || band * 0.5 == 0.
                {
                    break proof;
                }
                band *= 0.5;
            };
            require(
                proof.report.proven,
                "Actual network pole quotient remains unproven",
            )?;
            poles.push((end, proof));
        }
        let receipts = std::array::from_fn(|e| {
            poles
                .iter()
                .find(|(end, _)| usize::from(*end) == e)
                .and_then(|(_, p)| p.certificate.as_ref())
        });
        let source = &self.originals[boundary].0;
        let mut report = if boundary % 2 == 0 {
            let q = s.degree_v;
            let row = if target_pc.control_points[0][1] == s.knots_v[q] {
                0
            } else {
                q
            };
            let next = if row == 0 { 1 } else { q - 1 };
            let i = s.degree_u / 2;
            let axes = (0..3)
                .filter(|&k| s.control_points[i][row][k] != s.control_points[i][next][k])
                .collect::<Vec<_>>();
            require(axes.len() == 1, "Actual network collar axis is not unique")?;
            nurbs_core::surface_g2_pcurves::inspect_extrusion_boundary(
                c,
                [source_pc, target_pc],
                [source, s],
                axes[0],
                receipts,
                nurbs_core::surface_g2_pcurves::G2Budget {
                    tolerances: self.options.tolerances,
                    max_cells: allowance - used,
                    max_exact,
                },
            )
        } else {
            let axes = (0..3)
                .filter(|&k| {
                    source
                        .control_points
                        .iter()
                        .flatten()
                        .all(|p| p[k] == source.control_points[0][0][k])
                })
                .collect::<Vec<_>>();
            require(
                axes.len() == 1,
                "Actual original support plane is not unique",
            )?;
            nurbs_core::surface_g2_pcurves::inspect_planar_boundary(
                c,
                [source_pc, target_pc],
                [source, s],
                axes[0],
                receipts,
                nurbs_core::surface_g2_pcurves::G2Budget {
                    tolerances: self.options.tolerances,
                    max_cells: allowance - used,
                    max_exact,
                },
            )
        }?;
        report.cells += used;
        Ok(report)
    }
}

/// Failure remains attached to its owned original interval, with no admission.
pub struct NetworkSupportReport {
    pub boundary: usize,
    pub patch: usize,
    pub parameter: [f64; 2],
    pub report: Result<nurbs_core::surface_g2_pcurves::ExtrusionBoundaryReport>,
}
