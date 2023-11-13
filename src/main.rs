use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::mpsc;

use anyhow::Result;
use leptos::html::Canvas;
use leptos::*;
use view::render::render::Renderer;
use view::render::viewer;
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
    let _window = winit::window::WindowBuilder::new()
        .with_canvas(Some(canvas.deref().clone()))
        .build(&event_loop)?;

    event_loop.run(move |event, _elwt| match event {
        event::Event::WindowEvent { event, .. } => match event {
            WindowEvent::KeyboardInput { event, .. } => {
                leptos::logging::log!("{:?}", event);
            }
            WindowEvent::RedrawRequested => {
                if let Err(msg) = viewer.borrow_mut().render() {
                    logging::error!("faild to render because {:?}", msg);
                } else {
                    logging::log!("success");
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
