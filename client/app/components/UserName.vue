<script setup lang="ts">
import { useTable } from 'spacetimedb/vue'
import { tables } from '~/spacetimedb'

const [userData] = useTable(tables.me)
</script>

<template>
  <div>
    {{ userData[0]?.id }}
  </div>
</template>
