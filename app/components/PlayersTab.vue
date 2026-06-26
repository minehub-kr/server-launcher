<script setup lang="ts">
import { useLauncher } from '~/composables/useLauncherState'

const launcher = useLauncher()
const playerHeadUrl = (player: string) => `https://mc-heads.net/avatar/${encodeURIComponent(player)}/40`
const playerInitials = (player: string) => player.slice(0, 2).toUpperCase()
const hideBrokenHead = (event: Event) => {
  if (event.target instanceof HTMLImageElement) event.target.style.display = 'none'
}
</script>

<template>
  <section class="panel p-4">
    <div class="mb-3 flex flex-wrap items-center justify-between gap-2">
      <div class="flex items-center gap-2">
        <h3 class="text-sm font-semibold text-highlighted">접속 플레이어</h3>
        <UBadge color="neutral" variant="outline">{{ launcher.status.players.length }}</UBadge>
      </div>
      <UButton
        size="sm"
        color="neutral"
        variant="subtle"
        icon="i-lucide-refresh-cw"
        :disabled="!launcher.activeProfileRunning"
        :loading="launcher.loading === 'players-refresh'"
        @click="launcher.refreshPlayers"
      >
        목록 갱신
      </UButton>
    </div>

    <div class="grid gap-2 md:grid-cols-2 xl:grid-cols-3">
      <div v-for="player in launcher.status.players" :key="player" class="player-row">
        <div class="flex min-w-0 items-center gap-3">
          <span class="avatar player-head shrink-0">
            <span class="player-initials">{{ playerInitials(player) }}</span>
            <img
              :src="playerHeadUrl(player)"
              :alt="`${player} 머리`"
              loading="lazy"
              decoding="async"
              @error="hideBrokenHead"
            >
          </span>
          <span class="truncate text-sm font-medium text-highlighted">{{ player }}</span>
        </div>
        <UButton
          size="sm"
          color="neutral"
          variant="subtle"
          icon="i-lucide-terminal"
          class="shrink-0"
          :disabled="!launcher.activeProfileRunning"
          @click="launcher.openPlayerAction(player)"
        >
          관리
        </UButton>
      </div>
    </div>
    <p v-if="!launcher.status.players.length" class="empty-note">
      {{ launcher.activeProfileRunning ? '접속 중인 플레이어가 없습니다.' : '서버가 실행 중이 아닙니다.' }}
    </p>
  </section>
</template>
