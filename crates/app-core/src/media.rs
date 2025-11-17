//! Media handling and image processing
//!
//! This module provides functionality for handling media files, particularly images.
//! Features include:
//! - Image loading from bytes or files
//! - Format conversion (JPEG, PNG)
//! - Compression and resizing
//! - Size and dimension validation
//! - Integration with AT Protocol blob storage

use image::{imageops::FilterType, DynamicImage, GenericImageView, ImageFormat};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use thiserror::Error;

/// Maximum image file size (1MB for Bluesky)
pub const MAX_IMAGE_SIZE: usize = 1_000_000;

/// Maximum image dimension (2000px for Bluesky)
pub const MAX_IMAGE_DIMENSION: u32 = 2000;

/// Default JPEG quality for compression
pub const DEFAULT_JPEG_QUALITY: u8 = 85;

/// Errors that can occur during media operations
#[derive(Debug, Error)]
pub enum MediaError {
    /// Image decoding error
    #[error("Failed to decode image: {0}")]
    DecodeError(String),

    /// Image encoding error
    #[error("Failed to encode image: {0}")]
    EncodeError(String),

    /// Unsupported format
    #[error("Unsupported image format: {0}")]
    UnsupportedFormat(String),

    /// File too large
    #[error("File size {size} exceeds maximum {max}")]
    FileTooLarge {
        /// Actual file size
        size: usize,
        /// Maximum allowed size
        max: usize,
    },

    /// Dimension too large
    #[error("Image dimension {dimension} exceeds maximum {max}")]
    DimensionTooLarge {
        /// Actual dimension
        dimension: u32,
        /// Maximum allowed dimension
        max: u32,
    },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid MIME type
    #[error("Invalid MIME type: {0}")]
    InvalidMimeType(String),
}

/// Result type for media operations
pub type Result<T> = std::result::Result<T, MediaError>;

/// Supported image formats for upload
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupportedFormat {
    /// JPEG format
    Jpeg,
    /// PNG format
    Png,
}

impl SupportedFormat {
    /// Get the MIME type for this format
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
        }
    }

    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
        }
    }

    /// Get the ImageFormat for this format
    pub fn to_image_format(&self) -> ImageFormat {
        match self {
            Self::Jpeg => ImageFormat::Jpeg,
            Self::Png => ImageFormat::Png,
        }
    }

    /// Try to detect format from MIME type
    pub fn from_mime_type(mime_type: &str) -> Result<Self> {
        match mime_type {
            "image/jpeg" | "image/jpg" => Ok(Self::Jpeg),
            "image/png" => Ok(Self::Png),
            _ => Err(MediaError::InvalidMimeType(mime_type.to_string())),
        }
    }
}

/// Processed image ready for upload
#[derive(Debug, Clone)]
pub struct ProcessedImage {
    /// Image data as bytes
    pub data: Vec<u8>,
    /// MIME type
    pub mime_type: String,
    /// Image width
    pub width: u32,
    /// Image height
    pub height: u32,
    /// File size in bytes
    pub size: usize,
}

impl ProcessedImage {
    /// Get aspect ratio
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}

/// Image processing options
#[derive(Debug, Clone)]
pub struct ImageOptions {
    /// Target format (default: JPEG)
    pub format: SupportedFormat,
    /// Maximum width (resizes if larger)
    pub max_width: Option<u32>,
    /// Maximum height (resizes if larger)
    pub max_height: Option<u32>,
    /// JPEG quality (1-100, default: 85)
    pub jpeg_quality: u8,
    /// Resize filter (default: Lanczos3)
    pub filter: FilterType,
}

impl Default for ImageOptions {
    fn default() -> Self {
        Self {
            format: SupportedFormat::Jpeg,
            max_width: Some(MAX_IMAGE_DIMENSION),
            max_height: Some(MAX_IMAGE_DIMENSION),
            jpeg_quality: DEFAULT_JPEG_QUALITY,
            filter: FilterType::Lanczos3,
        }
    }
}

/// Image processor for loading, converting, and compressing images
pub struct ImageProcessor {
    options: ImageOptions,
}

impl ImageProcessor {
    /// Create a new image processor with default options
    pub fn new() -> Self {
        Self {
            options: ImageOptions::default(),
        }
    }

    /// Create an image processor with custom options
    pub fn with_options(options: ImageOptions) -> Self {
        Self { options }
    }

    /// Set the target format
    pub fn with_format(mut self, format: SupportedFormat) -> Self {
        self.options.format = format;
        self
    }

    /// Set the maximum dimensions
    pub fn with_max_dimensions(mut self, width: u32, height: u32) -> Self {
        self.options.max_width = Some(width);
        self.options.max_height = Some(height);
        self
    }

    /// Set JPEG quality
    pub fn with_jpeg_quality(mut self, quality: u8) -> Self {
        self.options.jpeg_quality = quality.clamp(1, 100);
        self
    }

    /// Process image from bytes
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use app_core::media::ImageProcessor;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let image_bytes = std::fs::read("photo.jpg")?;
    /// let processor = ImageProcessor::new();
    /// let processed = processor.process_bytes(&image_bytes)?;
    /// println!("Processed image: {}x{}, {} bytes",
    ///     processed.width, processed.height, processed.size);
    /// # Ok(())
    /// # }
    /// ```
    pub fn process_bytes(&self, bytes: &[u8]) -> Result<ProcessedImage> {
        // Validate initial size
        if bytes.len() > MAX_IMAGE_SIZE * 2 {
            // Allow 2x for uncompressed input
            return Err(MediaError::FileTooLarge {
                size: bytes.len(),
                max: MAX_IMAGE_SIZE * 2,
            });
        }

        // Load image
        let img = image::load_from_memory(bytes).map_err(|e| MediaError::DecodeError(e.to_string()))?;

        self.process_image(img)
    }

    /// Process a DynamicImage
    fn process_image(&self, mut img: DynamicImage) -> Result<ProcessedImage> {
        let (width, height) = img.dimensions();

        // Validate original dimensions
        if width > MAX_IMAGE_DIMENSION * 2 || height > MAX_IMAGE_DIMENSION * 2 {
            return Err(MediaError::DimensionTooLarge {
                dimension: width.max(height),
                max: MAX_IMAGE_DIMENSION * 2,
            });
        }

        // Resize if necessary
        let needs_resize = if let (Some(max_w), Some(max_h)) =
            (self.options.max_width, self.options.max_height)
        {
            width > max_w || height > max_h
        } else {
            false
        };

        if needs_resize {
            let max_w = self.options.max_width.unwrap_or(u32::MAX);
            let max_h = self.options.max_height.unwrap_or(u32::MAX);

            // Maintain aspect ratio - determine which dimension is the limiting factor
            let (final_width, final_height) = if width as f32 / max_w as f32 > height as f32 / max_h as f32 {
                // Width is the limiting factor
                (max_w, (max_w as f32 / width as f32 * height as f32) as u32)
            } else {
                // Height is the limiting factor
                ((max_h as f32 / height as f32 * width as f32) as u32, max_h)
            };

            img = img.resize(final_width, final_height, self.options.filter);
        }

        // Encode to target format
        let mut output = Vec::new();
        let mut cursor = Cursor::new(&mut output);

        match self.options.format {
            SupportedFormat::Jpeg => {
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                    &mut cursor,
                    self.options.jpeg_quality,
                );
                img.write_with_encoder(encoder)
                    .map_err(|e| MediaError::EncodeError(e.to_string()))?;
            }
            SupportedFormat::Png => {
                img.write_to(&mut cursor, ImageFormat::Png)
                    .map_err(|e| MediaError::EncodeError(e.to_string()))?;
            }
        }

        let data = output;
        let size = data.len();

        // Validate final size
        if size > MAX_IMAGE_SIZE {
            return Err(MediaError::FileTooLarge {
                size,
                max: MAX_IMAGE_SIZE,
            });
        }

        let (final_width, final_height) = img.dimensions();

        Ok(ProcessedImage {
            data,
            mime_type: self.options.format.mime_type().to_string(),
            width: final_width,
            height: final_height,
            size,
        })
    }

    /// Auto-process image with smart quality adjustment
    ///
    /// This method will automatically reduce quality if the image is too large
    pub fn auto_process_bytes(&self, bytes: &[u8]) -> Result<ProcessedImage> {
        let mut options = self.options.clone();
        let mut quality = options.jpeg_quality;

        loop {
            let processor = ImageProcessor::with_options(options.clone());
            match processor.process_bytes(bytes) {
                Ok(processed) if processed.size <= MAX_IMAGE_SIZE => return Ok(processed),
                Ok(_) if quality > 60 => {
                    // Reduce quality and try again
                    quality -= 5;
                    options.jpeg_quality = quality;
                }
                Ok(_) => {
                    // Quality is already low, image is just too large
                    return Err(MediaError::FileTooLarge {
                        size: bytes.len(),
                        max: MAX_IMAGE_SIZE,
                    });
                }
                Err(e) => return Err(e),
            }
        }
    }
}

impl Default for ImageProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate image dimensions
pub fn validate_dimensions(width: u32, height: u32) -> Result<()> {
    if width > MAX_IMAGE_DIMENSION {
        return Err(MediaError::DimensionTooLarge {
            dimension: width,
            max: MAX_IMAGE_DIMENSION,
        });
    }
    if height > MAX_IMAGE_DIMENSION {
        return Err(MediaError::DimensionTooLarge {
            dimension: height,
            max: MAX_IMAGE_DIMENSION,
        });
    }
    Ok(())
}

/// Validate image file size
pub fn validate_size(size: usize) -> Result<()> {
    if size > MAX_IMAGE_SIZE {
        return Err(MediaError::FileTooLarge {
            size,
            max: MAX_IMAGE_SIZE,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_image(width: u32, height: u32) -> DynamicImage {
        DynamicImage::ImageRgb8(image::RgbImage::new(width, height))
    }

    #[test]
    fn test_supported_format_mime_types() {
        assert_eq!(SupportedFormat::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(SupportedFormat::Png.mime_type(), "image/png");
    }

    #[test]
    fn test_supported_format_extensions() {
        assert_eq!(SupportedFormat::Jpeg.extension(), "jpg");
        assert_eq!(SupportedFormat::Png.extension(), "png");
    }

    #[test]
    fn test_supported_format_from_mime_type() {
        assert_eq!(
            SupportedFormat::from_mime_type("image/jpeg").unwrap(),
            SupportedFormat::Jpeg
        );
        assert_eq!(
            SupportedFormat::from_mime_type("image/png").unwrap(),
            SupportedFormat::Png
        );
        assert!(SupportedFormat::from_mime_type("image/gif").is_err());
    }

    #[test]
    fn test_image_options_default() {
        let options = ImageOptions::default();
        assert_eq!(options.format, SupportedFormat::Jpeg);
        assert_eq!(options.max_width, Some(MAX_IMAGE_DIMENSION));
        assert_eq!(options.max_height, Some(MAX_IMAGE_DIMENSION));
        assert_eq!(options.jpeg_quality, DEFAULT_JPEG_QUALITY);
    }

    #[test]
    fn test_image_processor_creation() {
        let processor = ImageProcessor::new();
        assert_eq!(processor.options.format, SupportedFormat::Jpeg);

        let processor = ImageProcessor::new()
            .with_format(SupportedFormat::Png)
            .with_max_dimensions(1000, 1000)
            .with_jpeg_quality(90);

        assert_eq!(processor.options.format, SupportedFormat::Png);
        assert_eq!(processor.options.max_width, Some(1000));
        assert_eq!(processor.options.max_height, Some(1000));
        assert_eq!(processor.options.jpeg_quality, 90);
    }

    #[test]
    fn test_validate_dimensions() {
        assert!(validate_dimensions(1000, 1000).is_ok());
        assert!(validate_dimensions(MAX_IMAGE_DIMENSION, MAX_IMAGE_DIMENSION).is_ok());
        assert!(validate_dimensions(MAX_IMAGE_DIMENSION + 1, 1000).is_err());
        assert!(validate_dimensions(1000, MAX_IMAGE_DIMENSION + 1).is_err());
    }

    #[test]
    fn test_validate_size() {
        assert!(validate_size(500_000).is_ok());
        assert!(validate_size(MAX_IMAGE_SIZE).is_ok());
        assert!(validate_size(MAX_IMAGE_SIZE + 1).is_err());
    }

    #[test]
    fn test_processed_image_aspect_ratio() {
        let processed = ProcessedImage {
            data: vec![],
            mime_type: "image/jpeg".to_string(),
            width: 1000,
            height: 500,
            size: 0,
        };
        assert_eq!(processed.aspect_ratio(), 2.0);
    }

    #[test]
    fn test_process_small_image() {
        let img = create_test_image(800, 600);
        let mut bytes = Vec::new();
        img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();

        let processor = ImageProcessor::new();
        let result = processor.process_bytes(&bytes);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert!(processed.size <= MAX_IMAGE_SIZE);
        assert_eq!(processed.mime_type, "image/jpeg"); // Default format
    }

    #[test]
    fn test_process_large_image_with_resize() {
        let img = create_test_image(3000, 2000);
        let mut bytes = Vec::new();
        img.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();

        let processor = ImageProcessor::new().with_max_dimensions(1000, 1000);
        let result = processor.process_bytes(&bytes);
        assert!(result.is_ok());

        let processed = result.unwrap();
        assert!(processed.width <= 1000);
        assert!(processed.height <= 1000);
    }

    #[test]
    fn test_jpeg_quality_clamping() {
        let processor = ImageProcessor::new().with_jpeg_quality(150);
        assert_eq!(processor.options.jpeg_quality, 100);

        let processor = ImageProcessor::new().with_jpeg_quality(0);
        assert_eq!(processor.options.jpeg_quality, 1);
    }
}
