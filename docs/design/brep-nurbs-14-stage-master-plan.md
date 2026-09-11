# Master-plan: два постоянных движка и собственное Rust B-rep/NURBS ядро

Статус: synthesis десяти role-separated agent reviews всех 14 этапов,
2026-08-01. Это design evidence, а не формальная qualification или legal
approval.

Этот документ переводит пользовательский список из 14 пунктов в исполнимый
порядок. Он не заменяет подробную спецификацию
`rust-brep-nurbs-kernel.md`, а фиксирует решения, gates, независимые проверки
и stop-loss для реализации.

## Итоговый вердикт

**GO только для G0 Contract Pack. NO-GO для G1+, регистрации исполнимого
B-rep provider и изменения product behavior, пока G0 не закрыт.**

Текущий безопасный статус сохраняется:

- `legacy/current` навсегда маршрутизируется в постоянный Manifold backend;
- `openscad-viewer/brep-1` навсегда маршрутизируется в постоянный Rust B-rep
  backend;
- B-rep runtime сейчас отсутствует и честно публикуется как `not-deployed`;
- ошибка, timeout, cancel или revocation одного backend никогда не запускают
  другой backend;
- Manifold mesh не получает вымышленные B-rep topology identities;
- planned capability не является обещанием реализации или сроком поставки.

## Десять review lenses

Все 14 пунктов были явно оценены с каждой из десяти линз: девятью отдельными
subagent reports и primary synthesis. Это независимые задания/контексты внутри
одной agent-системы, но не организационно независимые внешние approvals:

1. primary synthesis и product/feasibility red-team;
2. system architecture и backend contracts;
3. compiler/language semantics и versioning;
4. NURBS/computational geometry;
5. B-rep topology, snapshots и persistent identity;
6. intersections, trimming, classification и Boolean;
7. numerical robustness, degeneracies, sewing/healing;
8. tessellation, scene ABI, renderer и browser Worker;
9. Rust/WASM runtime, security, licensing и MCP deployment;
10. verification, qualification и program-delivery red-team.

Version-bound запись ролей, scope, verdicts и dispositions хранится в
[`brep-nurbs-14-stage-review-manifest.json`](./brep-nurbs-14-stage-review-manifest.json).
Она доказывает выполненные review passes, но не заменяет будущие независимые
certificate verifiers, legal review или `QualificationPlan`.

По dependency order, отсутствию fallback, необходимости независимых oracle и
запрету параллельного старта G1–G3 разногласий нет. Открыты только решения,
которые должны стать ADR внутри G0.

## Исправленный порядок

Исходные 14 пунктов не являются линейными фазами. Нормативный порядок:

```text
G0  contracts, IDL, legacy oracle, qualification schema
 ↓
G1  executable SemanticProgram + Manifold parity
 ↓
G2a box: minimum math + topology + certified tessellation
G2b cylinder/cone: seams and apex
G2c sphere/torus: poles and double periodicity
G2d bounded NURBS foundation
 ↓
G3  native/WASM shadow, no user publication
 ↓
G4a protocol v6 + GeometrySceneV2 migration
G4b constructor-certified planar trim
G4c extrusion
G4d revolution
 ↓
G5a CC/CS queries and coverage verifier
G5b general trim arrangements and classification
G5c finite analytic SS matrix
G5d exact/certified sewing
G5e analytic Boolean with aggregate certificate
 ↓
N1  persistent naming qualification
R1  research-only NURBS SS go/narrow/stop
G6  only the finite NURBS SS matrix approved by R1
 ↓
G7  explicit brep-1 product opt-in and MCP provider
G8  staged rollout
 ↓
post-G8: healing, STEP, fillet, shelling, loft, sweep, offsets
```

Qualification is not the last box. An immutable `QualificationPlan` is an
entry condition for every G1+ gate.

## Consensus по 14 пунктам

| № | Пункт | Решение 10/10 | Фактический gate |
| --- | --- | --- | --- |
| 1 | Контракт двух движков | Закрепить сейчас, не ослаблять | G0, затем continuous |
| 2 | `SemanticProgram` | Spec сейчас; код только после G0 | G0 → G1 |
| 3 | Manifold через общий seam | Обязательный первый executable backend | G1 |
| 4 | Rust workspace | Не создавать 13 crates заранее | после G1, G2a |
| 5 | Математический фундамент | Contract сейчас; walking implementation | G0 → G2a+ |
| 6 | NURBS | Узкий bounded subset после primitive slices | G2d; R1/G6 later |
| 7 | B-rep topology | Schema сейчас; первый Rust vertical slice | G0 → G2a–c |
| 8 | Intersections/trimming | Разделить на certified gates | G4b, G5a–c, R1/G6 |
| 9 | Primitives/operations | Разделить primitives/features/Boolean | G2, G4, G5e |
| 10 | Healing | Не использовать для успешности Boolean | G5d sewing; healing post-G8 |
| 11 | Tessellation | Перенести к первому topology slice | G2a onward; G4a scene |
| 12 | Rust native/WASM | Native first, затем shadow и scene migration | G3 → G4a |
| 13 | MCP activation | Только qualified supervised provider | G7 → G8 |
| 14 | Qualification | Сквозной gate, не финальный тест | G0 → каждый gate |

## Карточки выполнения

### 1. Контракт двух постоянных движков

**Deliverables:** source-only routing ADR; immutable manifest/archive; exact
execution provenance; capability/refusal taxonomy; same-engine retry and
revocation policy; rollback/LKG states; browser/MCP parity contract.

**Entry:** текущий dual-engine MCP snapshot.
**Exit:** hash-stable routing manifest и frozen negative matrix для directives,
capabilities, provider states, cancel/stale, retry и persisted history.
**Independent oracle:** маленький reference-router без imports из production
router; source/write/read attestation tests.
**Kill:** caller-selected engine, cross-engine retry/fallback, B-rep identity на
Manifold mesh или смена `legacy/current` semantics.

### 2. Исполнимый `SemanticProgram`

**Deliverables:** versioned discriminated IDL; typed DAG; units, transforms,
coercions, empty/multi-result semantics; spans and occurrences; deterministic
diagnostics; capability closure; canonical binary encoding.

Нормативно разделяются:

- `sourceHash` — точные source bytes;
- `programHash` — canonical semantic program;
- `TopologySnapshotId` — validated B-rep value identity;
- `tessellationPolicyHash` — quality/export policy;
- `MeshAssetId` — derived mesh packet identity.

Нормативные domain-separated preimages:

```text
TopologySnapshotId = H(
  "topology-snapshot-v1",
  programHash,
  languageContract,
  kernelFingerprint,
  capabilityManifestVersion,
  topologySchemaVersion,
  toleranceEvidencePolicyHash,
  canonicalTopologyBytes
)

MeshAssetId = H(
  "mesh-asset-v1",
  TopologySnapshotId,
  tessellationPolicyHash,
  meshPacketSchemaVersion,
  canonicalMeshPacketBytes
)
```

Execution limits, deadlines and cache state не входят в эти value identities;
они аттестуются отдельно, а reuse разрешён только если сохранённые work/evidence
bounds удовлетворяют новым effective limits.

Deadline, budgets, queue state and publication policy не входят в
`programHash`. Коммутативные operands нельзя самовольно сортировать, если это
меняет diagnostics, provenance или identity.

**Entry:** approved G0 IDL/comparator.
**Exit:** encode/decode and migration fixtures, independent schema validator,
bounded parser/lowering fuzz, deterministic hashes native/TS.
**Kill:** две неудачные фундаментальные schema iterations, cache alias или две
разные semantics в browser и MCP.

### 3. Manifold adapter

**Повторный audit verdict текущего tree:**

- `PASS` для core-only producer discriminant на trusted executor path:
  executor выводит canonical producer occurrence/operation из validated core,
  и backend использует только `operationName=mirror`, а не source/span, чтобы
  отличить legacy zero-normal `mirror()` от сингулярного `multmatrix()` с той
  же матрицей; G1 ещё требует, чтобы validator/executor не вызывал backend при
  ambiguous/mismatched projection, а backend локально отказал null или
  non-transform name до allocation;
- `PASS` как локальный ABI/parity subgate для explicit runtime-only
  `materialized-empty`: legacy/current сохраняет фактически созданный empty
  Manifold handle, payload и уникальный lease через downstream operations, а
  assembler исключает его лишь на root publication boundary; targeted matrix
  сейчас green 53/53;
- `PASS` только как local behavioral subgate для discarded execution: lowerer
  теперь удерживает cutters после статически empty base, строит exact implicit
  union и backend действительно исполняет этот непубликуемый graph. Frozen
  non-manifold-cutter и original-ID fixtures green;
- `PASS` только как local behavioral subgate для двухфазного precedence:
  captured `assert(false)` откладывается до завершения kernel prefix, поэтому
  frozen open-polyhedron repro публикует раннюю kernel error; после зелёного
  prefix leases освобождаются до language terminal;
- `PASS` как qualification-local execution/effect subgate для exact object,
  созданного trusted lowerer: SPC/SPE 1.2 фиксирует `semantic-execution-v2`,
  authored kernel order, eager difference cutters и exact n-ary union.
  Production и independently implemented reference validator отвергают
  sibling-order mutation, `ownerOccurrence: primitive -> null` и forged
  terminal effect;
- `PASS` как qualification-local deferred-terminal subgate: terminal содержит
  canonical interrupted `occurrence`, lowerer регистрирует `$assign`, error
  template связывает operation, `errorName=OpenSCADParseError|TypeError` и
  `detailSha256`, а оба
  validators проверяют active subtree, maximal prefix и exact root owners.
  Post-child и partial-map terminal допустимы; завершённые outputs входят только
  в непубликуемый prefix;
- `LIMIT`: structural SPC/SPE доказывает self-consistency occurrence-production
  schedule/active subtree, но не связь этой структуры с source и не exact
  deepest/first source error. Эти свойства принадлежат trusted lowerer, а не
  переносимому validator. Normal executor принимает только exact
  process-local `WeakSet`-branded lowering object; decoded, migrated или copied
  SPC/SPE не исполняются;
- `FAIL` для boundary qualification: valid direct result уже может иметь
  257-character legacy entity ID при v5 limit 256, а отдельного worker terminal
  contract для этого случая нет; Node/MCP всё ещё вызывает синхронный Manifold
  in-process и не имеет hard-kill/join boundary.

**SHADOW ADAPTER: `FAIL`.** Существующий qualification-only adapter разрешено
сохранять и использовать как локальный differential discovery tool, но нельзя
называть qualified shadow и нельзя включать в runtime browser/MCP shadow.
Targeted `semanticProgram`/reference-validator/Manifold-plan suite — `250/250`
green; это
подтверждает implemented local behavior и
перечисленные
one-invariant negative vectors, но не превращает structural SPC/SPE в proof
exact source error. Manifold parity corpus всё ещё
вызывает historical evaluator в `beforeAll`, поэтому считается discovery, а не
независимым qualification oracle.

**PRODUCTION CUTOVER: `FAIL`.** Production v5 остаётся на pinned direct
evaluator. Помимо перечисленных blockers, новый backend нельзя аттестовать как
`legacy-direct-evaluator-v1`; versioned activation остаётся v6-only.

**Deliverables:** один `SemanticProgram -> ManifoldPlanBackend` evaluator для
browser и Node/MCP; отдельный v5 compatibility assembler; test-only fake
backend; differential runner; import-graph и lifecycle tests. В предлагаемом
разрезе зависимости направлены только так:

```text
source -> qualified lowerer -> trusted SemanticProgram
                                  |
                                  v
                    shared SemanticProgramExecutor
                                  |
                                  v
                       ManifoldPlanBackend
                                  |
                                  v
                 narrow CadKernelOps adapter

owning execution result + SPE1 provenance
                                  |
                                  v
             LegacyV5Assembler -> neutral legacy result -> existing v5 facade
```

`ManifoldPlanBackend` реализует session/lease ABI пункта 2 и принимает только
materialized node values, borrowed inputs, exact carrier key, `programHash`,
language contract, limits и signal. В его API, runtime payload и dependency
closure отсутствуют source text, compiler AST, parser result и protocol-v5
packet. Допустимое payload family закрыто двумя ключами:

```text
Region/d2/Mesh/RepresentationPreserving   -> opaque session region handle
SolidSet/d3/Mesh/RepresentationPreserving -> opaque session solid handle
```

Аналитические, certified и B-rep carriers возвращают typed refusal. Shared
mathematical `empty` создаёт общий executor; он payload-free, lease-free и не
подменяется opaque kernel object. Только для `legacy/current` executor допускает
отдельный exact-key runtime tag `materialized-empty`: backend обязан получить
его из реального kernel handle после точной `isEmpty` проверки, вернуть тот же
checked payload и уникальный lease, а executor передаёт его во все последующие
kernel operations как присутствующий operand. Это не opaque empty, потому что
состояние не спрятано в payload/truthiness, и не обычный `value`, потому что
такой root не материализуется в scene/mesh. Tag не сериализуется в SPC1/SPE1,
TSP1, protocol v5 или public v6 scene payload и запрещён для `brep-1` и
certified evidence. Exact `polyhedron(vertices=[], triangles=[])` допускается
только legacy/current; B-rep contract остаётся строгим.

На Manifold path классификация handle полна и двунаправленна: exact
`isEmpty=true` даёт `materialized-empty`, `false` даёт `value`, а backend не
возвращает shared `empty` вместо уже вызванной kernel operation. Shared `empty`
на этом path возникает только из доказанной executor reduction.

Backend имеет closed exhaustive mapping только для legacy polygonal
constructors, transforms, Boolean/hull, extrusion/revolve, projection и
offset, уже материализованных lowerer. Tessellation parameters и все legacy
quirks (winding, fill rule, cone/mirror, center, operation order and original
IDs) входят в frozen compatibility table, а не вычисляются повторным разбором
source.

`SemanticBackendContext.producer` является частью semantic input backend, хотя
не является полем node. Он вычисляется только из authenticated core как
единственный materializer occurrence и его static operation; operation,
occurrence и поэтому различие `mirror`/`multmatrix` уже входят в `programHash`.
Backend cache нельзя ключевать одним node encoding. Отсутствующий, неоднозначный
или недопустимый producer для transform — contract failure: validator/executor
отсекает ambiguous/mismatched projection до `evaluate`, а backend защитно
проверяет non-null и допустимое transform name до allocation. Producer задаёт
authored dispatch semantics, но не является доказательством non-emptiness,
runtime tag или public provenance: `materialized-empty` сохраняет producer
своего materializer, а downstream node получает собственного producer. Golden
pair обязан показывать: одна и та же singular matrix от zero-normal `mirror`
создаёт kernel empty handle и возвращает `materialized-empty`, а от
`multmatrix` проходит в kernel transform и в frozen fixture возвращает обычный
`value`.

Обычное pruning допустимо только для ветви, которую frozen semantics вообще не
вычисляет. Eager, но discarded geometry (сейчас прежде всего cutters после
empty base `difference`) observable: constructor/union может упасть, allocation
меняет original-ID partition, а работа влияет на budgets/cancel. Минимальная
фактически реализованная форма — не streaming и не второй evaluator, а
schema-minor SPC/SPE `1.2` с обязательным feature
`semantic.execution-v2` и exact hash-covered полем:

```ts
execution: {
  version: "semantic-execution-v2",
  evaluationOrder: readonly NodeId[],
  discardedEffects: readonly {
    tag: "legacy-difference-cutters",
    ownerOccurrence: OccurrenceIndex,
    root: NodeId,
  }[],
  terminal: null | {
    tag: "legacy-language-error",
    occurrence: OccurrenceIndex,
    diagnosticTemplate: DiagnosticTemplateIndex,
    prefixFrontier: readonly {
      ownerOccurrence: OccurrenceIndex | null,
      root: NodeId,
    }[],
  },
}
```

В фактическом qualification-only producer node storage канонизирован в точный
authored kernel order, а `evaluationOrder` обязан равняться
`[0, ..., nodes.length - 1]`. Этот порядок дополнительно воспроизводится из
occurrence production rules, а не принимается как произвольная topological
перестановка. Для `difference` lowerer сначала
заканчивает base bucket и его exact implicit union, затем вычисляет все cutter
statements и их eager exact n-ary implicit union даже при уже пустом base.
Несколько runtime loop activations одного authored statement агрегируются в
тот же base/cutter bucket до schedule derivation.
Каждый непубликуемый cutter root имеет единственную recomputable effect row.
Kernel-empty operands внутри effect graph остаются `materialized-empty` handles
до окончания union; только effect root затем освобождается.

Фактический `terminal` фиксирует canonical row прерванной dynamic activation,
error template и maximal kernel frontier. `$assign` имеет compiler-owned
node-null occurrence. Template в core связывает operation, `errorName` и
`detailSha256`; rendered message/span остаются source-bound в SPE1, а exact
error object для local facade — в несерилизуемом
`SemanticLoweringSuccess.terminalError`. Lowerer останавливается на первой
пойманной deterministic runtime language error. Structural validator проверяет,
что все более поздние rows принадлежат той же logical activation или её dynamic
subtree; он намеренно не может доказать, что claimed activation является exact
deepest/first source error. Это гарантирует trusted lowerer, а executor принимает
только exact object с process-local `WeakSet` brand.

`prefixFrontier` удерживает предшествующую и частично материализованную
геометрию от canonical pruning; при terminal `core.result` является exact
empty/bottom и assembler недостижим. Root, материализованный occurrence, обязан
иметь exact owner; compiler-owned internal union имеет `ownerOccurrence: null`.
Terminal `discardedEffects` всегда пуст: ранее выполненная discarded work уже
включена в prefix и отдельной второй effect row не дублируется. Production и
reference validators проверяют этот self-consistent structural scope, включая
authored schedule replay, active subtree и partial outputs. Но structural
validation не заменяет trusted source-to-error provenance. Поэтому normal
execution остаётся только in-process trusted-lowerer-only, а не portable SPC/SPE
execution.

Сам подход к статическому trace sound только для закрытого geometry-blind legacy subset:
планирование не вызывает kernel, не читает его результаты и не публикует
warnings/callbacks/history. Ранний kernel failure из schedule подавляет более
поздний planned terminal; при зелёном prefix наружу выходит именно terminal
diagnostic. Если admitted language когда-либо сможет ветвить или вычислять
значения по geometry/kernel result, G1 останавливается: нужен отдельно
версионированный streaming/chunk-attestation design, а не расширение этого
контракта.

Внутри plan dependency closure compiler/source разрешены только qualified
lowerer. Source-facing orchestrator может транспортировать request и вызвать
lowerer, но не передаёт source через plan seam и не консультируется с другим
evaluator. Semantic core/executor не импортируют types geometry-module. Только узкий kernel
adapter владеет workspace CAD session и не экспортирует эти types. Compatibility assembler может читать
trusted core, SPE1 provenance и owning execution result, но не source/compiler/
parser. Он, пока result lease жив, извлекает mesh/normals/metrics/BVH/semantic
edges и воспроизводит старые colors, ordering, metrics, exports и scene packet.
Assembler возвращает нейтральную старую result shape, не импортируя Worker/MCP
protocol; существующий source-facing facade добавляет неизменный v5 envelope.
Syntax/decode/trust и непланируемые lowering failures вообще не открывают
backend session и преобразуются facade из structured diagnostics по frozen v5
error map. Напротив, deterministic runtime language terminal публикуется только
после успешного выполнения всего authenticated kernel prefix и quiescent
failure-close. Lowerer warnings присоединяются там же в прежнем порядке.

Новые `opv1:`, `occv1:` и `entity:v2:` не попадают в v5. Версионированная
`LegacyV5CompatibilityMap` обязана доказуемо восстановить прежние `op:...`,
`entity:root>...`, loop value/duplicate ordinals, module/`children()` instance
paths, source spans/labels и mapping Manifold original ID -> producer. Если
SPE1 не содержит достаточно информации, исправляется ещё не замороженный SPE1
или пункт останавливается; assembler и backend не имеют права читать source.

`begin()` синхронно и атомарно публикует session над уже загруженным raw module.
Default qualification facade передаёт lazy `defaultBackend` в
`evaluateInternal`: source-length check и полное lowering завершаются до await
module readiness. Поэтому source/syntax/lowering failure не открывает и не
загружает backend; regression vectors фиксируют этот phase order. Каждая kernel
allocation регистрируется в session до yield/throw. Inputs borrowed только до
завершения `evaluate`; каждый payload-bearing output (`value` или
`materialized-empty`) получает уникальный session-local lease даже при
внутреннем sharing. `close(commit)` удаляет non-roots и атомарно
передаёт roots result lease; v5 assembler работает до
`await result.dispose()` в `finally`. Abort/failure close quiescently ждёт late
operations и делает ровно одну попытку удалить каждый owned handle. Если
primary kernel failure уже отображён в legacy error, а session cleanup также
падает, facade возвращает `AggregateError([mappedPrimary, cleanupError])`, не
теряя ни одну причину. Любой indeterminate Manifold `delete()` навсегда
quarantine-ит весь backend/module lane; все будущие `begin()` этого instance
отказывают `E_MANIFOLD_PLAN_LIFECYCLE`, и повторное использование module
запрещено. Process-global Manifold `cleanup()`
для этого backend запрещён: baseline — explicit per-session handle registry с
idempotent `.delete()`, либо отдельный disposable Worker/module. Пока Manifold
runtime глобален, shared runtime lane удерживается от первой kernel operation
до disposal результата. Qualification executor всегда вызывает `begin()` и
для terminal с пустым kernel prefix: historical source path открывает backend
до language evaluation, поэтому `begin()` failure обязана выиграть у
немедленной assertion. После успешного begin executor выполняет exact
`evaluationOrder` (включая пустой); ранняя kernel ошибка делает
`close(failure)` и выигрывает. После зелёного prefix executor
делает `close(failure, E_SEMANTIC_LANGUAGE_TERMINAL)`, ждёт освобождения всех
`value`/`materialized-empty` leases и лишь затем выбрасывает связанный
diagnostic. `close(commit)`, result lease и assembler в этой ветви запрещены.
Исторический и plan runs, когда они используются только для discovery,
выполняются строго последовательно либо в изолированных runtimes.

**Оставшаяся patch boundary (без runtime activation):**

1. Freeze schema-minor SPC/SPE 1.2, required feature
   `semantic.execution-v2`, execution version `semantic-execution-v2` и
   неизменный TSP 1.0 только после deliberate codec/hash/reference repin.
2. Сохранить authored-schedule replay, active-subtree, partial-map/post-child,
   sibling-order, exact-owner и forged-effect vectors в production и
   independently implemented reference validator. Это structural proof
   self-consistent schedule, не proof source-to-schedule equivalence или exact
   deepest/first source error.
3. Schema 1.0/1.1 objects and SPE/SPC frames, а также flat-v0, обязаны давать
   `E_SEMANTIC_RELOWER_REQUIRED`; upgrade возможен только fresh exact-source
   lowering с новым hash/attestation. Synthesized execution defaults запрещены.
4. Qualification evidence: frozen source-to-event fixtures и
   recording/fault-injected fake kernel проверяют order, begin precedence,
   terminal cleanup и mapping без runtime direct evaluator. Lazy provider-load
   regression сохраняет source-length/syntax/lowering-before-readiness order.
   Historical differential runner остаётся только discovery tool, не
   fallback/oracle и не acceptance authority.

**Safe integration:**

1. Current historical differential runner остаётся discovery-only: он один раз
   lower-ит source, затем последовательно выполняет historical path и
   plan+assembler; сравнение никогда не публикуется как v5 result и green
   corpus не означает PASS gate.
2. Runtime dev/CI shadow заблокирован, пока independent fixtures не доказывают
   source-to-event order/effect ownership и planned-terminal behavior,
   worker harness не фиксирует oversized v5 identity terminal, а MCP provider
   не вынесен в killable child/Worker.
3. После снятия blockers shadow запускается только после фиксации primary
   direct result, в отдельном bounded low-priority runtime. Timeout, hard kill,
   panic, leak quarantine или diff не меняет primary success/error, timings,
   queue, history, export или MCP response; наружу выходит только out-of-band
   comparison artifact. Rollback — выключение shadow flag.
4. Qualification shadow использует заранее замороженные corpus и comparator в
   browser и MCP. Browser shadow не делит Worker с primary; MCP parent владеет
   stdio/admission, child — kernel, deadline завершает и join-ит child.
5. При неизменном protocol v5 production activation запрещена. Старый v5
   execution descriptor аттестует `legacy-direct-evaluator-v1`; выдавать его
   для нового input contract было бы ложной аттестацией, а новый manifest
   изменил бы frozen observable. Новый immutable provider manifest допускается
   только вместе с protocol v6 (либо с отдельным явно одобренным versioned v5
   migration ADR, который не входит в этот пункт).

**Entry:** executable stage 2, accepted session/lease ABI and frozen G0 oracle.

**Acceptance/exit for shadow qualification:**

- import-graph tests запрещают source/compiler/parser/protocol imports в
  backend и Manifold types вне narrow adapter; fake backend доказывает, что
  executor и assembler source не получают;
- exhaustive operation/carrier/empty/error table и golden legacy identity/
  provenance mapping проходят без fallback;
- core-producer vectors различают equal-matrix zero `mirror` и singular
  `multmatrix`, доказывают hash inclusion и fail-closed missing/ambiguous/wrong
  producer: ambiguity/mismatch не достигает backend, а null/non-transform name
  локально отказывается до allocation;
- `materialized-empty` vectors доказывают exact tag/payload/unique-lease shape,
  legacy-only admission, отсутствие shared-empty reductions, сохранение
  authored operand order и exact Manifold mesh bytes downstream, omission
  assembler-ом лишь на root publication и exact-once disposal; hostile
  opaque-empty, value-masquerade, certified/B-rep и serialization cases
  получают refusal;
- exact empty-polyhedron fixture проходит только под `legacy/current`, а тот же
  SPC node/intent отвергается `brep-1`; targeted 53/53 parity является
  необходимым subgate, но сама по себе не квалифицирует shadow;
- current schema 1.2 / execution-v2 vectors доказывают feature/version/hash
  inclusion, bounded wire shape, authored-schedule replay и rejection sibling
  reorder, forged terminal effect и missing/wrong terminal root owner;
- migration vectors требуют `E_SEMANTIC_RELOWER_REQUIRED` для schema 1.0/1.1
  objects and SPE/SPC frames и flat-v0. Fresh exact-source lowering создаёт
  новый 1.2 hash/attestation; structural defaults и portable old-artifact
  execution запрещены;
- independent frozen traces и recording/fault-injected fake kernel покрывают
  empty-base difference с одним/несколькими cutters, base union до cutters,
  несколько runtime loop activations одного authored base/cutter statement,
  mixed dimensions, kernel-invalid constructor/union, warnings,
  palette/reduced state, original-ID equality partition, cancel и следующий
  published sibling; harness не импортирует runtime direct evaluator;
- planned-terminal corpus покрывает empty/nonempty prefix, nested
  transform/Boolean/module/loop frames, assignment failures, post-child parent
  terminals, partial-map outputs, later-sibling forgery, live cutter accumulation
  и frozen open-polyhedron-then-`assert(false)` repro. Ранняя kernel failure
  подавляет assertion; зелёный prefix сначала quiescently освобождает все
  leases, затем публикует assertion; commit, assembler и второй terminal
  недостижимы. Empty prefix всё равно проходит begin + failure-close, и
  begin failure выигрывает у planned language error;
- counted registry доказывает zero live handles после всех determinate success,
  failure, abort, late resolve, invalid/foreign lease и repeated/concurrent
  dispose paths. Injected indeterminate `delete()` сохраняет unresolved handle
  только внутри навсегда quarantined module lane; повторный `begin()` получает
  `E_MANIFOLD_PLAN_LIFECYCLE`. Mapped kernel primary + cleanup failure выходит
  ordered `AggregateError`; parallel-session test исключает cross-session cleanup;
- 100% заранее определённой parity по success, diagnostics/spans, scene/mesh
  bytes, normalized numbers, metrics, preview/full, provenance, export, cancel,
  queue и negative cases в browser/MCP;
- pinned v5 packet fixtures primary path остаются byte-for-byte неизменными, а
  shadow fault-injection не меняет ни одного primary observable;
- worker-level fixture для 256/257-character operation/entity/instance IDs
  доказывает заранее утверждённый bounded failed terminal до success
  publication, отсутствие truncation/hash alias и работоспособность следующего
  job без изменения v5 exact-key shape;
- MCP non-cooperative kernel fixture доказывает bounded cancel/deadline hard
  kill + join, responsiveness stdio parent, ровно один terminal, отсутствие
  late persistence/publication и успешный следующий request без engine fallback;
- memory slope, serialization/isolation и comparator mutation tests green; сам
  comparator обязан замечать каждый заявленный класс расхождения.

**Independent qualification evidence:** frozen source-to-event/result fixtures,
independent schema/trace validator и recording/fault-injected fake kernel.
Historical implementation остаётся отдельным production-v5 publisher до
versioned activation, но новый executor никогда не использует её как fallback,
oracle или источник schedule/terminal решения.

**Kill:** превращение `materialized-empty` в shared `empty`/обычный `value`,
его допуск вне `legacy/current`, вывод в wire/public scene или omission до
завершения downstream/effect operations; pruning kernel-evaluated discarded
subtree без аттестованного effect frontier; direct error против plan success;
публикация planned assertion раньше ранней kernel failure; commit/assembler до
terminal cleanup; неполный/невалидный prefix frontier, публикация partial
terminal outputs или расхождение occurrence-production schedule с node order;
разделение runtime loop activations одного authored `difference` statement на
несколько base/cutter buckets;
observable speculative planning или geometry-dependent control под static
trace; отсутствие `E_SEMANTIC_RELOWER_REQUIRED` для SPC/SPE 1.0/1.1 или flat-v0,
structural defaulting либо исполнение decoded/copied artifact в обход exact
process-local trusted-lowering object; утверждение, что structural replay сам
доказывает source-authored schedule или deepest/first source error; консультация
runtime direct evaluator как
fallback/oracle; source/compiler import или source parameter в
backend/assembler;
global cleanup, способный затронуть другой session/result; потеря primary или
cleanup component вместо ordered `AggregateError`, повторный delete либо
reopen/reuse Manifold backend/module после indeterminate cleanup вместо
permanent quarantine и `E_MANIFOLD_PLAN_LIFECYCLE`; false reuse старого
manifest; runtime shadow до worker identity и MCP hard-kill gates; влияние
shadow на primary; automatic fallback/engine retry; legacy ID, синтезированный
из нового ID без exact proof; leak/double-free/use-after-dispose/late-allocation;
unexplained diff; или ослабление comparator после просмотра результатов.

### 4. Минимальный Rust workspace

**Deliverables:** сначала 3–4 private crates/modules с машинно проверяемым
направлением зависимостей; reproducible native CLI; pinned toolchain/lock;
budget-aware allocations; `unsafe` только в отдельно audited ABI island.

Начальный разрез, а не обещание окончательной структуры:

```text
cad-core       numeric/evidence/geometry carriers
cad-topology   snapshots, transactions, validation
cad-kernel     operations facade and native CLI
cad-wasm       later, only ABI/packaging
```

Новые crates выделяются только после доказанного dependency boundary.

**Entry:** green G1 and approved dependency/licensing ADR.
**Exit:** native box CLI, architecture/compile-fail tests, SBOM/provenance and
offline locked build.
**Kill:** dependency cycles, публичный crate explosion, unresolved rights,
unbounded allocation or core dependency on JS/UI.

### 5. Математический фундамент

**Deliverables:** finite-input policy; units; immutable `ToleranceContext`;
filtered → outward interval → exact-sign predicates; certified constructions;
typed `Indeterminate/PrecisionExhausted/ResourceLimit`; evidence ledger;
portable native/WASM arithmetic policy.

**Entry:** numeric/evidence and determinism ADR.
**Exit:** finite predicate/construction matrix sufficient for each walking
slice, with rational/high-precision oracle and fixed work budgets.
**Independent oracle:** separate BigRational/interval implementation that does
not import production predicates.
**Kill:** global epsilon, snapping, hidden tolerance growth, rounded
construction treated as exact, NaN publication or `Indeterminate → false`.

### 6. NURBS foundation

**Deliverables:** finite positive-weight clamped matrix; validation of degrees,
knots, multiplicities, domains and weight conditioning; stable evaluation and
derivatives; insertion, Bézier decomposition, split/reverse, iso-curves and
conservative bounds. Projection/closest-point is a later certified query, not
an unchecked Newton helper.

**Entry:** G2a–c primitives/topology and sufficient math certificates.
**Exit:** all inputs in frozen degree/weight/domain matrix return certified
success; everything else returns typed refusal.
**Independent oracle:** arbitrary-precision rational/Bernstein evaluator plus
partition-of-unity, affine, split/reverse and refinement metamorphic tests.
**Kill:** success outside matrix, denominator without lower bound, general
surface/surface claim or topology decisions from an uncertified approximation.

### 7. B-rep topology

**Deliverables:** authoritative ownership, oriented coedges, loops, faces,
shells and solids; generational handles; immutable snapshots; COW transaction;
local validator; later global solid audit; deterministic `TopoId` lineage.

Dual authority is forbidden. Canonical model:

```text
Solid owns ordered SolidShellUse { shell, role }
Shell owns face uses and has no independent canonical role
Face owns loops; Loop owns coedges
reverse incidence is derived, never a second source of truth
```

One `Solid` is one connected material component; multi-lump operations return
`SolidSet`. Local validation and global manifold/material audit are distinct
typestates.

**Entry:** topology/ID/canonical-encoding ADR and stage 5 subset.
**Exit:** box→cone→sphere→torus invariant corpus, one-invariant mutations,
stale/sibling/ABA/COW/rollback tests and independent graph validator.
**Kill:** partial commit, invalid success, serialized runtime handle, centroid
naming or contradictory shell authority.

### 8. Intersections and trimming

**Deliverables:** immutable geometry-only reports; complete-domain coverage
certificates; lifted parameter traces; constructor trim first; then certified
arrangement/DCEL and classification; finite analytic SS matrix; research-only
general NURBS SS.

**Entry:** qualified NURBS/topology slices and constructor trim.
**Exit:** every case in a frozen pair/contact matrix is `Complete` with an
independently verified certificate; out-of-matrix and unresolved cases are
typed refusals and cannot mutate topology.
**Independent oracle:** verifier that imports neither solver predicates nor
producer validator, plus analytic/resultant and metamorphic fixtures.
**Kill:** false `Complete`, seed/marching as completeness proof, rounded
endpoint reconciliation, missed tangent/coincident/closed-loop branch or
unbounded certificate.

### 9. Primitives, features and Boolean

**Deliverables:** separate walking slices, never one feature bundle:

1. box;
2. cylinder/cone;
3. sphere/torus;
4. planar trim;
5. extrusion;
6. revolution;
7. exact/certified sewing;
8. analytic Boolean.

Booleans are regularized and return `SolidSet`; lower-dimensional contacts are
reported as evidence/diagnostics, not silently promoted to solids. Every
Boolean success has an aggregate certificate from intersections through global
audit.

**Entry/exit:** separate G2/G4/G5 gates and corpora per slice.
**Independent oracle:** analytic metrics/topology for primitives, exact 2D
profile oracle, and independent cell-selection truth table for Boolean.
**Kill:** Boolean before G5a–d, invalid atomic rollback, hidden simplify/heal or
scope expansion after a failed slice.

### 10. Sewing and healing

**Deliverables:** three different contracts: `canonicalize`, exact/certified
`sew`, and explicitly authorized `heal`. Sewing is G5d; general healing is a
post-G8 capability with displacement/refit/split ledger and full recertification.

**Entry:** complete boundary correspondence reports.
**Exit:** gap/duplicate/orientation corpus, idempotence/no-op, cumulative error
budget, lineage and fault/cancel rollback.
**Independent oracle:** topology diff plus interval displacement/containment
bounds.
**Kill:** automatic healing, tolerance growth, chain merge without global
bound, import/export side-effect or keeping an old ID after ambiguous merge.

### 11. Certified tessellation

**Deliverables:** shared `EdgeSamplingRegistry`; face-local lifted UV
constraints; deterministic refinement; separate geometric positions/render
corners; topology/provenance token tables; coverage/incidence/deviation
certificate; bounded triangle and peak-memory budgets.

**Entry:** first validated box snapshot, not completed healing.
**Exit:** every topology slice produces a watertight certified mesh; scene
publication follows at G4a.
**Independent oracle:** UV coverage/non-overlap, positive area/orientation,
shared-edge byte identity, T-junction and two-sided deviation verifier.
**Kill:** post-hoc weld, mesh used as topology oracle, cracked success,
unbounded refinement or preview changing B-rep topology.

### 12. Native/WASM and scene integration

**Deliverables:** native vertical slice first; then reproducible shadow WASM;
bounded resumable `JobMachine`; checked leased/chunked ABI; worker epochs;
protocol v6 and `GeometrySceneV2`; export lease and restart/rebuild contract.

Current protocol is v5. The only planned incompatible successor in this plan
is v6.

**Entry:** qualified native slices and end-to-end memory budget.
**Exit:** native/WASM canonical topology/status/certificate parity, ABI and
allocation fuzz, trap/OOM/cancel atomicity, leak slope and reproducible
artifact hash; then Manifold+primitive-B-rep v6 migration fixtures.
**Kill:** user-visible G3 output, stale token/epoch, partial publication,
unbounded cancellation, ABI drift or peak memory above admission budget.

### 13. B-rep activation in MCP

**Deliverables:** same evaluator/program/router as browser; supervised provider
with per-engine queue, quotas, watchdog and hard restart; immutable executable
manifest with qualification record; exact provenance through DuckDB and
exports; same-engine-only retry.

`brep-contract-v1` is not mutated into an executable manifest. Activation
publishes a new immutable version and lists only capabilities actually
qualified. STEP remains post-G8 even though early design metadata may mention
it as a long-term plan.

**Entry:** G1 + G3/G4 + the required geometry qualifications.
**Exit:** G7 opt-in with browser/MCP same-program parity, crash/hang/OOM,
revocation, spoof, history and rollback drills.
**Kill:** fake provider, MCP-specific semantics, in-process unkillable B-rep,
cross-engine retry, export from stale/unvalidated snapshot or `available`
without qualification.

### 14. Qualification system

**Deliverables:** immutable `QualificationPlan` schema; frozen positive,
negative and out-of-scope matrices; corpus/toolchain hashes; exact seeds/work
units/clean runs; memory/security/performance budgets; certificate mutation
tests; independent reviewers; SBOM/provenance/legal and rollback evidence.

**Entry:** schema is part of G0; a concrete approved plan precedes every G1+
implementation gate.
**Exit:** zero false-complete, no unresolved P0/P1 in shipped matrix, all
expected refusals, deterministic artifacts, signed release and successful
rollback drill.
**Kill:** post-hoc scope/threshold, oracle sharing the critical producer,
single-maintainer critical module, changed compiler/predicate/serializer without
qualification reset or unverifiable resource cost.

## G0: конкретный ближайший backlog

Никакие пункты ниже не меняют runtime routing или geometry results.

Живой статус pack: [`docs/qualification/g0-contract-pack.md`](../qualification/g0-contract-pack.md)
и [`docs/qualification/g0-contract-pack-status-v1.json`](../qualification/g0-contract-pack-status-v1.json).
G0 **не закрыт**, пока status JSON имеет `"closed": false` и owners остаются
`unassigned`.

| ID | Deliverable | Exit evidence |
| --- | --- | --- |
| G0.1 | Freeze review charter and ADR dependency graph | approved list of blocking ADR and owners |
| G0.2 | Freeze current toolchain/dependency fingerprints | `g0-toolchain-fingerprints-v1.json` (+ later full SBOM) |
| G0.3 | Engine/language/error-precedence ADR | full routing and refusal matrix |
| G0.4 | `SemanticProgram` discriminated IDL | ADR 0005 schema, examples, independent validator |
| G0.5 | Source/program/topology/policy/mesh hash ADR | canonical preimages and collision rules |
| G0.6 | Topology/ownership/typestate/lineage IDL | ADR 0007 + `topology-idl/` fixtures and mutations |
| G0.7 | Tolerance/predicates/constructions/evidence ADR | ADR 0006 + `g2a-predicate-inventory-v1.json` |
| G0.8 | Protocol v6/`GeometrySceneV2` IDL | checked-in wire fixtures and v5 migration matrix |
| G0.9 | Versioned legacy golden corpus manifest | source, output, error, cancel, MCP and provenance artifacts |
| G0.10 | Field-by-field differential comparator | mutation tests proving comparator sensitivity |
| G0.11 | `QualificationPlan` schema | immutable matrix/seeds/budgets/reset policy example |
| G0.12 | Resource/security/supervisor ADR | end-to-end memory, hard-kill and queue policy |
| G0.13 | Independent-reimplementation policy | allowlisted sources, rights inventory, contributor provenance |
| G0.14 | G1 qualification plan | exact pass/fail scope before G1 code starts |

## G0 Definition of Done

G0 закрыт только если одновременно:

- все blocking ADR приняты и не противоречат wire/schema fixtures;
- legacy corpus и comparator versioned, воспроизводимы и ловят intentional
  mutations;
- `SemanticProgram`, topology/evidence and protocol v6 IDL имеют независимые
  validators;
- source/program/topology/tessellation/asset identities не смешаны;
- memory, cancellation, supervisor, package and licensing policies измеримы;
- G1 `QualificationPlan` заморожен до реализации G1;
- нет unresolved P0/P1 в G0 scope.

До этого Rust workspace, B-rep provider и product-visible B-rep UI не создаются.
