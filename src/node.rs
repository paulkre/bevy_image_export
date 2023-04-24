use crate::ImageExporterSource;
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{Node, NodeRunError, RenderGraphContext},
        render_resource::{ImageCopyBuffer, ImageDataLayout},
        renderer::RenderContext,
    },
};
use std::num::NonZeroU32;

pub const NODE_NAME: &str = "image_export";

pub struct ImageExportNode;
impl Node for ImageExportNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        for source in world
            .resource::<RenderAssets<ImageExporterSource>>()
            .values()
        {
            if let Some(gpu_image) = world
                .resource::<RenderAssets<Image>>()
                .get(&source.source_handle)
            {
                render_context.command_encoder().copy_texture_to_buffer(
                    gpu_image.texture.as_image_copy(),
                    ImageCopyBuffer {
                        buffer: &source.buffer,
                        layout: ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(
                                NonZeroU32::new(source.padded_bytes_per_row).unwrap(),
                            ),
                            rows_per_image: None,
                        },
                    },
                    source.source_size,
                );
            }
        }

        Ok(())
    }
}
