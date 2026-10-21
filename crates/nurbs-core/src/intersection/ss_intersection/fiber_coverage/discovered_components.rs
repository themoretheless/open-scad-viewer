//! Automatic PARAMETER components of a complete discovered original root set.
//! A shared fixed-parameter root proves a connection, not physical branch
//! orientation, world embedding, periodic identification or trim topology.
use super::*;
use std::sync::Arc;
#[derive(Clone, Copy, Debug)]
pub struct DiscoveredComponentLimits {
    pub max_rectangles: usize,
    pub max_bridges: usize,
}
impl Default for DiscoveredComponentLimits {
    fn default() -> Self {
        Self {
            max_rectangles: 100000,
            max_bridges: 64,
        }
    }
}
#[derive(Clone, Debug)]
pub struct OriginalFiberPointConnection {
    charts: [usize; 2],
    point: QualifiedFiber,
}
impl OriginalFiberPointConnection {
    pub fn charts(&self) -> [usize; 2] {
        self.charts
    }
    /// A fresh original uniform-root certificate on a singleton free interval.
    pub fn point(&self) -> &QualifiedFiber {
        &self.point
    }
    fn matches(&self, other: &Self) -> bool {
        self.charts == other.charts
            && self.point.domain() == other.point.domain()
            && self.point.free_axis() == other.point.free_axis()
            && self.point.report() == other.point.report()
            && self.point.source().first() == other.point.source().first()
            && self.point.source().second() == other.point.source().second()
    }
}
#[derive(Clone, Debug)]
struct Data {
    bridges: Vec<OriginalFiberPointConnection>,
    components: Vec<Vec<usize>>,
    separations: Vec<FiberSeparation>,
    rectangles: usize,
}
#[derive(Clone, Debug)]
pub struct QualifiedDiscoveredFiberComponents {
    owner: Arc<QualifiedDiscoveredFiberAtlas>,
    limits: DiscoveredComponentLimits,
    data: Data,
}
pub struct DiscoveredFiberComponentsResult {
    components: Option<QualifiedDiscoveredFiberComponents>,
    reason: &'static str,
    rectangles: usize,
}
impl DiscoveredFiberComponentsResult {
    pub fn components(&self) -> Option<&QualifiedDiscoveredFiberComponents> {
        self.components.as_ref()
    }
    pub fn into_components(self) -> Option<QualifiedDiscoveredFiberComponents> {
        self.components
    }
    pub fn reason(&self) -> &'static str {
        self.reason
    }
    pub fn rectangles(&self) -> usize {
        self.rectangles
    }
}
fn root_box(f: &QualifiedFiber) -> Option<Box4> {
    let mut out = f.domain();
    let mut j = 0;
    let image = f.report().root_image?;
    for a in 0..4 {
        if a != f.free_axis().index() {
            out[a] = image[j];
            j += 1;
        }
    }
    Some(out)
}
fn separate(a: Box4, b: Box4) -> Option<usize> {
    (0..4).find(|&k| a[k][1] < b[k][0] || b[k][1] < a[k][0])
}
fn point_connection(
    source: &FiberSources,
    first: &QualifiedFiber,
    second: &QualifiedFiber,
    work: &mut Work,
) -> Result<Option<QualifiedFiber>> {
    let a = first.domain();
    let b = second.domain();
    let common: Box4 = std::array::from_fn(|k| [a[k][0].max(b[k][0]), a[k][1].min(b[k][1])]);
    if common.iter().any(|d| d[0] > d[1]) {
        return Ok(None);
    }
    let candidates = [
        first.free_axis(),
        second.free_axis(),
        FiberAxis::FirstU,
        FiberAxis::FirstV,
        FiberAxis::SecondU,
        FiberAxis::SecondV,
    ];
    let mut seen = [false; 4];
    for axis in candidates {
        let free = axis.index();
        if seen[free] {
            continue;
        }
        seen[free] = true;
        if (0..4).any(|k| k != free && common[k][0] >= common[k][1]) {
            continue;
        }
        let mid = common[free][0] * 0.5 + common[free][1] * 0.5;
        let raw_values = [mid, common[free][0], common[free][1]];
        let mut prev = f64::NAN;
        for value in raw_values {
            if value == prev {
                continue;
            }
            prev = value;
            if work.used == work.max {
                return Ok(None);
            }
            let mut domain = common;
            domain[free] = [value, value];
            let r = source.certify(
                domain,
                axis,
                FiberLimits {
                    max_rectangles: work.max - work.used,
                    preconditioner: None,
                },
            )?;
            work.used += r.report().rectangles;
            if r.report().reason == "rectangle-budget" {
                return Ok(None);
            }
            if let Some(point) = r.into_fiber() {
                return Ok(Some(point));
            }
        }
    }
    Ok(None)
}
fn construct(
    owner: &Arc<QualifiedDiscoveredFiberAtlas>,
    limits: DiscoveredComponentLimits,
) -> Result<(Option<Data>, &'static str, usize)> {
    check(
        (1..=100000).contains(&limits.max_rectangles) && (1..=256).contains(&limits.max_bridges),
        "Need bounded original component proof work",
    )?;
    let mut work = Work {
        used: 0,
        max: limits.max_rectangles,
    };
    if owner.replay(&mut work).is_err() {
        return Ok((None, "complete-atlas-replay-unproven", work.used));
    }
    let fs = owner.fibers();
    let mut labels = (0..fs.len()).collect::<Vec<_>>();
    let mut bridges = Vec::new();
    let boxes = fs.iter().map(root_box).collect::<Option<Vec<_>>>();
    let Some(boxes) = boxes else {
        return Ok((None, "original-root-enclosures-unproven", work.used));
    };
    for i in 0..fs.len() {
        for j in i + 1..fs.len() {
            if labels[i] == labels[j] || separate(boxes[i], boxes[j]).is_some() {
                continue;
            }
            if work.used == work.max {
                return Ok((None, "point-connection-work-unproven", work.used));
            }
            if let Some(point) = point_connection(owner.source(), &fs[i], &fs[j], &mut work)? {
                if bridges.len() == limits.max_bridges {
                    return Ok((None, "point-connection-count-unproven", work.used));
                }
                // The fresh point box is inside both actual chart boxes. Its
                // original root therefore belongs to both uniform root graphs;
                // no matching approximate endpoint or coordinate hash is used.
                bridges.push(OriginalFiberPointConnection {
                    charts: [i, j],
                    point,
                });
                let low = labels[i].min(labels[j]);
                let high = labels[i].max(labels[j]);
                for label in &mut labels {
                    if *label == high {
                        *label = low;
                    }
                }
            }
        }
    }
    let mut separations = Vec::new();
    for i in 0..fs.len() {
        for j in i + 1..fs.len() {
            if labels[i] != labels[j] {
                let Some(axis) = separate(boxes[i], boxes[j]) else {
                    return Ok((
                        None,
                        "cross-component-original-root-contact-unproven",
                        work.used,
                    ));
                };
                separations.push(FiberSeparation {
                    charts: [i, j],
                    axis,
                    intervals: [boxes[i][axis], boxes[j][axis]],
                });
            }
        }
    }
    let components = (0..fs.len())
        .filter(|&i| labels[i] == i)
        .map(|i| (0..fs.len()).filter(|&j| labels[j] == i).collect())
        .collect();
    Ok((
        Some(Data {
            bridges,
            components,
            separations,
            rectangles: work.used,
        }),
        "complete-original-parameter-components",
        work.used,
    ))
}
impl QualifiedDiscoveredFiberComponents {
    pub fn owner(&self) -> &Arc<QualifiedDiscoveredFiberAtlas> {
        &self.owner
    }
    pub fn connections(&self) -> &[OriginalFiberPointConnection] {
        &self.data.bridges
    }
    pub fn components(&self) -> &[Vec<usize>] {
        &self.data.components
    }
    pub fn separations(&self) -> &[FiberSeparation] {
        &self.data.separations
    }
    pub fn rectangles(&self) -> usize {
        self.data.rectangles
    }
    /// Fresh original atlas and every connection proposal share one allowance.
    /// Every pair in different connected unions must have a strict original
    /// root-box separation, yielding the complete parameter component census.
    pub fn reverify(&self) -> Result<()> {
        let (data, _, _) = construct(&self.owner, self.limits)?;
        let data = data
            .ok_or_else(|| crate::input("Original automatic component replay remains unproven"))?;
        check(
            data.components == self.data.components
                && data.separations == self.data.separations
                && data.rectangles == self.data.rectangles
                && data.bridges.len() == self.data.bridges.len()
                && data
                    .bridges
                    .iter()
                    .zip(&self.data.bridges)
                    .all(|(a, b)| a.matches(b)),
            "Original point connections, parameter components or separation census changed",
        )
    }
}
pub fn connect_discovered_fiber_atlas(
    owner: Arc<QualifiedDiscoveredFiberAtlas>,
    limits: DiscoveredComponentLimits,
) -> Result<DiscoveredFiberComponentsResult> {
    let (data, reason, rectangles) = construct(&owner, limits)?;
    Ok(DiscoveredFiberComponentsResult {
        components: data.map(|data| QualifiedDiscoveredFiberComponents {
            owner,
            limits,
            data,
        }),
        reason,
        rectangles,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_component_forest_and_cross_separations_need_fresh_original_replay() {
        let a = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: [0.25, 0.5]
                .into_iter()
                .map(|x| {
                    [0.1875, -0.3125, 0.1875]
                        .into_iter()
                        .enumerate()
                        .map(|(j, z)| vec![x, j as f64 / 2., z])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let b = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let atlas = Arc::new(
            discover_fiber_surfaces(
                &FiberSources::new(&a, &b).unwrap(),
                FiberDiscoveryLimits::default(),
            )
            .unwrap()
            .into_atlas()
            .unwrap(),
        );
        let o = connect_discovered_fiber_atlas(atlas, DiscoveredComponentLimits::default())
            .unwrap()
            .into_components()
            .unwrap();
        let mut bad = o.clone();
        bad.data.components.clear();
        assert!(bad.reverify().is_err());
        let mut bad = o.clone();
        bad.data.bridges[0].charts[0] = usize::MAX;
        assert!(bad.reverify().is_err());
        let mut bad = o.clone();
        bad.data.separations[0].intervals[0] = [0., 1.];
        assert!(bad.reverify().is_err());
        let mut bad = o;
        bad.limits.max_rectangles = 1;
        assert!(bad.reverify().is_err());
    }
}
