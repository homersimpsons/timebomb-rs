<script setup lang="ts">
import type { TableColumn, TimelineItem } from '@nuxt/ui'
import type { DeepReadonly } from '~/utils'
import { useReducer } from 'spacetimedb/vue'
import { reducers } from '~/spacetimedb'
import { Mood, Card } from '~/spacetimedb/types'
import type { GameLive, MyGameLive, User } from '~/spacetimedb/types'

const props = defineProps<{
  game: DeepReadonly<MyGameLive>
  me: User
}>()

const myRow = computed(() => props.game.playerRows.find(row => row.userId === props.me.id)!)
const otherRows = computed(() => props.game.playerRows.filter(row => row.userId !== props.me.id))
const myTurn = computed(() => props.game.timelineUsersId.at(-1) === props.me.id)
const bombCount = computed(() => props.game.playerRows.filter(p => p.playerCallBomb).length)
const defuseCount = computed(() => props.game.playerRows.map(p => p.playerCallDefuse ?? 0).reduce((acc, val) => acc + val, 0))
const defuseFoundCount = computed(() => props.game.timelineCards.filter(card => card.tag === Card.Defuse.tag).length)
const defuseRemainingCount = computed(() => props.game.playerRows.length - defuseFoundCount.value)

const showSensitiveInfo = ref(false)

const moodMap = {
  '😇': Mood.Angel,
  '🤨': Mood.Suspicious,
  '😃': Mood.Happy,
  '😴': Mood.Sleepy,
  '😈': Mood.Devil,
  '🧐': Mood.Detective,
  '😎': Mood.Cool,
  '😂': Mood.Laughing
}
type MoodMap = typeof moodMap
type MoodMapKeys = keyof MoodMap
const getMoodKeyByValue = (value: Mood): MoodMapKeys => Object.entries(moodMap).find(([_key, val]) => val.tag === value.tag)![0] as MoodMapKeys
const moods = Object.keys(moodMap) as MoodMapKeys[]
const myMood = computed(() => myRow.value.playerCallMood ? getMoodKeyByValue(myRow.value.playerCallMood) : undefined)
const gameCallMood = useReducer(reducers.gameCallMood)
const gameCallMoodHandler = (mood: MoodMapKeys) => gameCallMood({ mood: moodMap[mood] })
// TODO: Add unset mood

const gameCallDefuse = useReducer(reducers.gameCallDefuse)
const gameCallDefuseHandler = (defuse: number) => gameCallDefuse({ defuse: defuse })

const gameCallBomb = useReducer(reducers.gameCallBomb)
const gameCallBombHandler = (bomb: boolean) => gameCallBomb({ bomb })

const playersColumns: TableColumn<DeepReadonly<GameLive>>[] = [
  //   { accessorKey: 'name', header: 'Pseudo' },
  { accessorKey: 'playerCards', header: 'cards', cell: ({ row }) => row.original.playerCards.length },
  {
    accessorKey: 'playerCallBomb', header: 'bomb', cell: ({ row }) => {
      const callBomb = row.original.playerCallBomb
      return typeof callBomb === 'boolean' ? (callBomb ? '💣' : '🚫') : callBomb
    }
  },
  { accessorKey: 'playerCallDefuse', header: 'defuse' },
  {
    accessorKey: 'playerCallMood', header: 'mood', cell: ({ row }) => {
      const callMood = row.original.playerCallMood
      return callMood ? getMoodKeyByValue(callMood) : undefined
    }
  },
  { accessorKey: 'actions', header: 'Actions' }
]

const timeline = computed<TimelineItem[]>(() => {
  const timelineItems: TimelineItem[] = []
  for (const [idx, card] of props.game.timelineCards.entries()) {
    const picker = props.game.timelineUsersId[idx]
    const picked = props.game.timelineUsersId[idx + 1]
    timelineItems.push({
      description: `${picker} a pioché chez ${picked}`,
      icon: card.tag === Card.Secure.tag
        ? 'i-lucide-shield'
        : card.tag === Card.Defuse.tag
          ? 'i-lucide-shield-plus'
          : 'i-lucide-bomb'
    })
  }

  const playing = props.game.timelineUsersId.at(-1)
  timelineItems.push({
    description: `${playing} choisi une carte`,
    icon: 'i-lucide-shield-question-mark' // TODO? loading icon
  })

  return timelineItems
})

const gamePickDrawerOpen = ref(false)
const gamePickCard = useReducer(reducers.gamePickCard)
const gamePickCardHandler = async (pickPlayerId: number, cardIdx: number) => {
  await gamePickCard({ pickPlayerId, cardIdx })
  gamePickDrawerOpen.value = false
}
</script>

<template>
  <UContainer>
    <UCard class="mb-2">
      <template #header>
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <h2>Mes informations</h2>
            <UButton
              v-show="myTurn"
              label="mon tour"
              loading
              size="xs"
            />
          </div>
          <USwitch
            v-model="showSensitiveInfo"
            checked-icon="i-lucide-eye"
            unchecked-icon="i-lucide-eye-off"
          />
        </div>
      </template>

      <!-- TODO: _1, _2, _3 -->
      <div class="flex items-center gap-4 mb-2">
        <div>
          <img
            v-show="! showSensitiveInfo"
            src="~/assets/img/back.png"
            alt="Hidden"
          >
          <img
            v-show="showSensitiveInfo"
            v-if="myRow.playerSherlock"
            src="~/assets/img/sherlock_0.png"
            alt="Sherlock"
          >
          <img
            v-show="showSensitiveInfo"
            v-else
            src="~/assets/img/moriarty_0.png"
            alt="Moriarty"
          >
        </div>
        <div>
          <div>
            <div class="flex items-start gap-4 mb-2">
              <UFormField
                label="bomb"
                name="bomb"
              >
                <USwitch
                  :model-value="myRow.playerCallBomb"
                  @update:model-value="gameCallBombHandler"
                />
              </UFormField>
              <UFormField
                label="defuse"
                name="defuse"
              >
                <UInputNumber
                  size="xs"
                  :model-value="myRow.playerCallDefuse"
                  @update:model-value="gameCallDefuseHandler"
                />
              </UFormField>
            </div>
            <UFormField
              label="mood"
              name="mood"
            >
              <URadioGroup
                :model-value="myMood"
                :items="moods"
                orientation="horizontal"
                variant="card"
                indicator="hidden"
                size="xs"
                @update:model-value="gameCallMoodHandler"
              />
            </UFormField>
          </div>
        </div>
      </div>
      <div class="flex gap-2">
        <!-- TODO: Fix design when <5 cards -->
        <div
          v-for="card in myRow.playerCards"
          :key="card"
        >
          <img
            v-show="! showSensitiveInfo"
            src="~/assets/img/back.png"
            alt="Hidden"
          >
          <img
            v-show="showSensitiveInfo"
            v-if="card.tag === Card.Secure.tag"
            src="~/assets/img/secure_cable.png"
            alt="Secure"
          >
          <img
            v-show="showSensitiveInfo"
            v-else-if="card.tag === Card.Defuse.tag"
            src="~/assets/img/defusing_cable.png"
            alt="Defuse"
          >
          <img
            v-show="showSensitiveInfo"
            v-else-if="card.tag === Card.Bomb.tag"
            src="~/assets/img/bomb.png"
            alt="Bomb"
          >
        </div>
      </div>
    </UCard>
    <UCard class="mb-2">
      <template #header>
        <div class="flex justify-between items-center">
          <h2>Partie</h2>
          <div class="flex items-center gap-4">
            <UBadge
              icon="i-lucide-bomb"
              :label="`${bombCount} / 1`"
              :color="bombCount === 1 ? 'success' : 'warning'"
            />
            <UBadge
              icon="i-lucide-shield-plus"
              :label="`${defuseCount} / ${defuseRemainingCount}`"
              :color="defuseCount === defuseRemainingCount ? 'success' : 'warning'"
            />
          </div>
        </div>
      </template>
      <UTable
        :data="otherRows"
        :columns="playersColumns"
        class="flex-1"
      >
        <template #actions-cell="{ row }">
          <UDrawer
            v-model:open="gamePickDrawerOpen"
            :handle="false"
            title="Choisis une carte à piocher"
            close
          >
            <UButton v-show="myTurn">
              Pick
            </UButton>
            <template #body>
              <UContainer>
                <!-- TODO: Fix design when <5 cards -->
                <div class="flex gap-4">
                  <div
                    v-for="(_card, cardIdx) in row.original.playerCards"
                    :key="cardIdx"
                    class="m-2"
                  >
                    <img
                      src="~/assets/img/back.png"
                      @click="gamePickCardHandler(row.original.userId, cardIdx)"
                    >
                  </div>
                </div>
              </UContainer>
            </template>
          </UDrawer>
        </template>
      </UTable>
    </UCard>
    <UCard class="mb-2">
      <template #header>
        <h2>Événements</h2>
      </template>
      <UTimeline :items="timeline" />
    </UCard>
  </UContainer>
</template>
