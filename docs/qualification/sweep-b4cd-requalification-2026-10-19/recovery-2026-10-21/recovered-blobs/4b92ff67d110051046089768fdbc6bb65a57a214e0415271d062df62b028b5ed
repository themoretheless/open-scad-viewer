//! Small renderer-independent migration surface for position-only path meshes.
//!
//! Builders accept document-space `f32` points used by UI renderers. Geometry
//! runs in the planar kernel's binary64 domain; only the completed, validated
//! result is converted and appended to the caller's vertex buffers. This is the
//! path/fill/stroke subset needed by Curvex, not a general event/attribute API.
//! Builders accept up to 65,536 segments and contours. Flattened fills share the
//! kernel's 65,536 source-vertex budget; intersection/output budgets also apply.

pub mod path {
    use crate::path::{BezierPath, PathSegment};
    use crate::{Result, check};

    use crate::limits::PATH_SEGMENTS as MAX_PATH_ITEMS;

    pub mod math {
        #[derive(Debug, Clone, Copy, PartialEq, Default)]
        pub struct Point {
            pub x: f32,
            pub y: f32,
        }

        pub const fn point(x: f32, y: f32) -> Point {
            Point { x, y }
        }
    }

    use math::Point;

    #[derive(Debug, Clone, Default)]
    pub struct Path {
        pub(super) contours: Vec<BezierPath>,
        error: Option<&'static str>,
    }

    impl Path {
        pub fn builder() -> Builder {
            Builder::default()
        }

        pub(super) fn validate(&self) -> Result<()> {
            check(
                self.error.is_none(),
                self.error.unwrap_or("Invalid render path"),
            )
        }
    }

    #[derive(Debug, Default)]
    pub struct Builder {
        path: Path,
        active: Option<BezierPath>,
        segment_count: usize,
    }

    impl Builder {
        fn validate_points(&mut self, points: &[Point]) -> bool {
            if self.path.error.is_some() {
                return false;
            }
            if points.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) {
                self.path.error = Some("Invalid render path coordinate");
                return false;
            }
            true
        }

        pub fn begin(&mut self, at: Point) {
            if !self.validate_points(&[at]) {
                return;
            }
            if self.active.is_some() {
                self.path.error = Some("Render path contour must end before beginning another");
                return;
            }
            if self.path.contours.len() >= MAX_PATH_ITEMS {
                self.path.error = Some("Render path contour budget exceeded");
                return;
            }
            self.active = Some(BezierPath {
                start: pair(at),
                segments: Vec::new(),
                closed: false,
            });
        }

        fn segment(&mut self, segment: PathSegment) {
            if self.segment_count >= MAX_PATH_ITEMS {
                self.path.error = Some("Render path segment budget exceeded");
                return;
            }
            if let Some(contour) = &mut self.active {
                contour.segments.push(segment);
                self.segment_count += 1;
            } else {
                self.path.error = Some("Render path segment requires begin");
            }
        }

        pub fn line_to(&mut self, to: Point) {
            if self.validate_points(&[to]) {
                self.segment(PathSegment::Line { to: pair(to) });
            }
        }

        pub fn cubic_bezier_to(&mut self, c1: Point, c2: Point, to: Point) {
            if self.validate_points(&[c1, c2, to]) {
                self.segment(PathSegment::Cubic {
                    c1: pair(c1),
                    c2: pair(c2),
                    to: pair(to),
                });
            }
        }

        pub fn end(&mut self, close: bool) {
            if self.path.error.is_some() {
                return;
            }
            if let Some(mut contour) = self.active.take() {
                contour.closed = close;
                self.path.contours.push(contour);
            } else {
                self.path.error = Some("Render path end requires begin");
            }
        }

        pub fn build(mut self) -> Path {
            if self.active.is_some() {
                self.path.error = Some("Render path contour is not ended");
            }
            self.path
        }
    }

    fn pair(p: Point) -> [f64; 2] {
        [p.x as f64, p.y as f64]
    }
}

pub mod tess {
    use super::path::{
        Path,
        math::{Point, point},
    };
    pub use crate::stroke::{LineCap, LineJoin};
    pub use crate::tessellation::FillRule;
    use crate::tessellation::{FillMesh, tessellate_rings};
    use crate::{Result, check};

    #[derive(Debug, Clone, PartialEq)]
    pub struct VertexBuffers<Vertex, Index> {
        pub vertices: Vec<Vertex>,
        pub indices: Vec<Index>,
    }

    impl<Vertex, Index> Default for VertexBuffers<Vertex, Index> {
        fn default() -> Self {
            Self {
                vertices: Vec::new(),
                indices: Vec::new(),
            }
        }
    }

    impl<Vertex, Index> VertexBuffers<Vertex, Index> {
        pub fn new() -> Self {
            Self::default()
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct FillVertex {
        position: Point,
    }
    impl FillVertex {
        pub fn position(&self) -> Point {
            self.position
        }
    }
    pub type StrokeVertex = FillVertex;

    pub struct BuffersBuilder<'a, Vertex, Constructor> {
        buffers: &'a mut VertexBuffers<Vertex, u32>,
        constructor: Constructor,
    }
    impl<'a, Vertex, Constructor> BuffersBuilder<'a, Vertex, Constructor> {
        pub fn new(buffers: &'a mut VertexBuffers<Vertex, u32>, constructor: Constructor) -> Self {
            Self {
                buffers,
                constructor,
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct FillOptions {
        pub fill_rule: FillRule,
        pub tolerance: f32,
    }
    impl Default for FillOptions {
        fn default() -> Self {
            Self {
                fill_rule: FillRule::EvenOdd,
                tolerance: 0.1,
            }
        }
    }
    impl FillOptions {
        pub fn with_fill_rule(mut self, rule: FillRule) -> Self {
            self.fill_rule = rule;
            self
        }
        pub fn with_tolerance(mut self, tolerance: f32) -> Self {
            self.tolerance = tolerance;
            self
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct StrokeOptions {
        pub line_width: f32,
        pub tolerance: f32,
        pub line_cap: LineCap,
        pub line_join: LineJoin,
        pub miter_limit: f32,
    }
    impl Default for StrokeOptions {
        fn default() -> Self {
            Self {
                line_width: 1.,
                tolerance: 0.1,
                line_cap: LineCap::Butt,
                line_join: LineJoin::Miter,
                miter_limit: 4.,
            }
        }
    }
    impl StrokeOptions {
        pub fn with_line_width(mut self, width: f32) -> Self {
            self.line_width = width;
            self
        }
        pub fn with_tolerance(mut self, tolerance: f32) -> Self {
            self.tolerance = tolerance;
            self
        }
        pub fn with_line_cap(mut self, cap: LineCap) -> Self {
            self.line_cap = cap;
            self
        }
        pub fn with_line_join(mut self, join: LineJoin) -> Self {
            self.line_join = join;
            self
        }
        pub fn with_miter_limit(mut self, limit: f32) -> Self {
            self.miter_limit = limit;
            self
        }
    }

    #[derive(Debug, Default)]
    pub struct FillTessellator;
    impl FillTessellator {
        pub fn new() -> Self {
            Self
        }

        pub fn tessellate_path<V, F: FnMut(FillVertex) -> V>(
            &mut self,
            path: &Path,
            options: &FillOptions,
            output: &mut BuffersBuilder<'_, V, F>,
        ) -> Result<()> {
            path.validate()?;
            validate_tolerance(options.tolerance)?;
            let rings = path
                .contours
                .iter()
                .filter(|contour| !contour.segments.is_empty())
                .map(|contour| contour.flatten_tol(options.tolerance as f64))
                .collect::<Result<Vec<_>>>()?;
            append_mesh(tessellate_rings(&rings, options.fill_rule)?, output)
        }
    }

    #[derive(Debug, Default)]
    pub struct StrokeTessellator;
    impl StrokeTessellator {
        pub fn new() -> Self {
            Self
        }

        pub fn tessellate_path<V, F: FnMut(StrokeVertex) -> V>(
            &mut self,
            path: &Path,
            options: &StrokeOptions,
            output: &mut BuffersBuilder<'_, V, F>,
        ) -> Result<()> {
            path.validate()?;
            validate_tolerance(options.tolerance)?;
            check(
                options.line_width.is_finite() && options.line_width > 0.,
                "Invalid render stroke width",
            )?;
            check(
                options.miter_limit.is_finite() && options.miter_limit >= 1.,
                "Invalid render miter limit",
            )?;
            let core_options = crate::stroke::StrokeOptions {
                width: options.line_width as f64,
                cap: options.line_cap,
                join: options.line_join,
                miter_limit: options.miter_limit as f64,
                ..Default::default()
            };
            if path.contours.len() == 1 && !path.contours[0].segments.is_empty() {
                return append_mesh(
                    crate::stroke::tessellate_stroke(
                        &path.contours[0],
                        &core_options,
                        options.tolerance as f64,
                    )?,
                    output,
                );
            }
            let mut rings = Vec::new();
            for contour in &path.contours {
                if contour.segments.is_empty() {
                    continue;
                }
                rings.extend(crate::stroke::stroke_rings(
                    contour,
                    &core_options,
                    options.tolerance as f64,
                )?);
            }
            // A single outline operation already returned a normalized region.
            // Multiple independently stroked contours still need a joint union.
            let mesh = if path.contours.len() == 1 {
                crate::tessellation::tessellate_normalized_rings(&rings)?
            } else {
                tessellate_rings(&rings, FillRule::NonZero)?
            };
            append_mesh(mesh, output)
        }
    }

    fn validate_tolerance(value: f32) -> Result<()> {
        check(value.is_finite() && value > 0., "Invalid render tolerance")
    }

    fn append_mesh<V, F: FnMut(FillVertex) -> V>(
        mesh: FillMesh,
        output: &mut BuffersBuilder<'_, V, F>,
    ) -> Result<()> {
        let base = output.buffers.vertices.len();
        check(
            base.checked_add(mesh.positions.len())
                .is_some_and(|n| n <= u32::MAX as usize),
            "Render vertex index overflow",
        )?;
        let positions: Vec<Point> = mesh
            .positions
            .iter()
            .map(|p| point(p[0] as f32, p[1] as f32))
            .collect();
        check(
            positions.iter().all(|p| p.x.is_finite() && p.y.is_finite()),
            "Render coordinate conversion overflow",
        )?;
        let mut indices = Vec::with_capacity(mesh.indices.len());
        for index in mesh.indices {
            check(
                (index as usize) < positions.len(),
                "Invalid render mesh index",
            )?;
            indices.push((base + index as usize) as u32);
        }
        // Complete both conversion and constructor calls before publishing any
        // geometry. A later contour/coordinate failure cannot leave a partial fill.
        let vertices: Vec<V> = positions
            .into_iter()
            .map(|position| (output.constructor)(FillVertex { position }))
            .collect();
        output.buffers.vertices.extend(vertices);
        output.buffers.indices.extend(indices);
        Ok(())
    }

    #[test]
    fn conversion_overflow_is_rejected_before_calling_vertex_constructor() {
        let original = VertexBuffers {
            vertices: vec![point(3., 4.)],
            indices: vec![0],
        };
        let mut buffers = original.clone();
        let mut calls = 0;
        let mesh = FillMesh {
            positions: vec![[0., 0.], [f64::MAX, 0.], [0., 1.]],
            indices: vec![0, 1, 2],
        };
        let result = append_mesh(
            mesh,
            &mut BuffersBuilder::new(&mut buffers, |v: FillVertex| {
                calls += 1;
                v.position()
            }),
        );
        assert!(result.is_err());
        assert_eq!(calls, 0);
        assert_eq!(buffers, original);
    }
}

#[cfg(test)]
mod tests {
    use super::path::{
        Path,
        math::{Point, point},
    };
    use super::tess::*;

    fn rect(builder: &mut super::path::Builder, min: f32, max: f32) {
        builder.begin(point(min, min));
        builder.line_to(point(max, min));
        builder.line_to(point(max, max));
        builder.line_to(point(min, max));
        builder.end(true);
    }

    fn area(mesh: &VertexBuffers<Point, u32>) -> f64 {
        mesh.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|tri| {
                let [a, b, c] = [
                    mesh.vertices[tri[0] as usize],
                    mesh.vertices[tri[1] as usize],
                    mesh.vertices[tri[2] as usize],
                ];
                ((b.x as f64 - a.x as f64) * (c.y as f64 - a.y as f64)
                    - (b.y as f64 - a.y as f64) * (c.x as f64 - a.x as f64))
                    * 0.5
            })
            .sum()
    }

    #[test]
    fn compound_render_fill_preserves_holes_under_selected_rule() {
        let mut builder = Path::builder();
        rect(&mut builder, 0., 10.);
        rect(&mut builder, 2., 8.);
        let path = builder.build();
        for (rule, expected) in [(FillRule::EvenOdd, 64.), (FillRule::NonZero, 100.)] {
            let mut buffers = VertexBuffers::<Point, u32>::new();
            FillTessellator::new()
                .tessellate_path(
                    &path,
                    &FillOptions::default().with_fill_rule(rule),
                    &mut BuffersBuilder::new(&mut buffers, |v: FillVertex| v.position()),
                )
                .unwrap();
            assert_eq!(area(&buffers), expected);
        }
    }

    #[test]
    fn closed_stroke_renders_ribbon_with_empty_center() {
        let mut builder = Path::builder();
        rect(&mut builder, 0., 10.);
        let mut buffers = VertexBuffers::<Point, u32>::new();
        StrokeTessellator::new()
            .tessellate_path(
                &builder.build(),
                &StrokeOptions::default()
                    .with_line_width(2.)
                    .with_tolerance(0.05),
                &mut BuffersBuilder::new(&mut buffers, |v: StrokeVertex| v.position()),
            )
            .unwrap();
        assert!((area(&buffers) - 80.).abs() < 1e-5);
        for tri in buffers.indices.as_chunks::<3>().0 {
            let points: Vec<_> = tri.iter().map(|&i| buffers.vertices[i as usize]).collect();
            let cx = points.iter().map(|p| p.x).sum::<f32>() / 3.;
            let cy = points.iter().map(|p| p.y).sum::<f32>() / 3.;
            assert!(cx <= 1. || cx >= 9. || cy <= 1. || cy >= 9.);
        }
    }

    #[test]
    fn complex_closed_stroke_is_not_limited_by_temporary_join_polygons() {
        let mut builder = Path::builder();
        let count = 1200;
        for i in 0..count {
            let angle = i as f32 * std::f32::consts::TAU / count as f32;
            let p = point(100. * angle.cos(), 100. * angle.sin());
            if i == 0 {
                builder.begin(p);
            } else {
                builder.line_to(p);
            }
        }
        builder.end(true);
        let mut buffers = VertexBuffers::<Point, u32>::new();
        StrokeTessellator::new()
            .tessellate_path(
                &builder.build(),
                &StrokeOptions::default()
                    .with_line_width(2.)
                    .with_tolerance(0.05),
                &mut BuffersBuilder::new(&mut buffers, |v: StrokeVertex| v.position()),
            )
            .unwrap();
        assert!((area(&buffers) - 400. * std::f64::consts::PI).abs() < 0.1);
        assert!(buffers.vertices.len() > count);
    }

    #[test]
    fn imported_five_thousand_point_contour_renders_completely() {
        let mut builder = Path::builder();
        let count = 5000;
        let radius = 10_000.;
        for i in 0..count {
            let angle = i as f64 * std::f64::consts::TAU / count as f64;
            let p = point((radius * angle.cos()) as f32, (radius * angle.sin()) as f32);
            if i == 0 {
                builder.begin(p);
            } else {
                builder.line_to(p);
            }
        }
        builder.end(true);
        let mut mesh = VertexBuffers::<Point, u32>::new();
        FillTessellator::new()
            .tessellate_path(
                &builder.build(),
                &FillOptions::default(),
                &mut BuffersBuilder::new(&mut mesh, |v: FillVertex| v.position()),
            )
            .unwrap();
        assert_eq!(mesh.vertices.len(), count);
        assert_eq!(mesh.indices.len() / 3, count - 2);
        assert!((area(&mesh) / (std::f64::consts::PI * radius * radius) - 1.).abs() < 1e-6);
    }

    #[test]
    fn translated_and_extreme_finite_f32_compound_fills_preserve_holes() {
        for (min, max) in [
            (10_000_000_f32, 10_000_128_f32),
            (-f32::MAX, f32::MAX),
            (1e-30_f32, 2e-30_f32),
        ] {
            // Interpolate in binary64 so the full f32 span itself cannot overflow.
            let span = max as f64 - min as f64;
            let inner_min = (min as f64 + span * 0.25) as f32;
            let inner_max = (min as f64 + span * 0.75) as f32;
            let mut builder = Path::builder();
            rect(&mut builder, min, max);
            rect(&mut builder, inner_min, inner_max);
            let mut mesh = VertexBuffers::<Point, u32>::new();
            FillTessellator::new()
                .tessellate_path(
                    &builder.build(),
                    &FillOptions::default(),
                    &mut BuffersBuilder::new(&mut mesh, |v: FillVertex| v.position()),
                )
                .unwrap();
            let expected = span * span - (inner_max as f64 - inner_min as f64).powi(2);
            assert!(!mesh.indices.is_empty());
            assert!(
                mesh.vertices
                    .iter()
                    .all(|p| p.x.is_finite() && p.y.is_finite())
            );
            assert!(
                (area(&mesh) / expected - 1.).abs() < 1e-6,
                "bounds {min}, {max}"
            );
        }
    }

    #[test]
    fn custom_gradient_vertices_receive_document_positions_and_append_indices() {
        let mut builder = Path::builder();
        builder.begin(point(0., 0.));
        builder.cubic_bezier_to(point(0., 2.), point(2., 2.), point(2., 0.));
        builder.end(true);
        let path = builder.build();
        let mut buffers = VertexBuffers::<(Point, f32), u32>::new();
        buffers.vertices.push((point(-1., -1.), -1.));
        FillTessellator::new()
            .tessellate_path(
                &path,
                &FillOptions::default().with_tolerance(0.02),
                &mut BuffersBuilder::new(&mut buffers, |v: FillVertex| {
                    let p = v.position();
                    (p, p.x / 2.)
                }),
            )
            .unwrap();
        assert!(
            buffers
                .indices
                .iter()
                .all(|&i| i > 0 && (i as usize) < buffers.vertices.len())
        );
        assert!(buffers.vertices[1..].iter().all(|(p, color)| p.x >= 0.
            && p.x <= 2.
            && p.y >= 0.
            && (*color - p.x / 2.).abs() < 1e-6));
        assert!(buffers.indices.len() >= 3);
    }

    #[test]
    fn failed_paths_and_options_leave_existing_buffers_untouched() {
        let mut valid = Path::builder();
        rect(&mut valid, 0., 1.);
        let valid = valid.build();
        let original = VertexBuffers {
            vertices: vec![point(7., 8.)],
            indices: vec![0_u32],
        };
        let mut buffers = original.clone();
        for tolerance in [f32::NAN, 0., f32::INFINITY] {
            assert!(
                FillTessellator::new()
                    .tessellate_path(
                        &valid,
                        &FillOptions::default().with_tolerance(tolerance),
                        &mut BuffersBuilder::new(&mut buffers, |v: FillVertex| v.position())
                    )
                    .is_err()
            );
            assert_eq!(buffers, original);
        }
        let mut invalid = Path::builder();
        rect(&mut invalid, 0., 1.);
        invalid.begin(point(f32::INFINITY, 0.));
        assert!(
            FillTessellator::new()
                .tessellate_path(
                    &invalid.build(),
                    &FillOptions::default(),
                    &mut BuffersBuilder::new(&mut buffers, |v: FillVertex| v.position())
                )
                .is_err()
        );
        assert_eq!(buffers, original);
        let mut unended = Path::builder();
        unended.begin(point(0., 0.));
        unended.line_to(point(1., 1.));
        assert!(
            StrokeTessellator::new()
                .tessellate_path(
                    &unended.build(),
                    &StrokeOptions::default(),
                    &mut BuffersBuilder::new(&mut buffers, |v: StrokeVertex| v.position())
                )
                .is_err()
        );
        assert_eq!(buffers, original);
    }

    #[test]
    fn varied_curvex_f32_open_strokes_cover_their_centerlines() {
        let mut state = 0x6173_ab29_u32;
        let mut random = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state as f32 / u32::MAX as f32
        };
        for case in 0..36 {
            let (p0, p3) = ([10_f32, 75.], [90_f32, 25.]);
            let c1 = if case == 0 {
                [15., -20.]
            } else {
                [-40. + 180. * random(), -100. + 250. * random()]
            };
            let c2 = if case == 0 {
                [85., 120.]
            } else {
                [-40. + 180. * random(), -100. + 250. * random()]
            };
            let d1 = [p0[0] - 2. * c1[0] + c2[0], p0[1] - 2. * c1[1] + c2[1]];
            let d2 = [c1[0] - 2. * c2[0] + p3[0], c1[1] - 2. * c2[1] + p3[1]];
            let second = (d1[0] * d1[0] + d1[1] * d1[1])
                .max(d2[0] * d2[0] + d2[1] * d2[1])
                .sqrt();
            let steps = (3. * second).sqrt().ceil().clamp(1., 256.) as usize;
            let inverse = 1. / steps as f32;
            let mut points = vec![point(p0[0], p0[1])];
            // Match Curvex's f32 Wang sampler, including multiply-by-inverse.
            for i in 1..=steps {
                let t = i as f32 * inverse;
                let mt = 1. - t;
                let (a, b, c, d) = (mt * mt * mt, 3. * mt * mt * t, 3. * mt * t * t, t * t * t);
                points.push(point(
                    a * p0[0] + b * c1[0] + c * c2[0] + d * p3[0],
                    a * p0[1] + b * c1[1] + c * c2[1] + d * p3[1],
                ));
            }
            let mut builder = Path::builder();
            builder.begin(points[0]);
            for &p in &points[1..] {
                builder.line_to(p);
            }
            builder.end(false);
            let path = builder.build();
            for width in [0.05, 7., 25.] {
                let mut mesh = VertexBuffers::<Point, u32>::new();
                let result = StrokeTessellator::new().tessellate_path(
                    &path,
                    &StrokeOptions::default()
                        .with_line_width(width)
                        .with_tolerance(0.05)
                        .with_line_join(
                            [LineJoin::Miter, LineJoin::Round, LineJoin::Bevel][case % 3],
                        )
                        .with_line_cap(
                            [LineCap::Butt, LineCap::Round, LineCap::Square][(case / 3) % 3],
                        ),
                    &mut BuffersBuilder::new(&mut mesh, |v: StrokeVertex| v.position()),
                );
                assert!(
                    result.is_ok(),
                    "case {case} width {width} controls {c1:?} {c2:?}: {result:?}"
                );
                assert!(!mesh.indices.is_empty());
                for edge in points.windows(2) {
                    let p = [
                        (edge[0].x as f64 + edge[1].x as f64) * 0.5,
                        (edge[0].y as f64 + edge[1].y as f64) * 0.5,
                    ];
                    let covered = mesh.indices.as_chunks::<3>().0.iter().any(|tri| {
                        (0..3).all(|i| {
                            let a = mesh.vertices[tri[i] as usize];
                            let b = mesh.vertices[tri[(i + 1) % 3] as usize];
                            (b.x as f64 - a.x as f64) * (p[1] - a.y as f64)
                                - (b.y as f64 - a.y as f64) * (p[0] - a.x as f64)
                                >= -1e-8
                        })
                    });
                    assert!(covered, "case {case} width {width} misses centerline {p:?}");
                }
            }
        }
    }
}
