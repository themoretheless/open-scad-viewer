# Развитая loft-библиотека с направляющими и касательными

Реализовано 2026-10-01 по подтверждённому запросу пользователя.

## Доступные операции

- Linear/aligned, natural cubic, clamped и closed rational loft.
- `natural_loft::clamped_control_tangents`: отдельный контроль касательной для каждого согласованного U-контроля на обоих концах. Cartesian dP/dt = sum(N_i*w_i*t_i)/sum(N_i*w_i), W′=0; производные по нормализованному V масштабируются диапазоном authored parameters.
- `guided_loft::interpolate`: полные направляющие, включая одну или несколько внутренних. Недостающие U=0/1 границы при нескольких направляющих сохраняются из базового loft. Допускается совместное задание направляющих и концевых полей касательных.
- Rust, JSON transport, WASM, TypeScript и Rush; JSON authoring и OBJ mesh export. STEP /9 проверен через BRep-интеграцию.

Rush-пример: `examples/rush/guided-loft-surface.r`. TypeScript: `guidedLoftNurbsCurves` и `controlTangentLoftNurbsCurves`. Rush: `guided_loft_surface` и `control_tangent_loft_surface`.

## Контракт и границы

2..11 непериодических 3D-сечений; 1..11 направляющих, включая синтезированные границы. До 32 контролей по каждой оси. Станции конечны и строго возрастают; станции направляющих заданы в [0,1]. Параметризация нормализуется, ориентация задаётся автором. Рациональные веса должны оставаться положительными.

Однородные пересечения сечений и направляющих, включая масштабы весов, должны совпадать точно. При заданных касательных однородные производные направляющих на концах должны совпадать с базовой поверхностью. Консервативное binary64-сравнение может отклонить математически совместимые условия после различного округления. Поиск пересечений, автоматическое переориентирование и snapping не выполняются.

Closed loft сохраняет циклические C2 jets, но не создаёт крышки или solid. Нет непрерывного сертификата погрешности, регулярности, отсутствия самопересечений или автоматического G1/G2 сопряжения с другой поверхностью.

## Проверки

- 402 native unit tests `nurbs-core` прошли.
- 29 регрессий STEP /9 импортера прошли, включая полюса, senses, periodic carriers, rational bodies и resource refusals.
- Все 3 теста `crates/brep-core/tests/guided_loft_step.rs` прошли: guided+tangents, рациональная/множественная матрица, общий замкнутый шов.
- 32 теста в 7 TypeScript-файлах прошли: guided/natural/clamped/closed/Gordon/section surfaces и Brotli packing. Проверены независимые quadratic и rational Hermite эталоны, отказы, Rush units/references, JSON/OBJ и импорт замкнутого шва из опубликованного WASM.
- `vue-tsc --noEmit` прошёл.
- Геометрический WASM пересобран с исправлением замкнутого STEP-ребра. Языковой WASM, embedded payload и identity согласованы и прошли интеграционный прогон. Во время параллельной публикации был временный mismatch длины; повторный прогон прошёл после согласования артефактов.

## Независимый STEP и визуальный результат

OpenCascade 8 через изолированное окружение `/tmp/loft-step-ocp` проверил 5 STEP fixtures: rational-guided, rational-natural, rational-clamped, multiple-interior-guides, closed-shared-seam. Каждый импорт даёт BRepCheck valid=true, одну поверхность и 0 solid. На 25 точках каждого случая максимальная ошибка позиции <=4.45e-16; для closed seam sampled V-производные <=1.78e-15. Guided quadratic также проверен по независимой аналитической формуле и производной. Скрипты: `scripts/verify-guided-loft-step.py`, `scripts/verify-loft-step-matrix.py`.

Через настоящий редактор импортированы и собраны guided, natural, clamped и closed Rush-примеры, перенесены в Mesh. Просмотрены четыре screenshots CPU SVG fallback: curved guided panel, natural/clamped panels и closed ring. Browser pageerror отсутствуют. Headless GPU-рендер не квалифицирован.

STEP, sampled references, OpenCascade JSON, browser JSON и изображения сохранены в `docs/qualification/guided-loft-2026-10-01/`. Это проверки конкретных fixtures, не универсальная непрерывная сертификация.

## Исправление, найденное при проверке

STEP importer ошибочно считал любое ребро с одной endpoint vertex degenerate. Теперь pole-edge требует также постоянной контрольной оболочки кривой. Полноценная замкнутая кривая сохраняется как неполярное общее ребро шва; проверки ориентации и surface correspondence сохранены.

## Последующее расширение

Автоматическое согласование, G1/G2 и capped solids добавлены по следующему запросу пользователя. См. `docs/design/advanced-loft-2026-10-02.md`; ограничения первой версии выше относятся к строгому `guided_loft::interpolate`.
