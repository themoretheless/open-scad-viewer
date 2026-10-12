/** True when `value` has no lone UTF-16 surrogate, so it encodes losslessly as UTF-8. */
export function isWellFormedUnicode(value: string): boolean {
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(++index)
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false
    } else if (unit >= 0xdc00 && unit <= 0xdfff) return false
  }
  return true
}

/** Cuts `value` to at most `maximum` UTF-16 units without splitting a surrogate pair. */
export function truncateWellFormed(value: string, maximum: number): string {
  if (value.length <= maximum) return value
  let end = maximum
  const last = value.charCodeAt(end - 1)
  if (last >= 0xd800 && last <= 0xdbff) end--
  return value.slice(0, end)
}
