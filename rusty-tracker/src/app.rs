use futures_util::StreamExt;
use header::SideCar;
use leptos::{logging::*, *};
use leptos_hotkeys::{provide_hotkeys_context, scopes, use_hotkeys, HotkeysContext};
use leptos_use::{use_element_size, UseElementSizeReturn};
use sequence::Sequence;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use std::fmt::Display;
use tauri_sys::event;
use tracker_lib::{
    ChannelIndex, Float, MidiNote, MidiNoteCmd, PlaybackCmd, TrackerState, LINE_LEN,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

mod header;
pub mod sequence;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "core"])]
    pub async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(catch, js_namespace = ["window", "__TAURI__", "event"])]
    pub async fn listen(
        event: &str,
        closure: &Closure<dyn Fn(JsValue)>,
    ) -> Result<JsValue, JsValue>;
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
    vel: u8,
    channel: ChannelIndex,
    start: usize,
    stop: usize,
    note_number: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
struct RmNoteArgs {
    channel: ChannelIndex,
    row: usize,
    note_number: usize,
}

#[derive(
    Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct NoteSetStorage {
    note: MidiNote,
    loc: (usize, usize),
    n_lines: i64,
    // display_loc: (usize, usize),
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

async fn listen_on_playhead_event(event_writer: WriteSignal<usize>) {
    loop {
        let mut events = event::listen::<usize>("playhead").await.unwrap();

        while let Some(event) = events.next().await {
            log!("Received playhead event.");
            event_writer.set(event.payload);
        }
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (tracker_state, set_tracker_state) = create_signal(TrackerState::empty());
    #[allow(unused_variables)]
    let (font_size, set_font_size) = create_signal(FontSize::Base);
    let (start_row, set_start_row) = create_signal(0);
    let (note_storage, set_note_storage) = create_signal::<Option<NoteSetStorage>>(None);
    let (playhead, set_playhead) = create_signal(0);
    let main_el = create_node_ref::<html::Main>();
    let HotkeysContext { .. } = provide_hotkeys_context(main_el, false, scopes!());

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
    spawn_local(listen_on_playhead_event(set_playhead));

    create_effect(move |_| {
        let n_lines = num_lines(font_size.get_untracked().into());

        if (playhead.get() - start_row.get_untracked()) >= (n_lines as Float * 0.75) as usize {
            set_start_row.update(|row| *row += n_lines / 2);
        }
    });

    let get_state = move || {
        // let _width = width.get();
        // let size = font_size.get().into();
        let n_rows = num_lines(font_size.get_untracked().into());

        log!(
            "n_rows: {n_rows:0X} | height: {}",
            window().inner_height().unwrap().as_f64().unwrap() - header_h.get_untracked()
        );

        log!(
            "n_rows: {n_rows:0X} ({n_rows}, in base 10) | height: {}",
            window().inner_height().unwrap().as_f64().unwrap() - header_h.get_untracked()
        );

        let args = GetStateArgs {
            start_row: start_row.get_untracked(),
            n_rows,
        };

        spawn_local(async move {
            // warn!("inside async block");
            log!("requesting state");
            if let Err(e) = invoke("get_state", to_value(&args).unwrap()).await {
                error!("getting state resulted in {e:?}");
            }
            // else {
            //     log!("got state");
            // }
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
                        selected.n_lines = loc.0 as i64 - selected.loc.0 as i64;
                    }
                });
            }
            _ => {}
        }
    });

    // let state_size = move || tracker_state.get().sequences[0].len();

    let line_numbers = move || {
        let sr = start_row.get();

        view! {
            <For
                each=move || (sr..sr + tracker_state.get().sequences[0].data.len()).into_iter()
                key=move |ln| (*ln, *ln == playhead.get())
                children=move |ln| {
                    let line_num = format!("{:04X}", ln);

                    let click = move |_| {
                        let args = PlaybackArgs {
                            playback_cmd: PlaybackCmd::SetCursor(ln),
                        };

                        spawn_local(async move {
                            // logging::warn!("")
                            invoke("playback", to_value(&args).unwrap()).await;
                        });
                    };

                    let class = if ln == playhead.get() {
                        "bg-maroon"
                    } else {
                        ""
                    };

                    view! {
                        <div class=class>
                            <button on:click=click>
                                { line_num }
                            </button>
                        </div>
                    }
                }
            />
        }
        // .collect()
    };

    let get_line_nums = move || {
        // let line_nums = line_numbers();
        // nums.append(&mut line_nums);

        view! {
            <div class="justify-center text-center col-span-1 grid-flow-row p-2">
                <div class=""> <br/> </div>
                <div class=""> <br/> </div>
                <div class=""> <br/> </div>
                { line_numbers }
            </div>
        }
    };

    let get_start = create_memo(move |_| tracker_state.get().display_start);

    let get_sequences = move || {
        // let start = get_start.get();
        // let _n_lines = state_size();

        // (start..start + 4)
        //     .into_iter()
        //     .map(|i| {
        //         view! {
        //             // <Sequence state=tracker_state set_state=set_tracker_state i=i/>
        //             <Sequence state=tracker_state i=i get_loc=location get_mode=mode set_loc=set_location get_storage=note_storage start_row=start_row/>
        //         }
        //     })
        //     .collect_view()
        view! {
            <For
                each=move || {
                    let start = get_start.get();

                    start..start + 4
                }
                key=move |i| *i
                children=move |i| {
                    view! {
                        <Sequence state=tracker_state i=i get_loc=location get_mode=mode set_loc=set_location get_storage=note_storage start_row=start_row/>
                    }
                }
            />
        }
    };

    let set_backend_note = move || {
        if let Some(note) = note_storage.get() {
            let midi_code = note.note;
            // let (start_loc, stop_loc) = if note.display_loc.0 < note.end_loc.0 {
            //     (note.display_loc, note.end_loc)
            // } else {
            //     (note.end_loc, note.display_loc)
            // };
            let start_loc = note.loc;
            let stop_loc = if note.n_lines > 0 {
                note.loc.0 + note.n_lines as usize
            } else {
                note.loc.0 - (note.n_lines * -1) as usize
            } + start_row.get();
            // let state = tracker_state.get();
            let channel = start_loc.1 / 6;
            let note_num = start_loc.1 % 6;

            let (start_row, stop_row) = if stop_loc < start_loc.0 {
                (stop_loc, start_loc.0)
            } else {
                (start_loc.0, stop_loc)
            };

            let args_play = AddNoteArgs {
                note: midi_code,
                vel: 64,
                channel: channel as ChannelIndex,
                note_number: note_num,
                start: start_row,
                stop: stop_row,
            };

            warn!("sending note to backend");

            spawn_local(async move {
                // warn!("adding note async block");
                invoke("add_note", to_value(&args_play).unwrap()).await;

                get_state();
            });
        }
    };

    let rm_backend_note = move || {
        // if let Some(note) = note_storage.get() {
        let loc = location.get();
        // let state = tracker_state.get();
        let channel = loc.1 / 6;
        let note_num = loc.1 % 6;

        let args_play = RmNoteArgs {
            channel: channel as ChannelIndex,
            note_number: note_num,
            row: loc.0 + start_row.get(),
        };

        warn!("removing note on backend");

        spawn_local(async move {
            // warn!("adding note async block");
            invoke("rm_note", to_value(&args_play).unwrap()).await;

            get_state();
        });
        // }
    };

    let set_display_note = move |note: Option<MidiNoteCmd>| {
        let loc = location.get();
        // let state = tracker_state.get();
        let channel = loc.1 / 6;
        let note_num = loc.1 % 6;

        set_tracker_state
            .update(|state| state.sequences[channel].data[loc.0].notes[note_num] = note);
    };

    let set_note = move |mut note_update_func: Box<dyn FnMut(MidiNote) -> MidiNote>| {
        // update note_storage
        set_note_storage.update(|storage| {
            if let Some(storage) = storage {
                storage.note = note_update_func(storage.note);
            }
        });
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
        // TODO: if in edit mode, add check if the new cell is already populated.
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
        // TODO: if in edit mode, add check if the new cell is already populated.
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
        if mode.get() == Mode::Move || mode.get() == Mode::Edit {
            cursor_up()
        }

        log!("w has been pressed");
    });

    use_hotkeys!(("keys") => move |_| {
        if mode.get() == Mode::Move || mode.get() == Mode::Edit {
            cursor_down()
        }

        log!("s has been pressed");
    });

    use_hotkeys!(("shiftleft+keyw") => move |_| {
        if mode.get() == Mode::Move || mode.get() == Mode::Edit {
            set_start_row.update(|row| if *row != 0 { *row = *row - 1 } else { *row = LINE_LEN - num_lines(font_size.get().into()) });

            get_state();
            cursor_down();
        }

        if mode.get() == Mode::Edit {
            set_note_storage.update(|storage|
                // if let Some(selected) = storage
                //     && !(selected.n_lines < 0
                //         && (selected.n_lines as usize > selected.loc.0 ))
                //     && !(selected.n_lines > 0
                //         && (selected.n_lines as usize > (LINE_LEN - selected.0)))
                // {
                //     (*storage).unwrap().n_lines -= 1
                // }
                if let Some(selected) = storage  {
                    (*storage).unwrap().n_lines = (location.get_untracked().0 as i64 + start_row.get() as i64) - selected.loc.0 as i64;
                }

            );
        }

        log!("shift w has been pressed");
    });

    use_hotkeys!(("shiftleft+keys") => move |_| {
        if mode.get() == Mode::Move || mode.get() == Mode::Edit {
            set_start_row.update(|row| if *row != LINE_LEN - num_lines(font_size.get().into()) { *row = *row + 1 } else { *row = 0 });

            get_state();
            cursor_up();
        }

        if mode.get() == Mode::Edit {
            set_note_storage.update(|storage| {
                    // if let Some(selected) = storage
                    //     && !(selected.n_lines < 0
                    //         && (selected.n_lines as usize > selected.loc.0 ))
                    //     && !(selected.n_lines > 0
                    //         && (selected.n_lines as usize > (LINE_LEN - selected.0)))
                    // {
                    //     (*storage).unwrap().n_lines -= 1
                    // }
                    if let Some(selected) = storage  {
                        log!("{} - {} = {}", location.get_untracked().0 as i64, selected.loc.0 as i64, location.get_untracked().0 as i64 - selected.loc.0 as i64);
                        (*storage).unwrap().n_lines = (location.get_untracked().0 as i64 + start_row.get() as i64) - selected.loc.0 as i64;

                    }
                }
            );
        }

        log!("shift s has been pressed");
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

    // use_hotkeys!(("keyw") => move |_| {
    //     if mode.get() == Mode::Edit {
    //         // set_note_storage.update(|storage| if let Some(selected) = storage && selected.end_loc.0 > 0 {
    //         //     selected.end_loc.0 -= 1
    //         // });
    //
    //         cursor_up();
    //     }
    // });
    //
    // use_hotkeys!(("keys") => move |_| {
    //     if mode.get() == Mode::Edit {
    //         // set_note_storage.update(|storage| if let Some(selected) = storage {
    //         //     selected.end_loc.0 += 1
    //         // });
    //
    //         cursor_down();
    //     }
    // });

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
                let loc = location.get_untracked();

                log!("settings scope to edit");
                // toggle_scope.call("edit".to_string());
                set_mode.set(Mode::Edit);
                set_note_storage.set(Some(NoteSetStorage { note: 0, loc: (loc.0 + start_row.get_untracked(), loc.1), n_lines: 1 }));
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

    // use_hotkeys!(("Backspace") => move |_| {
    use_hotkeys!(("Delete") => move |_| {
        log!("Delete key has been pressed");

        if mode.get() == Mode::Move {
            // remove note from backend
            rm_backend_note();
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

            </div>

            <div class="justify-center text-center grid grid-cols-12 h-fit max-h-fit">
                { get_line_nums }
                { get_sequences() }
                <div class="col-span-3 grid-flow-row p-2">
                    <div class=""> <br/> </div>
                    <div class=""> <br/> </div>
                    <SideCar set_playhead/>
                </div>
            </div>
        </main>
    }
}
