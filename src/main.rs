use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use anyhow::Result;
use leptos::html::Canvas;
use leptos::prelude::*;
use leptos::mount::mount_to_body;
use leptos::task::spawn_local;
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

struct WinitApp {
    window: Option<Window>,
    canvas: HtmlCanvasElement,
    viewer: Rc<RefCell<Viewer>>,
}

impl WinitApp {
    fn new(canvas: HtmlCanvasElement, viewer: Rc<RefCell<Viewer>>) -> Self {
        Self {
            window: None,
            canvas,
            viewer,
        }
    }
}

impl ApplicationHandler for WinitApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(
            event_loop
                .create_window(Window::default_attributes().with_canvas(Some(self.canvas.clone())))
                .expect("create window failed"),
        );
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
                    let canvas = self.canvas.clone();
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
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    let canvas: NodeRef<Canvas> = NodeRef::new();
    let render: Rc<RefCell<Option<Renderer>>> = Default::default();
    let viewer = Rc::new(RefCell::new(viewer::Viewer::new(render.clone())));
    let viewer_wrapper: ViewerWrapper = SendWrapper::new(viewer.clone());

    let (tx, rx) = mpsc::channel();
    {
        let canvas_for_closure = canvas.clone();
        Effect::new(move |_| {
            if let Some(c) = canvas_for_closure.get() {
                let canvas_el: web_sys::HtmlCanvasElement = c.into();
                web_sys::console::log_1(&"send canvas".into());
                let _ = tx.send(canvas_el);
            }
        });

        let viewer_wrapper = viewer_wrapper.clone();
        mount_to_body(move || view! { <view::App canvas viewer = viewer_wrapper.clone() />});
        web_sys::console::log_1(&"mount".into());
    }

    let event_loop = event_loop::EventLoop::new()?;
    let canvas = rx.recv()?;
    event_loop.set_control_flow(event_loop::ControlFlow::Wait);

    let mut app = WinitApp::new(canvas, viewer);
    event_loop.run_app(&mut app)?;

    Result::Ok(())
}
