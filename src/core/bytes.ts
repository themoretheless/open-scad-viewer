/** Big-endian reads from font and image tables. */

export function u32(bytes: Uint8Array, offset: number): number {
  return ((bytes[offset] * 0x1000000)
    + (bytes[offset + 1] << 16)
    + (bytes[offset + 2] << 8)
    + bytes[offset + 3]) >>> 0
}
