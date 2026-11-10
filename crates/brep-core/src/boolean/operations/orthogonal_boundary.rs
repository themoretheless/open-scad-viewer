//! Exact integer-grid boundary coalescing. Rectangles partition only exposed
//! cells on the same oriented plane; no geometric fitting or hole filling.
use super::*;

pub(super) fn polygons(
    coordinates: &[Vec<f64>; 3],
    occupied: &BTreeSet<[usize; 3]>,
) -> Result<Vec<Vec<[f64; 3]>>> {
    let shape = coordinates.each_ref().map(|v| v.len() - 1);
    let mut planes = BTreeMap::<(usize, bool, usize), BTreeSet<[usize; 2]>>::new();
    for cell in occupied {
        for axis in 0..3 {
            for positive in [false, true] {
                let adjacent = if positive {
                    cell[axis] as isize + 1
                } else {
                    cell[axis] as isize - 1
                };
                let exposed = adjacent < 0 || adjacent >= shape[axis] as isize || {
                    let mut neighbor = *cell;
                    neighbor[axis] = adjacent as usize;
                    !occupied.contains(&neighbor)
                };
                if exposed {
                    let u = (axis + 1) % 3;
                    let v = (axis + 2) % 3;
                    planes
                        .entry((axis, positive, cell[axis] + usize::from(positive)))
                        .or_default()
                        .insert([cell[u], cell[v]]);
                }
            }
        }
    }
    let mut result = Vec::new();
    for ((axis, positive, plane), mut cells) in planes {
        let u = (axis + 1) % 3;
        let v = (axis + 2) % 3;
        while let Some(&[x, y]) = cells.first() {
            let mut right = x + 1;
            while cells.contains(&[right, y]) {
                right += 1;
            }
            let mut top = y + 1;
            while (x..right).all(|i| cells.contains(&[i, top])) {
                top += 1;
            }
            // Every removed cell is present. These disjoint rectangles cover
            // all and only the exposed source cells, independent of UV/world
            // roundoff; world corners are existing grid coordinate values.
            for i in x..right {
                for j in y..top {
                    cells.remove(&[i, j]);
                }
            }
            let corners = if positive {
                [[x, y], [right, y], [right, top], [x, top]]
            } else {
                [[x, y], [x, top], [right, top], [right, y]]
            };
            result.push(
                corners
                    .into_iter()
                    .map(|q| {
                        let mut p = [0.; 3];
                        p[axis] = coordinates[axis][plane];
                        p[u] = coordinates[u][q[0]];
                        p[v] = coordinates[v][q[1]];
                        p
                    })
                    .collect(),
            );
            if result.len() > 256 {
                return Err(Error::new(
                    "BREP_RESOURCE_LIMIT",
                    "Coalesced orthogonal result exceeds 256 faces",
                ));
            }
        }
    }
    Ok(result)
}
