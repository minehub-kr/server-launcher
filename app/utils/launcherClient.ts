import { invoke } from '@tauri-apps/api/core'
import {
  cloneProfile,
  defaultProperties,
  emptyAccessLists,
  mockStatus,
  type AccessLists,
  type ServerKind,
  type ServerProfile,
  type ServerVersion
} from '~/types/launcher'

const mockVersions: Record<ServerKind, ServerVersion[]> = {
  paper: ['1.21.11', '1.21.10', '1.21.8', '1.20.6'].map((id) => ({ id, label: `Paper ${id}`, kind: 'release' })),
  folia: ['1.21.11', '1.21.8', '1.21.6'].map((id) => ({ id, label: `Folia ${id}`, kind: 'release' })),
  purpur: ['1.21.11', '1.21.10', '1.21.8', '1.20.6'].map((id) => ({ id, label: `Purpur ${id}`, kind: 'release' })),
  vanilla: ['1.21.11', '1.21.10', '1.21.8', '1.20.6'].map((id) => ({ id, label: `Vanilla ${id}`, kind: 'release' }))
}

const mockProfiles: ServerProfile[] = []
let mockAccess = emptyAccessLists()
let mockRuntimeStatus = mockStatus()
const mockAcceptedEula = new Set<string>()

export const isTauriRuntime = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export const launcherCall = async <T,>(command: string, args?: Record<string, unknown>) => {
  if (isTauriRuntime()) return invoke<T>(command, args)

  await new Promise((resolve) => setTimeout(resolve, 120))
  if (command === 'list_server_versions') return mockVersions[(args?.kind as ServerKind) || 'paper'] as T
  if (command === 'list_profiles') return mockProfiles as T
  if (command === 'choose_server_directory') return '/Users/example/Minecraft Servers/New Server' as T
  if (command === 'scan_java_versions') return [] as T
  if (command === 'lookup_minecraft_profile') {
    const name = String(args?.name || 'Player')
    return { name, uuid: '00000000-0000-0000-0000-000000000000' } as T
  }
  if (command === 'read_access_lists') return mockAccess as T
  if (command === 'save_access_lists') {
    mockAccess = args?.lists as AccessLists
    return mockAccess as T
  }
  if (command === 'create_profile') {
    const input = args?.input as Partial<ServerProfile> & { serverDir?: string }
    const profile: ServerProfile = {
      id: `profile-${Date.now()}`,
      name: input.name || `${input.kind || 'paper'} ${input.minecraftVersion || '1.21.11'}`,
      kind: input.kind || 'paper',
      minecraftVersion: input.minecraftVersion || '1.21.11',
      serverDir: input.serverDir || '/Users/example/Minecraft Servers/New Server',
      memoryMb: input.memoryMb || 4096,
      javaPath: null,
      lastUsed: null,
      settings: defaultProperties()
    }
    mockProfiles.push(profile)
    return profile as T
  }
  if (command === 'update_profile') {
    const profile = args?.profile as ServerProfile
    const index = mockProfiles.findIndex((item) => item.id === profile.id)
    if (index >= 0) mockProfiles[index] = cloneProfile(profile)
    return profile as T
  }
  if (command === 'delete_profile') {
    const profileId = String(args?.profileId || '')
    if (mockRuntimeStatus.currentProfileId === profileId) throw new Error('실행 중인 프로필은 삭제할 수 없습니다.')
    const index = mockProfiles.findIndex((profile) => profile.id === profileId)
    if (index < 0) throw new Error('프로필을 찾지 못했습니다.')
    mockProfiles.splice(index, 1)
    return { profiles: mockProfiles, fileDeleteError: null } as T
  }
  if (command === 'eula_status') {
    const profileId = String(args?.profileId || '')
    const profile = mockProfiles.find((item) => item.id === profileId)
    return {
      accepted: mockAcceptedEula.has(profileId),
      path: `${profile?.serverDir || '/Users/example/Minecraft Servers/New Server'}/eula.txt`,
      url: 'https://aka.ms/MinecraftEULA'
    } as T
  }
  if (command === 'accept_eula') {
    const profileId = String(args?.profileId || '')
    const profile = mockProfiles.find((item) => item.id === profileId)
    mockAcceptedEula.add(profileId)
    return {
      accepted: true,
      path: `${profile?.serverDir || '/Users/example/Minecraft Servers/New Server'}/eula.txt`,
      url: 'https://aka.ms/MinecraftEULA'
    } as T
  }
  if (command === 'resolve_server_plan') {
    const profile = mockProfiles.find((item) => item.id === args?.profileId)
    return {
      profileId: profile?.id || '',
      version: profile?.minecraftVersion || '1.21.11',
      serverKind: profile?.kind || 'paper',
      requiredJava: 21,
      javaComponent: 'java-runtime-delta',
      java: null,
      serverAvailable: true,
      serverNote: '브라우저 미리보기 모드입니다.'
    } as T
  }
  if (command === 'read_server_config') {
    return {
      properties: defaultProperties(),
      propertiesRaw: '',
      configFiles: ['bukkit.yml', 'spigot.yml', 'paper.yml', 'config/paper-global.yml'].map((relativePath) => ({
        name: relativePath.split('/').pop() || relativePath,
        relativePath,
        exists: false,
        editable: false,
        content: ''
      })),
      configFields: [],
      jsonLists: ['ops.json', 'whitelist.json', 'banned-players.json'].map((name) => ({ name, exists: false, content: '[]\n' })),
      restartRequired: false
    } as T
  }
  if (command === 'save_server_config') return args?.bundle as T
  if (command === 'start_server') {
    if (!mockAcceptedEula.has(String(args?.profileId || ''))) throw new Error('Minecraft EULA 동의가 필요합니다.')
    mockRuntimeStatus = {
      ...mockRuntimeStatus,
      running: true,
      status: 'running',
      currentProfileId: args?.profileId as string,
      logs: ['Starting mock server', 'Done (1.000s)! For help, type "help"']
    }
    return { ...mockRuntimeStatus, logs: [...mockRuntimeStatus.logs] } as T
  }
  if (command === 'stop_server') {
    mockRuntimeStatus.logs.push('> stop')
    mockRuntimeStatus = { ...mockRuntimeStatus, status: 'stopping' }
    return { ...mockRuntimeStatus, logs: [...mockRuntimeStatus.logs] } as T
  }
  if (command === 'send_server_command') {
    const serverCommand = String(args?.command || '')
    mockRuntimeStatus.logs.push(`> ${serverCommand}`)
    return { ...mockRuntimeStatus, logs: [...mockRuntimeStatus.logs] } as T
  }
  if (command === 'server_status') return { ...mockRuntimeStatus, logs: [...mockRuntimeStatus.logs] } as T
  if (command === 'network_diagnostics') {
    const profile = mockProfiles.find((item) => item.id === args?.profileId)
    const port = profile?.settings.serverPort || 25565
    return {
      port,
      localAddress: '192.168.0.20',
      publicAddress: '203.0.113.10',
      lanEndpoint: `192.168.0.20:${port}`,
      publicEndpoint: `203.0.113.10:${port}`,
      localReachable: mockRuntimeStatus.currentProfileId === profile?.id,
      externalReachable: false,
      note: '브라우저 미리보기 모드입니다.',
      checkedAt: Math.floor(Date.now() / 1000)
    } as T
  }
  if (command === 'open_upnp_port') {
    const profile = mockProfiles.find((item) => item.id === args?.profileId)
    const port = profile?.settings.serverPort || 25565
    return {
      externalAddress: '203.0.113.10',
      internalAddress: '192.168.0.20',
      externalPort: port,
      internalPort: port,
      protocol: 'TCP',
      note: '브라우저 미리보기 모드입니다.'
    } as T
  }
  if (command === 'system_metrics') {
    const cpuUsage = 24 + Math.random() * 18
    const memoryUsage = 48 + Math.random() * 9
    return {
      cpuUsage,
      memoryTotalMb: 32768,
      memoryUsedMb: Math.round(32768 * memoryUsage / 100),
      memoryUsage,
      sampledAt: Math.floor(Date.now() / 1000)
    } as T
  }
  if (command === 'list_plugins' || command === 'search_modrinth') return [] as T
  if (command === 'set_plugin_enabled') return [] as T
  if (command === 'install_modrinth_plugin') return { filename: 'plugin.jar' } as T
  if (command === 'create_backup') return { filename: 'backup.zip', path: '/tmp/backup.zip', size: 0 } as T
  if (command === 'open_server_path') return undefined as T
  if (command === 'current_app_version') return 'dev' as T
  if (command === 'check_for_update') {
    return {
      available: false,
      currentVersion: 'dev',
      version: null,
      notes: null,
      pubDate: null
    } as T
  }
  throw new Error('Tauri 앱에서 사용할 수 있는 기능입니다.')
}
