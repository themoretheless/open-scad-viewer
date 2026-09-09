//! Sampled inward shell for closed triangle meshes. BVHs keep grid queries bounded.
use polygon_kernel::{proximity::closest_triangle, BuiltMesh, Error, Mesh, Result};
type P = [f64; 3];
fn sub(a: P, b: P) -> P {
    std::array::from_fn(|i| a[i] - b[i])
}
fn dot(a: P, b: P) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn cross(a: P, b: P) -> P {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
#[derive(Clone)]
struct Triangle {
    p: [P; 3],
    min: P,
    max: P,
}
struct Node {
    min: P,
    max: P,
    triangles: Vec<Triangle>,
    children: Option<Box<[Node; 2]>>,
}
impl Node {
    fn build(mut triangles: Vec<Triangle>) -> Self {
        let min = std::array::from_fn(|k| {
            triangles
                .iter()
                .map(|t| t.min[k])
                .fold(f64::INFINITY, f64::min)
        });
        let max = std::array::from_fn(|k| {
            triangles
                .iter()
                .map(|t| t.max[k])
                .fold(f64::NEG_INFINITY, f64::max)
        });
        if triangles.len() <= 8 {
            return Self {
                min,
                max,
                triangles,
                children: None,
            };
        }
        let axis = (0..3)
            .max_by(|&a, &b| (max[a] - min[a]).total_cmp(&(max[b] - min[b])))
            .unwrap();
        triangles
            .sort_by(|a, b| (a.min[axis] + a.max[axis]).total_cmp(&(b.min[axis] + b.max[axis])));
        let right = triangles.split_off(triangles.len() / 2);
        Self {
            min,
            max,
            triangles: vec![],
            children: Some(Box::new([Self::build(triangles), Self::build(right)])),
        }
    }
    fn bound(&self, p: P) -> f64 {
        (0..3)
            .map(|k| (self.min[k] - p[k]).max(p[k] - self.max[k]).max(0.).powi(2))
            .sum()
    }
    fn nearest(&self, p: P, best: &mut f64) {
        if self.bound(p) > *best {
            return;
        }
        if let Some(children) = &self.children {
            let first = usize::from(children[1].bound(p) < children[0].bound(p));
            children[first].nearest(p, best);
            children[1 - first].nearest(p, best)
        } else {
            for t in &self.triangles {
                let q = closest_triangle(p, t.p[0], t.p[1], t.p[2]);
                *best = best.min(dot(sub(p, q), sub(p, q)))
            }
        }
    }
    fn distance(&self, p: P) -> f64 {
        let mut best = f64::INFINITY;
        self.nearest(p, &mut best);
        best.sqrt()
    }
    fn hits(&self, p: P, d: P, hits: &mut Vec<f64>) {
        let mut low: f64 = 0.;
        let mut high = f64::INFINITY;
        for k in 0..3 {
            let a = (self.min[k] - p[k]) / d[k];
            let b = (self.max[k] - p[k]) / d[k];
            low = low.max(a.min(b));
            high = high.min(a.max(b));
        }
        if high < low {
            return;
        }
        if let Some(children) = &self.children {
            children[0].hits(p, d, hits);
            children[1].hits(p, d, hits)
        } else {
            for t in &self.triangles {
                let e1 = sub(t.p[1], t.p[0]);
                let e2 = sub(t.p[2], t.p[0]);
                let h = cross(d, e2);
                let det = dot(e1, h);
                if det.abs() < 1e-13 {
                    continue;
                }
                let s = sub(p, t.p[0]);
                let u = dot(s, h) / det;
                if !(-1e-10..=1. + 1e-10).contains(&u) {
                    continue;
                }
                let q = cross(s, e1);
                let v = dot(d, q) / det;
                if v < -1e-10 || u + v > 1. + 1e-10 {
                    continue;
                }
                let along = dot(e2, q) / det;
                if along > 1e-10 {
                    hits.push(along)
                }
            }
        }
    }
    fn signed_distance(&self, p: P) -> f64 {
        let distance = self.distance(p);
        if distance < 1e-12 {
            return 0.;
        }
        let mut hits = vec![];
        self.hits(p, [1., 0.3713906763541037, 0.127831], &mut hits);
        hits.sort_by(f64::total_cmp);
        hits.dedup_by(|a, b| (*a - *b).abs() < 1e-8 * (1. + a.abs().max(b.abs())));
        if hits.len() % 2 == 1 {
            -distance
        } else {
            distance
        }
    }
}
pub fn shell(mesh: &Mesh, openings: &[usize], thickness: f64, step: f64) -> Result<BuiltMesh> {
    shell_options(mesh, openings, thickness, step, false)
}
pub fn shell_options(mesh: &Mesh, openings: &[usize], thickness: f64, step: f64, adaptive: bool) -> Result<BuiltMesh> {
    let report = mesh.inspect()?;
    if !report.closed || report.signed_volume_mm3 <= 0. || report.degenerate_triangles > 0 {
        return Err(Error::new(
            "Sampled Shell requires a closed, outward-oriented, nondegenerate mesh",
        ));
    }
    if !thickness.is_finite()
        || !step.is_finite()
        || thickness <= 0.
        || step <= 0.
        || step > thickness / 3. + 1e-9
    {
        return Err(Error::new(
            "Shell grid step must be positive and no larger than one third of wall thickness",
        ));
    }
    let count = mesh.indices.len() / 3;
    if count > 30000
        || openings.is_empty()
        || openings.len() >= count
        || openings.iter().any(|&t| t >= count)
    {
        return Err(Error::new(
            "Select valid openings; sampled Shell supports up to 30000 source triangles",
        ));
    }
    let triangles: Vec<_> = mesh
        .indices
        .chunks_exact(3)
        .map(|ids| {
            let p = ids.map_array(mesh);
            Triangle {
                min: std::array::from_fn(|k| p.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min)),
                max: std::array::from_fn(|k| {
                    p.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max)
                }),
                p,
            }
        })
        .collect();
    let removed: std::collections::BTreeSet<_> = openings.iter().copied().collect();
    let retained = Node::build(
        triangles
            .iter()
            .enumerate()
            .filter(|(i, _)| !removed.contains(i))
            .map(|(_, t)| t.clone())
            .collect(),
    );
    if !openings.iter().any(|&i| {
        let t = &triangles[i];
        let center = std::array::from_fn(|k| (t.p[0][k] + t.p[1][k] + t.p[2][k]) / 3.);
        retained.distance(center) > thickness + step / 2.
    }) {
        return Err(Error::new("Opening is too small for the wall thickness and grid resolution; select a larger connected face patch"));
    }
    let all = Node::build(triangles);
    let min = std::array::from_fn(|k| all.min[k] - (2.123 + 0.07 * k as f64) * step);
    let max = std::array::from_fn(|k| all.max[k] + (2.413 + 0.11 * k as f64) * step);
    let cells: [usize; 3] = std::array::from_fn(|k| ((max[k] - min[k]) / step).ceil() as usize);
    if cells.iter().any(|&n| n > if adaptive {256} else {64}) {
        return Err(Error::new(
            if adaptive {"Shell exceeds 256 adaptive cells per axis; increase grid step"} else {"Shell exceeds the 64-cell grid per axis; increase grid step or wall thickness"},
        ));
    }
    let field = |p| all.signed_distance(p).max(retained.distance(p) - thickness);
    let grid = sdf_kernel::Grid { min, max, cells };
    let output = if adaptive { adaptive_tiles(field, &grid)? } else { sdf_kernel::polygonize_with(field, &grid)? };
    let report = output.inspect()?;
    if !report.closed || output.indices.is_empty() || report.signed_volume_mm3 <= 0. {
        return Err(Error::new(
            "Sampled Shell produced an invalid boundary; try a different grid step",
        ));
    }
    Ok(BuiltMesh {
        mesh: output,
        report,
    })
}
/// Sparse subdivision skips blocks whose Lipschitz distance bound excludes the zero set.
/// All active leaf tiles share one requested spacing, avoiding coarse/fine cracks.
fn adaptive_tiles(field: impl Fn(P)->f64, grid: &sdf_kernel::Grid) -> Result<Mesh> {
    use std::collections::BTreeMap;
    let delta:P=std::array::from_fn(|k|(grid.max[k]-grid.min[k])/grid.cells[k] as f64);
    let coord=|i:[usize;3]|std::array::from_fn(|k|grid.min[k]+delta[k]*i[k] as f64);
    let mut pending=vec![([0usize;3],grid.cells)];
    let mut output=Mesh {positions:vec![],indices:vec![],uv:None};
    let mut welded=BTreeMap::<[i64;3],usize>::new();
    let quantum=delta.iter().copied().fold(f64::INFINITY,f64::min)*1e-7;
    let mut samples=0usize;
    while let Some((lo,hi))=pending.pop(){
        let min=coord(lo);let max=coord(hi);
        let center=std::array::from_fn(|k|(min[k]+max[k])/2.);
        let radius=dot(sub(max,min),sub(max,min)).sqrt()/2.;
        if field(center).abs()>radius+quantum {continue}
        let cells:[usize;3]=std::array::from_fn(|k|hi[k]-lo[k]);
        let axis=(0..3).max_by_key(|&k|cells[k]).unwrap();
        if cells[axis]>16 {let mid=(lo[axis]+hi[axis])/2;let mut a=hi;a[axis]=mid;let mut b=lo;b[axis]=mid;pending.push((lo,a));pending.push((b,hi));continue}
        samples+=cells.iter().map(|n|n+1).product::<usize>();
        if samples>4_000_000 {return Err(Error::new("Adaptive Shell exceeds four million samples; increase grid step"))}
        let tile=sdf_kernel::polygonize_tile(&field,&sdf_kernel::Grid{min,max,cells},false)?;
        let ids:Vec<usize>=tile.positions.chunks_exact(3).map(|p|{let key=std::array::from_fn(|k|((p[k]-grid.min[k])/quantum).round() as i64);*welded.entry(key).or_insert_with(||{let id=output.positions.len()/3;output.positions.extend_from_slice(p);id})}).collect();
        for t in tile.indices.chunks_exact(3){let tri=t.iter().map(|&i|ids[i]).collect::<Vec<_>>();if tri[0]!=tri[1]&&tri[1]!=tri[2]&&tri[0]!=tri[2]{output.indices.extend(tri)}}
        if output.indices.len()>300_000 {return Err(Error::new("Adaptive Shell exceeds 100000 triangles; increase grid step"))}
    }
    Ok(output)
}
trait TrianglePoints {
    fn map_array(&self, mesh: &Mesh) -> [P; 3];
}
impl TrianglePoints for [usize] {
    fn map_array(&self, mesh: &Mesh) -> [P; 3] {
        std::array::from_fn(|i| std::array::from_fn(|k| mesh.positions[self[i] * 3 + k]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bvh_distance_and_sign_agree_with_reference() {
        let mesh = polygon_kernel::cad::cube([10.; 3], false).unwrap();
        let triangles = mesh
            .indices
            .chunks_exact(3)
            .map(|ids| {
                let p = ids.map_array(&mesh);
                Triangle {
                    min: std::array::from_fn(|k| {
                        p.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min)
                    }),
                    max: std::array::from_fn(|k| {
                        p.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max)
                    }),
                    p,
                }
            })
            .collect();
        let bvh = Node::build(triangles);
        for i in 0..250 {
            let p = [
                ((i * 73) % 197) as f64 / 10. - 5.,
                ((i * 31) % 193) as f64 / 10. - 5.,
                ((i * 17) % 191) as f64 / 10. - 5.,
            ];
            let actual = bvh.signed_distance(p);
            let expected = polygon_kernel::proximity::signed_distance(&mesh, p);
            assert!(
                (actual - expected).abs() < 1e-7,
                "{p:?}: {actual} != {expected}"
            )
        }
    }
}

/// A bounded spatial graph of rounded struts, optionally blended into a skin.
pub fn lattice(mesh:&Mesh, nodes:Vec<P>, edges:Vec<[usize;2]>, radius:f64, skin:f64, step:f64, organic:bool, open_top:bool, wall_depth:f64, keep_core:bool)->Result<BuiltMesh>{
    let report=mesh.inspect()?;
    if !report.closed || report.signed_volume_mm3<=0. || mesh.indices.len()/3>30000 {return Err(Error::new("Spatial lattice requires a closed outward solid with at most 30000 source triangles"))}
    if nodes.is_empty()||nodes.len()>125||edges.is_empty()||edges.len()>400||nodes.iter().flatten().any(|v|!v.is_finite())||edges.iter().any(|e|e[0]>=nodes.len()||e[1]>=nodes.len()||e[0]==e[1]) {return Err(Error::new("Spatial lattice graph exceeds limits or has invalid nodes"))}
    if !radius.is_finite()||!skin.is_finite()||!step.is_finite()||radius<=0.||skin<0.||step<=0.||step>radius*0.8||skin>0.&&step>skin/2. {return Err(Error::new("Grid step must resolve the strut diameter and skin: step <= diameter/2.5 and skin/2"))}
    let triangles=mesh.indices.chunks_exact(3).map(|ids|{let p=ids.map_array(mesh);Triangle{min:std::array::from_fn(|k|p.iter().map(|p|p[k]).fold(f64::INFINITY,f64::min)),max:std::array::from_fn(|k|p.iter().map(|p|p[k]).fold(f64::NEG_INFINITY,f64::max)),p}}).collect();
    let all=Node::build(triangles);
    let segments:Vec<_>=edges.iter().enumerate().map(|(i,e)|{let a=nodes[e[0]];let d=sub(nodes[e[1]],a);let length2=dot(d,d);(a,d,length2,radius*if organic{1.0+0.3*((i*73%101) as f64/100.)}else{1.})}).collect();
    if segments.iter().any(|s|s.2<1e-12){return Err(Error::new("Coincident lattice nodes"))}
    let min:P=std::array::from_fn(|k|all.min[k]-(2.123+0.07*k as f64)*step);
    let max:P=std::array::from_fn(|k|all.max[k]+(2.413+0.11*k as f64)*step);
    let cells=std::array::from_fn(|k|((max[k]-min[k])/step).ceil() as usize);
    if cells.iter().any(|&n|n>64){return Err(Error::new("Spatial lattice exceeds 64 grid cells per axis. Increase strut thickness and grid step."))}
    if !wall_depth.is_finite() || wall_depth<0. || wall_depth>0. && wall_depth<step*2. {return Err(Error::new("Wall depth must be at least two sampling steps"))}
    let blend=radius*0.7;
    let field=|p:P|{
        let source=all.signed_distance(p);
        let mut graph=f64::INFINITY;
        for &(a,d,length2,r) in &segments {let q=sub(p,a);let t=(dot(q,d)/length2).clamp(0.,1.);let delta: P=std::array::from_fn(|k|q[k]-t*d[k]);let distance=dot(delta,delta).sqrt()-r;
            if organic&&graph.is_finite(){let h=((blend-(graph-distance).abs())/blend).max(0.);graph=graph.min(distance)-h*h*blend/4.}else{graph=graph.min(distance)}
        }
        let skin_field=if skin>0.{(-source-skin).max(if open_top{p[2]-(all.max[2]-skin)}else{f64::NEG_INFINITY})}else{f64::INFINITY};
        let material=graph.min(skin_field);
        let material=if wall_depth>0. {let band=-source-wall_depth;let walls=material.max(band);if keep_core {walls.min(source+wall_depth)}else{walls}}else{material};
        source.max(material)
    };
    let output=sdf_kernel::polygonize_with(field,&sdf_kernel::Grid{min,max,cells})?;
    let result=output.inspect()?;
    if !result.closed||result.degenerate_triangles>0||result.signed_volume_mm3<=0.||result.signed_volume_mm3>=report.signed_volume_mm3 {return Err(Error::new("Spatial lattice did not produce a valid lighter closed surface; adjust cell size or resolution"))}
    Ok(BuiltMesh{mesh:output,report:result})
}
