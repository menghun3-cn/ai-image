<script setup lang="ts">
import { ref, onMounted } from "vue";
import { getLogContent } from "@/lib/tauri";
import { RefreshCwIcon, Trash2Icon, FolderOpenIcon } from "lucide-vue-next";
import { openLogDir } from "@/lib/tauri";

const logContent = ref<string>("");
const isLoading = ref(false);
const error = ref<string | null>(null);

async function loadLogContent() {
  isLoading.value = true;
  error.value = null;
  try {
    logContent.value = await getLogContent();
  } catch (e) {
    error.value = String(e);
    console.error("加载日志失败:", e);
  } finally {
    isLoading.value = false;
  }
}

async function handleOpenLogDir() {
  try {
    await openLogDir();
  } catch (e) {
    console.error("打开日志目录失败:", e);
  }
}

function clearLog() {
  logContent.value = "";
}

onMounted(() => {
  loadLogContent();
});
</script>

<template>
  <div class="h-full flex flex-col">
    <!-- 头部 -->
    <div class="flex items-center justify-between p-4 border-b bg-card">
      <div>
        <h1 class="text-xl font-semibold">日志查看</h1>
        <p class="text-sm text-muted-foreground mt-1">查看应用程序运行日志</p>
      </div>
      <div class="flex items-center gap-2">
        <button
          @click="handleOpenLogDir"
          class="flex items-center gap-1.5 px-3 py-2 text-sm font-medium bg-muted hover:bg-muted/80 border rounded-md transition-colors"
        >
          <FolderOpenIcon class="w-4 h-4" />
          打开目录
        </button>
        <button
          @click="clearLog"
          class="flex items-center gap-1.5 px-3 py-2 text-sm font-medium bg-muted hover:bg-muted/80 border rounded-md transition-colors"
        >
          <Trash2Icon class="w-4 h-4" />
          清空显示
        </button>
        <button
          @click="loadLogContent"
          :disabled="isLoading"
          class="flex items-center gap-1.5 px-3 py-2 text-sm font-medium bg-primary text-primary-foreground hover:bg-primary/90 rounded-md transition-colors disabled:opacity-50"
        >
          <RefreshCwIcon class="w-4 h-4" :class="{ 'animate-spin': isLoading }" />
          刷新
        </button>
      </div>
    </div>

    <!-- 日志内容 -->
    <div class="flex-1 overflow-auto p-4">
      <div v-if="error" class="p-4 rounded-lg border border-destructive/50 bg-destructive/10 text-destructive">
        <p class="font-medium">加载日志失败</p>
        <p class="text-sm mt-1">{{ error }}</p>
      </div>
      
      <div v-else-if="!logContent" class="flex flex-col items-center justify-center h-full text-muted-foreground">
        <p>暂无日志内容</p>
        <button
          @click="loadLogContent"
          class="mt-4 px-4 py-2 text-sm font-medium bg-primary text-primary-foreground hover:bg-primary/90 rounded-md transition-colors"
        >
          加载日志
        </button>
      </div>
      
      <pre v-else class="font-mono text-sm whitespace-pre-wrap break-all bg-muted p-4 rounded-lg min-h-full">{{ logContent }}</pre>
    </div>
  </div>
</template>
