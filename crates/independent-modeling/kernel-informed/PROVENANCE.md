# Опора на Plasticity и независимая реализация

Цель — получить геометрические отличия от чистой базы, без переноса исходников,
псевдокода, ABI, численных констант или проприетарного runtime.

Наблюдавшийся target: Plasticity Beta 26.2.0-beta.39, ARM64 pk.node,
SHA-256 5f00e0c6bcf0497e2e27674629492aeb37228c06dfad1ddda455e221c1ca7f1d.
Результаты реверса лежат вне workspace и не нужны для сборки.

## Наблюдения и собственные решения

| Наблюдение | Независимое решение и граница вывода |
|---|---|
| Extrusion вызывает PK_BODY_extrude; затем встречаются запросы тела/ориентированной поверхности грани, нормали и освобождение tracking results | Новый конструктор сохраняет carrier surfaces, oriented coedges и BoundaryTrack. Это собственная модель; структуры Parasolid не восстановлены и не воспроизведены |
| Loft вызывает PK_BODY_make_lofted_body, работает с entities, regions и tracking | Loft принимает собственные Curve и выдаёт явный Compatibility report; common refinement и rational degree elevation спроектированы самостоятельно. Наблюдения не доказывают, что Parasolid использует этот алгоритм |
| Boolean вызывает PK_BODY_boolean_2 и содержит ветки работы с regions/general topology | Выбран ограниченный, проверяемый геометрический домен — convex planar halfspaces. Алгоритм отсечения не является реконструкцией Parasolid Boolean |
| Тесселяция отделена от исходной геометрии; присутствует PK_TOPOL_facet_2 | У Body отдельно хранятся кривые/поверхности/топология; faceting возвращает производную сетку и карту исходных граней |

Опорные Evidence из предыдущего исследования:

- Extrusion `ev_5baf46930da945e7c0ee0f50a950487338dccd1b36562d211f4ff1d09bf90644`, pk.node 0x13a08f0, PK_BODY_extrude call 0x13a0a44.
- Loft `ev_83eee674e9121b66fb38b3d2e976a02ac68489cb72a57d2e00cb12f1bc3ed42e`, pk.node 0x10a2a5c, PK_BODY_make_lofted_body call 0x10a2d54.
- Boolean assembly: function 0xd95930, PK_BODY_boolean_2 call 0xd964e0.
- Faceting assembly: PK_TOPOL_facet_2 call в сохранённом native callsite inventory.

Геометрические формулы реализованы в Rust независимо. В новый пакет не включены
артефакты приложения, фрагменты декомпиляции, таблицы/enum values SDK или target API.
Единственная Cargo-зависимость — собственный `modeling-clean`; из него используются
общие Curve/Surface evaluators, affine/vector math, Mesh и простой polygon validator.
Новые B-rep, common-refinement/elevation и convex-clipping реализации находятся
здесь и не являются фасадом над его прежними операциями.

Известная граница исходных сведений: наблюдавшиеся wrappers в основном делегируют
геометрию Parasolid. Само ядро Parasolid не восстановлено. Типы Ghidra частичны,
были предупреждения о flow reconstruction. Поэтому соответствие поведения
Plasticity, её точность и её поддерживаемые геометрические случаи не заявляются.
