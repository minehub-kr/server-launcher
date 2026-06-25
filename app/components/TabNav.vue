<script setup lang="ts">
import { useLauncher } from '~/composables/useLauncherState'

const launcher = useLauncher()
</script>

<template>
  <nav class="flex flex-wrap gap-2 border-b border-default px-5 py-4">
    <UButton
      v-for="tab in launcher.tabs"
      :key="tab.value"
      :icon="tab.icon"
      :color="launcher.mainTab === tab.value ? 'primary' : 'neutral'"
      :variant="launcher.mainTab === tab.value ? 'solid' : 'subtle'"
      :disabled="tab.value === 'plugins' && !launcher.canUsePlugins"
      @click="launcher.mainTab = tab.value"
    >
      {{ tab.label }}
    </UButton>
  </nav>
</template>
