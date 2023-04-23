use bevy::prelude::UVec2;
use image::{error::UnsupportedErrorKind, ColorType, ImageBuffer, ImageError, Rgba};

pub struct SaveImageDescriptor<'a> {
    pub data: Vec<u8>,
    pub resolution: UVec2,
    pub frame_id: u32,
    pub output_dir: &'a str,
    pub extension: &'a str,
}

pub fn save_image_file(desc: SaveImageDescriptor) {
    std::fs::create_dir_all(desc.output_dir).expect("Output path could not be created");

    let path = format!(
        "{}/{:05}.{}",
        desc.output_dir, desc.frame_id, desc.extension
    );

    let buffer =
        ImageBuffer::<Rgba<u8>, _>::from_raw(desc.resolution.x, desc.resolution.y, desc.data)
            .unwrap();

    match buffer.save(path) {
        Err(ImageError::Unsupported(err)) => {
            if let UnsupportedErrorKind::Format(hint) = err.kind() {
                println!("image format {} is not supported", hint);
            }
        }
        _ => {}
    }
}

pub fn crop_image_width(data: &mut Vec<u8>, resolution: UVec2, target_width: u32) {
    let bpp = ColorType::Rgba8.bytes_per_pixel() as usize;
    let ow = target_width as usize;
    let mut corrected_data = Vec::<u8>::with_capacity(bpp * ow * resolution.y as usize);

    for chunk in data.chunks(bpp * resolution.x as usize) {
        corrected_data.extend_from_slice(&chunk[..bpp * ow]);
    }

    *data = corrected_data;
}
