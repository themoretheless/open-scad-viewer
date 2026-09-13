import {readFileSync, writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {fileURLToPath} from 'node:url'

// Preparation only: no gateway requests or database writes.
const root = new URL('../', import.meta.url)
const preparedAt = new Date().toISOString()
const gatewayObservation = JSON.parse(readFileSync(new URL('docs/architecture/brep-2026-09-12/gateway-observation.json', root), 'utf8'))
const documents = [
  ['README.md', 'open-scad-viewer — текущие возможности'],
  ['crates/brep-core/README.md', 'B-rep — точный исполняемый набор операций и ограничения'],
  ['crates/cad-predicates/README.md', 'B-rep — кандидат точных предикатов, контексты и пределы'],
  ['docs/design/brep-completion-status.md', 'B-rep — аудит реализации и незавершённые требования'],
  ['docs/design/modelgraph-nurbs-own-kernel.md', 'ModelGraph NURBS — синтаксис, операции и допустимые параметры'],
  ['docs/design/brep-semantic-diagnostic.md', 'B-rep — изолированный семантический запуск и ограничения'],
  ['docs/design/nurbs-intersection-queries.md', 'NURBS — численные запросы пересечений и границы свидетельств'],
  ['docs/architecture/brep-2026-09-12/README.md', 'B-rep — актуализация знаний 2026-09-12'],
].map(([path, title]) => {
  const bytes = readFileSync(new URL(path, root))
  const sha256 = createHash('sha256').update(bytes).digest('hex')
  return {path, sha256, bytes: bytes.length, future_tool_arguments: {
    path: fileURLToPath(new URL(path, root)), uri: `project://open-scad-viewer/repository/${path}`,
    title, wing: 'open-scad-viewer', room: 'brep-implementation',
    metadata_json: JSON.stringify({project: 'open-scad-viewer', kind: 'observed-code-checkpoint',
      verified_at: preparedAt.slice(0,10), source_path: path, source_sha256: sha256,
      scope: 'bounded B-rep implementation; see explicit partial/open requirements',
      full_completion_proved: false, production_brep_provider_available: false}),
  }}
})
writeFileSync(new URL('docs/architecture/brep-2026-09-12/ingest-manifest.json', root), JSON.stringify({
  schema_version: 1, prepared_at: preparedAt, status: 'prepared_not_ingested',
  ingestion_performed: false, gateway: 'http://127.0.0.1:7432/mcp',
  ingestion_blocker: 'The responding gateway uses a different database and mock embedding identity; awaiting confirmation of the intended target corpus.',
  gateway_observation: gatewayObservation,
  policy: {order: 'sequential', remove_deleted: false, verify_source_hash_before_write: true,
    verify_returned_document_ids_and_content: true}, documents,
}, null, 2) + '\n')
console.log(`Prepared ${documents.length} B-rep RAG documents; no ingestion performed.`)
