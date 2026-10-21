# Замкнутый sweep в Rust — аудит существующего конструктора

`brep_core::analytic::progressive_profile_body_with_rmf_policy` переносит authored rational loops вдоль замкнутой траектории с refinement и строит periodic B-rep incidence. Замкнутый путь требует согласованной касательной и совместимых endpoint laws; rotation-minimizing frames распределяют holonomy по пути. Первый профиль задан в исходных world coordinates относительно начала пути.

В конце повторяется первое сечение, но топология повторно использует исходные вершины и рёбра. Крышек нет. Внешний контур образует outer shell; отверстия образуют inner shells того же Body. Общие edges имеют по два coedge uses, включая seam. Сохраняются рациональные profile degrees и weights. Бюджеты sections/refinement и 1024 faces ограничивают ресурс; непринятая аппроксимация не передаётся в авторство Body.

Body-режим предоставляет source-owned continuous error для original RMF transport и сохранённых стенок. Проверяются continuous_bound и continuous_error_upper, затем итоговые boundary_error_upper и boundary_error_within_budget после сохранения/декомпозиции. Отсутствие доказательства или превышение preview budget не допускает построения; итоговая boundary receipt читается отдельно. G1/G2 seam, global regularity, embedding и solid этим не подтверждаются. STEP round-trip относится к конкретному fixture.

Независимый тест `crates/brep-core/tests/closed_sweep_native.rs` использует неплоскую polynomial Bézier-траекторию пятой степени. Её позиции и касательные вычисляются отдельно по формулам Bernstein; world points границ стенок должны сохранять радиус и лежать в нормальной плоскости пути. Дополнительно проверяются полное замыкание incidence, по два использования каждого edge, сохранение исходника и отказы при endpoint scale mismatch и недостаточном refinement.

Существующий `closed_progressive_hollow_body_owns_periodic_seam_and_inner_shell` в `rational_section_loft.rs` отдельно проверяет полый замкнутый sweep, inner shell, отсутствие caps и STEP void topology. Прогоны завершены: 3 closed-sweep tests и 19 section-body tests прошли. NURBS-0067 имеет статус native для указанного конструктивного контракта.

Открытый cross-section также поддержан на уровне поверхности через `nurbs_core::framed_sweep::checked_sweep`: rational профиль переносится вдоль закрытого пути, storage seam совпадает по controls и weights. Независимый неплоский тест проверяет метрику сечений, положение центра, periodic storage и отказ при недостигнутом sampled budget. Он не требует создавать Body для открытого профиля.

## Проверенный нативный рубеж — 2026-10-15

На том же неплоском пути, профиле и сетке 129 sections default 512-step proof дала continuous upper 0.1255099434, а политика 4096/100000/1000000 — 0.0166506896 при неизменном sampled deviation 0.0006636334. Допуск 0.03 не увеличивался. Полный Body-тест требует итоговую boundary_error_within_budget=Some(true) и boundary upper <=0.03. Сечения проверяются по независимым Bernstein formulas исходного пути, включая owned seam и обе coedge uses каждого ребра.

Surface-only framed_sweep остаётся sampled API; его periodic C0 storage seam проверен отдельно. Команды, SHA-256, полный вывод и количественная диагностика сохранены в docs/qualification/nurbs-closed-sweep-2026-10-15/.
