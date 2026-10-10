<script setup lang="ts">
import type { TimelineItem } from '@nuxt/ui'
import type { DeepReadonly } from '~/utils'
import { useReducer } from 'spacetimedb/vue'
import { reducers } from '~/spacetimedb'
import { Mood, Card, Role } from '~/spacetimedb/types'
import type { MyGameLive, User } from '~/spacetimedb/types'
import sherlock0 from '~/assets/img/sherlock_0.png'
import sherlock1 from '~/assets/img/sherlock_1.png'
import sherlock2 from '~/assets/img/sherlock_2.png'
import sherlock3 from '~/assets/img/sherlock_3.png'
import sherlock4 from '~/assets/img/sherlock_4.png'
import moriarty0 from '~/assets/img/moriarty_0.png'
import moriarty1 from '~/assets/img/moriarty_1.png'
import moriarty2 from '~/assets/img/moriarty_2.png'
import backImg from '~/assets/img/back.png'
import secureImg from '~/assets/img/secure_cable.png'
import defuseImg from '~/assets/img/defusing_cable.png'
import bombImg from '~/assets/img/bomb.png'

const props = defineProps<{
  game: DeepReadonly<MyGameLive>
  me: User
}>()

const myRow = computed(() => props.game.playerRows.find(row => row.userId === props.me.id)!)
const otherRows = computed(() => props.game.playerRows.filter(row => row.userId !== props.me.id))
const playingUserId = computed(() => props.game.timelineUsersId.at(-1))
const myTurn = computed(() => playingUserId.value === props.me.id)
const bombCount = computed(() => props.game.playerRows.filter(p => p.callsBomb).length)
const defuseCount = computed(() => props.game.playerRows.map(p => p.callsDefuse ?? 0).reduce((acc, val) => acc + val, 0))
const defuseFoundCount = computed(() => props.game.timelineCards.filter(card => card.tag === Card.Defuse.tag).length)
const defuseRemainingCount = computed(() => props.game.playerRows.length - defuseFoundCount.value)

const showSensitiveInfo = ref(false)

// eslint-disable-next-line vue/return-in-computed-property -- already covered by TypeScript
const myRoleImg = computed((): string => {
  switch (myRow.value.role.tag) {
    case Role.Sherlock0.tag: return sherlock0
    case Role.Moriarty0.tag: return moriarty0
    case Role.Sherlock1.tag: return sherlock1
    case Role.Moriarty1.tag: return moriarty1
    case Role.Sherlock2.tag: return sherlock2
    case Role.Sherlock3.tag: return sherlock3
    case Role.Moriarty2.tag: return moriarty2
    case Role.Sherlock4.tag: return sherlock4
  }
})
// @ts-expect-error TS2345 TypeScript incorrectly reports an error for the includes check
const myRoleAlt = computed(() => [Role.Moriarty0.tag, Role.Moriarty1.tag, Role.Moriarty2.tag].includes(myRow.value.role.tag) ? 'Moriarty' : 'Sherlock')

const cardImages = {
  [Card.Secure.tag]: { src: secureImg, alt: 'Secure' },
  [Card.Defuse.tag]: { src: defuseImg, alt: 'Defuse' },
  [Card.Bomb.tag]: { src: bombImg, alt: 'Bomb' }
} as Record<string, { src: string, alt: string }>
const myCards = computed(() => myRow.value.cards.map(card => cardImages[card.tag]!))

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
const myMood = computed(() => myRow.value.callsMood ? getMoodKeyByValue(myRow.value.callsMood) : undefined)
const gameCallMood = useReducer(reducers.gameCallMood)
const gameCallMoodHandler = (mood: MoodMapKeys) => gameCallMood({ mood: moodMap[mood] })
// TODO: Add unset mood

const gameCallDefuse = useReducer(reducers.gameCallDefuse)
const gameCallDefuseHandler = (defuse: number) => gameCallDefuse({ defuse: defuse })

const gameCallBomb = useReducer(reducers.gameCallBomb)
const gameCallBombHandler = (bomb: boolean) => gameCallBomb({ bomb })

const timeline = computed<TimelineItem[]>(() => {
  const timelineItems: TimelineItem[] = []
  const userIdToNameMap = Object.fromEntries(props.game.playerRows.map(p => [p.userId, p.userName]))
  for (const [idx, card] of props.game.timelineCards.entries()) {
    const picker = userIdToNameMap[props.game.timelineUsersId[idx]!]
    const picked = userIdToNameMap[props.game.timelineUsersId[idx + 1]!]
    const cardLabel = {
      [Card.Secure.tag]: 'un câble sécurisé',
      [Card.Defuse.tag]: 'un câble de désamorçage',
      [Card.Bomb.tag]: 'la bombe'
    }[card.tag]
    timelineItems.push({
      description: `${picker} a pioché ${cardLabel} chez ${picked}`,
      icon: {
        [Card.Secure.tag]: 'i-lucide-shield',
        [Card.Defuse.tag]: 'i-lucide-shield-plus',
        [Card.Bomb.tag]: 'i-lucide-bomb'
      }[card.tag]
    })
  }

  const playing = userIdToNameMap[props.game.timelineUsersId.at(-1)!]
  timelineItems.push({
    description: `${playing} choisi une carte`,
    icon: 'i-lucide-shield-question-mark' // TODO? loading icon
  })

  return timelineItems
})

const gamePickCard = useReducer(reducers.gamePickCard)
const gamePickCardHandler = async (pickPlayerId: number, cardIdx: number) => {
  await gamePickCard({ pickPlayerId, cardIdx })
  pickerSelected.value = null
}
const pickerSelected = ref<number | null>(null)
</script>

<template>
  <UContainer class="pt-2 grid grid-cols-1 gap-2 md:grid-cols-2">
    <UCard>
      <template #header>
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <h2>Mes cartes</h2>
            <!-- TODO: Cartes modifiées -->
          </div>
          <UFormField
            :label="showSensitiveInfo ? 'Masquer' : 'Afficher'"
            name="showSensitiveInfo"
            orientation="horizontal"
            class="flex items-center"
          >
            <USwitch
              v-model="showSensitiveInfo"
              checked-icon="i-lucide-eye"
              unchecked-icon="i-lucide-eye-off"
            />
          </UFormField>
        </div>
      </template>

      <div class="flex items-center gap-4">
        <figure class="w-1/3 shrink-0 text-center">
          <img
            v-show="!showSensitiveInfo"
            class="w-full"
            :src="backImg"
            alt="Hidden"
          >
          <img
            v-show="showSensitiveInfo"
            class="w-full"
            :src="myRoleImg"
            :alt="myRoleAlt"
          >
          <figcaption
            class="text-sm"
            :class="{ 'blur-sm': !showSensitiveInfo }"
          >
            {{ showSensitiveInfo ? myRoleAlt : 'Hidden' }}
          </figcaption>
        </figure>
        <div class="w-2/3 flex justify-center flex-wrap">
          <figure
            v-for="(card, cardIdx) in myCards"
            :key="cardIdx"
            class="text-center w-1/3 p-2"
          >
            <img
              v-show="!showSensitiveInfo"
              :src="backImg"
              alt="Hidden"
            >
            <img
              v-show="showSensitiveInfo"
              :src="card.src"
              :alt="card.alt"
            >
            <figcaption
              class="text-sm"
              :class="{ 'blur-sm': !showSensitiveInfo }"
            >
              {{ showSensitiveInfo ? card.alt : 'Hidden' }}
            </figcaption>
          </figure>
        </div>
      </div>
    </UCard>
    <UCard>
      <template #header>
        <h2>Mes annonces</h2>
      </template>
      <div class="space-y-4">
        <UFormField
          label="Avez-vous la bombe ?"
          name="bomb"
          orientation="horizontal"
        >
          <USwitch
            :model-value="myRow.callsBomb"
            @update:model-value="gameCallBombHandler"
          />
        </UFormField>
        <UFormField
          label="Combien avez-vous de désamorçage ?"
          name="defuse"
          orientation="horizontal"
        >
          <UInputNumber
            size="xs"
            :model-value="myRow.callsDefuse"
            @update:model-value="gameCallDefuseHandler"
          />
        </UFormField>
        <UFormField
          label="Quelle est votre humeur ?"
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
    </UCard>
    <UCard class="md:col-span-2">
      <template #header>
        <div class="flex justify-between items-center">
          <div class="flex items-center gap-2">
            <h2>Partie</h2>
            <UButton
              v-show="myTurn"
              label="À vous de jouer"
              loading
              size="xs"
            />
          </div>
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
      <UPageGrid>
        <UPageCard
          v-for="player in otherRows"
          :key="player.userId"
          @click="pickerSelected = player.userId"
        >
          <template #title>
            <p class="flex items-center gap-2">
              {{ player.userName }}
              <UButton
                v-if="!player.connected"
                :loading="true"
                size="xs"
                label="Déconnecté"
                color="warning"
              />
              <UButton
                v-if="playingUserId === player.userId"
                :loading="true"
                size="xs"
                label="Réfléchit où piocher"
              />
              <UDrawer
                v-if="myTurn"
                :handle="false"
                title="Choisis une carte à piocher"
                close
                :open="pickerSelected === player.userId"
                @update:open="val => pickerSelected = (val ? player.userId : null)"
              >
                <UButton
                  icon="i-lucide-hand"
                  size="xs"
                  label="Piocher une carte"
                />
                <template #body>
                  <UContainer>
                    <div class="flex gap-4">
                      <div
                        v-for="(_card, cardIdx) in player.cards"
                        :key="cardIdx"
                        class="m-2"
                      >
                        <img
                          src="~/assets/img/back.png"
                          @click="gamePickCardHandler(player.userId, cardIdx)"
                        >
                      </div>
                    </div>
                  </UContainer>
                </template>
              </UDrawer>
            </p>
          </template>

          <div class="mt-3 grid grid-cols-2 gap-2 text-sm">
            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-playing-cards-fan" />
              <span>{{ player.cards.length }} cartes</span>
            </div>

            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-bomb" />
              <span>
                {{ player.callsBomb === true ? 'Bombe' : 'Pas de bombe' }}
              </span>
            </div>

            <div class="flex items-center gap-2">
              <UIcon name="i-lucide-shield-plus" />
              <span>{{ player.callsDefuse ?? 0 }} désamorçage</span>
            </div>

            <div class="flex items-center gap-2">
              <span>
                {{ player.callsMood ? getMoodKeyByValue(player.callsMood) : '—' }}
              </span>
              <span>humeur</span>
            </div>
          </div>
        </UPageCard>
      </UPageGrid>
    </UCard>
    <UCard class="md:col-span-2">
      <template #header>
        <h2>Événements</h2>
      </template>
      <UTimeline :items="timeline" />
    </UCard>
  </UContainer>
</template>
