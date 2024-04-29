use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::mpsc;

use anyhow::Result;
use leptos::html::Canvas;
use leptos::*;
use view::render::render::Renderer;
use view::render::viewer::{self, MousePressed, Viewer};
use view::App;
use web_sys::HtmlCanvasElement;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::platform::web::WindowAttributesExtWebSys;
use winit::window::Window;
use winit::{event, event_loop};

extern crate console_error_panic_hook;
use std::panic;

struct App {
    window: Option<Window>,
    canvas: HtmlCanvasElement,
    viewer: Rc<RefCell<Viewer>>,
}

impl App {
    fn new(canvas: HtmlCanvasElement, viewer: Rc<RefCell<Viewer>>) -> Self {
        Self {
            window: None,
            canvas,
            viewer,
        }
    }
}

impl ApplicationHandler for App {
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
                leptos::logging::log!("{:?}", event);
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
                    logging::error!("faild to render because {:?}", msg);
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
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<()> {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    let canvas: NodeRef<Canvas> = create_node_ref();
    let render: Rc<RefCell<Option<Renderer>>> = Default::default();
    let viewer = Rc::new(RefCell::new(viewer::Viewer::new(render.clone())));

    let (tx, rx) = mpsc::channel();
    {
        let render = render.clone();
        canvas.on_load(move |canvas: HtmlElement<Canvas>| {
            let c = canvas.clone();
            spawn_local(async move {
                let canvas = canvas.deref();
                match Renderer::new(canvas.clone()).await {
                    Ok(r) => {
                        render.borrow_mut().replace(r);
                    }
                    Err(e) => {
                        logging::error!("create viewer failed by {:?}", e);
                    }
                }
            });
            tx.send(c.clone()).unwrap();
        });

        let viewer = viewer.clone();
        leptos::mount_to_body(move || view! { <App canvas viewer = viewer.clone() />});
    }

    let event_loop = event_loop::EventLoop::new()?;
    let canvas = rx.recv()?;
    event_loop.set_control_flow(event_loop::ControlFlow::Wait);

    let mut app = App::new(canvas.deref().clone(), viewer.clone());
    event_loop.run_app(&mut app)?;

    Result::Ok(())
}
