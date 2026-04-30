import { invoke } from "@tauri-apps/api/core";

export type GenerationStatus = "idle" | "generating" | "success" | "error";

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
}

export interface ModelLists {
  modelscope: string[];
  nvidia: string[];
  gemini: string[];
  openrouter: string[];
  openai: string[];
  siliconflow: string[];
}

export interface AppConfig {
  providers: ProvidersConfig;
  default_provider: string;
  default_output_dir: string;
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
