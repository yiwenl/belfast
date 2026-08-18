use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

mod input;

pub use input::InputEvent;

use belfast::Device;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

pub struct ExampleContext {
    pub device: Device,
    pub format: wgpu::TextureFormat,
    pub width: u32,
    pub height: u32,
}

pub trait Example: 'static {
    fn new(context: &ExampleContext) -> Self;

    fn resize(&mut self, _context: &ExampleContext) {}

    fn input(&mut self, _context: &ExampleContext, _event: InputEvent) {}

    fn update(&mut self, _context: &ExampleContext, _delta_seconds: f32) {}

    fn render(&mut self, context: &ExampleContext, surface_view: &wgpu::TextureView);
}

pub fn run<E: Example>(title: &'static str) {
    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = ExampleApplication::<E>::new(title);
    event_loop.run_app(&mut app).expect("run example");
}

pub fn begin_render_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
    clear_color: wgpu::Color,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("ExampleSurfacePass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(clear_color),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}

struct ExampleApplication<E: Example> {
    title: &'static str,
    state: Option<ExampleState<E>>,
}

impl<E: Example> ExampleApplication<E> {
    fn new(title: &'static str) -> Self {
        Self { title, state: None }
    }
}

impl<E: Example> ApplicationHandler for ExampleApplication<E> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            self.state = Some(ExampleState::new(event_loop, self.title));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        if state.window.id() != window_id {
            return;
        }

        if let Some(input_event) = state.input.process(&event) {
            state.example.input(&state.context, input_event);
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size),
            WindowEvent::RedrawRequested => state.render(event_loop),
            _ => {}
        }
    }
}

struct ExampleState<E: Example> {
    window: Arc<Window>,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    context: ExampleContext,
    example: E,
    input: input::InputState,
    last_frame_at: Instant,
    pending_gpu_errors: PendingGpuErrors,
}

#[derive(Clone, Default)]
struct PendingGpuErrors {
    message: Arc<Mutex<Option<String>>>,
}

impl PendingGpuErrors {
    fn record(&self, error: wgpu::Error) {
        let message = match error {
            wgpu::Error::OutOfMemory { .. } => "WebGPU device ran out of memory".to_owned(),
            wgpu::Error::Validation { description, .. } => {
                format!("WebGPU validation error: {description}")
            }
            wgpu::Error::Internal { description, .. } => {
                format!("WebGPU internal error: {description}")
            }
        };
        self.record_message(message);
    }

    fn record_message(&self, message: String) {
        let mut pending = self
            .message
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if pending.is_none() {
            *pending = Some(message);
        }
    }

    fn take(&self) -> Option<String> {
        self.message
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
}

impl<E: Example> ExampleState<E> {
    fn new(event_loop: &ActiveEventLoop, title: &'static str) -> Self {
        let attributes = Window::default_attributes().with_title(title);
        let window = Arc::new(event_loop.create_window(attributes).expect("create window"));
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(event_loop.owned_display_handle()),
        ));
        let surface = instance
            .create_surface(window.clone())
            .expect("create wgpu surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("request surface-compatible adapter");
        let (gpu, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("BelfastExampleDevice"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }))
        .expect("request wgpu device");
        let pending_gpu_errors = PendingGpuErrors::default();
        let uncaptured = pending_gpu_errors.clone();
        gpu.on_uncaptured_error(Arc::new(move |error| uncaptured.record(error)));
        let lost = pending_gpu_errors.clone();
        gpu.set_device_lost_callback(move |reason, message| {
            lost.record_message(format!("WebGPU device lost ({reason:?}): {message}"));
        });

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(capabilities.formats[0]);
        let width = size.width.max(1);
        let height = size.height.max(1);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&gpu, &config);

        let context = ExampleContext {
            device: Device::from_wgpu(gpu, queue, format),
            format,
            width,
            height,
        };
        let example = E::new(&context);
        window.request_redraw();

        Self {
            window,
            instance,
            surface,
            config,
            context,
            example,
            input: Default::default(),
            last_frame_at: Instant::now(),
            pending_gpu_errors,
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.context.width = size.width;
        self.context.height = size.height;
        self.surface
            .configure(self.context.device.gpu(), &self.config);
        self.example.resize(&self.context);
    }

    fn render(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(error) = self.pending_gpu_errors.take() {
            eprintln!("{error}");
            event_loop.exit();
            return;
        }

        let now = Instant::now();
        let delta_seconds = (now - self.last_frame_at).as_secs_f32().min(0.1);
        self.last_frame_at = now;

        let (frame, reconfigure_after_present) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => (frame, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => (frame, true),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface
                    .configure(self.context.device.gpu(), &self.config);
                self.last_frame_at = Instant::now();
                self.window.request_redraw();
                return;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                self.window.request_redraw();
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                if let Err(error) = self.recreate_surface() {
                    eprintln!("{error}");
                    event_loop.exit();
                    return;
                }
                self.last_frame_at = Instant::now();
                self.window.request_redraw();
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                let error = self
                    .pending_gpu_errors
                    .take()
                    .unwrap_or_else(|| "WebGPU surface validation failed".into());
                eprintln!("{error}");
                event_loop.exit();
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.example.update(&self.context, delta_seconds);
        self.example.render(&self.context, &view);
        frame.present();
        if reconfigure_after_present {
            self.surface
                .configure(self.context.device.gpu(), &self.config);
        }
        if let Some(error) = self.pending_gpu_errors.take() {
            eprintln!("{error}");
            event_loop.exit();
            return;
        }
        self.window.request_redraw();
    }

    fn recreate_surface(&mut self) -> Result<(), String> {
        let surface = self
            .instance
            .create_surface(self.window.clone())
            .map_err(|error| format!("failed to recreate example surface: {error}"))?;
        surface.configure(self.context.device.gpu(), &self.config);
        self.surface = surface;
        Ok(())
    }
}
