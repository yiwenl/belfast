use std::sync::{Arc, Mutex};

#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};

mod input;

pub use input::InputEvent;
pub use web_time::Instant;

use belfast::{pick_surface_color, Device};
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
    #[allow(dead_code)]
    pub color_space: wgpu::SurfaceColorSpace,
    #[allow(dead_code)]
    pub hdr: bool,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ExampleRunOptions {
    pub hdr: bool,
}

pub trait Example: 'static {
    fn new(context: &ExampleContext) -> Self;

    fn resize(&mut self, _context: &ExampleContext) {}

    fn input(&mut self, _context: &ExampleContext, _event: InputEvent) {}

    fn update(&mut self, _context: &ExampleContext, _delta_seconds: f32) {}

    fn render(&mut self, context: &ExampleContext, surface_view: &wgpu::TextureView);
}

#[allow(dead_code)]
pub fn run<E: Example>(title: &'static str) {
    run_with::<E>(title, ExampleRunOptions { hdr: false });
}

#[allow(dead_code)]
pub fn run_with<E: Example>(title: &'static str, options: ExampleRunOptions) {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    let app = ExampleApplication::<E>::new(title, options);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = app;
        event_loop.run_app(&mut app).expect("run example");
    }
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app);
    }
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

fn log_error(message: &str) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(message));
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("{message}");
}

fn log_info(message: &str) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(message));
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("{message}");
}

fn create_window(event_loop: &ActiveEventLoop, title: &'static str) -> Arc<Window> {
    #[cfg(target_arch = "wasm32")]
    let attributes = {
        use winit::dpi::LogicalSize;
        use winit::platform::web::WindowAttributesExtWebSys;
        let mut attributes = Window::default_attributes()
            .with_title(title)
            .with_append(true)
            .with_prevent_default(false);
        if let Some(web_window) = web_sys::window() {
            let width = web_window
                .inner_width()
                .ok()
                .and_then(|value| value.as_f64())
                .unwrap_or(1.0);
            let height = web_window
                .inner_height()
                .ok()
                .and_then(|value| value.as_f64())
                .unwrap_or(1.0);
            attributes = attributes.with_inner_size(LogicalSize::new(width, height));
        }
        attributes
    };
    #[cfg(not(target_arch = "wasm32"))]
    let attributes = Window::default_attributes().with_title(title);
    Arc::new(event_loop.create_window(attributes).expect("create window"))
}

struct ExampleApplication<E: Example> {
    title: &'static str,
    options: ExampleRunOptions,
    state: Option<ExampleState<E>>,
    #[cfg(target_arch = "wasm32")]
    pending: Rc<RefCell<Option<ExampleState<E>>>>,
    #[cfg(target_arch = "wasm32")]
    started: bool,
}

impl<E: Example> ExampleApplication<E> {
    fn new(title: &'static str, options: ExampleRunOptions) -> Self {
        Self {
            title,
            options,
            state: None,
            #[cfg(target_arch = "wasm32")]
            pending: Rc::new(RefCell::new(None)),
            #[cfg(target_arch = "wasm32")]
            started: false,
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn take_pending_state(&mut self) {
        if self.state.is_none() {
            if let Some(mut state) = self.pending.borrow_mut().take() {
                let size = state.window.inner_size();
                state.resize(size);
                self.state = Some(state);
            }
        }
    }
}

impl<E: Example> ApplicationHandler for ExampleApplication<E> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        #[cfg(target_arch = "wasm32")]
        {
            if self.started {
                return;
            }
            self.started = true;
        }

        let window = create_window(event_loop, self.title);
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(event_loop.owned_display_handle()),
        ));
        let surface = instance
            .create_surface(window.clone())
            .expect("create wgpu surface");

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.state = Some(pollster::block_on(ExampleState::new(
                window,
                instance,
                surface,
                self.title,
                self.options,
            )));
        }
        #[cfg(target_arch = "wasm32")]
        {
            let pending = Rc::clone(&self.pending);
            let title = self.title;
            let options = self.options;
            wasm_bindgen_futures::spawn_local(async move {
                let state = ExampleState::new(window, instance, surface, title, options).await;
                let window = state.window.clone();
                *pending.borrow_mut() = Some(state);
                window.request_redraw();
            });
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        #[cfg(target_arch = "wasm32")]
        self.take_pending_state();
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

    #[cfg(target_arch = "wasm32")]
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.take_pending_state();
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
    async fn new(
        window: Arc<Window>,
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        title: &'static str,
        options: ExampleRunOptions,
    ) -> Self {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
            .expect("request surface-compatible adapter");
        let required_limits = if cfg!(target_arch = "wasm32") {
            wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
        } else {
            wgpu::Limits::default()
        };
        let (gpu, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("BelfastExampleDevice"),
                required_features: wgpu::Features::empty(),
                required_limits,
                ..Default::default()
            })
            .await
            .expect("request wgpu device");
        let pending_gpu_errors = PendingGpuErrors::default();
        let uncaptured = pending_gpu_errors.clone();
        gpu.on_uncaptured_error(Arc::new(move |error| uncaptured.record(error)));
        let lost = pending_gpu_errors.clone();
        gpu.set_device_lost_callback(move |reason, message| {
            lost.record_message(format!("WebGPU device lost ({reason:?}): {message}"));
        });

        let capabilities = surface.get_capabilities(&adapter);
        let choice = pick_surface_color(&capabilities, options.hdr);
        let format = choice.format;
        let hdr_info = surface.display_hdr_info(&adapter);
        log_info(&format!(
            "{title}: format={format:?} color_space={:?} hdr={} headroom={:?}",
            choice.color_space,
            choice.hdr,
            hdr_info.tone_map_headroom()
        ));
        window.set_title(&format!(
            "{title} [{format:?} {:?} hdr={}]",
            choice.color_space, choice.hdr
        ));
        // After await: winit-web starts at 0×0 and only updates via ResizeObserver.
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            color_space: choice.color_space,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&gpu, &config);

        let context = ExampleContext {
            device: Device::from_wgpu(gpu, queue, format, Some(choice.hdr)),
            format,
            color_space: choice.color_space,
            hdr: choice.hdr,
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
            log_error(&error);
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
                    log_error(&error);
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
                log_error(&error);
                event_loop.exit();
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.example.update(&self.context, delta_seconds);
        self.example.render(&self.context, &view);
        self.context.device.queue().present(frame);
        if reconfigure_after_present {
            self.surface
                .configure(self.context.device.gpu(), &self.config);
        }
        if let Some(error) = self.pending_gpu_errors.take() {
            log_error(&error);
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
