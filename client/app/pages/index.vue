<script setup lang="ts">
import { useTable } from 'spacetimedb/vue'
import { tables } from '~/spacetimedb'

const [userData] = useTable(tables.me)
const [gameLive] = useTable(tables.myGameLive)
const [gameLobby] = useTable(tables.gameLobby)

const me = computed(() => userData.value[0])
const myGameLive = computed(() => gameLive.value[0] ?? null)
const [gameDone] = useTable(tables.gameDone)
</script>

<template>
  <div v-if="me">
    <GameLive
      v-if="myGameLive"
      :game="myGameLive"
      :me="me"
    />
    <!-- <GameLive v-if="true" /> -->
    <GameLobby
      v-else
      :game-lobby="gameLobby"
      :game-done="gameDone"
      :me="me"
    />
    <!-- hat-glasses, hat-glasses -->
  </div>
</template>
