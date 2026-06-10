use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("网络错误: {0}")]
    Network(#[from] reqwest::Error),

    #[error("API 错误 [{status}]: {message}")]
    Api { status: u16, message: String },

    #[error("响应解析错误: {0}")]
    InvalidResponse(String),

    #[error("配置错误: {0}")]
    Config(String),

    #[error("文件系统错误: {0}")]
    FileSystem(#[from] std::io::Error),

    #[error("未知错误: {0}")]
    Unknown(String),

    #[error("图片下载失败 [{url}]: {message}")]
    DownloadFailed { url: String, message: String },
}

pub type Result<T> = std::result::Result<T, ProviderError>;
