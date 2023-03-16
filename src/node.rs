use crate::plugin::ImageExportTask;
use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{Node, NodeRunError, RenderGraphContext},
        render_resource::{Extent3d, ImageCopyBuffer, ImageDataLayout},
        renderer::{RenderContext, RenderDevice},
    },
};
use std::num::NonZeroU32;

pub const NODE_NAME: &str = "image_export";

#[derive(Default)]
pub struct ImageExportNode {
    tasks: Vec<ImageExportTask>,
}

impl Node for ImageExportNode {
    fn update(&mut self, world: &mut World) {
        self.tasks = world
            .query::<&ImageExportTask>()
            .iter(world)
            .cloned()
            .collect();
    }

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        for ImageExportTask {
            render_target,
            output_buffer,
            resolution,
            ..
        } in self.tasks.iter()
        {
            let image = world
                .get_resource::<RenderAssets<Image>>()
                .unwrap()
                .get(&render_target)
                .unwrap();

            let format = image.texture_format.describe();

            let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(
                (resolution.x as usize / format.block_dimensions.0 as usize)
                    * format.block_size as usize,
            );

            render_context.command_encoder().copy_texture_to_buffer(
                image.texture.as_image_copy(),
                ImageCopyBuffer {
                    buffer: &output_buffer,
                    layout: ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(NonZeroU32::new(padded_bytes_per_row as u32).unwrap()),
                        rows_per_image: None,
                    },
                },
                Extent3d {
                    width: resolution.x,
                    height: resolution.y,
                    ..default()
                },
            );
        }

        Ok(())
    }
}
