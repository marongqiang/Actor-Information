use crate::db;
use crate::utils::error::CommandResult;
use sha2::Digest;
use std::path::PathBuf;

/// Get the images cache directory
pub fn get_images_dir() -> PathBuf {
    db::get_data_dir().join("images")
}

/// Download an image from URL and save to local cache as WebP.
/// Returns the local file path.
pub async fn download_and_cache(url: &str, category: &str) -> CommandResult<String> {
    let images_dir = get_images_dir().join(category);
    std::fs::create_dir_all(&images_dir).map_err(|e| {
        crate::utils::error::CommandError::internal(&format!("创建图片目录失败: {}", e))
    })?;

    // Generate filename from URL hash
    let mut hasher = sha2::Sha256::new();
    hasher.update(url.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    let filename = format!("{}.webp", &hash[..16]);
    let filepath = images_dir.join(&filename);

    // Check if already cached
    if filepath.exists() {
        return Ok(filepath.to_string_lossy().to_string());
    }

    // Download image
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| crate::utils::error::CommandError::network(&format!("创建HTTP客户端失败: {}", e)))?;

    let response = client.get(url).send().await?;
    let bytes = response.bytes().await?;

    // Convert to WebP using image crate
    let img = image::load_from_memory(&bytes).map_err(|e| {
        crate::utils::error::CommandError::internal(&format!("图片解码失败: {}", e))
    })?;

    // Resize if too large (max 800px width for posters, 400px for avatars)
    let max_width = if category == "avatars" { 400u32 } else { 800u32 };
    let img = if img.width() > max_width {
        let ratio = max_width as f64 / img.width() as f64;
        let new_height = (img.height() as f64 * ratio) as u32;
        img.resize(max_width, new_height, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    // Save as WebP
    let webp = webp_encode(&img)?;
    std::fs::write(&filepath, &webp).map_err(|e| {
        crate::utils::error::CommandError::internal(&format!("保存图片失败: {}", e))
    })?;

    Ok(filepath.to_string_lossy().to_string())
}

/// Simple WebP encoding using the image crate's built-in support
fn webp_encode(img: &image::DynamicImage) -> CommandResult<Vec<u8>> {
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    // Encode as WebP using the image crate
    let mut webp_data = Vec::new();
    {
        let mut cursor = std::io::Cursor::new(&mut webp_data);
        let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut cursor);
        encoder.encode(&rgba, width, height, image::ExtendedColorType::Rgba8)
            .map_err(|e| crate::utils::error::CommandError::internal(&format!("WebP编码失败: {}", e)))?;
    }

    Ok(webp_data)
}
