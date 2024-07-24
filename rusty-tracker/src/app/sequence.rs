use crate::{Mode, NoteSetStorage};
use leptos::{logging::*, *};
use tracker_lib::{get_cmd_arg_val, Cmd, CmdArg, MidiNoteCmd, RowData, TrackerState};

#[component]
pub fn Sequence(
    state: ReadSignal<TrackerState>,
    i: usize,
    get_loc: ReadSignal<(usize, usize)>,
    get_storage: ReadSignal<Option<NoteSetStorage>>,
    set_loc: WriteSignal<(usize, usize)>,
    get_mode: ReadSignal<Mode>,
    start_row: ReadSignal<usize>,
) -> impl IntoView {
    if !state.get_untracked().sequences[i].is_empty() {
        let tmp_state = state.get_untracked().sequences[i][0];
        let n_notes = tmp_state.notes.len();
        let n_cmds = tmp_state.cmds.len();

        let get_sequence = create_memo(move |_| state.get().sequences[i].to_owned());

        let row_memo = create_memo(move |_| {
            let memos: Vec<(usize, Memo<RowData>)> = get_sequence
                .get()
                .into_iter()
                .enumerate()
                .map(|(row_i, _)| (row_i, create_memo(move |_| get_sequence.get()[row_i])))
                .collect();

            memos
        });

        view! {
            <div class="col-span-2 grid-flow-row p-2">
                <SequenceHeader i=i n_notes=n_notes n_cmds=n_cmds/>
                <For
                    each=move || row_memo.get()
                    key=|mem| (mem.0, mem.1.get())
                    children=move |(row_i, memo)| {
                        log!("generating row {row_i:0X} from sequence {i}");

                        view! {
                            <SequenceRow
                                dat=memo
                                sequence_i=i
                                row_i=row_i
                                get_loc=get_loc
                                get_mode=get_mode
                                set_loc=set_loc
                                get_storage=get_storage
                                start_row=start_row
                            />
                        }
                    }
                />
            </div>
        }
    } else {
        view! {
            <div class="col-span-2 grid-flow-row p-2">
            </div>
        }
    }
}

fn note_to_name(midi_note: MidiNoteCmd) -> String {
    let midi_note = match midi_note {
        MidiNoteCmd::PlayNote(note) => note,
        MidiNoteCmd::StopNote(note) => note,
        MidiNoteCmd::HoldNote => return "|||".into(),
    };

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
pub fn SequenceRow(
    sequence_i: usize,
    row_i: usize,
    dat: Memo<RowData>,
    get_loc: ReadSignal<(usize, usize)>,
    get_storage: ReadSignal<Option<NoteSetStorage>>,
    set_loc: WriteSignal<(usize, usize)>,
    get_mode: ReadSignal<Mode>,
    start_row: ReadSignal<usize>,
) -> impl IntoView {
    view! {
        <div class="grid grid-flow-col">
            <For
                each=move || dat.get().notes.into_iter().enumerate()
                key=|note| note.clone()
                children=move |(i, note)| {
                    view! {
                        <NoteDisplay
                            note=note
                            sequence_i=sequence_i
                            row_i=row_i
                            note_num=i
                            get_loc=get_loc
                            get_mode=get_mode
                            set_loc=set_loc
                            get_storage=get_storage
                            start_row=start_row
                        />
                    }
                }
            />
            <For
                each=move || dat.get().cmds.into_iter().enumerate()
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
    let note_headers = (0..n_notes)
        .map(|n| {
            view! {
                <div>
                    { format!("N-{n}") }
                </div>
            }
        })
        .collect_view();
    let cmd_headers = (0..n_cmds)
        .map(|n| {
            view! {
                <div>
                    { format!("C-{n}") }
                </div>
            }
        })
        .collect_view();

    view! {
        <div class="">
            <div class="">
                { format!("Track => {i}") }
            </div>

            <div class="grid grid-flow-col">
                { note_headers }
                { cmd_headers }
            </div>
        </div>
    }
}

#[component]
fn NoteDisplay(
    note: Option<MidiNoteCmd>,
    sequence_i: usize,
    row_i: usize,
    note_num: usize,
    get_loc: ReadSignal<(usize, usize)>,
    get_storage: ReadSignal<Option<NoteSetStorage>>,
    set_loc: WriteSignal<(usize, usize)>,
    get_mode: ReadSignal<Mode>,
    start_row: ReadSignal<usize>,
) -> impl IntoView {
    let null_str = "---";

    let display = move || {
        let this_loc = (row_i, (6 * sequence_i) + note_num);

        if this_loc == get_loc.get() && get_mode.get() == Mode::Edit {
            note_to_name(MidiNoteCmd::PlayNote(
                get_storage.get().unwrap_or(NoteSetStorage::default()).note,
            ))
        } else {
            match note {
                Some(name) => note_to_name(name),
                None => null_str.to_string(),
            }
        }
    };

    let class = move || {
        let this_loc = (row_i, (6 * sequence_i) + note_num);

        if this_loc == get_loc.get()
            && (get_mode.get() == Mode::Move || get_mode.get() == Mode::Edit)
        {
            "bg-sapphire"
        } else if let Some(store) = get_storage.get()
            && get_mode.get() == Mode::Edit
            && store.display_loc.1 == this_loc.1
            && ((this_loc.0 + start_row.get() <= store.display_loc.0
                && this_loc.0 >= store.end_loc.0)
                || (this_loc.0 + start_row.get() >= store.display_loc.0
                    && this_loc.0 <= store.end_loc.0))
        {
            // log!("making box green. {:?} - {:?}", this_loc, store);
            "bg-green"
        } else {
            ""
        }
    };

    view! {
        <button
            on:click=move |ev| {
                ev.prevent_default();

                set_loc.set((row_i, (6 * sequence_i) + note_num));
            }
            class=class
        >
            { display }
        </button>
    }
}
