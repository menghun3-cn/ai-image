import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { GenerationStatus, AppConfig, OptimizeResult } from "@/lib/tauri";

export const useGenerationStore = defineStore("generation", () => {
  // 生成状态
  const isGenerating = ref(false);
  const progress = ref(0);
  const status = ref<GenerationStatus>("idle");
  const resultImage = ref<string | null>(null);
  const error = ref<string | null>(null);
  const generationStartTime = ref<number | null>(null);
  const generationElapsedTime = ref(0);
  const generationDuration = ref(0);
  const currentPrompt = ref("");

  // 生成参数
  const provider = ref(localStorage.getItem("lastProvider") || "agnes");
  const model = ref(localStorage.getItem(`lastModel_${provider.value}`) || "");
  const aspectRatio = ref(localStorage.getItem("lastAspectRatio") || "1:1");
  const outputDir = ref("images");
  const isBatchMode = ref(localStorage.getItem("lastBatchMode") === "true");

  // 批量生成状态
  const isBatchGenerating = ref(false);
  const batchProgress = ref({ current: 0, total: 0 });
  const batchResults = ref<Array<{ index: number; prompt: string; success: boolean; image_path?: string; error?: string }>>([]);
  const showBatchResults = ref(false);

  // 优化状态
  const isOptimizing = ref(false);
  const optimizeResult = ref<OptimizeResult | null>(null);
  const optimizeError = ref<string | null>(null);

  // 配置
  const config = ref<AppConfig | null>(null);

  // 计算属性
  const models = computed(() => {
    if (!config.value) return [];
    return config.value.models[provider.value as keyof typeof config.value.models] || [];
  });

  const formattedElapsedTime = computed(() => {
    const seconds = Math.floor(generationElapsedTime.value / 1000);
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    if (minutes > 0) {
      return `${minutes}分${remainingSeconds}秒`;
    }
    return `${remainingSeconds}秒`;
  });

  const formattedGenerationDuration = computed(() => {
    const seconds = Math.floor(generationDuration.value / 1000);
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    if (minutes > 0) {
      return `${minutes}分${remainingSeconds}秒`;
    }
    return `${remainingSeconds}秒`;
  });

  // 方法
  function setProvider(newProvider: string) {
    provider.value = newProvider;
    localStorage.setItem("lastProvider", newProvider);
    // 恢复该提供商上次使用的模型
    const savedModel = localStorage.getItem(`lastModel_${newProvider}`);
    if (savedModel && models.value.includes(savedModel)) {
      model.value = savedModel;
    } else if (models.value.length > 0) {
      model.value = models.value[0];
    } else {
      model.value = "";
    }
  }

  function setModel(newModel: string) {
    model.value = newModel;
    localStorage.setItem(`lastModel_${provider.value}`, newModel);
  }

  function setAspectRatio(newRatio: string) {
    aspectRatio.value = newRatio;
    localStorage.setItem("lastAspectRatio", newRatio);
  }

  function setBatchMode(value: boolean) {
    isBatchMode.value = value;
    localStorage.setItem("lastBatchMode", String(value));
  }

  function startGeneration(prompt: string) {
    isGenerating.value = true;
    progress.value = 0;
    status.value = "generating";
    error.value = null;
    resultImage.value = null;
    currentPrompt.value = prompt;
    generationStartTime.value = Date.now();
    generationElapsedTime.value = 0;
    generationDuration.value = 0;
  }

  function updateProgress(value: number) {
    progress.value = value;
  }

  function updateElapsedTime() {
    if (generationStartTime.value) {
      generationElapsedTime.value = Date.now() - generationStartTime.value;
    }
  }

  function generationSuccess(imagePath: string) {
    isGenerating.value = false;
    progress.value = 100;
    status.value = "success";
    resultImage.value = imagePath;
    if (generationStartTime.value) {
      generationDuration.value = Date.now() - generationStartTime.value;
    }
  }

  function generationFailed(errorMsg: string) {
    isGenerating.value = false;
    status.value = "error";
    error.value = errorMsg;
  }

  function resetState() {
    isGenerating.value = false;
    progress.value = 0;
    status.value = "idle";
    error.value = null;
    resultImage.value = null;
    generationStartTime.value = null;
    generationElapsedTime.value = 0;
  }

  // 批量生成方法
  function startBatchGeneration(total: number) {
    isBatchGenerating.value = true;
    batchProgress.value = { current: 0, total };
    batchResults.value = [];
    showBatchResults.value = false;
  }

  function updateBatchProgress(current: number, total: number) {
    batchProgress.value = { current, total };
  }

  function addBatchResult(result: { index: number; prompt: string; success: boolean; image_path?: string; error?: string }) {
    batchResults.value.push(result);
  }

  function finishBatchGeneration() {
    isBatchGenerating.value = false;
    showBatchResults.value = true;
  }

  function resetBatchState() {
    isBatchGenerating.value = false;
    batchProgress.value = { current: 0, total: 0 };
    batchResults.value = [];
    showBatchResults.value = false;
  }

  function setConfig(newConfig: AppConfig) {
    config.value = newConfig;
    outputDir.value = newConfig.default_output_dir;
  }

  return {
    // 状态
    isGenerating,
    progress,
    status,
    resultImage,
    error,
    generationElapsedTime,
    formattedElapsedTime,
    generationDuration,
    formattedGenerationDuration,
    currentPrompt,
    // 参数
    provider,
    model,
    aspectRatio,
    outputDir,
    isBatchMode,
    models,
    // 批量生成
    isBatchGenerating,
    batchProgress,
    batchResults,
    showBatchResults,
    // 优化
    isOptimizing,
    optimizeResult,
    optimizeError,
    // 配置
    config,
    // 方法
    setProvider,
    setModel,
    setAspectRatio,
    setBatchMode,
    startGeneration,
    updateProgress,
    updateElapsedTime,
    generationSuccess,
    generationFailed,
    resetState,
    setConfig,
    // 批量生成方法
    startBatchGeneration,
    updateBatchProgress,
    addBatchResult,
    finishBatchGeneration,
    resetBatchState,
  };
});
