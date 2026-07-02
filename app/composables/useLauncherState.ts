import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useColorMode, useToast } from '#imports'
import { computed, inject, onBeforeUnmount, onMounted, provide, reactive, ref, watch, type InjectionKey } from 'vue'
import { launcherCall as call, isTauriRuntime as isTauri } from '~/utils/launcherClient'
import { stripConsoleCodes } from '~/utils/consoleLog'
import {
  cloneProfile,
  emptyAccessLists,
  gamemodeOptions,
  mockStatus,
  playerActionOptions,
  serverKinds,
  statusColor,
  statusLabel,
  tabs,
  themeOptions,
  type AccessKind,
  type AccessLists,
  type AppUpdateInfo,
  type BackupInfo,
  type DeleteProfileResult,
  type EulaStatus,
  type InstalledPlugin,
  type JavaRuntime,
  type MetricPoint,
  type MinecraftIdentity,
  type MainTab,
  type ModrinthProject,
  type NetworkDiagnostics,
  type PlayerAction,
  type PluginFile,
  type PluginUpdateSummary,
  type ServerConfigBundle,
  type ServerKind,
  type ServerLogEvent,
  type ServerPlan,
  type ServerProfile,
  type ServerStatus,
  type ServerVersion,
  type SystemMetrics,
  type ThemeMode,
  type UpdatedPlugin,
  type UpdateProgressEvent,
  type UpnpMappingResult
} from '~/types/launcher'

export const useLauncherState = () => {
  const colorMode = useColorMode() as unknown as { preference: ThemeMode }
  const toast = useToast()
  const profiles = ref<ServerProfile[]>([])
  const profilesLoaded = ref(false)
  const selectedProfileId = ref('')
  const profileDraft = ref<ServerProfile | null>(null)
  const versions = ref<ServerVersion[]>([])
  const createVersions = ref<ServerVersion[]>([])
  const javaVersions = ref<JavaRuntime[]>([])
  const plan = ref<ServerPlan | null>(null)
  const status = ref<ServerStatus>(mockStatus())
  const profileLogs = ref<Record<string, string[]>>({})
  const eula = ref<EulaStatus | null>(null)
  const eulaDialogOpen = ref(false)
  const eulaAgreementChecked = ref(false)
  const network = ref<NetworkDiagnostics | null>(null)
  const networkAutoRefreshing = ref(false)
  const metrics = ref<SystemMetrics | null>(null)
  const metricHistory = ref<MetricPoint[]>([])
  const config = ref<ServerConfigBundle | null>(null)
  const accessLists = ref<AccessLists>(emptyAccessLists())
  const plugins = ref<PluginFile[]>([])
  const modrinthProjects = ref<ModrinthProject[]>([])
  const pluginUpdateSummary = ref<PluginUpdateSummary | null>(null)
  const mainTab = ref<MainTab>('console')
  const appSettingsOpen = ref(false)
  const newProfileOpen = ref(false)
  const profileDeleteOpen = ref(false)
  const deleteProfileFiles = ref(false)
  const versionLoading = ref(false)
  const createVersionLoading = ref(false)
  const logQuery = ref('')
  const commandText = ref('')
  const pluginQuery = ref('')
  const loading = ref('')
  const backupInfo = ref<BackupInfo | null>(null)
  const appVersion = ref('')
  const playerActionOpen = ref(false)
  const selectedPlayerName = ref('')
  const playerAction = ref<PlayerAction>('kick')
  const playerActionReason = ref('')
  const playerActionGamemode = ref('survival')
  const accessName = reactive<Record<AccessKind, string>>({ ops: '', whitelist: '', bannedPlayers: '' })
  const banReason = ref('Banned by an operator.')
  const accessRawOpen = reactive<Record<AccessKind, boolean>>({ ops: false, whitelist: false, bannedPlayers: false })
  const defaultNewProfile = () => ({
    name: '',
    kind: 'paper' as ServerKind,
    minecraftVersion: '',
    serverDir: '',
    memoryMb: 4096
  })
  const newProfile = ref(defaultNewProfile())
  const appUpdate = ref<AppUpdateInfo | null>(null)
  const appUpdateDismissed = ref(false)
  const appUpdateProgress = ref(0)
  const appUpdateProgressLabel = ref('')
  const appUpdateInstalling = ref(false)
  const emptyLogLine = '서버 로그가 여기에 표시됩니다.'

  let pollTimer: ReturnType<typeof setInterval> | undefined
  let networkTimer: ReturnType<typeof setInterval> | undefined
  let metricsTimer: ReturnType<typeof setInterval> | undefined
  const unlisteners: UnlistenFn[] = []
  let versionRequestId = 0
  let createVersionRequestId = 0
  let appUpdateDownloadedBytes = 0
  let runtimeLogProfileId = ''

  const selectedProfile = computed(() => profiles.value.find((profile) => profile.id === selectedProfileId.value) || null)
  const needsOnboarding = computed(() => profilesLoaded.value && profiles.value.length === 0)
  const activeProfileRunning = computed(() => status.value.running && status.value.currentProfileId === selectedProfileId.value)
  const anyServerRunning = computed(() => status.value.running)
  const canUsePlugins = computed(() => !!selectedProfile.value && selectedProfile.value.kind !== 'vanilla')
  const pluginUpdateCount = computed(() => pluginUpdateSummary.value?.updatable || 0)
  const hasPluginUpdates = computed(() => pluginUpdateCount.value > 0)
  const selectedProfileLogs = computed(() => {
    const logs = selectedProfileId.value ? profileLogs.value[selectedProfileId.value] : []
    return logs?.length ? logs : [emptyLogLine]
  })
  const filteredLogs = computed(() => {
    const query = logQuery.value.trim().toLowerCase()
    return selectedProfileLogs.value.filter((line) => !query || stripConsoleCodes(line).toLowerCase().includes(query))
  })
  const versionOptions = computed(() => versions.value.map((version) => ({ label: version.id, value: version.id })))
  const createVersionOptions = computed(() => createVersions.value.map((version) => ({ label: version.id, value: version.id })))
  const profileKindLabel = computed(() => selectedProfile.value ? serverKinds.find((kind) => kind.value === selectedProfile.value?.kind)?.label : '-')
  const selectedJava = computed(() => javaVersions.value.find((java) => java.path === profileDraft.value?.javaPath) || null)
  const selectedJavaPath = computed(() => selectedJava.value?.path || profileDraft.value?.javaPath || '자동 선택')
  const whitelistEnabled = computed(() => config.value?.properties.whiteList === true)
  const profileRuntimeStatus = (profile: ServerProfile) => status.value.currentProfileId === profile.id ? status.value.status : 'stopped'
  const formatJson = (value: unknown) => `${JSON.stringify(value, null, 2)}\n`
  const validPlayerName = (name: string) => /^[A-Za-z0-9_]{1,16}$/.test(name)
  const sampleLabel = (sampledAt: number) => new Date(sampledAt * 1000).toLocaleTimeString('ko-KR', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false
  })

  const friendlyError = (error: unknown) => {
    const text = error instanceof Error ? error.message : String(error)
    if (text.includes('404') || text.includes('Not Found')) return '선택한 서버 구현체가 해당 Minecraft 버전을 지원하지 않습니다.'
    return text
  }

  const notifySuccess = (title: string, description?: string) => {
    toast.add({ title, description, color: 'success', icon: 'i-lucide-check' })
  }

  const notifyError = (title: string, description?: string) => {
    toast.add({ title, description, color: 'error', icon: 'i-lucide-circle-alert' })
  }

  const notifyWarning = (title: string, description?: string) => {
    toast.add({ title, description, color: 'warning', icon: 'i-lucide-triangle-alert' })
  }

  const setError = (error: unknown) => {
    notifyError('작업 실패', friendlyError(error))
  }

  const updateProfileLogs = (profileId: string, logs: string[]) => {
    profileLogs.value = {
      ...profileLogs.value,
      [profileId]: logs.slice(-1000)
    }
  }

  const pushRuntimeLog = (line: string) => {
    const profileId = status.value.currentProfileId || runtimeLogProfileId
    if (!profileId) return
    const logs = profileLogs.value[profileId] || []
    if (logs.at(-1) === line) return
    const nextLogs = [...logs, line]
    updateProfileLogs(profileId, nextLogs)
    if (status.value.currentProfileId === profileId) {
      status.value = { ...status.value, logs: nextLogs.slice(-1000) }
    }
  }

  const applyStatus = (next: ServerStatus) => {
    const profileId = next.currentProfileId || status.value.currentProfileId || runtimeLogProfileId
    const previousLogs = profileId ? profileLogs.value[profileId] || [] : status.value.logs
    const sameRuntime = profileId && profileId === status.value.currentProfileId && next.status === status.value.status
    const logs = sameRuntime && previousLogs.length >= next.logs.length ? [...previousLogs] : [...next.logs]
    if (profileId) updateProfileLogs(profileId, logs)
    runtimeLogProfileId = next.running || next.currentProfileId ? profileId || '' : ''
    status.value = { ...next, logs }
  }

  const runTask = async <T,>(key: string, task: () => Promise<T>) => {
    loading.value = key
    try {
      return await task()
    } catch (error) {
      setError(error)
    } finally {
      loading.value = ''
    }
  }

  const loadVersions = async (kind: ServerKind, forCreate = false, clearSelection = false) => {
    const requestId = forCreate ? ++createVersionRequestId : ++versionRequestId
    const previousVersion = profileDraft.value?.minecraftVersion || ''

    if (forCreate) {
      createVersionLoading.value = true
      createVersions.value = []
      newProfile.value.minecraftVersion = ''
    } else {
      versionLoading.value = true
      if (clearSelection && profileDraft.value?.kind === kind) {
        versions.value = []
        profileDraft.value.minecraftVersion = ''
      }
    }

    try {
      const result = await call<ServerVersion[]>('list_server_versions', { kind })
      if (forCreate) {
        if (requestId !== createVersionRequestId || newProfile.value.kind !== kind) return
        createVersions.value = result
        newProfile.value.minecraftVersion = result[0]?.id || ''
        return
      }

      if (requestId !== versionRequestId || profileDraft.value?.kind !== kind) return
      versions.value = result
      if (profileDraft.value) {
        profileDraft.value.minecraftVersion = previousVersion && result.some((version) => version.id === previousVersion)
          ? previousVersion
          : result[0]?.id || ''
      }
    } finally {
      if (forCreate) {
        if (requestId === createVersionRequestId && newProfile.value.kind === kind) createVersionLoading.value = false
      } else if (requestId === versionRequestId && profileDraft.value?.kind === kind) {
        versionLoading.value = false
      }
    }
  }

  const refreshProfiles = async () => {
    try {
      profiles.value = await call<ServerProfile[]>('list_profiles')
      if (!selectedProfileId.value || !profiles.value.some((profile) => profile.id === selectedProfileId.value)) {
        selectedProfileId.value = profiles.value[0]?.id || ''
      }
    } finally {
      profilesLoaded.value = true
    }
  }

  const refreshStatus = async () => {
    const previousStatus = status.value.status
    const previousProfileId = status.value.currentProfileId
    applyStatus(await call<ServerStatus>('server_status'))
    if (selectedProfile.value && (previousStatus !== status.value.status || previousProfileId !== status.value.currentProfileId)) {
      await refreshConfig()
      loadNetworkDiagnostics().catch(() => { network.value = null })
    }
  }

  const refreshPlan = async () => {
    plan.value = selectedProfile.value ? await call<ServerPlan>('resolve_server_plan', { profileId: selectedProfile.value.id }) : null
  }

  const refreshConfig = async () => {
    config.value = selectedProfile.value ? await call<ServerConfigBundle>('read_server_config', { profileId: selectedProfile.value.id }) : null
  }

  const loadNetworkDiagnostics = async () => {
    network.value = selectedProfile.value
      ? await call<NetworkDiagnostics>('network_diagnostics', { profileId: selectedProfile.value.id })
      : null
  }

  const refreshNetworkDiagnostics = async () => runTask('network-diagnostics', loadNetworkDiagnostics)

  const refreshNetworkDiagnosticsInBackground = async () => {
    if (!selectedProfile.value || networkAutoRefreshing.value || loading.value === 'network-diagnostics') return
    networkAutoRefreshing.value = true
    try {
      await loadNetworkDiagnostics()
    } catch {
      network.value = null
    } finally {
      networkAutoRefreshing.value = false
    }
  }

  const openUpnpPort = async () => runTask('upnp', async () => {
    if (!selectedProfile.value) return
    const result = await call<UpnpMappingResult>('open_upnp_port', { profileId: selectedProfile.value.id })
    notifySuccess('UPnP 포트를 열었습니다.', `${result.externalAddress || result.internalAddress}:${result.externalPort}/${result.protocol}`)
    await loadNetworkDiagnostics()
  })

  const pushMetrics = (next: SystemMetrics) => {
    metrics.value = next
    metricHistory.value = [
      ...metricHistory.value.slice(-39),
      { ...next, label: sampleLabel(next.sampledAt) }
    ]
  }

  const refreshSystemMetrics = async () => {
    pushMetrics(await call<SystemMetrics>('system_metrics'))
  }

  const refreshAccessLists = async () => {
    accessLists.value = selectedProfile.value
      ? await call<AccessLists>('read_access_lists', { profileId: selectedProfile.value.id })
      : emptyAccessLists()
  }

  const refreshPlugins = async () => {
    const profile = selectedProfile.value
    if (!profile || profile.kind === 'vanilla') {
      plugins.value = []
      modrinthProjects.value = []
      pluginUpdateSummary.value = null
      return
    }
    plugins.value = await call<PluginFile[]>('list_plugins', { profileId: profile.id })
  }

  const refreshProfileData = async () => {
    const profile = selectedProfile.value
    if (!profile) {
      profileDraft.value = null
      config.value = null
      eula.value = null
      closeEulaDialog()
      network.value = null
      plugins.value = []
      pluginUpdateSummary.value = null
      accessLists.value = emptyAccessLists()
      return
    }

    profileDraft.value = cloneProfile(profile)
    await loadVersions(profile.kind)
    await Promise.all([refreshPlan(), refreshConfig(), refreshAccessLists(), refreshPlugins()])
    checkPluginUpdates().catch(() => {})
    refreshEulaStatus().catch(() => { eula.value = null })
    loadNetworkDiagnostics().catch(() => { network.value = null })
  }

  const chooseDirectory = async (target: 'create' | 'profile') => {
    const path = await call<string | null>('choose_server_directory')
    if (!path) return
    if (target === 'create') newProfile.value.serverDir = path
    if (target === 'profile' && profileDraft.value) profileDraft.value.serverDir = path
  }

  const resetNewProfile = () => {
    newProfile.value = defaultNewProfile()
  }

  const openNewProfileModal = () => {
    resetNewProfile()
    newProfileOpen.value = true
    loadVersions(newProfile.value.kind, true).catch(setError)
  }

  const closeNewProfileModal = () => {
    newProfileOpen.value = false
  }

  const openDeleteProfileDialog = () => {
    if (!selectedProfile.value) return
    deleteProfileFiles.value = false
    profileDeleteOpen.value = true
  }

  const closeDeleteProfileDialog = () => {
    profileDeleteOpen.value = false
    deleteProfileFiles.value = false
  }

  const openEulaDialog = (status: EulaStatus) => {
    eula.value = status
    eulaAgreementChecked.value = false
    eulaDialogOpen.value = true
  }

  const closeEulaDialog = () => {
    eulaDialogOpen.value = false
    eulaAgreementChecked.value = false
  }

  const createProfile = async () => runTask('create-profile', async () => {
    if (!newProfile.value.minecraftVersion) await loadVersions(newProfile.value.kind, true)
    if (!newProfile.value.minecraftVersion) throw new Error('선택 가능한 Minecraft 버전이 없습니다.')
    const profile = await call<ServerProfile>('create_profile', { input: newProfile.value })
    profiles.value = await call<ServerProfile[]>('list_profiles')
    selectedProfileId.value = profile.id
    closeNewProfileModal()
    resetNewProfile()
    notifySuccess('프로필을 만들었습니다.')
  })

  const saveProfile = async () => runTask('save-profile', async () => {
    if (!profileDraft.value) return
    const saved = await call<ServerProfile>('update_profile', { profile: profileDraft.value })
    const index = profiles.value.findIndex((profile) => profile.id === saved.id)
    if (index >= 0) profiles.value[index] = cloneProfile(saved)
    notifySuccess('프로필 설정을 저장했습니다.')
    await refreshProfileData()
  })

  const deleteProfile = async () => runTask('delete-profile', async () => {
    const profile = selectedProfile.value
    if (!profile) return
    if (activeProfileRunning.value) throw new Error('실행 중인 프로필은 삭제할 수 없습니다.')

    const result = await call<DeleteProfileResult>('delete_profile', {
      profileId: profile.id,
      deleteFiles: deleteProfileFiles.value
    })
    profiles.value = result.profiles
    selectedProfileId.value = profiles.value[0]?.id || ''
    closeDeleteProfileDialog()
    notifySuccess('프로필을 삭제했습니다.')
    if (result.fileDeleteError) {
      notifyWarning('프로필은 삭제했지만 서버 폴더는 남았습니다.', result.fileDeleteError)
    }
    await refreshProfileData()
  })

  const saveConfig = async () => runTask('save-config', async () => {
    if (!selectedProfile.value || !config.value) return
    config.value = await call<ServerConfigBundle>('save_server_config', {
      profileId: selectedProfile.value.id,
      bundle: config.value
    })
    await refreshProfiles()
    notifySuccess('서버 설정을 저장했습니다.', '일부 값은 재시작 후 적용됩니다.')
  })

  const startSelectedServer = async () => {
    if (!selectedProfile.value) return
    runtimeLogProfileId = selectedProfile.value.id
    applyStatus(await call<ServerStatus>('start_server', { profileId: selectedProfile.value.id }))
    await refreshProfileData()
  }

  const toggleServer = async () => runTask('server', async () => {
    if (!selectedProfile.value) return
    if (activeProfileRunning.value) {
      applyStatus(await call<ServerStatus>('stop_server'))
      await refreshProfileData()
      return
    }

    const status = await call<EulaStatus>('eula_status', { profileId: selectedProfile.value.id })
    eula.value = status
    if (!status.accepted) {
      openEulaDialog(status)
      return
    }
    await startSelectedServer()
  })

  const acceptEulaAndStart = async () => runTask('server', async () => {
    if (!selectedProfile.value || !eulaAgreementChecked.value) return
    eula.value = await call<EulaStatus>('accept_eula', { profileId: selectedProfile.value.id })
    closeEulaDialog()
    await startSelectedServer()
  })

  const refreshEulaStatus = async () => {
    eula.value = selectedProfile.value
      ? await call<EulaStatus>('eula_status', { profileId: selectedProfile.value.id })
      : null
  }

  const sendCommand = async (command = commandText.value) => runTask('command', async () => {
    const trimmed = command.trim()
    if (!trimmed) return
    commandText.value = ''
    applyStatus(await call<ServerStatus>('send_server_command', { command: trimmed }))
  })

  const refreshPlayers = async () => runTask('players-refresh', async () => {
    if (!activeProfileRunning.value) throw new Error('실행 중인 서버에서만 플레이어 목록을 갱신할 수 있습니다.')
    applyStatus(await call<ServerStatus>('send_server_command', { command: 'list' }))
  })

  const openPlayerAction = (player: string) => {
    selectedPlayerName.value = player
    playerAction.value = 'kick'
    playerActionReason.value = ''
    playerActionGamemode.value = 'survival'
    playerActionOpen.value = true
  }

  const closePlayerAction = () => {
    playerActionOpen.value = false
  }

  const playerActionCommand = () => {
    const player = selectedPlayerName.value.trim()
    if (!validPlayerName(player)) throw new Error('올바른 Minecraft 닉네임이 아닙니다.')

    const reason = playerActionReason.value.trim().replace(/\s+/g, ' ')
    if (playerAction.value === 'kick') return `kick ${player} ${reason || 'Kicked by an operator.'}`
    if (playerAction.value === 'ban') return `ban ${player} ${reason || 'Banned by an operator.'}`
    if (playerAction.value === 'pardon') return `pardon ${player}`
    if (playerAction.value === 'op') return `op ${player}`
    if (playerAction.value === 'deop') return `deop ${player}`
    if (playerAction.value === 'whitelist-add') return `whitelist add ${player}`
    if (playerAction.value === 'whitelist-remove') return `whitelist remove ${player}`
    return `gamemode ${playerActionGamemode.value} ${player}`
  }

  const runPlayerAction = async () => runTask('player-action', async () => {
    if (!activeProfileRunning.value) throw new Error('실행 중인 서버에서만 플레이어 명령을 전송할 수 있습니다.')

    const action = playerAction.value
    const command = playerActionCommand()
    applyStatus(await call<ServerStatus>('send_server_command', { command }))

    if (['kick', 'ban', 'pardon'].includes(action)) {
      applyStatus(await call<ServerStatus>('send_server_command', { command: 'list' }))
    }

    if (['op', 'deop', 'ban', 'pardon', 'whitelist-add', 'whitelist-remove'].includes(action)) {
      refreshAccessLists().catch(() => {})
    }

    closePlayerAction()
    notifySuccess('플레이어 명령을 전송했습니다.', `> ${command}`)
  })

  const searchPlugins = async () => runTask('search-plugins', async () => {
    if (!selectedProfile.value || selectedProfile.value.kind === 'vanilla') return
    modrinthProjects.value = await call<ModrinthProject[]>('search_modrinth', {
      query: pluginQuery.value,
      gameVersion: selectedProfile.value.minecraftVersion,
      loader: selectedProfile.value.kind
    })
  })

  const checkPluginUpdates = async () => runTask('plugin-updates', async () => {
    if (!selectedProfile.value || selectedProfile.value.kind === 'vanilla') {
      pluginUpdateSummary.value = null
      return
    }

    pluginUpdateSummary.value = await call<PluginUpdateSummary>('check_plugin_updates', {
      profileId: selectedProfile.value.id
    })
    plugins.value = pluginUpdateSummary.value.plugins
    if (pluginUpdateSummary.value.updatable > 0) {
      notifyWarning(
        '플러그인 업데이트가 있습니다.',
        `${pluginUpdateSummary.value.updatable}개 플러그인을 업데이트할 수 있습니다.`
      )
    }
  })

  const installPlugin = async (project: ModrinthProject) => runTask(`install-${project.project_id}`, async () => {
    if (!selectedProfile.value) return
    const installed = await call<InstalledPlugin>('install_modrinth_plugin', {
      profileId: selectedProfile.value.id,
      projectId: project.project_id,
      title: project.title,
      loader: selectedProfile.value.kind
    })
    notifySuccess('플러그인을 설치했습니다.', `${installed.filename} - 서버 재시작 후 로드됩니다.`)
    await refreshPlugins()
    await checkPluginUpdates()
  })

  const setPluginEnabled = async (plugin: PluginFile, enabled: boolean) => runTask(`plugin-${plugin.filename}`, async () => {
    if (!selectedProfile.value) return
    plugins.value = await call<PluginFile[]>('set_plugin_enabled', {
      profileId: selectedProfile.value.id,
      filename: plugin.filename,
      enabled
    })
    notifySuccess(
      enabled ? '플러그인을 활성화했습니다.' : '플러그인을 비활성화했습니다.',
      '서버 재시작 후 적용됩니다.'
    )
    await checkPluginUpdates()
  })

  const installPluginUpdate = async (plugin: PluginFile) => runTask(`plugin-update-${plugin.filename}`, async () => {
    if (!selectedProfile.value) return
    const updated = await call<UpdatedPlugin>('install_plugin_update', {
      profileId: selectedProfile.value.id,
      filename: plugin.filename
    })
    notifySuccess('플러그인을 업데이트했습니다.', `${updated.displayName} ${updated.version}`)
    await refreshPlugins()
    await checkPluginUpdates()
  })

  const accessPayload = () => ({
    ...accessLists.value,
    rawOps: accessRawOpen.ops ? accessLists.value.rawOps : formatJson(accessLists.value.ops),
    rawWhitelist: accessRawOpen.whitelist ? accessLists.value.rawWhitelist : formatJson(accessLists.value.whitelist),
    rawBannedPlayers: accessRawOpen.bannedPlayers ? accessLists.value.rawBannedPlayers : formatJson(accessLists.value.bannedPlayers)
  })

  const addAccessEntry = async (kind: AccessKind) => runTask(`access-${kind}`, async () => {
    if (!selectedProfile.value) return
    const name = accessName[kind].trim()
    if (!name) throw new Error('Minecraft 닉네임을 입력해 주세요.')
    const identity = await call<MinecraftIdentity>('lookup_minecraft_profile', { name })

    if (kind === 'ops') {
      if (!accessLists.value.ops.some((entry) => entry.uuid === identity.uuid)) {
        accessLists.value.ops.push({ uuid: identity.uuid, name: identity.name, level: 4, bypassesPlayerLimit: false })
      }
    } else if (kind === 'whitelist') {
      if (!accessLists.value.whitelist.some((entry) => entry.uuid === identity.uuid)) {
        accessLists.value.whitelist.push({ uuid: identity.uuid, name: identity.name })
      }
    } else if (!accessLists.value.bannedPlayers.some((entry) => entry.uuid === identity.uuid)) {
      accessLists.value.bannedPlayers.push({
        uuid: identity.uuid,
        name: identity.name,
        created: new Date().toISOString(),
        source: 'Minehub Server Launcher',
        expires: 'forever',
        reason: banReason.value.trim() || 'Banned by an operator.'
      })
    }

    accessName[kind] = ''
  })

  const removeAccessEntry = (kind: AccessKind, index: number) => {
    if (kind === 'ops') accessLists.value.ops.splice(index, 1)
    if (kind === 'whitelist') accessLists.value.whitelist.splice(index, 1)
    if (kind === 'bannedPlayers') accessLists.value.bannedPlayers.splice(index, 1)
  }

  const saveAccessLists = async () => runTask('save-access', async () => {
    if (!selectedProfile.value) return
    const shouldApplyNow = activeProfileRunning.value
    accessLists.value = await call<AccessLists>('save_access_lists', {
      profileId: selectedProfile.value.id,
      lists: accessPayload()
    })
    notifySuccess(
      '권한/차단 목록을 저장했습니다.',
      shouldApplyNow ? '실행 중인 서버에 명령으로 반영했습니다.' : '서버 다음 실행 시 적용됩니다.'
    )
  })

  const setWhitelistEnabled = async (enabled: boolean) => runTask('whitelist-toggle', async () => {
    if (!selectedProfile.value) return
    if (!config.value) await refreshConfig()
    if (!config.value) throw new Error('서버 설정을 불러오지 못했습니다.')

    const shouldApplyNow = activeProfileRunning.value
    config.value.properties.whiteList = enabled
    config.value = await call<ServerConfigBundle>('save_server_config', {
      profileId: selectedProfile.value.id,
      bundle: config.value
    })
    await refreshProfiles()

    if (shouldApplyNow) {
      applyStatus(await call<ServerStatus>('send_server_command', {
        command: enabled ? 'whitelist on' : 'whitelist off'
      }))
    }

    notifySuccess(
      enabled ? '화이트리스트를 켰습니다.' : '화이트리스트를 껐습니다.',
      shouldApplyNow ? '실행 중인 서버에 즉시 반영했습니다.' : '서버 다음 실행 시 적용됩니다.'
    )
  })

  const createBackup = async () => runTask('backup', async () => {
    if (!selectedProfile.value) return
    backupInfo.value = await call<BackupInfo>('create_backup', { profileId: selectedProfile.value.id })
    notifySuccess('백업을 만들었습니다.', backupInfo.value.filename)
  })

  const openPath = async (target: string) => runTask(`open-${target}`, async () => {
    if (!selectedProfile.value) return
    await call<void>('open_server_path', { profileId: selectedProfile.value.id, target })
  })

  const goToSettings = () => {
    mainTab.value = 'settings'
  }

  const fetchAppUpdate = async () => {
    const info = await call<AppUpdateInfo>('check_for_update')
    appUpdate.value = info
    appVersion.value = info.currentVersion
    if (!info.available) appUpdateDismissed.value = false
  }

  const checkForAppUpdate = async (silent = false) => {
    if (silent) {
      try {
        await fetchAppUpdate()
      } catch {
        appUpdate.value = null
      }
      return
    }
    return runTask('update-check', fetchAppUpdate)
  }

  const dismissAppUpdate = () => {
    appUpdateDismissed.value = true
  }

  const installAppUpdate = async () => runTask('update-install', async () => {
    appUpdateProgress.value = 0
    appUpdateDownloadedBytes = 0
    appUpdateProgressLabel.value = '업데이트를 다운로드하는 중입니다.'
    const installed = await call<boolean>('download_and_install_update')
    if (installed) {
      appUpdateInstalling.value = true
      appUpdateProgressLabel.value = '업데이트 설치 준비가 완료되었습니다. 앱을 다시 시작하세요.'
      notifySuccess('업데이트가 준비되었습니다.', '앱을 다시 시작하면 새 버전이 적용됩니다.')
    }
  })

  const bindRuntimeEvents = async () => {
    if (!isTauri()) return
    unlisteners.push(await listen<ServerLogEvent>('server-log', (event) => pushRuntimeLog(event.payload.line)))
    unlisteners.push(await listen<ServerStatus>('server-status', async (event) => {
      const previousStatus = status.value.status
      const previousProfileId = status.value.currentProfileId
      applyStatus(event.payload)
      if (selectedProfile.value && (previousStatus !== status.value.status || previousProfileId !== status.value.currentProfileId)) {
        await refreshConfig()
        loadNetworkDiagnostics().catch(() => { network.value = null })
      }
    }))
    unlisteners.push(await listen<UpdateProgressEvent>('updater-progress', (event) => {
      const { chunkLength, contentLength } = event.payload
      appUpdateDownloadedBytes += chunkLength
      if (contentLength && contentLength > 0) {
        const received = Math.min(contentLength, appUpdateDownloadedBytes)
        appUpdateProgress.value = Math.min(1, received / contentLength)
        appUpdateProgressLabel.value = `업데이트를 다운로드하는 중입니다. (${Math.round(appUpdateProgress.value * 100)}%)`
      } else {
        appUpdateProgressLabel.value = `업데이트를 다운로드하는 중입니다. (${appUpdateDownloadedBytes} 바이트)`
      }
    }))
    unlisteners.push(await listen<boolean>('updater-installing', (event) => {
      appUpdateInstalling.value = event.payload
      if (event.payload) appUpdateProgressLabel.value = '업데이트를 설치하는 중입니다.'
    }))
  }

  watch(() => newProfile.value.kind, (kind) => loadVersions(kind, true))
  watch(selectedProfileId, refreshProfileData)
  watch(mainTab, async (tab) => {
    if (tab === 'settings') await refreshConfig()
    if (tab === 'ops') await Promise.all([refreshAccessLists(), refreshConfig()])
  })
  watch(() => profileDraft.value?.kind, async (kind, oldKind) => {
    if (!profileDraft.value || !kind || kind === oldKind) return
    await loadVersions(kind, false, true)
  })

  onMounted(async () => {
    try {
      await bindRuntimeEvents()
      await Promise.all([
        refreshProfiles(),
        loadVersions(newProfile.value.kind, true),
        call<JavaRuntime[]>('scan_java_versions').then((result) => { javaVersions.value = result }),
        call<string>('current_app_version').then((result) => { appVersion.value = result }),
        refreshStatus(),
        refreshSystemMetrics()
      ])
      await refreshProfileData()
      pollTimer = setInterval(refreshStatus, 10000)
      networkTimer = setInterval(() => {
        refreshNetworkDiagnosticsInBackground().catch(() => {})
      }, 30000)
      metricsTimer = setInterval(() => {
        refreshSystemMetrics().catch(() => {})
      }, 3000)
      checkForAppUpdate(true)
    } catch (error) {
      setError(error)
    }
  })

  onBeforeUnmount(() => {
    if (pollTimer) clearInterval(pollTimer)
    if (networkTimer) clearInterval(networkTimer)
    if (metricsTimer) clearInterval(metricsTimer)
    for (const unlisten of unlisteners) unlisten()
  })

  return reactive({
    colorMode,
    profiles,
    profilesLoaded,
    selectedProfileId,
    profileDraft,
    versions,
    createVersions,
    javaVersions,
    plan,
    status,
    eula,
    eulaDialogOpen,
    eulaAgreementChecked,
    network,
    networkAutoRefreshing,
    metrics,
    metricHistory,
    config,
    accessLists,
    plugins,
    modrinthProjects,
    pluginUpdateSummary,
    mainTab,
    appSettingsOpen,
    newProfileOpen,
    profileDeleteOpen,
    deleteProfileFiles,
    versionLoading,
    createVersionLoading,
    logQuery,
    commandText,
    pluginQuery,
    loading,
    backupInfo,
    appVersion,
    playerActionOpen,
    selectedPlayerName,
    playerAction,
    playerActionReason,
    playerActionGamemode,
    accessName,
    banReason,
    accessRawOpen,
    newProfile,
    appUpdate,
    appUpdateDismissed,
    appUpdateProgress,
    appUpdateProgressLabel,
    appUpdateInstalling,
    selectedProfile,
    needsOnboarding,
    activeProfileRunning,
    anyServerRunning,
    canUsePlugins,
    pluginUpdateCount,
    hasPluginUpdates,
    filteredLogs,
    versionOptions,
    createVersionOptions,
    profileKindLabel,
    selectedJava,
    selectedJavaPath,
    whitelistEnabled,
    serverKinds,
    tabs,
    themeOptions,
    playerActionOptions,
    gamemodeOptions,
    statusLabel,
    statusColor,
    profileRuntimeStatus,
    notifySuccess,
    notifyError,
    notifyWarning,
    refreshProfileData,
    refreshConfig,
    refreshNetworkDiagnostics,
    openUpnpPort,
    refreshSystemMetrics,
    refreshAccessLists,
    refreshPlugins,
    chooseDirectory,
    resetNewProfile,
    openNewProfileModal,
    closeNewProfileModal,
    openDeleteProfileDialog,
    closeDeleteProfileDialog,
    closeEulaDialog,
    createProfile,
    saveProfile,
    deleteProfile,
    saveConfig,
    toggleServer,
    acceptEulaAndStart,
    refreshEulaStatus,
    sendCommand,
    refreshPlayers,
    openPlayerAction,
    closePlayerAction,
    runPlayerAction,
    searchPlugins,
    checkPluginUpdates,
    installPlugin,
    setPluginEnabled,
    installPluginUpdate,
    addAccessEntry,
    removeAccessEntry,
    saveAccessLists,
    setWhitelistEnabled,
    createBackup,
    openPath,
    goToSettings,
    checkForAppUpdate,
    dismissAppUpdate,
    installAppUpdate
  })
}

export type LauncherState = ReturnType<typeof useLauncherState>

const launcherKey: InjectionKey<LauncherState> = Symbol('launcher')

export const provideLauncher = (launcher: LauncherState) => provide(launcherKey, launcher)

export const useLauncher = () => {
  const launcher = inject(launcherKey)
  if (!launcher) throw new Error('Launcher state is not provided.')
  return launcher
}
