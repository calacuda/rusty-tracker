use crate::invoke;
use leptos::*;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use tracing::field::display;
use tracker_lib::{get_cmd_arg_val, ChannelIndex, Cmd, CmdArg, MidiNote, RowData, TrackerState};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
struct AddNoteArgs {
    note: MidiNote,
    channel: ChannelIndex,
    row: usize,
    note_number: usize,
}

#[component]
pub fn Sequence(
    state: ReadSignal<TrackerState>,
    // set_state: WriteSignal<TrackerState>,
    i: usize,
) -> impl IntoView {
    let tmp_state = state.get_untracked().sequences[i][0];
    let n_notes = tmp_state.notes.len();
    let n_cmds = tmp_state.cmds.len();

    view! {
        <div class="col-span-2 grid-flow-row p-2">
            <SequenceHeader i=i n_notes=n_notes n_cmds=n_cmds/>
            <For
                each=move || state.get().sequences[i].clone().into_iter().enumerate()
                key=|row_elm| row_elm.clone()
                children=move |(row_i, dat)| {

                    view! {
                        <SequenceRow dat=dat sequence_i=i row_i=row_i/>
                    }
                }
            />
        </div>
    }
}

fn note_to_name(midi_note: MidiNote) -> String {
    let note_name_i = midi_note % 12;
    let octave = midi_note / 12;

    let note_names = [
        "C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-", "B#",
    ];
    let note_name = note_names[note_name_i as usize];

    format!("{note_name}{octave:X}")
}

fn cmd_to_display(cmd: Cmd, arg: Option<CmdArg>) -> String {
    // TODO: display only the first two decimal points of arg.
    format!(
        "{}{}",
        cmd,
        match arg {
            Some(val) => format!("{:02X}", get_cmd_arg_val(val)),
            None => "--".to_string(),
        }
    )
}

#[component]
pub fn SequenceRow(sequence_i: usize, row_i: usize, dat: RowData) -> impl IntoView {
    view! {
        <div class="grid grid-flow-col">
            <For
                each=move || dat.notes.into_iter().enumerate()
                key=|note| note.clone()
                children=move |(i, note)| {
                    view! {
                        <NoteDisplay note=note sequence_i=sequence_i row_i=row_i note_num=i/>
                    }
                }
            />
            <For
                each=move || dat.cmds.into_iter().enumerate()
                key=|cmd| cmd.clone()
                children=move |(_i, cmd)| {
                    let display = match cmd {
                        Some((name, arg)) => cmd_to_display(name, arg),
                        None => "---".to_string(),
                    };

                    view! {
                        <p> { display } </p>
                    }
                }
            />
        </div>

    }
}

#[component]
pub fn SequenceHeader(i: usize, n_notes: usize, n_cmds: usize) -> impl IntoView {
    view! {
        <div class="">
            <div class="">
                { format!("Track => {i}") }
            </div>

            <div class="grid grid-flow-col">
                <For
                    each = move || (0..n_notes)
                    key = |n| *n
                    children = move |n| {
                        view! {
                            <div>
                                { format!("N-{n}") }
                            </div>
                        }
                    }
                />
                <For
                    each = move || (0..n_cmds)
                    key = |n| *n
                    children = move |n| {
                        view! {
                            <div>
                                { format!("C-{n}") }
                            </div>                        }
                    }
                />
            </div>
        </div>
    }
}

#[component]
fn NoteDisplay(
    note: Option<MidiNote>,
    sequence_i: usize,
    row_i: usize,
    note_num: usize,
) -> impl IntoView {
    let null_str = "---";

    let (get_note, set_note) = create_signal(note);
    // let (focus, set_note) = create_signal(note);

    let display = move || match get_note.get() {
        Some(name) => note_to_name(name),
        None => null_str.to_string(),
    };

    let send_note_to_backend = move || {
        let channel = sequence_i as ChannelIndex;

        if let Some(note) = get_note.get() {
            let args = AddNoteArgs {
                note,
                channel,
                note_number: note_num,
                row: row_i,
            };

            // logging::warn!("sending note to backend");

            spawn_local(async move {
                // logging::warn!("inside async block");
                invoke("add_note", to_value(&args).unwrap()).await;
            });
        } else {
            logging::warn!("not sending note to backend");
        }
    };

    let up_semi = move || {
        match get_note.get() {
            Some(midi_note) => set_note.set(Some((midi_note + 1) % 128)),
            None => set_note.set(Some(0)),
        }

        send_note_to_backend();
    };

    let up_octave = move || {
        match get_note.get() {
            Some(midi_note) => set_note.set(Some((midi_note + 12) % 128)),
            None => set_note.set(Some(0)),
        }

        send_note_to_backend();
    };

    let down_semi = move || {
        match get_note.get() {
            Some(midi_note) => set_note.set(Some((midi_note as i32 - 1) as MidiNote % 128)),
            None => set_note.set(Some(0)),
        }

        send_note_to_backend();
    };

    let down_octave = move || {
        match get_note.get() {
            Some(midi_note) => set_note.set(Some((midi_note as i32 - 12) as MidiNote % 128)),
            None => set_note.set(Some(0)),
        }

        send_note_to_backend();
    };

    view! {
        <button
            // type="text"
            // on:input=move |ev| {
            //     ev.prevent_default();
            // }

            on:keydown=move |ev| {
                ev.prevent_default();
            }

            on:keyup=move |ev| {
                ev.prevent_default();

                // logging::warn!("event => {:?}", ev.code());

                match ev.code().as_str() {
                    "ArrowLeft" => down_octave(),
                    "ArrowRight" => up_octave(),
                    "ArrowDown" => down_semi(),
                    "ArrowUp" => up_semi(),
                    _ => {}
                }
            }

            // prop:value=move || {
            //     match get_note.get() {
            //         Some(midi_note) => note_to_name(midi_note),
            //         None => null_str.to_string(),
            //     }
            // }
        >
            { display }
        </button>
    }
}
