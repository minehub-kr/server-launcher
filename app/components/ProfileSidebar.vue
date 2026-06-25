<script setup lang="ts">
import { useLauncher } from '~/composables/useLauncherState'

const launcher = useLauncher()
</script>

<template>
  <aside class="sidebar flex h-full min-h-0 flex-col border-r border-default">
    <div class="border-b border-default p-4">
      <div class="flex items-center justify-between gap-3">
        <div>
          <h1 class="text-xl font-semibold tracking-normal text-highlighted">Minehub Server Launcher</h1>
          <p class="mt-1 text-xs text-muted">멀티 프로필 서버 운영 도구</p>
        </div>
        <UBadge :color="launcher.statusColor(launcher.status.status)" variant="soft">
          {{ launcher.statusLabel(launcher.status.status) }}
        </UBadge>
      </div>
    </div>

    <section class="min-h-0 flex-1 overflow-auto p-4">
      <div class="mb-3 flex items-center justify-between">
        <h2 class="text-sm font-semibold text-highlighted">프로필</h2>
        <UBadge color="neutral" variant="outline">{{ launcher.profiles.length }}</UBadge>
      </div>

      <div class="space-y-2">
        <button
          v-for="profile in launcher.profiles"
          :key="profile.id"
          class="profile-row w-full"
          :class="{ active: profile.id === launcher.selectedProfileId }"
          @click="launcher.selectedProfileId = profile.id"
        >
          <span class="min-w-0">
            <span class="block truncate text-sm font-medium">{{ profile.name }}</span>
            <span class="mt-1 block truncate text-xs text-muted">
              {{ profile.kind }} {{ profile.minecraftVersion }} · {{ profile.settings.serverPort }}
            </span>
          </span>
          <UBadge :color="launcher.statusColor(launcher.profileRuntimeStatus(profile))" variant="soft">
            {{ launcher.statusLabel(launcher.profileRuntimeStatus(profile)) }}
          </UBadge>
        </button>

        <p v-if="!launcher.profiles.length" class="empty-note">아직 서버 프로필이 없습니다.</p>
      </div>
    </section>

    <section class="border-t border-default p-4">
      <UButton block icon="i-lucide-plus" :loading="launcher.loading === 'new-profile-open'" @click="launcher.openNewProfileModal">
        프로필 추가
      </UButton>
    </section>
  </aside>
</template>
