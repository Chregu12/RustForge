<script setup lang="ts">
import { computed, type HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'

interface Props {
  class?: HTMLAttributes['class']
}

const props = defineProps<Props>()

const classes = computed(() =>
  cn('rounded-lg border bg-card text-card-foreground shadow-sm', props.class)
)
</script>

<template>
  <div :class="classes">
    <slot />
  </div>
</template>
