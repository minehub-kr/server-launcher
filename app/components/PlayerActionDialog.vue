<script setup lang="ts">
import { useLauncher } from '~/composables/useLauncherState'

const launcher = useLauncher()
</script>

<template>
  <UModal
    v-model:open="launcher.playerActionOpen"
    title="플레이어 관리"
    :description="launcher.selectedPlayerName"
    :dismissible="launcher.loading !== 'player-action'"
    :close="launcher.loading !== 'player-action'"
  >
    <template #body>
      <form id="player-action-form" class="space-y-4" @submit.prevent="launcher.runPlayerAction">
        <UFormField label="대상 플레이어" class="settings-field">
          <UInput v-model="launcher.selectedPlayerName" class="w-full" icon="i-lucide-user" disabled />
        </UFormField>

        <UFormField label="명령" class="settings-field">
          <USelect v-model="launcher.playerAction" :items="launcher.playerActionOptions" class="w-full" />
        </UFormField>

        <UFormField
          v-if="launcher.playerAction === 'kick' || launcher.playerAction === 'ban'"
          label="사유"
          class="settings-field"
        >
          <UInput
            v-model="launcher.playerActionReason"
            class="w-full"
            icon="i-lucide-message-square"
            :placeholder="launcher.playerAction === 'kick' ? 'Kicked by an operator.' : 'Banned by an operator.'"
          />
        </UFormField>

        <UFormField v-if="launcher.playerAction === 'gamemode'" label="게임 모드" class="settings-field">
          <USelect v-model="launcher.playerActionGamemode" :items="launcher.gamemodeOptions" class="w-full" />
        </UFormField>
      </form>
    </template>

    <template #footer>
      <div class="flex w-full justify-end gap-2">
        <UButton color="neutral" variant="subtle" :disabled="launcher.loading === 'player-action'" @click="launcher.closePlayerAction">
          취소
        </UButton>
        <UButton
          type="submit"
          form="player-action-form"
          icon="i-lucide-send"
          :loading="launcher.loading === 'player-action'"
          :disabled="!launcher.activeProfileRunning"
        >
          실행
        </UButton>
      </div>
    </template>
  </UModal>
</template>
