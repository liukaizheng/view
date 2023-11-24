use std::{cell::RefCell, collections::HashMap, io::BufReader, rc::Rc};

use js_sys::Uint8Array;
use leptos::*;
use tobj::Material;

use crate::render::viewer::Viewer;

pub mod render;

type RawModel = (Vec<f64>, Vec<usize>);

#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    id: u32,
    name: RwSignal<String>,
    data: RwSignal<RawModel>,
    show: RwSignal<bool>,
}

impl Model {
    fn new(name: String, model: RawModel, id: u32) -> Self {
        Self {
            id,
            name: create_rw_signal(name),
            data: create_rw_signal(model),
            show: create_rw_signal(true),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Models(Vec<Model>);

impl Models {
    fn new() -> Self {
        Self(vec![])
    }

    fn add(&mut self, model: Model) {
        self.0.push(model);
    }

    fn remove(&mut self, id: u32) {
        self.0.retain(|m| m.id != id);
    }

    fn hide(&mut self) {
        for m in &mut self.0 {
            let show = m.show.clone();
            if show.get() {
                show.update(|s| *s = false);
            }
        }
    }
}

async fn read_obj_from_file(file: web_sys::File) -> Result<RawModel, String> {
    let buffer = wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await;
    if let Ok(buffer) = buffer {
        let array = Uint8Array::new(&buffer);
        let bytes = array.to_vec();
        let bytes: &[u8] = &bytes;
        let mut reader = BufReader::new(bytes);
        if let Ok(models) = tobj::load_obj_buf(&mut reader, &tobj::LoadOptions::default(), |_| {
            Ok((vec![Material::default()], HashMap::new()))
        }) {
            let mesh = &models.0[0].mesh;
            return Ok((
                mesh.positions.clone(),
                Vec::from_iter(mesh.indices.iter().map(|idx| *idx as usize)),
            ));
        }
    } else {
        leptos::logging::warn!("failed to read file buffer");
    }
    Err("failed to read obj".to_owned())
}

fn write_obj(points: &[f64], triangles: &[usize]) -> String {
    let mut txt = "".to_owned();
    for p in points.chunks(3) {
        txt.push_str(&format!("v {} {} {}\n", p[0], p[1], p[2]));
    }
    for tri in triangles.chunks(3) {
        txt.push_str(&format!("f {} {} {}\n", tri[0] + 1, tri[1] + 1, tri[2] + 1));
    }
    txt
}

#[component]
pub fn Model(model: Model, viewer: Rc<RefCell<Viewer>>) -> impl IntoView {
    {
        let viewer = viewer.clone();
        create_effect(move |_| {
            viewer.borrow_mut().set_visible(model.id, model.show.get());
        });
    }

    let write_to_local = move |_| {
        let (points, triangles) = model.data.get();
        let txt = write_obj(&points, &triangles);
        let parts = js_sys::Array::of1(&unsafe { Uint8Array::view(txt.as_bytes()).into() });
        let mut properties = web_sys::BlobPropertyBag::new();
        properties.type_("model/obj");
        if let Ok(blob) =
            web_sys::Blob::new_with_buffer_source_sequence_and_options(&parts, &properties)
        {
            let link = html::a();
            let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
            link.set_href(&url);
            link.set_download(&(model.name.get() + ".obj"));
            link.click();
            web_sys::Url::revoke_object_url(&link.href()).unwrap();
        }
    };

    let set_models = use_context::<WriteSignal<Models>>().unwrap();
    let destroy = move |_| {
        set_models.update(|models| {
            let id = model.id;
            models.remove(model.id);
            viewer.borrow_mut().remove_data(id);
        });
    };
    let toggle_show = move |_| {
        model.show.update(|show| *show = !*show);
    };
    const SHOW_CLASS_ATTR: &str = "w-full";
    const HIDE_CLASS_ATTR: &str = "w-full text-gray-400";
    view! {
          <li class = "group/li w-full p-2 hover:bg-emerald-100">
              <div class = "flex flex-1 items-center justify-center">
                  <label class = move || if model.show.get() { SHOW_CLASS_ATTR} else {HIDE_CLASS_ATTR}>
                      {move || model.name.get() }
                  </label>
                  <div class = "flex">
                  <button on:click = toggle_show class = "group/button w-6 h-6 hover:bg-emerald-200 rounded-full items-center justify-center hidden group-hover/li:flex mr-1">
                      { move || {
                          if model.show.get() {
                              view! { <svg class= "w-4 h-4 stroke-2 stroke-emerald-900" aria-hidden="true" fill="none" viewBox="0 0 20 14"> <g> <path d="M10 10a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z"/> <path d="M10 13c4.97 0 9-2.686 9-6s-4.03-6-9-6-9 2.686-9 6 4.03 6 9 6Z"/> </g> </svg>}
                          } else {
                              view! {<svg class= "w-4 h-4 stroke-2 stroke-emerald-900" aria-hidden="true" fill="none" viewBox="0 0 20 18"><path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" d="M1.933 10.909A4.357 4.357 0 0 1 1 9c0-1 4-6 9-6m7.6 3.8A5.068 5.068 0 0 1 19 9c0 1-3 6-9 6-.314 0-.62-.014-.918-.04M2 17 18 1m-5 8a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z"/>
    </svg>}
                          }
                      }}
                  </button>
                  <button on:click = write_to_local class = "group/button w-6 h-6 hover:bg-emerald-200 rounded-full items-center justify-center hidden group-hover/li:flex mr-1">
                      <svg viewBox="0 0 24 24" class = "w-4 h-4 fill-emerald-900"><path d="M18.948 11.112C18.511 7.67 15.563 5 12.004 5c-2.756 0-5.15 1.611-6.243 4.15-2.148.642-3.757 2.67-3.757 4.85 0 2.757 2.243 5 5 5h1v-2h-1c-1.654 0-3-1.346-3-3 0-1.404 1.199-2.757 2.673-3.016l.581-.102.192-.558C8.153 8.273 9.898 7 12.004 7c2.757 0 5 2.243 5 5v1h1c1.103 0 2 .897 2 2s-.897 2-2 2h-2v2h2c2.206 0 4-1.794 4-4a4.008 4.008 0 0 0-3.056-3.888z"></path><path d="M13.004 14v-4h-2v4h-3l4 5 4-5z"></path></svg>
                  </button>
                  <button on:click = destroy class = "group/button w-6 h-6 hover:bg-emerald-200 rounded-full items-center justify-center hidden group-hover/li:flex">
                      <svg viewBox="0 0 24 24" stroke-linecap="round" class = "w-4 h-4 stroke-2 stroke-emerald-900"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                  </button>
                  </div>
              </div>
          </li>
      }
}

#[component]
pub fn ModelList(
    models: ReadSignal<Models>,
    set_models: WriteSignal<Models>,
    viewer: Rc<RefCell<Viewer>>,
) -> impl IntoView {
    let (fix, set_fix) = create_signal(false);
    let fix_model = {
        let viewer = viewer.clone();
        move |_| {
            let viewer = viewer.clone();
            leptos::spawn_local(async move {
                set_fix.set(true);
                gloo_timers::future::TimeoutFuture::new(10).await;
                let mut points = Vec::<f64>::new();
                let mut triangles = Vec::<usize>::new();
                let mut tri_in_shells = Vec::new();
                let mut n_points = 0;
                let mut idx = 0;
                for model in models().0.iter() {
                    if !model.show.get() {
                        continue;
                    }
                    let data = &model.data.get();
                    points.extend(data.0.iter());
                    triangles.extend(data.1.iter().map(|idx| idx + n_points));
                    tri_in_shells.resize(tri_in_shells.len() + data.1.len() / 3, idx);
                    idx += 1;
                    n_points = points.len() / 3;
                }
                let raw_model = gpf::polygonlization::make_mesh_for_triangles(
                    &points,
                    &triangles,
                    &tri_in_shells,
                );
                set_models.update(|models| {
                    models.hide();
                });
                let id = viewer
                    .borrow_mut()
                    .append_mesh(&raw_model.0, &raw_model.1, None);
                set_models.update(|models| {
                    models.add(Model::new(
                        format!("model{}", models.0.len()),
                        raw_model,
                        id,
                    ));
                });
                set_fix(false);
            });
        }
    };

    let file_input = create_node_ref::<html::Input>();
    let viewer_clone = viewer.clone();
    let on_change = move |_| {
        if let Some(files) = file_input.get().unwrap().files() {
            for i in 0..files.length() {
                if let Some(file) = files.item(i) {
                    if let Some(name) = file
                        .name()
                        .strip_suffix(".obj")
                        .and_then(|s| Some(s.to_owned()))
                    {
                        let viewer_clone = viewer_clone.clone();
                        spawn_local(async move {
                            if let Ok(raw_model) = read_obj_from_file(file).await {
                                let id = viewer_clone.borrow_mut().append_mesh(
                                    &raw_model.0,
                                    &raw_model.1,
                                    None,
                                );
                                set_models.update(|models| {
                                    models.add(Model::new(name, raw_model, id));
                                });
                            }
                        });
                    }
                }
            }
        }
    };

    view! {
        <div class = "flex flex-wrap max-w-sm mt-10 mr-10">
            <input type = "file" node_ref = file_input id = "add" on:change = on_change accept = ".obj" multiple class ="opacity-0"/>
                <label for = "add" class = "w-full">
                    <svg viewBox="0 0 24 24" stroke-linecap ="round" class = "w-8 h-8 stroke-emerald-900 bg-emerald-100 stroke-1 hover:stroke-2 hover:bg-emerald-200 rounded-full"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
                </label>
                <div class = "mt-2 w-full">
                    <ul role = "list" class = "ml-10 w-full divide-y divide-gray-100 shadow rounded bg-white">
                        <For
                            each = move || models().0.clone()
                            key = |model| model.id
                            children = move |model: Model| view!{ <Model model viewer = viewer.clone()/>}
                        />
                    </ul>
                </div>
            <button
                class = "w-20 h-fit p-1 mt-3 rounded-full border border-emerald-600 bg-emerald-100 hover:bg-emerald-200"
                class:hidden = { move || models.with(|m| m.0.is_empty()) }
                disabled = { fix }
                on:click = fix_model
            >
                {move || {
                    fix.with(|fix| {
                        if *fix { "Fixing" } else { "Fix " }
                    })
                }}
            </button>
        </div>
    }
}

#[component]
pub fn App(
    canvas: NodeRef<leptos_dom::html::Canvas>,
    viewer: Rc<RefCell<Viewer>>,
) -> impl IntoView {
    let (models, set_models) = create_signal(Models::new());
    provide_context(set_models);

    view! {
        <div class = "flex w-full h-full flex-1">
            <div>
                <ModelList models set_models viewer/>
            </div>
            <div class = "w-full h-full">
                <canvas node_ref = canvas class = "w-full h-full"/>
            </div>
        </div>
    }
}
