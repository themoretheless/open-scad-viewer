use super::Surface;
/// Exact rational R(q)=(q³+9q)/(1+q²), Y=q² on -2<=q<=2.
/// R'=(q²-3)²/(1+q²)², but the physical curve is regular through Y.
/// Degree elevation gives strictly positive integer weights; an exact common
/// XYZ scale makes all original Cartesian poles integer as well.
pub fn fixture() -> (Surface, Surface) {
    fn choose(n: usize, k: usize) -> i64 {
        let mut x = 1;
        for j in 0..k {
            x = x * (n - j) as i64 / (j + 1) as i64;
        }
        x
    }
    fn coefficient(p: &[i64], i: usize) -> i64 {
        p.iter()
            .enumerate()
            .take(i + 1)
            .map(|(k, c)| {
                let den = choose(6, k);
                assert_eq!(60 % den, 0);
                c * choose(i, k) * (60 / den)
            })
            .sum()
    }
    let mut poles = Vec::new();
    let mut weights = Vec::new();
    for i in 0..=6 {
        let w = coefficient(&[5, -16, 16], i);
        assert!(w > 0);
        assert_eq!(23100 % w, 0);
        let x = (23100 / w) * coefficient(&[-26, 84, -96, 64], i);
        let y = (23100 / w) * coefficient(&[20, -144, 400, -512, 256], i);
        poles.push(
            (0..3)
                .map(|j| vec![x as f64, y as f64, 0.5 * j as f64])
                .collect(),
        );
        weights.push(vec![w as f64; 3]);
    }
    let a = Surface {
        degree_u: 6,
        degree_v: 2,
        knots_u: [vec![0.; 7], vec![1.; 7]].concat(),
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: poles,
        weights,
        periodic_u: false,
        periodic_v: false,
    };
    let mut b = a.clone();
    for (i, row) in b.control_points.iter_mut().enumerate() {
        for p in row {
            p[2] += 1.;
        }
        for w in &mut b.weights[i] {
            *w *= 2f64.powi(6 - i as i32);
        }
    }
    (a, b)
}
