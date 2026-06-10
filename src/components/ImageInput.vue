<script setup lang="ts">
import { ref, computed } from "vue";
import { PlusIcon, LinkIcon, XIcon, ImageIcon } from "lucide-vue-next";
import { open } from "@tauri-apps/plugin-dialog";
import { readFile } from "@tauri-apps/plugin-fs";

interface ReferenceImage {
  id: string;
  type: "file" | "url";
  source: string; // 文件路径或 URL
  preview: string; // base64 预览
}

const props = defineProps<{
  modelValue: ReferenceImage[];
  disabled?: boolean;
  maxImages?: number;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: ReferenceImage[]): void;
  (e: "maxReached"): void;
}>();

const isDragging = ref(false);
const showUrlInput = ref(false);
const urlInput = ref("");
const urlInputRef = ref<HTMLInputElement | null>(null);

const images = computed({
  get: () => props.modelValue,
  set: (value) => emit("update:modelValue", value),
});

const maxImages = computed(() => props.maxImages ?? Infinity);
const canAddMore = computed(() => images.value.length < maxImages.value);

// 生成唯一 ID
function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

// 将 Uint8Array 转换为 Base64
function arrayBufferToBase64(buffer: Uint8Array): string {
  const bytes = new Uint8Array(buffer);
  let binary = "";
  const len = bytes.byteLength;
  const chunkSize = 0x8000;
  for (let i = 0; i < len; i += chunkSize) {
    const chunk = bytes.subarray(i, i + chunkSize);
    binary += String.fromCharCode.apply(null, chunk as unknown as number[]);
  }
  return btoa(binary);
}

// 获取图片的 MIME 类型
function getMimeType(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase() || "png";
  if (ext === "jpg" || ext === "jpeg") return "image/jpeg";
  if (ext === "webp") return "image/webp";
  if (ext === "gif") return "image/gif";
  if (ext === "bmp") return "image/bmp";
  return "image/png";
}

// 添加本地图片
async function addLocalImage() {
  if (props.disabled || !canAddMore.value) return;

  try {
    const selected = await open({
      multiple: true,
      filters: [
        {
          name: "图片文件",
          extensions: ["png", "jpg", "jpeg", "webp", "gif", "bmp"],
        },
      ],
    });

    if (!selected) return;

    const files = Array.isArray(selected) ? selected : [selected];
    let addedCount = 0;

    for (const filePath of files) {
      // 检查是否已达到最大数量
      if (images.value.length >= maxImages.value) {
        emit("maxReached");
        break;
      }

      try {
        const data = await readFile(filePath);
        const mimeType = getMimeType(filePath);
        const base64 = arrayBufferToBase64(data);
        const preview = `data:${mimeType};base64,${base64}`;

        const newImage: ReferenceImage = {
          id: generateId(),
          type: "file",
          source: filePath,
          preview,
        };

        images.value = [...images.value, newImage];
        addedCount++;
      } catch (e) {
        console.error("读取图片失败:", filePath, e);
      }
    }
  } catch (e) {
    console.error("选择图片失败:", e);
  }
}

// 显示 URL 输入框
function showUrlInputBox() {
  if (props.disabled || !canAddMore.value) {
    if (!canAddMore.value) {
      emit("maxReached");
    }
    return;
  }
  showUrlInput.value = true;
  setTimeout(() => {
    urlInputRef.value?.focus();
  }, 0);
}

// 添加 URL 图片
async function addUrlImage() {
  if (!urlInput.value.trim()) {
    showUrlInput.value = false;
    return;
  }

  // 检查是否已达到最大数量
  if (images.value.length >= maxImages.value) {
    emit("maxReached");
    return;
  }

  const url = urlInput.value.trim();

  // 验证 URL 格式
  if (!url.match(/^https?:\/\/.+/i)) {
    alert("请输入有效的图片 URL（以 http:// 或 https:// 开头）");
    return;
  }

  try {
    // 尝试加载图片以验证
    const img = new Image();
    img.crossOrigin = "anonymous";

    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve();
      img.onerror = () => reject(new Error("图片加载失败"));
      img.src = url;
    });

    const newImage: ReferenceImage = {
      id: generateId(),
      type: "url",
      source: url,
      preview: url,
    };

    images.value = [...images.value, newImage];
    urlInput.value = "";
    showUrlInput.value = false;
  } catch (e) {
    alert("无法加载该图片，请检查 URL 是否正确");
  }
}

// 删除图片
function removeImage(id: string) {
  images.value = images.value.filter((img) => img.id !== id);
}

// 处理拖拽进入
function handleDragEnter(e: DragEvent) {
  e.preventDefault();
  e.stopPropagation();
  if (!props.disabled) {
    isDragging.value = true;
  }
}

// 处理拖拽离开
function handleDragLeave(e: DragEvent) {
  e.preventDefault();
  e.stopPropagation();
  isDragging.value = false;
}

// 处理拖拽悬停
function handleDragOver(e: DragEvent) {
  e.preventDefault();
  e.stopPropagation();
}

// 处理拖放
async function handleDrop(e: DragEvent) {
  e.preventDefault();
  e.stopPropagation();
  isDragging.value = false;

  if (props.disabled || !canAddMore.value) return;

  const files = e.dataTransfer?.files;
  if (!files || files.length === 0) return;

  for (const file of Array.from(files)) {
    // 检查是否已达到最大数量
    if (images.value.length >= maxImages.value) {
      emit("maxReached");
      break;
    }

    // 只处理图片文件
    if (!file.type.startsWith("image/")) continue;

    try {
      const arrayBuffer = await file.arrayBuffer();
      const uint8Array = new Uint8Array(arrayBuffer);
      const base64 = arrayBufferToBase64(uint8Array);
      const preview = `data:${file.type};base64,${base64}`;

      const newImage: ReferenceImage = {
        id: generateId(),
        type: "file",
        source: file.name,
        preview,
      };

      images.value = [...images.value, newImage];
    } catch (err) {
      console.error("处理拖放图片失败:", err);
    }
  }
}

// 取消 URL 输入
function cancelUrlInput() {
  urlInput.value = "";
  showUrlInput.value = false;
}

// 处理 URL 输入框按键
function handleUrlKeydown(e: KeyboardEvent) {
  if (e.key === "Enter") {
    addUrlImage();
  } else if (e.key === "Escape") {
    cancelUrlInput();
  }
}
</script>

<template>
  <div
    class="border rounded-lg p-4 transition-colors"
    :class="{
      'border-primary bg-primary/5': isDragging,
      'border-border': !isDragging,
      'opacity-50': disabled,
    }"
    @dragenter="handleDragEnter"
    @dragleave="handleDragLeave"
    @dragover="handleDragOver"
    @drop="handleDrop"
  >
    <!-- 已添加的图片缩略图 -->
    <div v-if="images.length > 0" class="flex flex-wrap gap-3 mb-4">
      <div
        v-for="img in images"
        :key="img.id"
        class="relative group w-20 h-20 rounded-lg overflow-hidden border"
      >
        <img
          :src="img.preview"
          class="w-full h-full object-cover"
          alt="参考图片"
        />
        <button
          @click="removeImage(img.id)"
          class="absolute top-0.5 right-0.5 w-5 h-5 rounded-full bg-black/60 hover:bg-black/80 text-white flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity"
          :disabled="disabled"
        >
          <XIcon class="w-3 h-3" />
        </button>
        <div
          v-if="img.type === 'url'"
          class="absolute bottom-0 left-0 right-0 bg-black/50 text-white text-[10px] text-center py-0.5"
        >
          URL
        </div>
      </div>
    </div>

    <!-- URL 输入框 -->
    <div v-if="showUrlInput" class="flex gap-2 mb-3">
      <input
        ref="urlInputRef"
        v-model="urlInput"
        type="text"
        placeholder="输入图片 URL..."
        class="flex-1 px-3 py-2 text-sm border rounded-lg focus:outline-none focus:ring-2 focus:ring-primary"
        @keydown="handleUrlKeydown"
      />
      <button
        @click="addUrlImage"
        class="px-3 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 text-sm"
      >
        添加
      </button>
      <button
        @click="cancelUrlInput"
        class="px-3 py-2 border rounded-lg hover:bg-muted text-sm"
      >
        取消
      </button>
    </div>

    <!-- 添加按钮区域 -->
    <div class="flex items-center gap-3 flex-wrap">
      <button
        @click="addLocalImage"
        :disabled="disabled || !canAddMore"
        class="flex items-center gap-2 px-3 py-2 border rounded-lg hover:bg-muted disabled:opacity-50 transition-colors"
        title="添加本地图片"
      >
        <PlusIcon class="w-4 h-4" />
        <span class="text-sm">添加图片</span>
      </button>

      <button
        @click="showUrlInputBox"
        :disabled="disabled || showUrlInput || !canAddMore"
        class="flex items-center gap-2 px-3 py-2 border rounded-lg hover:bg-muted disabled:opacity-50 transition-colors"
        title="添加图片链接"
      >
        <LinkIcon class="w-4 h-4" />
        <span class="text-sm">链接</span>
      </button>

      <span class="text-xs text-muted-foreground ml-2">
        <ImageIcon class="w-3 h-3 inline mr-1" />
        支持拖拽图片到此处
      </span>

      <!-- 最大数量提示 -->
      <span v-if="maxImages !== Infinity" class="text-xs text-muted-foreground">
        {{ images.length }}/{{ maxImages }}
      </span>
    </div>

    <!-- 拖拽提示 -->
    <div
      v-if="isDragging"
      class="mt-3 text-center text-sm text-primary font-medium"
    >
      松开鼠标添加图片
    </div>
  </div>
</template>
