<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useLauncher } from '~/composables/useLauncherState'
import { parseConsoleLine } from '~/utils/consoleLog'

const launcher = useLauncher()
const logViewport = ref<HTMLElement | null>(null)
const chartPoints = (values: number[]) => {
  if (!values.length) return ''
  const width = 100
  const height = 48
  const step = values.length > 1 ? width / (values.length - 1) : 0
  return values
    .map((value, index) => `${index * step},${height - (Math.min(100, Math.max(0, value)) / 100) * height}`)
    .join(' ')
}
const cpuPoints = computed(() => chartPoints(launcher.metricHistory.map((point) => point.cpuUsage)))
const memoryPoints = computed(() => chartPoints(launcher.metricHistory.map((point) => point.memoryUsage)))
const latestMetricLabel = computed(() => launcher.metricHistory.at(-1)?.label || '수집 중')
const networkCheckedLabel = computed(() => launcher.network?.checkedAt
  ? new Date(launcher.network.checkedAt * 1000).toLocaleTimeString('ko-KR', {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      hour12: false
    })
  : '확인 전')
const memoryText = computed(() => {
  const metrics = launcher.metrics
  return metrics ? `${metrics.memoryUsedMb.toLocaleString()} / ${metrics.memoryTotalMb.toLocaleString()} MB` : '-'
})
const percentText = (value?: number) => `${Math.round(value || 0)}%`
const reachLabel = (value?: boolean | null) => value === true ? '가능' : value === false ? '불가' : '확인 전'
const reachColor = (value?: boolean | null) => value === true ? 'success' : value === false ? 'error' : 'neutral'

const scrollToBottom = async () => {
  await nextTick()
  if (logViewport.value) logViewport.value.scrollTop = logViewport.value.scrollHeight
}

watch(
  () => [launcher.filteredLogs.length, launcher.selectedProfileId, launcher.status.currentProfileId, launcher.logQuery],
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
        <div class="mb-3 flex items-center justify-between gap-2">
          <p class="metric-label">접속 정보</p>
          <UButton
            size="sm"
            color="neutral"
            variant="subtle"
            :icon="launcher.networkAutoRefreshing ? 'i-lucide-loader-circle' : 'i-lucide-refresh-cw'"
            :class="{ 'animate-spin': launcher.networkAutoRefreshing }"
            :loading="launcher.loading === 'network-diagnostics'"
            :disabled="!launcher.selectedProfile"
            @click="launcher.refreshNetworkDiagnostics"
          />
        </div>
        <p class="metric-value">{{ launcher.network?.publicEndpoint || launcher.network?.lanEndpoint || '-' }}</p>
        <div class="mt-3 grid gap-2 text-sm">
          <div class="flex items-center justify-between gap-3">
            <span class="text-muted">포트</span>
            <span class="font-medium text-highlighted">{{ launcher.network?.port || launcher.config?.properties.serverPort || '-' }}</span>
          </div>
          <div class="flex items-center justify-between gap-3">
            <span class="text-muted">LAN 주소</span>
            <span class="truncate font-medium text-highlighted">{{ launcher.network?.lanEndpoint || '-' }}</span>
          </div>
          <div class="flex items-center justify-between gap-3">
            <span class="text-muted">공인 주소</span>
            <span class="truncate font-medium text-highlighted">{{ launcher.network?.publicEndpoint || '-' }}</span>
          </div>
          <div class="flex items-center justify-between gap-3">
            <span class="text-muted">로컬 접속</span>
            <UBadge :color="reachColor(launcher.network?.localReachable)" variant="soft">{{ reachLabel(launcher.network?.localReachable) }}</UBadge>
          </div>
          <div class="flex items-center justify-between gap-3">
            <span class="text-muted">외부 접속</span>
            <div class="flex items-center gap-2">
              <UTooltip v-if="launcher.network?.externalReachable === false" text="UPnP로 포트 열기">
                <UButton size="xs" color="neutral" variant="subtle" icon="i-lucide-router"
                  :loading="launcher.loading === 'upnp'" :disabled="!launcher.selectedProfile"
                  @click="launcher.openUpnpPort" />
              </UTooltip>
              <UBadge :color="reachColor(launcher.network?.externalReachable)" variant="soft">{{ reachLabel(launcher.network?.externalReachable) }}</UBadge>
            </div>
          </div>
        </div>
        <p v-if="launcher.network?.note" class="mt-3 text-xs leading-5 text-muted">{{ launcher.network.note }}</p>
      </div>

      <div class="panel p-4">
        <div class="mb-3 flex items-center justify-between gap-2">
          <p class="metric-label">시스템 리소스</p>
          <span class="text-xs text-muted">{{ latestMetricLabel }}</span>
        </div>
        <div class="grid grid-cols-2 gap-3">
          <div>
            <p class="text-xs text-muted">CPU</p>
            <p class="text-lg font-semibold text-highlighted">{{ percentText(launcher.metrics?.cpuUsage) }}</p>
          </div>
          <div>
            <p class="text-xs text-muted">메모리</p>
            <p class="text-lg font-semibold text-highlighted">{{ percentText(launcher.metrics?.memoryUsage) }}</p>
            <p class="truncate text-xs text-muted">{{ memoryText }}</p>
          </div>
        </div>
        <svg class="mt-4 h-28 w-full overflow-visible" viewBox="0 0 100 48" preserveAspectRatio="none" role="img" aria-label="CPU 및 메모리 사용량 그래프">
          <line x1="0" y1="48" x2="100" y2="48" stroke="var(--ui-border)" stroke-width="1" />
          <line x1="0" y1="24" x2="100" y2="24" stroke="var(--ui-border)" stroke-width="0.5" opacity="0.7" />
          <polyline v-if="memoryPoints" :points="memoryPoints" fill="none" stroke="#f59e0b" stroke-width="2" vector-effect="non-scaling-stroke" />
          <polyline v-if="cpuPoints" :points="cpuPoints" fill="none" stroke="var(--ui-primary)" stroke-width="2" vector-effect="non-scaling-stroke" />
        </svg>
        <div class="mt-2 flex items-center gap-4 text-xs text-muted">
          <span class="inline-flex items-center gap-1"><span class="size-2 rounded-sm bg-primary" />CPU</span>
          <span class="inline-flex items-center gap-1"><span class="size-2 rounded-sm bg-amber-500" />메모리</span>
        </div>
      </div>

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
