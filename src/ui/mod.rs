use std::{cell::RefCell, collections::HashMap, io::BufReader, rc::Rc};

use js_sys::{ArrayBuffer, Uint8Array};
use leptos::*;
use tobj::Material;
use wasm_bindgen::prelude::*;
use web_sys::FileReader;

type RawModel = (Vec<f64>, Vec<usize>);

pub struct Model {
    name: RwSignal<String>,
    data: RwSignal<RawModel>,
}

impl Model {
    fn new(name: String, model: RawModel) -> Self {
        Self {
            name: create_rw_signal(name),
            data: create_rw_signal(model),
        }
    }
}

pub struct Models(Vec<RawModel>);

impl Models {
    fn new() -> Self {
        Self(vec![])
    }
}

#[component]
pub fn Model(model: Model) -> impl IntoView {
    view! {
        <label>
            {move || model.name.get() }
        </label>
    }
}

async fn read_obj_from_file(file: web_sys::File) {
    let buffer = wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await;
    if let Ok(buffer) = buffer {
        let array = Uint8Array::new(&buffer);
        let bytes = array.to_vec();
        let bytes: &[u8] = &bytes;
        let mut reader = BufReader::new(bytes);
        let model = tobj::load_obj_buf(&mut reader, &tobj::LoadOptions::default(), |_| {
            Ok((vec![Material::default()], HashMap::new()))
        });
        log!("the model is {:?}", model);
    } else {
        warn!("failed to get file buffer");
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (models, set_models) = create_signal(Models::new());
    provide_context(set_models);

    let file_input = create_node_ref::<html::Input>();
    let on_change = move |_| {
        if let Some(files) = file_input.get().unwrap().files() {
            for i in 0..files.length() {
                if let Some(file) = files.item(i) {
                    log!("read file {:?}", file.name());
                    spawn_local(async move {
                        read_obj_from_file(file).await;
                    });
                    // let buffer = wasm_bindgen_futures::JsFuture::from(buffer_promise).await;
                    // let reader = Rc::new(RefCell::new(FileReader::new().unwrap()));
                    // let txt: Rc<RefCell<Option<String>>> = Default::default();
                    // let reader_clone = reader.clone();
                    // let txt_clone = txt.clone();
                    // let on_load: Closure<dyn FnMut()> = Closure::new(move || {
                    //     let result = reader_clone.borrow().result().unwrap().as_string();
                    //     *txt_clone.borrow_mut() = result.clone();
                    //     let mut reader = BufReader::new(result.as_ref().unwrap().as_bytes());
                    //     let obj =
                    //         tobj::load_obj_buf(&mut reader, &tobj::LoadOptions::default(), |_| {
                    //             Ok((vec![Material::default()], HashMap::new()))
                    //         });
                    //     log!("read txt: {:?}", obj);
                    // });
                    // reader
                    //     .borrow()
                    //     .set_onload(Some(on_load.as_ref().unchecked_ref()));
                    // reader.borrow().read_as_text(&file).unwrap();
                    // on_load.forget();
                }
            }
        }
    };
    view! {
        <main>
            <label for = "model upload"> "+" </label>
            <li>
            <input
                type="file"
                node_ref=file_input
                on:change = on_change
                name = "model upload"
                id = "model upload"
                accept = ".obj"
                opacity = 0
                multiple
            />
            </li>
            <li> dfdsf </li>
            <li> dfdsf </li>
        </main>
    }
}
