//! Original affine pose and its FIRST temporal derivative, certified once per
//! temporal restriction under this atlas owner's aggregate source ledger.
//! Reusing it for different UV boxes never repeats or borrows source work from
//! another certificate. Literal unit vectors isolate the linear part directly;
//! subtracting independently enclosed translated images would lose correlation.
use super::*;
#[derive(Clone, Copy)]
pub(crate) struct Pose {
    pub(crate) origin: [V3; 2],
    pub(crate) linear: [[V3; 2]; 3],
}
impl Pose {
    pub(crate) fn enclose(self, q: [V3; 3], qt: V3) -> Result<cell::Enclosure> {
        let apply = |v: V3, order: usize| -> Result<V3> {
            let mut r = [I::point(0.); 3];
            for j in 0..3 {
                r = iv::add(r, iv::scale(self.linear[j][order], v[j])?)?;
            }
            Ok(r)
        };
        Ok(cell::Enclosure {
            value: iv::add(self.origin[0], apply(q[0], 0)?)?,
            columns: [
                apply(q[1], 0)?,
                apply(q[2], 0)?,
                iv::add(iv::add(self.origin[1], apply(q[0], 1)?)?, apply(qt, 0)?)?,
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn affine_source_pose_differentiates_both_the_pose_and_the_moving_carrier() {
        let t = I::new(0.25, 0.5).unwrap();
        let u = I::new(0., 1.).unwrap();
        let v = I::new(0., 1.).unwrap();
        let z = I::point(0.);
        let pose = Pose {
            origin: [
                [t.mul(t).unwrap(), t.mul(I::point(2.)).unwrap(), z],
                [t.mul(I::point(2.)).unwrap(), I::point(2.), z],
            ],
            linear: [
                [[I::point(1.).add(t).unwrap(), z, z], [I::point(1.), z, z]],
                [[z, I::point(2.).sub(t).unwrap(), z], [z, I::point(-1.), z]],
                [
                    [t.mul(I::point(0.125)).unwrap(), z, I::point(1.)],
                    [I::point(0.125), z, z],
                ],
            ],
        };
        let uv = u.mul(v).unwrap();
        let q = [
            [u, v, t.mul(uv).unwrap()],
            [I::point(1.), z, t.mul(v).unwrap()],
            [z, I::point(1.), t.mul(u).unwrap()],
        ];
        let e = pose.enclose(q, [z, z, uv]).unwrap();
        for t in [0.25, 0.375, 0.5] {
            for u in [0., 0.5, 1.] {
                for v in [0., 0.5, 1.] {
                    let actual = [
                        [
                            t * t + (1. + t) * u + 0.125 * t * t * u * v,
                            2. * t + (2. - t) * v,
                            t * u * v,
                        ],
                        [1. + t + 0.125 * t * t * v, 0., t * v],
                        [0.125 * t * t * u, 2. - t, t * u],
                        [2. * t + u + 0.25 * t * u * v, 2. - v, u * v],
                    ];
                    for (ranges, expected) in [e.value, e.columns[0], e.columns[1], e.columns[2]]
                        .into_iter()
                        .zip(actual)
                    {
                        for k in 0..3 {
                            assert!(ranges[k].lo <= expected[k] && expected[k] <= ranges[k].hi);
                        }
                    }
                }
            }
        }
    }
}
