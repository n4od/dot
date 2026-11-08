/**
 * The web demo paints the eyes as holes in a mask, so they reveal the page
 * colour. A transparent desktop window has no page colour — a hole would show
 * the wallpaper and the orbit rings drawn behind the body — so the eyes are
 * filled instead, and that fill has to contrast with the ink.
 *
 * Shared by the pet window and the settings preview: if these two disagreed,
 * the picker would be lying about what the pet looks like.
 */
function luminance(hex: string): number {
  const n = parseInt(hex.replace('#', ''), 16)
  const r = ((n >> 16) & 0xff) / 255
  const g = ((n >> 8) & 0xff) / 255
  const b = (n & 0xff) / 255
  return 0.2126 * r + 0.7152 * g + 0.0722 * b
}

export function eyeFillFor(ink: string): string {
  return luminance(ink) > 0.55 ? '#111116' : '#fdfdfd'
}
