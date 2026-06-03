<script setup lang="ts">
import { XIcon, AlertTriangleIcon, InfoIcon, CheckCircleIcon } from "lucide-vue-next";

interface Props {
  show: boolean;
  title: string;
  message: string;
  type?: "info" | "warning" | "error" | "success";
  confirmText?: string;
  cancelText?: string;
  showCancel?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  type: "info",
  confirmText: "确定",
  cancelText: "取消",
  showCancel: false,
});

const emit = defineEmits<{
  confirm: [];
  cancel: [];
}>();

const iconMap = {
  info: InfoIcon,
  warning: AlertTriangleIcon,
  error: AlertTriangleIcon,
  success: CheckCircleIcon,
};

const colorMap = {
  info: "text-blue-500",
  warning: "text-amber-500",
  error: "text-red-500",
  success: "text-green-500",
};

const buttonColorMap = {
  info: "bg-primary hover:bg-primary/90",
  warning: "bg-amber-500 hover:bg-amber-600",
  error: "bg-red-500 hover:bg-red-600",
  success: "bg-green-500 hover:bg-green-600",
};

function handleConfirm() {
  emit("confirm");
}

function handleCancel() {
  emit("cancel");
}

function handleOverlayClick() {
  if (!props.showCancel) {
    emit("confirm");
  } else {
    emit("cancel");
  }
}
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div
        v-if="show"
        class="fixed inset-0 z-50 flex items-center justify-center p-4"
        @click.self="handleOverlayClick"
      >
        <!-- Backdrop -->
        <div class="absolute inset-0 bg-black/50 backdrop-blur-sm" />

        <!-- Dialog -->
        <div
          class="relative w-full max-w-md rounded-xl border bg-card p-6 shadow-2xl"
          @click.stop
        >
          <!-- Close button -->
          <button
            v-if="!showCancel"
            class="absolute right-4 top-4 rounded-lg p-1 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
            @click="handleConfirm"
          >
            <XIcon class="h-4 w-4" />
          </button>

          <!-- Content -->
          <div class="flex items-start gap-4">
            <div :class="['mt-0.5 shrink-0', colorMap[type]]">
              <component :is="iconMap[type]" class="h-6 w-6" />
            </div>
            <div class="flex-1">
              <h3 class="text-lg font-semibold">{{ title }}</h3>
              <p class="mt-2 text-sm text-muted-foreground">{{ message }}</p>
            </div>
          </div>

          <!-- Actions -->
          <div class="mt-6 flex justify-end gap-3">
            <button
              v-if="showCancel"
              class="rounded-lg border px-4 py-2 text-sm font-medium transition-colors hover:bg-muted"
              @click="handleCancel"
            >
              {{ cancelText }}
            </button>
            <button
              :class="['rounded-lg px-4 py-2 text-sm font-medium text-white transition-colors', buttonColorMap[type]]"
              @click="handleConfirm"
            >
              {{ confirmText }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
