<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ImageIcon, VideoIcon, SettingsIcon, Grid3X3Icon, FilmIcon, InfoIcon, FileTextIcon } from "lucide-vue-next";

const router = useRouter();
const route = useRoute();

// 使用计算属性实时同步路由状态
const currentRoute = computed(() => route.path);

function navigate(path: string) {
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
        @click="navigate('/video')"
        :class="[
          'p-3 rounded-lg transition-colors',
          currentRoute === '/video' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted',
        ]"
        title="生成视频"
      >
        <VideoIcon class="w-5 h-5" />
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
      <button
        @click="navigate('/video-gallery')"
        :class="[
          'p-3 rounded-lg transition-colors',
          currentRoute === '/video-gallery' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted',
        ]"
        title="视频库"
      >
        <FilmIcon class="w-5 h-5" />
      </button>
      <div class="flex-1"></div>
      <button
        @click="navigate('/logs')"
        :class="[
          'p-3 rounded-lg transition-colors',
          currentRoute === '/logs' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted',
        ]"
        title="查看日志"
      >
        <FileTextIcon class="w-5 h-5" />
      </button>
      <button
        @click="navigate('/about')"
        :class="[
          'p-3 rounded-lg transition-colors',
          currentRoute === '/about' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted',
        ]"
        title="关于"
      >
        <InfoIcon class="w-5 h-5" />
      </button>
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
