//! Draws the 3D scene into a texture with wgpu. The texture is shown in the
//! window as an image, and can be read back to save a picture or to test.

use bytemuck::{Pod, Zeroable};
use eframe::wgpu;
use eframe::wgpu::util::DeviceExt;

use super::camera::Camera;
use super::scene::{Scene, Vertex};

/// Colour target format: 8-bit sRGB, so lighting is computed in linear
/// light and stored gamma-encoded.
pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
/// The same bytes read as plain 8-bit values: egui expects gamma-encoded
/// colours in its textures.
const DISPLAY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const NORMAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// Multisample antialiasing: layers thinner than a pixel (a 3 mm membrane
/// seen from afar) fade into a faint line instead of breaking into dashes.
const SAMPLES: u32 = 4;
/// Background, a very light grey-blue (linear RGB).
const BACKGROUND: wgpu::Color = wgpu::Color {
    r: 0.88,
    g: 0.90,
    b: 0.93,
    a: 1.0,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Globals {
    proj: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    sun: [f32; 4],
    edges: [f32; 4],
    clip_min: [f32; 4],
    clip_max: [f32; 4],
    eye: [f32; 4],
    forward: [f32; 4],
}

/// Render targets of one size.
struct Targets {
    size: [u32; 2],
    /// The finished picture.
    color: wgpu::Texture,
    color_view: wgpu::TextureView,
    /// Multisampled colour, normal and depth, resolved into `color` and
    /// `normal_view`.
    color_ms_view: wgpu::TextureView,
    normal_ms_view: wgpu::TextureView,
    normal_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    edges_bind: wgpu::BindGroup,
    /// A plain 8-bit copy of the picture for egui, on graphics cards that
    /// cannot view the sRGB picture as plain values (some OpenGL drivers).
    display: Option<wgpu::Texture>,
}

pub struct Renderer {
    model_pipeline: wgpu::RenderPipeline,
    glass_pipeline: wgpu::RenderPipeline,
    edges_pipeline: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    globals_bind: wgpu::BindGroup,
    edges_layout: wgpu::BindGroupLayout,
    targets: Option<Targets>,
    opaque: Option<(wgpu::Buffer, u32)>,
    glass: Option<(wgpu::Buffer, u32)>,
    /// The device can view a texture in another format of the same bytes
    /// (wgpu's `VIEW_FORMATS`); otherwise the picture is copied for display.
    view_formats: bool,
    /// The section box, if on: everything outside it is cut away.
    clip: Option<(glam::Vec3, glam::Vec3)>,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, view_formats: bool) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("archisim 3d"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders.wgsl").into()),
        });
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let edges_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("normal and distance"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let globals = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            }],
        });
        let model_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("model"),
            bind_group_layouts: &[Some(&globals_layout)],
            immediate_size: 0,
        });
        let edges_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("edges"),
                bind_group_layouts: &[Some(&globals_layout), Some(&edges_layout)],
                immediate_size: 0,
            });
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32],
        };
        let primitive = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            // Back faces are dropped by the shader, except through the cut
            // of a section box, where they are painted as its caps.
            cull_mode: None,
            ..Default::default()
        };
        let model_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("model"),
            layout: Some(&model_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_model"),
                compilation_options: Default::default(),
                buffers: &[Some(vertex_layout.clone())],
            },
            primitive,
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: SAMPLES,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_model"),
                compilation_options: Default::default(),
                targets: &[Some(COLOR_FORMAT.into()), Some(NORMAL_FORMAT.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let glass_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("glass"),
            layout: Some(&model_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_model"),
                compilation_options: Default::default(),
                buffers: &[Some(vertex_layout.clone())],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..primitive
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: SAMPLES,
                ..Default::default()
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_glass"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let edges_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("edges"),
            layout: Some(&edges_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fullscreen"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_edges"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            model_pipeline,
            glass_pipeline,
            edges_pipeline,
            globals,
            globals_bind,
            edges_layout,
            targets: None,
            opaque: None,
            glass: None,
            view_formats,
            clip: None,
        }
    }

    /// Sets (or clears) the section box of the next pictures.
    pub fn set_clip(&mut self, clip: Option<(glam::Vec3, glam::Vec3)>) {
        self.clip = clip;
    }

    /// Uploads the scene's triangles.
    pub fn set_scene(&mut self, device: &wgpu::Device, scene: &Scene) {
        let upload = |label: &str, v: &[Vertex]| {
            (!v.is_empty()).then(|| {
                let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: bytemuck::cast_slice(v),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                (buf, v.len() as u32)
            })
        };
        self.opaque = upload("opaque", &scene.opaque);
        self.glass = upload("glass", &scene.glass);
    }

    fn targets(&mut self, device: &wgpu::Device, size: [u32; 2]) -> &Targets {
        if self.targets.as_ref().is_none_or(|t| t.size != size) {
            let extent = wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            };
            let texture = |label: &str,
                           format,
                           usage,
                           view_formats: &[wgpu::TextureFormat],
                           sample_count: u32| {
                device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: extent,
                    mip_level_count: 1,
                    sample_count,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats,
                })
            };
            let attach = wgpu::TextureUsages::RENDER_ATTACHMENT;
            let color = texture(
                "3d colour",
                COLOR_FORMAT,
                attach | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
                if self.view_formats {
                    &[DISPLAY_FORMAT]
                } else {
                    &[]
                },
                1,
            );
            let display = (!self.view_formats).then(|| {
                texture(
                    "3d display copy",
                    DISPLAY_FORMAT,
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    &[],
                    1,
                )
            });
            let color_ms = texture("3d colour samples", COLOR_FORMAT, attach, &[], SAMPLES);
            let normal = texture(
                "3d normal and distance",
                NORMAL_FORMAT,
                attach | wgpu::TextureUsages::TEXTURE_BINDING,
                &[],
                1,
            );
            let normal_ms = texture("3d normal samples", NORMAL_FORMAT, attach, &[], SAMPLES);
            let depth = texture("3d depth", DEPTH_FORMAT, attach, &[], SAMPLES);
            let normal_view = normal.create_view(&Default::default());
            let edges_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("normal and distance"),
                layout: &self.edges_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&normal_view),
                }],
            });
            self.targets = Some(Targets {
                size,
                color_view: color.create_view(&Default::default()),
                color,
                color_ms_view: color_ms.create_view(&Default::default()),
                normal_ms_view: normal_ms.create_view(&Default::default()),
                normal_view,
                depth_view: depth.create_view(&Default::default()),
                edges_bind,
                display,
            });
        }
        self.targets.as_ref().expect("just created")
    }

    /// Draws the scene seen by `camera` into a `size` pixel texture and
    /// returns it. `edge_step` is the contour width in pixels.
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera: &Camera,
        size: [u32; 2],
        edge_step: u32,
    ) -> &wgpu::Texture {
        let size = [size[0].clamp(1, 8192), size[1].clamp(1, 8192)];
        let aspect = size[0] as f32 / size[1] as f32;
        let sun = glam::Vec3::new(-0.45, -0.6, 0.66).normalize();
        let globals = Globals {
            proj: camera.projection(aspect).to_cols_array_2d(),
            view: camera.view().to_cols_array_2d(),
            sun: [sun.x, sun.y, sun.z, 0.0],
            edges: [edge_step.max(1) as f32, 1.0, 0.0, 0.0],
            clip_min: self
                .clip
                .map_or([0.0; 4], |(lo, _)| [lo.x, lo.y, lo.z, 1.0]),
            clip_max: self
                .clip
                .map_or([0.0; 4], |(_, hi)| [hi.x, hi.y, hi.z, 1.0]),
            eye: {
                let e = camera.eye();
                [e.x, e.y, e.z, if camera.perspective { 1.0 } else { 0.0 }]
            },
            forward: {
                let f = camera.forward();
                [f.x, f.y, f.z, 0.0]
            },
        };
        queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));
        self.targets(device, size);
        let t = self.targets.as_ref().expect("targets exist");

        let mut encoder = device.create_command_encoder(&Default::default());
        let depth = |load| {
            Some(wgpu::RenderPassDepthStencilAttachment {
                view: &t.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            })
        };
        // 1. The opaque model, with its normals and distances.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("model"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &t.color_ms_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(BACKGROUND),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &t.normal_ms_view,
                        depth_slice: None,
                        resolve_target: Some(&t.normal_view),
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Discard,
                        },
                    }),
                ],
                depth_stencil_attachment: depth(wgpu::LoadOp::Clear(1.0)),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some((buf, n)) = &self.opaque {
                pass.set_pipeline(&self.model_pipeline);
                pass.set_bind_group(0, &self.globals_bind, &[]);
                pass.set_vertex_buffer(0, buf.slice(..));
                pass.draw(0..*n, 0..1);
            }
        }
        // 2. The glass over it; the samples are averaged into the picture.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("glass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &t.color_ms_view,
                    depth_slice: None,
                    resolve_target: Some(&t.color_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: depth(wgpu::LoadOp::Load),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if let Some((buf, n)) = &self.glass {
                pass.set_pipeline(&self.glass_pipeline);
                pass.set_bind_group(0, &self.globals_bind, &[]);
                pass.set_vertex_buffer(0, buf.slice(..));
                pass.draw(0..*n, 0..1);
            }
        }
        // 3. Contour lines in ink.
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("contours"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &t.color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.edges_pipeline);
            pass.set_bind_group(0, &self.globals_bind, &[]);
            pass.set_bind_group(1, &t.edges_bind, &[]);
            pass.draw(0..3, 0..1);
        }
        if let Some(display) = &t.display {
            encoder.copy_texture_to_texture(
                t.color.as_image_copy(),
                display.as_image_copy(),
                t.color.size(),
            );
        }
        queue.submit([encoder.finish()]);
        &t.color
    }

    /// A view of the last picture for egui (gamma-encoded values).
    pub fn display_view(&self) -> Option<wgpu::TextureView> {
        let t = self.targets.as_ref()?;
        if let Some(display) = &t.display {
            return Some(display.create_view(&Default::default()));
        }
        Some(t.color.create_view(&wgpu::TextureViewDescriptor {
            format: Some(DISPLAY_FORMAT),
            ..Default::default()
        }))
    }

    /// The last picture as RGBA bytes (sRGB), row by row.
    pub fn read_pixels(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<(u32, u32, Vec<u8>)> {
        let t = self.targets.as_ref()?;
        let [w, h] = t.size;
        let row = (w * 4).div_ceil(256) * 256;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3d readback"),
            size: (row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            t.color.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::PollType::wait_indefinitely()).ok()?;
        rx.recv().ok()?.ok()?;
        let data = slice.get_mapped_range().ok()?;
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            let start = (y * row) as usize;
            out.extend_from_slice(&data[start..start + (w * 4) as usize]);
        }
        drop(data);
        buffer.unmap();
        Some((w, h, out))
    }
}
