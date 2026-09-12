// Release comparison of the position-only APIs used by Curvex.
// Three warmup runs, 21 samples; geometry construction is outside measurement.
use lyon_tessellation as lyon;
use planar_geometry::render::{path as own_path, tess as own};
use std::{hint::black_box, time::Instant};
#[derive(Clone)]
enum Step {
    Begin(f32, f32),
    Line(f32, f32),
    Cubic([f32; 6]),
    End(bool),
}
struct Case {
    name: &'static str,
    steps: Vec<Step>,
    stroke: bool,
    evenodd: bool,
    width: f32,
}
struct Geometry {
    vertices: Vec<[f32; 2]>,
    indices: Vec<u32>,
}
struct Stats {
    median: f64,
    p95: f64,
    vertices: usize,
    triangles: usize,
    area: f64,
}
fn rect(s: &mut Vec<Step>, x: f32, y: f32, w: f32, h: f32) {
    s.extend([
        Step::Begin(x, y),
        Step::Line(x + w, y),
        Step::Line(x + w, y + h),
        Step::Line(x, y + h),
        Step::End(true),
    ]);
}
fn circle(s: &mut Vec<Step>, cx: f32, cy: f32, r: f32) {
    let k = r * 0.5522848;
    s.extend([
        Step::Begin(cx + r, cy),
        Step::Cubic([cx + r, cy + k, cx + k, cy + r, cx, cy + r]),
        Step::Cubic([cx - k, cy + r, cx - r, cy + k, cx - r, cy]),
        Step::Cubic([cx - r, cy - k, cx - k, cy - r, cx, cy - r]),
        Step::Cubic([cx + k, cy - r, cx + r, cy - k, cx + r, cy]),
        Step::End(true),
    ]);
}
fn cases() -> Vec<Case> {
    let mut result = Vec::new();
    let mut steps = Vec::new();
    rect(&mut steps, 0., 0., 100., 80.);
    result.push(Case {
        name: "rectangle_fill",
        steps,
        stroke: false,
        evenodd: false,
        width: 2.,
    });
    let mut steps = Vec::new();
    rect(&mut steps, 0., 0., 100., 100.);
    for y in 0..4 {
        for x in 0..4 {
            rect(
                &mut steps,
                5. + x as f32 * 23.,
                5. + y as f32 * 23.,
                15.,
                15.,
            );
        }
    }
    result.push(Case {
        name: "compound_16_holes_fill",
        steps,
        stroke: false,
        evenodd: true,
        width: 2.,
    });
    let mut steps = Vec::new();
    circle(&mut steps, 0., 0., 50.);
    result.push(Case {
        name: "cubic_circle_fill",
        steps: steps.clone(),
        stroke: false,
        evenodd: false,
        width: 2.,
    });
    result.push(Case {
        name: "cubic_circle_stroke",
        steps,
        stroke: true,
        evenodd: false,
        width: 2.,
    });
    let mut steps = Vec::new();
    rect(&mut steps, 0., 0., 60., 50.);
    rect(&mut steps, 30., 20., 60., 50.);
    result.push(Case {
        name: "overlap_nonzero_fill",
        steps,
        stroke: false,
        evenodd: false,
        width: 2.,
    });
    result.push(Case {
        name: "bowtie_evenodd_fill",
        steps: vec![
            Step::Begin(0., 0.),
            Step::Line(80., 80.),
            Step::Line(0., 80.),
            Step::Line(80., 0.),
            Step::End(true),
        ],
        stroke: false,
        evenodd: true,
        width: 2.,
    });
    result.push(Case {
        name: "open_cubic_stroke",
        steps: vec![
            Step::Begin(0., 0.),
            Step::Cubic([0., 100., 100., -100., 100., 0.]),
            Step::End(false),
        ],
        stroke: true,
        evenodd: false,
        width: 2.,
    });
    let mut steps = vec![Step::Begin(10., 75.)];
    let (p0, c1, c2, p3) = ([10_f32, 75.], [15_f32, -20.], [85_f32, 120.], [90_f32, 25.]);
    // Exact f32 polyline passed to the real Curvex gradient stroke renderer.
    let inverse = 1. / 28_f32;
    for i in 1..=28 {
        let t = i as f32 * inverse;
        let mt = 1. - t;
        let (a, b, c, d) = (mt * mt * mt, 3. * mt * mt * t, 3. * mt * t * t, t * t * t);
        steps.push(Step::Line(
            a * p0[0] + b * c1[0] + c * c2[0] + d * p3[0],
            a * p0[1] + b * c1[1] + c * c2[1] + d * p3[1],
        ));
    }
    steps.push(Step::End(false));
    result.push(Case {
        name: "curvex_sampled_gradient_stroke",
        steps,
        stroke: true,
        evenodd: false,
        width: 7.,
    });
    for (n, stroke, name) in [
        (1200, true, "polyline_1200_stroke"),
        (4096, false, "polyline_4096_fill"),
    ] {
        let mut steps = Vec::new();
        for i in 0..n {
            let a = i as f32 * std::f32::consts::TAU / n as f32;
            let (x, y) = (100. * a.cos(), 100. * a.sin());
            steps.push(if i == 0 {
                Step::Begin(x, y)
            } else {
                Step::Line(x, y)
            });
        }
        steps.push(Step::End(true));
        result.push(Case {
            name,
            steps,
            stroke,
            evenodd: false,
            width: 2.,
        });
    }
    result
}
fn measure(mut render: impl FnMut() -> Result<Geometry, String>) -> Result<Stats, String> {
    let mut sample = render()?;
    let mut area = 0.;
    for t in sample.indices.chunks_exact(3) {
        let [a, b, c] = [
            sample.vertices[t[0] as usize],
            sample.vertices[t[1] as usize],
            sample.vertices[t[2] as usize],
        ];
        area += ((b[0] - a[0]) as f64 * (c[1] - a[1]) as f64
            - (b[1] - a[1]) as f64 * (c[0] - a[0]) as f64)
            .abs()
            * 0.5;
    }
    for _ in 0..3 {
        black_box(render()?);
    }
    let mut times = Vec::new();
    for _ in 0..21 {
        let start = Instant::now();
        sample = black_box(render()?);
        times.push(start.elapsed().as_secs_f64() * 1e6);
    }
    times.sort_by(f64::total_cmp);
    Ok(Stats {
        median: times[10],
        p95: times[19],
        vertices: sample.vertices.len(),
        triangles: sample.indices.len() / 3,
        area,
    })
}
fn native(case: &Case) -> Result<Stats, String> {
    let mut builder = own_path::Path::builder();
    for s in &case.steps {
        match *s {
            Step::Begin(x, y) => builder.begin(own_path::math::point(x, y)),
            Step::Line(x, y) => builder.line_to(own_path::math::point(x, y)),
            Step::Cubic(p) => builder.cubic_bezier_to(
                own_path::math::point(p[0], p[1]),
                own_path::math::point(p[2], p[3]),
                own_path::math::point(p[4], p[5]),
            ),
            Step::End(c) => builder.end(c),
        }
    }
    let path = builder.build();
    let mut ft = own::FillTessellator::new();
    let mut st = own::StrokeTessellator::new();
    measure(|| {
        let mut output = own::VertexBuffers::<[f32; 2], u32>::new();
        let constructor = |v: own::FillVertex| {
            let p = v.position();
            [p.x, p.y]
        };
        let result = if case.stroke {
            st.tessellate_path(
                &path,
                &own::StrokeOptions::default()
                    .with_line_width(case.width)
                    .with_tolerance(0.05),
                &mut own::BuffersBuilder::new(&mut output, constructor),
            )
        } else {
            ft.tessellate_path(
                &path,
                &own::FillOptions::default()
                    .with_tolerance(0.05)
                    .with_fill_rule(if case.evenodd {
                        own::FillRule::EvenOdd
                    } else {
                        own::FillRule::NonZero
                    }),
                &mut own::BuffersBuilder::new(&mut output, constructor),
            )
        };
        result.map_err(|e| e.to_string())?;
        Ok(Geometry {
            vertices: output.vertices,
            indices: output.indices,
        })
    })
}
fn baseline(case: &Case) -> Result<Stats, String> {
    let mut builder = lyon_path::Path::builder();
    for s in &case.steps {
        match *s {
            Step::Begin(x, y) => {
                builder.begin(lyon_path::math::point(x, y));
            }
            Step::Line(x, y) => {
                builder.line_to(lyon_path::math::point(x, y));
            }
            Step::Cubic(p) => {
                builder.cubic_bezier_to(
                    lyon_path::math::point(p[0], p[1]),
                    lyon_path::math::point(p[2], p[3]),
                    lyon_path::math::point(p[4], p[5]),
                );
            }
            Step::End(c) => builder.end(c),
        }
    }
    let path = builder.build();
    let mut ft = lyon::FillTessellator::new();
    let mut st = lyon::StrokeTessellator::new();
    measure(|| {
        let mut output = lyon::VertexBuffers::<[f32; 2], u32>::new();
        let result = if case.stroke {
            st.tessellate_path(
                &path,
                &lyon::StrokeOptions::default()
                    .with_line_width(case.width)
                    .with_tolerance(0.05),
                &mut lyon::BuffersBuilder::new(&mut output, |v: lyon::StrokeVertex| {
                    let p = v.position();
                    [p.x, p.y]
                }),
            )
        } else {
            ft.tessellate_path(
                &path,
                &lyon::FillOptions::default()
                    .with_tolerance(0.05)
                    .with_fill_rule(if case.evenodd {
                        lyon::FillRule::EvenOdd
                    } else {
                        lyon::FillRule::NonZero
                    }),
                &mut lyon::BuffersBuilder::new(&mut output, |v: lyon::FillVertex| {
                    let p = v.position();
                    [p.x, p.y]
                }),
            )
        };
        result.map_err(|e| e.to_string())?;
        Ok(Geometry {
            vertices: output.vertices,
            indices: output.indices,
        })
    })
}
fn json(result: Result<Stats, String>) -> String {
    match result {
        Ok(s) => format!(
            "{{\"medianUs\":{:.3},\"p95Us\":{:.3},\"vertices\":{},\"triangles\":{},\"area\":{:.6}}}",
            s.median, s.p95, s.vertices, s.triangles, s.area
        ),
        Err(e) => format!("{{\"error\":{e:?}}}"),
    }
}
fn main() {
    println!("[");
    let cases = cases();
    for (i, case) in cases.iter().enumerate() {
        println!(
            "{{\"case\":{:?},\"native\":{},\"lyon\":{}}}{}",
            case.name,
            json(native(case)),
            json(baseline(case)),
            if i + 1 == cases.len() { "" } else { "," }
        );
    }
    println!("]");
}
