//! Bounded orthographic painter BSP and linear-attribute triangle clipping.
//! Nodes use arena indices, so arbitrarily deep leaves require no recursion.
use crate::{Result, fail};
use math_core::{cross, sub};
pub type Triangle = [Vec<f64>; 3];
pub type Plane = [f64; 4];
#[derive(Clone, Debug)]
pub struct Fragment {
    pub owner: usize,
    pub triangle: Triangle,
}
#[derive(Default, Debug)]
pub struct Parts {
    pub front: Vec<Triangle>,
    pub back: Vec<Triangle>,
    pub coplanar: Vec<Triangle>,
}
#[derive(Debug)]
pub struct Node {
    pub plane: Plane,
    pub fragments: Vec<Fragment>,
    pub front: Option<usize>,
    pub back: Option<usize>,
}
#[derive(Debug, Default)]
pub struct Bsp {
    pub nodes: Vec<Node>,
    pub root: Option<usize>,
    pub width: usize,
    pub fragment_count: usize,
    pub operation_count: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub fragments: usize,
    pub operations: usize,
    pub tolerance: f64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            fragments: 100_000,
            operations: 100_000,
            tolerance: 0.,
        }
    }
}
fn length(n: [f64; 3]) -> f64 {
    n[0].hypot(n[1]).hypot(n[2])
}
fn xyz(v: &[f64]) -> [f64; 3] {
    [v[0], v[1], v[2]]
}
fn valid(t: &Triangle, width: usize) -> bool {
    width >= 3
        && t.iter()
            .all(|v| v.len() == width && v.iter().all(|x| x.is_finite()))
}
fn normal(t: &Triangle) -> [f64; 3] {
    cross(sub(xyz(&t[1]), xyz(&t[0])), sub(xyz(&t[2]), xyz(&t[0])))
}
/// Attribute coordinates after xyz are interpolated with the same cut parameter.
pub fn split(t: &Triangle, plane: Plane, tolerance: f64) -> Result<Parts> {
    let norm = length([plane[0], plane[1], plane[2]]);
    if !valid(t, t[0].len())
        || !plane.iter().all(|x| x.is_finite())
        || !norm.is_finite()
        || norm == 0.
        || !tolerance.is_finite()
        || tolerance < 0.
    {
        return Err(fail("Invalid transparent triangle split"));
    }
    let distances: [f64; 3] = std::array::from_fn(|i| {
        (plane[0] * t[i][0] + plane[1] * t[i][1] + plane[2] * t[i][2] + plane[3]) / norm
    });
    if !distances.iter().all(|x| x.is_finite()) {
        return Err(fail("Unrepresentable transparent triangle distance"));
    }
    let signs: [i8; 3] = std::array::from_fn(|i| {
        let v = &t[i];
        let roundoff = 16.
            * f64::EPSILON
            * ((plane[0] * v[0]).abs()
                + (plane[1] * v[1]).abs()
                + (plane[2] * v[2]).abs()
                + plane[3].abs())
            / norm;
        if distances[i].abs() <= tolerance.max(roundoff) {
            0
        } else if distances[i] > 0. {
            1
        } else {
            -1
        }
    });
    let mut out = Parts::default();
    if signs.iter().all(|&s| s == 0) {
        out.coplanar.push(t.clone())
    } else if signs.iter().all(|&s| s >= 0) {
        out.front.push(t.clone())
    } else if signs.iter().all(|&s| s <= 0) {
        out.back.push(t.clone())
    } else {
        let mut positive = Vec::new();
        let mut negative = Vec::new();
        for i in 0..3 {
            let j = (i + 1) % 3;
            let a = &t[i];
            let b = &t[j];
            if signs[i] >= 0 {
                positive.push(a.clone())
            }
            if signs[i] <= 0 {
                negative.push(a.clone())
            }
            if signs[i] * signs[j] < 0 {
                let parameter = distances[i] / (distances[i] - distances[j]);
                let p = a
                    .iter()
                    .zip(b)
                    .map(|(&a, &b)| a * (1. - parameter) + b * parameter)
                    .collect::<Vec<_>>();
                positive.push(p.clone());
                negative.push(p)
            }
        }
        for (polygon, result) in [(positive, &mut out.front), (negative, &mut out.back)] {
            for i in 1..polygon.len() - 1 {
                result.push([
                    polygon[0].clone(),
                    polygon[i].clone(),
                    polygon[i + 1].clone(),
                ])
            }
        }
    }
    Ok(out)
}
impl Bsp {
    /// Builds once. Camera traversal can use these immutable nodes without recutting.
    pub fn build(fragments: Vec<Fragment>, limits: Limits) -> Result<Self> {
        if limits.fragments == 0
            || limits.operations == 0
            || !limits.tolerance.is_finite()
            || limits.tolerance < 0.
        {
            return Err(fail("Invalid transparency BSP limits"));
        }
        if fragments.len() > limits.fragments || fragments.len() > limits.operations {
            return Err(fail("Transparency input limit exceeded"));
        }
        let width = fragments.first().map_or(0, |f| f.triangle[0].len());
        if fragments
            .iter()
            .any(|f| f.triangle.iter().any(|v| v.len() != width))
        {
            return Err(fail("Inconsistent transparency vertex width"));
        }
        if fragments.iter().any(|f| !valid(&f.triangle, width)) {
            return Err(fail("Invalid transparency BSP vertex"));
        }
        let mut bsp = Self {
            width,
            ..Self::default()
        };
        bsp.partition(fragments, 0, None, limits)?;
        Ok(bsp)
    }
    fn operation(&mut self, limits: Limits) -> Result<()> {
        self.operation_count += 1;
        if self.operation_count > limits.operations {
            Err(fail("Transparency operation limit exceeded"))
        } else {
            Ok(())
        }
    }
    fn check_count(&self, extra: usize, limits: Limits) -> Result<()> {
        if self.fragment_count + extra > limits.fragments {
            Err(fail("Transparency fragment limit exceeded"))
        } else {
            Ok(())
        }
    }
    fn attach(&mut self, node: usize, parent: Option<(usize, bool)>) {
        if let Some((id, front)) = parent {
            if front {
                self.nodes[id].front = Some(node)
            } else {
                self.nodes[id].back = Some(node)
            }
        } else {
            self.root = Some(node)
        }
    }
    fn child(&self, parent: Option<(usize, bool)>) -> Option<usize> {
        if let Some((id, front)) = parent {
            if front {
                self.nodes[id].front
            } else {
                self.nodes[id].back
            }
        } else {
            self.root
        }
    }
    fn partition(
        &mut self,
        fragments: Vec<Fragment>,
        depth: usize,
        parent: Option<(usize, bool)>,
        limits: Limits,
    ) -> Result<()> {
        if fragments.len() > 16 {
            let t = &fragments[0].triangle;
            let n = normal(t);
            let l = length(n);
            let plane = [
                n[0] / l,
                n[1] / l,
                n[2] / l,
                -((0..3).fold(0., |s, i| s + n[i] * t[0][i])) / l,
            ];
            let mut coplanar = l > 0.;
            if coplanar {
                for f in &fragments {
                    self.operation(limits)?;
                    if split(&f.triangle, plane, limits.tolerance)?
                        .coplanar
                        .is_empty()
                    {
                        coplanar = false;
                        break;
                    }
                }
            }
            if coplanar {
                for f in fragments {
                    self.insert(f, parent, limits)?
                }
                return Ok(());
            }
        }
        let mut entries = fragments
            .into_iter()
            .map(|f| {
                let center: [f64; 3] =
                    std::array::from_fn(|k| f.triangle.iter().fold(0., |s, v| s + v[k] / 3.));
                (f, center)
            })
            .collect::<Vec<_>>();
        let spans: [f64; 3] = std::array::from_fn(|k| {
            let mut min = f64::INFINITY;
            let mut max = f64::NEG_INFINITY;
            for (_, c) in &entries {
                min = min.min(c[k]);
                max = max.max(c[k])
            }
            max - min
        });
        let mut axis = 0;
        for i in 1..3 {
            if spans[i] > spans[axis] {
                axis = i
            }
        }
        entries.sort_by(|a, b| {
            a.1[axis]
                .partial_cmp(&b.1[axis])
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if entries.len() > 16 && depth < 8 && spans[axis] > limits.tolerance {
            let mut plane = [0., 0., 0., -entries[entries.len() / 2].1[axis]];
            plane[axis] = 1.;
            let mut node = Node {
                plane,
                fragments: Vec::new(),
                front: None,
                back: None,
            };
            let mut front = Vec::new();
            let mut back = Vec::new();
            for (f, _) in &entries {
                self.operation(limits)?;
                let parts = split(&f.triangle, plane, limits.tolerance)?;
                for (parts, target) in [
                    (parts.front, &mut front),
                    (parts.back, &mut back),
                    (parts.coplanar, &mut node.fragments),
                ] {
                    for triangle in parts {
                        target.push(Fragment {
                            owner: f.owner,
                            triangle,
                        })
                    }
                }
                self.check_count(front.len() + back.len() + node.fragments.len(), limits)?;
            }
            if (front.len() + back.len()) as f64 > entries.len() as f64 * 1.5 {
                for (f, _) in entries {
                    self.insert(f, parent, limits)?
                }
                return Ok(());
            }
            self.fragment_count += node.fragments.len();
            let id = self.nodes.len();
            self.nodes.push(node);
            self.attach(id, parent);
            self.partition(front, depth + 1, Some((id, true)), limits)?;
            self.partition(back, depth + 1, Some((id, false)), limits)?;
        } else {
            let mut pending = vec![(0, entries.len())];
            while let Some((start, end)) = pending.pop() {
                if start >= end {
                    continue;
                }
                let middle = (start + end) / 2;
                self.insert(entries[middle].0.clone(), parent, limits)?;
                pending.push((start, middle));
                pending.push((middle + 1, end));
            }
        }
        Ok(())
    }
    fn insert(
        &mut self,
        fragment: Fragment,
        parent: Option<(usize, bool)>,
        limits: Limits,
    ) -> Result<()> {
        let mut pending = vec![(fragment, parent)];
        while let Some((f, parent)) = pending.pop() {
            self.operation(limits)?;
            let n = normal(&f.triangle);
            let l = length(n);
            if !l.is_finite() {
                return Err(fail("Unrepresentable transparency plane"));
            }
            if l == 0. {
                continue;
            }
            if let Some(id) = self.child(parent) {
                let parts = split(&f.triangle, self.nodes[id].plane, limits.tolerance)?;
                self.check_count(parts.coplanar.len(), limits)?;
                self.fragment_count += parts.coplanar.len();
                for triangle in parts.coplanar {
                    self.nodes[id].fragments.push(Fragment {
                        owner: f.owner,
                        triangle,
                    })
                }
                for (parts, side) in [(parts.front, true), (parts.back, false)] {
                    for triangle in parts {
                        pending.push((
                            Fragment {
                                owner: f.owner,
                                triangle,
                            },
                            Some((id, side)),
                        ))
                    }
                }
                self.check_count(pending.len(), limits)?;
            } else {
                self.check_count(1, limits)?;
                self.fragment_count += 1;
                let normal = n.map(|v| v / l);
                let plane = [
                    normal[0],
                    normal[1],
                    normal[2],
                    -(0..3).fold(0., |s, i| s + normal[i] * f.triangle[0][i]),
                ];
                let id = self.nodes.len();
                self.nodes.push(Node {
                    plane,
                    fragments: vec![f],
                    front: None,
                    back: None,
                });
                self.attach(id, parent);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn layer(z: f64) -> Triangle {
        [
            vec![-1., -1., z, 0.],
            vec![1., -1., z, 1.],
            vec![0., 1., z, 0.5],
        ]
    }
    #[test]
    fn arena_bounds_work_and_ignores_degenerate_geometry() {
        let source = (0..1000)
            .map(|i| Fragment {
                owner: i,
                triangle: layer(i as f64),
            })
            .collect();
        let bsp = Bsp::build(source, Limits::default()).unwrap();
        assert_eq!(bsp.fragment_count, 1000);
        assert!(bsp.operation_count < 11000);
        assert!(
            Bsp::build(
                (0..100)
                    .map(|i| Fragment {
                        owner: i,
                        triangle: layer(i as f64)
                    })
                    .collect(),
                Limits {
                    operations: 100,
                    ..Limits::default()
                }
            )
            .unwrap_err()
            .message
            .contains("operation limit")
        );
        assert_eq!(
            Bsp::build(
                vec![Fragment {
                    owner: 0,
                    triangle: [vec![0.; 3], vec![0.; 3], vec![0.; 3]]
                }],
                Limits::default()
            )
            .unwrap()
            .fragment_count,
            0
        );
    }
    #[test]
    fn clipped_attributes_and_nearby_layers_remain_distinct() {
        let t = [
            vec![-1., -1., -0.6, 0.],
            vec![1., -1., 0.6, 1.],
            vec![0., 1., 0., 0.5],
        ];
        let parts = split(&t, [1., 0., 0., 0.], 0.).unwrap();
        assert_eq!(parts.front.len(), 1);
        assert_eq!(parts.back.len(), 1);
        for triangle in parts.front.iter().chain(&parts.back) {
            for v in triangle {
                if v[0] == 0. {
                    assert_eq!(&v[2..], &[0., 0.5]);
                }
            }
        }
        assert_eq!(
            split(&layer(1e-12), [0., 0., 1., 0.], 0.)
                .unwrap()
                .front
                .len(),
            1
        );
        assert!(split(&t, [0.; 4], 0.).is_err());
        assert!(split(&t, [1., 0., 0., 0.], f64::NAN).is_err());
    }
}
