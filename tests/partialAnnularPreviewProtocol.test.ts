import { expect, it } from 'vitest'
import { mainSolidExpectation, mainSolidResult, type MainSolidJob } from '../src/kernel/mainSolidProtocol'
import { MainSolidWorkerClient, type MainSolidPort } from '../src/kernel/mainSolidWorkerClient'

type FakePort = MainSolidPort & { request?: Parameters<MainSolidPort['postMessage']>[0]; terminated?: boolean }
/** Fields of a successful worker reply inspected by the handler test. */
interface ReceivedReply {
  ok: boolean
  result: { evidence: unknown; body: { brep: unknown; mesh: { indices: ArrayLike<number> } } }
}

const body = {
  id: 'source',
  brep: { topologyIds: { bodies: ['b:source'] } },
  mesh: { positions: [0, 0, 0, 1, 0, 0, 0, 1, 0], indices: [0, 1, 2] },
}
const job = { kind: 'partialAnnularPreview', body, edge: 2, radius: 1.25 } as unknown as MainSolidJob
const result = {
  body: structuredClone(body),
  evidence: {
    qualification: {
      status: 'preview-only',
      commitAllowed: false,
      boundaryIntersectionProof: 'unqualified',
      transitionContinuityProof: 'unqualified',
    },
  },
}

it('requires preview qualification, source identity and finite indexed display geometry', () => {
  const expectation = mainSolidExpectation(job)
  expect(mainSolidResult(expectation, result)).toBe(true)
  for (const mutate of [
    (r: typeof result) => (r.evidence.qualification.commitAllowed = true),
    (r: typeof result) => (r.evidence.qualification.status = 'complete'),
    (r: typeof result) => (r.evidence.qualification.boundaryIntersectionProof = 'complete'),
    (r: typeof result) => (r.body.id = 'other'),
    (r: typeof result) => (r.body.brep.topologyIds.bodies[0] = 'b:other'),
    (r: typeof result) => (r.body.mesh.positions[0] = Infinity),
    (r: typeof result) => (r.body.mesh.indices[0] = 999),
  ]) {
    const reply = structuredClone(result)
    mutate(reply)
    expect(mainSolidResult(expectation, reply)).toBe(false)
  }
})

it('drops a partial-annular preview delivered after cancellation and replacement', async () => {
  const ports: FakePort[] = []
  const client = new MainSolidWorkerClient(() => {
    const port: FakePort = {
      onmessage: null,
      onerror: null,
      onmessageerror: null,
      postMessage(m) {
        this.request = m
      },
      terminate() {
        this.terminated = true
      },
    }
    ports.push(port)
    return port
  })
  try {
    const first = client.run(job),
      cancelled = expect(first).rejects.toMatchObject({ name: 'AbortError' })
    const old = ports[0].request!,
      late = ports[0].onmessage
    client.cancel()
    await cancelled
    const second = client.run(job)
    let settled = false
    void second.then(() => {
      settled = true
    })
    late!({ data: { version: 1, id: old.id, kind: job.kind, ok: true, result } } as MessageEvent)
    await Promise.resolve()
    expect(settled).toBe(false)
    const port = ports.at(-1)!
    port.onmessage!({ data: { version: 1, id: port.request!.id, kind: job.kind, ok: true, result } } as MessageEvent)
    await expect(second).resolves.toMatchObject({ evidence: { qualification: { commitAllowed: false } } })
  } finally {
    client.dispose()
  }
})

it('accepts and executes the partial-annular request through the complete worker handler', async () => {
  const { createBrepTube, tessellateNurbsBrep, partialAnnularPreview } = await import('../src/kernel/geometry/brep')
  const { createMainSolidWorkerHandler } = await import('../src/kernel/mainSolidWorkerRuntime')
  const brep = createBrepTube(20, 5, 6)
  const source = { id: 'annular', name: 'Annular', brep, mesh: tessellateNurbsBrep(brep, 2) }
  const actualJob = { kind: 'partialAnnularPreview' as const, body: source, edge: 2, radius: 1.25 }
  const messages: ReceivedReply[] = [],
    handle = createMainSolidWorkerHandler(message => messages.push(message as unknown as ReceivedReply))
  await handle({ version: 1, id: 1, job: actualJob })
  expect(messages).toHaveLength(1)
  expect(messages[0].ok).toBe(true)
  expect(mainSolidResult(mainSolidExpectation(actualJob), messages[0].result)).toBe(true)
  // Verify that the worker transports the requested geometry and its display
  // mesh, rather than pinning a triangle count that changes with tessellation.
  const expected = partialAnnularPreview(brep, actualJob.edge, actualJob.radius)
  expect(messages[0].result.evidence).toEqual(expected)
  expect(messages[0].result.body.brep).toEqual(expected.model)
  expect(messages[0].result.body.mesh).toEqual(tessellateNurbsBrep(expected.model, 12))
  expect(messages[0].result.body.mesh.indices.length / 3).toBeGreaterThan(0)
  expect(messages[0].result.body.mesh.indices.length / 3).toBeLessThanOrEqual(20000)
})
