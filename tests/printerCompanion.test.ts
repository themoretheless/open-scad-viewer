import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  companionDiscover,
  companionHealth,
  companionSend,
  utf8ToBase64,
} from '../src/services/printerCompanion'

afterEach(() => { vi.unstubAllGlobals(); vi.restoreAllMocks() })

describe('printerCompanion', () => {
  it('reports health false when fetch fails', async () => {
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new Error('offline')))
    await expect(companionHealth()).resolves.toBe(false)
  })

  it('discovers and sends through the loopback companion', async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce({ ok: true, text: async () => JSON.stringify({ printers: [{ vendor: 'bambu', host: '10.0.0.2', serial: 'S1' }] }) })
      .mockResolvedValueOnce({ ok: true, text: async () => JSON.stringify({ remoteName: 'a.gcode', verified: true }) })
    vi.stubGlobal('fetch', fetchMock)
    await expect(companionDiscover()).resolves.toEqual([{ vendor: 'bambu', host: '10.0.0.2', serial: 'S1' }])
    await expect(companionSend('moonraker', { host: 'http://10.0.0.3:7125' }, 'a.gcode', utf8ToBase64('G28'))).resolves.toEqual({
      remoteName: 'a.gcode',
      verified: true,
      gcodeState: undefined,
    })
    expect(fetchMock.mock.calls[1][0]).toContain('/send')
    expect(JSON.parse(fetchMock.mock.calls[1][1].body)).toMatchObject({
      vendor: 'moonraker',
      fileName: 'a.gcode',
      bytesBase64: utf8ToBase64('G28'),
    })
  })
})
