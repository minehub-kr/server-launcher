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
      ],
      link: [
        { rel: 'stylesheet', href: 'https://cdn.jsdelivr.net/gh/orioncactus/pretendard/dist/web/static/pretendard.css' }
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
