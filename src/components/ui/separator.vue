<script setup lang="ts">
import { computed } from "vue";
import { cn } from "@/lib/utils";

interface Props {
  class?: string;
  orientation?: "horizontal" | "vertical";
}

const props = withDefaults(defineProps<Props>(), {
  orientation: "horizontal",
});

const classes = computed(() => {
  return cn(
    "shrink-0 bg-border",
    props.orientation === "horizontal" ? "h-[1px] w-full" : "h-full w-[1px]",
    props.class
  );
});
</script>

<template>
  <div :class="classes" role="separator" :aria-orientation="orientation" />
</template>
