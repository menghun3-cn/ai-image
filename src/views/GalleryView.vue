<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { getImages, deleteImage, openOutputDir, loadConfig } from "@/lib/tauri";
import { message, confirm } from "@tauri-apps/plugin-dialog";
import { TrashIcon, FolderOpenIcon, RefreshCwIcon, ImageIcon, ChevronLeftIcon, ChevronRightIcon, XIcon } from "lucide-vue-next";
import { formatTime } from "@/lib/utils";
import { readFile } from "@tauri-apps/plugin-fs";

interface ImageItem {
  path: string;
  name: string;
  time: number;
  url?: string;
}

// 缓存图片 URL
const imageUrlCache = new Map<string, string>();

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

const images = ref<ImageItem[]>([]);
const outputDir = ref("images");
const isLoading = ref(false);
const selectedImage = ref<ImageItem | null>(null);
const selectedIndex = ref<number>(-1);

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
  await loadImages();
});

onUnmounted(() => {
  removeKeyListener();
});

async function loadImages() {
  isLoading.value = true;
  try {
    // 确保 outputDir 有值
    const dir = outputDir.value || "images";
    console.log("[Gallery] Loading images from:", dir);
    const loadedImages = await getImages(dir);
    console.log("[Gallery] Loaded images:", loadedImages.length);
    
    // 为每张图片加载 URL
    images.value = await Promise.all(
      loadedImages.map(async (img) => ({
        ...img,
        url: await loadImageUrl(img.path)
      }))
    );
    
    console.log("[Gallery] Images with URLs loaded:", images.value.length);
  } catch (e) {
    console.error("Failed to load images:", e);
  } finally {
    isLoading.value = false;
  }
}

async function handleDelete(path: string) {
  const confirmed = await confirm("确定要删除这张图片吗？", { title: "确认删除", kind: "warning" });
  if (!confirmed) return;

  try {
    await deleteImage(path);
    // 如果删除的是当前预览的图片，关闭预览
    if (selectedImage.value?.path === path) {
      closeImageModal();
    }
    await loadImages();
  } catch (e) {
    await message("删除失败: " + String(e), { title: "错误", kind: "error" });
  }
}

async function handleOpenDir() {
  try {
    await openOutputDir(outputDir.value);
  } catch (e) {
    console.error("Failed to open dir:", e);
  }
}

function openImageModal(image: ImageItem, index: number) {
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
    selectedImage.value = images.value[selectedIndex.value];
  }
}

function goToNextImage() {
  if (selectedIndex.value < images.value.length - 1) {
    selectedIndex.value++;
    selectedImage.value = images.value[selectedIndex.value];
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
        <p class="text-sm text-muted-foreground mt-1">共 {{ images.length }} 张图片</p>
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
          @click="loadImages"
          :disabled="isLoading"
          class="flex items-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 disabled:opacity-50"
        >
          <RefreshCwIcon :class="['w-4 h-4', { 'animate-spin': isLoading }]" />
          刷新
        </button>
      </div>
    </div>

    <!-- Empty State -->
    <div v-if="images.length === 0 && !isLoading" class="text-center py-20">
      <ImageIcon class="w-16 h-16 mx-auto text-muted-foreground mb-4" />
      <p class="text-muted-foreground">暂无图片</p>
      <p class="text-xs text-muted-foreground mt-2">生成的图片将显示在这里</p>
    </div>

    <!-- Image Grid -->
    <div v-else class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4">
      <div
        v-for="(image, index) in images"
        :key="image.path"
        class="group relative border rounded-lg overflow-hidden hover:shadow-lg transition-shadow"
      >
        <img
          :src="image.url"
          class="w-full aspect-square object-cover cursor-pointer"
          @click="openImageModal(image, index)"
        />
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
    </div>

    <!-- Image Modal -->
    <div
      v-if="selectedImage"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/90 backdrop-blur-sm"
      @click="closeImageModal"
    >
      <div class="relative max-w-[90vw] max-h-[90vh] p-4" @click.stop>
        <img
          :src="selectedImage?.url"
          alt="Preview"
          class="max-w-full max-h-[85vh] object-contain rounded-lg shadow-2xl"
        />
        
        <!-- 图片信息 -->
        <div class="absolute bottom-6 left-1/2 -translate-x-1/2 px-4 py-2 rounded-lg bg-black/50 text-white text-sm">
          <span>{{ selectedImage.name }}</span>
          <span class="mx-2">|</span>
          <span>{{ selectedIndex + 1 }} / {{ images.length }}</span>
        </div>

        <!-- 关闭按钮 -->
        <button
          @click="closeImageModal"
          class="absolute top-6 right-6 w-10 h-10 rounded-full bg-black/50 hover:bg-black/70 text-white flex items-center justify-center transition-colors"
        >
          <XIcon class="w-5 h-5" />
        </button>

        <!-- 删除按钮 -->
        <button
          @click.stop="handleDelete(selectedImage.path)"
          class="absolute top-6 right-20 w-10 h-10 rounded-full bg-red-500/80 hover:bg-red-500 text-white flex items-center justify-center transition-colors"
        >
          <TrashIcon class="w-4 h-4" />
        </button>

        <!-- 上一张按钮 -->
        <button
          v-if="selectedIndex > 0"
          @click.stop="goToPreviousImage"
          class="absolute left-4 top-1/2 -translate-y-1/2 w-12 h-12 rounded-full bg-black/50 hover:bg-black/70 text-white flex items-center justify-center transition-colors"
        >
          <ChevronLeftIcon class="w-6 h-6" />
        </button>

        <!-- 下一张按钮 -->
        <button
          v-if="selectedIndex < images.length - 1"
          @click.stop="goToNextImage"
          class="absolute right-4 top-1/2 -translate-y-1/2 w-12 h-12 rounded-full bg-black/50 hover:bg-black/70 text-white flex items-center justify-center transition-colors"
        >
          <ChevronRightIcon class="w-6 h-6" />
        </button>
      </div>
    </div>
  </div>
</template>
