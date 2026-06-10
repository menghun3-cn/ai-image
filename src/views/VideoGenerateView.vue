<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import { generateVideo, getVideoOutputDir, openOutputDir } from "@/lib/tauri";
import { readFile } from "@tauri-apps/plugin-fs";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useVideoGenerationStore, durationPresets, resolutionPresets } from "@/stores/videoGeneration";
import ImageInput from "@/components/ImageInput.vue";
import {
  VideoIcon,
  Loader2Icon,
  FolderOpenIcon,
  AlertCircleIcon,
  Settings2Icon,
  XIcon,
  InfoIcon,
  CheckCircleIcon,
} from "lucide-vue-next";

const store = useVideoGenerationStore();

// 视频输出目录
const videoOutputDir = ref("video");

// Toast 通知
interface Toast {
  id: number;
  type: "info" | "success" | "error";
  message: string;
}
const toasts = ref<Toast[]>([]);
let toastId = 0;

function showToast(type: "info" | "success" | "error", message: string, duration = 3000) {
  const id = ++toastId;
  toasts.value.push({ id, type, message });
  if (duration > 0) {
    setTimeout(() => {
      removeToast(id);
    }, duration);
  }
}

function removeToast(id: number) {
  const index = toasts.value.findIndex((t) => t.id === id);
  if (index > -1) {
    toasts.value.splice(index, 1);
  }
}

// 视频 URL（用于预览）
const videoBlobUrl = ref<string | null>(null);
const videoPlayer = ref<HTMLVideoElement | null>(null);

// 加载视频
async function loadVideo(path: string | null) {
  if (!path) {
    videoBlobUrl.value = null;
    return;
  }

  try {
    const assetUrl = convertFileSrc(path);

    if (videoBlobUrl.value && videoBlobUrl.value.startsWith("blob:")) {
      URL.revokeObjectURL(videoBlobUrl.value);
    }

    videoBlobUrl.value = assetUrl;
  } catch (error) {
    console.error("[Video] 加载失败:", error);
    await loadVideoBlob(path);
  }
}

// 备用方案: 使用 Blob URL
async function loadVideoBlob(path: string | null) {
  if (!path) {
    videoBlobUrl.value = null;
    return;
  }

  try {
    const fileData = await readFile(path);
    const blob = new Blob([fileData], { type: "video/mp4" });
    const url = URL.createObjectURL(blob);

    if (videoBlobUrl.value && videoBlobUrl.value.startsWith("blob:")) {
      URL.revokeObjectURL(videoBlobUrl.value);
    }

    videoBlobUrl.value = url;
  } catch (error) {
    console.error("[Video] Blob 方案失败:", error);
    showToast("error", "视频预览加载失败");
    videoBlobUrl.value = null;
  }
}

// 监听视频路径变化
watch(
  () => store.resultVideoPath,
  (newPath) => {
    loadVideo(newPath);
  },
  { immediate: true }
);

// 视频加载错误处理
function handleVideoError(e: Event) {
  const videoEl = e.target as HTMLVideoElement;
  const error = videoEl.error;

  if (error && error.code === 4 && store.resultVideoPath) {
    loadVideoBlob(store.resultVideoPath);
    return;
  }

  showToast("error", "视频播放失败，请尝试打开目录查看");
}

// 监听进度事件
let unlistenProgress: UnlistenFn | null = null;
let progressTimer: ReturnType<typeof setInterval> | null = null;

onMounted(async () => {
  try {
    videoOutputDir.value = await getVideoOutputDir();

    // 从 localStorage 加载参考图片
    store.loadReferenceImages();

    // 如果页面刷新前有正在进行的生成，重置状态
    if (store.isGenerating) {
      store.generationFailed("生成被中断，请重新生成");
    }

    // 监听进度事件
    unlistenProgress = await listen("video-generation-progress", (event) => {
      const payload = event.payload as { status: string; progress: number };
      store.updateProgress(payload.progress);

      const statusUpper = payload.status.toUpperCase();
      if (statusUpper === "COMPLETED" || statusUpper === "SUCCESS") {
        store.setStatus("downloading");
      } else if (statusUpper === "FAILED" || statusUpper === "ERROR") {
        store.setStatus("error");
      } else if (statusUpper === "QUEUED" || statusUpper === "PENDING") {
        if (store.status === "creating") {
          store.setStatus("processing");
        }
      } else {
        store.setStatus("processing");
      }
    });

    // 启动进度定时器
    startProgressTimer();
  } catch (e) {
    console.error("初始化失败:", e);
  }
});

onUnmounted(() => {
  if (unlistenProgress) {
    unlistenProgress();
  }
  stopProgressTimer();
});

// 启动进度更新定时器
function startProgressTimer() {
  stopProgressTimer();
  progressTimer = setInterval(() => {
    if (store.isGenerating) {
      store.updateElapsedTime();
    }
  }, 1000);
}

// 停止进度更新定时器
function stopProgressTimer() {
  if (progressTimer) {
    clearInterval(progressTimer);
    progressTimer = null;
  }
}

// 处理图片数量超限
function handleMaxImagesReached() {
  showToast("info", "最多只能选择5张图片", 3000);
}

// 生成视频
async function handleGenerate() {
  if (!store.canGenerate) {
    showToast("info", "请输入视频描述提示词");
    return;
  }

  store.startGeneration();

  try {
    store.setStatus("processing");

    // 构建生成选项
    const options = store.getGenerationOptions();
    options.output_dir = videoOutputDir.value;

    const result = await generateVideo(options);

    if (result.success && result.video_path) {
      store.generationSuccess(result.video_path);
      showToast("success", `视频生成成功！耗时: ${store.formattedElapsedTime}`, 5000);
    } else {
      store.generationFailed(result.error || "生成失败");
      showToast("error", result.error || "生成失败");
    }
  } catch (e) {
    const errorMsg = String(e);
    store.generationFailed(errorMsg);
    showToast("error", errorMsg);
  }
}

// 打开输出目录
async function handleOpenOutputDir() {
  try {
    await openOutputDir(videoOutputDir.value);
  } catch (e) {
    showToast("error", String(e));
  }
}
</script>

<template>
  <div class="h-full flex flex-col p-6 relative">
    <!-- Toast Notifications -->
    <div class="fixed top-4 right-4 z-50 flex flex-col gap-2">
      <TransitionGroup name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          :class="[
            'flex items-center gap-3 px-4 py-3 rounded-lg shadow-lg border min-w-[300px] max-w-[500px]',
            toast.type === 'success' && 'bg-green-50 border-green-200 text-green-800 dark:bg-green-900/20 dark:border-green-800 dark:text-green-200',
            toast.type === 'error' && 'bg-red-50 border-red-200 text-red-800 dark:bg-red-900/20 dark:border-red-800 dark:text-red-200',
            toast.type === 'info' && 'bg-blue-50 border-blue-200 text-blue-800 dark:bg-blue-900/20 dark:border-blue-800 dark:text-blue-200',
          ]"
        >
          <CheckCircleIcon v-if="toast.type === 'success'" class="w-5 h-5 flex-shrink-0" />
          <AlertCircleIcon v-else-if="toast.type === 'error'" class="w-5 h-5 flex-shrink-0" />
          <InfoIcon v-else class="w-5 h-5 flex-shrink-0" />
          <span class="flex-1 text-sm">{{ toast.message }}</span>
          <button @click="removeToast(toast.id)" class="p-1 hover:bg-black/5 dark:hover:bg-white/10 rounded transition-colors">
            <XIcon class="w-4 h-4" />
          </button>
        </div>
      </TransitionGroup>
    </div>

    <!-- Header -->
    <div class="mb-6">
      <h1 class="text-2xl font-bold flex items-center gap-2">
        <VideoIcon class="w-6 h-6" />
        生成视频
      </h1>
      <p class="text-muted-foreground mt-1">使用 Agnes AI 将文字描述或图片转换为视频</p>
    </div>

    <!-- Main Content -->
    <div class="flex-1 flex gap-6 overflow-hidden">
      <!-- Left Panel - Input -->
      <div class="flex-1 flex flex-col gap-4 overflow-y-auto">
        <!-- Current Mode Display -->
        <div class="flex items-center justify-between p-3 bg-muted/50 rounded-lg">
          <div class="flex items-center gap-2">
            <span class="text-sm text-muted-foreground">当前模式:</span>
            <span class="text-sm font-medium px-2 py-1 bg-primary/10 text-primary rounded"> {{ store.modeDisplayText }} </span>
          </div>
          <span class="text-xs text-muted-foreground">
            {{ store.referenceImages.length === 0 ? "未选择图片" : `已选择 ${store.referenceImages.length} 张图片` }}
          </span>
        </div>

        <!-- Prompt Input -->
        <div class="flex flex-col gap-2">
          <label class="text-sm font-medium">视频描述</label>
          <textarea
            :value="store.prompt"
            @input="(e) => store.setPrompt((e.target as HTMLTextAreaElement).value)"
            rows="4"
            class="w-full px-3 py-2 border rounded-lg resize-none focus:outline-none focus:ring-2 focus:ring-primary"
            placeholder="描述你想要生成的视频内容..."
            :disabled="store.isGenerating"
          />
        </div>

        <!-- Image Upload Section - 使用 ImageInput 组件 -->
        <div class="flex flex-col gap-2">
          <div class="flex items-center justify-between">
            <label class="text-sm font-medium">参考图片 (可选)</label>
            <div class="flex items-center gap-2">
              <!-- 关键帧模式开关 (仅当图片>=2时显示) -->
              <label v-if="store.referenceImages.length >= 2" class="flex items-center gap-2 text-sm cursor-pointer">
                <input v-model="store.isKeyframesMode" type="checkbox" class="rounded border" :disabled="store.isGenerating" />
                <span class="text-muted-foreground">关键帧模式</span>
              </label>
              <button
                v-if="store.referenceImages.length > 0"
                @click="store.clearReferenceImages"
                class="text-xs text-destructive hover:underline"
                :disabled="store.isGenerating"
              >
                清空全部
              </button>
            </div>
          </div>

          <!-- 使用 ImageInput 组件 -->
          <ImageInput
            v-model="store.referenceImages"
            :disabled="store.isGenerating"
            :max-images="5"
            @max-reached="handleMaxImagesReached"
          />

          <!-- Mode Hint -->
          <p class="text-xs text-muted-foreground">
            <span v-if="store.referenceImages.length === 1">已选择1张图片，将使用单图生视频模式</span>
            <span v-else-if="store.referenceImages.length >= 2">
              已选择 {{ store.referenceImages.length }} 张图片，将使用{{ store.isKeyframesMode ? "关键帧" : "多图" }}模式
            </span>
          </p>
        </div>

        <!-- Duration Selection -->
        <div class="flex flex-col gap-2">
          <label class="text-sm font-medium">
            视频时长 (约{{ store.videoDuration }}秒)
          </label>
          <div class="flex gap-2 flex-wrap">
            <button
              v-for="(preset, index) in durationPresets"
              :key="index"
              @click="store.setDurationPreset(index)"
              :disabled="store.isGenerating"
              :class="[
                'px-4 py-2 rounded-lg text-sm border transition-colors',
                store.selectedDurationIndex === index
                  ? 'bg-primary text-primary-foreground border-primary'
                  : 'hover:bg-muted',
              ]"
            >
              {{ preset.label }}
            </button>
          </div>
        </div>

        <!-- Resolution Selection -->
        <div class="flex flex-col gap-2">
          <label class="text-sm font-medium">分辨率</label>
          <div class="flex gap-2 flex-wrap">
            <button
              v-for="(preset, index) in resolutionPresets"
              :key="index"
              @click="store.setResolutionPreset(index)"
              :disabled="store.isGenerating"
              :class="[
                'px-4 py-2 rounded-lg text-sm border transition-colors',
                store.selectedResolutionIndex === index
                  ? 'bg-primary text-primary-foreground border-primary'
                  : 'hover:bg-muted',
              ]"
            >
              {{ preset.label }}
            </button>
          </div>
        </div>

        <!-- Advanced Settings -->
        <div class="border rounded-lg overflow-hidden">
          <button
            @click="store.setShowAdvanced(!store.showAdvanced)"
            class="w-full px-4 py-3 flex items-center justify-between hover:bg-muted/50 transition-colors"
            :disabled="store.isGenerating"
          >
            <div class="flex items-center gap-2">
              <Settings2Icon class="w-4 h-4" />
              <span class="text-sm font-medium">高级设置</span>
            </div>
            <span class="text-muted-foreground">{{ store.showAdvanced ? "收起" : "展开" }}</span>
          </button>

          <div v-if="store.showAdvanced" class="px-4 pb-4 space-y-4 border-t">
            <!-- Seed -->
            <div class="pt-4">
              <label class="block text-sm font-medium mb-2">随机种子 (可选)</label>
              <input
                :value="store.seed"
                @input="(e) => store.setSeed((e.target as HTMLInputElement).value ? parseInt((e.target as HTMLInputElement).value) : undefined)"
                type="number"
                placeholder="留空则随机生成"
                class="w-full px-3 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary"
                :disabled="store.isGenerating"
              />
            </div>

            <!-- Negative Prompt -->
            <div>
              <label class="block text-sm font-medium mb-2">反向提示词 (可选)</label>
              <textarea
                :value="store.negativePrompt"
                @input="(e) => store.setNegativePrompt((e.target as HTMLTextAreaElement).value)"
                rows="2"
                class="w-full px-3 py-2 border rounded-lg resize-none focus:outline-none focus:ring-2 focus:ring-primary"
                placeholder="描述你不希望出现在视频中的内容..."
                :disabled="store.isGenerating"
              />
            </div>
          </div>
        </div>

        <!-- Generate Button -->
        <button
          @click="handleGenerate"
          :disabled="!store.canGenerate"
          class="w-full py-3 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50 flex items-center justify-center gap-2"
        >
          <Loader2Icon v-if="store.isGenerating" class="w-5 h-5 animate-spin" />
          <VideoIcon v-else class="w-5 h-5" />
          {{ store.isGenerating ? store.statusText : `生成视频 (${store.modeDisplayText})` }}
        </button>

        <!-- Error Message -->
        <div v-if="store.errorMessage" class="p-3 bg-red-50 border border-red-200 rounded-lg text-red-800 text-sm">
          <div class="flex items-start gap-2">
            <AlertCircleIcon class="w-5 h-5 flex-shrink-0 mt-0.5" />
            <div>
              <p class="font-medium">生成失败</p>
              <p class="mt-1">{{ store.errorMessage }}</p>
            </div>
          </div>
        </div>
      </div>

      <!-- Right Panel - Preview -->
      <div class="w-[480px] flex flex-col gap-4">
        <div class="flex-1 bg-black rounded-lg overflow-hidden relative">
          <!-- Video Player -->
          <video
            v-if="videoBlobUrl"
            ref="videoPlayer"
            :src="videoBlobUrl"
            controls
            class="w-full h-full object-contain"
            @error="handleVideoError"
          />

          <!-- Empty State -->
          <div v-else class="w-full h-full flex flex-col items-center justify-center text-white/60">
            <VideoIcon class="w-16 h-16 mb-4 opacity-50" />
            <p class="text-sm">生成的视频将在这里显示</p>
          </div>

          <!-- Loading Overlay -->
          <div
            v-if="store.isGenerating"
            class="absolute inset-0 bg-black/80 flex flex-col items-center justify-center text-white"
          >
            <Loader2Icon class="w-12 h-12 animate-spin mb-4" />
            <p class="text-lg font-medium">{{ store.statusText }}</p>
            <div v-if="store.progress > 0" class="mt-4 w-64 h-2 bg-white/20 rounded-full overflow-hidden">
              <div class="h-full bg-primary transition-all duration-300" :style="{ width: `${store.progress}%` }" />
            </div>
          </div>
        </div>

        <!-- Video Info & Actions -->
        <div v-if="store.resultVideoPath" class="bg-muted/50 rounded-lg p-4 space-y-3">
          <div class="flex items-center gap-2 text-sm">
            <CheckCircleIcon class="w-4 h-4 text-green-500" />
            <span class="font-medium">生成成功</span>
            <span class="text-muted-foreground">({{ store.formattedElapsedTime }})</span>
          </div>

          <div class="text-xs text-muted-foreground break-all">{{ store.resultVideoPath }}</div>

          <div class="flex gap-2">
            <button
              @click="handleOpenOutputDir"
              class="flex-1 flex items-center justify-center gap-2 px-4 py-2 border rounded-lg hover:bg-muted transition-colors text-sm"
            >
              <FolderOpenIcon class="w-4 h-4" />
              打开目录
            </button>
          </div>
        </div>

        <!-- Video Parameters Info -->
        <div class="bg-muted/30 rounded-lg p-4 text-sm space-y-2">
          <h4 class="font-medium flex items-center gap-2">
            <InfoIcon class="w-4 h-4" />
            当前参数
          </h4>
          <div class="grid grid-cols-2 gap-2 text-xs text-muted-foreground">
            <div>模式: {{ store.modeDisplayText }}</div>
            <div>时长: {{ store.videoDuration }}秒</div>
            <div>分辨率: {{ store.width }}×{{ store.height }}</div>
            <div>帧率: {{ store.frameRate }}fps</div>
            <div>帧数: {{ store.numFrames }}</div>
            <div>图片数: {{ store.referenceImages.length }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* Toast 动画 */
.toast-enter-active,
.toast-leave-active {
  transition: all 0.3s ease;
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(100%);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(100%);
}
</style>
