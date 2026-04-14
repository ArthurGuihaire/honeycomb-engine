use crate::{
    object::{ColoredObject, GPUTransform},
    scene::Scene,
    utils::SurfaceError,
    vertex::Vertex,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use std::{process::exit, sync::Arc};
use winit::{
    event_loop::{ActiveEventLoop, EventLoop},
    window::Window,
};

mod buffer;
pub mod object;
mod scene;
pub mod utils;
pub mod vertex;

pub fn create_event_loop() -> EventLoop<()> {
    EventLoop::new().unwrap()
}

struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

pub struct Renderer {
    pub window: Arc<Window>,
    pub is_surface_configured: bool,
    gpu: Arc<GpuContext>,
    config: wgpu::SurfaceConfiguration,
    basic_render_pipeline: wgpu::RenderPipeline,

    texture_render_pipeline: wgpu::RenderPipeline,
    texture_bind_group_layout: wgpu::BindGroupLayout,

    scenes: Vec<Scene>,
    active_scene: Option<usize>,
    surface: wgpu::Surface<'static>,

    asset_root: PathBuf,
}

impl Renderer {
    pub async fn new(event_loop: &ActiveEventLoop) -> Self {
        //Window creation
        let window_attributes = Window::default_attributes();
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        window.set_visible(true);
        window.request_redraw();

        //Instance
        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).unwrap();
        let eventual_adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        });

        let adapter = eventual_adapter.await.unwrap();

        //Device and queue creation
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        println!("Chosen format: {:?}", surface_format);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        //Basic shader initialization
        let basic_shader =
            device.create_shader_module(wgpu::include_wgsl!("../shaders/basic.wgsl"));
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Basic render pipeline layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        //Shared states between render pipelines
        let primitive_state = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        };
        let multisample_state = wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        };
        let basic_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("basic pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &basic_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc(), GPUTransform::desc()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &basic_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: primitive_state,
                depth_stencil: None,
                multisample: multisample_state,
                multiview: None,
                cache: None,
            });

        let texture_shader =
            device.create_shader_module(wgpu::include_wgsl!("../shaders/texture.wgsl"));
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture bind group layout"),
                //Slot 0 is texture, slot 1 is sampler
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
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

        let texture_render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("texture pipeline"),
                bind_group_layouts: &[&texture_bind_group_layout],
                push_constant_ranges: &[],
            });

        let texture_render_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("texture pipeline"),
                layout: Some(&texture_render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &texture_shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc(), GPUTransform::desc()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &texture_shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: primitive_state,
                depth_stencil: None,
                multisample: multisample_state,
                multiview: None,
                cache: None,
            });

        let gpu = Arc::new(GpuContext { device, queue });

        //really annoying path parsing stuff
        let asset_root = {
            if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
                //Running from cargo crate so use cargo path
                let mut path = PathBuf::from(manifest_dir);
                path.push("resources");
                path
            } else {
                //running from executable so get root manually
                let mut exe_path = std::env::current_exe().expect("Error: failed to get exe path");
                exe_path.pop();
                exe_path.push("resources");
                exe_path
            }
        };

        Self {
            surface,
            gpu,
            config,
            is_surface_configured: false,
            window,
            basic_render_pipeline,
            texture_bind_group_layout,
            texture_render_pipeline,
            scenes: Vec::new(),
            active_scene: None,
            asset_root,
        }
    }

    pub fn create_scene(&mut self) -> usize {
        self.scenes.push(Scene::new(self.gpu.clone()));
        self.active_scene = Some(self.scenes.len() - 1);
        self.scenes.len() - 1
    }

    pub fn add_static_object(
        &mut self,
        scene: usize,
        vertices: &[Vertex],
        indices: &[u16],
    ) -> ColoredObject {
        let scene_obj = &mut self.scenes[scene];
        scene_obj.add_static_object(vertices, indices)
    }

    //use weird generic to allow passing by string or by std::path::Path
    pub fn add_scene_material<T: AsRef<std::path::Path>>(
        &mut self,
        scene: usize,
        texture_image_path: T,
    ) {
        let path = self.asset_root.join(texture_image_path);
        println!("{}", path.display());
        self.scenes[scene].add_material(&path.to_string_lossy(), &self.texture_bind_group_layout);
    }

    pub fn render(&self) -> Result<(), utils::SurfaceError> {
        self.window.request_redraw();
        if !self.is_surface_configured {
            return Ok(());
        };

        let output_maybe = self.surface.get_current_texture();
        let output = match output_maybe {
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                return Err(SurfaceError::Outdated);
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                return Err(SurfaceError::OutOfMemory);
            }
            Err(e) => {
                return Err(SurfaceError::Other(e));
            }
            Ok(texture) => texture,
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("basic render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&self.basic_render_pipeline);
            match self.active_scene {
                None => println!("Warning: no scene selected"),
                Some(scene_index) => {
                    let scene = &self.scenes[scene_index];
                    scene.render(&mut render_pass);
                    render_pass.draw_indexed(0..scene.indices.len() as u32, 0, 0..1);
                }
            }
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        self.window.request_redraw();

        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.gpu.device, &self.config);
        self.is_surface_configured = true;
    }
}
