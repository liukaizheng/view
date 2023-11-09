use std::ops::Deref;
use std::sync::mpsc;

use anyhow::Result;
use leptos::html::Canvas;
use leptos::*;
use view::App;
use winit::event::WindowEvent;
use winit::platform::web::WindowBuilderExtWebSys;
use winit::{event, event_loop};

fn main() -> Result<()> {
    let cs: NodeRef<Canvas> = create_node_ref();

    let (tx, rx) = mpsc::channel();
    cs.on_load(move |c: HtmlElement<Canvas>| {
        tx.send(c.clone()).unwrap();
    });
    leptos::mount_to_body(move || view! { <App canvas = cs/>});

    let event_loop = event_loop::EventLoop::new()?;
    let canvas = rx.recv()?;
    let _window = winit::window::WindowBuilder::new()
        .with_canvas(Some(canvas.deref().clone()))
        .build(&event_loop)?;

    event_loop.run(move |event, elwt| match event {
        event::Event::WindowEvent { event, .. } => match event {
            WindowEvent::KeyboardInput { event, .. } => {
                leptos::logging::log!("{:?}", event);
            }
            _ => {}
        },
        _ => {}
    })?;

    Result::Ok(())
}
