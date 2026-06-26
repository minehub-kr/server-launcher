<script setup lang="ts">
import { reactive } from 'vue'
import { useLauncher } from '~/composables/useLauncherState'
import type { ConfigFormField } from '~/types/launcher'

const launcher = useLauncher()
const rawOpen = reactive<Record<string, boolean>>({})
const fieldsFor = (relativePath: string) => launcher.config?.configFields.filter((field) => field.file === relativePath) || []
const fieldText = (field: ConfigFormField) => typeof field.value === 'string' ? field.value : String(field.value ?? '')
const fieldNumber = (field: ConfigFormField) => {
  const value = typeof field.value === 'number' ? field.value : Number(field.value)
  return Number.isFinite(value) ? value : 0
}
const setFieldValue = (field: ConfigFormField, value: unknown) => {
  if (field.kind === 'boolean') {
    field.value = value === true
    return
  }
  if (field.kind === 'number') {
    const next = typeof value === 'number' ? value : Number(value)
    field.value = Number.isFinite(next) ? next : 0
    return
  }
  field.value = String(value ?? '')
}
</script>

<template>
  <section class="grid gap-4 xl:grid-cols-[420px_minmax(0,1fr)]">
    <div class="space-y-4">
      <div class="panel p-4">
        <h3 class="mb-3 text-sm font-semibold text-highlighted">프로필 설정</h3>
        <div v-if="launcher.profileDraft" class="space-y-3">
          <UFormField label="이름" class="settings-field">
            <UInput v-model="launcher.profileDraft.name" class="w-full" icon="i-lucide-pencil" />
          </UFormField>
          <UFormField label="구현체" class="settings-field">
            <USelect v-model="launcher.profileDraft.kind" :items="launcher.serverKinds.map(({ label, value }) => ({ label, value }))" class="w-full" />
          </UFormField>
          <UFormField label="Minecraft 버전" class="settings-field">
            <USelect
              v-model="launcher.profileDraft.minecraftVersion"
              :items="launcher.versionOptions"
              class="w-full"
              :disabled="launcher.versionLoading"
            />
            <p v-if="launcher.versionLoading" class="mt-1 text-xs text-muted">버전 목록을 불러오는 중입니다.</p>
          </UFormField>
          <UFormField label="서버 폴더" class="settings-field">
            <div class="control-row">
              <UInput v-model="launcher.profileDraft.serverDir" class="w-full" icon="i-lucide-folder" />
              <UButton icon="i-lucide-folder-open" color="neutral" variant="subtle" @click="launcher.chooseDirectory('profile')" />
            </div>
          </UFormField>
          <UFormField label="메모리" class="settings-field">
            <UInput v-model.number="launcher.profileDraft.memoryMb" class="w-full" type="number" min="512" step="512" icon="i-lucide-memory-stick" />
          </UFormField>
          <UFormField label="Java 런타임" class="settings-field">
            <USelect
              v-model="launcher.profileDraft.javaPath"
              :items="[{ label: '자동 선택', value: null }, ...launcher.javaVersions.map((java) => ({ label: `Java ${java.major} - ${java.path}`, value: java.path }))]"
              class="w-full"
            />
          </UFormField>
          <UButton
            icon="i-lucide-save"
            :loading="launcher.loading === 'save-profile'"
            :disabled="launcher.versionLoading || !launcher.profileDraft.minecraftVersion"
            @click="launcher.saveProfile"
          >
            프로필 저장
          </UButton>
        </div>
      </div>

      <div class="panel p-4">
        <div class="mb-3">
          <h3 class="text-sm font-semibold text-highlighted">위험 작업</h3>
          <p class="mt-1 text-xs text-muted">프로필을 삭제해도 기본적으로 서버 폴더와 월드는 보존됩니다.</p>
        </div>
        <UButton
          color="error"
          variant="subtle"
          icon="i-lucide-trash"
          :disabled="!launcher.selectedProfile || launcher.activeProfileRunning"
          @click="launcher.openDeleteProfileDialog"
        >
          프로필 삭제
        </UButton>
        <p v-if="launcher.activeProfileRunning" class="mt-2 text-xs text-muted">실행 중인 프로필은 중지한 뒤 삭제할 수 있습니다.</p>
      </div>
    </div>

    <div class="space-y-4">
      <div class="panel p-4">
        <div class="mb-4 flex items-center justify-between">
          <h3 class="text-sm font-semibold text-highlighted">server.properties</h3>
          <UBadge color="warning" variant="soft">재시작 필요</UBadge>
        </div>
        <div v-if="launcher.config" class="settings-grid">
          <UFormField label="포트" class="settings-field">
            <UInput v-model.number="launcher.config.properties.serverPort" class="w-full" type="number" />
          </UFormField>
          <UFormField label="최대 플레이어" class="settings-field">
            <UInput v-model.number="launcher.config.properties.maxPlayers" class="w-full" type="number" />
          </UFormField>
          <UFormField label="MOTD" class="settings-field">
            <UInput v-model="launcher.config.properties.motd" class="w-full" />
          </UFormField>
          <UFormField label="난이도" class="settings-field">
            <USelect v-model="launcher.config.properties.difficulty" class="w-full" :items="['peaceful', 'easy', 'normal', 'hard']" />
          </UFormField>
          <UFormField label="게임 모드" class="settings-field">
            <USelect v-model="launcher.config.properties.gamemode" class="w-full" :items="['survival', 'creative', 'adventure', 'spectator']" />
          </UFormField>
          <UFormField label="View distance" class="settings-field">
            <UInput v-model.number="launcher.config.properties.viewDistance" class="w-full" type="number" />
          </UFormField>
          <UFormField label="Simulation distance" class="settings-field">
            <UInput v-model.number="launcher.config.properties.simulationDistance" class="w-full" type="number" />
          </UFormField>
          <UCheckbox v-model="launcher.config.properties.onlineMode" class="check-row" label="온라인 모드" />
          <UCheckbox v-model="launcher.config.properties.pvp" class="check-row" label="PVP" />
          <UCheckbox v-model="launcher.config.properties.enableCommandBlock" class="check-row" label="커맨드 블록" />
          <UCheckbox v-model="launcher.config.properties.whiteList" class="check-row" label="화이트리스트" />
        </div>
      </div>

      <div class="panel p-4">
        <div class="mb-3 flex items-center justify-between">
          <h3 class="text-sm font-semibold text-highlighted">고급 설정 파일</h3>
          <UButton size="sm" color="neutral" variant="subtle" icon="i-lucide-refresh-cw" @click="launcher.refreshConfig">다시 감지</UButton>
        </div>
        <div v-if="launcher.config?.configFiles.length" class="space-y-4">
          <div v-for="file in launcher.config.configFiles" :key="file.relativePath" class="advanced-config">
            <div class="mb-2 flex items-center justify-between gap-2">
              <span class="truncate text-sm font-medium text-highlighted">{{ file.relativePath }}</span>
              <UBadge :color="file.exists ? 'success' : 'neutral'" variant="soft">{{ file.exists ? '감지됨' : '미생성' }}</UBadge>
            </div>
            <div v-if="file.editable" class="space-y-3">
              <div v-if="fieldsFor(file.relativePath).length" class="settings-grid">
                <div v-for="field in fieldsFor(file.relativePath)" :key="`${field.file}:${field.path}`" class="space-y-1">
                  <UCheckbox
                    v-if="field.kind === 'boolean'"
                    :model-value="field.value === true"
                    class="check-row"
                    :label="field.label"
                    @update:model-value="setFieldValue(field, $event)"
                  />
                  <template v-else>
                    <div class="flex items-center gap-2">
                      <p class="text-xs font-medium text-muted">{{ field.label }}</p>
                      <UBadge v-if="field.restartRequired" size="xs" color="warning" variant="soft">재시작 필요</UBadge>
                    </div>
                    <UInput
                      v-if="field.kind === 'number'"
                      :model-value="fieldNumber(field)"
                      class="w-full"
                      type="number"
                      @update:model-value="setFieldValue(field, $event)"
                    />
                    <USelect
                      v-else-if="field.kind === 'select'"
                      :model-value="fieldText(field)"
                      class="w-full"
                      :items="field.options || []"
                      @update:model-value="setFieldValue(field, $event)"
                    />
                    <UInput
                      v-else
                      :model-value="fieldText(field)"
                      class="w-full"
                      @update:model-value="setFieldValue(field, $event)"
                    />
                  </template>
                </div>
              </div>
              <p v-else class="empty-note">폼으로 지원하는 핵심 설정이 없습니다. 원문 편집을 사용하세요.</p>

              <div>
                <UButton
                  size="sm"
                  color="neutral"
                  variant="subtle"
                  :icon="rawOpen[file.relativePath] ? 'i-lucide-chevron-up' : 'i-lucide-chevron-down'"
                  @click="rawOpen[file.relativePath] = !rawOpen[file.relativePath]"
                >
                  원문 편집
                </UButton>
                <UTextarea
                  v-if="rawOpen[file.relativePath]"
                  v-model="file.content"
                  class="mt-3 w-full font-mono text-xs"
                  :rows="10"
                />
              </div>
            </div>
            <p v-else class="empty-note">서버가 첫 구동 후 생성하면 여기서 편집할 수 있습니다.</p>
          </div>
        </div>
      </div>

      <UButton icon="i-lucide-save" :loading="launcher.loading === 'save-config'" @click="launcher.saveConfig">설정 저장</UButton>
    </div>
  </section>
</template>
