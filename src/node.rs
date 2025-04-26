use crate::GpuImageExportSource;
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{Node, NodeRunError, RenderGraphContext, RenderLabel},
        renderer::RenderContext,
        texture::GpuImage,
    },
};
use wgpu::{TexelCopyBufferInfo, TexelCopyBufferLayout};

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct ImageExportLabel;

pub struct ImageExportNode;
impl Node for ImageExportNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        for (_, source) in world
            .resource::<RenderAssets<GpuImageExportSource>>()
            .iter()
        {
            if let Some(gpu_image) = world
                .resource::<RenderAssets<GpuImage>>()
                .get(&source.source_handle)
            {
                render_context.command_encoder().copy_texture_to_buffer(
                    gpu_image.texture.as_image_copy(),
                    TexelCopyBufferInfo {
                        buffer: &source.buffer,
                        layout: TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(source.padded_bytes_per_row),
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
