/**
 * Twenty exact herringbone gears serialize to about 20 MB because helical faces
 * retain cubic loft control points. Browser drafts have a separate smaller quota.
 */
export const MAX_DOCUMENT_CHARACTERS = 64 * 1024 * 1024
