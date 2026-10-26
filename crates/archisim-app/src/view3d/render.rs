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

/// Drawing over what is there: the colour by its opacity, and the
/// picture's opacity kept (with wgpu's `ALPHA_BLENDING` it would drop
/// under 1 on every contour line and every pane of glass, and what is
/// behind the picture in the window would show through there).
const OVER: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
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
    cut: [f32; 4],
    front: [f32; 4],
    /// The sun's view and projection, for the shadows.
    sun_vp: [[f32; 4]; 4],
    /// x: shadows on; y: how far a point is moved off its face before
    /// looking it up (metres); z: samples of the shadow map; w: shade of
    /// inner corners on.
    look: [f32; 4],
    /// The inverse of the camera's view and projection: the line of sight
    /// through each pixel, for the sky.
    inv_vp: [[f32; 4]; 4],
    /// The light (`look::LightColours`): the sun's colour and strength,
    /// the sky's above and at the horizon, the zenith, the haze and the
    /// far ground (its w: the ground's height).
    sun_color: [f32; 4],
    sky_hi: [f32; 4],
    sky_lo: [f32; 4],
    zenith: [f32; 4],
    haze: [f32; 4],
    ground: [f32; 4],
    /// The grid on the ground: centre, reach and step (0: none).
    grid: [f32; 4],
    /// x = 1 with the sky drawn; y: night (0…1); z, w: where the haze
    /// begins and where it is thickest, metres from the eye.
    atmos: [f32; 4],
    /// Lamps lit: position and reach; `lamp_count.x` of them.
    lamps: [[f32; 4]; MAX_LAMPS],
    lamp_count: [f32; 4],
    /// What a cut shows: xyz a flat colour (linear), w = 1 to use it;
    /// otherwise each material its own cut colour (`CapStyle`).
    caps: [f32; 4],
}

/// What a cut shows in 3D (pedido por Antonio el 26/10/2026: lo cortado se
/// veía hueco): each material its cut colour, as the plan's poché (a wall
/// with its layers), or one flat colour for everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CapStyle {
    #[default]
    Layers,
    Flat([u8; 3]),
}

/// Lamps that light the scene at night (the nearest to what is looked at).
pub const MAX_LAMPS: usize = 16;

/// The shadow map: the scene's depth seen from the sun.
struct ShadowMap {
    size: u32,
    view: wgpu::TextureView,
    bind: wgpu::BindGroup,
}

/// What a shadow map was drawn for: the scene, the sun, the cut away
/// walls, what is drawn stretched and the map's size.
type ShadowKey = (
    u64,
    [i32; 3],
    Option<super::cutaway::CutRule>,
    u64,
    u32,
    [i32; 3],
    [i32; 3],
);

/// A vertex buffer and how many vertices it has.
type Triangles = (wgpu::Buffer, u32);

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
    /// The textures' pictures (`TextureMode::Pictures`; otherwise one
    /// pixel), and whether they are still to be sent to the card.
    pictures: wgpu::Texture,
    pictures_bind: wgpu::BindGroup,
    pictures_pending: bool,
    model_pipeline: wgpu::RenderPipeline,
    /// The grass: as the model, leaving the normals and distances as they
    /// are.
    grass_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    shadow_layout: wgpu::BindGroupLayout,
    shadow_sampler: wgpu::Sampler,
    /// The shadow map, and what it was drawn for.
    shadow: Option<(ShadowMap, Option<ShadowKey>)>,
    /// A map with nothing in it, for pictures without shadows.
    no_shadow: ShadowMap,
    /// The quality of the picture and the direction towards the sun (none:
    /// below the horizon, no shadows).
    quality: super::look::Quality,
    sun: Option<glam::Vec3>,
    /// The box of the scene, for the sun's view.
    bounds: (glam::Vec3, glam::Vec3),
    /// The part of the scene the shadows are drawn for (around what is
    /// looked at), if not all of it: finer shadows on a large site.
    focus: Option<(glam::Vec3, glam::Vec3)>,
    /// Counts the scenes and overlays uploaded, so the shadows know.
    uploads: u64,
    glass_pipeline: wgpu::RenderPipeline,
    edges_pipeline: wgpu::RenderPipeline,
    globals: wgpu::Buffer,
    globals_bind: wgpu::BindGroup,
    edges_layout: wgpu::BindGroupLayout,
    targets: Option<Targets>,
    opaque: Option<(wgpu::Buffer, u32)>,
    grass: Option<(wgpu::Buffer, u32)>,
    glass: Option<(wgpu::Buffer, u32)>,
    /// What is being stretched, drawn over the scene (whose own copy of
    /// it is hidden): its opaque and glass triangles.
    overlay: Option<(Option<Triangles>, Option<Triangles>)>,
    /// The device can view a texture in another format of the same bytes
    /// (wgpu's `VIEW_FORMATS`); otherwise the picture is copied for display.
    view_formats: bool,
    /// The section box, if on: everything outside it is cut away.
    clip: Option<(glam::Vec3, glam::Vec3)>,
    caps: CapStyle,
    /// The cut away walls, if on.
    cut: Option<super::cutaway::CutRule>,
    /// Something of the scene is selected: the contour pass outlines it.
    selected: bool,
    /// Contours one pixel wide (ArchiCAD's "thin lines"), instead of two.
    pub thin_lines: bool,
    /// The background left transparent (premultiplied: what is drawn
    /// under the picture, the ground grid, shows where there is no model).
    transparent: bool,
    /// The sky, the haze, the light and the grid (the Maqueta's).
    atmosphere: Option<super::look::Atmosphere>,
    /// The lamps of the scene: where their light comes from and how far it
    /// reaches.
    lamps: Vec<[f32; 4]>,
}

/// How the materials' textures are drawn in the 3D view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureMode {
    /// Plain colours: the texture code is left out of the shader.
    Off,
    /// Simpler textures, for graphics drivers that hang on the full ones
    /// (AMD on Intel Macs): no `switch`, no loops.
    Light,
    /// The full textures looked up in pictures made once on the
    /// processor: as good as the full ones and lighter for slow cards.
    Pictures,
    Full,
}

impl TextureMode {
    pub const ALL: [TextureMode; 4] = [Self::Full, Self::Pictures, Self::Light, Self::Off];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Sin texturas",
            Self::Light => "Ligeras",
            Self::Pictures => "En imagen",
            Self::Full => "Completas",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Off => "no",
            Self::Light => "ligeras",
            Self::Pictures => "imagen",
            Self::Full => "completas",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|m| m.key() == k)
    }
}

/// The shaders with the textures drawn as `mode` says.
pub fn shader_source(mode: TextureMode) -> String {
    let full = include_str!("shaders.wgsl");
    let (Some(a), Some(b)) = (full.find("// ---- Textures"), full.find("struct ModelOut")) else {
        return full.to_string();
    };
    match mode {
        TextureMode::Full => full.to_string(),
        TextureMode::Light => format!(
            "{}{}{}",
            &full[..a],
            include_str!("textures_light.wgsl"),
            &full[b..]
        ),
        TextureMode::Pictures => format!(
            "{}{}{}",
            &full[..a],
            include_str!("textures_image.wgsl"),
            &full[b..]
        ),
        TextureMode::Off => format!(
            "{}fn tex_lod(world: vec3<f32>, n_in: vec3<f32>, tex: vec2<f32>) -> f32 {{\n    return 0.0;\n}}\n\nfn textured(base: vec3<f32>, world: vec3<f32>, n_in: vec3<f32>, tex: vec2<f32>) -> vec3<f32> {{\n    return base;\n}}\n\n{}",
            &full[..a],
            &full[b..]
        ),
    }
}

impl Renderer {
    pub fn new(device: &wgpu::Device, view_formats: bool, textures: TextureMode) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("archisim 3d"),
            source: wgpu::ShaderSource::Wgsl(shader_source(textures).into()),
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
        // Bindings 1 and 2 of the group 1 (the contour pass has its
        // picture at binding 0 of its own group 1).
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow map"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        let (pictures, pictures_view) = pictures_texture(device, textures);
        let pictures_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("texture pictures"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        // The group 2 of the model's passes: the textures' pictures.
        let pictures_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture pictures"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pictures_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("texture pictures"),
            layout: &pictures_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&pictures_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&pictures_sampler),
                },
            ],
        });
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow map"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let model_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("model"),
            bind_group_layouts: &[
                Some(&globals_layout),
                Some(&shadow_layout),
                Some(&pictures_layout),
            ],
            immediate_size: 0,
        });
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("shadow"),
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
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32, 4 => Float32x2, 5 => Float32x4, 6 => Float32, 7 => Float32x3],
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
        let grass_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("grass"),
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
                targets: &[
                    Some(COLOR_FORMAT.into()),
                    Some(wgpu::ColorTargetState {
                        format: NORMAL_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::empty(),
                    }),
                ],
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
                    blend: Some(OVER),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_shadow"),
                compilation_options: Default::default(),
                buffers: &[Some(vertex_layout.clone())],
            },
            primitive,
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                // Faces do not shade themselves (shadow acne).
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_shadow"),
                compilation_options: Default::default(),
                targets: &[],
            }),
            multiview_mask: None,
            cache: None,
        });
        let no_shadow = shadow_map(device, &shadow_layout, &shadow_sampler, 1);
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
                    blend: Some(OVER),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self {
            pictures_pending: textures == TextureMode::Pictures,
            pictures,
            pictures_bind,
            model_pipeline,
            grass_pipeline,
            shadow_pipeline,
            shadow_layout,
            shadow_sampler,
            shadow: None,
            no_shadow,
            quality: Default::default(),
            sun: None,
            bounds: (glam::Vec3::ZERO, glam::Vec3::ONE),
            focus: None,
            uploads: 0,
            glass_pipeline,
            edges_pipeline,
            globals,
            globals_bind,
            edges_layout,
            targets: None,
            opaque: None,
            grass: None,
            glass: None,
            overlay: None,
            view_formats,
            clip: None,
            caps: CapStyle::Layers,
            cut: None,
            selected: false,
            thin_lines: false,
            transparent: false,
            atmosphere: None,
            lamps: Vec::new(),
        }
    }

    /// The sky, the light and the grid of the next pictures (none: the
    /// plain background and the project's sun).
    pub fn set_atmosphere(&mut self, a: Option<super::look::Atmosphere>) {
        self.atmosphere = a;
    }

    /// Leaves the background of the next pictures transparent (or not).
    pub fn set_transparent(&mut self, on: bool) {
        self.transparent = on;
    }

    /// Sets (or clears) the section box of the next pictures.
    pub fn set_clip(&mut self, clip: Option<(glam::Vec3, glam::Vec3)>) {
        self.clip = clip;
    }

    /// What the cuts of the section box and the cut away walls show.
    pub fn set_caps(&mut self, caps: CapStyle) {
        self.caps = caps;
    }

    /// The quality of the next pictures and the direction towards the sun
    /// (none: no shadows).
    pub fn set_look(&mut self, quality: super::look::Quality, sun: Option<glam::Vec3>) {
        self.quality = quality;
        self.sun = sun.map(|s| s.normalize_or(glam::Vec3::Z));
    }

    /// The part of the scene the shadows are drawn for (none: all of it).
    pub fn set_shadow_focus(&mut self, focus: Option<(glam::Vec3, glam::Vec3)>) {
        self.focus = focus;
    }

    /// The box the shadows are drawn for: the scene's, cut to the focus
    /// in plan (all its height: what is above still casts shadows).
    fn shadow_region(&self) -> (glam::Vec3, glam::Vec3) {
        let (lo, hi) = self.bounds;
        match self.focus {
            Some((a, b)) => {
                let l = glam::Vec3::new(lo.x.max(a.x), lo.y.max(a.y), lo.z);
                let h = glam::Vec3::new(hi.x.min(b.x), hi.y.min(b.y), hi.z);
                if l.x < h.x && l.y < h.y {
                    (l, h)
                } else {
                    (lo, hi)
                }
            }
            None => (lo, hi),
        }
    }

    /// Sets (or clears) the cut away walls of the next pictures.
    pub fn set_cut(&mut self, cut: Option<super::cutaway::CutRule>) {
        self.cut = cut;
    }

    /// Uploads (or clears) what is drawn over the scene in place of the
    /// selected element (`Scene::of_solids`).
    pub fn set_overlay(&mut self, device: &wgpu::Device, overlay: Option<&Scene>) {
        self.uploads += 1;
        self.overlay = overlay.map(|o| {
            (
                upload(device, "overlay", &o.opaque),
                upload(device, "overlay glass", &o.glass),
            )
        });
    }

    /// Uploads the scene's triangles.
    pub fn set_scene(&mut self, device: &wgpu::Device, scene: &Scene) {
        let upload = |label: &str, v: &[Vertex]| upload(device, label, v);
        self.uploads += 1;
        self.bounds = (scene.lo, scene.hi);
        self.opaque = upload("opaque", &scene.opaque);
        self.grass = upload("grass", &scene.grass);
        self.glass = upload("glass", &scene.glass);
        self.selected = scene.opaque.iter().any(|v| v.select > 0.5);
        self.lamps = scene.lamps.clone();
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
        if self.pictures_pending {
            self.pictures_pending = false;
            upload_pictures(queue, &self.pictures);
        }
        let size = [size[0].clamp(1, 8192), size[1].clamp(1, 8192)];
        let aspect = size[0] as f32 / size[1] as f32;
        let light = self
            .atmosphere
            .as_ref()
            .map_or_else(Default::default, |a| a.light);
        let colours = light.colours();
        let project_sun = light.sun(self.sun);
        // Without the model's sun (below the horizon), a light from the
        // south-west and no shadows; at night, no sun at all.
        let sun = project_sun.unwrap_or(glam::Vec3::new(-0.45, -0.6, 0.66).normalize());
        let shadow_size = project_sun.and(self.quality.shadow_size());
        let sun_strength = if light == super::look::Light::Night {
            0.0
        } else {
            colours.sun[3]
        };
        // The lamps nearest to what is looked at, lit at night.
        let mut lamps = [[0.0f32; 4]; MAX_LAMPS];
        let mut lamp_n = 0;
        if colours.night > 0.0 {
            let mut near: Vec<&[f32; 4]> = self.lamps.iter().collect();
            let t = camera.target;
            near.sort_by(|a, b| {
                let da = glam::Vec3::new(a[0], a[1], a[2]).distance_squared(t);
                let db = glam::Vec3::new(b[0], b[1], b[2]).distance_squared(t);
                da.total_cmp(&db)
            });
            for (slot, l) in lamps.iter_mut().zip(near) {
                *slot = *l;
                lamp_n += 1;
            }
        }
        let vp = camera.projection(aspect) * camera.view();
        let (grid, ground_z, sky_on) = match &self.atmosphere {
            Some(a) => (
                a.grid
                    .map_or([0.0; 4], |(c, reach, step)| [c.x, c.y, reach, step]),
                a.ground_z,
                1.0,
            ),
            None => ([0.0; 4], 0.0, 0.0),
        };
        let haze_from = camera.distance * 1.3 + 6.0;
        let region = self.shadow_region();
        let sun_vp = sun_view_proj(sun, region);
        // A shadow-map pixel, in metres: how far a point is moved off its
        // face before it is looked up.
        let texel = shadow_size.map_or(0.0, |n| {
            let (lo, hi) = region;
            (hi - lo).length() / n as f32
        });
        let globals = Globals {
            proj: camera.projection(aspect).to_cols_array_2d(),
            view: camera.view().to_cols_array_2d(),
            sun: [sun.x, sun.y, sun.z, 0.0],
            edges: [
                edge_step.max(1) as f32,
                if self.thin_lines { -1.0 } else { 1.0 },
                if self.selected || self.overlay.is_some() {
                    1.0
                } else {
                    0.0
                },
                if self.overlay.is_some() { 1.0 } else { 0.0 },
            ],
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
            cut: super::cutaway::CutRule::uniforms(self.cut.as_ref()).0,
            front: super::cutaway::CutRule::uniforms(self.cut.as_ref()).1,
            sun_vp: sun_vp.to_cols_array_2d(),
            look: [
                if shadow_size.is_some() { 1.0 } else { 0.0 },
                texel * 1.5,
                self.quality.shadow_taps(),
                if self.quality.occlusion() { 1.0 } else { 0.0 },
            ],
            inv_vp: vp.inverse().to_cols_array_2d(),
            sun_color: [colours.sun[0], colours.sun[1], colours.sun[2], sun_strength],
            sky_hi: colours.sky_hi,
            sky_lo: colours.sky_lo,
            zenith: colours.zenith,
            haze: colours.haze,
            ground: [
                colours.ground[0],
                colours.ground[1],
                colours.ground[2],
                ground_z,
            ],
            grid,
            atmos: [sky_on, colours.night, haze_from, haze_from * 5.0 + 60.0],
            lamps,
            lamp_count: [lamp_n as f32, 0.0, 0.0, 0.0],
            caps: match self.caps {
                CapStyle::Layers => [0.0, 0.0, 0.0, 0.0],
                CapStyle::Flat([r, g, b]) => [
                    super::scene::srgb_to_linear(r),
                    super::scene::srgb_to_linear(g),
                    super::scene::srgb_to_linear(b),
                    1.0,
                ],
            },
        };
        queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&globals));
        self.targets(device, size);

        let mut encoder = device.create_command_encoder(&Default::default());
        // 0. The shadow map, only when what casts the shadows changed.
        if let Some(n) = shadow_size {
            let key: ShadowKey = (
                self.uploads,
                (sun * 1000.0).round().as_ivec3().to_array(),
                self.cut,
                u64::from(self.overlay.is_some()),
                n,
                (region.0 * 100.0).round().as_ivec3().to_array(),
                (region.1 * 100.0).round().as_ivec3().to_array(),
            );
            if self.shadow.as_ref().is_none_or(|(m, _)| m.size != n) {
                let map = shadow_map(device, &self.shadow_layout, &self.shadow_sampler, n);
                self.shadow = Some((map, None));
            }
            let (map, drawn) = self.shadow.as_mut().expect("just made");
            if *drawn != Some(key) {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("shadow"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &map.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                let over = self.overlay.as_ref().and_then(|o| o.0.as_ref());
                for (buf, count) in self.opaque.iter().chain(over) {
                    pass.set_pipeline(&self.shadow_pipeline);
                    pass.set_bind_group(0, &self.globals_bind, &[]);
                    pass.set_vertex_buffer(0, buf.slice(..));
                    pass.draw(0..*count, 0..1);
                }
                *drawn = Some(key);
            }
        }
        let shadow_bind = match (&self.shadow, shadow_size) {
            (Some((m, _)), Some(_)) => &m.bind,
            _ => &self.no_shadow.bind,
        };
        let t = self.targets.as_ref().expect("targets exist");
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
                            load: wgpu::LoadOp::Clear(if self.atmosphere.is_some() {
                                // The haze: what the edges of the model
                                // are blended with, as the sky behind.
                                wgpu::Color {
                                    r: colours.haze[0] as f64,
                                    g: colours.haze[1] as f64,
                                    b: colours.haze[2] as f64,
                                    a: 1.0,
                                }
                            } else if self.transparent {
                                wgpu::Color::TRANSPARENT
                            } else {
                                BACKGROUND
                            }),
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
            let over = self.overlay.as_ref().and_then(|o| o.0.as_ref());
            for (buf, n) in self.opaque.iter().chain(over) {
                pass.set_pipeline(&self.model_pipeline);
                pass.set_bind_group(0, &self.globals_bind, &[]);
                pass.set_bind_group(1, shadow_bind, &[]);
                pass.set_bind_group(2, &self.pictures_bind, &[]);
                pass.set_vertex_buffer(0, buf.slice(..));
                pass.draw(0..*n, 0..1);
            }
            if let Some((buf, n)) = &self.grass {
                pass.set_pipeline(&self.grass_pipeline);
                pass.set_bind_group(0, &self.globals_bind, &[]);
                pass.set_bind_group(1, shadow_bind, &[]);
                pass.set_bind_group(2, &self.pictures_bind, &[]);
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
            let over = self.overlay.as_ref().and_then(|o| o.1.as_ref());
            for (buf, n) in self.glass.iter().chain(over) {
                pass.set_pipeline(&self.glass_pipeline);
                pass.set_bind_group(0, &self.globals_bind, &[]);
                pass.set_bind_group(1, shadow_bind, &[]);
                pass.set_bind_group(2, &self.pictures_bind, &[]);
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

/// A vertex buffer of some triangles (none if there are none).
fn upload(device: &wgpu::Device, label: &str, v: &[Vertex]) -> Option<(wgpu::Buffer, u32)> {
    (!v.is_empty()).then(|| {
        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(v),
            usage: wgpu::BufferUsages::VERTEX,
        });
        (buf, v.len() as u32)
    })
}

/// Mip levels of the textures' pictures.
fn picture_levels() -> u32 {
    archisim_render::textures::SIZE.trailing_zeros() + 1
}

/// The texture of the textures' pictures: one layer for each texture
/// with all its levels of detail (`Pictures`), or one pixel.
fn pictures_texture(
    device: &wgpu::Device,
    mode: TextureMode,
) -> (wgpu::Texture, wgpu::TextureView) {
    let full = mode == TextureMode::Pictures;
    let side = if full {
        archisim_render::textures::SIZE as u32
    } else {
        1
    };
    let layers = if full {
        archisim_core::TextureKind::ALL.len() as u32
    } else {
        1
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("texture pictures"),
        size: wgpu::Extent3d {
            width: side,
            height: side,
            depth_or_array_layers: layers,
        },
        mip_level_count: if full { picture_levels() } else { 1 },
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    (texture, view)
}

/// Sends the textures' pictures to the card, each layer with its levels
/// of detail (each a quarter of the one before, its pixels averaged).
fn upload_pictures(queue: &wgpu::Queue, texture: &wgpu::Texture) {
    if texture.size().width <= 1 {
        return;
    }
    for kind in archisim_core::TextureKind::ALL {
        let tile = archisim_render::textures::tile(kind);
        let layer = kind.number() - 1;
        let mut level = tile.rgba.clone();
        let mut n = tile.size;
        for mip in 0..picture_levels() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture,
                    mip_level: mip,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &level,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * n as u32),
                    rows_per_image: Some(n as u32),
                },
                wgpu::Extent3d {
                    width: n as u32,
                    height: n as u32,
                    depth_or_array_layers: 1,
                },
            );
            if n == 1 {
                break;
            }
            let m = n / 2;
            let mut next = vec![0u8; m * m * 4];
            for j in 0..m {
                for i in 0..m {
                    for k in 0..4 {
                        let at = |x: usize, y: usize| level[(y * n + x) * 4 + k] as u32;
                        let sum = at(2 * i, 2 * j)
                            + at(2 * i + 1, 2 * j)
                            + at(2 * i, 2 * j + 1)
                            + at(2 * i + 1, 2 * j + 1);
                        next[(j * m + i) * 4 + k] = ((sum + 2) / 4) as u8;
                    }
                }
            }
            level = next;
            n = m;
        }
    }
}

/// A shadow map `n` pixels a side, cleared (nothing in shadow).
fn shadow_map(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    n: u32,
) -> ShadowMap {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shadow map"),
        size: wgpu::Extent3d {
            width: n,
            height: n,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("shadow map"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    ShadowMap {
        size: n,
        view,
        bind,
    }
}

/// The sun's view of the scene's box, as a parallel projection that just
/// holds it.
pub fn sun_view_proj(sun: glam::Vec3, (lo, hi): (glam::Vec3, glam::Vec3)) -> glam::Mat4 {
    use glam::camera::rh::{proj::directx, view::look_at_mat4};
    let centre = (lo + hi) / 2.0;
    let radius = ((hi - lo).length() / 2.0).max(1.0);
    let up = if sun.z.abs() > 0.99 {
        glam::Vec3::Y
    } else {
        glam::Vec3::Z
    };
    let view = look_at_mat4(centre + sun * radius * 2.0, centre, up);
    let (mut a, mut b) = (glam::Vec3::splat(f32::MAX), glam::Vec3::splat(f32::MIN));
    for i in 0..8 {
        let corner = glam::Vec3::new(
            if i & 1 == 0 { lo.x } else { hi.x },
            if i & 2 == 0 { lo.y } else { hi.y },
            if i & 4 == 0 { lo.z } else { hi.z },
        );
        let q = view.transform_point3(corner);
        a = a.min(q);
        b = b.max(q);
    }
    // In front of the eye the view's z is negative.
    let (near, far) = ((-b.z - 1.0).max(0.01), -a.z + 1.0);
    directx::orthographic(a.x, b.x, a.y, b.y, near, far) * view
}
