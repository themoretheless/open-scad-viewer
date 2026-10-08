# NURBS-0061: труба с круговыми сечениями постоянного радиуса

Статус integrated; полная continuous constant-radius квалификация впереди.
pipe::checked(path,radius,initial_normal,sections,max_deviation) возвращает
существующий CheckedSweep{surface:Option,report}. JSON surface_pipe.
Path3D, radius и budget положительные lengths, normal задаёт начальную
ориентацию frame, sections2..32. Для closed path требуется4..32 sections.

Общий framed_sweep::sample даёт start/tangent, включая one-sided handling;
единственное изменение этого helper — pub(crate) visibility. Rational
circle primitive создаёт полный профиль с центром path start и плоскостью
нормальной tangent. Общий checked_sweep переносит rigid circle sections
через double-reflection frames. Rational weights сохранены.

В авторских section stations radius постоянен, сечение нормально пути.
Между stations линейный rational loft не является exact constant-radius
surface/normal offset. Report сравнивает section controls на4x finer
sampled frame grid, не доказывает continuous error, exact RMF, rounding
или постоянство radius между stations. accepted:false возвращает
surface:null при sampled deviation>budget; конструктор не ослабляет budget.
Самопересечения, curvature-radius admissibility, cap/wall/solid topology
не сертифицированы; результат — side surface.

Native regression: cylinder radial equation и axial advance между
stations для straight path; normality/radius на32 bent-path sections,
coarse refusal vs fine acceptance; closed circle seam/closed report;
invalid radius, frame, budget и2D path refusals. Bounds проверяются
численно с1e-10mm tolerance, не являются certificates.

Интеграция WASM/TypeScript/Rush и программного editor entrypoint завершена.
Два примера включены в общий набор80fixtures с JSON/OBJ/PLY regression;
mesh coordinates сравниваются с допуском1e-5mm, indices и rational definitions сохраняются.
Четыре focused native tests и полный native-прогон360tests/2doctests прошли.
TypeScript typecheck и407packaged tests в23files прошли.
Rush запрещает length units у sections; radius и budget требуют length.
Первый packaged прогон выявил sections:32mm; contextual scalar check исправлен,
language WASM пересобран, полный повторный прогон прошёл.
Geometry WASM:9877786bytes, SHA256
5c1039d173b2ff59880eee0740642393e5d250398c5ea613e2a8071a98fd2cf8.

Continuous constant-radius tube/RMF, rounding-inclusive error, фактический
visual editor review и STEP остаются неквалифицированными.
