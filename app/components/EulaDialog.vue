<script setup lang="ts">
import { useLauncher } from '~/composables/useLauncherState'

const launcher = useLauncher()
</script>

<template>
  <UModal
    v-model:open="launcher.eulaDialogOpen"
    title="Minecraft EULA 동의 필요"
    description="서버를 실행하기 전에 사용자의 동의가 필요합니다."
    :dismissible="launcher.loading !== 'server'"
    :close="launcher.loading !== 'server'"
  >
    <template #body>
      <div class="space-y-4">
        <div class="rounded-md border border-default bg-muted/30 p-3">
          <p class="text-sm font-medium text-highlighted">{{ launcher.selectedProfile?.name || '선택된 프로필' }}</p>
          <p class="mt-1 break-all text-xs text-muted">{{ launcher.eula?.path || 'eula.txt' }}</p>
        </div>

        <UButton
          :to="launcher.eula?.url || 'https://aka.ms/MinecraftEULA'"
          target="_blank"
          color="neutral"
          variant="subtle"
          icon="i-lucide-external-link"
        >
          Minecraft EULA 열기
        </UButton>

        <UCheckbox
          v-model="launcher.eulaAgreementChecked"
          label="Minecraft EULA 내용을 확인했고 이에 동의합니다."
          :disabled="launcher.loading === 'server'"
        />
      </div>
    </template>

    <template #footer>
      <div class="flex w-full justify-end gap-2">
        <UButton color="neutral" variant="subtle" :disabled="launcher.loading === 'server'" @click="launcher.closeEulaDialog">
          취소
        </UButton>
        <UButton
          icon="i-lucide-play"
          :loading="launcher.loading === 'server'"
          :disabled="!launcher.eulaAgreementChecked"
          @click="launcher.acceptEulaAndStart"
        >
          동의하고 실행
        </UButton>
      </div>
    </template>
  </UModal>
</template>
