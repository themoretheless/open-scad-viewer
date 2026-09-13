import {readFileSync, writeFileSync} from 'node:fs'
import {fileURLToPath} from 'node:url'
import {renderPredicateCandidateJson, renderPredicateCandidateRust} from '../tests/support/predicateCandidateCorpus'

const outputs = [
  ['../tests/fixtures/predicate-candidate-corpus-v1.json', renderPredicateCandidateJson()],
  ['../tests/fixtures/predicate-candidate-corpus-v1.rs', renderPredicateCandidateRust()],
] as const
for (const [relative, text] of outputs) {
  const path = fileURLToPath(new URL(relative, import.meta.url))
  if (process.argv.includes('--check')) {
    if (readFileSync(path, 'utf8') !== text) throw new Error(`Candidate predicate fixture is stale: ${path}`)
  } else writeFileSync(path, text)
}
