use std::{collections::HashMap, io::BufReader};

use js_sys::Uint8Array;
use leptos::*;
use tobj::Material;
use web_sys::{CustomEvent, HtmlCanvasElement};

mod canvas;
use canvas::Canvas;

type RawModel = (Vec<f64>, Vec<usize>);

#[derive(Clone, Debug, PartialEq)]
pub struct Model {
    id: uuid::Uuid,
    name: RwSignal<String>,
    data: RwSignal<RawModel>,
}

impl Model {
    fn new(name: String, model: RawModel) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name: create_rw_signal(name),
            data: create_rw_signal(model),
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

    fn remove(&mut self, id: uuid::Uuid) {
        self.0.retain(|m| m.id != id);
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
pub fn Model(model: Model) -> impl IntoView {
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
        set_models.update(|models| models.remove(model.id));
    };
    view! {
        <li class = "group/li w-full p-2 hover:bg-emerald-100">
            <div class = "flex flex-1 items-center justify-center">
                <label class = "w-full">
                    {move || model.name.get() }
                </label>
                <div class = "flex">
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
pub fn ModelList(models: ReadSignal<Models>, set_models: WriteSignal<Models>) -> impl IntoView {
    let fix_action = create_action(move |_: &()| {
        let mut points = Vec::<f64>::new();
        let mut triangles = Vec::<usize>::new();
        let mut tri_in_shells = Vec::new();
        let mut n_points = 0;
        for (i, model) in models().0.iter().enumerate() {
            let data = &model.data.get();
            points.extend(data.0.iter());
            triangles.extend(data.1.iter().map(|idx| idx + n_points));
            tri_in_shells.resize(tri_in_shells.len() + data.1.len() / 3, i);
            n_points = points.len() / 3;
        }
        async move {
            gloo_timers::future::TimeoutFuture::new(0).await;
            gpf::polygonlization::make_mesh_for_triangles(&points, &triangles, &tri_in_shells)
        }
    });

    let fix_pending = fix_action.pending();
    let fixed_model = fix_action.value();

    create_effect(move |_| {
        if fixed_model().is_some() {
            if let Ok(event) = CustomEvent::new("ce_update_list") {
                if let Err(err) = window().dispatch_event(&event) {
                    leptos::logging::warn!(
                        "failed to dispath 'update list' event with error {:?}",
                        err
                    );
                }
            }
        }
    });

    window_event_listener_untyped("ce_update_list", move |_| {
        if let Some(raw_model) = fixed_model() {
            set_models.update(|models| {
                models.add(Model::new(format!("model{}", models.0.len()), raw_model));
            });
        }
    });

    let file_input = create_node_ref::<html::Input>();
    let on_change = move |_| {
        if let Some(files) = file_input.get().unwrap().files() {
            for i in 0..files.length() {
                if let Some(file) = files.item(i) {
                    if let Some(name) = file
                        .name()
                        .strip_suffix(".obj")
                        .and_then(|s| Some(s.to_owned()))
                    {
                        spawn_local(async move {
                            if let Ok(raw_model) = read_obj_from_file(file).await {
                                set_models.update(|models| {
                                    models.add(Model::new(name, raw_model));
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
                            children = move |model: Model| view!{ <Model model/>}
                        />
                    </ul>
                </div>
            <button
                class = "w-20 h-fit p-1 mt-3 rounded-full border border-emerald-600 bg-emerald-100 hover:bg-emerald-200"
                class:hidden = { move || models.with(|m| m.0.is_empty())}
                disabled = fix_pending
                on:click = move |_| {
                    fix_action.dispatch(());
                }
            >
                {move || {
                    fix_pending.with(|fix| {
                        if *fix { "Fixing" } else { "Fix " }
                    })
                }}
            </button>
        </div>
    }
}

#[component]
pub fn App(canvas: NodeRef<leptos_dom::html::Canvas>) -> impl IntoView {
    let (models, set_models) = create_signal(Models::new());
    provide_context(set_models);

    view! {
        <div class = "flex w-full h-full flex-1">
            <div>
                <ModelList models set_models />
            </div>
            <div class = "w-full h-full">
                <Canvas canvas />
            </div>
        </div>
    }
}
