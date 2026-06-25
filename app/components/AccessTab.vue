<script setup lang="ts">
import { useLauncher } from '~/composables/useLauncherState'

const launcher = useLauncher()
</script>

<template>
  <section class="space-y-4">
    <div class="flex flex-wrap items-center justify-between gap-2">
      <div>
        <h3 class="text-sm font-semibold text-highlighted">권한/차단</h3>
        <p class="mt-1 text-xs text-muted">닉네임으로 Mojang UUID를 조회한 뒤 서버 JSON 목록에 저장합니다.</p>
      </div>
      <div class="flex gap-2">
        <UButton color="neutral" variant="subtle" icon="i-lucide-refresh-cw" @click="launcher.refreshAccessLists">새로고침</UButton>
        <UButton icon="i-lucide-save" :loading="launcher.loading === 'save-access'" @click="launcher.saveAccessLists">목록 저장</UButton>
      </div>
    </div>

    <div class="grid gap-4 xl:grid-cols-3">
      <div class="panel p-4">
        <h4 class="mb-3 text-sm font-semibold text-highlighted">관리자</h4>
        <div class="control-row mb-3">
          <UInput v-model="launcher.accessName.ops" class="w-full" icon="i-lucide-user-plus" placeholder="닉네임" @keyup.enter="launcher.addAccessEntry('ops')" />
          <UButton icon="i-lucide-plus" :loading="launcher.loading === 'access-ops'" @click="launcher.addAccessEntry('ops')">추가</UButton>
        </div>
        <div class="space-y-2">
          <div v-for="(entry, index) in launcher.accessLists.ops" :key="entry.uuid" class="access-row">
            <div class="min-w-0">
              <p class="truncate text-sm font-medium text-highlighted">{{ entry.name }}</p>
              <p class="truncate text-xs text-muted">{{ entry.uuid }}</p>
            </div>
            <UInput v-model.number="entry.level" type="number" min="1" max="4" size="sm" class="w-20 shrink-0" />
            <UButton size="sm" color="error" variant="subtle" icon="i-lucide-trash" @click="launcher.removeAccessEntry('ops', index)" />
          </div>
          <p v-if="!launcher.accessLists.ops.length" class="empty-note">등록된 관리자가 없습니다.</p>
        </div>
        <UButton class="mt-3" size="sm" color="neutral" variant="subtle" @click="launcher.accessRawOpen.ops = !launcher.accessRawOpen.ops">원문 편집</UButton>
        <UTextarea v-if="launcher.accessRawOpen.ops" v-model="launcher.accessLists.rawOps" class="mt-3 w-full font-mono text-xs" :rows="8" />
      </div>

      <div class="panel p-4">
        <div class="mb-3 flex items-start justify-between gap-3">
          <div class="min-w-0">
            <h4 class="text-sm font-semibold text-highlighted">화이트리스트</h4>
            <p class="mt-1 text-xs text-muted">{{ launcher.whitelistEnabled ? '접속을 목록에 있는 플레이어로 제한합니다.' : '모든 플레이어 접속을 허용합니다.' }}</p>
          </div>
          <USwitch
            :model-value="launcher.whitelistEnabled"
            :loading="launcher.loading === 'whitelist-toggle'"
            :disabled="!launcher.selectedProfile || !launcher.config"
            label="화리"
            @update:model-value="launcher.setWhitelistEnabled($event)"
          />
        </div>
        <div class="control-row mb-3">
          <UInput v-model="launcher.accessName.whitelist" class="w-full" icon="i-lucide-user-plus" placeholder="닉네임" @keyup.enter="launcher.addAccessEntry('whitelist')" />
          <UButton icon="i-lucide-plus" :loading="launcher.loading === 'access-whitelist'" @click="launcher.addAccessEntry('whitelist')">추가</UButton>
        </div>
        <div class="space-y-2">
          <div v-for="(entry, index) in launcher.accessLists.whitelist" :key="entry.uuid" class="access-row">
            <div class="min-w-0">
              <p class="truncate text-sm font-medium text-highlighted">{{ entry.name }}</p>
              <p class="truncate text-xs text-muted">{{ entry.uuid }}</p>
            </div>
            <UButton size="sm" color="error" variant="subtle" icon="i-lucide-trash" @click="launcher.removeAccessEntry('whitelist', index)" />
          </div>
          <p v-if="!launcher.accessLists.whitelist.length" class="empty-note">화이트리스트가 비어 있습니다.</p>
        </div>
        <UButton class="mt-3" size="sm" color="neutral" variant="subtle" @click="launcher.accessRawOpen.whitelist = !launcher.accessRawOpen.whitelist">원문 편집</UButton>
        <UTextarea v-if="launcher.accessRawOpen.whitelist" v-model="launcher.accessLists.rawWhitelist" class="mt-3 w-full font-mono text-xs" :rows="8" />
      </div>

      <div class="panel p-4">
        <h4 class="mb-3 text-sm font-semibold text-highlighted">차단 플레이어</h4>
        <div class="control-row mb-2">
          <UInput v-model="launcher.accessName.bannedPlayers" class="w-full" icon="i-lucide-user-x" placeholder="닉네임" @keyup.enter="launcher.addAccessEntry('bannedPlayers')" />
          <UButton icon="i-lucide-ban" :loading="launcher.loading === 'access-bannedPlayers'" @click="launcher.addAccessEntry('bannedPlayers')">차단</UButton>
        </div>
        <UInput v-model="launcher.banReason" class="mb-3 w-full" icon="i-lucide-message-square" placeholder="차단 사유" />
        <div class="space-y-2">
          <div v-for="(entry, index) in launcher.accessLists.bannedPlayers" :key="entry.uuid" class="access-row">
            <div class="min-w-0">
              <p class="truncate text-sm font-medium text-highlighted">{{ entry.name }}</p>
              <p class="truncate text-xs text-muted">{{ entry.reason }}</p>
            </div>
            <UButton size="sm" color="error" variant="subtle" icon="i-lucide-trash" @click="launcher.removeAccessEntry('bannedPlayers', index)" />
          </div>
          <p v-if="!launcher.accessLists.bannedPlayers.length" class="empty-note">차단된 플레이어가 없습니다.</p>
        </div>
        <UButton class="mt-3" size="sm" color="neutral" variant="subtle" @click="launcher.accessRawOpen.bannedPlayers = !launcher.accessRawOpen.bannedPlayers">원문 편집</UButton>
        <UTextarea v-if="launcher.accessRawOpen.bannedPlayers" v-model="launcher.accessLists.rawBannedPlayers" class="mt-3 w-full font-mono text-xs" :rows="8" />
      </div>
    </div>
  </section>
</template>
