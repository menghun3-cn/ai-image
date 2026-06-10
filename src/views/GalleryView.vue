<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, nextTick } from "vue";
import { useGalleryStore } from "@/stores/gallery";
import { openOutputDir, loadConfig } from "@/lib/tauri";
import Dialog from "@/components/Dialog.vue";
import { TrashIcon, FolderOpenIcon, RefreshCwIcon, ImageIcon, ChevronLeftIcon, ChevronRightIcon, XIcon, Loader2Icon } from "lucide-vue-next";
import { formatTime } from "@/lib/utils";
import { readFile } from "@tauri-apps/plugin-fs";

// Store
const store = useGalleryStore();

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
const outputDir = ref("images");
const isLoadingMore = ref(false);
const selectedImage = ref<typeof store.displayedImages[0] | null>(null);
const selectedIndex = ref<number>(-1);
const imageGridRef = ref<HTMLElement | null>(null);

// 从 Store 获取状态
const allImages = computed(() => store.allImages);
const displayedImages = computed(() => store.displayedImages);
const isLoading = computed(() => store.isLoading);
const loadedCount = computed(() => store.displayedCount);
const hasMoreImages = computed(() => store.hasMoreImages);

// 加载配置
const INITIAL_LOAD_COUNT = 24; // 初始加载数量
const CHUNK_SIZE = 12; // 每批加载数量

// 缓存图片 URL
const imageUrlCache = new Map<string, string>();

// IntersectionObserver 实例
let imageObserver: IntersectionObserver | null = null;

// 辅助函数：将 Uint8Array 转换为 Base64（浏览器兼容）
function arrayBufferToBase64(buffer: Uint8Array): string {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  const len = bytes.byteLength;
  // 分块处理避免栈溢出
  const chunkSize = 0x8000; // 32KB chunks
  for (let i = 0; i < len; i += chunkSize) {
    const chunk = bytes.subarray(i, i + chunkSize);
    binary += String.fromCharCode.apply(null, chunk as unknown as number[]);
  }
  return btoa(binary);
}

// 辅助函数：将文件路径转换为可访问的 URL（使用 Base64）
async function loadImageUrl(path: string): Promise<string> {
  // 检查缓存
  if (imageUrlCache.has(path)) {
    return imageUrlCache.get(path)!;
  }
  
  try {
    // 读取文件并转换为 Base64
    const data = await readFile(path);
    const ext = path.split('.').pop()?.toLowerCase() || 'png';
    const mimeType = ext === 'jpg' || ext === 'jpeg' ? 'image/jpeg' : 
                     ext === 'webp' ? 'image/webp' : 'image/png';
    
    // 使用 ArrayBuffer 转 Base64 避免栈溢出
    const base64 = arrayBufferToBase64(data);
    const url = `data:${mimeType};base64,${base64}`;
    
    // 缓存 URL
    imageUrlCache.set(path, url);
    return url;
  } catch (e) {
    console.error("[Gallery] Failed to load image:", path, e);
    return '';
  }
}

onMounted(async () => {
  // 从配置加载输出目录
  try {
    const config = await loadConfig();
    console.log("[Gallery] Loaded config:", config);
    if (config?.default_output_dir) {
      outputDir.value = config.default_output_dir;
      console.log("[Gallery] Set outputDir to:", outputDir.value);
    } else {
      console.log("[Gallery] Using default outputDir: images");
    }
  } catch (e) {
    console.error("Failed to load config:", e);
  }
  
  // 加载图片（带缓存）
  await loadImages();
});

onUnmounted(() => {
  removeKeyListener();
  // 清理 IntersectionObserver
  if (imageObserver) {
    imageObserver.disconnect();
    imageObserver = null;
  }
});

// 加载图片（带缓存）
async function loadImages(forceRefresh: boolean = false) {
  const dir = outputDir.value || "images";
  
  // 调用 Store 加载，返回 true 表示使用了缓存
  const usedCache = await store.loadImages(dir, forceRefresh);
  
  if (usedCache) {
    console.log("[Gallery] 使用缓存，无需重新加载");
    // 恢复 IntersectionObserver
    nextTick(() => {
      initIntersectionObserver();
    });
  } else {
    // 新加载的数据，需要加载初始批次
    await loadMoreImages(INITIAL_LOAD_COUNT);
    
    // 初始化 IntersectionObserver
    initIntersectionObserver();
    
    // 如果还有更多图片，在后台继续加载
    if (hasMoreImages.value) {
      loadRemainingInBackground();
    }
  }
}

// 初始化 IntersectionObserver 用于可视区域优先加载
function initIntersectionObserver() {
  if (imageObserver) {
    imageObserver.disconnect();
  }
  
  imageObserver = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        const path = entry.target.getAttribute('data-path');
        if (path) {
          // 找到对应的图片并优先加载
          const imageItem = displayedImages.value.find(img => img.path === path);
          if (imageItem && !imageItem.url && !imageItem.loading) {
            loadImageItem(imageItem);
          }
        }
      }
    });
  }, {
    root: null,
    rootMargin: '100px', // 提前 100px 开始加载
    threshold: 0.1
  });
  
  // 观察所有未加载的图片元素
  nextTick(() => {
    const imageElements = document.querySelectorAll('[data-path]');
    imageElements.forEach(el => {
      const path = el.getAttribute('data-path');
      const imageItem = displayedImages.value.find(img => img.path === path);
      if (imageItem && !imageItem.url) {
        imageObserver?.observe(el);
      }
    });
  });
}

// 加载单个图片
async function loadImageItem(imageItem: typeof store.displayedImages[0]) {
  if (imageItem.loading || imageItem.url) return;
  
  store.setImageLoading(imageItem.path, true);
  
  try {
    const url = await loadImageUrl(imageItem.path);
    store.setImageUrl(imageItem.path, url);
  } catch (e) {
    console.error("[Gallery] Failed to load image item:", e);
  } finally {
    store.setImageLoading(imageItem.path, false);
  }
}

// 加载更多图片
async function loadMoreImages(count: number = CHUNK_SIZE) {
  if (isLoadingMore.value) return;
  if (loadedCount.value >= allImages.value.length) return;
  
  isLoadingMore.value = true;
  
  const start = loadedCount.value;
  const end = Math.min(start + count, allImages.value.length);
  const batch = allImages.value.slice(start, end);
  
  try {
    // 并行加载这一批图片
    await Promise.all(
      batch.map(async (img) => {
        if (!img.url) {
          const url = await loadImageUrl(img.path);
          store.setImageUrl(img.path, url);
        }
      })
    );
    
    // 更新 Store 中的显示数量
    store.loadMore(end - start);
    
    console.log(`[Gallery] Loaded batch: ${start} - ${end}, total loaded: ${loadedCount.value}`);
    
    // 重新初始化 IntersectionObserver
    nextTick(() => {
      initIntersectionObserver();
    });
  } catch (e) {
    console.error("[Gallery] Failed to load image batch:", e);
  } finally {
    isLoadingMore.value = false;
  }
}

// 后台继续加载剩余图片
async function loadRemainingInBackground() {
  while (loadedCount.value < allImages.value.length) {
    // 使用 setTimeout 让出主线程，避免阻塞 UI
    await new Promise(resolve => setTimeout(resolve, 100));
    await loadMoreImages(CHUNK_SIZE);
  }
  console.log("[Gallery] All images loaded:", loadedCount.value);
}

// 手动加载更多
async function handleLoadMore() {
  await loadMoreImages(CHUNK_SIZE);
}

async function handleDelete(path: string) {
  const confirmed = await showDialog({
    title: "确认删除",
    message: "确定要删除这张图片吗？",
    type: "warning",
    showCancel: true,
  });
  if (!confirmed) return;

  try {
    // 如果删除的是当前预览的图片，关闭预览
    if (selectedImage.value?.path === path) {
      closeImageModal();
    }
    // 从本地缓存中移除
    imageUrlCache.delete(path);
    // 从 Store 中删除
    await store.removeImage(path);
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
    await openOutputDir(outputDir.value);
  } catch (e) {
    console.error("Failed to open dir:", e);
  }
}

function openImageModal(image: typeof store.displayedImages[0], index: number) {
  selectedImage.value = image;
  selectedIndex.value = index;
  addKeyListener();
}

function closeImageModal() {
  selectedImage.value = null;
  selectedIndex.value = -1;
  removeKeyListener();
}

function goToPreviousImage() {
  if (selectedIndex.value > 0) {
    selectedIndex.value--;
    selectedImage.value = displayedImages.value[selectedIndex.value];
  }
}

function goToNextImage() {
  if (selectedIndex.value < displayedImages.value.length - 1) {
    selectedIndex.value++;
    selectedImage.value = displayedImages.value[selectedIndex.value];
  }
}

function handleKeyDown(event: KeyboardEvent) {
  if (event.key === "ArrowLeft") {
    event.preventDefault();
    goToPreviousImage();
  } else if (event.key === "ArrowRight") {
    event.preventDefault();
    goToNextImage();
  } else if (event.key === "Escape") {
    closeImageModal();
  }
}

function addKeyListener() {
  document.addEventListener("keydown", handleKeyDown);
}

function removeKeyListener() {
  document.removeEventListener("keydown", handleKeyDown);
}
</script>

<template>
  <div class="p-6">
    <div class="flex items-center justify-between mb-6">
      <div>
        <h1 class="text-2xl font-bold">图库</h1>
        <p class="text-sm text-muted-foreground mt-1">
          共 {{ allImages.length }} 张图片
          <span v-if="loadedCount < allImages.length" class="text-primary">
            （已加载 {{ loadedCount }} 张）
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
          @click="() => loadImages(true)"
          :disabled="isLoading"
          class="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50"
        >
          <RefreshCwIcon :class="['w-4 h-4', { 'animate-spin': isLoading }]" />
          刷新
        </button>
      </div>
    </div>

    <!-- Empty State -->
    <div v-if="allImages.length === 0 && !isLoading" class="text-center py-20">
      <ImageIcon class="w-16 h-16 mx-auto text-muted-foreground mb-4" />
      <p class="text-muted-foreground">暂无图片</p>
      <p class="text-xs text-muted-foreground mt-2">生成的图片将显示在这里</p>
    </div>

    <!-- Loading State -->
    <div v-else-if="isLoading" class="text-center py-20">
      <Loader2Icon class="w-12 h-12 mx-auto text-primary animate-spin mb-4" />
      <p class="text-muted-foreground">正在加载图片...</p>
    </div>

    <!-- Image Grid -->
    <div v-else class="space-y-4">
      <div ref="imageGridRef" class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
        <div
          v-for="(image, index) in displayedImages"
          :key="image.path"
          :data-path="image.path"
          class="group relative border rounded-lg overflow-hidden hover:shadow-lg transition-shadow"
        >
          <img
            v-if="image.url"
            :src="image.url"
            class="w-full aspect-square object-cover cursor-pointer"
            @click="openImageModal(image, index)"
            loading="lazy"
          />
          <div
            v-else
            class="w-full aspect-square bg-muted flex items-center justify-center"
          >
            <Loader2Icon class="w-6 h-6 animate-spin text-muted-foreground" />
          </div>
          <!-- 删除按钮 - 右上角 -->
          <button
            @click.stop="handleDelete(image.path)"
            class="absolute top-2 right-2 p-1.5 bg-black/50 hover:bg-red-500 text-white rounded-full opacity-0 group-hover:opacity-100 transition-opacity"
            title="删除"
          >
            <TrashIcon class="w-3.5 h-3.5" />
          </button>
          <div class="p-2 bg-card">
            <p class="text-xs text-muted-foreground truncate">{{ image.name }}</p>
            <p class="text-xs text-muted-foreground">{{ formatTime(image.time) }}</p>
          </div>
        </div>
        
        <!-- 骨架屏占位（未加载的图片） -->
        <div
          v-for="i in Math.min(allImages.length - displayedImages.length, 8)"
          :key="`skeleton-${i}`"
          class="border rounded-lg overflow-hidden"
        >
          <div class="w-full aspect-square bg-muted animate-pulse" />
          <div class="p-2 bg-card space-y-2">
            <div class="h-3 bg-muted rounded animate-pulse w-3/4" />
            <div class="h-3 bg-muted rounded animate-pulse w-1/2" />
          </div>
        </div>
      </div>
      
      <!-- 加载更多按钮 -->
      <div v-if="hasMoreImages && !isLoadingMore" class="text-center py-4">
        <button
          @click="handleLoadMore"
          class="px-6 py-2 border rounded-lg hover:bg-muted transition-colors"
        >
          加载更多（剩余 {{ allImages.length - loadedCount }} 张）
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

    <!-- Image Modal -->
    <div
      v-if="selectedImage"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/90 backdrop-blur-sm"
      @click="closeImageModal"
    >
      <button
        @click.stop="closeImageModal"
        class="absolute top-4 right-4 p-2 text-white hover:bg-white/10 rounded-full"
      >
        <XIcon class="w-6 h-6" />
      </button>
      
      <button
        v-if="selectedIndex > 0"
        @click.stop="goToPreviousImage"
        class="absolute left-4 top-1/2 -translate-y-1/2 p-2 text-white hover:bg-white/10 rounded-full"
      >
        <ChevronLeftIcon class="w-8 h-8" />
      </button>
      
      <button
        v-if="selectedIndex < displayedImages.length - 1"
        @click.stop="goToNextImage"
        class="absolute right-4 top-1/2 -translate-y-1/2 p-2 text-white hover:bg-white/10 rounded-full"
      >
        <ChevronRightIcon class="w-8 h-8" />
      </button>
      
      <img
        :src="selectedImage.url"
        class="max-w-[90vw] max-h-[90vh] object-contain"
        @click.stop
      />
    </div>

    <Dialog
      :show="dialog.show"
      :title="dialog.title"
      :message="dialog.message"
      :type="dialog.type"
      :show-cancel="dialog.showCancel"
      @confirm="handleDialogConfirm"
      @cancel="handleDialogCancel"
    />
  </div>
</template>
