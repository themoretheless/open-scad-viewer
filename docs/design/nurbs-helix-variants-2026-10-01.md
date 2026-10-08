# Эллиптическая и коническая винтовые линии — NURBS-0024 / NURBS-0022

Статус новых вариантов: integrated. Rust/WASM/TypeScript/Rush, программный
вход редактора и JSON/OBJ/PLY проверены; visual/STEP и непрерывная оценка
с учётом округления остаются незакрытыми.
Используют общий Hermite-генератор цилиндрической helix, без копирования
алгоритма построения кривой и без добавления новых каталожных ID.

`helix::approximate_elliptic(center,radius_x,radius_y,height,turns,
phase_degrees,max_deviation)` задаёт постоянные положительные полуоси XY.
P(t)=center+[rx*cos(phase+ωt),ry*sin(phase+ωt),height*t]. Ось фиксирована Z.

`helix::approximate_conical(center,start_radius,end_radius,height,turns,
phase_degrees,max_deviation)` задаёт r(t)=(1−t)*r0+t*r1 и круговые XY-сечения.
Радиусы конечны и неотрицательны, хотя бы один строго положителен. Нулевая
начальная/конечная вершина допускается; регулярность там не сертифицирована.
Это линейный радиус по t с постоянной осевой скоростью, без variable pitch.

Length-параметры — center/radii/height/budget; turns безразмерен и ненулевой,
phase в градусах. До85кубических spans/256контролей. Аналитические производные
точек включают производную радиуса, а не только вращательную составляющую.
Для каждой координаты M_d≤max(r_d)*|ω|^4+4*|r'_d|*|ω|^3.
Оценка ideal Hermite remainder — hypot(Mx,My)/(384*spans^4). Для эллипса
производная радиуса равна нулю. Все оценки вычисляются binary64 и не включают
округление sin/cos, контролей или вычисления NURBS: continuousBound:false,
roundingCertified:false. Непредставимые оценки и бюджет сверх85spans отклоняются.

JSON: `curve_elliptic_helix` с radius_x/radius_y и `curve_conical_helix` с
start_radius/end_radius, остальные поля совпадают с curve_helix. Возвращают
curve/report. Native regression сравнивает1001точку с независимыми формулами
для эллиптической helix, расширяющегося конуса и конуса до нулевой вершины.
Проверяются недопустимые радиусы; существующие цилиндрические тесты сохранены.
Всего5целевых helix-тестов прошли. Выборка не заменяет полный error certificate.

Добавлены TypeScript-адаптеры `approximateEllipticHelixNurbsCurve` и
`approximateConicalHelixNurbsCurve`; оба возвращают curve и весь типизированный
NurbsHelixApproximation.report с несертфицированными флагами.
`vue-tsc --noEmit` прошёл. Упакованный геометрический WASM с вариантами
ещё не проверен; graph/Rush/editor и экспорт остаются следующими шагами.

Добавлены graph-schema, исполнение с сохранением constructionReport и Rush
`elliptic_helix_curve` / `conical_helix_curve`. Все поля обязательны; радиусы
используют длину, turns безразмерен, phase angular. Примеры:
`examples/rush/elliptic-helix-extrusion.r` и `conical-helix-extrusion.r`.
TypeScript, native frontend tests обоих вариантов и цилиндрического случая
прошли; runtime/languages-bridge прошли cargo check. Упакованный WASM,
единицы через пакет, геометрия через TS, редактор и экспорт ещё ожидаются.

Пакетная проверка завершена: 273 теста в 8 файлах прошли, включая
60 фиксированных Rush-примеров и прежние sweep/helix-конструкторы. Проверены
независимые формулы обоих вариантов, размерности через языковой WASM,
constructionReport и запрет сертифицированного error bound при этих
аппроксимациях. JSON сохраняет hash и определения; OBJ/PLY сохраняют
индексы и координаты отображаемой сетки с допуском 1e-5 mm. Это
программный вход редактора, а не визуальная проверка viewport.
`vue-tsc --noEmit` и `git diff --check` прошли. Геометрический WASM:
9802632 bytes, SHA256 `2dde6fd4c7c49175c3550194bd46b59db216b7c9545f9ecdac428555acb65a58`.
