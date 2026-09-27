import fs from 'node:fs'

// https://nuxt.com/docs/api/configuration/nuxt-config
export default defineNuxtConfig({
  modules: [
    '@nuxt/eslint',
    '@nuxt/ui'
  ],

  devtools: {
    enabled: true
  },

  css: ['~/assets/css/main.css'],

  runtimeConfig: {
    public: {
      spacetimedbDbName: process.env.SPACETIMEDB_DB_NAME,
      spacetimedbHost: process.env.SPACETIMEDB_HOST
    }
  },

  routeRules: {
    '/': { prerender: true }
  },

  compatibilityDate: '2026-06-30',

  hooks: {
    'nitro:build:public-assets': (nitro) => {
      // ensure GitHub Pages doesn't run Jekyll on _nuxt assets
      fs.writeFileSync(`${nitro.options.output.publicDir}/.nojekyll`, '')
    }
  },

  eslint: {
    config: {
      stylistic: {
        commaDangle: 'never',
        braceStyle: '1tbs'
      }
    }
  }
})
