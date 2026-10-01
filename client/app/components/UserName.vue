<script setup lang="ts">
import { useDebounceFn } from '@vueuse/core'
import { useReducer, useTable } from 'spacetimedb/vue'
import { reducers, tables } from '~/spacetimedb'

const toast = useToast()

const [userData] = useTable(tables.me)
const userUpdate = useReducer(reducers.userUpdate)
const userUpdateHandler = useDebounceFn((name: string) =>
  userUpdate({ name }).catch(err => toast.add({ title: 'Erreur', description: err.message })),
300)
</script>

<template>
  <UInput
    v-if="userData[0]"
    :model-value="userData[0].name"
    :model-modifiers="{ trim: true }"
    :max-length="15"
    :min-length="1"
    trailing-icon="i-lucide-pencil"
    size="md"
    @update:model-value="userUpdateHandler"
  />
</template>
