import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { spawnSync } from 'node:child_process'
import { describe, expect, it } from 'vitest'
import { analyzeExchangeCorpusFile, createExchangeCorpusReport } from '../src/exchange/exchangeCorpus'

describe('repeatable exchange corpus report', () => {
  it('separates parser admission from geometric conformance and source availability', () => {
    const bytes = new TextEncoder().encode('#1=A(#2);')
    const file = analyzeExchangeCorpusFile('official/model.step', 'step', bytes, 'digest')
    const report = createExchangeCorpusReport([
      { id: 'cax-if', origin: 'CAX-IF', locator: 'local-cache', availability: 'downloaded', files: [file] },
      { id: 'nist', origin: 'NIST', locator: 'remote', availability: 'not-downloaded', files: [] },
    ])
    expect(file).toMatchObject({ status: 'admitted', entities: 1 })
    expect(report).toMatchObject({ semantics: 'parser-admission-only', totals: { files: 1, downloadedSources: 1, unavailableSources: 1 } })
    expect(report.sources[1]?.files).toEqual([])
  })

  it('produces byte-identical repeat reports and does not relabel self-authored files as official', () => {
    const directory = mkdtempSync(join(tmpdir(), 'exchange-corpus-'))
    const first = join(directory, 'first.json'), second = join(directory, 'second.json')
    const run = (output: string) => spawnSync(process.execPath, ['--import', 'tsx', 'scripts/run-exchange-corpus.mts', '--output', output], { cwd: process.cwd(), encoding: 'utf8' })
    expect(run(first).status).toBe(0)
    expect(run(second).status).toBe(0)
    expect(readFileSync(first, 'utf8')).toBe(readFileSync(second, 'utf8'))
    const report = JSON.parse(readFileSync(first, 'utf8'))
    expect(report.sources.find((source: { id: string }) => source.id === 'cax-if').availability).toBe('not-downloaded')
    expect(report.sources.find((source: { id: string }) => source.id === 'nist').availability).toBe('not-downloaded')
    expect(report.sources.find((source: { id: string }) => source.id === 'self-authored-fixtures').files.length).toBeGreaterThan(0)
  })
})
