# NURBS-0062: труба переменного радиуса

Нативный API pipe::checked_variable(path,radius,normal,sections,budget).
JSON surface_variable_pipe использует radius как rational curve.
Путь 3D; закон радиуса — rational scalar curve с controls [r,0,0], r>0.
Положительные веса обеспечивают положительность закона на активном domain.
Домены path и radius независимо нормализуются; закон задан по параметру,
не по длине пути. sections2..32, closed path минимум4. Budget>0.

Сечения unit-circle переносятся shared double-reflection frames, затем
масштабируются относительно центров пути. Closed paths требуют точного
совпадения endpoint radius и повторяют первое сечение в конце.
Linear rational loft обеспечивает круговые authored sections, но между
ними не является точной трубой заданного радиуса. Refinement сравнивает
controls с четырёхкратной sampled сеткой и включает изменение радиуса.
accepted:false даёт surface:null; budget не ослабляется.
Report continuousBound:false; непрерывные ошибки, rounding, exact RMF,
regularity, self-intersections, caps/walls/solid topology не доказаны.

Два новых native regression tests: независимый cone radius/height между
stations, curved-section radii, coarse budget refusal/fine acceptance,
closed radius mismatch и nonpositive law refusal. Ошибка первого теста
с неверно предположенным U-domain исправлена на действительный knot domain.
Полный native прогон362tests и2doctests прошёл. Изменённый JSON dispatch
дополнительно скомпилирован:6focused pipe tests прошли.
TypeScript wrapper, строгий graph schema и Rush modifier variable_pipe_surface
подключены. radius_law — ссылка на rational3D curve; normal dimensionless,
sections dimensionless integer, radius control coordinates и budget lengths.
Новый native language fixture test и TypeScript typecheck прошли.
Geometry WASM packaging завершён:9884118bytes, SHA256
1a04bc61b129e6c43b03030ab24305d1e9367dddf37965f6c9a0493eaf6cf5a8.
Полный packaged прогон419tests в24files прошёл, включая82Rush fixtures.
JSON сохраняет rational definitions, OBJ/PLY mesh coordinates tolerance1e-5mm,
indices точные. Программный editor entrypoint обоих примеров прошёл.
Независимая weighted radius equation проверена на authored section stations.
Первый packaged прогон обнаружил пропущенную radius_law dependency при
обходе графа; traversal/conditional selection исправлены, language WASM
пересобран, полный повторный прогон прошёл. Continuous variable-radius/RMF,
rounding-inclusive, actual visual editor review и STEP остаются открытыми.
