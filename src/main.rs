use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::mpsc;

use anyhow::Result;
use leptos::html::Canvas;
use leptos::*;
use view::render::render::Renderer;
use view::App;
use winit::event::WindowEvent;
use winit::platform::web::WindowBuilderExtWebSys;
use winit::{event, event_loop};

fn main() -> Result<()> {
    let cs: NodeRef<Canvas> = create_node_ref();
    let render: Rc<RefCell<Option<Renderer>>> = Default::default();

    let (tx, rx) = mpsc::channel();
    {
        let render = render.clone();
        cs.on_load(move |canvas: HtmlElement<Canvas>| {
            let c = canvas.clone();
            spawn_local(async move {
                let canvas = canvas.deref();
                let local_render = Renderer::new(canvas.clone()).await;
                *render.borrow_mut() = Some(local_render);
            });
            tx.send(c.clone()).unwrap();
        });
    }
    leptos::mount_to_body(move || view! { <App canvas = cs/>});

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
            WindowEvent::RedrawRequested => {
                logging::log!("here");
                render.borrow().as_ref().unwrap().render();
                window.request_redraw();
            }
            _ => {}
        },
        _ => {}
    })?;

    Result::Ok(())
}
