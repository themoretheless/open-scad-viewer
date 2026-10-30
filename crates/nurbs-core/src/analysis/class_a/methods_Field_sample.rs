//! class a operations: sample.
use super::*;
impl Field {
pub(super) fn sample(
        surface: &Surface,
        nu: usize,
        nv: usize,
        inset: f64,
        f: impl Fn(&Evaluation) -> Option<f64>,
        limits: &Limits,
    ) -> Result<Self> {
        check(nu >= 2 && nv >= 2, "Grid needs at least 2 cells per axis")?;
        check(
            (0. ..0.5).contains(&inset),
            "Grid inset must lie in [0, 0.5)",
        )?;
        if nu > limits.max_grid_axis || nv > limits.max_grid_axis {
            return Err(resource("class_a grid axis exceeds the sampling budget"));
        }
        if nu.checked_mul(nv).map_or(true, |c| c > limits.max_segments) {
            return Err(resource("class_a grid cell count exceeds the sampling budget"));
        }
        let (du, dv) = surface_domains(surface);
        let mut values = Vec::with_capacity((nu + 1) * (nv + 1));
        for j in 0..=nv {
            let tv = inset + (1. - 2. * inset) * j as f64 / nv as f64;
            let v = dv[0] + (dv[1] - dv[0]) * tv;
            for i in 0..=nu {
                let tu = inset + (1. - 2. * inset) * i as f64 / nu as f64;
                let u = du[0] + (du[1] - du[0]) * tu;
                let value = match surface.evaluate(u, v) {
                    Ok(ev) => f(&ev).unwrap_or(f64::NAN),
                    Err(_) => f64::NAN,
                };
                values.push(value);
            }
        }
        Ok(Self {
            nu,
            nv,
            inset,
            domain_u: du,
            domain_v: dv,
            values,
        })
    }
pub(super) fn at(&self, i: usize, j: usize) -> f64 {
        self.values[j * (self.nu + 1) + i]
    }
/// Доли ячеек (0..1 по сетке) → (u,v) с учётом inset-выборки.
    pub(super) fn uv(&self, fu: f64, fv: f64) -> [f64; 2] {
        let map = |f: f64| self.inset + (1. - 2. * self.inset) * f;
        [
            self.domain_u[0] + (self.domain_u[1] - self.domain_u[0]) * map(fu),
            self.domain_v[0] + (self.domain_v[1] - self.domain_v[0]) * map(fv),
        ]
    }
/// Марширующие квадраты: сегменты изолинии уровня `level` в (u,v).
    /// Число сегментов ограничено единым [`Budget`] поверх
    /// `limits.max_segments` (пункт 1065, стадия `class-a.marching-squares`).
    pub(super) fn marching_segments(&self, level: f64, limits: &Limits) -> Result<Vec<([f64; 2], [f64; 2])>> {
        let mut guard = Budget::with_iterations(limits.max_segments)?
            .guard("class-a.marching-squares");
        let mut segments = Vec::new();
        for j in 0..self.nv {
            for i in 0..self.nu {
                let corner = [
                    self.at(i, j),
                    self.at(i + 1, j),
                    self.at(i + 1, j + 1),
                    self.at(i, j + 1),
                ];
                if corner.iter().any(|x| !x.is_finite()) {
                    continue;
                }
                // Узлы ячейки в долях области: 0=(i,j) 1=(i+1,j) 2=(i+1,j+1) 3=(i,j+1).
                let node = [
                    [i as f64 / self.nu as f64, j as f64 / self.nv as f64],
                    [(i + 1) as f64 / self.nu as f64, j as f64 / self.nv as f64],
                    [(i + 1) as f64 / self.nu as f64, (j + 1) as f64 / self.nv as f64],
                    [i as f64 / self.nu as f64, (j + 1) as f64 / self.nv as f64],
                ];
                let edges = [(0, 1), (1, 2), (2, 3), (3, 0)];
                let mut hits = Vec::with_capacity(4);
                for &(a, b) in &edges {
                    let (fa, fb) = (corner[a] - level, corner[b] - level);
                    // Строгая смена знака: переход ровно через узел сетки
                    // пропускаем в обеих соседних ячейках — иначе узел
                    // порождает дублирующиеся фрагменты изолинии.
                    if fa * fb < 0. {
                        let t = fa / (fa - fb);
                        hits.push(self.uv(
                            node[a][0] + t * (node[b][0] - node[a][0]),
                            node[a][1] + t * (node[b][1] - node[a][1]),
                        ));
                    }
                }
                if hits.len() == 2 {
                    guard.tick()?;
                    segments.push((hits[0], hits[1]));
                } else if hits.len() == 4 {
                    // Неоднозначная ячейка: сцепляем по парам соседних рёбер.
                    guard.tick()?;
                    segments.push((hits[0], hits[1]));
                    guard.tick()?;
                    segments.push((hits[2], hits[3]));
                }
            }
        }
        Ok(segments)
    }
}
