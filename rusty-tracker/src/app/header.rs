use super::sequence::note_to_display;
use crate::{
    app::{PlaybackArgs, TIMEOUT_DURATION},
    invoke,
};
use async_std::future;
use futures_util::StreamExt;
use leptos::{logging::*, *};
use serde::Serialize;
use serde_wasm_bindgen::to_value;
use tauri_sys::event;
use tracker_lib::{MidiNote, PlaybackCmd};
use wasm_bindgen_futures::spawn_local;

#[derive(Serialize)]
struct TempoArgs {
    tempo: u64,
}

#[derive(Serialize)]
struct BeatArgs {
    beat: u64,
}

// #[component]
// pub fn Header() -> impl IntoView {
//     view! {
//         // settings menu
//         <p> "settings menu (WIP)" </p>
//         <div class="justify-center text-center gap-x-2 flex">
//             <button
//                 class="bg-peach px-2"
//                 on:click=move |_| {
//                     let args = PlaybackArgs {
//                         playback_cmd: PlaybackCmd::Play,
//                     };
//
//                     spawn_local(async move {
//                         log!("starting playback");
//                         invoke("playback", to_value(&args).unwrap()).await;
//                     });
//                 }
//             >
//                 "start playback"
//             </button>
//             // <div class="p-2"></div>
//             <button
//                 class="bg-peach px-2"
//                 on:click=move |_| {
//                     let args = PlaybackArgs {
//                         playback_cmd: PlaybackCmd::Stop,
//                     };
//
//                     spawn_local(async move {
//                         log!("stoping playback");
//                         invoke("playback", to_value(&args).unwrap()).await;
//                     });
//                 }
//             >
//                 "stop playback"
//             </button>
//         </div>
//     }
// }

#[component]
pub fn SideCar(set_playhead: WriteSignal<usize>) -> impl IntoView {
    view! {
        <h1>"Setttings"</h1>
        // playback controls
        <PlaybackControls set_playhead=set_playhead/>
        // song information (bpm, row_beat)
        <SettingsMenu/>
        // wave table selection & what note is playing on what track
        <ActivityMonitor/>
        // spectrograph
        // <Spectrograph/>
        // waveform analyzer
    }
}

#[component]
fn PlaybackControls(set_playhead: WriteSignal<usize>) -> impl IntoView {
    view! {
        <div class="justify-center text-center gap-x-2 flex">
            <p>
                "playback: "
            </p>
            <button
                class="bg-peach px-2"
                on:click=move |_| {
                    let args = PlaybackArgs {
                        playback_cmd: PlaybackCmd::Play,
                    };

                    spawn_local(async move {
                        log!("starting playback");
                        if let Ok(res) = future::timeout(TIMEOUT_DURATION, invoke("playback", to_value(&args).unwrap())).await {
                            if let Err(e) = res {
                                error!("starthreads.replace()tting playback failed with error: {e:?}");
                            }
                        } else {
                            error!("starting playback timed-out");
                        }
                    });
                }
            >
                "start"
            </button>
            // <div class="p-2"></div>
            <button
                class="bg-peach px-2"
                on:click=move |_| {
                    let args = PlaybackArgs {
                        playback_cmd: PlaybackCmd::Stop,
                    };

                    spawn_local(async move {
                        log!("stoping");
                        if let Ok(res) = future::timeout(TIMEOUT_DURATION, invoke("playback", to_value(&args).unwrap())).await {
                            if let Err(e) = res {
                                error!("stopping playback failed with error: {e:?}");
                            }
                            set_playhead.set(0);
                        } else {
                            error!("stopping playback timed-out");
                        }
                    });
                }
            >
                "stop"
            </button>
        </div>

    }
}

#[component]
fn SettingsMenu() -> impl IntoView {
    let (tempo, set_tempo) = create_signal(110);
    let (beat, set_beat) = create_signal(8);

    let backend_tempo = move || {
        let args = TempoArgs {
            tempo: tempo.get_untracked(),
        };

        log!("sending tempo to backend");

        spawn_local(async move {
            // warn!("adding note async block");
            if let Err(e) = invoke("set_tempo", to_value(&args).unwrap()).await {
                error!("attempt to set the tempo failed with error: {e:?}");
            }
        });
    };

    let backend_beat = move || {
        let args = BeatArgs {
            beat: beat.get_untracked(),
        };

        log!("sending beat to backend");

        spawn_local(async move {
            // warn!("adding note async block");
            if let Err(e) = invoke("set_beat", to_value(&args).unwrap()).await {
                error!("attempt to set the beat failed with error: {e:?}");
            }
        });
    };

    let tempo_change = move |ev| {
        set_tempo.set(event_target_value(&ev).parse().unwrap());

        // send tempo change to the back-end
        backend_tempo();
    };

    let row_beat_change = move |ev| {
        if let Ok(new_beat) = event_target_value(&ev).parse() {
            let old_beat = beat.get();

            let set_beat_to = match (old_beat, new_beat) {
                (1, 0) => 512,
                (2, 3) => 4,
                (2, 1) => 1,
                (4, 3) => 2,
                (4, 5) => 8,
                (8, 7) => 4,
                (8, 9) => 16,
                (16, 15) => 8,
                (16, 17) => 32,
                (32, 31) => 16,
                (32, 33) => 64,
                (64, 63) => 32,
                (64, 65) => 128,
                (128, 127) => 64,
                (128, 129) => 256,
                (256, 255) => 128,
                (256, 257) => 512,
                (512, 511) => 256,
                (512, 513) => 1,
                _ => new_beat,
            };

            // log!("old_beat = {old_beat}");
            // log!("new_beat = {new_beat}");
            // log!("set_beat_to = {set_beat_to}");

            set_beat.set(set_beat_to);

            // send beat change to back-end
            backend_beat();
        }
    };

    backend_tempo();
    backend_beat();

    view! {
        <div class="grid grid-flow-col gap-x-2">
            <div class="justify-center text-center">
                <h1> "Tempo:" </h1>
                <input type="number" name="tempo" min=20 max=420 value=110 on:change=tempo_change/>
            </div>
            // <div> </div>
            <div class="justify-center text-center">
                <h1> "Beat:" </h1>
                <div class="flex flex-row justify-center text-center">
                    <p> "1/" </p>
                    <input type="number" name="beat" min=0 max=513 prop:value=beat on:change=row_beat_change/>
                </div>
            </div>
        </div>
    }
}

async fn listen_on_note_change_event(
    event_writer: WriteSignal<Option<MidiNote>>,
    track_number: usize,
) {
    loop {
        let mut events = event::listen::<(usize, Option<MidiNote>)>("note-change")
            .await
            .unwrap();

        while let Some(event) = events.next().await {
            let (track, midi_note) = event.payload;

            if track == track_number {
                log!("Received  event.");
                event_writer.set(midi_note);
            }
        }
    }
}

#[component]
fn TrackMonitor(track_number: usize) -> impl IntoView {
    // make signal
    let (playing_note, set_playing_note) = create_signal(None);

    // start event listener to update signal
    spawn_local(listen_on_note_change_event(set_playing_note, track_number));

    view! {
        { move ||
            match playing_note.get() {
                Some(note) => view! {
                    <div>
                        <br/>
                        {note_to_display(note)}
                        <br/>
                    </div>
                },
                None => view! {
                    <div>
                        <br/>
                        <br/>
                        <br/>
                    </div>
                },
            }
        }
    }
}

#[component]
fn ActivityMonitor() -> impl IntoView {
    view! {
        <div class="grid grid-flow-col gap-x-2">
            <For
                each=move || (0..4)
                key=move |i| *i
                children=move |i| view! {
                    <div class="bg-green">
                        <div>
                            // TODO: wavetable setter
                        </div>
                        <TrackMonitor track_number=i/>
                    </div>
                }
            />
        </div>
    }
}
