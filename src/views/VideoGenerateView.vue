<script setup lang="ts">
import { ref, onMounted, computed, watch, onUnmounted } from "vue";
import { generateVideo, getVideoOutputDir, openOutputDir, type VideoGenerationMode } from "@/lib/tauri";
import { readFile } from "@tauri-apps/plugin-fs";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { VideoIcon, Loader2Icon, FolderOpenIcon, PlayIcon, AlertCircleIcon, Settings2Icon, XIcon, InfoIcon, CheckCircleIcon, ImageIcon, UploadIcon, LinkIcon } from "lucide-vue-next";

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
const numFrames = ref(121);
const frameRate = ref(24);
const seed = ref<number | undefined>(undefined);
const negativePrompt = ref("");

// 图片管理 - 统一使用一个数组，支持本地路径和URL
interface ImageItem {
  id: string;
  path: string; // 本地路径或URL
  type: 'local' | 'url';
  name: string;
}
const imageItems = ref<ImageItem[]>([]);
const showUrlInput = ref(false);
const urlInput = ref("");
const isKeyframesMode = ref(false);

// 计算当前生成模式
const detectedMode = computed<VideoGenerationMode>(() => {
  const count = imageItems.value.length;
  if (count === 0) return "text";
  if (count === 1) return "single";
  if (isKeyframesMode.value) return "keyframes";
  return "multi";
});

// 模式显示文本
const modeDisplayText = computed(() => {
  switch (detectedMode.value) {
    case "text": return "文生视频";
    case "single": return "单图生视频";
    case "multi": return "多图生视频";
    case "keyframes": return "关键帧模式";
    default: return "文生视频";
  }
});

// 预设时长选项
const durationPresets = [
  { label: "3 秒", frames: 73, fps: 24 },
  { label: "5 秒", frames: 121, fps: 24 },
  { label: "10 秒", frames: 241, fps: 24 },
];

const selectedDuration = ref(1);

// 分辨率选项
const resolutionPresets = [
  { label: "1152 x 768 (16:9)", width: 1152, height: 768 },
  { label: "768 x 1152 (9:16)", width: 768, height: 1152 },
  { label: "1024 x 1024 (1:1)", width: 1024, height: 1024 },
];

const selectedResolution = ref(0);

// 生成唯一ID
function generateId(): string {
  return Date.now().toString(36) + Math.random().toString(36).substr(2);
}

// 判断是否为URL
function isUrl(str: string): boolean {
  return str.startsWith('http://') || str.startsWith('https://') || str.startsWith('data:image/');
}

// 选择本地图片
async function selectLocalImages() {
  const selected = await open({
    multiple: true,
    filters: [
      { name: "图片", extensions: ["png", "jpg", "jpeg", "webp", "gif"] },
    ],
  });
  if (selected && Array.isArray(selected)) {
    // 限制最多5张图片
    const remainingSlots = 5 - imageItems.value.length;
    const pathsToAdd = selected.slice(0, remainingSlots);
    
    pathsToAdd.forEach(path => {
      imageItems.value.push({
        id: generateId(),
        path: path,
        type: 'local',
        name: path.split('\\').pop() || path.split('/').pop() || '图片'
      });
    });
    
    if (selected.length > remainingSlots) {
      showToast("info", "最多只能选择5张图片", 3000);
    }
  } else if (selected && typeof selected === 'string') {
    // 单选情况
    if (imageItems.value.length < 5) {
      const path = selected as string;
      imageItems.value.push({
        id: generateId(),
        path: path,
        type: 'local',
        name: path.split('\\').pop() || path.split('/').pop() || '图片'
      });
    } else {
      showToast("info", "最多只能选择5张图片", 3000);
    }
  }
}

// 添加URL图片
function addUrlImage() {
  const url = urlInput.value.trim();
  if (!url) {
    showToast("info", "请输入图片URL");
    return;
  }
  
  if (!isUrl(url)) {
    showToast("error", "请输入有效的图片URL (http:// 或 https://)");
    return;
  }
  
  if (imageItems.value.length >= 5) {
    showToast("info", "最多只能添加5张图片");
    return;
  }
  
  // 从URL提取文件名
  let name = '网络图片';
  try {
    const urlObj = new URL(url);
    const pathname = urlObj.pathname;
    const filename = pathname.split('/').pop();
    if (filename) {
      name = decodeURIComponent(filename);
    }
  } catch {
    // URL解析失败，使用默认名称
  }
  
  imageItems.value.push({
    id: generateId(),
    path: url,
    type: 'url',
    name: name
  });
  
  urlInput.value = "";
  showUrlInput.value = false;
  showToast("success", "图片URL已添加");
}

// 移除图片
function removeImage(id: string) {
  const index = imageItems.value.findIndex(item => item.id === id);
  if (index > -1) {
    imageItems.value.splice(index, 1);
  }
}

// 清空所有图片
function clearAllImages() {
  imageItems.value = [];
}

// 计算实际视频时长
const videoDuration = computed(() => {
  return (numFrames.value / frameRate.value).toFixed(1);
});

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
    
    if (videoBlobUrl.value && videoBlobUrl.value.startsWith('blob:')) {
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
    const blob = new Blob([fileData], { type: 'video/mp4' });
    const url = URL.createObjectURL(blob);
    
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

// 监听视频路径变化
watch(resultVideoPath, (newPath) => {
  loadVideo(newPath);
}, { immediate: true });

// 视频加载错误处理
function handleVideoError(e: Event) {
  const videoEl = e.target as HTMLVideoElement;
  const error = videoEl.error;
  
  if (error && error.code === 4 && resultVideoPath.value) {
    loadVideoBlob(resultVideoPath.value);
    return;
  }
  
  showToast("error", "视频播放失败，请尝试打开目录查看");
}

// 监听进度事件
let unlistenProgress: UnlistenFn | null = null;

onMounted(async () => {
  try {
    videoOutputDir.value = await getVideoOutputDir();
    
    if (isGenerating.value) {
      isGenerating.value = false;
      generationStatus.value = "error";
      errorMessage.value = "生成被中断，请重新生成";
      saveState();
    }
    
    unlistenProgress = await listen("video-generation-progress", (event) => {
      const payload = event.payload as { status: string; progress: number };
      generationProgress.value = payload.progress;
      
      const statusUpper = payload.status.toUpperCase();
      if (statusUpper === "COMPLETED" || statusUpper === "SUCCESS") {
        generationStatus.value = "downloading";
      } else if (statusUpper === "FAILED" || statusUpper === "ERROR") {
        generationStatus.value = "error";
      } else if (statusUpper === "QUEUED" || statusUpper === "PENDING") {
        if (generationStatus.value === "creating") {
          generationStatus.value = "processing";
        }
      } else {
        generationStatus.value = "processing";
      }
    });
  } catch (e) {
    console.error("初始化失败:", e);
  }
});

onUnmounted(() => {
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
  generationProgress.value = 0;

  startTime.value = Date.now();
  elapsedTime.value = 0;
  timerInterval = setInterval(() => {
    if (startTime.value) {
      elapsedTime.value = Math.floor((Date.now() - startTime.value) / 1000);
    }
  }, 1000);

  try {
    generationStatus.value = "processing";

    // 构建生成选项
    const options: any = {
      prompt: prompt.value.trim(),
      output_dir: videoOutputDir.value,
      width: width.value,
      height: height.value,
      num_frames: numFrames.value,
      frame_rate: frameRate.value,
      seed: seed.value,
      negative_prompt: negativePrompt.value || undefined,
      image_mode: detectedMode.value,
    };

    // 根据图片数量添加参数
    const imageCount = imageItems.value.length;
    if (imageCount === 1) {
      options.image = imageItems.value[0].path;
    } else if (imageCount >= 2) {
      options.images = imageItems.value.map(item => item.path);
    }

    const result = await generateVideo(options);

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
  const progressStr = isGenerating.value ? ` ${generationProgress.value}%` : "";
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
        使用 Agnes AI 将文字描述或图片转换为视频
      </p>
    </div>

    <!-- Main Content -->
    <div class="flex-1 flex gap-6 overflow-hidden">
      <!-- Left Panel - Input -->
      <div class="flex-1 flex flex-col gap-4 overflow-y-auto">
        <!-- Current Mode Display -->
        <div class="flex items-center justify-between p-3 bg-muted/50 rounded-lg">
          <div class="flex items-center gap-2">
            <span class="text-sm text-muted-foreground">当前模式:</span>
            <span class="text-sm font-medium px-2 py-1 bg-primary/10 text-primary rounded">
              {{ modeDisplayText }}
            </span>
          </div>
          <span class="text-xs text-muted-foreground">
            {{ imageItems.length === 0 ? '未选择图片' : `已选择 ${imageItems.length} 张图片` }}
          </span>
        </div>

        <!-- Image Upload Section -->
        <div class="flex flex-col gap-2">
          <div class="flex items-center justify-between">
            <label class="text-sm font-medium">参考图片 (可选)</label>
            <div class="flex items-center gap-2">
              <!-- 关键帧模式开关 (仅当图片>=2时显示) -->
              <label v-if="imageItems.length >= 2" class="flex items-center gap-2 text-sm cursor-pointer">
                <input
                  v-model="isKeyframesMode"
                  type="checkbox"
                  class="rounded border"
                  :disabled="isGenerating"
                />
                <span class="text-muted-foreground">关键帧模式</span>
              </label>
              <button
                v-if="imageItems.length > 0"
                @click="clearAllImages"
                class="text-xs text-destructive hover:underline"
                :disabled="isGenerating"
              >
                清空全部
              </button>
            </div>
          </div>

          <!-- Image List -->
          <div v-if="imageItems.length > 0" class="space-y-2">
            <div
              v-for="(item, index) in imageItems"
              :key="item.id"
              class="flex items-center gap-3 border rounded-lg p-3"
            >
              <span class="w-6 h-6 rounded-full bg-primary/10 text-primary text-xs flex items-center justify-center font-medium">
                {{ index + 1 }}
              </span>
              <ImageIcon v-if="item.type === 'local'" class="w-5 h-5 text-muted-foreground" />
              <LinkIcon v-else class="w-5 h-5 text-blue-500" />
              <span class="flex-1 text-sm truncate" :title="item.path">
                {{ item.name }}
              </span>
              <span v-if="item.type === 'url'" class="text-xs text-blue-500">URL</span>
              <button
                @click="removeImage(item.id)"
                class="p-1.5 hover:bg-destructive/10 text-destructive rounded transition-colors"
                :disabled="isGenerating"
              >
                <XIcon class="w-4 h-4" />
              </button>
            </div>
          </div>

          <!-- Add Image Buttons -->
          <div v-if="imageItems.length < 5" class="flex gap-2">
            <button
              @click="selectLocalImages"
              class="flex-1 border-2 border-dashed border-muted-foreground/25 rounded-lg p-4 flex items-center justify-center gap-2 hover:border-primary/50 hover:bg-muted/50 transition-colors"
              :class="{ 'pointer-events-none opacity-50': isGenerating }"
            >
              <UploadIcon class="w-5 h-5 text-muted-foreground" />
              <span class="text-sm text-muted-foreground">选择本地图片</span>
            </button>
            <button
              @click="showUrlInput = !showUrlInput"
              class="flex-1 border-2 border-dashed border-muted-foreground/25 rounded-lg p-4 flex items-center justify-center gap-2 hover:border-primary/50 hover:bg-muted/50 transition-colors"
              :class="{ 'pointer-events-none opacity-50': isGenerating }"
            >
              <LinkIcon class="w-5 h-5 text-muted-foreground" />
              <span class="text-sm text-muted-foreground">添加图片URL</span>
            </button>
          </div>

          <!-- URL Input -->
          <div v-if="showUrlInput" class="flex gap-2">
            <input
              v-model="urlInput"
              type="text"
              placeholder="输入图片URL (http:// 或 https://)"
              class="flex-1 px-3 py-2 rounded-lg border bg-background text-sm"
              @keyup.enter="addUrlImage"
            />
            <button
              @click="addUrlImage"
              class="px-4 py-2 bg-primary text-primary-foreground rounded-lg text-sm hover:bg-primary/90"
            >
              添加
            </button>
          </div>

          <!-- Mode Hint -->
          <p class="text-xs text-muted-foreground">
            <span v-if="imageItems.length === 0">不选择图片将使用文生视频模式</span>
            <span v-else-if="imageItems.length === 1">已选择1张图片，将使用单图生视频模式</span>
            <span v-else-if="imageItems.length >= 2">
              已选择{{ imageItems.length }}张图片，将使用{{ isKeyframesMode ? '关键帧' : '多图' }}模式
            </span>
          </p>
        </div>

        <!-- Prompt Input -->
        <div class="flex flex-col gap-2">
          <label class="text-sm font-medium">视频描述</label>
          <textarea
            v-model="prompt"
            @input="updatePrompt(($event.target as HTMLTextAreaElement).value)"
            :placeholder="imageItems.length === 0
              ? '描述你想要生成的视频内容，例如：一只可爱的猫咪在草地上玩耍...'
              : '描述视频动画效果，例如：The woman slowly turns around and looks back at the camera...'"
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
          {{ isGenerating ? "生成中..." : `生成视频 (${modeDisplayText})` }}
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
      <div class="w-[480px] flex flex-col gap-4">
        <!-- Video Preview -->
        <div class="flex-1 border rounded-lg overflow-hidden bg-black flex items-center justify-center relative">
          <video
            v-if="videoBlobUrl"
            ref="videoPlayer"
            :src="videoBlobUrl"
            controls
            class="max-w-full max-h-full"
            @error="handleVideoError"
          />
          <div v-else class="text-center p-8">
            <VideoIcon class="w-16 h-16 text-muted-foreground mx-auto mb-4" />
            <p class="text-muted-foreground">生成的视频将在这里显示</p>
          </div>
        </div>

        <!-- Video Info -->
        <div v-if="resultVideoPath" class="p-4 border rounded-lg space-y-2">
          <p class="text-sm font-medium">视频信息</p>
          <p class="text-xs text-muted-foreground break-all">{{ resultVideoPath.replace(/^.*[\\/]/, '/video/') }}</p>
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
