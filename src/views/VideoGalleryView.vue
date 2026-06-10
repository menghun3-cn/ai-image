<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, nextTick } from "vue";
import { useVideoGalleryStore } from "@/stores/videoGallery";
import { openVideoDir, loadConfig } from "@/lib/tauri";
import Dialog from "@/components/Dialog.vue";
import { TrashIcon, FolderOpenIcon, RefreshCwIcon, VideoIcon, XIcon, PlayIcon, Loader2Icon } from "lucide-vue-next";
import { formatTime } from "@/lib/utils";
import { readFile } from "@tauri-apps/plugin-fs";

// Store
const store = useVideoGalleryStore();

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

// 本地状态
const outputDir = ref("video");
const isLoadingMore = ref(false);
const selectedVideo = ref<typeof store.displayedVideos[0] | null>(null);
const isPlaying = ref(false);
const videoPlayerRef = ref<HTMLVideoElement | null>(null);
const previewBlobUrl = ref<string | null>(null);
const videoGridRef = ref<HTMLElement | null>(null);

// 从 Store 获取状态
const allVideos = computed(() => store.allVideos);
const displayedVideos = computed(() => store.displayedVideos);
const isLoading = computed(() => store.isLoading);
const loadedCount = computed(() => store.displayedCount);
const hasMoreVideos = computed(() => store.hasMoreVideos);

// 加载配置
const INITIAL_LOAD_COUNT = 12; // 初始加载数量（视频比图片大，数量减少）
const CHUNK_SIZE = 6; // 每批加载数量

// IntersectionObserver 实例
let videoObserver: IntersectionObserver | null = null;

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
  
  // 加载视频（带缓存）
  await loadVideos();
});

onUnmounted(() => {
  // 清理预览用的 blob URL
  if (previewBlobUrl.value) {
    URL.revokeObjectURL(previewBlobUrl.value);
  }
  // 清理 IntersectionObserver
  if (videoObserver) {
    videoObserver.disconnect();
    videoObserver = null;
  }
  removeKeyListener();
});

// 加载视频（带缓存）
async function loadVideos(forceRefresh: boolean = false) {
  const dir = outputDir.value || "video";
  
  // 调用 Store 加载，返回 true 表示使用了缓存
  const usedCache = await store.loadVideos(dir, forceRefresh);
  
  if (usedCache) {
    console.log("[VideoGallery] 使用缓存，无需重新加载");
    // 恢复 IntersectionObserver
    nextTick(() => {
      initIntersectionObserver();
    });
  } else {
    // 新加载的数据，需要加载初始批次
    await loadMoreVideos(INITIAL_LOAD_COUNT);
    
    // 初始化 IntersectionObserver
    initIntersectionObserver();
    
    // 如果还有更多视频，在后台继续加载
    if (hasMoreVideos.value) {
      loadRemainingInBackground();
    }
  }
}

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

// 初始化 IntersectionObserver 用于可视区域优先加载
function initIntersectionObserver() {
  if (videoObserver) {
    videoObserver.disconnect();
  }
  
  videoObserver = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        const path = entry.target.getAttribute('data-path');
        if (path) {
          // 找到对应的视频并优先加载
          const videoItem = displayedVideos.value.find(v => v.path === path);
          if (videoItem && !videoItem.blobUrl && !videoItem.loading) {
            loadVideoItem(videoItem);
          }
        }
      }
    });
  }, {
    root: null,
    rootMargin: '100px', // 提前 100px 开始加载
    threshold: 0.1
  });
  
  // 观察所有未加载的视频元素
  nextTick(() => {
    const videoElements = document.querySelectorAll('[data-path]');
    videoElements.forEach(el => {
      const path = el.getAttribute('data-path');
      const videoItem = displayedVideos.value.find(v => v.path === path);
      if (videoItem && !videoItem.blobUrl) {
        videoObserver?.observe(el);
      }
    });
  });
}

// 加载单个视频
async function loadVideoItem(videoItem: typeof store.displayedVideos[0]) {
  if (videoItem.loading || videoItem.blobUrl) return;
  
  store.setVideoLoading(videoItem.path, true);
  
  try {
    console.log("[VideoGallery] Loading video as blob:", videoItem.path);
    const fileData = await readFile(videoItem.path);
    const blob = new Blob([fileData], { type: 'video/mp4' });
    const blobUrl = URL.createObjectURL(blob);
    store.setVideoBlobUrl(videoItem.path, blobUrl);
    console.log("[VideoGallery] Created blob URL:", blobUrl);
  } catch (error) {
    console.error("[VideoGallery] Failed to load video as blob:", error);
  } finally {
    store.setVideoLoading(videoItem.path, false);
  }
}

// 加载更多视频
async function loadMoreVideos(count: number = CHUNK_SIZE) {
  if (isLoadingMore.value) return;
  if (loadedCount.value >= allVideos.value.length) return;
  
  isLoadingMore.value = true;
  
  const start = loadedCount.value;
  const end = Math.min(start + count, allVideos.value.length);
  const batch = allVideos.value.slice(start, end);
  
  try {
    // 并行加载这一批视频
    await Promise.all(
      batch.map(async (video) => {
        if (!video.blobUrl) {
          const blobUrl = await loadVideoAsBlob(video.path);
          if (blobUrl) {
            store.setVideoBlobUrl(video.path, blobUrl);
          }
        }
      })
    );
    
    // 更新 Store 中的显示数量
    store.loadMore(end - start);
    
    console.log(`[VideoGallery] Loaded batch: ${start} - ${end}, total loaded: ${loadedCount.value}`);
    
    // 重新初始化 IntersectionObserver
    nextTick(() => {
      initIntersectionObserver();
    });
  } catch (e) {
    console.error("[VideoGallery] Failed to load video batch:", e);
  } finally {
    isLoadingMore.value = false;
  }
}

// 后台继续加载剩余视频
async function loadRemainingInBackground() {
  while (loadedCount.value < allVideos.value.length) {
    // 使用 setTimeout 让出主线程，避免阻塞 UI
    await new Promise(resolve => setTimeout(resolve, 200));
    await loadMoreVideos(CHUNK_SIZE);
  }
  console.log("[VideoGallery] All videos loaded:", loadedCount.value);
}

// 手动加载更多
async function handleLoadMore() {
  await loadMoreVideos(CHUNK_SIZE);
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
    // 如果删除的是当前预览的视频，关闭预览
    if (selectedVideo.value?.path === path) {
      closeVideoModal();
    }
    // 从 Store 中删除
    await store.removeVideo(path);
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

async function openVideoModal(video: typeof store.displayedVideos[0]) {
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
      // 同时更新 store 中的 blobUrl
      store.setVideoBlobUrl(video.path, blobUrl);
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
        <p class="text-sm text-muted-foreground mt-1">
          共 {{ allVideos.length }} 个视频
          <span v-if="loadedCount < allVideos.length" class="text-primary">
            （已加载 {{ loadedCount }} 个）
          </span>
        </p>
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
          @click="() => loadVideos(true)"
          :disabled="isLoading"
          class="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50"
        >
          <RefreshCwIcon :class="['w-4 h-4', { 'animate-spin': isLoading }]" />
          刷新
        </button>
      </div>
    </div>

    <!-- Empty State -->
    <div v-if="allVideos.length === 0 && !isLoading" class="text-center py-20">
      <VideoIcon class="w-16 h-16 mx-auto text-muted-foreground mb-4" />
      <p class="text-muted-foreground">暂无视频</p>
      <p class="text-xs text-muted-foreground mt-2">生成的视频将显示在这里</p>
    </div>

    <!-- Loading State -->
    <div v-else-if="isLoading" class="text-center py-20">
      <Loader2Icon class="w-12 h-12 mx-auto text-primary animate-spin mb-4" />
      <p class="text-muted-foreground">正在加载视频...</p>
    </div>

    <!-- Video Grid -->
    <div v-else class="space-y-4">
      <div ref="videoGridRef" class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <div
          v-for="video in displayedVideos"
          :key="video.path"
          :data-path="video.path"
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
            <div v-else class="flex items-center justify-center">
              <Loader2Icon class="w-8 h-8 animate-spin text-white/50" />
            </div>
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
        
        <!-- 骨架屏占位（未加载的视频） -->
        <div
          v-for="i in Math.min(allVideos.length - displayedVideos.length, 6)"
          :key="`skeleton-${i}`"
          class="border rounded-lg overflow-hidden"
        >
          <div class="w-full aspect-video bg-muted animate-pulse" />
          <div class="p-3 bg-card space-y-2">
            <div class="h-4 bg-muted rounded animate-pulse w-3/4" />
            <div class="h-3 bg-muted rounded animate-pulse w-1/2" />
          </div>
        </div>
      </div>
      
      <!-- 加载更多按钮 -->
      <div v-if="hasMoreVideos && !isLoadingMore" class="text-center py-4">
        <button
          @click="handleLoadMore"
          class="px-6 py-2 border rounded-lg hover:bg-muted transition-colors"
        >
          加载更多（剩余 {{ allVideos.length - loadedCount }} 个）
        </button>
      </div>
      
      <!-- 正在加载更多 -->
      <div v-else-if="isLoadingMore" class="text-center py-4">
        <div class="flex items-center justify-center gap-2 text-muted-foreground">
          <Loader2Icon class="w-4 h-4 animate-spin" />
          <span>正在加载...</span>
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
            <Loader2Icon class="w-12 h-12 animate-spin text-white/30" />
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
