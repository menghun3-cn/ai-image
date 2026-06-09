<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { 
  Loader2Icon, 
  DownloadIcon, 
  CheckCircleIcon, 
  AlertCircleIcon,
  RefreshCwIcon,
  GithubIcon,
  InfoIcon
} from "lucide-vue-next";

interface UpdateInfo {
  version: string;
  current_version: string;
  notes: string | null;
  pub_date: string | null;
  has_update: boolean;
}

const currentVersion = ref("");
const updateInfo = ref<UpdateInfo | null>(null);
const isChecking = ref(false);
const isDownloading = ref(false);
const error = ref("");
const successMessage = ref("");

// 获取当前版本
async function loadCurrentVersion() {
  try {
    currentVersion.value = await invoke("get_app_version");
  } catch (e) {
    console.error("获取版本失败:", e);
    currentVersion.value = "2.0.1";
  }
}

// 检查更新
async function checkUpdate() {
  isChecking.value = true;
  error.value = "";
  successMessage.value = "";
  updateInfo.value = null;

  try {
    const result = await invoke<UpdateInfo>("check_update");
    updateInfo.value = result;
    
    if (result.has_update) {
      successMessage.value = `发现新版本: ${result.version}`;
    } else {
      successMessage.value = "当前已是最新版本";
    }
  } catch (e) {
    error.value = `检查更新失败: ${e}`;
    console.error("检查更新失败:", e);
  } finally {
    isChecking.value = false;
  }
}

// 下载并安装更新
async function downloadAndInstall() {
  if (!updateInfo.value?.has_update) return;

  isDownloading.value = true;
  error.value = "";
  successMessage.value = "正在下载更新，请稍候...";

  try {
    await invoke("download_and_install_update");
    successMessage.value = "更新下载完成，应用将自动重启...";
  } catch (e) {
    error.value = `更新失败: ${e}`;
    console.error("更新失败:", e);
    isDownloading.value = false;
  }
}

// 打开 GitHub 仓库
async function openGithub() {
  try {
    await open("https://github.com/menghun3-cn/ai-image/");
  } catch (e) {
    console.error("打开链接失败:", e);
  }
}

onMounted(() => {
  loadCurrentVersion();
  // 自动检查更新
  checkUpdate();
});
</script>

<template>
  <div class="p-6 max-w-2xl mx-auto h-full flex flex-col">
    <div class="flex items-center gap-3 mb-6">
      <div class="w-16 h-16 bg-primary rounded-xl flex items-center justify-center text-primary-foreground text-2xl font-bold">
        AI
      </div>
      <div>
        <h1 class="text-2xl font-bold">ai-image</h1>
        <p class="text-muted-foreground">AI 图片生成工具</p>
      </div>
    </div>

    <!-- 版本信息卡片 -->
    <Card class="mb-6 flex-shrink-0">
      <CardHeader>
        <CardTitle class="flex items-center gap-2">
          <InfoIcon class="w-5 h-5" />
          版本信息
        </CardTitle>
        <CardDescription>当前应用版本和更新状态</CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        <div class="flex items-center justify-between">
          <span class="text-muted-foreground">当前版本</span>
          <Badge variant="secondary" class="text-lg px-3 py-1">
            v{{ currentVersion }}
          </Badge>
        </div>

        <Separator />

        <!-- 更新状态 -->
        <div v-if="updateInfo" class="space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-muted-foreground">最新版本</span>
            <div class="flex items-center gap-2">
              <span class="text-base font-medium">v{{ updateInfo.version }}</span>
              <Badge 
                v-if="updateInfo.has_update" 
                variant="default"
                class="text-xs px-2 py-0.5"
              >
                可升级
              </Badge>
              <Badge 
                v-else 
                variant="secondary"
                class="text-xs px-2 py-0.5"
              >
                已是最新
              </Badge>
            </div>
          </div>

          <!-- 更新说明 -->
          <div v-if="updateInfo.notes" class="bg-muted p-3 rounded-lg">
            <p class="text-sm font-medium mb-2">更新说明:</p>
            <div class="text-sm text-muted-foreground whitespace-pre-wrap">
              {{ updateInfo.notes }}
            </div>
          </div>

          <!-- 发布日期 -->
          <div v-if="updateInfo.pub_date" class="text-sm text-muted-foreground">
            发布日期: {{ new Date(updateInfo.pub_date).toLocaleString('zh-CN') }}
          </div>
        </div>

        <!-- 操作按钮 -->
        <div class="flex gap-3 pt-2">
          <Button 
            variant="outline" 
            @click="checkUpdate"
            :disabled="isChecking"
          >
            <RefreshCwIcon 
              class="w-4 h-4 mr-2" 
              :class="{ 'animate-spin': isChecking }"
            />
            {{ isChecking ? '检查中...' : '检查更新' }}
          </Button>

          <Button 
            v-if="updateInfo?.has_update"
            @click="downloadAndInstall"
            :disabled="isDownloading"
          >
            <Loader2Icon 
              v-if="isDownloading" 
              class="w-4 h-4 mr-2 animate-spin" 
            />
            <DownloadIcon 
              v-else 
              class="w-4 h-4 mr-2" 
            />
            {{ isDownloading ? '下载中...' : '立即升级' }}
          </Button>
        </div>

        <!-- 状态消息 -->
        <div v-if="error" class="flex items-center gap-2 text-destructive text-sm">
          <AlertCircleIcon class="w-4 h-4" />
          {{ error }}
        </div>

        <div v-if="successMessage && !error" class="flex items-center gap-2 text-green-600 text-sm">
          <CheckCircleIcon class="w-4 h-4" />
          {{ successMessage }}
        </div>
      </CardContent>
    </Card>

    <!-- 项目信息卡片 -->
    <Card class="flex-shrink-0">
      <CardHeader>
        <CardTitle class="flex items-center gap-2">
          <GithubIcon class="w-5 h-5" />
          开源项目
        </CardTitle>
        <CardDescription>本项目基于 Tauri + Vue 开发</CardDescription>
      </CardHeader>
      <CardContent class="space-y-4">
        <div class="grid grid-cols-2 gap-4 text-sm">
          <div>
            <span class="text-muted-foreground">作者</span>
            <p class="font-medium">menghun3-cn</p>
          </div>
          <div>
            <span class="text-muted-foreground">许可证</span>
            <p class="font-medium">MIT</p>
          </div>
          <div>
            <span class="text-muted-foreground">技术栈</span>
            <p class="font-medium">Tauri v2 + Vue 3</p>
          </div>
          <div>
            <span class="text-muted-foreground">开发语言</span>
            <p class="font-medium">Rust + TypeScript</p>
          </div>
        </div>

        <Separator />

        <Button variant="outline" class="w-full" @click="openGithub">
          <GithubIcon class="w-4 h-4 mr-2" />
          访问 GitHub 仓库
        </Button>
      </CardContent>
    </Card>

    <!-- 版权信息 -->
    <p class="text-center text-sm text-muted-foreground mt-auto pt-6">
      © 2026 ai-image. All rights reserved.
    </p>
  </div>
</template>
