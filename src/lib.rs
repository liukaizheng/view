use std::{cell::RefCell, collections::HashMap, io::BufReader, rc::Rc};

use js_sys::Uint8Array;
use leptos::prelude::*;
use leptos::task::spawn_local;
use send_wrapper::SendWrapper;
use tobj::Material;
use wasm_bindgen::JsCast;

use crate::render::viewer::Viewer;

pub type ViewerWrapper = SendWrapper<Rc<RefCell<Viewer>>>;

pub mod render;

type RawModel = (Vec<f64>, Vec<usize>);

#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    id: u32,
    name: RwSignal<String>,
    data: RwSignal<RawModel>,
    show: RwSignal<bool>,
    show_edges: RwSignal<bool>,
    edge_width: RwSignal<f64>,
    edge_color: RwSignal<String>,
    face_color: RwSignal<String>,
    face_alpha: RwSignal<f64>,
}

impl Model {
    fn new(name: String, model: RawModel, id: u32) -> Self {
        Self {
            id,
            name: RwSignal::new(name),
            data: RwSignal::new(model),
            show: RwSignal::new(true),
            show_edges: RwSignal::new(false),
            edge_width: RwSignal::new(1.0),
            edge_color: RwSignal::new("#000000".to_string()),
            face_color: RwSignal::new("#cccccc".to_string()),
            face_alpha: RwSignal::new(1.0),
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
                show.set(false);
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
        web_sys::console::warn_1(&"failed to read file buffer".into());
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

fn hex_to_rgba(hex: &str) -> [f32; 4] {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
    [r, g, b, 1.0]
}

#[component]
pub fn Model(model: Model) -> impl IntoView {
    let viewer = expect_context::<ViewerWrapper>();
    {
        let viewer = viewer.clone();
        Effect::new(move |_| {
            viewer.borrow_mut().set_visible(model.id, model.show.get());
        });
    }

    {
        let viewer = viewer.clone();
        Effect::new(move |_| {
             let width = if model.show_edges.get() { model.edge_width.get() as f32 } else { 0.0 };
            viewer
                .borrow_mut()
                .set_edge_width(model.id, width);
        });
    }

    {
        let viewer = viewer.clone();
        Effect::new(move |_| {
            let color = hex_to_rgba(&model.edge_color.get());
            viewer
                .borrow_mut()
                .set_edge_color(model.id, color);
        });
    }

    {
        let viewer = viewer.clone();
        Effect::new(move |_| {
            let color = hex_to_rgba(&model.face_color.get());
            viewer
                .borrow_mut()
                .set_face_color(model.id, [color[0], color[1], color[2]]);
        });
    }

    {
        let viewer = viewer.clone();
        Effect::new(move |_| {
            viewer
                .borrow_mut()
                .set_face_alpha(model.id, model.face_alpha.get() as f32);
        });
    }

    let write_to_local = move |_| {
        let (points, triangles) = model.data.get();
        let txt = write_obj(&points, &triangles);
        let parts = js_sys::Array::of1(&unsafe { Uint8Array::view(txt.as_bytes()).into() });
        let properties = web_sys::BlobPropertyBag::new();
        properties.set_type("model/obj");
        if let Ok(blob) =
            web_sys::Blob::new_with_buffer_source_sequence_and_options(&parts, &properties)
        {
            let link = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .create_element("a")
                .unwrap();
            let link: web_sys::HtmlAnchorElement = link.dyn_into().unwrap();
            let url = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
            link.set_href(&url);
            link.set_download(&(model.name.get() + ".obj"));
            link.click();
            web_sys::Url::revoke_object_url(&link.href()).unwrap();
        }
    };

    let set_models = expect_context::<WriteSignal<Models>>();
    let viewer_for_destroy = viewer.clone();
    let destroy = move |_| {
        set_models.update(|models| {
            let id = model.id;
            models.remove(model.id);
            viewer_for_destroy.borrow_mut().remove_data(id);
        });
    };
    let toggle_show = move |_| {
        model.show.update(|show| *show = !*show);
    };
    let toggle_edges = move |_| {
        model.show_edges.update(|show| *show = !*show);
    };
    const SHOW_CLASS_ATTR: &str = "w-full";
    const HIDE_CLASS_ATTR: &str = "w-full text-gray-400";
    view! {
          <li class = "group/li w-full p-2 hover:bg-emerald-100 flex flex-col">
              <div class = "flex flex-1 items-center justify-center w-full">
                  <label class = move || if model.show.get() { SHOW_CLASS_ATTR} else {HIDE_CLASS_ATTR}>
                      {move || model.name.get() }
                  </label>
                  <div class = "flex">
                  <button on:click = toggle_show class = "group/button w-6 h-6 hover:bg-emerald-200 rounded-full items-center justify-center hidden group-hover/li:flex mr-1">
                      { move || {
                          if model.show.get() {
                               view! { <svg class= "w-4 h-4 stroke-2 stroke-emerald-900" aria-hidden="true" fill="none" viewBox="0 0 20 14"> <g> <path d="M10 10a3 3 0 1 0 0-6 3 3 0 0 0 0 6Z"/> <path d="M10 13c4.97 0 9-2.686 9-6s-4.03-6-9-6-9 2.686-9 6 4.03 6 9 6Z"/> </g> </svg>}.into_any()
                          } else {
                               view! {<svg class= "w-4 h-4 stroke-2 stroke-emerald-900" aria-hidden="true" fill="none" viewBox="0 0 20 18"><path stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" d="M1.933 10.909A4.357 4.357 0 0 1 1 9c0-1 4-6 9-6m7.6 3.8A5.068 5.068 0 0 1 19 9c0 1-3 6-9 6-.314 0-.62-.014-.918-.04M2 17 18 1m-5 8a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z"/>
        </svg>}.into_any()
                          }
                      }}
                  </button>
                    <button on:click = toggle_edges class = "group/button w-6 h-6 hover:bg-emerald-200 rounded-full items-center justify-center hidden group-hover/li:flex mr-1" title="Toggle Edges">
                        { move || {
                             if model.show_edges.get() {
                                  view! { <svg class= "w-4 h-4 stroke-2 stroke-emerald-900" fill="none" viewBox="0 0 24 24" stroke="currentColor"> <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z"/> </svg>}.into_any()
                             } else {
                                  view! { <svg class= "w-4 h-4 stroke-2 stroke-emerald-900" fill="none" viewBox="0 0 24 24" stroke="currentColor"> <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"/> </svg>}.into_any()
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
               { move || {
                 if model.show.get() {
                     view! {
                         <div class="flex items-center space-x-2 mt-2 px-2 text-xs w-full">
                            <span class="w-8">Face:</span>
                            <input type="range" min="0.0" max="1.0" step="0.01"
                                prop:value=move || model.face_alpha.get()
                                on:input=move |ev| model.face_alpha.set(event_target_value(&ev).parse().unwrap_or(1.0))
                                class="w-16"
                                title="Face Transparency"
                            />
                            <input type="color"
                                prop:value=move || model.face_color.get()
                                on:input=move |ev| model.face_color.set(event_target_value(&ev))
                                class="w-6 h-6 border-none bg-transparent"
                                title="Face Color"
                            />
                         </div>
                     }.into_any()
                 } else {
                     view! { <div/> }.into_any()
                 }
               }}
               { move || {
                 if model.show_edges.get() {
                     view! {
                         <div class="flex items-center space-x-2 mt-2 px-2 text-xs w-full">
                            <input type="range" min="0.5" max="5.0" step="0.5"
                                prop:value=move || model.edge_width.get()
                                on:input=move |ev| model.edge_width.set(event_target_value(&ev).parse().unwrap_or(1.0))
                                class="w-20"
                            />
                            <input type="color"
                                prop:value=move || model.edge_color.get()
                                on:input=move |ev| model.edge_color.set(event_target_value(&ev))
                                class="w-6 h-6 border-none bg-transparent"
                            />
                         </div>
                     }.into_any()
                 } else {
                     view! { <div/> }.into_any()
                 }
               }}
          </li>
      }
}

#[component]
pub fn ModelList(
    models: ReadSignal<Models>,
    set_models: WriteSignal<Models>,
) -> impl IntoView {
    let viewer = expect_context::<ViewerWrapper>();
    let (fix, set_fix) = signal(false);
    
    // Virtualization state
    let (scroll_top, set_scroll_top) = signal(0.0);
    let container_height = 500.0; // Fixed height for the visible area (pixels)
    const ITEM_HEIGHT: f64 = 60.0; // Estimated height of each list item (pixels)
    const BUFFER_ITEMS: usize = 5;

    // Computed visible range
    let visible_models = move || {
        let all_models = models.get().0;
        let total_count = all_models.len();
        
        let start_idx = (scroll_top.get() / ITEM_HEIGHT).floor() as usize;
        let start_idx = start_idx.saturating_sub(BUFFER_ITEMS);
        
        let visible_count = (container_height / ITEM_HEIGHT).ceil() as usize + 2 * BUFFER_ITEMS;
        let end_idx = (start_idx + visible_count).min(total_count);
        
        let subset = all_models[start_idx..end_idx].to_vec();
        
        let padding_top = start_idx as f64 * ITEM_HEIGHT;
        let padding_bottom = (total_count.saturating_sub(end_idx)) as f64 * ITEM_HEIGHT;
        
        (subset, padding_top, padding_bottom)
    };

    let fix_model = {
        let viewer = viewer.clone();
        move |_| {
            let viewer = viewer.clone();
            spawn_local(async move {
                set_fix.set(true);
                gloo_timers::future::TimeoutFuture::new(10).await;
                let mut points = Vec::<f64>::new();
                let mut triangles = Vec::<usize>::new();
                let mut tri_in_shells = Vec::new();
                let mut n_points = 0;
                let mut idx = 0;
                for model in models.get().0.iter() {
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
                // TODO: gpf dependency was removed, fix functionality disabled
                let raw_model: RawModel = (points, triangles);
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
                set_fix.set(false);
            });
        }
    };

    let file_input: NodeRef<leptos::html::Input> = NodeRef::new();
    let viewer_clone = viewer.clone();
    let on_change = move |_| {
        if let Some(input_el) = file_input.get() {
            let input_el: &web_sys::HtmlInputElement = &input_el;
            if let Some(files) = input_el.files() {
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
        }
    };

    view! {
        <div class = "flex flex-wrap max-w-sm mt-10 mr-10 h-full flex-col">
            <div class="flex w-full mb-2">
                <input type = "file" node_ref = file_input id = "add" on:change = on_change accept = ".obj" multiple class ="opacity-0 hidden"/>
                <label for = "add" class = "">
                    <svg viewBox="0 0 24 24" stroke-linecap ="round" class = "w-8 h-8 stroke-emerald-900 bg-emerald-100 stroke-1 hover:stroke-2 hover:bg-emerald-200 rounded-full cursor-pointer"><line x1="12" y1="5" x2="12" y2="19"></line><line x1="5" y1="12" x2="19" y2="12"></line></svg>
                </label>
                 <button
                    class = "w-20 h-fit p-1 ml-4 rounded-full border border-emerald-600 bg-emerald-100 hover:bg-emerald-200"
                    class:hidden = move || models.get().0.is_empty()
                    disabled = move || fix.get()
                    on:click = fix_model
                >
                    {move || {
                        if fix.get() { "Fixing" } else { "Fix " }
                    }}
                </button>
            </div>

            <div 
                class = "w-full divide-y divide-gray-100 shadow rounded bg-white overflow-y-auto border border-gray-200"
                style = format!("height: {}px;", container_height)
                on:scroll = move |ev| {
                     let target: web_sys::HtmlElement = event_target(&ev);
                     set_scroll_top.set(target.scroll_top() as f64);
                }
            >
                <div class="w-full relative">
                    {move || {
                        let (subset, p_top, p_bottom) = visible_models();
                        view! {
                            <div style=format!("height: {}px;", p_top)></div>
                            <ul role="list" class="w-full divide-y divide-gray-100">
                                <For
                                    each = move || subset.clone()
                                    key = |model| model.id
                                    let:model
                                >
                                    <Model model=model.clone()/>
                                </For>
                            </ul>
                            <div style=format!("height: {}px;", p_bottom)></div>
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

#[component]
pub fn App(
    canvas: NodeRef<leptos::html::Canvas>,
    viewer: ViewerWrapper,
) -> impl IntoView {
    web_sys::console::log_1(&"App component rendering".into());
    let (models, set_models) = signal(Models::new());
    provide_context(set_models);
    provide_context(viewer.clone());

    view! {
        <div class = "flex w-full h-full flex-1">
            <div>
                <ModelList models set_models/>
            </div>
            <div class = "w-full h-full">
                <canvas node_ref = canvas class = "w-full h-full"/>
            </div>
        </div>
    }
}
