use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::mpsc;

use anyhow::Result;
use leptos::html::Canvas;
use leptos::*;
use view::render::render::Renderer;
use view::render::viewer::{self, MousePressed};
use view::App;
use winit::event::WindowEvent;
use winit::platform::web::WindowBuilderExtWebSys;
use winit::{event, event_loop};

fn main() -> Result<()> {
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
                        logging::error!("{:?}", e);
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
    let window = winit::window::WindowBuilder::new()
        .with_canvas(Some(canvas.deref().clone()))
        .build(&event_loop)?;

    event_loop.run(move |event, _elwt| match event {
        event::Event::WindowEvent { event, .. } => match event {
            WindowEvent::KeyboardInput { event, .. } => {
                leptos::logging::log!("{:?}", event);
            }

            WindowEvent::CursorMoved { device_id: _, position } => {
                viewer.borrow_mut().mouse_move(position);
            }

            WindowEvent::MouseInput { device_id: _, state, button } => {
                match state  {
                    event::ElementState::Pressed => {
                        match button {
                            event::MouseButton::Left => {
                                viewer.borrow_mut().pressed_state = MousePressed::Left(None);
                            }
                            _ => {}
                        }

                    }
                    event::ElementState::Released => {
                        viewer.borrow_mut().pressed_state = MousePressed::None;
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                if let Err(msg) = viewer.borrow_mut().render() {
                    logging::error!("faild to render because {:?}", msg);
                } else {
                    logging::log!("success");
                    window.request_redraw();
                }
            }

            WindowEvent::Resized(size) => {
                let viewer = viewer.borrow();
                let mut render = viewer.render.borrow_mut();
                render.as_mut().unwrap().resize(size.width, size.height);
            }
            _ => {}
        },
        _ => {}
    })?;

    Result::Ok(())
}
