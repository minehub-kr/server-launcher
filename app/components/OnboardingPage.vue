<script setup lang="ts">
import { useLauncher } from '~/composables/useLauncherState'

const launcher = useLauncher()
</script>

<template>
  <section class="grid h-full min-h-0 grid-cols-[minmax(20rem,0.8fr)_minmax(0,1.2fr)] overflow-hidden max-lg:grid-cols-1">
    <aside class="flex min-h-0 flex-col justify-between border-r border-default bg-elevated p-8 max-lg:border-b max-lg:border-r-0 max-lg:p-6">
      <div>
        <div class="mb-5 flex size-12 items-center justify-center rounded-md bg-primary/10 text-primary">
          <UIcon name="i-lucide-server-cog" class="size-6" />
        </div>
        <h1 class="text-2xl font-semibold tracking-normal text-highlighted">Minehub Server Launcher</h1>
        <p class="mt-3 max-w-sm text-sm leading-6 text-muted">첫 서버 프로필을 설정하세요.</p>
      </div>

      <div class="mt-8 grid gap-3 text-sm text-muted">
        <div class="flex items-center gap-3">
          <UIcon name="i-lucide-box" class="size-4 text-primary" />
          <span>서버 구현체와 Minecraft 버전을 선택합니다.</span>
        </div>
        <div class="flex items-center gap-3">
          <UIcon name="i-lucide-folder" class="size-4 text-primary" />
          <span>서버 파일을 둘 폴더를 지정합니다.</span>
        </div>
        <div class="flex items-center gap-3">
          <UIcon name="i-lucide-memory-stick" class="size-4 text-primary" />
          <span>실행 메모리를 정합니다.</span>
        </div>
      </div>
    </aside>

    <div class="min-h-0 overflow-auto p-8 max-lg:p-6">
      <form class="mx-auto flex min-h-full w-full max-w-2xl flex-col justify-center gap-5 py-6" @submit.prevent="launcher.createProfile">
        <div class="space-y-2">
          <UBadge color="neutral" variant="outline">프로필 0개</UBadge>
          <h2 class="text-xl font-semibold tracking-normal text-highlighted">새 서버 프로필</h2>
        </div>

        <div class="grid gap-4">
          <UFormField label="프로필 이름" class="settings-field">
            <UInput v-model="launcher.newProfile.name" class="w-full" icon="i-lucide-pencil" placeholder="예: Survival Paper" />
          </UFormField>

          <div class="grid grid-cols-2 gap-4 max-sm:grid-cols-1">
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
          </div>

          <UFormField label="서버 폴더" class="settings-field">
            <div class="control-row">
              <UInput v-model="launcher.newProfile.serverDir" class="w-full" icon="i-lucide-folder" placeholder="서버 폴더" />
              <UTooltip text="폴더 선택">
                <UButton type="button" icon="i-lucide-folder-open" color="neutral" variant="subtle" @click="launcher.chooseDirectory('create')" />
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
        </div>

        <div class="flex justify-end">
          <UButton
            type="submit"
            icon="i-lucide-plus"
            :loading="launcher.loading === 'create-profile'"
            :disabled="launcher.createVersionLoading || !launcher.newProfile.minecraftVersion"
          >
            프로필 생성
          </UButton>
        </div>
      </form>
    </div>
  </section>
</template>
