use leptos::*;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use tracker_lib::{PlaybackCmd, TrackerState};
use wasm_bindgen::prelude::*;

mod sequence;

use sequence::Sequence;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"])]
    pub async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
struct PlaybackArgs {
    pub playback_cmd: PlaybackCmd,
}

#[component]
pub fn App() -> impl IntoView {
    let (tracker_state, set_tracker_state) = create_signal(TrackerState::default());

    let line_numbers = |get_state: Box<dyn FnOnce() -> TrackerState>| {
        (0..get_state()
            .sequences
            .iter()
            // .filter(|s| s.is_some())
            .map(|s| s.clone().len())
            .max()
            .unwrap_or(0))
            .into_iter()
            .map(|i| {
                let line_num = format!("{:04X}", i);

                let click = move |_| {
                    let args = PlaybackArgs {
                        playback_cmd: PlaybackCmd::SetCursor(i),
                    };

                    spawn_local(async move {
                        // logging::warn!("");
                        invoke("playback", to_value(&args).unwrap()).await;
                    });
                };

                view! {
                    <button class="grid grid-flow-col" on:click=click>
                        { line_num }
                    </button>
                }
            })
            .collect()
    };

    let get_line_nums = move || {
        let get_state = Box::new(move || tracker_state.get());
        // let mut nums = vec![
        //     view! {<div class="grid grid-flow-col"> <br/> </div>},
        //     view! {<div class="grid grid-flow-col"> <br/> </div>},
        // ];
        let line_nums: Vec<_> = line_numbers(get_state);
        // nums.append(&mut line_nums);

        view! {
            <div class="col-span-1 grid-flow-row p-2">
                <div class="grid grid-flow-col"> <br/> </div>
                <div class="grid grid-flow-col"> <br/> </div>
                { line_nums.collect_view() }
            </div>
        }
    };

    let get_sequences = move || {
        let start = tracker_state.get().display_start;
        (start..start + 4)
            .into_iter()
            .map(|i| {
                view! {
                    // <Sequence state=tracker_state set_state=set_tracker_state i=i/>
                    <Sequence state=tracker_state i=i/>
                }
            })
            .collect_view()
    };

    view! {
        <main class="justify-center text-center">
            <div class="">
                // TODO: make settings menu
                <p> "settings menu (WIP)" </p>
                <button on:click=move |_| {
                    let args = PlaybackArgs {
                        playback_cmd: PlaybackCmd::Play,
                    };

                    spawn_local(async move {
                        logging::warn!("starting playback");
                        invoke("playback", to_value(&args).unwrap()).await;
                    });
                }>
                    "start playback"
                </button>

            </div>

            <div class="grid grid-cols-12">
                // <div class="col-span-1 grid-flow-row p-2">
                { get_line_nums }
                // </div>
                { get_sequences }
            </div>
        </main>
    }
}
