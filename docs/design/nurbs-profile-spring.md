# Пружина с произвольным рациональным профилем

`nurbs_core::profile_spring::approximate(profile,Request)` переносит исходный rational profile в нормальной плоскости аналитической цилиндрической спирали. Ось спирали +Z, radius>0, signed height и turns≠0; center/phase задают размещение. Локальный X направлен радиально наружу, Y=tangent×radial. Profile — 2D или 3D XY с точными Z-controls=0, не более 32 controls; сохраняются degree, knots, weights, periodic storage. Это поверхность по профилю, не полая/сплошная B-rep пружина с крышками.

Обозначим ω=2π*turns, L=hypot(Rω,H), α=Rω/L, β=H/L, θ=phase+ωt. Целевая поверхность:

- X=Cx+(R+x(u))*cosθ−β*y(u)*sinθ;
- Y=Cy+(R+x(u))*sinθ+β*y(u)*cosθ;
- Z=Cz+H*t−α*y(u).

При всех t профиль лежит в плоскости, перпендикулярной касательной helix; кадр ортонормален в вещественной арифметике. Отрицательные turns дают противоположную навивку, отрицательная H — убывающий Z, H=0 — плоская coil. Наложение витков и вырождение при больших/осевых профилях не исключаются.

Каждая исходная control trajectory интерполируется cubic Hermite по t. Веса исходного profile постоянны по T, поэтому итоговый Surface остаётся rational по U, cubic по T и сохраняет исходный профиль на interpolation stations. Патчи используют не более десяти spans (31 T-controls), исходный U domain и общий T∈[0,1]. Соседние патчи имеют одинаковые boundary controls, вычисленные по одним исходным station values. Ресурс max_spans=1..1024; разбиение удваивается до estimate или span budget. При budget stop возвращается вся спираль до t=1, не частичный результат.

Четвёртая T-производная каждой control trajectory имеет норму |ω|⁴*hypot(R+control.x,β*control.y). Положительная rational basis ограничивает всю поверхность максимумом этих норм. Отсюда global cubic Hermite remainder ≤extent*(|ω|*max_cell_width)^4/384. max_cell_width берётся из фактической authored station grid; эта же оценка управляет refinement и итоговым Stop. Report.real_arithmetic_error_estimate предполагает точные вещественные операции и trig samples. Libm, controls/evaluation rounding и формальный C1 certificate между патчами сюда не включены; EstimateReached не является полной rounding-inclusive гарантией tolerance.

Конечность inputs, ненулевой tangent, положительные radius/tolerance и budgets проверяются. Surface validation ограничивает численную представимость и control net. Выход — полный набор поверхностей, без cap, self-intersection, clearance, manifold/solid audit или автоматически подтверждённого замыкания для H=0. Нормальный frame является явной analytic convention, не RMF transport.

Независимые regressions проверяют rational ellipse profile против analytic target, normal-plane и distance-to-center invariants в boundary stations, signed turns/height/flat coil, точное совпадение patch boundaries, отсутствие изменения исходного profile, полный T domain при budget stop и invalid inputs. Проверка 2026-10-14: три группы integration tests прошли offline/locked, no-default-features. Проверены обе навивки, positive/negative/zero height, 2D и 3D XY rational profiles, normal-plane/metric invariants, boundaries, non-power-of-two span budget и invalid inputs. Hash исходника конструктора совпал с предшествующим снимком; последняя версия тестов с 3D XY case повторно прошла. NURBS-0070: native в указанном контракте surface constructor.
