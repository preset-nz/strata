export const COLOUR_LABELS = [
  "red",
  "orange",
  "yellow",
  "green",
  "blue",
  "purple",
  "grey",
] as const

export type ColourLabel = (typeof COLOUR_LABELS)[number]

export const COLOUR_SWATCH: Record<ColourLabel, string> = {
  red: "bg-red-500",
  orange: "bg-orange-500",
  yellow: "bg-yellow-400",
  green: "bg-green-500",
  blue: "bg-blue-500",
  purple: "bg-purple-500",
  grey: "bg-zinc-400",
}
