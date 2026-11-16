import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  DISPLAY_BUDGET_STORAGE_KEY,
  getDisplayBudget,
  parseDisplayBudget,
  resetDisplayBudgetSetting,
  setDisplayBudget,
  subscribeDisplayBudget,
} from '../src/render/displayBudgetSetting'
import { DEFAULT_DISPLAY_BUDGET, displayBudgetFor } from '../src/render/displayScheduler'

const store = new Map<string, string>()

beforeEach(() => {
  store.clear()
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => store.set(key, value),
    removeItem: (key: string) => store.delete(key),
  })
  resetDisplayBudgetSetting()
})
afterEach(() => vi.unstubAllGlobals())

describe('display budget setting', () => {
  it('falls back to the default for missing or malformed values', () => {
    expect(parseDisplayBudget(null)).toBe(DEFAULT_DISPLAY_BUDGET)
    expect(parseDisplayBudget('{')).toBe(DEFAULT_DISPLAY_BUDGET)
    expect(parseDisplayBudget('"256"')).toBe(DEFAULT_DISPLAY_BUDGET)
    expect(parseDisplayBudget('{"megabytes":"512"}')).toBe(DEFAULT_DISPLAY_BUDGET)
    expect(parseDisplayBudget('{"megabytes":500,"boundingBoxProxies":true}')).toEqual(displayBudgetFor(512, true))
  })

  it('persists a change, notifies subscribers once and survives a reload', () => {
    const seen = vi.fn()
    const unsubscribe = subscribeDisplayBudget(seen)
    expect(getDisplayBudget()).toEqual(DEFAULT_DISPLAY_BUDGET)
    const next = displayBudgetFor(128, true)
    setDisplayBudget(next)
    setDisplayBudget(displayBudgetFor(128, true))
    expect(seen).toHaveBeenCalledTimes(1)
    expect(seen).toHaveBeenCalledWith(next)
    expect(JSON.parse(store.get(DISPLAY_BUDGET_STORAGE_KEY)!)).toEqual({ megabytes: 128, boundingBoxProxies: true })
    unsubscribe()
    setDisplayBudget(displayBudgetFor(64))
    expect(seen).toHaveBeenCalledTimes(1)
    resetDisplayBudgetSetting()
    expect(getDisplayBudget()).toEqual(displayBudgetFor(64))
  })
})
