use super::sequence::note_to_display;
use crate::{app::PlaybackArgs, invoke};
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
                        invoke("playback", to_value(&args).unwrap()).await;
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
                        invoke("playback", to_value(&args).unwrap()).await;
                        set_playhead.set(0);
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
                    <input type="number" name="beat" min=1 max=64 value=4 on:change=row_beat_change/>
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

    // let display = move || match playing_note.get() {
    //     Some(note) => note_to_display(note),
    //     None => String::new(),
    // };

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
