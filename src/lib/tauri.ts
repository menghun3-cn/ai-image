import { invoke } from "@tauri-apps/api/core";

export type GenerationStatus = "idle" | "generating" | "success" | "error";

// 参考图片类型
export interface ReferenceImage {
  id: string;
  type: "file" | "url";
  source: string;
  preview: string;
}

export interface GenerationOptions {
  prompt: string;
  provider: string;
  model?: string;
  output_dir: string;
  width: number;
  height: number;
  steps?: number;
  guidance_scale?: number;
  seed?: number;
  image?: string; // Base64 编码的参考图片
}

export interface GenerationResult {
  success: boolean;
  image_path?: string;
  error?: string;
  retries?: number;
}

export interface BatchGenerationOptions {
  prompts: string[];
  provider: string;
  model?: string;
  output_dir: string;
  width: number;
  height: number;
}

export interface SingleGenerationResult {
  index: number;
  prompt: string;
  success: boolean;
  image_path?: string;
  error?: string;
  duration_ms: number;
}

export interface BatchGenerationResult {
  total: number;
  success_count: number;
  failed_count: number;
  results: SingleGenerationResult[];
}

export interface OptimizeResult {
  success: boolean;
  optimized_prompt?: string;
  original_intent?: string;
  style?: string;
  negative_prompt?: string;
  tips?: string;
  error?: string;
}

export interface ImageInfo {
  path: string;
  name: string;
  time: number;
}

export interface ProviderConfig {
  api_key: string;
  endpoint: string;
}

export interface ProvidersConfig {
  modelscope: ProviderConfig;
  nvidia: ProviderConfig;
  gemini: ProviderConfig;
  openrouter: ProviderConfig;
  openai: ProviderConfig;
  siliconflow: ProviderConfig;
  agnes: ProviderConfig;
}

export interface ModelLists {
  modelscope: string[];
  nvidia: string[];
  gemini: string[];
  openrouter: string[];
  openai: string[];
  siliconflow: string[];
  agnes: string[];
}

export interface AppConfig {
  providers: ProvidersConfig;
  default_provider: string;
  default_output_dir: string;
  default_video_output_dir: string;
  default_width: number;
  default_height: number;
  proxy: string;
  proxy_enabled: boolean;
  theme: string;
  models: ModelLists;
  default_steps?: number;
  default_guidance_scale?: number;
  default_seed?: number;
}

export async function generateImage(options: GenerationOptions): Promise<GenerationResult> {
  return invoke<GenerationResult>("generate_image", { options });
}

export async function optimizePrompt(prompt: string): Promise<OptimizeResult> {
  return invoke<OptimizeResult>("optimize_prompt", { prompt });
}

export async function getImages(outputDir: string): Promise<ImageInfo[]> {
  return invoke<ImageInfo[]>("get_images", { outputDir });
}

export async function deleteImage(path: string): Promise<void> {
  return invoke("delete_image", { path });
}

export async function openOutputDir(path: string): Promise<void> {
  return invoke("open_output_dir", { path });
}

export async function loadConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("load_config");
}

export async function saveConfig(config: AppConfig): Promise<void> {
  return invoke("save_config", { config });
}

export async function getProviderModels(provider: string): Promise<string[]> {
  return invoke<string[]>("get_provider_models", { provider });
}

export async function batchGenerateImages(options: BatchGenerationOptions): Promise<BatchGenerationResult> {
  return invoke<BatchGenerationResult>("batch_generate_images", { options });
}

// 视频生成相关
export type VideoGenerationMode = "text" | "single" | "multi" | "keyframes";

export interface VideoGenerationOptions {
  prompt: string;
  output_dir: string;
  width?: number;
  height?: number;
  num_frames?: number;
  frame_rate?: number;
  seed?: number;
  negative_prompt?: string;
  // 图生视频相关参数
  image?: string;
  images?: string[];
  image_mode?: VideoGenerationMode;
}

export interface VideoGenerationResult {
  success: boolean;
  video_path?: string;
  error?: string;
}

export type VideoGenerationStatus = "idle" | "creating" | "processing" | "downloading" | "success" | "error";

export async function generateVideo(options: VideoGenerationOptions): Promise<VideoGenerationResult> {
  return invoke<VideoGenerationResult>("generate_video", { options });
}

export async function getVideoOutputDir(): Promise<string> {
  return invoke<string>("get_video_output_dir");
}

// Agnes 模型管理相关
export interface AgnesModel {
  id: string;
  name: string;
  description: string;
}

export interface AgnesModels {
  text_to_text: AgnesModel[];
  text_to_image: AgnesModel[];
  text_to_video: AgnesModel[];
}

export interface AgnesModelsStore {
  text_to_text: AgnesModel[];
  text_to_image: AgnesModel[];
  text_to_video: AgnesModel[];
  last_updated?: number;
}

export interface UpdateAgnesModelsRequest {
  endpoint: string;
  api_key: string;
}

export interface UpdateAgnesModelsResponse {
  success: boolean;
  message: string;
  data?: AgnesModelsStore;
}

export async function updateAgnesModels(request: UpdateAgnesModelsRequest): Promise<UpdateAgnesModelsResponse> {
  return invoke<UpdateAgnesModelsResponse>("update_agnes_models", { request });
}

export async function getAgnesModels(): Promise<AgnesModelsStore> {
  return invoke<AgnesModelsStore>("get_agnes_models");
}

export async function getDefaultAgnesModels(): Promise<AgnesModelsStore> {
  return invoke<AgnesModelsStore>("get_default_agnes_models");
}

// 视频库相关
export interface VideoInfo {
  path: string;
  name: string;
  time: number;
}

export async function getVideos(outputDir: string): Promise<VideoInfo[]> {
  return invoke<VideoInfo[]>("get_videos", { outputDir });
}

export async function deleteVideo(path: string): Promise<void> {
  return invoke("delete_video", { path });
}

export async function openVideoDir(path: string): Promise<void> {
  return invoke("open_video_dir", { path });
}

// 通用提供商模型获取
export interface ProviderModel {
  id: string;
  name: string;
  description: string;
}

export interface FetchProviderModelsRequest {
  provider: string;
  api_key: string;
  endpoint?: string;
}

export interface FetchProviderModelsResponse {
  success: boolean;
  message: string;
  models?: ProviderModel[];
}

export async function fetchProviderModels(
  request: FetchProviderModelsRequest
): Promise<FetchProviderModelsResponse> {
  return invoke<FetchProviderModelsResponse>("fetch_provider_models", { request });
}

// 文件夹选择
export async function pickFolder(defaultPath?: string): Promise<string | null> {
  return invoke<string | null>("pick_folder", { defaultPath });
}

// 默认存储路径
export interface DefaultStoragePaths {
  image_dir: string;
  video_dir: string;
  data_dir: string;
}

export async function getDefaultStoragePaths(): Promise<DefaultStoragePaths> {
  return invoke<DefaultStoragePaths>("get_default_storage_paths");
}

// 打开日志目录
export async function openLogDir(): Promise<void> {
  return invoke("open_log_dir");
}

// 获取日志内容
export async function getLogContent(): Promise<string> {
  return invoke<string>("get_log_content");
}

// 重新下载图片相关
export interface RetryDownloadOptions {
  image_url: string;
  output_dir: string;
  filename?: string;
}

export interface RetryDownloadResult {
  success: boolean;
  image_path?: string;
  error?: string;
}

export async function retryDownloadImage(
  options: RetryDownloadOptions
): Promise<RetryDownloadResult> {
  return invoke<RetryDownloadResult>("retry_download_image", { 
    imageUrl: options.image_url,
    outputDir: options.output_dir,
    filename: options.filename
  });
}
