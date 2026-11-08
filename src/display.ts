/**
 * Display names for the engine's catalogues.
 *
 * The ids (`cercle`, `encre`, `somnolent`) are French because they come from the
 * upstream project and are what `config.json` stores — renaming them would break
 * every existing file and the engine's own tests. So the ids stay as storage
 * keys and this is the English the interface speaks.
 *
 * An unknown id falls back to the id itself, which is what the pet does too: a
 * hand-edited config degrades rather than blanks out.
 */

export const SHAPE_NAMES: Record<string, string> = {
  cercle: 'Circle',
  galet: 'Pebble',
  squircle: 'Squircle',
  capsule: 'Capsule',
  triangle: 'Triangle',
  hexagone: 'Hexagon',
  nuage: 'Cloud',
  goutte: 'Drop'
}

export const COLOR_NAMES: Record<string, string> = {
  encre: 'Ink',
  brun: 'Brown',
  rouge: 'Red',
  orange: 'Orange',
  ambre: 'Amber',
  vert: 'Green',
  turquoise: 'Turquoise',
  bleu: 'Blue',
  violet: 'Violet',
  rose: 'Rose',
  gris: 'Grey',
  creme: 'Cream'
}

export const EXPRESSION_NAMES: Record<string, string> = {
  neutre: 'Neutral',
  attentif: 'Attentive',
  surpris: 'Surprised',
  excite: 'Excited',
  heureux: 'Happy',
  hilare: 'Giggling',
  colere: 'Angry',
  triste: 'Sad',
  effraye: 'Scared',
  mefiant: 'Wary',
  confus: 'Confused',
  curieux: 'Curious',
  fier: 'Proud',
  timide: 'Shy',
  blase: 'Bored',
  somnolent: 'Sleepy'
}

export const STYLE_NAMES: Record<string, string> = {
  plat: 'Flat',
  argile: 'Clay',
  peluche: 'Plush'
}

export function displayName(names: Record<string, string>, id: string): string {
  return names[id] ?? id
}