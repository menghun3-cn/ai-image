<script setup lang="ts">
import { ref, onMounted, watch, nextTick } from "vue";
import { loadConfig, saveConfig } from "@/lib/tauri";
import type { AppConfig } from "@/lib/tauri";
import Dialog from "@/components/Dialog.vue";
import { InfoIcon, SlidersIcon, KeyIcon, GlobeIcon, FolderIcon, EyeIcon, EyeOffIcon, ExternalLinkIcon } from "lucide-vue-next";

// 各平台获取 API Key 的链接
const providerLinks: Record<string, string> = {
  agnes: "https://platform.agnes-ai.com/",
  openrouter: "https://openrouter.ai/settings/keys",
  modelscope: "https://modelscope.cn/my/myaccesstoken",
  nvidia: "https://build.nvidia.com/explore/discover",
  gemini: "https://aistudio.google.com/apikey",
  openai: "https://platform.openai.com/api-keys",
  siliconflow: "https://cloud.siliconflow.cn/me/account/ak",
};

const config = ref<AppConfig | null>(null);
const activeTab = ref(localStorage.getItem("lastSettingsTab") || "api");
const saveError = ref<string | null>(null);

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

// 监听标签变化，持久化到 localStorage
watch(activeTab, (newTab) => {
  localStorage.setItem("lastSettingsTab", newTab);
});

// 监听配置变化，自动保存
let isLoading = true;
watch(
  config,
  async (newConfig, oldConfig) => {
    // 跳过初始加载时的保存
    if (isLoading || !oldConfig || !newConfig) {
      isLoading = false;
      return;
    }
    
    // 延迟保存，避免频繁写入
    await nextTick();
    try {
      await saveConfig(newConfig);
      saveError.value = null;
    } catch (e) {
      console.error("自动保存失败:", e);
      saveError.value = String(e);
    }
  },
  { deep: true }
);

// 控制每个 API Key 的显示/隐藏状态
const showKeys = ref({
  agnes: false,
  openrouter: false,
  modelscope: false,
  nvidia: false,
  gemini: false,
  openai: false,
  siliconflow: false,
});

function toggleKeyVisibility(key: keyof typeof showKeys.value) {
  showKeys.value[key] = !showKeys.value[key];
}

const appVersion = "2.0.0";
const tauriVersion = "2.10.3";

onMounted(async () => {
  try {
    config.value = await loadConfig();
    // 标记加载完成
    isLoading = false;
  } catch (e) {
    console.error("Failed to load config:", e);
    await showDialog({
      title: "错误",
      message: "加载配置失败: " + String(e),
      type: "error",
    });
  }
});

async function resetToDefaults() {
  if (!config.value) return;
  const confirmed = await showDialog({
    title: "确认",
    message: "确定要重置所有设置为默认值吗？",
    type: "warning",
    showCancel: true,
  });
  if (confirmed) {
    config.value.default_steps = 30;
    config.value.default_guidance_scale = 7.5;
    config.value.default_seed = -1;
    config.value.theme = "system";
    // 自动保存会触发
  }
}

const tabs = [
  { id: "api", label: "API 配置", icon: KeyIcon },
  { id: "model", label: "模型参数", icon: SlidersIcon },
  { id: "proxy", label: "代理设置", icon: GlobeIcon },
  { id: "general", label: "常规设置", icon: FolderIcon },
  { id: "about", label: "关于", icon: InfoIcon },
];
</script>

<template>
  <div class="h-full flex">
    <!-- Sidebar Tabs -->
    <aside class="w-48 border-r bg-muted/30 flex flex-col">
      <div class="p-4 border-b">
        <h1 class="text-lg font-bold">设置</h1>
      </div>
      <nav class="flex-1 p-2 space-y-1">
        <button
          v-for="tab in tabs"
          :key="tab.id"
          @click="activeTab = tab.id"
          :class="[
            'w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors text-left',
            activeTab === tab.id
              ? 'bg-primary text-primary-foreground'
              : 'text-muted-foreground hover:bg-muted hover:text-foreground',
          ]"
        >
          <component :is="tab.icon" class="w-4 h-4" />
          {{ tab.label }}
        </button>
      </nav>
      

    </aside>

    <!-- Main Content -->
    <main class="flex-1 overflow-auto p-6">
      <div v-if="!config" class="flex items-center justify-center h-full">
        <div class="text-center">
          <div class="w-8 h-8 border-2 border-primary border-t-transparent rounded-full animate-spin mx-auto mb-4"></div>
          <p class="text-muted-foreground">加载配置中...</p>
        </div>
      </div>

      <div v-else class="max-w-2xl mx-auto space-y-6">
        <!-- API 配置 -->
        <div v-if="activeTab === 'api'" class="space-y-6">
          <div class="flex items-center gap-3 pb-4 border-b">
            <KeyIcon class="w-5 h-5 text-primary" />
            <div>
              <h2 class="text-lg font-semibold">API 配置</h2>
              <p class="text-sm text-muted-foreground">配置各平台的 API Key</p>
            </div>
          </div>

          <div class="grid gap-5">
            <div class="p-4 rounded-lg border bg-card">
              <div class="flex items-center justify-between mb-2">
                <label class="flex items-center gap-2 text-sm font-medium">
                  <span class="w-2 h-2 rounded-full bg-pink-500"></span>
                  Agnes AI API Key
                </label>
                <a
                  :href="providerLinks.agnes"
                  target="_blank"
                  rel="noopener noreferrer"
                  class="flex items-center gap-1 text-xs text-muted-foreground hover:text-primary transition-colors"
                >
                  <ExternalLinkIcon class="w-3 h-3" />
                  获取 Key
                </a>
              </div>
              <div class="relative">
                <input
                  v-model="config.providers.agnes.api_key"
                  :type="showKeys.agnes ? 'text' : 'password'"
                  class="w-full px-3 py-2 pr-10 border rounded-md focus:outline-none focus:ring-2 focus:ring-primary/50 text-sm"
                  placeholder="agnes-..."
                />
                <button
                  type="button"
                  @click="toggleKeyVisibility('agnes')"
                  class="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground transition-colors"
                >
                  <EyeIcon v-if="showKeys.agnes" class="w-4 h-4" />
                  <EyeOffIcon v-else class="w-4 h-4" />
                </button>
              </div>
              <p class="text-xs text-muted-foreground mt-1.5">免费使用，推荐首选</p>
            </div>

            <div class="p-4 rounded-lg border bg-card">
              <div class="flex items-center justify-between mb-2">
                <label class="flex items-center gap-2 text-sm font-medium">
                  <span class="w-2 h-2 rounded-full bg-green-500"></span>
                  OpenRouter API Key
                </label>
                <a
                  :href="providerLinks.openrouter"
                  target="_blank"
                  rel="noopener noreferrer"
                  class="flex items-center gap-1 text-xs text-muted-foreground hover:text-primary transition-colors"
                >
                  <ExternalLinkIcon class="w-3 h-3" />
                  获取 Key
                </a>
              </div>
              <div class="relative">
                <input
                  v-model="config.providers.openrouter.api_key"
                  :type="showKeys.openrouter ? 'text' : 'password'"
                  class="w-full px-3 py-2 pr-10 border rounded-md focus:outline-none focus:ring-2 focus:ring-primary/50 text-sm"
                  placeholder="sk-or-..."
                />
                <button
                  type="button"
                  @click="toggleKeyVisibility('openrouter')"
                  class="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground transition-colors"
                >
                  <EyeIcon v-if="showKeys.openrouter" class="w-4 h-4" />
                  <EyeOffIcon v-else class="w-4 h-4" />
                </button>
              </div>
              <p class="text-xs text-muted-foreground mt-1.5">用于图片生成和提示词优化</p>
            </div>

            <div class="p-4 rounded-lg border bg-card">
              <div class="flex items-center justify-between mb-2">
                <label class="flex items-center gap-2 text-sm font-medium">
                  <span class="w-2 h-2 rounded-full bg-blue-500"></span>
                  ModelScope API Key
                </label>
                <a
                  :href="providerLinks.modelscope"
                  target="_blank"
                  rel="noopener noreferrer"
                  class="flex items-center gap-1 text-xs text-muted-foreground hover:text-primary transition-colors"
                >
                  <ExternalLinkIcon class="w-3 h-3" />
                  获取 Key
                </a>
              </div>
              <div class="relative">
                <input
                  v-model="config.providers.modelscope.api_key"
                  :type="showKeys.modelscope ? 'text' : 'password'"
                  class="w-full px-3 py-2 pr-10 border rounded-md focus:outline-none focus:ring-2 focus:ring-primary/50 text-sm"
                  placeholder="可选，用于阿里云 ModelScope"
                />
                <button
                  type="button"
                  @click="toggleKeyVisibility('modelscope')"
                  class="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground transition-colors"
                >
                  <EyeIcon v-if="showKeys.modelscope" class="w-4 h-4" />
                  <EyeOffIcon v-else class="w-4 h-4" />
                </button>
              </div>
            </div>

            <div class="p-4 rounded-lg border bg-card">
              <div class="flex items-center justify-between mb-2">
                <label class="flex items-center gap-2 text-sm font-medium">
                  <span class="w-2 h-2 rounded-full bg-purple-500"></span>
                  NVIDIA API Key
                </label>
                <a
                  :href="providerLinks.nvidia"
                  target="_blank"
                  rel="noopener noreferrer"
                  class="flex items-center gap-1 text-xs text-muted-foreground hover:text-primary transition-colors"
                >
                  <ExternalLinkIcon class="w-3 h-3" />
                  获取 Key
                </a>
              </div>
              <div class="relative">
                <input
                  v-model="config.providers.nvidia.api_key"
                  :type="showKeys.nvidia ? 'text' : 'password'"
                  class="w-full px-3 py-2 pr-10 border rounded-md focus:outline-none focus:ring-2 focus:ring-primary/50 text-sm"
                  placeholder="nvapi-..."
                />
                <button
                  type="button"
                  @click="toggleKeyVisibility('nvidia')"
                  class="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground transition-colors"
                >
                  <EyeIcon v-if="showKeys.nvidia" class="w-4 h-4" />
                  <EyeOffIcon v-else class="w-4 h-4" />
                </button>
              </div>
            </div>

            <div class="p-4 rounded-lg border bg-card">
              <div class="flex items-center justify-between mb-2">
                <label class="flex items-center gap-2 text-sm font-medium">
                  <span class="w-2 h-2 rounded-full bg-red-500"></span>
                  Gemini API Key
                </label>
                <a
                  :href="providerLinks.gemini"
                  target="_blank"
                  rel="noopener noreferrer"
                  class="flex items-center gap-1 text-xs text-muted-foreground hover:text-primary transition-colors"
                >
                  <ExternalLinkIcon class="w-3 h-3" />
                  获取 Key
                </a>
              </div>
              <div class="relative">
                <input
                  v-model="config.providers.gemini.api_key"
                  :type="showKeys.gemini ? 'text' : 'password'"
                  class="w-full px-3 py-2 pr-10 border rounded-md focus:outline-none focus:ring-2 focus:ring-primary/50 text-sm"
                  placeholder="AIza..."
                />
                <button
                  type="button"
                  @click="toggleKeyVisibility('gemini')"
                  class="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground transition-colors"
                >
                  <EyeIcon v-if="showKeys.gemini" class="w-4 h-4" />
                  <EyeOffIcon v-else class="w-4 h-4" />
                </button>
              </div>
            </div>

            <div class="p-4 rounded-lg border bg-card">
              <div class="flex items-center justify-between mb-2">
                <label class="flex items-center gap-2 text-sm font-medium">
                  <span class="w-2 h-2 rounded-full bg-gray-500"></span>
                  OpenAI API Key
                </label>
                <a
                  :href="providerLinks.openai"
                  target="_blank"
                  rel="noopener noreferrer"
                  class="flex items-center gap-1 text-xs text-muted-foreground hover:text-primary transition-colors"
                >
                  <ExternalLinkIcon class="w-3 h-3" />
                  获取 Key
                </a>
              </div>
              <div class="relative">
                <input
                  v-model="config.providers.openai.api_key"
                  :type="showKeys.openai ? 'text' : 'password'"
                  class="w-full px-3 py-2 pr-10 border rounded-md focus:outline-none focus:ring-2 focus:ring-primary/50 text-sm"
                  placeholder="sk-..."
                />
                <button
                  type="button"
                  @click="toggleKeyVisibility('openai')"
                  class="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground transition-colors"
                >
                  <EyeIcon v-if="showKeys.openai" class="w-4 h-4" />
                  <EyeOffIcon v-else class="w-4 h-4" />
                </button>
              </div>
            </div>

            <div class="p-4 rounded-lg border bg-card">
              <div class="flex items-center justify-between mb-2">
                <label class="flex items-center gap-2 text-sm font-medium">
                  <span class="w-2 h-2 rounded-full bg-orange-500"></span>
                  SiliconFlow API Key
                </label>
                <a
                  :href="providerLinks.siliconflow"
                  target="_blank"
                  rel="noopener noreferrer"
                  class="flex items-center gap-1 text-xs text-muted-foreground hover:text-primary transition-colors"
                >
                  <ExternalLinkIcon class="w-3 h-3" />
                  获取 Key
                </a>
              </div>
              <div class="relative">
                <input
                  v-model="config.providers.siliconflow.api_key"
                  :type="showKeys.siliconflow ? 'text' : 'password'"
                  class="w-full px-3 py-2 pr-10 border rounded-md focus:outline-none focus:ring-2 focus:ring-primary/50 text-sm"
                  placeholder="sf-..."
                />
                <button
                  type="button"
                  @click="toggleKeyVisibility('siliconflow')"
                  class="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground transition-colors"
                >
                  <EyeIcon v-if="showKeys.siliconflow" class="w-4 h-4" />
                  <EyeOffIcon v-else class="w-4 h-4" />
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- 模型参数 -->
        <div v-if="activeTab === 'model'" class="space-y-6">
          <div class="flex items-center gap-3 pb-4 border-b">
            <SlidersIcon class="w-5 h-5 text-primary" />
            <div>
              <h2 class="text-lg font-semibold">模型参数</h2>
              <p class="text-sm text-muted-foreground">配置默认生成参数</p>
            </div>
          </div>

          <div class="grid gap-5">
            <div class="p-4 rounded-lg border bg-card">
              <div class="flex items-center justify-between mb-3">
                <label class="text-sm font-medium">推理步数 (Steps)</label>
                <span class="text-xs px-2 py-1 rounded-full bg-muted font-mono">{{ config.default_steps || 30 }}</span>
              </div>
              <input
                v-model.number="config.default_steps"
                type="range"
                min="1"
                max="50"
                class="w-full h-2 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
              />
              <div class="flex justify-between text-xs text-muted-foreground mt-1.5">
                <span>1 (快)</span>
                <span>25 (推荐)</span>
                <span>50 (高质量)</span>
              </div>
            </div>

            <div class="p-4 rounded-lg border bg-card">
              <div class="flex items-center justify-between mb-3">
                <label class="text-sm font-medium">引导强度 (Guidance Scale)</label>
                <span class="text-xs px-2 py-1 rounded-full bg-muted font-mono">{{ config.default_guidance_scale || 7.5 }}</span>
              </div>
              <input
                v-model.number="config.default_guidance_scale"
                type="range"
                min="1"
                max="20"
                step="0.5"
                class="w-full h-2 bg-muted rounded-lg appearance-none cursor-pointer accent-primary"
              />
              <div class="flex justify-between text-xs text-muted-foreground mt-1.5">
                <span>1 (自由)</span>
                <span>7.5 (平衡)</span>
                <span>20 (严格)</span>
              </div>
            </div>

            <div class="p-4 rounded-lg border bg-card">
              <div class="flex items-center justify-between mb-3">
                <label class="text-sm font-medium">随机种子 (Seed)</label>
                <span class="text-xs px-2 py-1 rounded-full bg-muted font-mono">{{ config.default_seed === -1 || config.default_seed === undefined ? '随机' : config.default_seed }}</span>
              </div>
              <div class="flex items-center gap-3">
                <input
                  v-model.number="config.default_seed"
                  type="number"
                  :placeholder="config.default_seed === -1 || config.default_seed === undefined ? '随机种子' : '输入种子值'"
                  class="flex-1 px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-primary/50 text-sm"
                />
                <button
                  @click="config.default_seed = -1"
                  class="px-3 py-2 text-sm border rounded-md hover:bg-muted transition-colors"
                  :class="config.default_seed === -1 || config.default_seed === undefined ? 'bg-primary text-primary-foreground border-primary' : ''"
                >
                  随机
                </button>
              </div>
              <p class="text-xs text-muted-foreground mt-1.5">固定种子可复现相同结果，-1 表示随机</p>
            </div>

            <div class="flex justify-end">
              <button
                @click="resetToDefaults"
                class="px-4 py-2 text-sm text-muted-foreground hover:text-foreground border rounded-md hover:bg-muted transition-colors"
              >
                恢复默认参数
              </button>
            </div>
          </div>
        </div>

        <!-- 代理设置 -->
        <div v-if="activeTab === 'proxy'" class="space-y-6">
          <div class="flex items-center gap-3 pb-4 border-b">
            <GlobeIcon class="w-5 h-5 text-primary" />
            <div>
              <h2 class="text-lg font-semibold">代理设置</h2>
              <p class="text-sm text-muted-foreground">配置网络代理</p>
            </div>
          </div>

          <div class="p-4 rounded-lg border bg-card">
            <div class="flex items-center gap-3 mb-4">
              <input
                v-model="config.proxy_enabled"
                type="checkbox"
                id="proxy_enabled"
                class="w-4 h-4 rounded border-gray-300 text-primary focus:ring-primary"
              />
              <label for="proxy_enabled" class="text-sm font-medium">启用代理</label>
            </div>

            <div :class="{ 'opacity-50 pointer-events-none': !config.proxy_enabled }">
              <label class="block text-sm font-medium mb-2">代理地址</label>
              <input
                v-model="config.proxy"
                type="text"
                class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-primary/50 text-sm"
                placeholder="http://127.0.0.1:7890"
              />
              <p class="text-xs text-muted-foreground mt-1.5">支持 HTTP/HTTPS/SOCKS5 代理</p>
            </div>
          </div>
        </div>

        <!-- 常规设置 -->
        <div v-if="activeTab === 'general'" class="space-y-6">
          <div class="flex items-center gap-3 pb-4 border-b">
            <FolderIcon class="w-5 h-5 text-primary" />
            <div>
              <h2 class="text-lg font-semibold">常规设置</h2>
              <p class="text-sm text-muted-foreground">通用选项</p>
            </div>
          </div>

          <div class="grid gap-5">
            <div class="p-4 rounded-lg border bg-card">
              <label class="block text-sm font-medium mb-2">默认提供商</label>
              <select
                v-model="config.default_provider"
                class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-primary/50 text-sm bg-background"
              >
                <option value="agnes">Agnes AI (免费)</option>
                <option value="modelscope">ModelScope (阿里云)</option>
                <option value="nvidia">NVIDIA</option>
                <option value="gemini">Google Gemini</option>
                <option value="openrouter">OpenRouter</option>
                <option value="openai">OpenAI</option>
                <option value="siliconflow">硅基流动 (SiliconFlow)</option>
              </select>
            </div>

            <div class="p-4 rounded-lg border bg-card">
              <label class="block text-sm font-medium mb-2">图片默认输出目录</label>
              <input
                v-model="config.default_output_dir"
                type="text"
                class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-primary/50 text-sm"
                placeholder="images"
              />
            </div>

            <div class="p-4 rounded-lg border bg-card">
              <label class="block text-sm font-medium mb-2">视频默认输出目录</label>
              <input
                v-model="config.default_video_output_dir"
                type="text"
                class="w-full px-3 py-2 border rounded-md focus:outline-none focus:ring-2 focus:ring-primary/50 text-sm"
                placeholder="video"
              />
            </div>

            <div class="p-4 rounded-lg border bg-card">
              <label class="block text-sm font-medium mb-2">主题</label>
              <div class="grid grid-cols-3 gap-2">
                <button
                  @click="config.theme = 'light'"
                  :class="[
                    'px-3 py-2 text-sm border rounded-md transition-colors',
                    config.theme === 'light' ? 'bg-primary text-primary-foreground border-primary' : 'hover:bg-muted'
                  ]"
                >
                  ☀️ 浅色
                </button>
                <button
                  @click="config.theme = 'dark'"
                  :class="[
                    'px-3 py-2 text-sm border rounded-md transition-colors',
                    config.theme === 'dark' ? 'bg-primary text-primary-foreground border-primary' : 'hover:bg-muted'
                  ]"
                >
                  🌙 深色
                </button>
                <button
                  @click="config.theme = 'system'"
                  :class="[
                    'px-3 py-2 text-sm border rounded-md transition-colors',
                    config.theme === 'system' || !config.theme ? 'bg-primary text-primary-foreground border-primary' : 'hover:bg-muted'
                  ]"
                >
                  💻 跟随系统
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- 关于 -->
        <div v-if="activeTab === 'about'" class="space-y-6">
          <div class="flex items-center gap-3 pb-4 border-b">
            <InfoIcon class="w-5 h-5 text-primary" />
            <div>
              <h2 class="text-lg font-semibold">关于</h2>
              <p class="text-sm text-muted-foreground">应用信息</p>
            </div>
          </div>

          <div class="p-6 rounded-lg border bg-card text-center">
            <div class="w-16 h-16 mx-auto mb-4 rounded-xl bg-gradient-to-br from-primary to-primary/60 flex items-center justify-center text-primary-foreground text-2xl font-bold">
              AI
            </div>
            <h3 class="text-xl font-bold mb-1">AI Image V2</h3>
            <p class="text-sm text-muted-foreground mb-4">AI 图片生成桌面应用</p>
            
            <div class="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-muted text-xs">
              <span class="w-2 h-2 rounded-full bg-green-500"></span>
              版本 {{ appVersion }}
            </div>
          </div>

          <div class="grid gap-3">
            <div class="flex items-center justify-between p-3 rounded-lg border bg-card">
              <span class="text-sm text-muted-foreground">Tauri 版本</span>
              <span class="text-sm font-mono">{{ tauriVersion }}</span>
            </div>
            <div class="flex items-center justify-between p-3 rounded-lg border bg-card">
              <span class="text-sm text-muted-foreground">Vue 版本</span>
              <span class="text-sm font-mono">3.5.13</span>
            </div>
            <div class="flex items-center justify-between p-3 rounded-lg border bg-card">
              <span class="text-sm text-muted-foreground">Rust 版本</span>
              <span class="text-sm font-mono">1.85+</span>
            </div>
            <div class="flex items-center justify-between p-3 rounded-lg border bg-card">
              <span class="text-sm text-muted-foreground">支持平台</span>
              <span class="text-sm">Windows 10/11</span>
            </div>
          </div>

          <div class="p-4 rounded-lg border bg-muted/50">
            <h4 class="text-sm font-medium mb-2">功能特性</h4>
            <ul class="text-sm text-muted-foreground space-y-1">
              <li>• 支持 6 个 AI 图片生成提供商</li>
              <li>• 批量生成与单图生成</li>
              <li>• AI 提示词优化</li>
              <li>• 图库管理与预览</li>
              <li>• 8 种图片比例选择</li>
            </ul>
          </div>

          <div class="text-center text-xs text-muted-foreground">
            <p>© 2026 AI Image V2. All rights reserved.</p>
            <p class="mt-1">基于 Tauri + Vue 构建</p>
          </div>
        </div>
      </div>
    </main>
  </div>

  <!-- 对话框组件 -->
  <Dialog
    :show="dialog.show"
    :title="dialog.title"
    :message="dialog.message"
    :type="dialog.type"
    :show-cancel="dialog.showCancel"
    @confirm="handleDialogConfirm"
    @cancel="handleDialogCancel"
  />
</template>
