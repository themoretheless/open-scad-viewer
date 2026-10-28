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

/// Two interval evaluations of the SAME original affine expression. Composing
/// the fixed initial basis before multiplying carrier jets preserves their
/// coordinate correlation. Intersecting with the local-coordinate evaluation
/// retains both sound bounds; no rounded matrix becomes a new source.
#[derive(Clone, Copy)]
pub(crate) struct PairedPose {
    pub(crate) local: Pose,
    world: Option<Pose>,
}
impl PairedPose {
    pub(crate) fn new(local: Pose, basis: [V3; 3]) -> Result<Self> {
        let composed = (|| -> Result<Pose> {
            let mut linear = [[[I::point(0.); 3]; 2]; 3];
            for j in 0..3 {
                for order in 0..2 {
                    for k in 0..3 {
                        linear[j][order] = iv::add(
                            linear[j][order],
                            iv::scale(local.linear[k][order], basis[k][j])?,
                        )?;
                    }
                }
            }
            Ok(Pose {
                origin: local.origin,
                linear,
            })
        })();
        let world = match composed {
            Ok(p) => Some(p),
            Err(e) if e.code == "NURBS_NUMERIC_ERROR" => None,
            Err(e) => return Err(e),
        };
        Ok(Self { local, world })
    }
    pub(crate) fn enclose(
        self,
        q: [V3; 3],
        qt: V3,
        world_q: [V3; 3],
        world_qt: V3,
    ) -> Result<cell::Enclosure> {
        let local = self.local.enclose(q, qt);
        let Some(world) = self.world else {
            return local;
        };
        let world = world.enclose(world_q, world_qt);
        match (local, world) {
            (Ok(mut a), Ok(b)) => {
                for k in 0..3 {
                    a.value[k] = a.value[k].intersect(b.value[k].lo, b.value[k].hi)?;
                }
                for j in 0..3 {
                    for k in 0..3 {
                        a.columns[j][k] =
                            a.columns[j][k].intersect(b.columns[j][k].lo, b.columns[j][k].hi)?;
                    }
                }
                Ok(a)
            }
            (Ok(a), Err(e)) if e.code == "NURBS_NUMERIC_ERROR" => Ok(a),
            (Err(e), Ok(b)) if e.code == "NURBS_NUMERIC_ERROR" => Ok(b),
            (Err(e), _) | (_, Err(e)) => Err(e),
        }
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
    #[test]
    fn composed_basis_encloses_original_world_jets_and_both_time_terms() {
        let t = I::new(0.25, 0.5).unwrap();
        let u = I::new(0., 1.).unwrap();
        let v = u;
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
        // Composition is algebraic for any fixed matrix; no orthogonality of
        // this test proposal is assumed by the enclosure implementation.
        let basis = [[1., 1., 0.], [0., 1., 1.], [1., 0., 1.]].map(|r| r.map(I::point));
        let uv = u.mul(v).unwrap();
        let world = [
            [u, v, t.mul(uv).unwrap()],
            [I::point(1.), z, t.mul(v).unwrap()],
            [z, I::point(1.), t.mul(u).unwrap()],
        ];
        let world_t = [z, z, uv];
        let local = world.map(|p| super::super::local(p, basis).unwrap());
        let local_t = super::super::local(world_t, basis).unwrap();
        let old = pose.enclose(local, local_t).unwrap();
        let e = PairedPose::new(pose, basis)
            .unwrap()
            .enclose(local, local_t, world, world_t)
            .unwrap();
        for (new, old) in [e.value, e.columns[0], e.columns[1], e.columns[2]]
            .into_iter()
            .zip([old.value, old.columns[0], old.columns[1], old.columns[2]])
        {
            for k in 0..3 {
                assert!(new[k].lo >= old[k].lo && new[k].hi <= old[k].hi);
            }
        }
        for t in [0.25, 0.375, 0.5] {
            for u in [0., 0.5, 1.] {
                for v in [0., 0.5, 1.] {
                    let actual = [
                        [
                            t * t + (1. + t) * (u + v) + 0.125 * t * (u + t * u * v),
                            2. * t + (2. - t) * (v + t * u * v),
                            u + t * u * v,
                        ],
                        [
                            1. + t + 0.125 * t * (1. + t * v),
                            (2. - t) * t * v,
                            1. + t * v,
                        ],
                        [1. + t + 0.125 * t * t * u, (2. - t) * (1. + t * u), t * u],
                        [
                            2. * t + u + v + 0.125 * u + 0.25 * t * u * v,
                            2. - v + 2. * u * v - 2. * t * u * v,
                            u * v,
                        ],
                    ];
                    for (range, point) in [e.value, e.columns[0], e.columns[1], e.columns[2]]
                        .into_iter()
                        .zip(actual)
                    {
                        for k in 0..3 {
                            assert!(range[k].lo <= point[k] && point[k] <= range[k].hi);
                        }
                    }
                }
            }
        }
    }
}
