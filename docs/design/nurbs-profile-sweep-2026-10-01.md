# NURBS-0063: произвольный профиль с масштабом вдоль пути

Статус: integrated; визуальная проверка и STEP-квалификация ещё ожидаются.

`profile_sweep::checked(profile,path,scale,normal,sections,budget)` переносит
рациональный 3D-профиль по rotation-minimizing frames с положительным
рациональным dimensionless scale. Scale controls `[value,0,0]`; домены profile,
path и scale независимы. Profile authored относительно path start; initial
normal выбирает frame, не переориентирует исходный profile. Scale применяется
ко всем локальным координатам, включая tangent offsets.

Фиксированное число сечений2..32; closed paths минимум4, matching seam tangent
и equal scale endpoints. Holonomy correction сохраняет C0 seam. Fine comparison
использует fourfold stations и не доказывает непрерывную ошибку:
`continuousBound:false`. При превышении budget поверхность не возвращается.

JSON `surface_profile_sweep`; TypeScript `checkedProfileSweepNurbsSurface`
принимает dimensionless `NurbsScaleLaw`; graph/Rush `profile_sweep(profile,path,
scale:{degree,knots,values,weights},normal:...,sections:...,max_deviation:...)`.
Единицы scale и sections безразмерны; normal direction безразмерен; budget length.
Unknown record fields, размерный scale и неверные units отклоняются.

Примеры `profile-sweep.r`, `closed-profile-sweep.r`. Независимые equations,
рациональные weights, curved/closed paths, недостаточный budget и несовпадение
endpoint scale проверены в `tests/nurbsProfileSweep.test.ts`. Общий packaged
WASM-прогон423tests/13files прошёл, включая JSON/OBJ/PLY round-trip и реальный
программный editor entrypoint. Свидетельство:
`docs/qualification/sweep-coverage-2026-10-01/wasm-tests.json`.

Совместные scale/twist, четыре orientation modes, arc-length spacing и
автоматическое refinement расширяют семейство через `progressive_sweep`;
полный контракт и незавершённый объём описаны в
`docs/design/sweep-coverage-2026-10-01.md`. Continuous RMF approximation,
rounding-inclusive accuracy, regularity/self-intersection, solid topology и STEP
не квалифицированы текущими sampled tests.
