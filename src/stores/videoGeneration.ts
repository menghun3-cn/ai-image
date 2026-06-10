import { defineStore } from "pinia";
import { ref, computed } from "vue";
import type { ReferenceImage } from "@/lib/tauri";

export type VideoGenerationMode = "text" | "single" | "multi" | "keyframes";
export type VideoGenerationStatus = "idle" | "creating" | "processing" | "downloading" | "success" | "error";

// 视频时长预设
export const durationPresets = [
  { label: "3 秒", frames: 81, fps: 24 },
  { label: "5 秒", frames: 121, fps: 24 },
  { label: "10 秒", frames: 241, fps: 24 },
  { label: "18 秒", frames: 441, fps: 24 },
];

// 分辨率预设
export const resolutionPresets = [
  { label: "1152 x 768 (16:9)", width: 1152, height: 768 },
  { label: "768 x 1152 (9:16)", width: 768, height: 1152 },
  { label: "1024 x 1024 (1:1)", width: 1024, height: 1024 },
];

export const useVideoGenerationStore = defineStore("videoGeneration", () => {
  // 生成状态
  const isGenerating = ref(false);
  const status = ref<VideoGenerationStatus>("idle");
  const progress = ref(0);
  const resultVideoPath = ref<string | null>(null);
  const errorMessage = ref<string | null>(null);

  // 生成时间统计
  const startTime = ref<number | null>(null);
  const elapsedTime = ref(0);

  // 生成参数
  const prompt = ref(localStorage.getItem("lastVideoPrompt") || "");
  const referenceImages = ref<ReferenceImage[]>([]);
  const isKeyframesMode = ref(false);

  // 从 localStorage 加载参考图片
  function loadReferenceImages() {
    try {
      const saved = localStorage.getItem("videoReferenceImages");
      if (saved) {
        referenceImages.value = JSON.parse(saved);
      }
    } catch (e) {
      console.error("[videoGeneration store] Failed to load reference images:", e);
      referenceImages.value = [];
    }
  }

  // 保存参考图片到 localStorage
  function saveReferenceImages() {
    try {
      localStorage.setItem("videoReferenceImages", JSON.stringify(referenceImages.value));
    } catch (e) {
      console.error("[videoGeneration store] Failed to save reference images:", e);
    }
  }

  // 视频参数
  const selectedDurationIndex = ref(1); // 默认5秒
  const selectedResolutionIndex = ref(0); // 默认1152x768
  const seed = ref<number | undefined>(undefined);
  const negativePrompt = ref("");
  const showAdvanced = ref(false);

  // 计算属性
  const generationMode = computed<VideoGenerationMode>(() => {
    const count = referenceImages.value.length;
    if (count === 0) return "text";
    if (count === 1) return "single";
    if (isKeyframesMode.value) return "keyframes";
    return "multi";
  });

  const modeDisplayText = computed(() => {
    switch (generationMode.value) {
      case "text":
        return "文生视频";
      case "single":
        return "单图生视频";
      case "multi":
        return "多图生视频";
      case "keyframes":
        return "关键帧模式";
      default:
        return "文生视频";
    }
  });

  const currentDuration = computed(() => durationPresets[selectedDurationIndex.value]);
  const currentResolution = computed(() => resolutionPresets[selectedResolutionIndex.value]);

  const numFrames = computed(() => currentDuration.value.frames);
  const frameRate = computed(() => currentDuration.value.fps);
  const width = computed(() => currentResolution.value.width);
  const height = computed(() => currentResolution.value.height);

  const videoDuration = computed(() => {
    return (numFrames.value / frameRate.value).toFixed(1);
  });

  const formattedElapsedTime = computed(() => {
    const minutes = Math.floor(elapsedTime.value / 60);
    const seconds = elapsedTime.value % 60;
    return minutes > 0 ? `${minutes}分${seconds}秒` : `${seconds}秒`;
  });

  const statusText = computed(() => {
    const timeStr = elapsedTime.value > 0 ? ` (${formattedElapsedTime.value})` : "";
    const progressStr = isGenerating.value ? ` ${progress.value}%` : "";
    switch (status.value) {
      case "creating":
        return `创建任务中...${timeStr}`;
      case "processing":
        return `视频生成中${progressStr}（这可能需要几分钟）...${timeStr}`;
      case "downloading":
        return `下载视频中...${timeStr}`;
      case "success":
        return `生成成功！总耗时: ${formattedElapsedTime.value}`;
      case "error":
        return `生成失败${timeStr}`;
      default:
        return "";
    }
  });

  const canGenerate = computed(() => {
    return prompt.value.trim().length > 0 && !isGenerating.value;
  });

  // 方法
  function setPrompt(value: string) {
    prompt.value = value;
    localStorage.setItem("lastVideoPrompt", value);
  }

  function setReferenceImages(images: ReferenceImage[]) {
    referenceImages.value = images;
    saveReferenceImages();
  }

  function addReferenceImage(image: ReferenceImage) {
    if (referenceImages.value.length < 5) {
      referenceImages.value = [...referenceImages.value, image];
      saveReferenceImages();
    }
  }

  function removeReferenceImage(id: string) {
    referenceImages.value = referenceImages.value.filter((img) => img.id !== id);
    saveReferenceImages();
  }

  function clearReferenceImages() {
    referenceImages.value = [];
    saveReferenceImages();
  }

  function setKeyframesMode(value: boolean) {
    isKeyframesMode.value = value;
  }

  function setDurationPreset(index: number) {
    selectedDurationIndex.value = index;
  }

  function setResolutionPreset(index: number) {
    selectedResolutionIndex.value = index;
  }

  function setSeed(value: number | undefined) {
    seed.value = value;
  }

  function setNegativePrompt(value: string) {
    negativePrompt.value = value;
  }

  function setShowAdvanced(value: boolean) {
    showAdvanced.value = value;
  }

  function startGeneration() {
    isGenerating.value = true;
    status.value = "creating";
    progress.value = 0;
    errorMessage.value = null;
    resultVideoPath.value = null;
    startTime.value = Date.now();
    elapsedTime.value = 0;
  }

  function updateProgress(value: number) {
    progress.value = value;
  }

  function updateElapsedTime() {
    if (startTime.value) {
      elapsedTime.value = Math.floor((Date.now() - startTime.value) / 1000);
    }
  }

  function setStatus(newStatus: VideoGenerationStatus) {
    status.value = newStatus;
  }

  function generationSuccess(videoPath: string) {
    isGenerating.value = false;
    status.value = "success";
    resultVideoPath.value = videoPath;
    progress.value = 100;
  }

  function generationFailed(error: string) {
    isGenerating.value = false;
    status.value = "error";
    errorMessage.value = error;
  }

  function resetState() {
    isGenerating.value = false;
    status.value = "idle";
    progress.value = 0;
    errorMessage.value = null;
    resultVideoPath.value = null;
    startTime.value = null;
    elapsedTime.value = 0;
  }

  // 生成选项（用于调用 generateVideo）
  function getGenerationOptions() {
    const options: any = {
      prompt: prompt.value.trim(),
      width: width.value,
      height: height.value,
      num_frames: numFrames.value,
      frame_rate: frameRate.value,
      image_mode: generationMode.value,
    };

    if (seed.value !== undefined) {
      options.seed = seed.value;
    }

    if (negativePrompt.value.trim()) {
      options.negative_prompt = negativePrompt.value.trim();
    }

    // 根据图片数量添加参数
    const imageCount = referenceImages.value.length;
    if (imageCount === 1) {
      options.image = referenceImages.value[0].source;
    } else if (imageCount >= 2) {
      options.images = referenceImages.value.map((item) => item.source);
    }

    return options;
  }

  return {
    // 状态
    isGenerating,
    status,
    progress,
    resultVideoPath,
    errorMessage,
    elapsedTime,
    formattedElapsedTime,
    statusText,
    // 参数
    prompt,
    referenceImages,
    isKeyframesMode,
    selectedDurationIndex,
    selectedResolutionIndex,
    seed,
    negativePrompt,
    showAdvanced,
    // 计算属性
    generationMode,
    modeDisplayText,
    currentDuration,
    currentResolution,
    numFrames,
    frameRate,
    width,
    height,
    videoDuration,
    canGenerate,
    // 方法
    setPrompt,
    setReferenceImages,
    addReferenceImage,
    removeReferenceImage,
    clearReferenceImages,
    loadReferenceImages,
    setKeyframesMode,
    setDurationPreset,
    setResolutionPreset,
    setSeed,
    setNegativePrompt,
    setShowAdvanced,
    startGeneration,
    updateProgress,
    updateElapsedTime,
    setStatus,
    generationSuccess,
    generationFailed,
    resetState,
    getGenerationOptions,
  };
});
