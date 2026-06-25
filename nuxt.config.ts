export default defineNuxtConfig({
  compatibilityDate: '2026-06-25',
  devtools: { enabled: false },
  telemetry: false,
  ssr: false,
  modules: ['@nuxt/ui'],
  css: ['~/assets/css/main.css'],
  ui: {
    fonts: false,
    colorMode: true
  },
  app: {
    head: {
      title: 'Minehub Server Launcher',
      meta: [
        { name: 'viewport', content: 'width=device-width, initial-scale=1' }
      ]
    }
  },
  vite: {
    clearScreen: false,
    envPrefix: ['VITE_', 'TAURI_'],
    server: {
      strictPort: true
    }
  },
  ignore: ['**/src-tauri/**']
})
