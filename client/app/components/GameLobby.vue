<script setup lang="ts">
import type { TableColumn } from '@nuxt/ui'
import { useReducer } from 'spacetimedb/vue'
import { reducers } from '~/spacetimedb'
import { type GameLobby, type User, type GameDone, Card } from '~/spacetimedb/types'
import type { DeepReadonly } from '~/utils'

type ReadonlyGameDone = DeepReadonly<GameDone>

const props = defineProps<{
  gameLobby: readonly GameLobby[]
  gameDone: readonly ReadonlyGameDone[]
  me: User
}>()

const toast = useToast()

type GameRow = {
  id: number
  users: [number, boolean][]
}

const groupedGames = computed(() => {
  const games = new Map<number, GameRow>()
  for (const game of props.gameLobby) {
    const g = games.getOrInsert(game.lobbyId, {
      id: game.lobbyId,
      users: []
    } as GameRow)
    g.users.push([game.userId, game.ready])
  }

  return Array.from(games.values())
})

const myGameIdx = computed(() => groupedGames.value.findIndex(g => g.users.some(u => u[0] === props.me.id)))
const myGame = computed(() => myGameIdx.value !== -1 ? groupedGames.value[myGameIdx.value] : null)
const otherGames = computed<GameRow[]>(() => myGameIdx.value !== -1 ? groupedGames.value.toSpliced(myGameIdx.value, 1) : groupedGames.value)
const isReady = computed(() => myGame.value?.users.some(u => u[0] === props.me.id && u[1]) ?? false)

const columns: TableColumn<GameRow>[] = [
  { accessorKey: 'users', header: 'Players' },
  { accessorKey: 'actions', header: 'Actions' }
]

const gameJoin = useReducer(reducers.gameJoin)
const gameJoinHandler = (id?: number) => gameJoin({ id }).catch(err => toast.add({ title: 'Erreur', description: err.message }))
const gameLeaveReducer = useReducer(reducers.gameLeave)
const gameLeaveHandler = () => gameLeaveReducer()
const gameReadyReducer = useReducer(reducers.gameReady)
const gameReadyHandler = (ready: boolean) => gameReadyReducer({ ready })

const recentGames = computed(() => props.gameDone.toSorted((a, b) => b.id - a.id))

function isMoriartyWin(game: ReadonlyGameDone) {
  return game.timelineCards.at(-1)!.tag === Card.Bomb.tag
    || game.timelineCards.filter(c => c.tag === Card.Defuse.tag).length < game.players.length
}
</script>

<template>
  <UContainer class="pt-2">
    <UCard
      v-if="myGame"
      class="mb-2"
    >
      <template #header>
        <div class="flex justify-between items-center">
          <h2>Mon lobby</h2>
          <div class="flex items-center gap-4">
            <USwitch
              label="Prêt"
              :model-value="isReady"
              @update:model-value="gameReadyHandler"
            />
            <UButton
              label="Quitter"
              icon="i-lucide-log-out"
              @click="gameLeaveHandler()"
            />
          </div>
        </div>
      </template>

      <div class="flex gap-2 flex-wrap">
        <template
          v-for="([id, ready], index) in myGame.users"
          :key="index"
        >
          <UBadge
            :icon="ready ? 'i-lucide-badge-check' : 'i-lucide-badge-alert'"
            :label="id"
            :color="ready ? 'success' : 'warning'"
            variant="soft"
          />
        </template>
      </div>
      <!-- TODO: Report number of players min / max -->
    </UCard>

    <UCard class="mb-2">
      <template #header>
        <div class="flex justify-between items-center">
          <h2>Autres lobbies</h2>
          <UButton
            v-if="!myGame"
            label="Créer"
            icon="i-lucide-plus"
            @click="gameJoinHandler(undefined)"
          />
        </div>
      </template>

      <UTable
        :columns="columns"
        :data="otherGames"
      >
        <template #users-cell="{ row }">
          <!-- TODO: replace userIds with actual user names -->
          {{ row.original.users.map(u => u[0]).join(', ') }}
        </template>
        <template #actions-cell="{ row }">
          <UButton
            v-if="!myGame"
            @click="gameJoinHandler(row.original.id)"
          >
            Rejoindre
          </UButton>
        </template>
      </UTable>
    </UCard>
    <UCard
      v-if="recentGames.length > 0"
      class="mb-2"
    >
      <template #header>
        <h2>Dernières parties</h2>
      </template>

      <template
        v-for="(game, index) in recentGames"
        :key="index"
      >
        <UCard class="mb-2">
          <p>Vainqueur: {{ isMoriartyWin(game) ? 'Moriarty' : 'Sherlock' }}</p>
          <p>Moriarty: {{ game.players.filter(p => p.isMoriarty).map(p => p.name).join(', ') }}</p>
          <p>Sherlock: {{ game.players.filter(p => !p.isMoriarty).map(p => p.name).join(', ') }}</p>
        </UCard>
        <!-- <UBadge :label="`Game ${game.id}`" variant="soft" /> -->
      </template>
    </UCard>
  </UContainer>
</template>
