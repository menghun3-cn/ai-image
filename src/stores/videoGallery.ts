import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { getVideos, refreshVideos, deleteVideo, getDirectoryMtime } from "@/lib/tauri";
import { convertFileSrc } from "@tauri-apps/api/core";
import type { VideoInfo } from "@/lib/tauri";

// 视频项（包含前端状态）
export interface GalleryVideoItem extends VideoInfo {
  url?: string;
  blobUrl?: string;
  loaded?: boolean;
  loading?: boolean;
}

// 目录缓存
interface DirectoryCache {
  videos: GalleryVideoItem[];
  displayedCount: number;
  cachedAt: number;
}

// 初始加载数量
const INITIAL_LOAD_COUNT = 12;

export const useVideoGalleryStore = defineStore("videoGallery", () => {
  // 按目录路径缓存数据
  const caches = ref<Map<string, DirectoryCache>>(new Map());
  
  // 当前目录
  const currentDir = ref("video");
  
  // 加载状态
  const isLoading = ref(false);
  const isLoadingMore = ref(false);
  
  // 当前显示的视频（从缓存中获取）
  const allVideos = computed<GalleryVideoItem[]>(() => {
    const cache = caches.value.get(currentDir.value);
    return cache?.videos || [];
  });
  
  // 已加载显示的视频数量
  const displayedCount = computed(() => {
    const cache = caches.value.get(currentDir.value);
    return cache?.displayedCount || 0;
  });
  
  // 是否还有更多视频
  const hasMoreVideos = computed(() => {
    return displayedCount.value < allVideos.value.length;
  });
  
  // 当前显示的视频列表
  const displayedVideos = computed(() => {
    const cache = caches.value.get(currentDir.value);
    if (!cache) return [];
    return cache.videos.slice(0, cache.displayedCount);
  });
  
  // 设置当前目录
  function setCurrentDir(dir: string) {
    currentDir.value = dir;
  }
  
  // 加载视频（带缓存）
  // 设计原则：切换页面时检测目录 mtime，无变化直接使用缓存保持浏览位置
  // 有变化时自动刷新，不影响用户体验
  async function loadVideos(dir: string, forceRefresh = false): Promise<boolean> {
    // 检查前端是否有缓存
    const existingCache = caches.value.get(dir);
    
    // 如果没有缓存，必须加载数据
    if (!existingCache) {
      return fetchAndUpdateVideos(dir, false);
    }
    
    // 强制刷新时，直接获取最新数据
    if (forceRefresh) {
      return fetchAndUpdateVideos(dir, true);
    }
    
    // 有前端缓存，检测目录 mtime 是否有变化
    try {
      const [mtime, fileCount] = await getDirectoryMtime(dir);
      
      // 如果 mtime 为 0，说明目录不存在或无法访问
      if (mtime === 0) {
        console.log("[VideoGalleryStore] 目录无法访问，使用现有缓存:", dir);
        currentDir.value = dir;
        return true;
      }
      
      // 比较 mtime 和文件数量
      const cacheMtime = existingCache.cachedAt / 1000; // 转换为秒
      const cacheFileCount = existingCache.videos.length;
      
      // 如果 mtime 相同且文件数量相同，认为缓存有效
      if (mtime <= cacheMtime && fileCount === cacheFileCount) {
        console.log("[VideoGalleryStore] 目录未变化，使用缓存:", dir);
        currentDir.value = dir;
        return true; // 使用缓存
      }
      
      // 目录有变化，需要刷新
      console.log("[VideoGalleryStore] 目录有变化，刷新数据:", dir, 
        "mtime:", mtime, "cacheMtime:", cacheMtime,
        "fileCount:", fileCount, "cacheCount:", cacheFileCount);
      return fetchAndUpdateVideos(dir, false);
    } catch (error) {
      console.error("[VideoGalleryStore] 检测目录变化失败:", error);
      // 检测失败，使用现有缓存
      currentDir.value = dir;
      return true;
    }
  }
  
  // 获取并更新视频数据
  async function fetchAndUpdateVideos(dir: string, forceRefresh: boolean): Promise<boolean> {
    isLoading.value = true;
    
    try {
      console.log("[VideoGalleryStore] 获取视频数据:", dir, "forceRefresh:", forceRefresh);
      
      const videos = forceRefresh 
        ? await refreshVideos(dir) 
        : await getVideos(dir);
      
      const existingCache = caches.value.get(dir);
      
      // 转换为 GalleryVideoItem，保留已有的 blob URL
      const galleryVideos: GalleryVideoItem[] = videos.map(video => {
        const existing = existingCache?.videos.find(e => e.path === video.path);
        return {
          ...video,
          url: convertFileSrc(video.path),
          blobUrl: existing?.blobUrl, // 保留已有的 blob URL
          loaded: existing?.loaded ?? false,
          loading: existing?.loading ?? false,
        };
      });
      
      // 更新缓存，但保持 displayedCount（不重置浏览位置）
      const newCount = existingCache 
        ? Math.min(existingCache.displayedCount, galleryVideos.length)
        : Math.min(INITIAL_LOAD_COUNT, galleryVideos.length);
      
      caches.value.set(dir, {
        videos: galleryVideos,
        displayedCount: newCount,
        cachedAt: Date.now(),
      });
      
      currentDir.value = dir;
      return false; // 重新加载了数据
    } catch (error) {
      console.error("[VideoGalleryStore] 获取视频失败:", error);
      throw error;
    } finally {
      isLoading.value = false;
    }
  }
  
  // 加载更多视频（增加 displayedCount）
  function loadMore(count: number) {
    const cache = caches.value.get(currentDir.value);
    if (!cache) return;
    
    const newCount = Math.min(cache.displayedCount + count, cache.videos.length);
    cache.displayedCount = newCount;
    console.log("[VideoGalleryStore] 加载更多:", newCount, "/", cache.videos.length);
  }
  
  // 更新视频 blob URL（懒加载后）
  function setVideoBlobUrl(path: string, blobUrl: string) {
    const cache = caches.value.get(currentDir.value);
    if (!cache) return;
    
    const video = cache.videos.find(v => v.path === path);
    if (video) {
      video.blobUrl = blobUrl;
      video.loaded = true;
      video.loading = false;
    }
  }
  
  // 设置视频加载中状态
  function setVideoLoading(path: string, loading: boolean) {
    const cache = caches.value.get(currentDir.value);
    if (!cache) return;
    
    const video = cache.videos.find(v => v.path === path);
    if (video) {
      video.loading = loading;
    }
  }
  
  // 删除视频
  async function removeVideo(path: string): Promise<void> {
    await deleteVideo(path);
    
    // 从缓存中移除
    const cache = caches.value.get(currentDir.value);
    if (cache) {
      const index = cache.videos.findIndex(v => v.path === path);
      if (index !== -1) {
        // 释放 blob URL
        const video = cache.videos[index];
        if (video.blobUrl) {
          URL.revokeObjectURL(video.blobUrl);
        }
        cache.videos.splice(index, 1);
        // 调整 displayedCount
        if (cache.displayedCount > cache.videos.length) {
          cache.displayedCount = cache.videos.length;
        }
      }
    }
  }
  
  // 清除指定目录缓存
  function clearCache(dir?: string) {
    if (dir) {
      const cache = caches.value.get(dir);
      if (cache) {
        // 释放所有 blob URL
        cache.videos.forEach(video => {
          if (video.blobUrl) {
            URL.revokeObjectURL(video.blobUrl);
          }
        });
      }
      caches.value.delete(dir);
      console.log("[VideoGalleryStore] 清除缓存:", dir);
    } else {
      // 释放所有 blob URL
      caches.value.forEach(cache => {
        cache.videos.forEach(video => {
          if (video.blobUrl) {
            URL.revokeObjectURL(video.blobUrl);
          }
        });
      });
      caches.value.clear();
      console.log("[VideoGalleryStore] 清除所有缓存");
    }
  }
  
  // 清除当前目录缓存
  function clearCurrentCache() {
    clearCache(currentDir.value);
  }
  
  return {
    // State
    currentDir,
    isLoading,
    isLoadingMore,
    caches,
    
    // Getters
    allVideos,
    displayedVideos,
    displayedCount,
    hasMoreVideos,
    
    // Actions
    setCurrentDir,
    loadVideos,
    loadMore,
    setVideoBlobUrl,
    setVideoLoading,
    removeVideo,
    clearCache,
    clearCurrentCache,
  };
});
