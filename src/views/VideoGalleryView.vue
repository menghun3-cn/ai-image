<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { getVideos, deleteVideo, openVideoDir, loadConfig } from "@/lib/tauri";
import Dialog from "@/components/Dialog.vue";
import { TrashIcon, FolderOpenIcon, RefreshCwIcon, VideoIcon, XIcon, PlayIcon } from "lucide-vue-next";
import { formatTime } from "@/lib/utils";
import { convertFileSrc } from "@tauri-apps/api/core";
import { readFile } from "@tauri-apps/plugin-fs";

// 对话框状态
const dialog = ref({
  show: false,
  title: "",
  message: "",
  type: "info" as "info" | "warning" | "error" | "success",
  showCancel: false,
});

// 对话框确认回调
let dialogResolve: ((value: boolean) => void) | null = null;

function showDialog(options: {
  title: string;
  message: string;
  type?: "info" | "warning" | "error" | "success";
  showCancel?: boolean;
}): Promise<boolean> {
  dialog.value = {
    show: true,
    title: options.title,
    message: options.message,
    type: options.type || "info",
    showCancel: options.showCancel || false,
  };
  return new Promise((resolve) => {
    dialogResolve = resolve;
  });
}

function handleDialogConfirm() {
  dialog.value.show = false;
  dialogResolve?.(true);
  dialogResolve = null;
}

function handleDialogCancel() {
  dialog.value.show = false;
  dialogResolve?.(false);
  dialogResolve = null;
}

interface VideoItem {
  path: string;
  name: string;
  time: number;
  url?: string;
  blobUrl?: string;
}

const videos = ref<VideoItem[]>([]);
const outputDir = ref("video");
const isLoading = ref(false);
const selectedVideo = ref<VideoItem | null>(null);
const isPlaying = ref(false);
const videoPlayerRef = ref<HTMLVideoElement | null>(null);
const previewBlobUrl = ref<string | null>(null);

onMounted(async () => {
  // 从配置加载输出目录
  try {
    const config = await loadConfig();
    console.log("[VideoGallery] Loaded config:", config);
    if (config?.default_video_output_dir) {
      outputDir.value = config.default_video_output_dir;
      console.log("[VideoGallery] Set outputDir to:", outputDir.value);
    } else {
      console.log("[VideoGallery] Using default outputDir: video");
    }
  } catch (e) {
    console.error("Failed to load config:", e);
  }
  await loadVideos();
});

onUnmounted(() => {
  // 清理 blob URL
  videos.value.forEach(video => {
    if (video.blobUrl) {
      URL.revokeObjectURL(video.blobUrl);
    }
  });
  if (previewBlobUrl.value) {
    URL.revokeObjectURL(previewBlobUrl.value);
  }
});

// 加载视频为 Blob URL（更可靠的方式）
async function loadVideoAsBlob(path: string): Promise<string | null> {
  try {
    console.log("[VideoGallery] Loading video as blob:", path);
    const fileData = await readFile(path);
    const blob = new Blob([fileData], { type: 'video/mp4' });
    const url = URL.createObjectURL(blob);
    console.log("[VideoGallery] Created blob URL:", url);
    return url;
  } catch (error) {
    console.error("[VideoGallery] Failed to load video as blob:", error);
    return null;
  }
}

async function loadVideos() {
  isLoading.value = true;
  try {
    // 确保 outputDir 有值
    const dir = outputDir.value || "video";
    console.log("[VideoGallery] Loading videos from:", dir);
    const loadedVideos = await getVideos(dir);
    console.log("[VideoGallery] Loaded videos:", loadedVideos.length);
    
    // 为每个视频生成 blob URL
    const videosWithUrls: VideoItem[] = [];
    for (const video of loadedVideos) {
      const blobUrl = await loadVideoAsBlob(video.path);
      videosWithUrls.push({
        ...video,
        url: convertFileSrc(video.path),
        blobUrl: blobUrl || undefined,
      });
    }
    
    videos.value = videosWithUrls;
    console.log("[VideoGallery] Videos with blob URLs loaded:", videos.value.length);
  } catch (e) {
    console.error("Failed to load videos:", e);
  } finally {
    isLoading.value = false;
  }
}

async function handleDelete(path: string) {
  const confirmed = await showDialog({
    title: "确认删除",
    message: "确定要删除这个视频吗？",
    type: "warning",
    showCancel: true,
  });
  if (!confirmed) return;

  try {
    await deleteVideo(path);
    // 如果删除的是当前预览的视频，关闭预览
    if (selectedVideo.value?.path === path) {
      closeVideoModal();
    }
    await loadVideos();
  } catch (e) {
    await showDialog({
      title: "错误",
      message: "删除失败: " + String(e),
      type: "error",
    });
  }
}

async function handleOpenDir() {
  try {
    await openVideoDir(outputDir.value);
  } catch (e) {
    console.error("Failed to open dir:", e);
  }
}

async function openVideoModal(video: VideoItem) {
  selectedVideo.value = video;
  isPlaying.value = false;
  
  // 使用 blob URL 进行预览（更可靠）
  if (video.blobUrl) {
    previewBlobUrl.value = video.blobUrl;
  } else {
    // 如果还没有 blob URL，创建一个
    const blobUrl = await loadVideoAsBlob(video.path);
    if (blobUrl) {
      previewBlobUrl.value = blobUrl;
    }
  }
  
  addKeyListener();
}

function closeVideoModal() {
  selectedVideo.value = null;
  isPlaying.value = false;
  // 不要在这里释放 blob URL，因为它可能还在列表中使用
  previewBlobUrl.value = null;
  removeKeyListener();
}



function handleKeyDown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    closeVideoModal();
  } else if (event.key === " ") {
    event.preventDefault();
    if (videoPlayerRef.value) {
      if (videoPlayerRef.value.paused) {
        videoPlayerRef.value.play();
      } else {
        videoPlayerRef.value.pause();
      }
    }
  }
}

function addKeyListener() {
  document.addEventListener("keydown", handleKeyDown);
}

function removeKeyListener() {
  document.removeEventListener("keydown", handleKeyDown);
}

// 视频加载错误处理
function handleVideoError(e: Event) {
  const videoEl = e.target as HTMLVideoElement;
  console.error("[VideoGallery] Video error:", videoEl.error);
}
</script>

<template>
  <div class="p-6">
    <div class="flex items-center justify-between mb-6">
      <div>
        <h1 class="text-2xl font-bold">视频库</h1>
        <p class="text-sm text-muted-foreground mt-1">共 {{ videos.length }} 个视频</p>
      </div>
      <div class="flex items-center gap-2">
        <button
          @click="handleOpenDir"
          class="flex items-center gap-2 px-4 py-2 border rounded-lg hover:bg-muted"
        >
          <FolderOpenIcon class="w-4 h-4" />
          打开目录
        </button>
        <button
          @click="loadVideos"
          :disabled="isLoading"
          class="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50"
        >
          <RefreshCwIcon :class="['w-4 h-4', { 'animate-spin': isLoading }]" />
          刷新
        </button>
      </div>
    </div>

    <!-- Empty State -->
    <div v-if="videos.length === 0 && !isLoading" class="text-center py-20">
      <VideoIcon class="w-16 h-16 mx-auto text-muted-foreground mb-4" />
      <p class="text-muted-foreground">暂无视频</p>
      <p class="text-xs text-muted-foreground mt-2">生成的视频将显示在这里</p>
    </div>

    <!-- Video Grid -->
    <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <div
        v-for="video in videos"
        :key="video.path"
        class="group relative border rounded-lg overflow-hidden hover:shadow-lg transition-shadow"
      >
        <!-- 视频缩略图/播放按钮区域 -->
        <div 
          class="w-full aspect-video bg-black flex items-center justify-center cursor-pointer relative overflow-hidden"
          @click="openVideoModal(video)"
        >
          <!-- 使用 video 标签显示第一帧作为封面 -->
          <video
            v-if="video.blobUrl"
            :src="video.blobUrl"
            class="w-full h-full object-cover"
            preload="metadata"
            muted
            playsinline
          ></video>
          <VideoIcon v-else class="w-12 h-12 text-white/50" />
          <!-- 播放按钮覆盖层 -->
          <div class="absolute inset-0 flex items-center justify-center bg-black/30 opacity-0 group-hover:opacity-100 transition-opacity">
            <div class="w-16 h-16 rounded-full bg-white/90 flex items-center justify-center">
              <PlayIcon class="w-8 h-8 text-black ml-1" />
            </div>
          </div>
        </div>
        
        <!-- 删除按钮 - 右上角 -->
        <button
          @click.stop="handleDelete(video.path)"
          class="absolute top-2 right-2 p-1.5 bg-black/50 hover:bg-red-500 text-white rounded-full opacity-0 group-hover:opacity-100 transition-opacity"
          title="删除"
        >
          <TrashIcon class="w-3.5 h-3.5" />
        </button>
        
        <!-- 视频信息 -->
        <div class="p-3 bg-card">
          <p class="text-sm font-medium truncate">{{ video.name }}</p>
          <p class="text-xs text-muted-foreground mt-1">{{ formatTime(video.time) }}</p>
        </div>
      </div>
    </div>

    <!-- Video Modal -->
    <div
      v-if="selectedVideo"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/90 backdrop-blur-sm"
      @click="closeVideoModal"
    >
      <div class="relative max-w-[90vw] max-h-[90vh] w-full" @click.stop>
        <!-- 关闭按钮 -->
        <button
          @click="closeVideoModal"
          class="absolute -top-10 right-0 p-2 text-white hover:text-gray-300 transition-colors"
        >
          <XIcon class="w-6 h-6" />
        </button>
        
        <!-- 视频播放器 -->
        <div class="relative bg-black rounded-lg overflow-hidden">
          <video
            v-if="previewBlobUrl"
            ref="videoPlayerRef"
            :src="previewBlobUrl"
            class="w-full max-h-[80vh]"
            controls
            autoplay
            @error="handleVideoError"
            @click.stop
          ></video>
          
          <!-- 加载中或播放按钮覆盖层 -->
          <div
            v-else
            class="w-full aspect-video flex items-center justify-center"
          >
            <VideoIcon class="w-20 h-20 text-white/30" />
          </div>
        </div>
        
        <!-- 视频信息 -->
        <div class="mt-4 text-white">
          <p class="text-lg font-medium">{{ selectedVideo.name }}</p>
          <p class="text-sm text-gray-400">{{ formatTime(selectedVideo.time) }}</p>
        </div>
      </div>
    </div>

    <!-- Dialog -->
    <Dialog
      v-model:show="dialog.show"
      :title="dialog.title"
      :message="dialog.message"
      :type="dialog.type"
      :show-cancel="dialog.showCancel"
      @confirm="handleDialogConfirm"
      @cancel="handleDialogCancel"
    />
  </div>
</template>
