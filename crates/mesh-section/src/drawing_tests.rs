use super::*;
use std::f64::consts::PI;

struct Builder {
    positions: Vec<f64>,
    indices: Vec<usize>,
}
impl Builder {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            indices: Vec::new(),
        }
    }
    fn point(&mut self, p: [f64; 3]) -> usize {
        self.positions.extend(p);
        self.positions.len() / 3 - 1
    }
    fn tri(&mut self, a: [f64; 3], b: [f64; 3], c: [f64; 3]) {
        for p in [a, b, c] {
            let i = self.point(p);
            self.indices.push(i);
        }
    }
    fn quad(&mut self, a: [f64; 3], b: [f64; 3], c: [f64; 3], d: [f64; 3]) {
        self.tri(a, b, c);
        self.tri(a, c, d);
    }
    fn view(&self) -> MeshView<'_> {
        MeshView::new(&self.positions, &self.indices)
    }
    fn cuboid(&mut self, lo: [f64; 3], hi: [f64; 3]) {
        let p = |x: usize, y: usize, z: usize| {
            [[lo[0], hi[0]][x], [lo[1], hi[1]][y], [lo[2], hi[2]][z]]
        };
        self.quad(p(0, 0, 0), p(0, 1, 0), p(1, 1, 0), p(1, 0, 0));
        self.quad(p(0, 0, 1), p(1, 0, 1), p(1, 1, 1), p(0, 1, 1));
        self.quad(p(0, 0, 0), p(1, 0, 0), p(1, 0, 1), p(0, 0, 1));
        self.quad(p(0, 1, 0), p(0, 1, 1), p(1, 1, 1), p(1, 1, 0));
        self.quad(p(0, 0, 0), p(0, 0, 1), p(0, 1, 1), p(0, 1, 0));
        self.quad(p(1, 0, 0), p(1, 1, 0), p(1, 1, 1), p(1, 0, 1));
    }
}

const N: usize = 16;

/// 20 mm cube with a 16-gon through hole of radius 5 along Z, outward oriented.
fn cube_with_hole() -> Builder {
    let mut m = Builder::new();
    let ring = |k: usize, r: Option<f64>, z: f64| {
        let a = 2. * PI * (k % N) as f64 / N as f64;
        let (c, s) = (a.cos(), a.sin());
        let t = r.unwrap_or_else(|| 10. / c.abs().max(s.abs()));
        let snap = |x: f64| (x * 1e9).round() / 1e9;
        [snap(10. + t * c), snap(10. + t * s), z]
    };
    for k in 0..N {
        let (o0, o1) = (|z| ring(k, None, z), |z| ring(k + 1, None, z));
        let (i0, i1) = (|z| ring(k, Some(5.), z), |z| ring(k + 1, Some(5.), z));
        m.quad(o0(20.), o1(20.), i1(20.), i0(20.));
        m.quad(o0(0.), i0(0.), i1(0.), o1(0.));
        m.quad(o0(0.), o1(0.), o1(20.), o0(20.));
        m.quad(i0(0.), i0(20.), i1(20.), i1(0.));
    }
    m
}

fn cylinder(n: usize) -> Builder {
    let mut m = Builder::new();
    let p = |k: usize, z: f64| {
        let a = 2. * PI * (k % n) as f64 / n as f64;
        [5. * a.cos(), 5. * a.sin(), z]
    };
    for k in 0..n {
        m.quad(p(k, 0.), p(k + 1, 0.), p(k + 1, 10.), p(k, 10.));
        m.tri([0., 0., 10.], p(k, 10.), p(k + 1, 10.));
        m.tri([0., 0., 0.], p(k + 1, 0.), p(k, 0.));
    }
    m
}

fn counts(s: &[DrawingSegment]) -> (usize, usize) {
    (
        s.iter().filter(|x| !x.hidden).count(),
        s.iter().filter(|x| x.hidden).count(),
    )
}

#[test]
fn cube_with_hole_front_and_right_show_hidden_bore_silhouettes() {
    let m = cube_with_hole();
    for view in [View::Front, View::Right] {
        let s = view_edges(&m.view(), view, ViewOptions::default()).unwrap();
        assert_eq!(counts(&s), (4, 2), "{view:?}: {s:?}");
        let hidden: Vec<_> = s.iter().filter(|x| x.hidden).collect();
        for h in &hidden {
            assert!(h.silhouette);
            assert_eq!(h.a[0], h.b[0]);
            assert!((h.b[1] - h.a[1] - 20.).abs() < 1e-9);
        }
        let mut xs: Vec<f64> = hidden.iter().map(|h| h.a[0].abs()).collect();
        xs.sort_by(f64::total_cmp);
        let expected = [5., 15.];
        for (x, e) in xs.iter().zip(expected) {
            assert!((x - e).abs() < 1e-9, "{xs:?}");
        }
    }
}

#[test]
fn cube_with_hole_top_shows_rim_without_hidden_lines() {
    let s = view_edges(&cube_with_hole().view(), View::Top, ViewOptions::default()).unwrap();
    assert_eq!(counts(&s), (4 + N, 0));
}

#[test]
fn cube_with_hole_iso_is_deterministic_and_hides_back_edges() {
    let m = cube_with_hole();
    let a = view_edges(&m.view(), View::Iso, ViewOptions::default()).unwrap();
    let b = view_edges(&m.view(), View::Iso, ViewOptions::default()).unwrap();
    assert_eq!(a, b);
    let (visible, hidden) = counts(&a);
    assert!(visible > 9 && hidden >= 3, "{visible} {hidden}");
}

#[test]
fn cylinder_front_view_has_two_silhouettes() {
    let s = view_edges(&cylinder(32).view(), View::Front, ViewOptions::default()).unwrap();
    assert_eq!(counts(&s), (4, 0), "{s:?}");
    let sil: Vec<_> = s.iter().filter(|x| x.silhouette).collect();
    assert_eq!(sil.len(), 2);
    let mut xs: Vec<f64> = sil.iter().map(|x| x.a[0]).collect();
    xs.sort_by(f64::total_cmp);
    assert!((xs[0] + 5.).abs() < 1e-9 && (xs[1] - 5.).abs() < 1e-9);
    let top = view_edges(&cylinder(32).view(), View::Top, ViewOptions::default()).unwrap();
    assert_eq!(counts(&top), (32, 0));
    assert!(top.iter().all(|x| !x.silhouette));
}

#[test]
fn section_through_cube_with_hole_is_hatched_annulus() {
    let m = cube_with_hole();
    let s = hatched_section(&m.view(), View::Top, 10., 1.).unwrap();
    assert_eq!(s.rings.len(), 2);
    let polygon = 0.5 * N as f64 * 25. * (2. * PI / N as f64).sin();
    assert!((s.area - (400. - polygon)).abs() < 1e-6, "{}", s.area);
    assert!(!s.hatch.is_empty());
    let inner = 5. * (PI / N as f64).cos();
    for h in &s.hatch {
        let mid = [(h[0][0] + h[1][0]) * 0.5, (h[0][1] + h[1][1]) * 0.5];
        assert!((-1e-9..=20. + 1e-9).contains(&mid[0]) && (-1e-9..=20. + 1e-9).contains(&mid[1]));
        assert!((mid[0] - 10.).hypot(mid[1] - 10.) > inner - 1e-9, "{h:?}");
    }
    let again = hatched_section(&m.view(), View::Top, 10., 1.).unwrap();
    assert_eq!(s.hatch, again.hatch);
    assert!(
        hatched_section(&m.view(), View::Top, 30., 1.)
            .unwrap()
            .rings
            .is_empty()
    );
}

#[test]
fn overhangs_skip_build_plate_and_find_slab_underside() {
    assert!(
        overhangs(&cube_with_hole().view(), 45., 1e-6)
            .unwrap()
            .triangles
            .is_empty()
    );
    let mut m = Builder::new();
    m.cuboid([0., 0., 0.], [2., 2., 5.]);
    m.cuboid([0., 0., 5.], [10., 10., 6.]);
    let r = overhangs(&m.view(), 45., 1e-6).unwrap();
    assert_eq!(r.triangles.len(), 2);
    assert!((r.area - 100.).abs() < 1e-9);
    assert!(overhangs(&m.view(), 90., 0.).is_err());
}

#[test]
fn invalid_options_are_rejected() {
    let m = cube_with_hole();
    let bad = ViewOptions {
        tolerance: 0.,
        ..ViewOptions::default()
    };
    assert!(view_edges(&m.view(), View::Top, bad).is_err());
    assert!(View::parse("bottom").is_err());
    assert_eq!(View::parse("iso").unwrap(), View::Iso);
    let tight = ViewOptions {
        max_work: 10,
        ..ViewOptions::default()
    };
    assert!(view_edges(&m.view(), View::Front, tight).is_err());
}
