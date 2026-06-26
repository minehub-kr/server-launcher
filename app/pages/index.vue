<script setup lang="ts">
import { provideLauncher, useLauncherState } from '~/composables/useLauncherState'

const launcher = useLauncherState()
provideLauncher(launcher)
</script>

<template>
  <main class="app-shell h-[100dvh] overflow-hidden">
    <section v-if="!launcher.profilesLoaded" class="grid h-full place-items-center">
      <div class="text-center">
        <UIcon name="i-lucide-loader-circle" class="mx-auto size-8 animate-spin text-muted" />
        <p class="mt-3 text-sm text-muted">프로필을 확인하는 중입니다.</p>
      </div>
    </section>

    <OnboardingPage v-else-if="launcher.needsOnboarding" />

    <div v-else class="app-frame mx-auto grid h-full max-w-[1540px] grid-cols-[340px_minmax(0,1fr)] overflow-hidden">
      <ProfileSidebar />

      <section class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
        <AppHeader />

        <div class="px-5 pt-4">
          <UAlert
            v-if="launcher.status.status === 'crashed'"
            color="error"
            variant="soft"
            icon="i-lucide-triangle-alert"
            title="Crash 또는 비정상 종료가 감지되었습니다."
            :description="launcher.status.exitMessage || undefined"
          />
        </div>

        <TabNav />

        <div class="min-h-0 flex-1 overflow-auto p-5">
          <section v-if="!launcher.selectedProfile" class="empty-panel">
            <UIcon name="i-lucide-server" class="size-10 text-muted" />
            <p class="mt-3 text-sm text-muted">왼쪽에서 서버 프로필을 만들거나 선택하세요.</p>
          </section>

          <ConsoleTab v-else-if="launcher.mainTab === 'console'" />
          <SettingsTab v-else-if="launcher.mainTab === 'settings'" />
          <PluginsTab v-else-if="launcher.mainTab === 'plugins'" />
          <PlayersTab v-else-if="launcher.mainTab === 'players'" />
          <AccessTab v-else-if="launcher.mainTab === 'ops'" />
          <BackupTab v-else-if="launcher.mainTab === 'backup'" />
        </div>
      </section>

      <NewProfileModal />
      <PlayerActionDialog />
    </div>
  </main>
</template>
