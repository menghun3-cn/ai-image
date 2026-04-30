<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { ImageIcon, SettingsIcon, Grid3X3Icon } from "lucide-vue-next";

const router = useRouter();
const currentRoute = ref("/");

onMounted(() => {
  currentRoute.value = router.currentRoute.value.path;
});

function navigate(path: string) {
  currentRoute.value = path;
  router.push(path);
}
</script>

<template>
  <div class="flex h-screen bg-background">
    <!-- Sidebar -->
    <aside class="w-16 border-r bg-card flex flex-col items-center py-4 gap-2">
      <button
        @click="navigate('/')"
        :class="[
          'p-3 rounded-lg transition-colors',
          currentRoute === '/' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted',
        ]"
        title="生成图片"
      >
        <ImageIcon class="w-5 h-5" />
      </button>
      <button
        @click="navigate('/gallery')"
        :class="[
          'p-3 rounded-lg transition-colors',
          currentRoute === '/gallery' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted',
        ]"
        title="图库"
      >
        <Grid3X3Icon class="w-5 h-5" />
      </button>
      <div class="flex-1"></div>
      <button
        @click="navigate('/settings')"
        :class="[
          'p-3 rounded-lg transition-colors',
          currentRoute === '/settings' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted',
        ]"
        title="设置"
      >
        <SettingsIcon class="w-5 h-5" />
      </button>
    </aside>

    <!-- Main Content -->
    <main class="flex-1 overflow-auto">
      <router-view />
    </main>
  </div>
</template>
