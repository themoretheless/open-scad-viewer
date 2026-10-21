# Геометрическая проверка G2 при заданном соответствии границ

`nurbs_core::surface_g2::inspect` непрерывно сравнивает позицию, нормаль и пространственный тензор кривизны двух границ. Соответствие задаётся нормализованным аффинным параметром с необязательным reverse. Поперечные скорости, касательный сдвиг поперечного параметра и нелинейность параметризации не требуют равных параметрических jets: сравниваются геометрические тензоры в общей пространственной системе координат.

Есть две ветви ориентации. Same сравнивает n_a−n_b и T_a−T_b; Opposite сравнивает n_a+n_b и T_a+T_b. Выбор знака связан для нормали и тензора, и одна ветвь должна подтвердить весь шов. Независимо выбирать лучший знак для нормали и кривизны или менять знак между ячейками нельзя. Принятая ориентация явно возвращается.

Позиционный допуск имеет размерность длины, normal chord — безразмерный в [0,2], tensor tolerance — Frobenius-норма в обратных единицах длины. Все допуски конечные и неотрицательные. Upper-боксы исходных рациональных геометрических объектов непрерывно покрывают каждую ячейку; нижние границы в свидетельских точках могут доказать исключение ветви. Нельзя принять G2 только потому, что несколько выбранных значений совпали.

WithinTolerance требует позиции в допуске и одной глобально проходящей ветви нормаль+тензор. CorrespondenceExceedsTolerance требует доказанного позиционного разрыва либо исключения обеих ориентаций. Во всех других случаях возвращается Unresolved. Верхние границы нормалей/тензоров отсутствуют при неподтверждённой регулярности или необходимой C2-гладкости покрытых свободных узлов. Отсутствующая граница не заменяется нулём. RegularityNotProven не доказывает сингулярность. Ориентация присутствует только при принятии.

Max_cells=1..100000 считает все обработанные ячейки, включая заменённых родителей. Исчерпание работы/представимой точности сохраняет глобальные границы и Unresolved. Начальное покрытие — полный интервал [0,1]. Некорректные входы и численная непредставимость возвращают ошибку без изменения источников. Точные нулевые допуски могут остаться неподтверждёнными из-за округления. Не выполняются trim-согласование, manifold-проверка, склейка periodic storage seam и автоматический поиск общего соответствия.

Пять независимых интеграционных тестов сравнивают z=x² при параметризации одного патча x=2u+3u²,y=v+u, проверяют разные степени и наборы узлов вдоль шва, противоположную ориентацию, доказанный curvature gap 2 между z=x² и z=2x² при одинаковом G1, отсутствие C2-базиса, сингулярность, малый бюджет и неверные допуски. Все 5 тестов G2 прошли; предыдущий регрессионный запуск также подтвердил 6 G1 и 4 shape_operator.

```sh
cargo test --manifest-path crates/Cargo.toml -p nurbs-core --no-default-features --test surface_g2 --test surface_g1 --test surface_shape_operator --quiet
```

Historical affine-only status: NURBS-0182 remained planned: реализована геометрическая непрерывная проверка при заданном аффинном соответствии, но исходный расширенный контракт включает также поиск и проверку общего неаффинного соответствия швов. Это отдельный недостающий этап, а не новая запись в счётчике. WASM, редактор и STEP для нового API отдельно не квалифицированы.

## Nonaffine correspondence extension (2026-10-15)

`surface_g2::inspect_with_correspondence` accepts 2..1025 strictly ordered
normalized parameter pairs, including (0,0) and (1,1). Each segment is affine;
the complete relation is continuous and bijective. Every map node partitions
the initial verification cover. Interval arithmetic encloses the mapped
parameter range, including rounding, before evaluating the original surfaces.
Position, normal and signed world-curvature tensor bounds cover the whole
relation. The orientation sign remains global. The work budget must cover all
initial segments and counts replaced parent cells as well as children.

`surface_g2::inspect_inferred` proposes an open-seam correspondence using a
bounded numerical nearest-point search (129 grid seeds and 40 local iterations
per interior node). It then calls the independent continuous verifier. Search
samples and extracted geometry are not proof of G2. A nonmonotone candidate
returns `candidate_monotone=false` and no verification report. A rejected
candidate does not disprove the existence of another correspondence.

This extension fixes the endpoint pairs. Closed-seam phase search, global
optimality of candidate search, trim ownership and topology authorship remain
outside this API. NURBS-0182 is native for the declared endpoint-fixed open-seam scope. The added regression compares the same parabolic
surface with linear versus quadratic boundary parameterization: the affine
relation must fail, while the supplied and inferred nonaffine relations must
pass a continuous enclosure. Validation results are recorded after the run.

Validation: all 7 integration tests passed (118.99 seconds). The rational
cylinder required 6000 verification cells; the 4000-cell probe correctly
returned Unresolved at tensor upper 0.15059479 for tolerance 0.15. The final
run retained the same geometric tolerances. See the dated qualification
artifact in `docs/qualification/nurbs-g2-nonaffine-2026-10-15/`.
