<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import { useLauncher } from '~/composables/useLauncherState'
import { parseConsoleLine } from '~/utils/consoleLog'

const launcher = useLauncher()
const logViewport = ref<HTMLElement | null>(null)

const scrollToBottom = async () => {
  await nextTick()
  if (logViewport.value) logViewport.value.scrollTop = logViewport.value.scrollHeight
}

watch(
  () => [launcher.filteredLogs.length, launcher.status.currentProfileId, launcher.logQuery],
  scrollToBottom,
  { flush: 'post', immediate: true }
)
</script>

<template>
  <section class="grid h-full min-h-0 min-w-0 gap-4 lg:grid-cols-[minmax(0,1fr)_340px]">
    <div class="panel flex min-h-0 min-w-0 flex-col">
      <div class="flex flex-wrap items-center justify-between gap-2 border-b border-default p-3">
        <div class="flex items-center gap-2">
          <h3 class="text-sm font-semibold text-highlighted">콘솔</h3>
          <UBadge color="neutral" variant="outline">{{ launcher.filteredLogs.length }}</UBadge>
        </div>
        <div class="control-row w-full sm:w-auto">
          <UInput v-model="launcher.logQuery" class="w-full" size="sm" icon="i-lucide-search" placeholder="로그 검색" />
          <UButton size="sm" color="neutral" variant="subtle" icon="i-lucide-folder-open" @click="launcher.openPath('logs')">로그</UButton>
        </div>
      </div>
      <div ref="logViewport" class="min-h-0 min-w-0 flex-1 overflow-y-auto overflow-x-hidden p-4 font-mono text-xs leading-6 text-toned">
        <p v-for="(line, index) in launcher.filteredLogs" :key="`${index}-${line}`" class="log-line whitespace-pre-wrap">
          <span
            v-for="(segment, segmentIndex) in parseConsoleLine(line)"
            :key="segmentIndex"
            class="log-segment"
            :style="segment.style"
          >{{ segment.text }}</span>
        </p>
      </div>
      <form class="control-row border-t border-default p-3" @submit.prevent="launcher.sendCommand()">
        <UInput v-model="launcher.commandText" class="w-full" icon="i-lucide-terminal" placeholder="say hello, list, stop..." :disabled="!launcher.activeProfileRunning" />
        <UButton type="submit" icon="i-lucide-send" :disabled="!launcher.activeProfileRunning" :loading="launcher.loading === 'command'">전송</UButton>
      </form>
    </div>

    <aside class="min-h-0 space-y-4 overflow-auto">
      <div class="panel p-4">
        <p class="metric-label">서버</p>
        <p class="metric-value">{{ launcher.selectedProfile?.kind }} {{ launcher.selectedProfile?.minecraftVersion }}</p>
        <p class="mt-1 text-sm text-muted">{{ launcher.plan?.serverNote || '상태 확인 중' }}</p>
      </div>
      <div class="panel p-4">
        <p class="metric-label">요구 Java</p>
        <p class="metric-value">{{ launcher.plan?.requiredJava ? `Java ${launcher.plan.requiredJava}+` : '-' }}</p>
        <p class="mt-1 text-sm text-muted">{{ launcher.plan?.javaComponent || '메타데이터 확인 중' }}</p>
      </div>
      <div class="panel p-4">
        <p class="metric-label">선택된 Java</p>
        <p class="metric-value">{{ launcher.selectedJava ? `Java ${launcher.selectedJava.major}` : launcher.profileDraft?.javaPath ? '직접 지정' : '자동 선택' }}</p>
        <p class="mt-1 break-all text-sm text-muted">{{ launcher.selectedJavaPath }}</p>
      </div>
      <div class="panel p-4">
        <p class="metric-label">실행 중 Java</p>
        <p class="metric-value">{{ launcher.status.java ? `Java ${launcher.status.java.major}` : '실행 안 됨' }}</p>
        <p class="mt-1 break-all text-sm text-muted">{{ launcher.status.java?.path || launcher.plan?.java?.path || '탐색된 Java가 없습니다.' }}</p>
        <UButton v-if="launcher.plan && !launcher.plan.java" class="mt-3" size="sm" color="warning" variant="soft" icon="i-lucide-settings" @click="launcher.goToSettings">Java 설정</UButton>
      </div>
      <div class="panel p-4">
        <p class="metric-label">플레이어</p>
        <p class="metric-value">{{ launcher.status.players.length }}</p>
        <UButton
          class="mt-3"
          size="sm"
          color="neutral"
          variant="subtle"
          icon="i-lucide-users"
          :disabled="!launcher.activeProfileRunning"
          :loading="launcher.loading === 'players-refresh'"
          @click="launcher.refreshPlayers"
        >
          목록 갱신
        </UButton>
      </div>
    </aside>
  </section>
</template>
