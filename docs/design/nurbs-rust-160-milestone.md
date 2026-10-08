# Рубеж 160 полезных возможностей Rust NURBS

Реестр выбранного набора 180: **160 реализованных, 20 planned**. Общий каталог 200: 79 integrated + 81 native, 40 planned. Набор 150 сохранён: 130 реализованных, 20 planned. Полная цель 180 не объявляется завершённой.

По сравнению с исходным счётчиком 152 закрыты восемь самостоятельных пунктов:

| ID | Возможность | Native API | Доказательства |
| --- | --- | --- | --- |
| N74 | Связный смещённый профиль с bevel joins и closure | `curve_offset_wire::bevel_wire` | 3 независимых profile_offset + 5 существующих wire tests; nurbs-profile-offset.md |
| N99 | Общая C2 поверхность S(u,v), включая неполиномиальные функции | `function_surface::approximate` | 4 tests, paraboloid exact error и sqrt graph; nurbs-function-surface.md |
| N166 | Самопересечения кривой: доказательство отсутствия или witness | `curve_self_intersection::inspect` | 4 tests, rational graph, curved closed circle, bow tie, exhaustion; nurbs-self-intersection-audits.md |
| N167 | Самопересечения/многократное покрытие поверхности | `surface_self_intersection::inspect` | 3 tests + 6 injectivity tests, graph rotation, exact interior fold; nurbs-self-intersection-audits.md |
| N172 | Перенос пространственного контура в UV с continuous admission | `curve_pullback::project` | 3 tests, rational plane and curved support; nurbs-curve-pullback.md |
| N193 | Общие indexed seam vertices при тесселяции патчей | `shared_tessellation::tessellate` | 3 tests, different knots/reversal/mismatch; nurbs-shared-tessellation.md |
| N196 | Ориентация оболочек с проверенными boundary/nesting prerequisites | `brep_core::volume_validity::inspect` | 3 targeted tests, outer cube, inner cavity, invalid/exhausted budgets; nurbs-normal-orientation.md |
| N198 | Непрерывный композиционный сертификат отклонения с округлением | `curve_surface_agreement::verify` | 14 targeted tests, rational/multispan/periodic/witness/exhaustion; nurbs-rounding-deviation-certificate.md |

Новые и отдельно вынесенные Native API проверены также без default features:

```sh
cargo test --manifest-path crates/Cargo.toml -p nurbs-core --no-default-features --test function_surface --test curve_pullback --test curve_self_intersection --test surface_self_intersection --test shared_tessellation --test profile_offset --quiet
cargo check --manifest-path crates/Cargo.toml -p brep-core --no-default-features --quiet
node scripts/audit-nurbs-catalog.mjs
git diff --check
```

Все **20 интеграционных tests без default features** прошли; brep-core check без default features прошёл. Дополнительно прошли 28 целевых library tests (5 wire, 6 injectivity, 14 composition, 3 orientation): 48 разных целевых проверок суммарно. Полный repository suite, WASM, viewer/editor и экспорт этим рубежом не квалифицируются.

Области поддержки и пределы перечислены в документах каждого API. Существенно: S(u,v) certificate требует корректных oracle intervals и global derivative bounds; самопересечения могут оставаться Unresolved, если достаточное доказательство не получено; UV inverse не обязательно единственный; shared tessellation относится к full rectangular charts и явно заданным affine seams; offset profile не является очисткой region topology. Эти исходы не выдаются за положительные сертификаты. Оставшиеся N188, N197, строгая квалификация gear/thread, general SSI и другие пункты сохраняют planned.
