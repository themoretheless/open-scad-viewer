# NURBS-0069: лента вдоль пути

Native ribbon::checked(path,width,normal,sections,budget), JSON surface_ribbon.
Ширина — положительный rational scalar law с controls [width,0,0]. Домены
пути и ширины нормализуются независимо, не по длине пути. normal — initial
width direction, проецируемое перпендикулярно start tangent; nonfinite/zero/
parallel direction отвергается. Полоса U=[0,1], centerline U=0.5.

Unit-width line section переносится shared double-reflection frames и
масштабируется around path centers. Internal checked_scaled_profile выделен
из variable pipe: прежняя pipe geometry/refinement формула сохранена.
Closed paths распределяют frame holonomy и требуют одинаковой endpoint width,
повторяя первое сечение; seam C0. sections2..32 (closed минимум4).
Linear loft сохраняет widths на authored sections, не exact continuous RMF
или width constraint между ними. Report sampled4x refinement, budget positive;
при превышении budget surface:null, tolerance не ослабляется.

Нативные проверки: straight path, varying width independent boundary formula,
оба направления normal и projection tangent component; curved section widths,
coarse refusal/fine acceptance; invalid scale и parallel direction; closed seam
и mismatching width refusal. Первый полный прогон выявил две ошибки тестовых
данных (zero-length line primitive для constant law и неверный parallel vector).
Они исправлены; повторный полный прогон366tests/2doctests прошёл.
После уточнения diagnostic strings shared helper3focused ribbon tests прошли.

TypeScript wrapper checkedRibbonNurbsSurface и строгий graph ribbon_surface
подключены. Rush path.ribbon_surface(width_law:curve,normal:...,sections:...,
max_deviation:...) сохраняет reference dependency; sections scalar integer,
budget length, law control coordinates length. Typecheck, native syntax test,
language WASM и packaged dimension test прошли. Два examples включены в
общий export/editor fixture набор. Geometry packaging завершён:
9890760bytes, SHA25610f04b637dfada86df6a274e5381ab244f98f670da98f8b6918205a7c59dfd15.
Общий packaged прогон437tests/25files прошёл, включая85Rush fixtures.
JSON hash/rational definitions сохранены, OBJ/PLY coordinate tolerance1e-5mm,
triangle indices совпадают. Программный editor entrypoint открытого и
замкнутого примеров прошёл; actual visual review этим не подтверждается. Continuous RMF/width,
rounding-inclusive error, self-intersections/regularity/solid topology, actual
visual editor и STEP не доказаны.
