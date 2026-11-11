/** Cheap limits shared by the UI and SVG runtime; importing these never loads WASM. */
export const SVG_MAX_BYTES = 4 * 1024 * 1024
export const SVG_MAX_FONT_BYTES = 4 * 1024 * 1024
export const SVG_MAX_TOTAL_FONT_BYTES = 8 * 1024 * 1024
export const SVG_MAX_FONTS = 16
