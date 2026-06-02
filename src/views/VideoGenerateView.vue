<script setup lang="ts">
import { ref, onMounted, computed, watch, onUnmounted } from "vue";
import { generateVideo, getVideoOutputDir, openOutputDir, type VideoGenerationResult } from "@/lib/tauri";
import { readFile } from "@tauri-apps/plugin-fs";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { convertFileSrc } from "@tauri-apps/api/core";
import { VideoIcon, Loader2Icon, FolderOpenIcon, PlayIcon, AlertCircleIcon, Settings2Icon, XIcon, InfoIcon, CheckCircleIcon } from "lucide-vue-next";

// 从 localStorage 恢复状态
const savedState = localStorage.getItem("videoGenerationState");
const parsedState = savedState ? JSON.parse(savedState) : null;

const prompt = ref(localStorage.getItem("lastVideoPrompt") || "");
const isGenerating = ref(parsedState?.isGenerating || false);
const generationStatus = ref<"idle" | "creating" | "processing" | "downloading" | "success" | "error">(parsedState?.generationStatus || "idle");
const resultVideoPath = ref<string | null>(parsedState?.resultVideoPath || null);
const errorMessage = ref<string | null>(parsedState?.errorMessage || null);
const generationProgress = ref(parsedState?.generationProgress || 0);
const videoOutputDir = ref("video");
const showAdvanced = ref(false);

// 保存状态到 localStorage
function saveState() {
  localStorage.setItem("videoGenerationState", JSON.stringify({
    isGenerating: isGenerating.value,
    generationStatus: generationStatus.value,
    resultVideoPath: resultVideoPath.value,
    errorMessage: errorMessage.value,
    elapsedTime: elapsedTime.value,
    startTime: startTime.value,
    generationProgress: generationProgress.value,
  }));
}

// 监听状态变化并保存
watch([isGenerating, generationStatus, resultVideoPath, errorMessage], saveState, { deep: true });

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
  const index = toasts.value.findIndex(t => t.id === id);
  if (index > -1) {
    toasts.value.splice(index, 1);
  }
}

// 生成时间统计
const startTime = ref<number | null>(null);
const elapsedTime = ref(0);
const elapsedTimeFormatted = computed(() => {
  const minutes = Math.floor(elapsedTime.value / 60);
  const seconds = elapsedTime.value % 60;
  return minutes > 0 ? `${minutes}分${seconds}秒` : `${seconds}秒`;
});
let timerInterval: ReturnType<typeof setInterval> | null = null;

// 视频参数
const width = ref(1152);
const height = ref(768);
const numFrames = ref(121); // 默认约 5 秒 (121帧 / 24fps ≈ 5s)
const frameRate = ref(24);
const seed = ref<number | undefined>(undefined);
const negativePrompt = ref("");

// 预设时长选项
const durationPresets = [
  { label: "3 秒", frames: 73, fps: 24 },
  { label: "5 秒", frames: 121, fps: 24 },
  { label: "10 秒", frames: 241, fps: 24 },
];

const selectedDuration = ref(1); // 默认 5 秒

// 分辨率选项
const resolutionPresets = [
  { label: "1152 x 768 (16:9)", width: 1152, height: 768 },
  { label: "768 x 1152 (9:16)", width: 768, height: 1152 },
  { label: "1024 x 1024 (1:1)", width: 1024, height: 1024 },
];

const selectedResolution = ref(0); // 默认 16:9

// 计算实际视频时长
const videoDuration = computed(() => {
  return (numFrames.value / frameRate.value).toFixed(1);
});

// 视频 URL（用于预览）
const videoBlobUrl = ref<string | null>(null);
const videoPlayer = ref<HTMLVideoElement | null>(null);

// 加载视频 - 方案1: 使用 convertFileSrc (推荐，性能更好)
async function loadVideo(path: string | null) {
  if (!path) {
    videoBlobUrl.value = null;
    return;
  }
  
  try {
    console.log("[Video] ========== 开始加载视频 ==========");
    console.log("[Video] 原始路径:", path);
    
    // 检查文件是否存在
    try {
      const fileData = await readFile(path);
      console.log("[Video] 文件存在，大小:", fileData.length, "bytes");
      console.log("[Video] 文件前20字节:", Array.from(fileData.slice(0, 20)).map(b => b.toString(16).padStart(2, '0')).join('-'));
    } catch (e) {
      console.error("[Video] 文件读取失败:", e);
    }
    
    // 方案1: 使用 Tauri 的 convertFileSrc 转换本地文件路径
    const assetUrl = convertFileSrc(path);
    console.log("[Video] convertFileSrc 结果:", assetUrl);
    console.log("[Video] URL 协议:", assetUrl.split(':')[0]);
    
    // 释放旧的 URL
    if (videoBlobUrl.value && videoBlobUrl.value.startsWith('blob:')) {
      URL.revokeObjectURL(videoBlobUrl.value);
    }
    
    videoBlobUrl.value = assetUrl;
    console.log("[Video] videoBlobUrl 已设置为:", videoBlobUrl.value);
    
  } catch (error) {
    console.error("[Video] convertFileSrc 方案失败，尝试 Blob 方案:", error);
    await loadVideoBlob(path);
  }
}

// 备用方案: 使用 Blob URL (如果 convertFileSrc 失败)
async function loadVideoBlob(path: string | null) {
  if (!path) {
    videoBlobUrl.value = null;
    return;
  }
  
  try {
    console.log("[Video] 使用 Blob 方案加载视频:", path);
    const fileData = await readFile(path);
    console.log("[Video] 文件读取成功，大小:", fileData.length, "bytes");
    
    const blob = new Blob([fileData], { type: 'video/mp4' });
    const url = URL.createObjectURL(blob);
    console.log("[Video] Blob URL 创建成功:", url);
    
    if (videoBlobUrl.value && videoBlobUrl.value.startsWith('blob:')) {
      URL.revokeObjectURL(videoBlobUrl.value);
    }
    
    videoBlobUrl.value = url;
  } catch (error) {
    console.error("[Video] Blob 方案失败:", error);
    showToast("error", "视频预览加载失败");
    videoBlobUrl.value = null;
  }
}

// 监听视频路径变化，自动加载
watch(resultVideoPath, (newPath) => {
  loadVideo(newPath);
}, { immediate: true });

// 视频加载错误处理
function handleVideoError(e: Event) {
  console.error("[Video] ========== 视频播放错误 ==========");
  const videoEl = e.target as HTMLVideoElement;
  const error = videoEl.error;
  
  console.error("[Video] 视频 src:", videoEl.src);
  console.error("[Video] 视频 currentSrc:", videoEl.currentSrc);
  console.error("[Video] 视频 networkState:", videoEl.networkState);
  console.error("[Video] 视频 readyState:", videoEl.readyState);
  
  if (error) {
    console.error("[Video] 错误代码:", error.code);
    console.error("[Video] 错误信息:", error.message);
    
    // 尝试用 Blob 方案重试
    if (error.code === 4 && resultVideoPath.value) {
      console.log("[Video] 格式不支持，尝试使用 Blob 方案重试...");
      loadVideoBlob(resultVideoPath.value);
      return;
    }
    
    let errorMsg = "视频播放失败";
    switch (error.code) {
      case 1:
        errorMsg = "视频加载被中止";
        break;
      case 2:
        errorMsg = "网络错误，无法加载视频";
        break;
      case 3:
        errorMsg = "视频解码错误，文件可能损坏";
        break;
      case 4:
        errorMsg = "视频格式不支持";
        break;
    }
    showToast("error", errorMsg + "，请尝试打开目录查看");
  } else {
    console.error("[Video] 无错误对象");
    showToast("error", "视频播放失败，请尝试打开目录查看");
  }
}
// 监听进度事件的 unlisten 函数
let unlistenProgress: UnlistenFn | null = null;

onMounted(async () => {
  try {
    videoOutputDir.value = await getVideoOutputDir();
    
    // 如果之前正在生成，标记为中断
    if (isGenerating.value) {
      isGenerating.value = false;
      generationStatus.value = "error";
      errorMessage.value = "生成被中断，请重新生成";
      saveState();
    }
    
    // 监听视频生成进度事件
    unlistenProgress = await listen("video-generation-progress", (event) => {
      const payload = event.payload as { status: string; progress: number };
      console.log("[Video] 收到进度事件:", JSON.stringify(payload));
      console.log("[Video] 当前状态:", generationStatus.value, "进度:", generationProgress.value);
      
      // 更新进度
      generationProgress.value = payload.progress;
      console.log("[Video] 进度已更新为:", generationProgress.value);
      
      // 根据状态更新 generationStatus
      const statusUpper = payload.status.toUpperCase();
      console.log("[Video] 处理状态:", statusUpper);
      
      if (statusUpper === "COMPLETED" || statusUpper === "SUCCESS") {
        generationStatus.value = "downloading";
      } else if (statusUpper === "FAILED" || statusUpper === "ERROR" || statusUpper === "FAILURE") {
        generationStatus.value = "error";
      } else if (statusUpper === "QUEUED" || statusUpper === "PENDING" || statusUpper === "CREATED") {
        // 排队中，切换到 processing 状态以显示进度
        if (generationStatus.value === "creating") {
          generationStatus.value = "processing";
          console.log("[Video] 状态已切换到 processing");
        }
      } else {
        // PROCESSING, RUNNING, IN_PROGRESS 等
        generationStatus.value = "processing";
      }
    });
  } catch (e) {
    console.error("获取视频输出目录失败:", e);
  }
});

onUnmounted(() => {
  // 清理事件监听
  if (unlistenProgress) {
    unlistenProgress();
  }
});

function updatePrompt(value: string) {
  prompt.value = value;
  localStorage.setItem("lastVideoPrompt", value);
}

function applyDurationPreset(index: number) {
  selectedDuration.value = index;
  const preset = durationPresets[index];
  numFrames.value = preset.frames;
  frameRate.value = preset.fps;
}

function applyResolutionPreset(index: number) {
  selectedResolution.value = index;
  const preset = resolutionPresets[index];
  width.value = preset.width;
  height.value = preset.height;
}

async function handleGenerate() {
  if (!prompt.value.trim()) {
    showToast("info", "请输入视频描述提示词");
    return;
  }

  isGenerating.value = true;
  generationStatus.value = "creating";
  errorMessage.value = null;
  resultVideoPath.value = null;
  generationProgress.value = 0; // 重置进度

  // 开始计时
  startTime.value = Date.now();
  elapsedTime.value = 0;
  timerInterval = setInterval(() => {
    if (startTime.value) {
      elapsedTime.value = Math.floor((Date.now() - startTime.value) / 1000);
    }
  }, 1000);

  try {
    generationStatus.value = "processing";
    
    // 使用 Promise 让出控制权，确保事件监听器可以处理事件
    const result = await new Promise<VideoGenerationResult>((resolve) => {
      setTimeout(async () => {
        const r = await generateVideo({
          prompt: prompt.value.trim(),
          output_dir: videoOutputDir.value,
          width: width.value,
          height: height.value,
          num_frames: numFrames.value,
          frame_rate: frameRate.value,
          seed: seed.value,
          negative_prompt: negativePrompt.value || undefined,
        });
        resolve(r);
      }, 100);
    });

    if (result.success && result.video_path) {
      generationStatus.value = "success";
      resultVideoPath.value = result.video_path;
      showToast("success", `视频生成成功！耗时: ${elapsedTimeFormatted.value}`, 5000);
    } else {
      generationStatus.value = "error";
      errorMessage.value = result.error || "生成失败";
      showToast("error", errorMessage.value);
    }
  } catch (e) {
    generationStatus.value = "error";
    errorMessage.value = String(e);
    showToast("error", String(e));
  } finally {
    isGenerating.value = false;
    // 停止计时
    if (timerInterval) {
      clearInterval(timerInterval);
      timerInterval = null;
    }
  }
}

async function handleOpenOutputDir() {
  try {
    await openOutputDir(videoOutputDir.value);
  } catch (e) {
    showToast("error", String(e));
  }
}

function getStatusText() {
  const timeStr = elapsedTime.value > 0 ? ` (${elapsedTimeFormatted.value})` : "";
  // 只要有进度就显示，包括0%
  const progressStr = isGenerating.value ? ` ${generationProgress.value}%` : "";
  console.log("[Video] getStatusText:", generationStatus.value, "进度:", generationProgress.value, "显示:", progressStr);
  switch (generationStatus.value) {
    case "creating":
      return `创建任务中...${timeStr}`;
    case "processing":
      return `视频生成中${progressStr}（这可能需要几分钟）...${timeStr}`;
    case "downloading":
      return `下载视频中...${timeStr}`;
    case "success":
      return `生成成功！总耗时: ${elapsedTimeFormatted.value}`;
    case "error":
      return `生成失败${timeStr}`;
    default:
      return "";
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
            toast.type === 'info' && 'bg-blue-50 border-blue-200 text-blue-800 dark:bg-blue-900/20 dark:border-blue-800 dark:text-blue-200'
          ]"
        >
          <CheckCircleIcon v-if="toast.type === 'success'" class="w-5 h-5 flex-shrink-0" />
          <AlertCircleIcon v-else-if="toast.type === 'error'" class="w-5 h-5 flex-shrink-0" />
          <InfoIcon v-else class="w-5 h-5 flex-shrink-0" />
          <span class="flex-1 text-sm">{{ toast.message }}</span>
          <button
            @click="removeToast(toast.id)"
            class="p-1 hover:bg-black/5 dark:hover:bg-white/10 rounded transition-colors"
          >
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
      <p class="text-muted-foreground mt-1">
        使用 Agnes AI 将文字描述转换为视频
      </p>
    </div>

    <!-- Main Content -->
    <div class="flex-1 flex gap-6 overflow-hidden">
      <!-- Left Panel - Input -->
      <div class="flex-1 flex flex-col gap-4 overflow-y-auto">
        <!-- Prompt Input -->
        <div class="flex flex-col gap-2">
          <label class="text-sm font-medium">视频描述</label>
          <textarea
            v-model="prompt"
            @input="updatePrompt(($event.target as HTMLTextAreaElement).value)"
            placeholder="描述你想要生成的视频内容，例如：一只可爱的猫咪在草地上玩耍..."
            class="min-h-[120px] p-4 rounded-lg border bg-background resize-none focus:outline-none focus:ring-2 focus:ring-primary"
            :disabled="isGenerating"
          />
        </div>

        <!-- Duration Presets -->
        <div class="flex flex-col gap-2">
          <label class="text-sm font-medium">视频时长 (约 {{ videoDuration }} 秒)</label>
          <div class="flex gap-2">
            <button
              v-for="(preset, index) in durationPresets"
              :key="index"
              @click="applyDurationPreset(index)"
              :class="[
                'px-4 py-2 rounded-lg border text-sm transition-colors',
                selectedDuration === index
                  ? 'bg-primary text-primary-foreground border-primary'
                  : 'bg-background hover:bg-muted'
              ]"
              :disabled="isGenerating"
            >
              {{ preset.label }}
            </button>
          </div>
        </div>

        <!-- Resolution Presets -->
        <div class="flex flex-col gap-2">
          <label class="text-sm font-medium">分辨率</label>
          <div class="flex gap-2 flex-wrap">
            <button
              v-for="(preset, index) in resolutionPresets"
              :key="index"
              @click="applyResolutionPreset(index)"
              :class="[
                'px-4 py-2 rounded-lg border text-sm transition-colors',
                selectedResolution === index
                  ? 'bg-primary text-primary-foreground border-primary'
                  : 'bg-background hover:bg-muted'
              ]"
              :disabled="isGenerating"
            >
              {{ preset.label }}
            </button>
          </div>
        </div>

        <!-- Advanced Settings -->
        <div class="border rounded-lg">
          <button
            @click="showAdvanced = !showAdvanced"
            class="w-full px-4 py-3 flex items-center justify-between hover:bg-muted/50 transition-colors"
          >
            <div class="flex items-center gap-2">
              <Settings2Icon class="w-4 h-4" />
              <span class="text-sm font-medium">高级设置</span>
            </div>
            <span class="text-muted-foreground">{{ showAdvanced ? '▼' : '▶' }}</span>
          </button>
          
          <div v-if="showAdvanced" class="p-4 border-t space-y-4">
            <!-- Frame Rate -->
            <div class="flex items-center gap-4">
              <label class="text-sm w-20">帧率 (FPS)</label>
              <input
                v-model.number="frameRate"
                type="number"
                min="1"
                max="60"
                class="flex-1 px-3 py-2 rounded-lg border bg-background"
                :disabled="isGenerating"
              />
              <span class="text-sm text-muted-foreground w-16">1-60</span>
            </div>

            <!-- Num Frames -->
            <div class="flex items-center gap-4">
              <label class="text-sm w-20">帧数</label>
              <input
                v-model.number="numFrames"
                type="number"
                min="1"
                max="441"
                class="flex-1 px-3 py-2 rounded-lg border bg-background"
                :disabled="isGenerating"
              />
              <span class="text-sm text-muted-foreground w-16">≤441</span>
            </div>

            <!-- Seed -->
            <div class="flex items-center gap-4">
              <label class="text-sm w-20">随机种子</label>
              <input
                v-model.number="seed"
                type="number"
                placeholder="随机"
                class="flex-1 px-3 py-2 rounded-lg border bg-background"
                :disabled="isGenerating"
              />
              <button
                @click="seed = undefined"
                class="text-sm text-muted-foreground hover:text-foreground w-16"
                :disabled="isGenerating"
              >
                清除
              </button>
            </div>

            <!-- Negative Prompt -->
            <div class="flex flex-col gap-2">
              <label class="text-sm">负向提示词</label>
              <input
                v-model="negativePrompt"
                type="text"
                placeholder="描述需要避免的内容..."
                class="px-3 py-2 rounded-lg border bg-background"
                :disabled="isGenerating"
              />
            </div>
          </div>
        </div>

        <!-- Generate Button -->
        <button
          @click="handleGenerate"
          :disabled="isGenerating || !prompt.trim()"
          class="px-6 py-3 bg-primary text-primary-foreground rounded-lg font-medium flex items-center justify-center gap-2 hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          <Loader2Icon v-if="isGenerating" class="w-5 h-5 animate-spin" />
          <VideoIcon v-else class="w-5 h-5" />
          {{ isGenerating ? "生成中..." : "生成视频" }}
        </button>

        <!-- Status -->
        <div v-if="generationStatus !== 'idle'" class="p-4 rounded-lg bg-muted">
          <div class="flex items-center gap-2">
            <Loader2Icon v-if="isGenerating" class="w-5 h-5 animate-spin text-primary" />
            <AlertCircleIcon v-else-if="generationStatus === 'error'" class="w-5 h-5 text-destructive" />
            <PlayIcon v-else-if="generationStatus === 'success'" class="w-5 h-5 text-green-500" />
            <span :class="{
              'text-primary': isGenerating,
              'text-destructive': generationStatus === 'error',
              'text-green-500': generationStatus === 'success'
            }">
              {{ getStatusText() }}
            </span>
          </div>
          <p v-if="errorMessage" class="text-sm text-destructive mt-2">
            {{ errorMessage }}
          </p>
        </div>

        <!-- Output Directory -->
        <div class="flex items-center gap-2 text-sm text-muted-foreground">
          <span>输出目录: {{ videoOutputDir }}</span>
          <button
            @click="handleOpenOutputDir"
            class="p-1 hover:bg-muted rounded transition-colors"
            title="打开目录"
          >
            <FolderOpenIcon class="w-4 h-4" />
          </button>
        </div>
      </div>

      <!-- Right Panel - Result -->
      <div class="w-[400px] flex flex-col gap-4">
        <div class="flex-1 border rounded-lg bg-muted/50 flex flex-col items-center justify-center p-4 min-h-[300px]">
          <template v-if="videoBlobUrl">
            <div class="w-full h-full flex flex-col">
              <video
                ref="videoPlayer"
                :src="videoBlobUrl"
                controls
                preload="auto"
                playsinline
                class="max-w-full max-h-full rounded-lg flex-1"
                @error="handleVideoError"
                @loadeddata="console.log('[Video] 视频数据已加载')"
                @canplay="console.log('[Video] 视频可以播放')"
              />
              <p class="text-xs text-muted-foreground mt-2 break-all">{{ resultVideoPath }}</p>
            </div>
          </template>
          <template v-else>
            <VideoIcon class="w-16 h-16 text-muted-foreground/50 mb-4" />
            <p class="text-muted-foreground text-center">
              生成的视频将在这里显示
            </p>
          </template>
        </div>

        <!-- Info Card -->
        <div class="p-4 border rounded-lg bg-card">
          <h3 class="font-medium mb-2">关于视频生成</h3>
          <ul class="text-sm text-muted-foreground space-y-1">
            <li>• 使用 Agnes AI 视频模型</li>
            <li>• 生成时间可能需要几分钟</li>
            <li>• 视频保存为 MP4 格式</li>
            <li>• 需要配置 Agnes API Key</li>
            <li>• 帧数必须满足 8n + 1 公式</li>
          </ul>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
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
