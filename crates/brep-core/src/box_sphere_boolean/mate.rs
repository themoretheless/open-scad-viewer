use super::*;

impl Mate for CanonicalBox<'_> {
    fn inside(&self, point: [f64; 3], band: f64) -> Result<bool> {
        let mut worst = f64::NEG_INFINITY;
        for face in &self.faces {
            worst = worst.max(dot(face.outward, sub(point, face.origin)));
        }
        if worst < -band {
            Ok(true)
        } else if worst > band {
            Ok(false)
        } else {
            Err(unsupported(
                "Box/sphere Boolean: region classification falls within the error band \
                 of a box face plane",
            ))
        }
    }

    fn uv_of(&self, face: usize, point: [f64; 3]) -> Result<[f64; 2]> {
        Ok(self.faces[face].uv_of(point))
    }

    fn arc_pcurve(&self, face: usize, arc: &Curve) -> Result<Curve> {
        Ok(affine_arc_pcurve(&self.model.faces[face].surface, arc))
    }

    fn network(&self, imprint: &mut Imprint<'_, Self>) -> Result<Option<SpatialRelation>> {
        let band = imprint.band;
        let sphere = imprint.sphere.clone();
        let contacts: Vec<(PlaneContact, f64)> = self
            .faces
            .iter()
            .map(|face| plane_contact(face.origin, face.outward, &sphere, band))
            .collect();
        if contacts
            .iter()
            .any(|(kind, _)| *kind == PlaneContact::Tangent)
        {
            return Err(unsupported(
                "Box/sphere Boolean: a box face plane lies within the tangency band of the sphere",
            ));
        }
        if contacts
            .iter()
            .any(|(kind, _)| *kind == PlaneContact::Outside)
        {
            return Ok(Some(SpatialRelation::Disjoint));
        }
        if contacts
            .iter()
            .all(|(kind, _)| *kind == PlaneContact::Inside)
        {
            return Ok(Some(imprint.mate_contains_sphere()));
        }
        // Section circles of the crossing planes, in each face's own basis
        // so that +phi is counter-clockwise in the face UV.
        for (i, face) in self.faces.iter().enumerate() {
            let (kind, d) = contacts[i];
            if kind != PlaneContact::Crossing {
                continue;
            }
            let rho2 = sphere.radius * sphere.radius - d * d;
            if !rho2.is_finite() || rho2 <= 0. {
                return Err(unsupported(
                    "Box/sphere Boolean: section circle radius is not certifiable",
                ));
            }
            let center: [f64; 3] = std::array::from_fn(|k| sphere.center[k] - d * face.outward[k]);
            imprint.circles.push(Circle::new(
                i,
                center,
                rho2.sqrt(),
                face.u.map(|x| x / face.u_len),
                face.v.map(|x| x / face.v_len),
            ));
        }
        self.edge_hits(imprint)?;
        self.face_intervals(imprint)?;
        Ok(None)
    }
}

impl CanonicalBox<'_> {
    /// Sphere ∩ box edges: the shared vertices of the arc network.
    fn edge_hits(&self, imprint: &mut Imprint<'_, Self>) -> Result<()> {
        let model = self.model;
        let c = imprint.sphere.center;
        let r = imprint.sphere.radius;
        let band = imprint.band;
        for (e, edge) in model.edges.iter().enumerate() {
            let p0 = model.vertices[edge.vertices[0]].point;
            let p1 = model.vertices[edge.vertices[1]].point;
            let dir = sub(p1, p0);
            let len = norm(dir);
            let qa = dot(dir, dir);
            let qb = 2. * dot(dir, sub(p0, c));
            let qc = dot(sub(p0, c), sub(p0, c)) - r * r;
            let disc = qb * qb - 4. * qa * qc;
            // Discriminant scale: (2 len r)^2 at a diametral hit.
            let disc_band = 8. * len * r * band;
            if disc.abs() <= disc_band {
                let t = -qb / (2. * qa);
                if (-band / len..=1. + band / len).contains(&t) {
                    return Err(unsupported(
                        "Box/sphere Boolean: the sphere is tangent to a box edge",
                    ));
                }
                continue;
            }
            if disc < 0. {
                continue;
            }
            let sq = disc.sqrt();
            let clear = band / len;
            for t in [(-qb - sq) / (2. * qa), (-qb + sq) / (2. * qa)] {
                if t < -clear || t > 1. + clear {
                    continue;
                }
                if t < 16. * clear + 1e-9 || t > 1. - 16. * clear - 1e-9 {
                    return Err(unsupported(
                        "Box/sphere Boolean: the sphere passes through a box vertex region",
                    ));
                }
                let point: [f64; 3] = std::array::from_fn(|k| p0[k] + t * dir[k]);
                let id = imprint.hit(point, e, t);
                for &face in &self.edge_faces[e] {
                    let Some(circle) = imprint.circles.iter_mut().find(|c| c.face == face) else {
                        return Err(unsupported(
                            "Box/sphere Boolean: edge hit on a face whose plane does not cross",
                        ));
                    };
                    let phi = circle.phi_of(point);
                    circle.hits.push((phi, id));
                }
            }
        }
        Ok(())
    }

    /// Per crossing face: the parts of the circle inside the rectangle.
    fn face_intervals(&self, imprint: &mut Imprint<'_, Self>) -> Result<()> {
        let band = imprint.band;
        for ci in 0..imprint.circles.len() {
            let face = &self.faces[imprint.circles[ci].face];
            let hits = imprint.circles[ci].sorted_hits()?;
            let clear_u = band / face.u_len + BOUNDARY_UV;
            let clear_v = band / face.v_len + BOUNDARY_UV;
            let inside_rect = |uv: [f64; 2]| -> Result<bool> {
                let inside = uv[0] > clear_u
                    && uv[0] < 1. - clear_u
                    && uv[1] > clear_v
                    && uv[1] < 1. - clear_v;
                let outside = uv[0] < -clear_u
                    || uv[0] > 1. + clear_u
                    || uv[1] < -clear_v
                    || uv[1] > 1. + clear_v;
                if inside {
                    Ok(true)
                } else if outside {
                    Ok(false)
                } else {
                    Err(unsupported(
                        "Box/sphere Boolean: section circle grazes the face boundary",
                    ))
                }
            };
            if hits.is_empty() {
                let mut corners_in = 0;
                for corner in [[0., 0.], [1., 0.], [1., 1.], [0., 1.]] {
                    if imprint.inside_sphere(face.point_at(corner))? {
                        corners_in += 1;
                    }
                }
                if corners_in == 4 {
                    // The face lies inside the sphere; the engine classifies
                    // it whole by its centre sample.
                    continue;
                }
                if corners_in != 0 {
                    return Err(unsupported(
                        "Box/sphere Boolean: face corners straddle the sphere without an edge hit",
                    ));
                }
                let circle = &mut imprint.circles[ci];
                let foot = face.uv_of(circle.center);
                let ru = circle.rho / face.u_len;
                let rv = circle.rho / face.v_len;
                if foot[0] - ru > clear_u
                    && foot[0] + ru < 1. - clear_u
                    && foot[1] - rv > clear_v
                    && foot[1] + rv < 1. - clear_v
                {
                    circle.push_ring();
                }
                // Otherwise the circle is disjoint from the rectangle (grazing
                // already refused through the hit scan): face untouched.
                continue;
            }
            let n = hits.len();
            if n % 2 != 0 {
                return Err(unsupported(
                    "Box/sphere Boolean: odd number of edge hits on one section circle",
                ));
            }
            let circle = &mut imprint.circles[ci];
            let mut expected: Option<bool> = None;
            for k in 0..n {
                let lo = hits[k].0;
                let hi = if k + 1 < n {
                    hits[k + 1].0
                } else {
                    hits[0].0 + TAU
                };
                let mid = circle.point_at((lo + hi) / 2.);
                let inside = inside_rect(face.uv_of(mid))?;
                if let Some(previous) = expected
                    && previous == inside
                {
                    return Err(unsupported(
                        "Box/sphere Boolean: section arcs do not alternate at edge hits",
                    ));
                }
                expected = Some(inside);
                if inside {
                    circle.push_interval(lo, hi, hits[k].1, hits[(k + 1) % n].1);
                }
            }
        }
        Ok(())
    }
}
