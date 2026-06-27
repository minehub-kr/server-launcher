<script setup lang="ts">
import { computed } from 'vue'
import { useLauncher } from '~/composables/useLauncherState'

const launcher = useLauncher()

const visible = computed(() =>
  !!launcher.appUpdate &&
  launcher.appUpdate.available &&
  !!launcher.appUpdate.version &&
  !launcher.appUpdateDismissed
)

const updateNotes = computed(() => launcher.appUpdate?.notes?.trim() || null)
const showNotes = computed(() => visible.value && !!updateNotes.value && !launcher.appUpdateInstalling)
const installing = computed(() => launcher.loading === 'update-install' || launcher.appUpdateInstalling)
const progressPercent = computed(() => Math.round((launcher.appUpdateProgress || 0) * 100))

const openReleaseNotes = () => {
  if (typeof window !== 'undefined') {
    window.open('https://github.com/minehub-kr/server-launcher/releases/latest', '_blank', 'noopener')
  }
}
</script>

<template>
  <div v-if="visible">
    <UAlert
      color="primary"
      variant="soft"
      icon="i-lucide-download"
      :title="`새 버전 ${launcher.appUpdate?.version}을(를) 사용할 수 있습니다.`"
      :description="installing ? launcher.appUpdateProgressLabel : `현재 버전: ${launcher.appUpdate?.currentVersion}`"
    >
      <template #actions>
        <UButton
          v-if="!installing"
          size="sm"
          color="primary"
          icon="i-lucide-download"
          :loading="launcher.loading === 'update-install'"
          @click="launcher.installAppUpdate"
        >
          업데이트 설치
        </UButton>
        <UButton
          v-if="!installing"
          size="sm"
          color="neutral"
          variant="subtle"
          icon="i-lucide-external-link"
          @click="openReleaseNotes"
        >
          릴리즈 노트
        </UButton>
        <UButton
          v-if="!installing"
          size="sm"
          color="neutral"
          variant="ghost"
          icon="i-lucide-x"
          @click="launcher.dismissAppUpdate"
        >
          나중에
        </UButton>
      </template>
    </UAlert>
    <UAlert
      v-if="installing"
      class="mt-2"
      color="info"
      variant="subtle"
      icon="i-lucide-loader-circle"
      :title="launcher.appUpdateProgressLabel"
      :description="`${progressPercent}% 다운로드됨`"
    />
    <div v-if="showNotes" class="update-notes mt-2">
      <p class="update-notes-title">릴리즈 노트</p>
      <pre class="update-notes-body">{{ updateNotes }}</pre>
    </div>
  </div>
</template>

<style scoped>
.update-notes {
  border: 1px solid var(--ui-border);
  border-radius: 12px;
  padding: 12px 16px;
  background: var(--ui-bg-elevated);
}
.update-notes-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--ui-text-highlighted);
  margin-bottom: 6px;
}
.update-notes-body {
  margin: 0;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-word;
  color: var(--ui-text-muted);
  max-height: 160px;
  overflow: auto;
}
</style>
