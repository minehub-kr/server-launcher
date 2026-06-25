<script setup lang="ts">
import { useLauncher } from '~/composables/useLauncherState'

const launcher = useLauncher()
</script>

<template>
  <section class="grid gap-4 xl:grid-cols-[420px_minmax(0,1fr)]">
    <div class="panel p-4">
      <div class="mb-3 flex items-center justify-between">
        <div class="min-w-0">
          <div class="flex flex-wrap items-center gap-2">
            <h3 class="text-sm font-semibold text-highlighted">설치된 플러그인</h3>
            <UBadge color="warning" variant="soft">재시작 필요</UBadge>
          </div>
          <p class="mt-1 text-xs text-muted">설치/활성화/비활성화 변경은 서버 재시작 후 적용됩니다.</p>
        </div>
        <UButton size="sm" color="neutral" variant="subtle" icon="i-lucide-folder-open" @click="launcher.openPath('plugins')">폴더</UButton>
      </div>
      <div class="space-y-2">
        <div v-for="plugin in launcher.plugins" :key="plugin.filename" class="plugin-row">
          <span class="min-w-0">
            <span class="block truncate text-sm font-medium">{{ plugin.displayName }}</span>
            <span class="text-xs text-muted">{{ (plugin.size / 1024 / 1024).toFixed(2) }} MB</span>
          </span>
          <div class="flex shrink-0 items-center gap-2">
            <UBadge :color="plugin.enabled ? 'success' : 'neutral'" variant="subtle">
              {{ plugin.enabled ? '활성화됨' : '비활성화됨' }}
            </UBadge>
            <UButton
              size="sm"
              :color="plugin.enabled ? 'warning' : 'success'"
              variant="subtle"
              :icon="plugin.enabled ? 'i-lucide-toggle-left' : 'i-lucide-toggle-right'"
              :loading="launcher.loading === `plugin-${plugin.filename}`"
              @click="launcher.setPluginEnabled(plugin, !plugin.enabled)"
            >
              {{ plugin.enabled ? '비활성화' : '활성화' }}
            </UButton>
          </div>
        </div>
        <p v-if="!launcher.plugins.length" class="empty-note">설치된 플러그인이 없습니다.</p>
      </div>
    </div>

    <div class="panel p-4">
      <h3 class="mb-3 text-sm font-semibold text-highlighted">Modrinth 검색</h3>
      <div class="control-row mb-3">
        <UInput v-model="launcher.pluginQuery" class="w-full" icon="i-lucide-search" placeholder="LuckPerms, Geyser..." @keyup.enter="launcher.searchPlugins" />
        <UButton icon="i-lucide-search" :loading="launcher.loading === 'search-plugins'" @click="launcher.searchPlugins">검색</UButton>
      </div>
      <div class="grid min-w-0 gap-3 2xl:grid-cols-2">
        <article v-for="project in launcher.modrinthProjects" :key="project.project_id" class="mod-card">
          <div class="flex min-w-0 gap-3">
            <img v-if="project.icon_url" :src="project.icon_url" alt="" class="size-10 shrink-0 rounded object-cover">
            <div class="min-w-0 flex-1 overflow-hidden">
              <h4 class="truncate text-sm font-semibold text-highlighted">{{ project.title }}</h4>
              <p class="mt-1 line-clamp-2 break-words text-xs text-muted">{{ project.description }}</p>
            </div>
          </div>
          <div class="mt-3 flex min-w-0 items-center justify-between gap-3">
            <span class="min-w-0 truncate text-xs text-muted">{{ project.downloads.toLocaleString() }} downloads</span>
            <UButton class="shrink-0" size="sm" icon="i-lucide-package-plus" :loading="launcher.loading === `install-${project.project_id}`" @click="launcher.installPlugin(project)">설치</UButton>
          </div>
        </article>
      </div>
      <p v-if="!launcher.modrinthProjects.length" class="empty-note">검색 결과가 없습니다.</p>
    </div>
  </section>
</template>
