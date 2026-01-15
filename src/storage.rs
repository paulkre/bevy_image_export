use bytemuck::AnyBitPattern;
use image::{EncodableLayout, ImageBuffer, Pixel, PixelWithColorType, Rgba};
use std::fs::create_dir_all;

#[derive(Debug, thiserror::Error)]
pub enum ImageStorageError {
    #[error("Failed to create directory: {0}")]
    CreateDir(#[from] std::io::Error),
    #[error("Failed to save image buffer: {0}")]
    SaveImageBuffer(#[from] image::ImageError),
    #[error("Failed to create image buffer")]
    BufferCreation,
}

fn save_buffer<P: Pixel + PixelWithColorType>(
    image_bytes: &[P::Subpixel],
    width: u32,
    height: u32,
    path: &str,
) -> Result<(), ImageStorageError>
where
    P::Subpixel: AnyBitPattern,
    [P::Subpixel]: EncodableLayout,
{
    let Some(buffer) = ImageBuffer::<P, _>::from_raw(width, height, image_bytes) else {
        return Err(ImageStorageError::BufferCreation);
    };

    buffer.save(path)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn save_image(
    output_dir: &str,
    extension: &str,
    mut image_bytes: Vec<u8>,
    bytes_per_row: usize,
    padded_bytes_per_row: usize,
    width: u32,
    height: u32,
    frame_id: u64,
) -> Result<(), ImageStorageError> {
    create_dir_all(output_dir)?;

    if bytes_per_row != padded_bytes_per_row {
        let mut unpadded_bytes = Vec::<u8>::with_capacity(height as usize * bytes_per_row);

        for padded_row in image_bytes.chunks(padded_bytes_per_row) {
            unpadded_bytes.extend_from_slice(&padded_row[..bytes_per_row]);
        }

        image_bytes = unpadded_bytes;
    }

    let path = format!("{}/{:05}.{}", output_dir, frame_id, extension);

    match extension {
        "exr" => {
            save_buffer::<Rgba<f32>>(
                bytemuck::cast_slice(&image_bytes),
                width,
                height,
                path.as_str(),
            )?;
        }
        _ => {
            save_buffer::<Rgba<u8>>(&image_bytes, width, height, path.as_str())?;
        }
    }

    Ok(())
}
