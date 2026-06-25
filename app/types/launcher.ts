export type ServerKind = 'paper' | 'folia' | 'purpur' | 'vanilla'
export type ThemeMode = 'system' | 'dark' | 'light'
export type MainTab = 'console' | 'settings' | 'plugins' | 'players' | 'ops' | 'backup'
export type AccessKind = 'ops' | 'whitelist' | 'bannedPlayers'
export type PlayerAction = 'kick' | 'ban' | 'pardon' | 'op' | 'deop' | 'whitelist-add' | 'whitelist-remove' | 'gamemode'

export type ServerVersion = {
  id: string
  kind: string
  label: string
}

export type JavaRuntime = {
  path: string
  major: number
  version: string
}

export type ServerProperties = {
  serverPort: number
  onlineMode: boolean
  motd: string
  maxPlayers: number
  difficulty: string
  gamemode: string
  pvp: boolean
  viewDistance: number
  simulationDistance: number
  enableCommandBlock: boolean
  whiteList: boolean
}

export type ServerProfile = {
  id: string
  name: string
  kind: ServerKind
  minecraftVersion: string
  serverDir: string
  memoryMb: number
  javaPath: string | null
  lastUsed: string | null
  settings: ServerProperties
}

export type ServerPlan = {
  profileId: string
  version: string
  serverKind: ServerKind
  requiredJava: number
  javaComponent: string
  java: JavaRuntime | null
  serverAvailable: boolean
  serverNote: string
}

export type ServerStatus = {
  running: boolean
  status: string
  players: string[]
  logs: string[]
  currentProfileId: string | null
  java: JavaRuntime | null
  dataDir: string
  crashDetected: boolean
  exitMessage: string | null
}

export type ServerLogEvent = {
  line: string
}

export type ConfigFile = {
  name: string
  relativePath: string
  exists: boolean
  editable: boolean
  content: string
}

export type ConfigFieldKind = 'boolean' | 'number' | 'text' | 'select'

export type ConfigFormField = {
  file: string
  path: string
  label: string
  kind: ConfigFieldKind
  value: boolean | number | string
  options?: string[]
  restartRequired: boolean
}

export type JsonListFile = {
  name: string
  exists: boolean
  content: string
}

export type ServerConfigBundle = {
  properties: ServerProperties
  propertiesRaw: string
  configFiles: ConfigFile[]
  configFields: ConfigFormField[]
  jsonLists: JsonListFile[]
  restartRequired: boolean
}

export type OpEntry = {
  uuid: string
  name: string
  level: number
  bypassesPlayerLimit: boolean
}

export type WhitelistEntry = {
  uuid: string
  name: string
}

export type BanEntry = {
  uuid: string
  name: string
  created: string
  source: string
  expires: string
  reason: string
}

export type AccessLists = {
  ops: OpEntry[]
  whitelist: WhitelistEntry[]
  bannedPlayers: BanEntry[]
  rawOps: string
  rawWhitelist: string
  rawBannedPlayers: string
}

export type MinecraftIdentity = {
  name: string
  uuid: string
}

export type PluginFile = {
  filename: string
  displayName: string
  enabled: boolean
  size: number
}

export type ModrinthProject = {
  project_id: string
  slug: string
  title: string
  description: string
  downloads: number
  icon_url: string | null
  categories: string[]
  versions: string[]
}

export type InstalledPlugin = {
  title: string
  version: string
  filename: string
  path: string
}

export type BackupInfo = {
  filename: string
  path: string
  size: number
}

export const defaultProperties = (): ServerProperties => ({
  serverPort: 25565,
  onlineMode: true,
  motd: 'A Minecraft Server',
  maxPlayers: 20,
  difficulty: 'easy',
  gamemode: 'survival',
  pvp: true,
  viewDistance: 10,
  simulationDistance: 10,
  enableCommandBlock: false,
  whiteList: false
})

export const emptyAccessLists = (): AccessLists => ({
  ops: [],
  whitelist: [],
  bannedPlayers: [],
  rawOps: '[]\n',
  rawWhitelist: '[]\n',
  rawBannedPlayers: '[]\n'
})

export const mockStatus = (): ServerStatus => ({
  running: false,
  status: 'stopped',
  players: [],
  logs: ['서버 로그가 여기에 표시됩니다.'],
  currentProfileId: null,
  java: null,
  dataDir: '',
  crashDetected: false,
  exitMessage: null
})

export const cloneProfile = (profile: ServerProfile): ServerProfile => ({
  ...profile,
  settings: { ...profile.settings }
})

export const serverKinds = [
  { label: 'Paper', value: 'paper' as ServerKind, icon: 'i-lucide-plug' },
  { label: 'Folia', value: 'folia' as ServerKind, icon: 'i-lucide-network' },
  { label: 'Purpur', value: 'purpur' as ServerKind, icon: 'i-lucide-settings-2' },
  { label: 'Vanilla', value: 'vanilla' as ServerKind, icon: 'i-lucide-box' }
]

export const tabs = [
  { label: '콘솔', value: 'console' as MainTab, icon: 'i-lucide-terminal' },
  { label: '설정', value: 'settings' as MainTab, icon: 'i-lucide-sliders-horizontal' },
  { label: '플러그인', value: 'plugins' as MainTab, icon: 'i-lucide-package' },
  { label: '플레이어', value: 'players' as MainTab, icon: 'i-lucide-users' },
  { label: '권한/차단', value: 'ops' as MainTab, icon: 'i-lucide-shield' },
  { label: '백업', value: 'backup' as MainTab, icon: 'i-lucide-archive' }
]

export const themeOptions = [
  { label: '시스템', value: 'system' as ThemeMode, icon: 'i-lucide-monitor' },
  { label: '라이트', value: 'light' as ThemeMode, icon: 'i-lucide-sun' },
  { label: '다크', value: 'dark' as ThemeMode, icon: 'i-lucide-moon' }
]

export const playerActionOptions = [
  { label: '킥', value: 'kick' as PlayerAction },
  { label: '밴', value: 'ban' as PlayerAction },
  { label: '밴 해제', value: 'pardon' as PlayerAction },
  { label: 'OP 부여', value: 'op' as PlayerAction },
  { label: 'OP 해제', value: 'deop' as PlayerAction },
  { label: '화이트리스트 추가', value: 'whitelist-add' as PlayerAction },
  { label: '화이트리스트 삭제', value: 'whitelist-remove' as PlayerAction },
  { label: '게임 모드 변경', value: 'gamemode' as PlayerAction }
]

export const gamemodeOptions = ['survival', 'creative', 'adventure', 'spectator'].map((mode) => ({
  label: mode,
  value: mode
}))

const statusText: Record<string, string> = {
  running: '실행 중',
  stopped: '중지됨',
  stopping: '종료 중',
  crashed: '충돌'
}

export const statusLabel = (value = 'stopped') => statusText[value] || value || '알 수 없음'
export const statusColor = (value = 'stopped') =>
  value === 'running' ? 'success' : value === 'crashed' ? 'error' : value === 'stopping' ? 'warning' : 'neutral'
