//! Exact classification of the principal curvature gap over original cells.
use super::*;
use num_traits::Signed;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UmbilicStatus {
    /// Both principal curvatures agree throughout a regular cell.
    EverywhereUmbilic,
    /// The curvature gap is strictly positive throughout the closed cell.
    EverywhereSeparated,
    /// There is at least one nonumbilic point. The whole-cell sign is unresolved.
    ContainsNonUmbilicPoint,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UmbilicCell {
    pub domain: [[f64; 2]; 2],
    pub status: UmbilicStatus,
}

#[derive(Clone, Debug)]
pub struct UmbilicAudit {
    pub cells: Option<Vec<UmbilicCell>>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Classify the principal curvature gap of each original UV cell using its
/// exact polynomial numerator. The original normal must be proved regular.
/// Results concern each cell's own traces, with no cross-knot smoothness
/// assumed. The audit makes no rounded field and no eigenvector chart.
pub fn audit_umbilics(source: &Surface, max_work: u64) -> Result<UmbilicAudit> {
    source.validate()?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Umbilic audit work must be in [1,1000000]",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| {
        let mut cells = Vec::new();
        for u in source.degree_u..source.control_points.len() {
            let u_domain = [source.knots_u[u], source.knots_u[u + 1]];
            if u_domain[0] == u_domain[1] {
                continue;
            }
            for v in source.degree_v..source.control_points[0].len() {
                let v_domain = [source.knots_v[v], source.knots_v[v + 1]];
                if v_domain[0] == v_domain[1] {
                    continue;
                }
                let target =
                    original_target(source, u, v, Field::SquaredPrincipalCurvatureGap, &mut work)?;
                let numerator = &target[0];
                let (pu, pv) = storage::degrees(std::slice::from_ref(numerator));
                let status = if numerator.is_empty() {
                    UmbilicStatus::EverywhereUmbilic
                } else if pu <= 25
                    && pv <= 25
                    && storage::bernstein(numerator, pu, pv, &mut work)?
                        .iter()
                        .all(Signed::is_positive)
                {
                    UmbilicStatus::EverywhereSeparated
                } else {
                    // A nonzero polynomial cannot vanish on the open rectangle.
                    UmbilicStatus::ContainsNonUmbilicPoint
                };
                cells.push(UmbilicCell {
                    domain: [u_domain, v_domain],
                    status,
                });
            }
        }
        Ok(cells)
    })();
    let (cells, refusal) = match result {
        Ok(cells) => (Some(cells), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(UmbilicAudit {
        cells,
        refusal,
        work_used: work.used,
    })
}
