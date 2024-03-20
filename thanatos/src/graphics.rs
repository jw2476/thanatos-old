use std::borrow::Cow;
use std::sync::Arc;
use winit::platform::run_on_demand::EventLoopExtRunOnDemand;
use tokio::sync::broadcast::{Sender, Receiver};
use crate::event::Event;

pub struct Window {
    event_loop: winit::event_loop::EventLoop<()>,
    window: Arc<winit::window::Window>,
    tx: Sender<Event>
}

impl Window {
    pub fn new(tx: Sender<Event>) -> Self {
        let event_loop = winit::event_loop::EventLoop::new().unwrap();
        let window = winit::window::WindowBuilder::new()
            .build(&event_loop)
            .unwrap();
        let window = Arc::new(window);
        Self { event_loop, window, tx }
    }

    pub fn tick(&mut self) -> bool {
        let mut should_close = false;

        self.event_loop
            .run_on_demand(|event, control| {
                control.exit();

                if let winit::event::Event::WindowEvent {
                    window_id: _,
                    event,
                } = event
                {
                    match event {
                        winit::event::WindowEvent::Resized(new_size) => {
                            self.tx.send(Event::Resized(new_size)).unwrap();
                        }
                        winit::event::WindowEvent::CloseRequested => {
                            should_close = true;
                        }
                        _ => (),
                    }
                }
            })
            .unwrap();


        should_close
    }
}

pub struct Graphics<'a> {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    shader: wgpu::ShaderModule,
    pipeline_layout: wgpu::PipelineLayout,
    render_pipeline: wgpu::RenderPipeline,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    rx: Receiver<Event>
}

impl<'a> Graphics<'a> {
    fn get_size(window: &winit::window::Window) -> winit::dpi::PhysicalSize<u32> {
        let mut size = window.inner_size();
        size.width = size.width.max(1);
        size.height = size.height.max(1);
        size
    }

    pub async fn new(window: &Window, rx: Receiver<Event>) -> Self {
        let size = Self::get_size(&window.window);

        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_limits: wgpu::Limits::downlevel_defaults()
                        .using_resolution(adapter.limits()),
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!(
                "../assets/shaders/shader.wgsl"
            ))),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let swapchain_format = swapchain_capabilities.formats[0];

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(swapchain_format.into())],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let config = surface
            .get_default_config(&adapter, size.width, size.height)
            .unwrap();
        surface.configure(&device, &config);

        Self {
            instance,
            adapter,
            surface,
            device,
            pipeline_layout,
            queue,
            render_pipeline,
            shader,
            config,
            size,
            rx
        }
    }

    pub async fn draw(&mut self) {
        while !self.rx.is_empty() {
            match self.rx.recv().await.unwrap() {
                Event::Resized(new_size) => {
                    self.config.width = new_size.width.max(1);
                    self.config.height = new_size.height.max(1);
                    self.surface.configure(&self.device, &self.config);
                }
            }
        }

        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
}
