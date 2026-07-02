<script setup lang="ts">
import { useLauncher } from '~/composables/useLauncherState'
import { computed } from 'vue'

const launcher = useLauncher()

const tabs = computed(() =>
  launcher.tabs.map((tab) => ({
    ...tab,
    disabled: tab.value === 'plugins' && !launcher.canUsePlugins,
    badge:
      tab.value === 'plugins' && launcher.pluginUpdateCount
        ? { content: launcher.pluginUpdateCount, color: 'warning', variant: 'soft', size: 'xs' }
        : undefined
  }))
)
</script>

<template>
  <nav class="border-b border-default px-5 py-4">
    <UTabs
      v-model="launcher.mainTab"
      :content="false"
      :items="tabs"
      variant="link"
      class="w-full"
    />
  </nav>
</template>
