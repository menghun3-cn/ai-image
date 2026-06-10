import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { getImages, refreshImages, deleteImage, getDirectoryMtime } from "@/lib/tauri";
import type { ImageInfo } from "@/lib/tauri";

// 图片项（包含前端状态）
export interface GalleryImageItem extends ImageInfo {
  url?: string;
  loaded?: boolean;
  loading?: boolean;
}

// 目录缓存
interface DirectoryCache {
  images: GalleryImageItem[];
  displayedCount: number;
  cachedAt: number;
}

// 初始加载数量
const INITIAL_LOAD_COUNT = 24;

export const useGalleryStore = defineStore("gallery", () => {
  // 按目录路径缓存数据
  const caches = ref<Map<string, DirectoryCache>>(new Map());
  
  // 当前目录
  const currentDir = ref("images");
  
  // 加载状态
  const isLoading = ref(false);
  const isLoadingMore = ref(false);
  
  // 当前显示的图片（从缓存中获取）
  const allImages = computed<GalleryImageItem[]>(() => {
    const cache = caches.value.get(currentDir.value);
    return cache?.images || [];
  });
  
  // 已加载显示的图片数量
  const displayedCount = computed(() => {
    const cache = caches.value.get(currentDir.value);
    return cache?.displayedCount || 0;
  });
  
  // 是否还有更多图片
  const hasMoreImages = computed(() => {
    return displayedCount.value < allImages.value.length;
  });
  
  // 当前显示的图片列表
  const displayedImages = computed(() => {
    const cache = caches.value.get(currentDir.value);
    if (!cache) return [];
    return cache.images.slice(0, cache.displayedCount);
  });
  
  // 设置当前目录
  function setCurrentDir(dir: string) {
    currentDir.value = dir;
  }
  
  // 加载图片（带缓存）
  // 设计原则：切换页面时检测目录 mtime，无变化直接使用缓存保持浏览位置
  // 有变化时自动刷新，不影响用户体验
  async function loadImages(dir: string, forceRefresh = false): Promise<boolean> {
    // 检查前端是否有缓存
    const existingCache = caches.value.get(dir);
    
    // 如果没有缓存，必须加载数据
    if (!existingCache) {
      return fetchAndUpdateImages(dir, false);
    }
    
    // 强制刷新时，直接获取最新数据
    if (forceRefresh) {
      return fetchAndUpdateImages(dir, true);
    }
    
    // 有前端缓存，检测目录 mtime 是否有变化
    try {
      const [mtime, fileCount] = await getDirectoryMtime(dir);
      
      // 如果 mtime 为 0，说明目录不存在或无法访问
      if (mtime === 0) {
        console.log("[GalleryStore] 目录无法访问，使用现有缓存:", dir);
        currentDir.value = dir;
        return true;
      }
      
      // 比较 mtime 和文件数量
      const cacheMtime = existingCache.cachedAt / 1000; // 转换为秒
      const cacheFileCount = existingCache.images.length;
      
      // 如果 mtime 相同且文件数量相同，认为缓存有效
      if (mtime <= cacheMtime && fileCount === cacheFileCount) {
        console.log("[GalleryStore] 目录未变化，使用缓存:", dir);
        currentDir.value = dir;
        return true; // 使用缓存
      }
      
      // 目录有变化，需要刷新
      console.log("[GalleryStore] 目录有变化，刷新数据:", dir, 
        "mtime:", mtime, "cacheMtime:", cacheMtime,
        "fileCount:", fileCount, "cacheCount:", cacheFileCount);
      return fetchAndUpdateImages(dir, false);
    } catch (error) {
      console.error("[GalleryStore] 检测目录变化失败:", error);
      // 检测失败，使用现有缓存
      currentDir.value = dir;
      return true;
    }
  }
  
  // 获取并更新图片数据
  async function fetchAndUpdateImages(dir: string, forceRefresh: boolean): Promise<boolean> {
    isLoading.value = true;
    
    try {
      console.log("[GalleryStore] 获取图片数据:", dir, "forceRefresh:", forceRefresh);
      
      const images = forceRefresh 
        ? await refreshImages(dir) 
        : await getImages(dir);
      
      const existingCache = caches.value.get(dir);
      
      // 转换为 GalleryImageItem，保留已有的 blob URL
      const galleryImages: GalleryImageItem[] = images.map(img => {
        const existing = existingCache?.images.find(e => e.path === img.path);
        return {
          ...img,
          url: existing?.url, // 保留已有的 URL
          loaded: existing?.loaded ?? false,
          loading: existing?.loading ?? false,
        };
      });
      
      // 更新缓存，但保持 displayedCount（不重置浏览位置）
      const newCount = existingCache 
        ? Math.min(existingCache.displayedCount, galleryImages.length)
        : Math.min(INITIAL_LOAD_COUNT, galleryImages.length);
      
      caches.value.set(dir, {
        images: galleryImages,
        displayedCount: newCount,
        cachedAt: Date.now(),
      });
      
      currentDir.value = dir;
      return false; // 重新加载了数据
    } catch (error) {
      console.error("[GalleryStore] 获取图片失败:", error);
      throw error;
    } finally {
      isLoading.value = false;
    }
  }
  
  // 加载更多图片（增加 displayedCount）
  function loadMore(count: number) {
    const cache = caches.value.get(currentDir.value);
    if (!cache) return;
    
    const newCount = Math.min(cache.displayedCount + count, cache.images.length);
    cache.displayedCount = newCount;
    console.log("[GalleryStore] 加载更多:", newCount, "/", cache.images.length);
  }
  
  // 更新图片 URL（懒加载后）
  function setImageUrl(path: string, url: string) {
    const cache = caches.value.get(currentDir.value);
    if (!cache) return;
    
    const image = cache.images.find(img => img.path === path);
    if (image) {
      image.url = url;
      image.loaded = true;
      image.loading = false;
    }
  }
  
  // 设置图片加载中状态
  function setImageLoading(path: string, loading: boolean) {
    const cache = caches.value.get(currentDir.value);
    if (!cache) return;
    
    const image = cache.images.find(img => img.path === path);
    if (image) {
      image.loading = loading;
    }
  }
  
  // 删除图片
  async function removeImage(path: string): Promise<void> {
    await deleteImage(path);
    
    // 从缓存中移除
    const cache = caches.value.get(currentDir.value);
    if (cache) {
      const index = cache.images.findIndex(img => img.path === path);
      if (index !== -1) {
        cache.images.splice(index, 1);
        // 调整 displayedCount
        if (cache.displayedCount > cache.images.length) {
          cache.displayedCount = cache.images.length;
        }
      }
    }
  }
  
  // 清除指定目录缓存
  function clearCache(dir?: string) {
    if (dir) {
      caches.value.delete(dir);
      console.log("[GalleryStore] 清除缓存:", dir);
    } else {
      caches.value.clear();
      console.log("[GalleryStore] 清除所有缓存");
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
    allImages,
    displayedImages,
    displayedCount,
    hasMoreImages,
    
    // Actions
    setCurrentDir,
    loadImages,
    loadMore,
    setImageUrl,
    setImageLoading,
    removeImage,
    clearCache,
    clearCurrentCache,
  };
});
