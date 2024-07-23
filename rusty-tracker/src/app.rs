use futures_util::StreamExt;
use leptos::{logging::*, *};
use leptos_hotkeys::{
    provide_hotkeys_context,
    scopes,
    use_hotkeys,
    // use_hotkeys_context,
    HotkeysContext,
};
use leptos_use::{use_element_size, UseElementSizeReturn};
use sequence::Sequence;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use std::fmt::Display;
use tauri_sys::event;
use tracker_lib::{ChannelIndex, MidiNote, MidiNoteCmd, PlaybackCmd, TrackerState};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

mod sequence;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"])]
    pub async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
struct PlaybackArgs {
    pub playback_cmd: PlaybackCmd,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct GetStateArgs {
    start_row: usize,
    // stop_row: usize,
    n_rows: usize,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum FontSize {
    #[serde(rename = "small")]
    SM,
    #[serde(rename = "normal")]
    Base,
    #[serde(rename = "large")]
    LG,
}

impl Display for FontSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            FontSize::Base => write!(f, "text-base"),
            FontSize::SM => write!(f, "text-sm"),
            FontSize::LG => write!(f, "text-lg"),
        }
    }
}

impl IntoAttribute for FontSize {
    fn into_attribute(self) -> Attribute {
        Attribute::String(match self {
            FontSize::SM => "text-sm".into(),
            FontSize::Base => "text-base".into(),
            FontSize::LG => "text-lg".into(),
        })
    }

    fn into_attribute_boxed(self: Box<Self>) -> Attribute {
        Attribute::String(match *self {
            FontSize::SM => "text-sm".into(),
            FontSize::Base => "text-base".into(),
            FontSize::LG => "text-lg".into(),
        })
    }
}

impl Into<f64> for FontSize {
    fn into(self) -> f64 {
        match self {
            FontSize::SM => 20.0,
            FontSize::Base => 24.0,
            FontSize::LG => 28.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mode {
    Move,
    Command,
    Edit,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
struct AddNoteArgs {
    note: MidiNote,
    channel: ChannelIndex,
    start: usize,
    stop: usize,
    note_number: usize,
}

#[derive(
    Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct NoteSetStorage {
    note: MidiNote,
    end_loc: (usize, usize),
    display_loc: (usize, usize),
}

async fn listen_on_state_change_event(event_writer: WriteSignal<TrackerState>) {
    loop {
        let mut events = event::listen::<TrackerState>("state-change").await.unwrap();

        while let Some(event) = events.next().await {
            log!("Received state-change event.");
            event_writer.set(event.payload);
        }
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (tracker_state, set_tracker_state) = create_signal(TrackerState::empty());
    #[allow(unused_variables)]
    let (font_size, set_font_size) = create_signal(FontSize::Base);
    let (note_storage, set_note_storage) = create_signal::<Option<NoteSetStorage>>(None);
    let main_el = create_node_ref::<html::Main>();
    let HotkeysContext { .. } = provide_hotkeys_context(main_el, false, scopes!());
    // #[allow(unused_variables)]
    // let HotkeysContext {
    //     enable_scope,
    //     disable_scope,
    //     ..
    // } = use_hotkeys_context();

    let header_el = create_node_ref::<html::Div>();

    // (row_i, sequence_i)
    let (location, set_location) = create_signal((0, 0));
    let (mode, set_mode) = create_signal(Mode::Move);

    let UseElementSizeReturn {
        width: header_w,
        height: header_h,
    } = use_element_size(header_el);

    let num_lines = move |size: f64| {
        // let size = size + 8.0;
        let size = size + 2.0;
        // ((window().inner_height().unwrap().as_f64().unwrap() - header_h.get_untracked() - size)
        //     / size)
        //     .floor() as usize
        ((window().inner_height().unwrap().as_f64().unwrap() - header_h.get_untracked()) / size)
            .floor() as usize
    };

    spawn_local(listen_on_state_change_event(set_tracker_state));

    let get_state = move || {
        // let _width = width.get();
        // let size = font_size.get().into();
        let n_rows = num_lines(font_size.get_untracked().into());

        // log!(
        //     "n_rows: {n_rows:0X} | height: {}",
        //     window().inner_height().unwrap().as_f64().unwrap() - header_h.get_untracked()
        // );
        //
        // log!(
        //     "n_rows: {n_rows:0X} ({n_rows}, in base 10) | height: {}",
        //     window().inner_height().unwrap().as_f64().unwrap() - header_h.get_untracked()
        // );

        let args = GetStateArgs {
            start_row: 0,
            n_rows,
        };

        spawn_local(async move {
            // warn!("inside async block");
            log!("requesting state");
            invoke("get_state", to_value(&args).unwrap()).await;
        });
    };

    create_effect(move |_| {
        // log!("height: {}", height.get());
        let _ = window().inner_height();
        let _ = header_h.get();
        log!(
            "height: {}",
            (window().inner_height().unwrap().as_f64().unwrap() - header_h.get())
        );
        log!("width: {}", header_w.get());
        log!("font_size: {}", font_size.get());

        get_state();
    });

    create_effect(move |_| {
        let loc = location.get();

        log!("location change => {loc:?}");

        match mode.get() {
            Mode::Edit => {
                set_note_storage.update(|storage| {
                    if let Some(selected) = storage {
                        selected.end_loc = loc
                    }
                });
            }
            _ => {}
        }
    });

    let state_size = move || tracker_state.get().sequences[0].len();

    let line_numbers = move || {
        (0..state_size())
            .into_iter()
            .map(|i| {
                let line_num = format!("{:04X}", i);

                let click = move |_| {
                    let args = PlaybackArgs {
                        playback_cmd: PlaybackCmd::SetCursor(i),
                    };

                    spawn_local(async move {
                        // logging::warn!("")
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
        let line_nums: Vec<_> = line_numbers();
        // nums.append(&mut line_nums);

        view! {
            <div class="col-span-1 grid-flow-row p-2">
                <div class="grid grid-flow-col"> <br/> </div>
                <div class="grid grid-flow-col"> <br/> </div>
                { line_nums.collect_view() }
            </div>
        }
    };

    let get_start = create_memo(move |_| tracker_state.get().display_start);

    let get_sequences = move || {
        let start = get_start.get_untracked();
        let _n_lines = state_size();

        (start..start + 4)
            .into_iter()
            .map(|i| {
                view! {
                    // <Sequence state=tracker_state set_state=set_tracker_state i=i/>
                    <Sequence state=tracker_state i=i get_loc=location get_mode=mode set_loc=set_location get_storage=note_storage/>
                }
            })
            .collect_view()
    };

    let set_backend_note = move || {
        if let Some(note) = note_storage.get() {
            let midi_code = note.note;
            let (start_loc, stop_loc) = if note.display_loc.0 < note.end_loc.0 {
                (note.display_loc, note.end_loc)
            } else {
                (note.end_loc, note.display_loc)
            };
            // let state = tracker_state.get();
            let channel = start_loc.1 / 6;
            let note_num = start_loc.1 % 6;

            let args_play = AddNoteArgs {
                note: midi_code,
                channel: channel as ChannelIndex,
                note_number: note_num,
                start: start_loc.0,
                stop: stop_loc.0,
            };

            warn!("sending note to backend");

            spawn_local(async move {
                // warn!("adding note async block");
                invoke("add_note", to_value(&args_play).unwrap()).await;

                get_state();
            });
        }
    };

    let set_display_note = move |note: Option<MidiNoteCmd>| {
        let loc = location.get();
        // let state = tracker_state.get();
        let channel = loc.1 / 6;
        let note_num = loc.1 % 6;

        set_tracker_state.update(|state| state.sequences[channel][loc.0].notes[note_num] = note);
    };

    let set_note = move |mut note_update_func: Box<dyn FnMut(MidiNote) -> MidiNote>| {
        // update note_storage
        set_note_storage.update(|storage| {
            if let Some(storage) = storage {
                storage.note = note_update_func(storage.note);
            }
        });

        // // update note
        // match note_storage.get() {
        //     Some(midi_note) => set_display_note(Some(MidiNoteCmd::PlayNote(midi_note.note))),
        //     None => set_display_note(None),
        // }
    };

    let up_semi = move || {
        log!("up semi-tone");
        // let loc = location.get();
        set_note_storage
            .update(|midi_note| midi_note.unwrap().note = (midi_note.unwrap().note + 1) % 128);
        set_note(Box::new(|storage| (storage + 1) % 128));
        // send_note_to_backend();
    };

    let up_octave = move || {
        log!("up octave");
        set_note_storage
            .update(|midi_note| midi_note.unwrap().note = (midi_note.unwrap().note + 12) % 128);

        set_note(Box::new(|storage| (storage + 12) % 128));
        // send_note_to_backend();
    };

    let down_semi = move || {
        log!("down semi-tone");
        set_note_storage.update(|midi_note| {
            midi_note.unwrap().note = (midi_note.unwrap().note as i32 - 1) as MidiNote % 128
        });

        set_note(Box::new(|storage| (storage as i32 - 1) as MidiNote % 128));
        // send_note_to_backend();
    };

    let down_octave = move || {
        log!("down octave");
        set_note_storage.update(|midi_note| {
            midi_note.unwrap().note = (midi_note.unwrap().note as i32 - 12) as MidiNote % 128
        });

        set_note(Box::new(|storage| (storage as i32 - 12) as MidiNote % 128));
        // send_note_to_backend();
    };

    let cursor_up = move || {
        // set_count.update(|c| *c += 1);
        set_location.update(|loc| {
            // if let Some((row, _)) = loc {
            if loc.0 == 0 {
                loc.0 = num_lines(font_size.get_untracked().into())
            } else {
                loc.0 -= 1
            }
            // }
        });
    };

    let cursor_down = move || {
        // set_count.update(|c| *c += 1);
        set_location.update(|loc| {
            // if let Some((row, _)) = loc {
            if loc.0 == num_lines(font_size.get_untracked().into()) {
                loc.0 = 0
            } else {
                loc.0 += 1
            }
            // }
        });
    };

    use_hotkeys!(("keyw") => move |_| {
        if mode.get() == Mode::Move {
            cursor_up()
        }

        log!("w has been pressed");
    });

    use_hotkeys!(("keys") => move |_| {
        if mode.get() == Mode::Move {
            cursor_down()
        }

        log!("s has been pressed");
    });

    let max_col = (4 + 2) * 4;

    use_hotkeys!(("keya") => move |_| {
        log!("a has been pressed");

        if mode.get() == Mode::Move {
            // set_count.update(|c| *c += 1);
            set_location.update(|loc| {
                // if let Some((_, col)) = loc {
                if loc.1 == 0 {
                    loc.1 = max_col
                } else {
                    loc.1 -= 1
                }
                // }
            });
        }
    });

    use_hotkeys!(("keyd") => move |_| {
        log!("d has been pressed");

        if mode.get() == Mode::Move {
            // set_count.update(|c| *c += 1);
            set_location.update(|loc| {
                // if let Some((_, col)) = loc {
                if loc.1 == max_col {
                    loc.1 = 0
                } else {
                    loc.1 += 1
                }
                // }
            });
        }
    });

    use_hotkeys!(("keyw") => move |_| {
        if mode.get() == Mode::Edit {
            set_note_storage.update(|storage| if let Some(selected) = storage && selected.end_loc.0 > 0 {
                selected.end_loc.0 -= 1
            });

            cursor_up();
        }
    });

    use_hotkeys!(("keys") => move |_| {
        if mode.get() == Mode::Edit {
            set_note_storage.update(|storage| if let Some(selected) = storage {
                selected.end_loc.0 += 1
            });

            cursor_down();
        }
    });

    use_hotkeys!(("Escape", "*") => move |_| {
        log!("Escape key has been pressed");

        match mode.get() {
            Mode::Move => {
                // toggle_scope.call("command".to_string());
                set_mode.set(Mode::Command);
            }
            Mode::Command => {
                // toggle_scope.call("move".to_string());
                set_mode.set(Mode::Move);
            }
            Mode::Edit => {
                // set_display_note(None);
                // set_display_note(None);
                set_note_storage.set(None);
                set_display_note(None);
                // toggle_scope.call("move".to_string());
                set_mode.set(Mode::Move);
            }
        }
    });

    use_hotkeys!(("Enter", "*") => move |_| {
        match mode.get() {
            Mode::Move => {
                let loc = location.get();

                log!("settings scope to edit");
                // toggle_scope.call("edit".to_string());
                set_mode.set(Mode::Edit);
                set_note_storage.set(Some(NoteSetStorage { note: 0, end_loc: (loc.0 + 1, loc.1), display_loc: loc }));
                cursor_down();
            }
            Mode::Edit => {
                set_backend_note();

                set_note_storage.set(None);

                log!("settings scope to move");
                // toggle_scope.call("move".to_string());
                set_mode.set(Mode::Move);
            }
            Mode::Command => {
                // TODO: execute command
            }
        }
    });

    use_hotkeys!(("Backspace") => move |_| {
        log!("Backspace key has been pressed");

        if mode.get() == Mode::Move {
            // set_note(None);
            // TODO: remove note from backend

            // toggle_scope.call("move".to_string());
            set_mode.set(Mode::Move);
        }
    });

    use_hotkeys!(("ArrowUp") => move |_| {
        if mode.get() == Mode::Edit {
            up_semi();
        }
    });

    use_hotkeys!(("ArrowDown") => move |_| {
        if mode.get() == Mode::Edit {
            down_semi();
        }
    });

    use_hotkeys!(("ArrowRight") => move |_| {
        if mode.get() == Mode::Edit {
            up_octave();
        }
    });

    use_hotkeys!(("ArrowLeft") => move |_| {
        if mode.get() == Mode::Edit {
            down_octave();
        }
    });

    view! {
        <main node_ref=main_el class={ move || format!("justify-center text-center w-full h-full {}", font_size.get()) }>
            <div node_ref=header_el>
                // TODO: make settings menu
                <p> "settings menu (WIP)" </p>
                <div class="justify-center text-center gap-x-2 flex">
                    <button
                        class="bg-peach"
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
                        class="bg-peach"
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
            </div>

            <div class="grid grid-cols-12 h-fit max-h-fit">
                { get_line_nums }
                { get_sequences() }
            </div>
        </main>
    }
}
