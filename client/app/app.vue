<script setup lang="ts">
import type { Identity } from 'spacetimedb'
import { DbConnection, type ErrorContext } from './spacetimedb'
import { SpacetimeDBProvider } from 'spacetimedb/vue'
import coverUrl from '~/assets/img/cover.jpeg?url'

useHead({
  meta: [
    { name: 'viewport', content: 'width=device-width, initial-scale=1' }
  ],
  link: [
    { rel: 'icon', href: '/favicon.ico' }
  ],
  htmlAttrs: {
    lang: 'fr'
  }
})

const title = 'Timebomb'
const description = 'Un jeu de bluff explosif !'

useSeoMeta({
  title,
  description,
  ogTitle: title,
  ogDescription: description,
  ogImage: coverUrl,
  twitterCard: 'summary_large_image'
})

const config = useRuntimeConfig()
const HOST = config.public.spacetimedbHost!
const DB_NAME = config.public.spacetimedbDbName!
const TOKEN_KEY = `${HOST}/${DB_NAME}/auth_token`

const isConnected = ref(false)

const onConnect = (_conn: DbConnection, identity: Identity, token: string) => {
  localStorage.setItem(TOKEN_KEY, token)
  isConnected.value = true
  console.log(
    'Connected to SpacetimeDB with identity:',
    identity.toHexString()
  )
}
const onDisconnect = () => {
  isConnected.value = false
  console.log('Disconnected from SpacetimeDB')
}

const onConnectError = (_ctx: ErrorContext, err: Error) => {
  isConnected.value = false
  console.log('Error connecting to SpacetimeDB:', err)
}
const connectionBuilder = import.meta.client
  ? DbConnection.builder()
      .withUri(HOST)
      .withDatabaseName(DB_NAME)
      .withToken(localStorage.getItem(TOKEN_KEY) || undefined)
      .onConnect(onConnect)
      .onDisconnect(onDisconnect)
      .onConnectError(onConnectError)
  : undefined
</script>

<template>
  <ClientOnly>
    <UApp>
      <SpacetimeDBProvider :connection-builder="connectionBuilder!">
        <UHeader :toggle="false">
          <template #left>
            <template v-if="isConnected">
              <UIcon
                name="i-lucide-globe-check"
                class="size-5"
              /> Connecté
            </template>
            <template v-else>
              <UIcon
                name="i-lucide-globe-x"
                class="size-5"
              /> Non connecté
            </template>
            <UserName /> <!-- TODO: move to center -->
          </template>

          <template #right>
            <UColorModeButton />

            <UDrawer
              direction="right"
              :handle="false"
              :close="true"
            >
              <UButton
                label="Règles"
                color="neutral"
                variant="subtle"
                trailing-icon="i-lucide-book-open-check"
              />

              <template #body>
                <LazyTbRules />
              </template>
            </UDrawer>
          </template>
        </UHeader>

        <UMain>
          <NuxtPage />
        </UMain>

        <USeparator icon="i-lucide-playing-cards-fan" />

        <UFooter>
          <template #left>
            <p class="text-sm text-muted">
              Built with Nuxt UI and SpacetimeDB • © {{ new Date().getFullYear() }}
            </p>
          </template>

          <template #right>
            <UButton
              to="https://github.com/homersimpsons/timebomb-rs"
              target="_blank"
              icon="i-simple-icons-github"
              aria-label="GitHub"
              color="neutral"
              variant="ghost"
            />
          </template>
        </UFooter>

        <template #fallback>
          <div class="p-4 text-center">
            <p>Loading...</p>
            <p>or failed to connect to spacetimedb</p>
          </div>
        </template>
      </SpacetimeDBProvider>
    </UApp>
  </ClientOnly>
</template>
