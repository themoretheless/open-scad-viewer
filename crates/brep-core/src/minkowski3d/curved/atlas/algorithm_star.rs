//! atlas algorithm: star.
use super::*;
pub fn star(
    vertex: u32,
    incident: &[usize],
    patches: &[Patch],
    positions: &[P],
    work: &mut Work,
) -> Result<usize> {
    let mut links = BTreeMap::<u32, (u32, Curve)>::new();
    let mut n: P = std::array::from_fn(|_| Q::zero());
    for &i in incident {
        let p = &patches[i];
        let k = p.ids.iter().position(|v| *v == vertex).unwrap();
        let c = bernstein::corners(&p.h);
        n = add(&n, &cross(&sub(&c[1], &c[0]), &sub(&c[2], &c[0])));
        let e = (k + 1) % 3;
        let curve = [
            bernstein::edges(&p.motion[0])[e].clone(),
            bernstein::edges(&p.motion[1])[e].clone(),
        ];
        if links
            .insert(p.ids[e], (p.ids[(e + 1) % 3], curve))
            .is_some()
        {
            return Err(fail("atlas vertex link branches"));
        }
    }
    match star_projected(vertex, incident, patches, positions, &links, &n, work) {
        Ok(count) => Ok(count),
        Err(first) => {
            // Area sums can lie exactly in a cap plane at a conical rim.
            // Normalize/deduplicate directions, then reprove the entire disk.
            let mut normals = BTreeSet::new();
            for &i in incident {
                let c = bernstein::corners(&patches[i].h);
                let v = cross(&sub(&c[1], &c[0]), &sub(&c[2], &c[0]));
                let norm: Q = v
                    .iter()
                    .map(|q| {
                        if *q < Q::zero() {
                            -q.clone()
                        } else {
                            q.clone()
                        }
                    })
                    .sum();
                if !norm.is_zero() {
                    normals.insert(v.map(|q| q / &norm));
                }
            }
            let n: P = std::array::from_fn(|k| normals.iter().map(|p| p[k].clone()).sum());
            if let Ok(count) =
                star_projected(vertex, incident, patches, positions, &links, &n, work)
            {
                return Ok(count);
            }
            for n in projection::directions(&normals, work)? {
                if let Ok(count) =
                    star_projected(vertex, incident, patches, positions, &links, &n, work)
                {
                    return Ok(count);
                }
            }
            Err(first)
        }
    }
}
