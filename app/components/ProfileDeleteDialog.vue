<script setup lang="ts">
import { useLauncher } from '~/composables/useLauncherState'

const launcher = useLauncher()
</script>

<template>
  <UModal
    v-model:open="launcher.profileDeleteOpen"
    title="프로필 삭제"
    description="앱에서 이 프로필을 제거합니다."
    :dismissible="launcher.loading !== 'delete-profile'"
    :close="launcher.loading !== 'delete-profile'"
  >
    <template #body>
      <div v-if="launcher.selectedProfile" class="space-y-4">
        <UAlert
          v-if="launcher.activeProfileRunning"
          color="error"
          variant="soft"
          icon="i-lucide-circle-alert"
          title="실행 중인 프로필은 삭제할 수 없습니다."
          description="서버를 중지한 뒤 다시 시도하세요."
        />

        <div class="rounded-md border border-default bg-muted/30 p-3">
          <p class="text-sm font-medium text-highlighted">{{ launcher.selectedProfile.name }}</p>
          <p class="mt-1 break-all text-xs text-muted">{{ launcher.selectedProfile.serverDir }}</p>
        </div>

        <UCheckbox
          v-model="launcher.deleteProfileFiles"
          class="check-row"
          label="서버 폴더도 삭제"
          :disabled="launcher.loading === 'delete-profile'"
        />

        <p class="text-xs leading-5 text-muted">
          체크하지 않으면 앱의 프로필 설정만 삭제하고 서버 파일과 월드는 그대로 둡니다.
        </p>
      </div>
    </template>

    <template #footer>
      <div class="flex w-full justify-end gap-2">
        <UButton color="neutral" variant="subtle" :disabled="launcher.loading === 'delete-profile'" @click="launcher.closeDeleteProfileDialog">
          취소
        </UButton>
        <UButton
          color="error"
          icon="i-lucide-trash"
          :loading="launcher.loading === 'delete-profile'"
          :disabled="!launcher.selectedProfile || launcher.activeProfileRunning"
          @click="launcher.deleteProfile"
        >
          삭제
        </UButton>
      </div>
    </template>
  </UModal>
</template>
