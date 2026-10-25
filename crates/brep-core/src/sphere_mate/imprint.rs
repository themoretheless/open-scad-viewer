use super::*;

impl<'m, M: Mate> Imprint<'m, M> {
    pub(super) fn new(
        a: &'m Model,
        b: &'m Model,
        mate_index: usize,
        mate: &'m M,
        sphere: CanonicalSphere,
        band: f64,
        operation: &str,
    ) -> Result<Self> {
        let sphere_model = [a, b][1 - mate_index];
        let sphere_pieces = Self::sphere_boundary_pieces(sphere_model);
        Ok(Self {
            asm: Assembler::new(a, b, operation)?,
            mate_index,
            mate,
            sphere,
            band,
            circles: Vec::new(),
            sphere_pieces,
            patch_uv: BTreeMap::new(),
            arcs: Vec::new(),
        })
    }

    fn sphere_index(&self) -> usize {
        1 - self.mate_index
    }
    fn sphere_model(&self) -> &'m Model {
        self.asm.src[self.sphere_index()]
    }

    /// Register a sphere hit on mate edge `e` at parameter `t`; a hit already
    /// registered at that edge parameter is returned instead of duplicated.
    pub(crate) fn hit(&mut self, point: [f64; 3], e: usize, t: f64) -> usize {
        let o = self.mate_index;
        if let Some(id) = self.asm.split_at(o, e, t, 1e-9) {
            return id;
        }
        let id = self.asm.special(point);
        self.asm.attach_edge(id, o, e, t);
        id
    }
    /// Per-face coedge index of each quarter-disk boundary piece (u-axis,
    /// quarter arc, v-axis) of a canonical sphere model.
    pub(crate) fn sphere_boundary_pieces(model: &Model) -> [[usize; 3]; 8] {
        std::array::from_fn(|face| {
            let wire = &model.loops[model.faces[face].outer];
            let mut map = [usize::MAX; 3];
            for (i, coedge) in wire.coedges.iter().enumerate() {
                let piece = if sphere_sphere::unit_quarter_arc(&coedge.pcurve) {
                    1
                } else if sphere_sphere::axis_line(&coedge.pcurve, [0., 0.], [1., 0.]) {
                    0
                } else {
                    2
                };
                map[piece] = i;
            }
            map
        })
    }
    /// True when the point is strictly inside the sphere; ambiguous → refusal.
    pub(crate) fn inside_sphere(&self, point: [f64; 3]) -> Result<bool> {
        sphere_inside(&self.sphere, point, self.band)
    }
    /// Containment relation "mate contains sphere" in call order.
    pub(crate) fn mate_contains_sphere(&self) -> SpatialRelation {
        if self.mate_index == 0 {
            SpatialRelation::AContainsB
        } else {
            SpatialRelation::BContainsA
        }
    }
    fn sphere_contains_mate(&self) -> SpatialRelation {
        if self.mate_index == 0 {
            SpatialRelation::BContainsA
        } else {
            SpatialRelation::AContainsB
        }
    }

    /// No arcs at all means the boundaries never meet: containment by a
    /// single certified sample each way, else disjoint.
    pub(super) fn no_arcs_relation(&self) -> Result<Option<SpatialRelation>> {
        if self.circles.iter().any(|c| !c.inside.is_empty()) {
            return Ok(None);
        }
        let mate_vertex = self.asm.src[self.mate_index].vertices[0].point;
        if self.inside_sphere(mate_vertex)? {
            return Ok(Some(self.sphere_contains_mate()));
        }
        if self.mate.inside(self.sphere.center, self.band)? {
            return Ok(Some(self.mate_contains_sphere()));
        }
        Ok(Some(SpatialRelation::Disjoint))
    }

    /// Sphere seam crossings of every inside arc. A crossing that coincides
    /// with an interval end (a mate edge hit on a seam) is merged into that
    /// hit vertex, which then lies on both operands' edges.
    pub(super) fn sphere_crossings(&mut self) -> Result<()> {
        let o = self.sphere_index();
        let scale = self.sphere.radius;
        for i in 0..self.circles.len() {
            if self.circles[i].inside.is_empty() {
                continue;
            }
            let (normal, middle) = {
                let c = &self.circles[i];
                (cross(c.e1, c.e2), c.center)
            };
            if self.seam_edges(i)?.is_some() {
                self.circles[i].seam = true;
                continue;
            }
            let sections = sphere_sphere::patch_sections(&self.sphere, normal, middle);
            for (patch, section) in sections {
                let PatchUvSection::Circle {
                    center: cc,
                    radius: ruv,
                    start,
                    end,
                } = section
                else {
                    return Err(unsupported(
                        "Sphere Boolean: section plane contains a sphere patch pole \
                         (great-circle section through original vertices)",
                    ));
                };
                if end - start >= TAU - 1e-9 {
                    let q = cc[0].hypot(cc[1]);
                    if cc[0] - ruv <= CORNER || cc[1] - ruv <= CORNER || q + ruv >= 1. - CORNER {
                        return Err(unsupported(
                            "Sphere Boolean: section circle is tangent to a sphere patch boundary",
                        ));
                    }
                    continue;
                }
                for angle in [start, end] {
                    let raw = [cc[0] + ruv * angle.cos(), cc[1] + ruv * angle.sin()];
                    let piece = piece_of(raw)?;
                    let w = snap_to_piece(piece, raw);
                    let model = self.sphere_model();
                    let coedge = &model.loops[model.faces[patch].outer].coedges
                        [self.sphere_pieces[patch][piece]];
                    let tau = piece_tau(piece, w, &coedge.pcurve);
                    let edge = coedge.edge;
                    let t = if coedge.reversed { 1. - tau } else { tau };
                    if !(1e-9..=1. - 1e-9).contains(&t) {
                        return Err(unsupported(
                            "Sphere Boolean: section passes through a sphere vertex",
                        ));
                    }
                    let point = point3(&model.faces[patch].surface.evaluate(w[0], w[1])?.point);
                    let circle = &self.circles[i];
                    let phi = circle.phi_of(point);
                    // Which inside interval owns this crossing, and whether it
                    // coincides with one of the interval's hit ends.
                    let mut owner = None;
                    for (k, span) in circle.inside.iter().enumerate() {
                        let local = span[0] + (phi - span[0]).rem_euclid(TAU);
                        if local < span[1] || (span[1] - local).abs() < EVENT_GAP {
                            let ends = circle.ends[k];
                            let at_end = if local - span[0] < EVENT_GAP {
                                ends[0]
                            } else if span[1] - local < EVENT_GAP {
                                ends[1]
                            } else {
                                None
                            };
                            if at_end.is_none() && (!local.is_finite() || local >= span[1]) {
                                continue;
                            }
                            owner = Some((k, local, at_end));
                            break;
                        }
                    }
                    let Some((k, local, at_end)) = owner else {
                        continue;
                    };
                    let existing = self
                        .asm
                        .splits
                        .get(&(o, edge))
                        .and_then(|table| table.iter().find(|(et, _)| (et - t).abs() <= 1e-9))
                        .map(|(_, id)| *id);
                    let id = match (existing, at_end) {
                        (Some(id), _) => id,
                        (None, Some(hit)) => {
                            if dist(self.asm.specials[hit].point, point) > 1e-7 * scale + self.band
                            {
                                return Err(unsupported(
                                    "Sphere Boolean: a seam crossing falls within the event \
                                     band of an edge hit without coinciding with it",
                                ));
                            }
                            self.asm.attach_edge(hit, o, edge, t);
                            hit
                        }
                        (None, None) => {
                            let id = self.asm.special(point);
                            self.asm.attach_edge(id, o, edge, t);
                            self.circles[i].events[k].push((local, id));
                            id
                        }
                    };
                    self.patch_uv.insert((patch, id), w);
                }
            }
        }
        Ok(())
    }

    /// Split each inside interval at its events, refine to quadrant sweeps
    /// and author the exact arc pieces with their owner patches.
    pub(super) fn refine_and_build_arcs(&mut self) -> Result<()> {
        for i in 0..self.circles.len() {
            if self.circles[i].seam {
                self.seam_arcs(i)?;
                continue;
            }
            let count = self.circles[i].inside.len();
            for k in 0..count {
                let (span, ends, mut events) = {
                    let c = &self.circles[i];
                    (c.inside[k], c.ends[k], c.events[k].clone())
                };
                events.sort_by(|a, b| a.0.total_cmp(&b.0));
                for w in events.windows(2) {
                    if w[1].0 - w[0].0 < EVENT_GAP {
                        return Err(unsupported(
                            "Sphere Boolean: two seam crossings coincide on one arc",
                        ));
                    }
                }
                let ring = ends[0].is_none();
                let mut stations: Vec<(f64, usize)> = Vec::new();
                if let Some(v0) = ends[0] {
                    stations.push((span[0], v0));
                }
                stations.extend(events.iter().copied());
                if ring && stations.is_empty() {
                    // Seed a ring that lies inside one patch and one face.
                    for q in 0..4 {
                        let phi = 0.37 + q as f64 * std::f64::consts::FRAC_PI_2;
                        let point = self.circles[i].point_at(phi);
                        let id = self.asm.special(point);
                        stations.push((phi, id));
                    }
                }
                let close = if ring {
                    let first = stations[0];
                    (first.0 + TAU, first.1)
                } else {
                    (span[1], ends[1].unwrap())
                };
                stations.push(close);
                let mut refined: Vec<(f64, usize)> = Vec::new();
                for w in 0..stations.len() - 1 {
                    refined.push(stations[w]);
                    let (lo, hi) = (stations[w].0, stations[w + 1].0);
                    let parts = ((hi - lo) / MAX_SPAN).ceil().max(1.) as usize;
                    for p in 1..parts {
                        let phi = lo + (hi - lo) * p as f64 / parts as f64;
                        let point = self.circles[i].point_at(phi);
                        let id = self.asm.special(point);
                        refined.push((phi, id));
                    }
                }
                refined.push(*stations.last().unwrap());
                for w in 0..refined.len() - 1 {
                    let (phi0, v0) = refined[w];
                    let (phi1, v1) = refined[w + 1];
                    let gap = phi1 - phi0;
                    let weight = (gap / 2.).cos();
                    if !weight.is_finite() || weight <= 0.5 {
                        return Err(unsupported(
                            "Sphere Boolean: arc piece exceeds a quadrant after refinement",
                        ));
                    }
                    let circle = &self.circles[i];
                    let mid = phi0 + gap / 2.;
                    let (sin, cos) = mid.sin_cos();
                    let shoulder: [f64; 3] = std::array::from_fn(|a| {
                        circle.center[a]
                            + (circle.rho / weight) * (cos * circle.e1[a] + sin * circle.e2[a])
                    });
                    let curve = Curve {
                        degree: 2,
                        knots: vec![0., 0., 0., 1., 1., 1.],
                        control_points: vec![
                            self.asm.specials[v0].point.to_vec(),
                            shoulder.to_vec(),
                            self.asm.specials[v1].point.to_vec(),
                        ],
                        weights: vec![1., weight, 1.],
                        periodic: false,
                    };
                    let midpoint = circle.point_at(mid);
                    let patch = self.owner_patch(midpoint)?;
                    let face = circle.face;
                    let k = self
                        .asm
                        .push_net([VKey::Special(v0), VKey::Special(v1)], curve);
                    self.arcs.push(Arc {
                        face,
                        patch: Some(patch),
                        key: EKey::Net(k),
                        reversed: false,
                    });
                }
            }
        }
        if self.arcs.len() > 512 {
            return Err(Error::new(
                "BREP_RESOURCE_LIMIT",
                "Sphere Boolean: arc network exceeds the admitted size",
            ));
        }
        Ok(())
    }

    /// The original sphere edges lying on circle `i`, as (edge, phi at
    /// vertex 0, phi at vertex 1 unwrapped near it), when the circle is a
    /// great circle in a seam plane of the sphere. `None` otherwise.
    fn seam_edges(&self, i: usize) -> Result<Option<Vec<(usize, f64, f64)>>> {
        let circle = &self.circles[i];
        let sphere = &self.sphere;
        if (circle.rho - sphere.radius).abs() > self.band
            || dist(circle.center, sphere.center) > self.band
        {
            return Ok(None);
        }
        let normal = cross(circle.e1, circle.e2);
        // The seam planes are spanned by the sphere frame axes: the pole
        // axis and the two equator directions of any patch.
        let aligned = (0..8).any(|patch| {
            let (a, b, pole) = sphere.frame(patch);
            [a, b, pole]
                .into_iter()
                .any(|axis| dot(normal, axis).abs() > 1. - 1e-9)
        });
        if !aligned {
            return Ok(None);
        }
        let model = self.sphere_model();
        let on_plane = |p: [f64; 3]| dot(normal, sub(p, circle.center)).abs() <= self.band;
        let mut edges = Vec::new();
        for (e, edge) in model.edges.iter().enumerate() {
            let p0 = model.vertices[edge.vertices[0]].point;
            let p1 = model.vertices[edge.vertices[1]].point;
            let mid = point3(&edge.curve.evaluate(0.5)?.point);
            if on_plane(p0) && on_plane(p1) && on_plane(mid) {
                let phi0 = circle.phi_of(p0);
                let phi1 = phi0 + (circle.phi_of(p1) - phi0 + std::f64::consts::PI).rem_euclid(TAU)
                    - std::f64::consts::PI;
                if ((phi1 - phi0).abs() - std::f64::consts::FRAC_PI_2).abs() > 1e-9 {
                    return Err(unsupported(
                        "Sphere Boolean: seam circle edge is not a quarter arc",
                    ));
                }
                edges.push((e, phi0, phi1));
            }
        }
        if edges.len() != 4 {
            return Err(unsupported(
                "Sphere Boolean: seam circle does not consist of four sphere edges",
            ));
        }
        Ok(Some(edges))
    }

    /// Pieces of a seam circle: every inside interval is covered by original
    /// sphere edges, split where a mate edge hit lies inside one of them.
    /// Hits that coincide with sphere vertices become those vertices.
    fn seam_arcs(&mut self, i: usize) -> Result<()> {
        let o = self.sphere_index();
        let edges = self
            .seam_edges(i)?
            .ok_or_else(|| unsupported("Sphere Boolean: seam circle lost its edges"))?;
        let model = self.sphere_model();
        let face = self.circles[i].face;
        let count = self.circles[i].inside.len();
        for k in 0..count {
            let (span, ends) = {
                let c = &self.circles[i];
                (c.inside[k], c.ends[k])
            };
            if !self.circles[i].events[k].is_empty() {
                return Err(unsupported(
                    "Sphere Boolean: seam circle carries interior seam events",
                ));
            }
            // Stations: interval ends, then every sphere vertex on the circle
            // strictly inside the interval.
            let mut stations: Vec<(f64, VKey)> = Vec::new();
            let end_station =
                |phi: f64, id: Option<usize>, this: &mut Self| -> Result<Option<(f64, VKey)>> {
                    let Some(id) = id else { return Ok(None) };
                    // A hit at a sphere vertex takes that vertex's identity.
                    let point = this.asm.specials[id].point;
                    for (e, phi0, phi1) in &edges {
                        for (vi, vphi) in [(0usize, *phi0), (1usize, *phi1)] {
                            let v = model.edges[*e].vertices[vi];
                            if dist(model.vertices[v].point, point)
                                <= this.band + 1e-9 * this.sphere.radius
                            {
                                this.asm.specials[id].orig = Some((o, v));
                            }
                            let _ = vphi;
                        }
                    }
                    Ok(Some((phi, VKey::Special(id))))
                };
            if let Some(st) = end_station(span[0], ends[0], self)? {
                stations.push(st);
            }
            let (lo, hi) = (span[0], span[1]);
            let ring = ends[0].is_none();
            for (e, phi0, phi1) in &edges {
                for (vi, vphi) in [(0usize, *phi0), (1usize, *phi1)] {
                    let mut local = lo + (vphi - lo).rem_euclid(TAU);
                    if ring && local > hi - EVENT_GAP {
                        local -= TAU;
                    }
                    let admitted = if ring {
                        local >= lo - EVENT_GAP && local < hi - EVENT_GAP
                    } else {
                        local > lo + EVENT_GAP && local < hi - EVENT_GAP
                    };
                    if admitted {
                        let v = model.edges[*e].vertices[vi];
                        if !stations.iter().any(|(_, key)| *key == VKey::Orig(o, v)) {
                            stations.push((local, VKey::Orig(o, v)));
                        }
                    }
                }
            }
            stations.sort_by(|a, b| a.0.total_cmp(&b.0));
            if let Some(st) = end_station(span[1], ends[1], self)? {
                stations.push(st);
            } else {
                let first = stations[0];
                stations.push((first.0 + TAU, first.1));
            }
            for w in 0..stations.len() - 1 {
                let (pa, ka) = stations[w];
                let (pb, kb) = stations[w + 1];
                if pb - pa < EVENT_GAP {
                    return Err(unsupported(
                        "Sphere Boolean: coincident stations on a seam circle",
                    ));
                }
                if pb - pa > std::f64::consts::FRAC_PI_2 + 1e-9 {
                    return Err(unsupported(
                        "Sphere Boolean: seam circle piece is not covered by one sphere edge",
                    ));
                }
                // The sphere edge covering [pa, pb].
                let mid = (pa + pb) / 2.;
                let mut owner = None;
                for (e, phi0, phi1) in &edges {
                    let (elo, ehi) = (phi0.min(*phi1), phi0.max(*phi1));
                    let local = elo + (mid - elo).rem_euclid(TAU);
                    if local > elo && local < ehi {
                        owner = Some((*e, *phi0, *phi1));
                        break;
                    }
                }
                let Some((e, phi0, phi1)) = owner else {
                    return Err(unsupported(
                        "Sphere Boolean: seam circle piece escapes the sphere edges",
                    ));
                };
                let forward = phi1 > phi0;
                // Edge parameters of the station points, inverted on the
                // edge curve itself (meridians and equator arcs are
                // parameterized differently).
                let circle = &self.circles[i];
                let limit = self.band + 1e-9 * self.sphere.radius;
                let ta = curve_parameter_of(&model.edges[e].curve, circle.point_at(pa), limit)?;
                let tb = curve_parameter_of(&model.edges[e].curve, circle.point_at(pb), limit)?;
                let (t_lo, t_hi) = (ta.min(tb), ta.max(tb));
                let snap = |t: f64| {
                    if t < 1e-9 {
                        0.
                    } else if t > 1. - 1e-9 {
                        1.
                    } else {
                        t
                    }
                };
                let (t_lo, t_hi) = (snap(t_lo), snap(t_hi));
                // Register interior hits as splits of this sphere edge.
                for (t, key) in [
                    (if forward { t_lo } else { t_hi }, ka),
                    (if forward { t_hi } else { t_lo }, kb),
                ] {
                    if let VKey::Special(id) = key
                        && self.asm.specials[id].orig.is_none()
                        && t > 0.
                        && t < 1.
                    {
                        let known = self
                            .asm
                            .splits
                            .get(&(o, e))
                            .map(|table| {
                                table
                                    .iter()
                                    .any(|(et, eid)| *eid == id && (et - t).abs() <= 1e-12)
                            })
                            .unwrap_or(false);
                        if !known {
                            self.asm.attach_edge(id, o, e, t);
                        }
                    }
                }
                let whole = t_lo == 0. && t_hi == 1.;
                let (key, curve) = if whole {
                    (EKey::Orig(o, e), model.edges[e].curve.clone())
                } else {
                    let table = self.asm.splits.get(&(o, e)).cloned().unwrap_or_default();
                    let n = table.len() + 1;
                    let j = (0..n)
                        .find(|&j| {
                            let lo = if j == 0 { 0. } else { table[j - 1].0 };
                            let hi = if j == n - 1 { 1. } else { table[j].0 };
                            (lo - t_lo).abs() <= 1e-12 && (hi - t_hi).abs() <= 1e-12
                        })
                        .ok_or_else(|| {
                            unsupported("Sphere Boolean: seam piece does not match the edge split")
                        })?;
                    (
                        EKey::Piece(o, e, j),
                        normalized(model.edges[e].curve.trim(t_lo, t_hi)?),
                    )
                };
                let curve = if forward { curve } else { curve.reverse()? };
                self.asm.push_net([ka, kb], curve);
                self.arcs.push(Arc {
                    face,
                    patch: None,
                    key,
                    reversed: !forward,
                });
            }
        }
        Ok(())
    }

    /// The single sphere patch containing a point strictly inside its chart.
    fn owner_patch(&self, point: [f64; 3]) -> Result<usize> {
        let mut found = None;
        for patch in 0..8 {
            let Some(uv) = self.sphere.invert_uv(patch, point) else {
                continue;
            };
            if strictly_inside_quarter_disk(uv, BOUNDARY_UV) {
                if found.is_some() {
                    return Err(unsupported(
                        "Sphere Boolean: arc midpoint lies in two sphere patches",
                    ));
                }
                found = Some(patch);
            }
        }
        found.ok_or_else(|| unsupported("Sphere Boolean: arc midpoint lies on a sphere seam"))
    }

    // -----------------------------------------------------------------------
    // Result topology
    // -----------------------------------------------------------------------

    /// Exact pcurve of an arc in its sphere patch, ends snapped to the chart
    /// vertices and resampled against the surface.
    fn patch_arc_pcurve(&self, k: usize, uv0: [f64; 2], uv1: [f64; 2]) -> Result<Curve> {
        let arc = &self.arcs[k];
        let Some(patch) = arc.patch else {
            return Err(unsupported(
                "Sphere Boolean: seam piece has no owning patch",
            ));
        };
        let mut pcurve = sphere_arc_pcurve(&self.sphere, patch, &self.asm.net[k].curve)?;
        pcurve.control_points[0] = uv0.to_vec();
        pcurve.control_points[2] = uv1.to_vec();
        let surface = &self.sphere_model().faces[patch].surface;
        verify_pcurve(
            &pcurve,
            &self.asm.net[k].curve,
            surface,
            1e-9 * self.sphere.radius.max(1.) + self.sphere.error,
        )?;
        Ok(pcurve)
    }

    /// UV of a special vertex in a sphere patch chart: the snapped seam value
    /// when it lies on the patch boundary, else a strict interior inversion.
    fn patch_vertex_uv(&mut self, patch: usize, id: usize) -> Result<[f64; 2]> {
        if let Some(&uv) = self.patch_uv.get(&(patch, id)) {
            return Ok(uv);
        }
        let point = self.asm.specials[id].point;
        let uv = self
            .sphere
            .invert_uv(patch, point)
            .ok_or_else(|| unsupported("Sphere Boolean: network vertex escapes its patch chart"))?;
        if !strictly_inside_quarter_disk(uv, CORNER) {
            return Err(unsupported(
                "Sphere Boolean: a network vertex lies on a sphere seam",
            ));
        }
        self.patch_uv.insert((patch, id), uv);
        Ok(uv)
    }

    /// Mate faces: untouched faces whole (classified by their centre sample),
    /// crossed faces through arrangement.
    pub(super) fn emit_mate(&mut self) -> Result<()> {
        let o = self.mate_index;
        let want = self.asm.want_inside[o];
        let model = self.asm.src[o];
        for face in 0..model.faces.len() {
            let reversed = face_reversed(model, face);
            let arcs: Vec<usize> = (0..self.arcs.len())
                .filter(|&k| self.arcs[k].face == face)
                .collect();
            if arcs.is_empty() {
                let centre = point3(&model.faces[face].surface.evaluate(0.5, 0.5)?.point);
                if self.inside_sphere(centre)? == want {
                    self.asm.emit_whole(o, face, reversed)?;
                }
                continue;
            }
            let mut chart = ChartVertices::default();
            let mut pieces = Vec::new();
            for (ka, kb, pcurve, key, rev) in self.asm.boundary_pieces(o, face)? {
                let ua = pcurve_point(&pcurve, 0.)?;
                let ub = pcurve_point(&pcurve, 1.)?;
                let a = chart.index(self.asm.chart_key(ka), ua);
                let b = chart.index(self.asm.chart_key(kb), ub);
                pieces.push(Piece {
                    v: [a, b],
                    pcurve,
                    key,
                    reversed: rev,
                });
            }
            let surface = model.faces[face].surface.clone();
            for k in arcs {
                let [ka, kb] = self.asm.net[k].ends;
                let mut pcurve = self.mate.arc_pcurve(face, &self.asm.net[k].curve)?;
                let ua = self.mate.uv_of(face, self.asm.point_of(ka))?;
                let ub = self.mate.uv_of(face, self.asm.point_of(kb))?;
                let a = chart.index(self.asm.chart_key(ka), ua);
                let b = chart.index(self.asm.chart_key(kb), ub);
                let n = pcurve.control_points.len();
                pcurve.control_points[0] = chart.uvs[a].to_vec();
                pcurve.control_points[n - 1] = chart.uvs[b].to_vec();
                verify_pcurve(
                    &pcurve,
                    &self.asm.net[k].curve,
                    &surface,
                    1e-9 * self.sphere.radius.max(1.) + self.band,
                )?;
                pieces.push(Piece {
                    v: [a, b],
                    pcurve,
                    key: self.arcs[k].key,
                    reversed: self.arcs[k].reversed,
                });
            }
            let sphere = self.sphere.clone();
            let band = self.band;
            let keep = move |p: [f64; 3]| Ok(sphere_inside(&sphere, p, band)? == want);
            self.asm
                .emit_chart(o, face, reversed, pieces, chart.len(), &keep)?;
        }
        Ok(())
    }

    /// Sphere patches: untouched patches whole, crossed through arrangement.
    pub(super) fn emit_sphere(&mut self) -> Result<()> {
        let o = self.sphere_index();
        let want = self.asm.want_inside[o];
        let model = self.sphere_model();
        for patch in 0..8 {
            let reversed = face_reversed(model, patch);
            let arcs: Vec<usize> = (0..self.arcs.len())
                .filter(|&k| self.arcs[k].patch == Some(patch))
                .collect();
            if arcs.is_empty() {
                // An interior sample, never the pole: seam-coincident cuts
                // pass exactly through the sphere vertices.
                let sample = point3(&model.faces[patch].surface.evaluate(0.3, 0.3)?.point);
                if self.mate.inside(sample, self.band)? == want {
                    self.asm.emit_whole(o, patch, reversed)?;
                }
                continue;
            }
            let mut chart = ChartVertices::default();
            let mut pieces = Vec::new();
            for (ka, kb, mut pcurve, key, rev) in self.asm.boundary_pieces(o, patch)? {
                let ua = match ka {
                    VKey::Special(id) => self.patch_vertex_uv(patch, id)?,
                    VKey::Orig(..) => pcurve_point(&pcurve, 0.)?,
                };
                let ub = match kb {
                    VKey::Special(id) => self.patch_vertex_uv(patch, id)?,
                    VKey::Orig(..) => pcurve_point(&pcurve, 1.)?,
                };
                let n = pcurve.control_points.len();
                pcurve.control_points[0] = ua.to_vec();
                pcurve.control_points[n - 1] = ub.to_vec();
                let a = chart.index(self.asm.chart_key(ka), ua);
                let b = chart.index(self.asm.chart_key(kb), ub);
                pieces.push(Piece {
                    v: [a, b],
                    pcurve,
                    key,
                    reversed: rev,
                });
            }
            for k in arcs {
                let [ka, kb] = self.asm.net[k].ends;
                let (VKey::Special(v0), VKey::Special(v1)) = (ka, kb) else {
                    return Err(unsupported(
                        "Sphere Boolean: patch arc bounded by an original vertex",
                    ));
                };
                let ua = self.patch_vertex_uv(patch, v0)?;
                let ub = self.patch_vertex_uv(patch, v1)?;
                let pcurve = self.patch_arc_pcurve(k, ua, ub)?;
                let a = chart.index(ka, ua);
                let b = chart.index(kb, ub);
                pieces.push(Piece {
                    v: [a, b],
                    pcurve,
                    key: EKey::Net(k),
                    reversed: false,
                });
            }
            let mate = self.mate;
            let band = self.band;
            let keep = move |p: [f64; 3]| Ok(mate.inside(p, band)? == want);
            self.asm
                .emit_chart(o, patch, reversed, pieces, chart.len(), &keep)?;
        }
        Ok(())
    }
}
