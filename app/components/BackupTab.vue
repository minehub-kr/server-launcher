<script setup lang="ts">
import { useLauncher } from '~/composables/useLauncherState'

const launcher = useLauncher()
</script>

<template>
  <section class="grid gap-4 lg:grid-cols-2">
    <div class="panel p-4">
      <h3 class="mb-2 text-sm font-semibold text-highlighted">수동 백업</h3>
      <p class="mb-4 text-sm text-muted">월드, 설정 파일, 플러그인 폴더를 zip 파일로 묶어 서버 폴더의 backups에 저장합니다.</p>
      <div class="flex gap-2">
        <UButton icon="i-lucide-archive" :loading="launcher.loading === 'backup'" @click="launcher.createBackup">백업 생성</UButton>
        <UButton color="neutral" variant="subtle" icon="i-lucide-folder-open" @click="launcher.openPath('backups')">백업 폴더</UButton>
      </div>
    </div>
    <div class="panel p-4">
      <h3 class="mb-2 text-sm font-semibold text-highlighted">최근 백업</h3>
      <p v-if="launcher.backupInfo" class="text-sm text-toned">{{ launcher.backupInfo.filename }} · {{ (launcher.backupInfo.size / 1024 / 1024).toFixed(2) }} MB</p>
      <p v-else class="empty-note">이번 세션에서 생성한 백업이 없습니다.</p>
    </div>
  </section>
</template>
