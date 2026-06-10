<script setup lang="ts">
import { ref, computed, onMounted, watch, onUnmounted } from "vue";
import { useGenerationStore } from "@/stores/generation";
import { generateImage, batchGenerateImages, optimizePrompt, loadConfig, openOutputDir, getProviderModels, retryDownloadImage, type ReferenceImage } from "@/lib/tauri";
import { message } from "@tauri-apps/plugin-dialog";
import { Wand2Icon, Loader2Icon, FolderOpenIcon, SparklesIcon, Maximize2Icon, XIcon, ListIcon, HelpCircleIcon, CheckCircle2Icon, DownloadIcon } from "lucide-vue-next";
import { listen } from "@tauri-apps/api/event";
import { readFile } from "@tauri-apps/plugin-fs";
import ImageInput from "@/components/ImageInput.vue";

const store = useGenerationStore();

// 参考图片列表
const referenceImages = ref<ReferenceImage[]>([]);

// 图片 URL 缓存
const imageUrlCache = ref<string | null>(null);

// 辅助函数：将 Uint8Array 转换为 Base64
function arrayBufferToBase64(buffer: Uint8Array): string {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  const len = bytes.byteLength;
  const chunkSize = 0x8000;
  for (let i = 0; i < len; i += chunkSize) {
    const chunk = bytes.subarray(i, i + chunkSize);
    binary += String.fromCharCode.apply(null, chunk as unknown as number[]);
  }
  return btoa(binary);
}

// 将文件路径转换为可访问的 URL（使用 Base64）
async function loadImageUrl(path: string): Promise<string> {
  try {
    const data = await readFile(path);
    const ext = path.split('.').pop()?.toLowerCase() || 'png';
    const mimeType = ext === 'jpg' || ext === 'jpeg' ? 'image/jpeg' :
                     ext === 'webp' ? 'image/webp' : 'image/png';
    const base64 = arrayBufferToBase64(data);
    return `data:${mimeType};base64,${base64}`;
  } catch (e) {
    console.error("[GenerateView] Failed to load image:", path, e);
    return '';
  }
}

// 计算属性：生成结果的图片 URL
const resultImageUrl = computed(() => {
  return imageUrlCache.value || '';
});

// 监听 resultImage 变化，自动加载图片
watch(() => store.resultImage, async (newPath) => {
  if (newPath) {
    imageUrlCache.value = await loadImageUrl(newPath);
  } else {
    imageUrlCache.value = null;
  }
});

// 获取批量生成图片的 URL（带缓存）
async function getBatchImageUrl(path: string): Promise<string> {
  if (batchImageUrls.value.has(path)) {
    return batchImageUrls.value.get(path)!;
  }
  const url = await loadImageUrl(path);
  if (url) {
    batchImageUrls.value.set(path, url);
  }
  return url;
}

// 监听批量结果变化，自动加载图片
watch(() => store.batchResults, async (results) => {
  for (const result of results) {
    if (result.success && result.image_path && !batchImageUrls.value.has(result.image_path)) {
      await getBatchImageUrl(result.image_path);
    }
  }
}, { deep: true });

const prompt = ref(localStorage.getItem("lastPrompt") || "");
const isOptimizing = ref(false);
const optimizeResult = ref<string | null>(null);
const showOptimizeModal = ref(false);
const showImageModal = ref(false);

// 批量生成状态
const batchPrompts = ref("");
const batchImageUrls = ref<Map<string, string>>(new Map());
const selectedBatchImage = ref<string | null>(null);

// 比例帮助弹窗
const showRatioHelp = ref(false);

// 自定义模型
const useCustomModel = ref(false);
const customModelName = ref("");

function updateCustomModel() {
  if (useCustomModel.value && customModelName.value.trim()) {
    store.setModel(customModelName.value.trim());
  }
}

// 生成进度定时器
let progressTimer: ReturnType<typeof setInterval> | null = null;

// 启动进度更新定时器
function startProgressTimer() {
  stopProgressTimer();
  progressTimer = setInterval(() => {
    store.updateElapsedTime();
  }, 100);
}

// 停止进度更新定时器
function stopProgressTimer() {
  if (progressTimer) {
    clearInterval(progressTimer);
    progressTimer = null;
  }
}

onUnmounted(() => {
  stopProgressTimer();
});

const providers = [
  { value: "agnes", label: "Agnes AI (免费)" },
  { value: "modelscope", label: "ModelScope (阿里云)" },
  { value: "nvidia", label: "NVIDIA" },
  { value: "gemini", label: "Google Gemini" },
  { value: "openrouter", label: "OpenRouter" },
  { value: "openai", label: "OpenAI" },
  { value: "siliconflow", label: "硅基流动 (SiliconFlow)" },
];

const aspectRatios = [
  { value: "1:1", label: "1:1 正方形", width: 1024, height: 1024, desc: "正方形" },
  { value: "9:16", label: "9:16 竖屏", width: 720, height: 1280, desc: "竖屏/手机壁纸" },
  { value: "16:9", label: "16:9 横屏", width: 1280, height: 720, desc: "横屏/宽屏" },
  { value: "3:4", label: "3:4 竖版", width: 768, height: 1024, desc: "竖版海报" },
  { value: "2:3", label: "2:3 照片", width: 768, height: 1152, desc: "照片比例" },
  { value: "3:2", label: "3:2 经典", width: 1152, height: 768, desc: "经典照片比例" },
  { value: "4:3", label: "4:3 标准", width: 1024, height: 768, desc: "标准显示器" },
  { value: "21:9", label: "21:9 超宽", width: 1344, height: 576, desc: "超宽屏/横幅" },
];

// 动态模型列表（用于支持从远端获取模型的提供商）
const dynamicModels = ref<Record<string, string[]>>({});

// 支持动态获取模型的提供商列表
const dynamicProviders = ['agnes', 'openai', 'siliconflow', 'openrouter', 'nvidia', 'gemini', 'modelscope'];

// 计算属性：获取当前提供商的模型列表
const currentModels = computed(() => {
  // 对于支持动态获取的提供商，使用动态获取的模型列表
  if (dynamicProviders.includes(store.provider) && dynamicModels.value[store.provider]?.length > 0) {
    return dynamicModels.value[store.provider];
  }
  // 其他提供商使用配置文件中的模型列表
  return store.models;
});

const selectedDimensions = computed(() => {
  return aspectRatios.find(r => r.value === store.aspectRatio) || aspectRatios[0];
});

// 加载提供商的模型列表
async function loadProviderModels(provider: string) {
  // 只加载支持动态获取的提供商
  if (!dynamicProviders.includes(provider)) {
    return;
  }
  
  try {
    const models = await getProviderModels(provider);
    if (models && models.length > 0) {
      dynamicModels.value[provider] = models;
      // 如果当前模型不在列表中，选择第一个
      if (!models.includes(store.model)) {
        store.setModel(models[0]);
      }
    }
  } catch (e) {
    console.error(`[GenerateView] 加载 ${provider} 模型失败:`, e);
  }
}

onMounted(async () => {
  try {
    const config = await loadConfig();
    store.setConfig(config);
    
    // 加载当前提供商的动态模型列表
    await loadProviderModels(store.provider);
    
    if (!store.model && currentModels.value.length > 0) {
      store.setModel(currentModels.value[0]);
    }
    
    // 恢复上一次成功生成的图片显示
    if (store.resultImage && store.status === 'success') {
      imageUrlCache.value = await loadImageUrl(store.resultImage);
    }
    
    // 如果正在生成中，恢复进度定时器
    if (store.isGenerating) {
      startProgressTimer();
    }
  } catch (e) {
    console.error("Failed to load config:", e);
  }
});

watch(prompt, (newVal) => {
  localStorage.setItem("lastPrompt", newVal);
});

async function handleGenerate() {
  if (!prompt.value.trim()) return;

  store.startGeneration(prompt.value);
  startProgressTimer();

  try {
    // 准备参考图片（取第一张）
    let imageBase64: string | undefined;
    if (referenceImages.value.length > 0) {
      const refImage = referenceImages.value[0];
      console.log("[debug-point frontend-ref-image]", {
        count: referenceImages.value.length,
        type: refImage.type,
        source: refImage.source,
        previewPrefix: refImage.preview?.slice(0, 30),
        previewLength: refImage.preview?.length || 0,
      });
      if (refImage.type === "file") {
        // 本地文件需要读取并转换为 base64（去掉 data URI 前缀）
        try {
          const data = await readFile(refImage.source);
          const ext = refImage.source.split('.').pop()?.toLowerCase() || 'png';
          const mimeType = ext === 'jpg' || ext === 'jpeg' ? 'image/jpeg' :
                           ext === 'webp' ? 'image/webp' : 'image/png';
          imageBase64 = `data:${mimeType};base64,${arrayBufferToBase64(data)}`;
        } catch (e) {
          console.error("读取参考图片失败:", e);
        }
      } else {
        // URL 类型直接使用
        imageBase64 = refImage.preview;
      }
    }

    console.log("[debug-point frontend-generate-payload]", {
      provider: store.provider,
      model: store.model || undefined,
      hasImage: Boolean(imageBase64),
      imageLength: imageBase64?.length || 0,
      imagePrefix: imageBase64?.slice(0, 30) || null,
    });

    const result = await generateImage({
      prompt: prompt.value,
      provider: store.provider,
      model: store.model || undefined,
      output_dir: store.outputDir,
      width: selectedDimensions.value.width,
      height: selectedDimensions.value.height,
      image: imageBase64,
    });

    stopProgressTimer();

    if (result.success && result.image_path) {
      store.generationSuccess(result.image_path);
    } else {
      // 检查是否是网络错误，如果是则保存图片URL
      const errorMsg = result.error || "生成失败";
      const imageUrl = extractImageUrlFromError(errorMsg);
      store.generationFailed(errorMsg, imageUrl);
    }
  } catch (e) {
    stopProgressTimer();
    const errorMsg = e instanceof Error ? e.message : "未知错误";
    const imageUrl = extractImageUrlFromError(errorMsg);
    store.generationFailed(errorMsg, imageUrl);
  }
}

// 从错误信息中提取图片URL
// 只有当错误是"图片下载失败"类型时才返回URL，其他错误返回 undefined
function extractImageUrlFromError(errorMsg: string): string | undefined {
  // 检查是否是图片下载失败的错误
  // 格式: "图片下载失败 [`https://xxx`]: 错误信息" 或 "图片下载失败 [https://xxx]: 错误信息"
  if (!errorMsg.includes("图片下载失败")) {
    return undefined;
  }
  
  // 匹配方括号中的URL（支持带反引号和不带反引号的情况）
  const urlMatch = errorMsg.match(/图片下载失败\s*\[`?([^\]`]+)`?\]/);
  return urlMatch ? urlMatch[1].trim() : undefined;
}

// 处理重新下载
async function handleRetryDownload() {
  if (!store.pendingImageUrl) return;

  store.startRetryDownload();

  try {
    const result = await retryDownloadImage({
      image_url: store.pendingImageUrl,
      output_dir: store.outputDir,
    });

    if (result.success && result.image_path) {
      store.retryDownloadSuccess(result.image_path);
      // 加载并显示图片
      imageUrlCache.value = await loadImageUrl(result.image_path);
    } else {
      store.retryDownloadFailed(result.error || "重新下载失败");
      await message("重新下载失败: " + (result.error || "未知错误"), { 
        title: "错误", 
        kind: "error" 
      });
    }
  } catch (e) {
    const errorMsg = e instanceof Error ? e.message : "未知错误";
    store.retryDownloadFailed(errorMsg);
    await message("重新下载失败: " + errorMsg, { 
      title: "错误", 
      kind: "error" 
    });
  }
}

async function handleOptimizePrompt() {
  if (!prompt.value.trim() || isOptimizing.value) return;

  isOptimizing.value = true;
  optimizeResult.value = null;

  try {
    const result = await optimizePrompt(prompt.value);
    if (result.success && result.optimized_prompt) {
      optimizeResult.value = result.optimized_prompt;
      showOptimizeModal.value = true;
    } else {
      await message("优化失败: " + (result.error || "未知错误"), { title: "错误", kind: "error" });
    }
  } catch (e) {
    await message("优化请求失败: " + (e instanceof Error ? e.message : "未知错误"), { title: "错误", kind: "error" });
  } finally {
    isOptimizing.value = false;
  }
}

function applyOptimizedPrompt() {
  if (optimizeResult.value) {
    prompt.value = optimizeResult.value;
    showOptimizeModal.value = false;
  }
}

async function handleOpenOutputDir() {
  if (store.resultImage) {
    try {
      const dir = store.resultImage.substring(0, store.resultImage.lastIndexOf("\\"));
      await openOutputDir(dir);
    } catch (e) {
      console.error("Failed to open output dir:", e);
    }
  }
}

async function handleBatchGenerate() {
  if (!batchPrompts.value.trim()) return;

  const prompts = batchPrompts.value.split("\n").filter(p => p.trim());
  if (prompts.length === 0) return;

  store.startBatchGeneration(prompts.length);

  const unlistenProgress = await listen("batch-progress", (event) => {
    const payload = event.payload as { current: number; total: number };
    store.updateBatchProgress(payload.current, payload.total);
  });

  const unlistenItem = await listen("batch-item-complete", (event) => {
    const payload = event.payload as { result: { index: number; prompt: string; success: boolean; image_path?: string; error?: string } };
    store.addBatchResult(payload.result);
  });

  try {
    await batchGenerateImages({
      prompts,
      provider: store.provider,
      model: store.model || undefined,
      output_dir: store.outputDir,
      width: selectedDimensions.value.width,
      height: selectedDimensions.value.height,
    });

    store.finishBatchGeneration();
  } catch (e) {
    await message("批量生成失败: " + String(e), { title: "错误", kind: "error" });
    store.resetBatchState();
  } finally {
    unlistenProgress();
    unlistenItem();
  }
}

const batchSuccessCount = computed(() => store.batchResults.filter(r => r.success).length);
const batchFailedCount = computed(() => store.batchResults.filter(r => !r.success).length);

// 显示批量图片预览
function showBatchImageModal(imagePath: string) {
  selectedBatchImage.value = imagePath;
}
</script>

<template>
  <div class="p-6 max-w-4xl mx-auto">
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-2xl font-bold">AI 图片生成</h1>
      <button
        @click="store.setBatchMode(!store.isBatchMode)"
        class="flex items-center gap-2 px-4 py-2 rounded-lg border hover:bg-muted transition-colors"
        :class="store.isBatchMode ? 'bg-primary text-primary-foreground' : ''"
      >
        <ListIcon class="w-4 h-4" />
        {{ store.isBatchMode ? '批量模式' : '单图模式' }}
      </button>
    </div>

    <!-- Single Mode Prompt Input -->
    <div v-if="!store.isBatchMode" class="mb-6">
      <label class="block text-sm font-medium mb-2">提示词</label>
      <div class="flex gap-2">
        <textarea
          v-model="prompt"
          rows="3"
          class="flex-1 px-3 py-2 border rounded-lg resize-none focus:outline-none focus:ring-2 focus:ring-primary"
          placeholder="描述你想要生成的图片..."
          :disabled="store.isGenerating"
        />
        <button
          @click="handleOptimizePrompt"
          :disabled="isOptimizing || !prompt.trim() || store.isGenerating"
          class="px-4 py-2 border rounded-lg hover:bg-muted disabled:opacity-50 flex flex-col items-center justify-center gap-1"
          title="优化提示词"
        >
          <Wand2Icon class="w-4 h-4" :class="{ 'animate-spin': isOptimizing }" />
          <span class="text-xs">优化</span>
        </button>
      </div>

      <!-- 参考图片输入 -->
      <div class="mt-3">
        <label class="block text-xs font-medium text-muted-foreground mb-2">
          参考图片（可选，用于以图生图）
        </label>
        <ImageInput
          v-model="referenceImages"
          :disabled="store.isGenerating"
        />
      </div>
    </div>

    <!-- Batch Mode Prompt Input -->
    <div v-else class="mb-6">
      <label class="block text-sm font-medium mb-2">批量提示词（每行一个）</label>
      <textarea
        v-model="batchPrompts"
        rows="6"
        class="w-full px-3 py-2 border rounded-lg resize-none focus:outline-none focus:ring-2 focus:ring-primary"
        placeholder="每行输入一个提示词，将依次生成多张图片..."
        :disabled="store.isBatchGenerating"
      />
      <p class="text-xs text-muted-foreground mt-1">
        当前 {{ batchPrompts.split('\n').filter(p => p.trim()).length }} 个提示词
      </p>
    </div>

    <!-- Optimize Result Modal -->
    <div v-if="showOptimizeModal && optimizeResult" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div class="bg-background rounded-lg p-6 max-w-lg w-full mx-4">
        <div class="flex items-center justify-between mb-4">
          <h3 class="font-medium">提示词优化结果</h3>
          <button @click="showOptimizeModal = false" class="text-muted-foreground hover:text-foreground">
            <XIcon class="w-5 h-5" />
          </button>
        </div>
        <div class="bg-muted p-4 rounded-lg mb-4">
          <p class="text-sm">{{ optimizeResult }}</p>
        </div>
        <div class="flex justify-end gap-2">
          <button @click="showOptimizeModal = false" class="px-4 py-2 border rounded-lg hover:bg-muted">
            取消
          </button>
          <button @click="applyOptimizedPrompt" class="px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90">
            应用优化结果
          </button>
        </div>
      </div>
    </div>

    <!-- Provider Selection -->
    <div class="mb-6">
      <label class="block text-sm font-medium mb-2">提供商</label>
      <select
        v-model="store.provider"
        @change="async (e) => {
          const newProvider = (e.target as HTMLSelectElement).value;
          store.setProvider(newProvider);
          // 如果切换到支持动态获取的提供商，加载模型列表
          if (dynamicProviders.includes(newProvider)) {
            await loadProviderModels(newProvider);
          }
        }"
        class="w-full px-3 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary"
        :disabled="store.isGenerating || store.isBatchGenerating"
      >
        <option v-for="p in providers" :key="p.value" :value="p.value">
          {{ p.label }}
        </option>
      </select>
    </div>

    <!-- Model Selection -->
    <div class="mb-6">
      <div class="flex items-center justify-between mb-2">
        <label class="text-sm font-medium">模型</label>
        <label class="flex items-center gap-2 text-sm">
          <input
            v-model="useCustomModel"
            type="checkbox"
            class="rounded"
            :disabled="store.isGenerating || store.isBatchGenerating"
          />
          自定义模型
        </label>
      </div>
      <select
        v-if="!useCustomModel"
        v-model="store.model"
        @change="(e) => store.setModel((e.target as HTMLSelectElement).value)"
        class="w-full px-3 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary"
        :disabled="store.isGenerating || store.isBatchGenerating"
      >
        <option v-for="m in currentModels" :key="m" :value="m">
          {{ m }}
        </option>
      </select>
      <input
        v-else
        v-model="customModelName"
        @blur="updateCustomModel"
        @keyup.enter="updateCustomModel"
        type="text"
        class="w-full px-3 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary"
        placeholder="输入自定义模型名称"
        :disabled="store.isGenerating || store.isBatchGenerating"
      />
    </div>

    <!-- Aspect Ratio -->
    <div class="mb-6">
      <div class="flex items-center gap-2 mb-2">
        <label class="text-sm font-medium">图片比例</label>
        <button @click="showRatioHelp = true" class="text-muted-foreground hover:text-foreground">
          <HelpCircleIcon class="w-4 h-4" />
        </button>
      </div>
      <select
        v-model="store.aspectRatio"
        @change="(e) => store.setAspectRatio((e.target as HTMLSelectElement).value)"
        class="w-full px-3 py-2 border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary"
        :disabled="store.isGenerating || store.isBatchGenerating"
      >
        <option v-for="r in aspectRatios" :key="r.value" :value="r.value">
          {{ r.label }} - {{ r.desc }} ({{ r.width }}×{{ r.height }})
        </option>
      </select>
    </div>

    <!-- Ratio Help Modal -->
    <div v-if="showRatioHelp" class="fixed inset-0 z-50 flex items-center justify-center bg-black/50" @click="showRatioHelp = false">
      <div class="bg-background rounded-lg p-6 max-w-md w-full mx-4" @click.stop>
        <div class="flex items-center justify-between mb-4">
          <h3 class="font-medium">图片比例说明</h3>
          <button @click="showRatioHelp = false" class="text-muted-foreground hover:text-foreground">
            <XIcon class="w-5 h-5" />
          </button>
        </div>
        <div class="space-y-2 text-sm">
          <div v-for="r in aspectRatios" :key="r.value" class="flex justify-between py-1 border-b last:border-0">
            <span class="font-medium">{{ r.label }}</span>
            <span class="text-muted-foreground">{{ r.desc }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Single Generate Button -->
    <button
      v-if="!store.isBatchMode"
      @click="handleGenerate"
      :disabled="store.isGenerating || !prompt.trim()"
      class="w-full py-3 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50 flex items-center justify-center gap-2 mb-6"
    >
      <Loader2Icon v-if="store.isGenerating" class="w-5 h-5 animate-spin" />
      <SparklesIcon v-else class="w-5 h-5" />
      {{ store.isGenerating ? "生成中..." : "生成图片" }}
    </button>

    <!-- Batch Generate Button -->
    <button
      v-else
      @click="handleBatchGenerate"
      :disabled="store.isBatchGenerating || !batchPrompts.trim()"
      class="w-full py-3 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50 flex items-center justify-center gap-2 mb-6"
    >
      <Loader2Icon v-if="store.isBatchGenerating" class="w-5 h-5 animate-spin" />
      <ListIcon v-else class="w-5 h-5" />
      {{ store.isBatchGenerating ? `生成中 ${store.batchProgress.current}/${store.batchProgress.total}` : "开始批量生成" }}
    </button>

    <!-- Single Progress -->
    <div v-if="!store.isBatchMode && store.isGenerating" class="mb-6">
      <div class="h-2 bg-muted rounded-full overflow-hidden">
        <div
          class="h-full bg-primary transition-all duration-300"
          :style="{ width: `${store.progress}%` }"
        />
      </div>
      <p class="text-sm text-muted-foreground mt-2 text-center">
        已用时: {{ store.formattedElapsedTime }}
      </p>
    </div>

    <!-- Batch Progress -->
    <div v-if="store.isBatchMode && store.isBatchGenerating" class="mb-6">
      <div class="h-2 bg-muted rounded-full overflow-hidden">
        <div
          class="h-full bg-primary transition-all duration-300"
          :style="{ width: `${(store.batchProgress.current / store.batchProgress.total) * 100}%` }"
        />
      </div>
      <p class="text-sm text-muted-foreground mt-2 text-center">
        进度: {{ store.batchProgress.current }} / {{ store.batchProgress.total }}
      </p>
    </div>

    <!-- Single Error -->
    <div v-if="!store.isBatchMode && store.status === 'error'" class="mb-6 p-4 bg-destructive/10 text-destructive rounded-lg">
      <div class="flex flex-col gap-3">
        <p>{{ store.error }}</p>
        <!-- 显示重新下载按钮（仅当有待下载的图片URL时） -->
        <div v-if="store.pendingImageUrl" class="flex items-center gap-3">
          <button
            @click="handleRetryDownload"
            :disabled="store.isRetryingDownload"
            class="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50"
          >
            <Loader2Icon v-if="store.isRetryingDownload" class="w-4 h-4 animate-spin" />
            <DownloadIcon v-else class="w-4 h-4" />
            {{ store.isRetryingDownload ? "下载中..." : "重新下载" }}
          </button>
          <span class="text-xs text-muted-foreground">图片已生成但下载失败，可点击重试</span>
        </div>
        <!-- 重新下载的错误提示 -->
        <p v-if="store.retryDownloadError" class="text-sm text-destructive">
          重新下载失败: {{ store.retryDownloadError }}
        </p>
      </div>
    </div>

    <!-- Single Result - 只在生成成功时显示 -->
    <div v-if="!store.isBatchMode && store.status === 'success' && store.resultImage" class="border rounded-lg p-4">
      <div class="flex items-center justify-between mb-4">
        <div>
          <h3 class="font-medium">生成结果</h3>
          <p class="text-xs text-muted-foreground mt-1">
            总耗时: {{ store.formattedGenerationDuration }} | 尺寸: {{ selectedDimensions.width }} × {{ selectedDimensions.height }}
          </p>
        </div>
        <button
          @click="handleOpenOutputDir"
          class="flex items-center gap-2 text-sm text-primary hover:underline"
        >
          <FolderOpenIcon class="w-4 h-4" />
          打开目录
        </button>
      </div>
      <div class="relative group">
        <img
          :src="resultImageUrl"
          class="max-w-full h-auto rounded-lg cursor-pointer"
          alt="Generated"
          @click="showImageModal = true"
        />
        <button
          @click="showImageModal = true"
          class="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity bg-black/50 hover:bg-black/70 text-white p-2 rounded-lg"
        >
          <Maximize2Icon class="w-4 h-4" />
        </button>
      </div>
    </div>

    <!-- Image Preview Modal - 只在生成成功时显示 -->
    <div
      v-if="showImageModal && store.status === 'success' && store.resultImage"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/90 backdrop-blur-sm"
      @click="showImageModal = false"
    >
      <div class="relative max-w-[90vw] max-h-[90vh] p-4" @click.stop>
        <img
          :src="resultImageUrl"
          alt="Preview"
          class="max-w-full max-h-[85vh] object-contain rounded-lg shadow-2xl"
        />
        <button
          @click="showImageModal = false"
          class="absolute top-6 right-6 w-10 h-10 rounded-full bg-black/50 hover:bg-black/70 text-white flex items-center justify-center transition-colors"
        >
          <XIcon class="w-5 h-5" />
        </button>
      </div>
    </div>

    <!-- Batch Results -->
    <div v-if="store.isBatchMode && store.showBatchResults" class="border rounded-lg p-4 mb-6">
      <div class="flex items-center justify-between mb-4">
        <h3 class="font-medium">批量生成结果</h3>
        <div class="flex gap-4 text-sm">
          <span class="text-green-600">成功: {{ batchSuccessCount }}</span>
          <span class="text-red-600">失败: {{ batchFailedCount }}</span>
          <span class="text-muted-foreground">总计: {{ store.batchResults.length }}</span>
        </div>
      </div>
      <div class="max-h-96 overflow-auto space-y-3">
        <div
          v-for="result in store.batchResults"
          :key="result.index"
          class="p-3 rounded-lg border"
          :class="result.success ? 'bg-green-50 border-green-200' : 'bg-red-50 border-red-200'"
        >
          <div class="flex items-center gap-3 mb-2">
            <div class="flex-shrink-0 w-6 h-6 rounded-full flex items-center justify-center text-xs font-medium"
              :class="result.success ? 'bg-green-500 text-white' : 'bg-red-500 text-white'"
            >
              {{ result.index + 1 }}
            </div>
            <div class="flex-1 min-w-0">
              <p class="text-sm truncate">{{ result.prompt }}</p>
              <p v-if="!result.success" class="text-xs text-red-600 mt-0.5">{{ result.error }}</p>
            </div>
            <div v-if="result.success" class="flex-shrink-0 text-green-600">
              <CheckCircle2Icon class="w-5 h-5" />
            </div>
            <div v-else class="flex-shrink-0 text-red-600">
              <XIcon class="w-5 h-5" />
            </div>
          </div>
          <!-- 成功时显示预览图片 -->
          <div v-if="result.success && result.image_path" class="mt-2">
            <img
              :src="batchImageUrls.get(result.image_path) || ''"
              class="max-w-full h-auto max-h-48 rounded-lg cursor-pointer hover:opacity-90 transition-opacity"
              alt="Generated"
              @click="showBatchImageModal(result.image_path!)"
            />
          </div>
        </div>
      </div>
    </div>

    <!-- Batch Image Preview Modal -->
    <div v-if="selectedBatchImage" class="fixed inset-0 z-50 flex items-center justify-center bg-black/80" @click="selectedBatchImage = null">
      <div class="relative max-w-[90vw] max-h-[90vh]" @click.stop>
        <button
          @click="selectedBatchImage = null"
          class="absolute -top-10 right-0 text-white hover:text-gray-300"
        >
          <XIcon class="w-6 h-6" />
        </button>
        <img
          :src="batchImageUrls.get(selectedBatchImage) || ''"
          class="max-w-full max-h-[85vh] rounded-lg"
          alt="Preview"
        />
      </div>
    </div>
  </div>
</template>
