use crate::{app::PlaybackArgs, invoke};
use leptos::{logging::*, *};
use serde::Serialize;
use serde_wasm_bindgen::to_value;
use tracker_lib::PlaybackCmd;
use wasm_bindgen_futures::spawn_local;

#[derive(Serialize)]
struct TempoArgs {
    tempo: u64,
}

#[derive(Serialize)]
struct BeatArgs {
    beat: u64,
}

#[component]
pub fn Header() -> impl IntoView {
    view! {
        // settings menu
        <p> "settings menu (WIP)" </p>
        <div class="justify-center text-center gap-x-2 flex">
            <button
                class="bg-peach px-2"
                on:click=move |_| {
                    let args = PlaybackArgs {
                        playback_cmd: PlaybackCmd::Play,
                    };

                    spawn_local(async move {
                        log!("starting playback");
                        invoke("playback", to_value(&args).unwrap()).await;
                    });
                }
            >
                "start playback"
            </button>
            // <div class="p-2"></div>
            <button
                class="bg-peach px-2"
                on:click=move |_| {
                    let args = PlaybackArgs {
                        playback_cmd: PlaybackCmd::Stop,
                    };

                    spawn_local(async move {
                        log!("stoping playback");
                        invoke("playback", to_value(&args).unwrap()).await;
                    });
                }
            >
                "stop playback"
            </button>
        </div>
    }
}

#[component]
pub fn SideCar() -> impl IntoView {
    let (tempo, set_tempo) = create_signal(110);
    let (beat, set_beat) = create_signal(4);

    let backend_tempo = move || {
        let args = TempoArgs {
            tempo: tempo.get_untracked(),
        };

        log!("sending tempo to backend");

        spawn_local(async move {
            // warn!("adding note async block");
            invoke("set_tempo", to_value(&args).unwrap()).await;
        });
    };

    let backend_beat = move || {
        let args = BeatArgs {
            beat: beat.get_untracked(),
        };

        log!("sending beat to backend");

        spawn_local(async move {
            // warn!("adding note async block");
            invoke("set_beat", to_value(&args).unwrap()).await;
        });
    };

    let tempo_change = move |ev| {
        // TODO: handle tempo change on the back-end
        set_tempo.set(event_target_value(&ev).parse().unwrap());

        backend_tempo();
    };
    let row_beat_change = move |ev| {
        // TODO: handle beat change on back-end
        set_beat.set(event_target_value(&ev).parse().unwrap());

        backend_beat();
    };

    backend_tempo();
    backend_beat();

    view! {
        <h1>"Setttings"</h1>
        <div class="justify-center text-center gap-x-2 flex">
            <button
                class="bg-peach px-2"
                on:click=move |_| {
                    let args = PlaybackArgs {
                        playback_cmd: PlaybackCmd::Play,
                    };

                    spawn_local(async move {
                        log!("starting playback");
                        invoke("playback", to_value(&args).unwrap()).await;
                    });
                }
            >
                "start playback"
            </button>
            // <div class="p-2"></div>
            <button
                class="bg-peach px-2"
                on:click=move |_| {
                    let args = PlaybackArgs {
                        playback_cmd: PlaybackCmd::Stop,
                    };

                    spawn_local(async move {
                        log!("stoping playback");
                        invoke("playback", to_value(&args).unwrap()).await;
                    });
                }
            >
                "stop playback"
            </button>
        </div>
        // song information (bpm, row_beat)
        <div class="grid grid-flow-col gap-x-2">
            <div class="justify-center text-center">
                // <label for="tempo"> "tempo" </label>
                <h1> "tempo" </h1>
                <input type="number" name="tempo" min=20 max=420 value=110 on:change=tempo_change/>
            </div>
            // <div> </div>
            <div class="justify-center text-center">
                // <label for="beat"> "beat" </label>
                <h1> "beat" </h1>
                <input type="number" name="beat" min=1 max=64 value="4" on:change=row_beat_change/>
            </div>
        </div>
        // wave table selection & what note is playing on what track
        // spectrograph
        // waveform analyzer
    }
}
