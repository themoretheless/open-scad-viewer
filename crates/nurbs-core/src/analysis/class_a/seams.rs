use super::*;

/// Граница патча для зебра-выборки шва.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeamEdge {
    UMin,
    UMax,
    VMin,
    VMax,
}

impl SeamEdge {
    /// (u,v) в долях домена для параметра шва t ∈ [0,1] и касательная d(u,v)/dt.
    pub(super) fn point(&self, t: f64) -> ([f64; 2], [f64; 2]) {
        match self {
            SeamEdge::UMin => ([0., t], [0., 1.]),
            SeamEdge::UMax => ([1., t], [0., 1.]),
            SeamEdge::VMin => ([t, 0.], [1., 0.]),
            SeamEdge::VMax => ([t, 1.], [1., 0.]),
        }
    }
}

/// Одна выборка по обе стороны шва.
#[derive(Clone, Copy, Debug)]
pub struct SeamSample {
    pub t: f64,
    /// Позиционный зазор |p_a − p_b|.
    pub position_gap: f64,
    /// Угол между нормалями, рад.
    pub normal_angle: f64,
    /// Относительный скачок кривизны нормальных сечений поперёк шва.
    pub curvature_jump: f64,
}

/// (964) Зебра-выборка шва двух поверхностей по общей граничной кривой.
#[derive(Clone, Debug)]
pub struct SeamReport {
    pub samples: Vec<SeamSample>,
    pub max_position_gap: f64,
    pub max_normal_angle: f64,
    pub max_curvature_jump: f64,
}

/// Нормальная кривизна в мировом направлении d (единица, в касательной плоскости).
pub(super) fn normal_curvature(ev: &Evaluation, d: [f64; 3]) -> Option<f64> {
    let (su, sv) = ev.first_derivatives()?;
    let (suu, suv, svv) = ev.second_derivatives()?;
    let n = ev.unit_normal()?;
    let (e, f, g) = (dot(su, su), dot(su, sv), dot(sv, sv));
    let det = e * g - f * f;
    if det <= 1e-28 {
        return None;
    }
    // Параметрическое направление: G·(du,dv) = (d·Su, d·Sv).
    let rhs_u = dot(d, su);
    let rhs_v = dot(d, sv);
    let du = (g * rhs_u - f * rhs_v) / det;
    let dv = (e * rhs_v - f * rhs_u) / det;
    let l = dot(n, suu);
    let m = dot(n, suv);
    let nn = dot(n, svv);
    let first = e * du * du + 2. * f * du * dv + g * dv * dv;
    if first <= 1e-28 {
        return None;
    }
    Some((l * du * du + 2. * m * du * dv + nn * dv * dv) / first)
}

pub fn seam_zebra(
    surface_a: &Surface,
    edge_a: SeamEdge,
    surface_b: &Surface,
    edge_b: SeamEdge,
    samples: usize,
    limits: &Limits,
) -> Result<SeamReport> {
    check(samples >= 2, "Seam zebra needs at least two samples")?;
    if samples > limits.max_polyline_points {
        return Err(resource("class_a seam sample count exceeds the budget"));
    }
    let eval_at = |s: &Surface, edge: SeamEdge, t: f64| -> Result<Evaluation> {
        let (frac, _) = edge.point(t);
        let (du, dv) = surface_domains(s);
        s.evaluate(
            du[0] + (du[1] - du[0]) * frac[0],
            dv[0] + (dv[1] - dv[0]) * frac[1],
        )
    };
    let mut report = SeamReport {
        samples: Vec::with_capacity(samples),
        max_position_gap: 0.,
        max_normal_angle: 0.,
        max_curvature_jump: 0.,
    };
    for k in 0..samples {
        let t = k as f64 / (samples - 1) as f64;
        let a = eval_at(surface_a, edge_a, t)?;
        let b = eval_at(surface_b, edge_b, t)?;
        let gap = norm(sub(a.point, b.point));
        let (Some(na), Some(nb)) = (a.unit_normal(), b.unit_normal()) else {
            return Err(crate::numeric_err(
                "class_a seam zebra hit a degenerate surface normal",
            ));
        };
        let angle = dot(na, nb).clamp(-1., 1.).acos();
        // Кривизна нормальных сечений поперёк шва: d ⟂ касательной шва.
        let (_, dir_a) = edge_a.point(t);
        let (_, dir_b) = edge_b.point(t);
        let tangent_a = a.first_derivatives().and_then(|(su, sv)| {
            unit([
                su[0] * dir_a[0] + sv[0] * dir_a[1],
                su[1] * dir_a[0] + sv[1] * dir_a[1],
                su[2] * dir_a[0] + sv[2] * dir_a[1],
            ])
        });
        let tangent_b = b.first_derivatives().and_then(|(su, sv)| {
            unit([
                su[0] * dir_b[0] + sv[0] * dir_b[1],
                su[1] * dir_b[0] + sv[1] * dir_b[1],
                su[2] * dir_b[0] + sv[2] * dir_b[1],
            ])
        });
        let jump = match tangent_a.zip(tangent_b) {
            Some((ta, tb)) => {
                let t_mid = unit([
                    0.5 * (ta[0] + tb[0]),
                    0.5 * (ta[1] + tb[1]),
                    0.5 * (ta[2] + tb[2]),
                ])
                .unwrap_or(ta);
                let da = unit(cross(na, t_mid));
                let db = unit(cross(nb, t_mid));
                match da.zip(db).and_then(|(da, db)| {
                    normal_curvature(&a, da).zip(normal_curvature(&b, db))
                }) {
                    Some((ka, kb)) => {
                        (ka - kb).abs() / ka.abs().max(kb.abs()).max(1e-12)
                    }
                    None => 0.,
                }
            }
            None => 0.,
        };
        report.max_position_gap = report.max_position_gap.max(gap);
        report.max_normal_angle = report.max_normal_angle.max(angle);
        report.max_curvature_jump = report.max_curvature_jump.max(jump);
        report.samples.push(SeamSample {
            t,
            position_gap: gap,
            normal_angle: angle,
            curvature_jump: jump,
        });
    }
    Ok(report)
}

/// (962) Конфиг допусков порогового класс-A валидатора.
#[derive(Clone, Copy, Debug)]
pub struct ClassATolerances {
    /// Класс A: позиционный зазор шва.
    pub a_position: f64,
    /// Класс A: угол между нормалями, рад.
    pub a_normal_angle: f64,
    /// Класс A: относительный скачок кривизны.
    pub a_curvature_jump: f64,
    /// Класс B (между A и C), те же величины.
    pub b_position: f64,
    pub b_normal_angle: f64,
    pub b_curvature_jump: f64,
}

impl Default for ClassATolerances {
    fn default() -> Self {
        Self {
            a_position: 0.01,
            a_normal_angle: 0.01,
            a_curvature_jump: 0.05,
            b_position: 0.05,
            b_normal_angle: 0.05,
            b_curvature_jump: 0.2,
        }
    }
}

/// Класс качества шва/грани.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SurfaceClass {
    /// Выше класса C (хуже B): не прошёл даже B-пороги.
    C = 0,
    B = 1,
    A = 2,
}

impl SurfaceClass {
    pub fn name(self) -> &'static str {
        match self {
            SurfaceClass::A => "A",
            SurfaceClass::B => "B",
            SurfaceClass::C => "C",
        }
    }
}

/// (962) Классификация шва G0/G1/G2 по конфигурируемым порогам.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeamContinuity {
    /// Даже G0 не достигнут.
    None,
    G0,
    G1,
    G2,
}

impl SeamContinuity {
    pub fn name(self) -> &'static str {
        match self {
            SeamContinuity::None => "none",
            SeamContinuity::G0 => "G0",
            SeamContinuity::G1 => "G1",
            SeamContinuity::G2 => "G2",
        }
    }
}

pub fn classify_seam(
    report: &SeamReport,
    position_tol: f64,
    normal_angle_tol: f64,
    curvature_jump_tol: f64,
) -> SeamContinuity {
    if report.max_position_gap > position_tol {
        SeamContinuity::None
    } else if report.max_normal_angle > normal_angle_tol {
        SeamContinuity::G0
    } else if report.max_curvature_jump > curvature_jump_tol {
        SeamContinuity::G1
    } else {
        SeamContinuity::G2
    }
}

/// (963) Результат пороговой валидации: класс + список нарушений.
#[derive(Clone, Debug)]
pub struct ClassAReport {
    pub class: SurfaceClass,
    pub continuity: SeamContinuity,
    pub violations: Vec<Defect>,
}

impl ClassAReport {
    /// Простая JSON-подобная сериализация без внешних зависимостей.
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(128 + 96 * self.violations.len());
        out.push_str("{\"class\":\"");
        out.push_str(self.class.name());
        out.push_str("\",\"continuity\":\"");
        out.push_str(self.continuity.name());
        out.push_str("\",\"violations\":[");
        for (i, v) in self.violations.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let kind = match v.kind {
                DefectKind::SeamPositionGap => "position_gap",
                DefectKind::SeamNormalAngle => "normal_angle",
                DefectKind::SeamCurvatureJump => "curvature_jump",
                DefectKind::HighlightKink => "highlight_kink",
                DefectKind::ReflectionJump => "reflection_jump",
                DefectKind::IsophoteUneven => "isophote_uneven",
                DefectKind::Dent => "dent",
            };
            out.push_str(&format!(
                "{{\"type\":\"{}\",\"t\":{},\"u\":{},\"v\":{},\"value\":{},\"threshold\":{}}}",
                kind,
                if v.uv[1].is_nan() {
                    format!("{}", v.uv[0])
                } else {
                    "null".to_string()
                },
                v.uv[0],
                v.uv[1],
                v.value,
                v.threshold,
            ));
        }
        out.push_str("]}");
        out
    }
}

/// (962/963) Пороговая класс-A валидация шва: класс A/B/C + нарушения.
pub fn validate_seam_class(
    report: &SeamReport,
    tolerances: &ClassATolerances,
) -> Result<ClassAReport> {
    check(
        tolerances.a_position >= 0.
            && tolerances.a_normal_angle >= 0.
            && tolerances.a_curvature_jump >= 0.
            && tolerances.b_position >= tolerances.a_position
            && tolerances.b_normal_angle >= tolerances.a_normal_angle
            && tolerances.b_curvature_jump >= tolerances.a_curvature_jump,
        "Class-A tolerances must be non-negative and B no tighter than A",
    )?;
    let mut violations = Vec::new();
    // Нарушения фиксируем относительно B-порогов (всё, что мешает классу B и выше),
    // а класс считаем отдельно; нарушение A-порога помечаем тем же типом.
    for s in &report.samples {
        let uv = [s.t, f64::NAN];
        if s.position_gap > tolerances.a_position {
            violations.push(Defect {
                kind: DefectKind::SeamPositionGap,
                uv,
                value: s.position_gap,
                threshold: tolerances.a_position,
            });
        }
        if s.normal_angle > tolerances.a_normal_angle {
            violations.push(Defect {
                kind: DefectKind::SeamNormalAngle,
                uv,
                value: s.normal_angle,
                threshold: tolerances.a_normal_angle,
            });
        }
        if s.curvature_jump > tolerances.a_curvature_jump {
            violations.push(Defect {
                kind: DefectKind::SeamCurvatureJump,
                uv,
                value: s.curvature_jump,
                threshold: tolerances.a_curvature_jump,
            });
        }
    }
    let class = if report.max_position_gap <= tolerances.a_position
        && report.max_normal_angle <= tolerances.a_normal_angle
        && report.max_curvature_jump <= tolerances.a_curvature_jump
    {
        SurfaceClass::A
    } else if report.max_position_gap <= tolerances.b_position
        && report.max_normal_angle <= tolerances.b_normal_angle
        && report.max_curvature_jump <= tolerances.b_curvature_jump
    {
        SurfaceClass::B
    } else {
        SurfaceClass::C
    };
    let continuity = classify_seam(
        report,
        tolerances.b_position,
        tolerances.b_normal_angle,
        tolerances.b_curvature_jump,
    );
    Ok(ClassAReport {
        class,
        continuity,
        violations,
    })
}
