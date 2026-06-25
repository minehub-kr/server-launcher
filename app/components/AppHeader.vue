<script setup lang="ts">
import { useLauncher } from '~/composables/useLauncherState'

const launcher = useLauncher()
</script>

<template>
  <header class="flex flex-wrap items-center justify-between gap-3 border-b border-default px-5 py-4">
    <div class="min-w-0">
      <div class="flex flex-wrap items-center gap-2">
        <h2 class="truncate text-lg font-semibold text-highlighted">{{ launcher.selectedProfile?.name || '프로필 선택 필요' }}</h2>
        <UBadge v-if="launcher.selectedProfile" color="neutral" variant="outline">{{ launcher.profileKindLabel }}</UBadge>
        <UBadge v-if="launcher.selectedProfile" :color="launcher.statusColor(launcher.profileRuntimeStatus(launcher.selectedProfile))" variant="soft">
          {{ launcher.statusLabel(launcher.profileRuntimeStatus(launcher.selectedProfile)) }}
        </UBadge>
        <UBadge v-if="launcher.plan && !launcher.plan.serverAvailable" color="error" variant="soft">지원 안 됨</UBadge>
        <UBadge v-if="launcher.plan && !launcher.plan.java" color="warning" variant="soft">Java 필요</UBadge>
        <UBadge v-if="launcher.config?.restartRequired" color="warning" variant="soft">재시작 필요</UBadge>
      </div>
      <p class="mt-1 truncate text-sm text-muted">{{ launcher.selectedProfile?.serverDir || launcher.status.dataDir }}</p>
    </div>

    <div class="relative flex items-center gap-2">
      <UButton
        color="neutral"
        variant="subtle"
        icon="i-lucide-folder-open"
        :disabled="!launcher.selectedProfile"
        @click="launcher.openPath('server')"
      >
        서버 폴더
      </UButton>
      <UButton color="neutral" variant="subtle" icon="i-lucide-refresh-cw" @click="launcher.refreshProfileData">새로고침</UButton>
      <UButton
        :color="launcher.activeProfileRunning ? 'error' : 'primary'"
        icon="i-lucide-power"
        :loading="launcher.loading === 'server'"
        :disabled="!launcher.selectedProfile || (launcher.anyServerRunning && !launcher.activeProfileRunning) || !!(launcher.plan && !launcher.plan.serverAvailable)"
        @click="launcher.toggleServer"
      >
        {{ launcher.activeProfileRunning ? '서버 중지' : '서버 실행' }}
      </UButton>
      <UButton color="neutral" variant="subtle" icon="i-lucide-settings" @click="launcher.appSettingsOpen = !launcher.appSettingsOpen" />
      <div v-if="launcher.appSettingsOpen" class="settings-popover">
        <div class="mb-3">
          <p class="text-sm font-semibold text-highlighted">앱 설정</p>
          <p class="mt-1 text-xs text-muted">테마는 앱 전체에 적용됩니다.</p>
        </div>
        <div class="grid gap-2">
          <UButton
            v-for="mode in launcher.themeOptions"
            :key="mode.value"
            :icon="mode.icon"
            :color="launcher.colorMode.preference === mode.value ? 'primary' : 'neutral'"
            :variant="launcher.colorMode.preference === mode.value ? 'solid' : 'subtle'"
            block
            @click="launcher.colorMode.preference = mode.value"
          >
            {{ mode.label }}
          </UButton>
        </div>
      </div>
    </div>
  </header>
</template>
