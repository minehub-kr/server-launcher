<script setup lang="ts">
import { useLauncher } from '~/composables/useLauncherState'

const launcher = useLauncher()
</script>

<template>
  <UModal
    v-model:open="launcher.newProfileOpen"
    title="새 프로필"
    description="서버 폴더와 Minecraft 버전을 선택해 새 서버 프로필을 만듭니다."
    :dismissible="launcher.loading !== 'create-profile'"
    :close="launcher.loading !== 'create-profile'"
  >
    <template #body>
      <form id="new-profile-form" class="space-y-4" @submit.prevent="launcher.createProfile">
        <UFormField label="프로필 이름" class="settings-field">
          <UInput v-model="launcher.newProfile.name" class="w-full" icon="i-lucide-pencil" placeholder="예: Survival Paper" />
        </UFormField>

        <UFormField label="구현체" class="settings-field">
          <USelect
            v-model="launcher.newProfile.kind"
            :items="launcher.serverKinds.map(({ label, value }) => ({ label, value }))"
            class="w-full"
          />
        </UFormField>

        <UFormField label="Minecraft 버전" class="settings-field">
          <USelect
            v-model="launcher.newProfile.minecraftVersion"
            :items="launcher.createVersionOptions"
            class="w-full"
            :disabled="launcher.createVersionLoading"
          />
          <p v-if="launcher.createVersionLoading" class="mt-1 text-xs text-muted">버전 목록을 불러오는 중입니다.</p>
        </UFormField>

        <UFormField label="서버 폴더" class="settings-field">
          <div class="control-row">
            <UInput v-model="launcher.newProfile.serverDir" class="w-full" icon="i-lucide-folder" placeholder="서버 폴더" />
            <UTooltip text="폴더 선택">
              <UButton icon="i-lucide-folder-open" color="neutral" variant="subtle" @click="launcher.chooseDirectory('create')" />
            </UTooltip>
          </div>
        </UFormField>

        <UFormField label="메모리" class="settings-field">
          <UInput
            v-model.number="launcher.newProfile.memoryMb"
            class="w-full"
            type="number"
            min="512"
            step="512"
            icon="i-lucide-memory-stick"
          />
        </UFormField>
      </form>
    </template>

    <template #footer>
      <div class="flex w-full justify-end gap-2">
        <UButton color="neutral" variant="subtle" :disabled="launcher.loading === 'create-profile'" @click="launcher.closeNewProfileModal">
          취소
        </UButton>
        <UButton
          type="submit"
          form="new-profile-form"
          icon="i-lucide-plus"
          :loading="launcher.loading === 'create-profile'"
          :disabled="launcher.createVersionLoading || !launcher.newProfile.minecraftVersion"
        >
          프로필 생성
        </UButton>
      </div>
    </template>
  </UModal>
</template>
