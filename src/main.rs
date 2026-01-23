use std::cell::RefCell;
use std::rc::Rc;
use anyhow::Result;
use leptos::html::Canvas;
use leptos::prelude::*;
use leptos::mount::mount_to_body;
use wasm_bindgen_futures::spawn_local;
use send_wrapper::SendWrapper;
use view::render::render::Renderer;
use view::render::viewer::{self, MousePressed, Viewer};
use view::ViewerWrapper;
use web_sys::HtmlCanvasElement;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::platform::web::WindowAttributesExtWebSys;
use winit::window::Window;
use winit::{event, event_loop};

extern crate console_error_panic_hook;
use std::panic;

#[derive(Debug)]
enum AppEvent {
    CanvasReady(HtmlCanvasElement),
}

struct WinitApp {
    window: Option<Window>,
    canvas: Option<HtmlCanvasElement>,
    viewer: Rc<RefCell<Viewer>>,
}

impl WinitApp {
    fn new(viewer: Rc<RefCell<Viewer>>) -> Self {
        Self {
            window: None,
            canvas: None,
            viewer,
        }
    }
}

impl ApplicationHandler<AppEvent> for WinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(canvas) = &self.canvas {
            if self.window.is_none() {
                web_sys::console::log_1(&"Creating window with ready canvas".into());
                self.window = Some(
                    event_loop
                        .create_window(Window::default_attributes().with_canvas(Some(canvas.clone())))
                        .expect("create window failed"),
                );
            }
        } else {
             web_sys::console::log_1(&"Resumed called but canvas not ready yet".into());
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::CanvasReady(canvas) => {
                web_sys::console::log_1(&"Received CanvasReady event".into());
                self.canvas = Some(canvas);
                // Trigger window creation if we're already resumed (mostly likely yes)
                if self.window.is_none() {
                     web_sys::console::log_1(&"Creating window from UserEvent".into());
                     self.window = Some(
                        event_loop
                            .create_window(Window::default_attributes().with_canvas(self.canvas.clone()))
                            .expect("create window failed"),
                    );
                }
            }
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                web_sys::console::log_1(&format!("{:?}", event).into());
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.viewer.borrow_mut().mouse_move(position);
            }

            WindowEvent::MouseInput { state, button, .. } => match state {
                event::ElementState::Pressed => match button {
                    event::MouseButton::Left => {
                        self.viewer.borrow_mut().pressed_state = MousePressed::Left(None);
                    }
                    _ => {}
                },
                event::ElementState::Released => {
                    self.viewer.borrow_mut().pressed_state = MousePressed::None;
                }
            },

            WindowEvent::MouseWheel { delta, .. } => match delta {
                event::MouseScrollDelta::PixelDelta(size) => {
                    self.viewer.borrow_mut().mouse_scroll(size.y);
                }
                _ => {}
            },

            WindowEvent::RedrawRequested => {
                if let Err(msg) = self.viewer.borrow_mut().render() {
                    web_sys::console::error_1(&format!("failed to render because {:?}", msg).into());
                } else {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
            }

            WindowEvent::Resized(size) => {
                let viewer = self.viewer.borrow();
                let mut render = viewer.render.borrow_mut();
                if let Some(render) = render.as_mut() {
                    render.resize(size.width, size.height);
                } else {
                    if let Some(canvas) = &self.canvas {
                        let canvas = canvas.clone();
                        let viewer = self.viewer.clone();
                         spawn_local(async move {
                            match Renderer::new(canvas.clone(), size.width, size.height).await {
                                Ok(r) => {
                                    viewer.borrow().render.borrow_mut().replace(r);
                                }
                                Err(e) => {
                                    web_sys::console::error_1(&format!("create viewer failed by {:?}", e).into());
                                }
                            }
                        })
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    web_sys::console::log_1(&"Initializing application...".into());

    let render: Rc<RefCell<Option<Renderer>>> = Default::default();
    let viewer = Rc::new(RefCell::new(viewer::Viewer::new(render.clone())));
    let viewer_wrapper: ViewerWrapper = SendWrapper::new(viewer.clone());
    
    let event_loop = event_loop::EventLoop::<AppEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();

    let canvas_ref: NodeRef<Canvas> = NodeRef::new();
    
    mount_to_body(move || {
        let canvas_ref = canvas_ref.clone();
        let proxy = proxy.clone();
        
        Effect::new(move |_| {
            if let Some(c) = canvas_ref.get() {
                 let canvas_el: HtmlCanvasElement = c.into();
                 web_sys::console::log_1(&"Canvas found in Effect, sending to proxy".into());
                 let _ = proxy.send_event(AppEvent::CanvasReady(canvas_el));
            }
        });
        
        view! { <view::App canvas=canvas_ref viewer=viewer_wrapper.clone() /> } 
    });
    web_sys::console::log_1(&"Leptos app mounted".into());

    event_loop.set_control_flow(event_loop::ControlFlow::Wait);

    let mut app = WinitApp::new(viewer);
    web_sys::console::log_1(&"Starting event loop".into());
    event_loop.run_app(&mut app)?;

    Result::Ok(())
}
