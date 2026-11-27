# Проверки STEP для геометрии NURBS

Статус `integrated` описывает доступность операции в Rust, WASM и Rush. Квалификация STEP относится к получившемуся типу геометрии и сценарию экспорта. Эта матрица фиксирует проверенную область; записи каталога сохраняют собственные статусы экспорта.

| Геометрия на выходе | Текущий контракт | Проверено | Открытая квалификация |
| --- | --- | --- | --- |
| Замкнутые B-rep-тела | Экспорт точного B-rep в STEP AP242 | [51 нативный сценарий](receipt.json), включая рациональные цилиндры, конусы, торы и тела с полостями. [Два экспорта отредактированных тел](browser-receipt.json) из production-редактора после Undo/Redo и перезагрузки | Привязка проверенных сценариев к отдельным операциям каталога |
| Открытая оболочка | STEP AP242 для оболочки, представленной как B-rep | Периодическая грань с отверстием сохранила поверхность, p-кривые и рёбра при внутреннем повторном импорте. [OpenCascade](periodic-sheet-occt.json) подтвердил одну оболочку, одну грань, два контура и площадь 1,25 мм². [Повторный экспорт из Rush после сдвига на 7 мм](periodic-sheet-edited-occt.json) сохранил топологию и площадь. Допуск габаритов 0,0000002 мм учитывает расширение границ импортёром на 0,0000001 мм | Скачивание аналогичного результата из production-интерфейса и независимая проверка других типов оболочек |
| Направляемый loft как открытая B-rep-оболочка | Экспорт STEP AP242 с исходными границами и поверхностью | Нативный повторный импорт сохраняет четыре ребра, точки и производные. [OpenCascade](guided-loft-occt.json) подтвердил одну оболочку, одну грань, один контур, четыре ребра и аналитическую площадь 2,295587149392638 мм² | Экспорт отредактированного loft из production-редактора |
| Отдельная поверхность | Rush требует B-rep. Экспорт поверхности возможен через явную грань с топологией | Проверка отказа для графа с отдельной поверхностью в `rushGraphNurbsMcp.test.ts`. Нативные тесты `guided_loft_step` проверяют рациональные поверхности в B-rep, включая границы и касательные | Независимая проверка отредактированных свободных поверхностей, оформленных в B-rep |
| Отдельная кривая | Rush требует B-rep | Проверка отказа в `rushPeriodicSheetDelivery.test.ts` | Покрыто для точного корня-кривой |
| Сборка | Нативный экспорт STEP AP242 с определениями и вхождениями | В нативном корпусе есть сборки, смешанные единицы и вложенные размещения. [Браузерный экспорт сборки](browser-assembly-receipt.json) проверен в XCAF. Он сохранил две группы, четыре детали, цвета, имена и габариты. [Нативный экспорт общих определений](shared-placement-occt.json) сохранил одно определение и два вхождения с точными матрицами. [Экспорт после изменения размещения в интерфейсе](browser-shared-placement.json) сохранил одно определение, исходное вхождение и связанное вхождение со сдвигом на 7 мм. [Браузерная проверка размещения в группе и цветов](browser-grouped-occurrence-colours.json) подтвердила путь «сборка → группа → экземпляр» в XCAF, сдвиг на 7 мм и привязку красного и синего к разным вхождениям через контексты AP242 | Редактор поддерживает один уровень групп. Цепочки подгрупп и преобразования самих групп остаются за пределами этой проверки |

## Привязка к каталогу

| ID | Подтверждённый сценарий |
| --- | --- |
| NURBS-0054 | [OpenCascade импортирует открытый loft](guided-loft-occt.json). Четыре ребра и аналитическая площадь. |
| NURBS-0560 | [Периодическая обрезанная оболочка](periodic-sheet-occt.json) и [экспорт после сдвига из Rush](periodic-sheet-edited-occt.json). Отверстие и площадь сохраняются. |
| NURBS-0939, NURBS-0940 | [Группа и цвета вхождений](browser-grouped-occurrence-colours.json) после правок в production-редакторе. Два вхождения имеют общее определение и отдельные контексты RGB. |

## Повторить проверку периодической оболочки

1. Запустить `STEP_PERIODIC_SHEET_OUTPUT=<каталог>/periodic-sheet.step cargo test --locked --manifest-path crates/Cargo.toml -p cad-step --test exact_periodic_trim_step_contracts`.
2. Скопировать [манифест](periodic-sheet-manifest.json) в каталог как `manifest.json`. Для свежего экспорта обновить SHA-256 до значения фактически записанного файла, сохранив ожидаемую геометрию.
3. Запустить `python scripts/verify-cad-step-occt.py <каталог> --no-preview`.


## Повторить проверку общего определения сборки

1. Запустить `CAD_STEP_OCCURRENCE_COLOUR_FIXTURE=<путь>.step cargo test --locked --manifest-path crates/Cargo.toml -p cad-step --test edited_gear_step shared_occurrences_keep_separate_surface_colours`.
2. Запустить `python scripts/verify-cad-shared-placement-occt.py <путь>.step`.

## Повторить проверку открытого loft

1. Запустить `LOFT_STEP_OUTPUT=<каталог>/guided-loft.step cargo test --locked --manifest-path crates/Cargo.toml -p cad-step --test guided_loft_step`.
2. Скопировать [манифест](guided-loft-manifest.json) как `manifest.json`. Площадь аналитической поверхности рассчитывается как √2 + asinh(1). Обновить SHA-256 по экспортированному файлу.
3. Запустить `python scripts/verify-cad-step-occt.py <каталог> --no-preview`.

## Повторить браузерную проверку размещения сборки

1. Собрать production-редактор и запустить `node scripts/check-solid-scene-edit-worker-browser.mjs <каталог> --absolute-placement --placement-step`.
2. Запустить `python scripts/verify-cad-shared-placement-occt.py <каталог>/instance-placed.step --offset-x-mm 7 --definition-volume-mm3 8000`.

## Повторить проверку цветов вхождений в группе

1. Собрать production-редактор и запустить `node scripts/check-solid-scene-edit-worker-browser.mjs <каталог> --absolute-placement --placement-step --grouped-colours`.
2. Запустить `python scripts/verify-cad-shared-placement-occt.py <каталог>/instance-grouped.step --offset-x-mm 7 --definition-volume-mm3 8000 --grouped-colours`. XCAF проверяет геометрию и путь в дереве, а STEP-модель OpenCascade проверяет привязку каждого RGB к его контексту вхождения.
