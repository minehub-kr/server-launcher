import type { CSSProperties } from 'vue'

export type ConsoleLogSegment = {
  text: string
  style?: CSSProperties
}

type LogStyle = {
  color?: string
  bold: boolean
  dim: boolean
  italic: boolean
  underline: boolean
  strike: boolean
}

const ansiColors: Record<number, string> = {
  30: '#000000',
  31: '#cd3131',
  32: '#0dbc79',
  33: '#e5e510',
  34: '#2472c8',
  35: '#bc3fbc',
  36: '#11a8cd',
  37: '#e5e5e5',
  90: '#666666',
  91: '#f14c4c',
  92: '#23d18b',
  93: '#f5f543',
  94: '#3b8eea',
  95: '#d670d6',
  96: '#29b8db',
  97: '#ffffff'
}

const minecraftColors: Record<string, string> = {
  '0': '#000000',
  '1': '#0000aa',
  '2': '#00aa00',
  '3': '#00aaaa',
  '4': '#aa0000',
  '5': '#aa00aa',
  '6': '#ffaa00',
  '7': '#aaaaaa',
  '8': '#555555',
  '9': '#5555ff',
  a: '#55ff55',
  b: '#55ffff',
  c: '#ff5555',
  d: '#ff55ff',
  e: '#ffff55',
  f: '#ffffff'
}

const styleKey = (style: CSSProperties = {}) => JSON.stringify(style)
const freshStyle = (): LogStyle => ({ bold: false, dim: false, italic: false, underline: false, strike: false })
const byteToHex = (value: number) => value.toString(16).padStart(2, '0')
const rgb = (red: number, green: number, blue: number) => `#${byteToHex(red)}${byteToHex(green)}${byteToHex(blue)}`
const isByte = (value: unknown): value is number => typeof value === 'number' && Number.isInteger(value) && value >= 0 && value <= 255

const xterm256 = (code: number) => {
  if (ansiColors[code]) return ansiColors[code]
  if (code >= 16 && code <= 231) {
    const value = code - 16
    const channel = (step: number) => step === 0 ? 0 : 55 + step * 40
    return rgb(channel(Math.floor(value / 36)), channel(Math.floor((value % 36) / 6)), channel(value % 6))
  }
  if (code >= 232 && code <= 255) {
    const value = 8 + (code - 232) * 10
    return rgb(value, value, value)
  }
  return undefined
}

const toCss = (state: LogStyle): CSSProperties | undefined => {
  const decorations = [
    state.underline && 'underline',
    state.strike && 'line-through'
  ].filter(Boolean) as string[]
  const style: CSSProperties = {}

  if (state.color) style.color = state.color
  if (state.bold) style.fontWeight = '700'
  if (state.dim) style.opacity = '0.72'
  if (state.italic) style.fontStyle = 'italic'
  if (decorations.length) style.textDecorationLine = decorations.join(' ')

  return Object.keys(style).length ? style : undefined
}

const applyAnsi = (state: LogStyle, paramsText: string) => {
  const params = (paramsText || '0').split(';').map((param) => Number(param || 0))

  for (let index = 0; index < params.length; index += 1) {
    const code = params[index] ?? 0
    if (code === 0) Object.assign(state, freshStyle())
    else if (code === 1) state.bold = true
    else if (code === 2) state.dim = true
    else if (code === 3) state.italic = true
    else if (code === 4) state.underline = true
    else if (code === 9) state.strike = true
    else if (code === 22) {
      state.bold = false
      state.dim = false
    } else if (code === 23) state.italic = false
    else if (code === 24) state.underline = false
    else if (code === 29) state.strike = false
    else if (code === 39) state.color = undefined
    else if (ansiColors[code]) state.color = ansiColors[code]
    else if (code === 38) {
      const mode = params[++index]
      if (mode === 2) {
        const [red, green, blue] = [params[index + 1], params[index + 2], params[index + 3]]
        if (isByte(red) && isByte(green) && isByte(blue)) {
          state.color = rgb(red, green, blue)
          index += 3
        }
      } else if (mode === 5) {
        const colorCode = params[++index]
        const color = typeof colorCode === 'number' ? xterm256(colorCode) : undefined
        if (color) state.color = color
      }
    }
  }
}

const applyMinecraftCode = (state: LogStyle, code: string) => {
  if (minecraftColors[code]) {
    Object.assign(state, freshStyle(), { color: minecraftColors[code] })
  } else if (code === 'l') state.bold = true
  else if (code === 'm') state.strike = true
  else if (code === 'n') state.underline = true
  else if (code === 'o') state.italic = true
  else if (code === 'r') Object.assign(state, freshStyle())
}

export const parseConsoleLine = (line: string): ConsoleLogSegment[] => {
  const segments: ConsoleLogSegment[] = []
  const state = freshStyle()
  let buffer = ''
  let index = 0

  const push = () => {
    if (!buffer) return
    const style = toCss(state)
    const last = segments.at(-1)
    if (last && styleKey(last.style) === styleKey(style)) last.text += buffer
    else segments.push({ text: buffer, style })
    buffer = ''
  }

  while (index < line.length) {
    if (line[index] === '\x1b' && line[index + 1] === '[') {
      const match = /^\x1b\[([0-9;?]*)([A-Za-z])/.exec(line.slice(index))
      if (match) {
        push()
        if (match[2] === 'm') applyAnsi(state, (match[1] || '').replaceAll('?', ''))
        index += match[0].length
        continue
      }
    }

    const code = line[index + 1]?.toLowerCase()
    if ((line[index] === '§' || line[index] === '&') && code && /^[0-9a-fklmnor]$/.test(code)) {
      push()
      applyMinecraftCode(state, code)
      index += 2
      continue
    }

    buffer += line[index]
    index += 1
  }

  push()
  return segments
}

export const stripConsoleCodes = (line: string) => parseConsoleLine(line).map((segment) => segment.text).join('')
