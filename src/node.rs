use std::num::NonZeroU32;

use bevy::{
    prelude::*,
    render::{render_asset::RenderAssets, render_resource::*, renderer::RenderDevice},
};

use super::ecs::ImageExportTask;

pub const NODE_NAME: &str = "image_export";

#[derive(Default)]
pub struct ImageExportNode {
    frame_id: u32,
    tasks: Vec<ImageExportTask>,
}

impl bevy::render::render_graph::Node for ImageExportNode {
    fn update(&mut self, world: &mut World) {
        self.frame_id = self.frame_id.wrapping_add(1);
        self.tasks = world
            .query::<&ImageExportTask>()
            .iter(world)
            .cloned()
            .collect();
    }

    fn run(
        &self,
        _graph: &mut bevy::render::render_graph::RenderGraphContext,
        render_context: &mut bevy::render::renderer::RenderContext,
        world: &World,
    ) -> Result<(), bevy::render::render_graph::NodeRunError> {
        self.tasks.iter().for_each(
            |ImageExportTask {
                 render_target,
                 output_buffer,
                 size,
                 ..
             }| {
                let image = world
                    .get_resource::<RenderAssets<Image>>()
                    .unwrap()
                    .get(&render_target)
                    .unwrap();

                let format = image.texture_format.describe();

                let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(
                    (size.x as usize / format.block_dimensions.0 as usize)
                        * format.block_size as usize,
                );

                render_context.command_encoder().copy_texture_to_buffer(
                    image.texture.as_image_copy(),
                    ImageCopyBuffer {
                        buffer: &output_buffer,
                        layout: ImageDataLayout {
                            offset: 0,
                            bytes_per_row: Some(
                                NonZeroU32::new(padded_bytes_per_row as u32).unwrap(),
                            ),
                            rows_per_image: Some(NonZeroU32::new(size.y).unwrap()),
                        },
                    },
                    Extent3d {
                        width: size.x,
                        height: size.y,
                        ..default()
                    },
                );
            },
        );

        Ok(())
    }
}
