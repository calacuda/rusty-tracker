// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use anyhow::{bail, Result};
use crossbeam::channel::{unbounded, Receiver, Sender};
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::Poll,
    // thread,
    time::{Duration, Instant},
};
use synth_lib::{audio::TrackerSynth, init_synth, Note};
use tauri::{
    async_runtime::{spawn, JoinHandle},
    Manager, State, Window,
};
// use tauri_sys::window::current_window;
use tracing::*;
use tracker_lib::{
    Channel, ChannelIndex, Cmd, CmdArg, MidiNote, MidiNoteCmd, MidiTarget, PlaybackCmd,
    PlaybackState, PlayerCmd, TrackerState, Wavetable,
};

pub const MAX_COL_LEN: usize = 0xFFFF;
const NANO_MIN: u64 = 60_000_000_000;

struct IO {
    line_out: JoinHandle<()>,
    note_out: JoinHandle<()>,
}

// #[derive(Serialize, Deserialize, Clone)]
pub struct Player {
    /// describes the state of playback e.g. playing, paused, etc.
    state: PlaybackState,
    /// describes where the midi data should be sent.
    target: MidiTarget,
    /// used to describe which channels should be played. all not here are ignored during playback.
    channels: Channel,
    /// usedd to receive control commands from other threads.
    ipc: Receiver<PlayerCmd>,
    /// the state of the song the user has written.
    song: Arc<Mutex<TrackerState>>,
    /// the synth that is used when `self.target` is set to `MidiTarget::BuiltinSynth`.
    synth: Arc<Mutex<TrackerSynth>>,
    // /// time till next event in nano_seconds
    // ttne: Mutex<usize>,
    /// the instant that the last beat was processed
    last_event: Instant,
    /// the amount of time between beats
    beat_time: Duration,
    /// the tempo of playback
    tempo: u64,
    /// which beat describes the time between rows
    beat: u64,
    // window: Option<Window>,
    line_out: Sender<usize>,
    notes_out: Sender<(usize, Option<MidiNote>)>,
}

impl Player {
    pub fn new(
        song: Arc<Mutex<TrackerState>>,
        synth: Arc<Mutex<TrackerSynth>>,
    ) -> (
        Self,
        (
            Sender<PlayerCmd>,
            Receiver<usize>,
            Receiver<(usize, Option<MidiNote>)>,
        ),
    ) {
        let (tx, rx) = unbounded();
        let (line_tx, line_rx) = unbounded();
        let (note_tx, note_rx) = unbounded();
        let tempo = 110;
        let beat = 4;

        (
            Player {
                state: PlaybackState::NotPlaying,
                target: MidiTarget::BuiltinSynth,
                channels: Channel::AllChannels,
                ipc: rx,
                song,
                // ttne: Mutex::new(0),
                last_event: Instant::now(),
                beat_time: Duration::from_nanos(NANO_MIN / tempo / beat),
                synth,
                tempo,
                beat,
                line_out: line_tx,
                notes_out: note_tx,
            },
            (tx, line_rx, note_rx),
        )
    }

    fn send_note(&mut self, note: MidiNoteCmd, channel: usize) {
        // let note = Note::from(note);
        let (note, play) = match note {
            MidiNoteCmd::PlayNote(note) => (Note::from(note), true),
            MidiNoteCmd::StopNote(note) => (Note::from(note), false),
            MidiNoteCmd::HoldNote => return,
        };

        match self.target {
            MidiTarget::BuiltinSynth => {
                if play {
                    if let Err(e) = self.synth.lock().unwrap().play(note, channel) {
                        error!("the built in synth failed to play \"{note}\" on channel \"{channel}\". failed with error {e}.")
                    }
                } else {
                    if let Err(e) = self.synth.lock().unwrap().stop(note, channel) {
                        error!("the built in synth failed to play \"{note}\" on channel \"{channel}\". failed with error {e}.")
                    }
                }
            }
            _ => error!("not implemented yet"),
        }
    }

    fn send_cmd(&mut self, _command: (Cmd, Option<CmdArg>), _channel: usize) {
        // TODO: implement this

        // let note = Note::from(note);
        //
        // match self.target {
        //     MidiTarget::BuiltinSynth => {
        //         self.synth.lock().unwrap().play(note, channel);
        //     }
        //     _ => error!("not implemented yet"),
        // }

        warn!("commands not implemented yet");
    }

    fn recalc_beat_time(&mut self) {
        self.beat_time = Duration::from_nanos(NANO_MIN / self.tempo / self.beat);
    }

    fn set_tempo(&mut self, tempo: u64) {
        if tempo != self.tempo {
            self.tempo = tempo;
            self.recalc_beat_time();
        }
    }

    fn set_beat(&mut self, beat: u64) {
        if beat != self.beat {
            self.beat = beat;
            self.recalc_beat_time();
        }
    }
}

impl Future for Player {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let s = Pin::<&mut Player>::into_inner(self);

        // read from self.ipc and do the thing it says
        if let Ok(cmd_msg) = s.ipc.try_recv() {
            match cmd_msg {
                PlayerCmd::VolumeSet((vol, channel)) => {
                    if let Err(e) = s.synth.lock().unwrap().set_volume(vol, channel) {
                        error!("{e}");
                    }
                }
                PlayerCmd::SetPlayingChannels(channels) => s.channels = channels,
                PlayerCmd::SetTarget(target) => s.target = target,
                PlayerCmd::PausePlayback => match s.state {
                    PlaybackState::Playing(line_num) => s.state = PlaybackState::Paused(line_num),
                    PlaybackState::Paused(_) => error!("playback is already paused."),
                    PlaybackState::NotPlaying => error!("can't pause, not playing."),
                },
                PlayerCmd::ResumePlayback => match s.state {
                    PlaybackState::Playing(_) => error!("can't play while already playing."),
                    PlaybackState::Paused(line_num) => s.state = PlaybackState::Playing(line_num),
                    PlaybackState::NotPlaying => s.state = PlaybackState::Playing(0),
                },
                PlayerCmd::StopPlayback => {
                    if let PlaybackState::NotPlaying = s.state {
                        error!("can't stop playing while already not playing");
                    } else {
                        s.state = PlaybackState::NotPlaying;
                    }
                }
                PlayerCmd::SetCursor(loc) => match s.state {
                    PlaybackState::Playing(_) => s.state = PlaybackState::Playing(loc),
                    PlaybackState::Paused(_) => s.state = PlaybackState::Paused(loc),
                    PlaybackState::NotPlaying => {
                        error!("can't set cursor location when there is no cursor location to set.")
                    }
                },
                PlayerCmd::SetTempo(tempo) => s.set_tempo(tempo),
                PlayerCmd::SetBeat(beat) => s.set_beat(beat),
                PlayerCmd::SetWavetable((channel, Wavetable::BuiltIn(waveform_type))) => {
                    if let Err(e) = s.synth.lock().unwrap().set_waveform(channel, waveform_type) {
                        error!(
                            "atempt to set channel {channel}'s synth to waveform {waveform_type:?} resulted in error, {e}"
                        );
                    }
                }
                PlayerCmd::SetWavetable((_channel, Wavetable::FromFile(_table_file))) => {
                    // TODO: add loading of wave table from file.
                    todo!("load wave table from file")
                }
            }
        }

        if let PlaybackState::Playing(line_i) = s.state {
            // let mut last_event = s.last_event.lock().unwrap();

            if Instant::now().duration_since(s.last_event) >= s.beat_time {
                s.last_event = Instant::now();

                if let Err(e) = s.line_out.send(line_i) {
                    error!("could not send line num over internal crossbeam channel. incountered error: {e}");
                };

                // .clone()
                // .map(|window| window.emit_all("playhead", line_i).unwrap());

                s.state = PlaybackState::Playing(
                    (line_i + 1) % s.song.lock().unwrap().sequences[0].len(),
                );
                info!("playback state: {:0X}", line_i);

                let notes: Vec<(usize, Vec<MidiNoteCmd>)> = s
                    .song
                    .lock()
                    .unwrap()
                    .sequences
                    .iter()
                    .enumerate()
                    .map(|(i, sequence)| {
                        // if let Some(lines) = sequence {
                        let row_dat = sequence[line_i % sequence.len()];

                        if let Some(MidiNoteCmd::PlayNote(note)) = row_dat.notes[0] {
                            s.notes_out.send((i, Some(note)));
                        } else if let Some(MidiNoteCmd::StopNote(_)) = row_dat.notes[0] {
                            s.notes_out.send((i, None));
                        }

                        (
                            i,
                            row_dat.notes.into_iter().filter_map(|note| note).collect(),
                        )
                        // } else {
                        //     None
                        // }
                    })
                    .collect();

                let cmds: Vec<(usize, Vec<(Cmd, Option<CmdArg>)>)> = s
                    .song
                    .lock()
                    .unwrap()
                    .sequences
                    .iter()
                    .enumerate()
                    .map(|(i, sequence)| {
                        // if let Some(lines) = sequence {
                        let row_dat = sequence[line_i % sequence.len()];

                        (i, row_dat.cmds.into_iter().filter_map(|cmd| cmd).collect())
                        // } else {
                        //     None
                        // }
                    })
                    .collect();

                notes.into_iter().for_each(|(channel, notes)| {
                    notes
                        .into_iter()
                        .for_each(|note| s.send_note(note, channel))
                });

                cmds.into_iter().for_each(|(channel, cmds)| {
                    cmds.into_iter().for_each(|cmd| s.send_cmd(cmd, channel))
                });
            }
        }

        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

#[tauri::command(rename_all = "snake_case")]
fn send_midi(_synth: State<'_, Arc<Mutex<TrackerState>>>, _midi_cmd: Vec<u8>) {
    // synth.stop(note);
    warn!("sending of generic MIDI commands is not yet implemented");
}

#[tauri::command(rename_all = "snake_case")]
fn add_note(
    state: State<'_, Arc<Mutex<TrackerState>>>,
    note: MidiNote,
    channel: ChannelIndex,
    start: usize,
    stop: usize,
    note_number: usize,
) {
    // println!("inside add_note");
    if let Err(e) = state.lock().unwrap().add_note(
        Some(MidiNoteCmd::PlayNote(note)),
        channel,
        start,
        note_number,
    ) {
        error!("failed to add note: {note:?}, to channel {channel}, at row {start}. this process failed with error: {e}");
    }

    for i in (start + 1)..stop {
        if let Err(e) =
            state
                .lock()
                .unwrap()
                .add_note(Some(MidiNoteCmd::HoldNote), channel, i, note_number)
        {
            error!("failed to add note: {note:?}, to channel {channel}, at row {i}. this process failed with error: {e}");
        }
    }

    if let Err(e) = state.lock().unwrap().add_note(
        Some(MidiNoteCmd::StopNote(note)),
        channel,
        stop,
        note_number,
    ) {
        error!("failed to add note: {note:?}, to channel {channel}, at row {stop}. this process failed with error: {e}");
    }

    // else {
    //     info!("added note {note} successfully");
    // }
}

#[tauri::command(rename_all = "snake_case")]
fn rm_note(
    state: State<'_, Arc<Mutex<TrackerState>>>,
    channel: ChannelIndex,
    row: usize,
    note_number: usize,
) {
    // println!("inside add_note");
    if let Err(e) = state.lock().unwrap().rm_note(channel, row, note_number) {
        error!("failed to rm note on row {row}, from channel {channel}. this process failed with error: {e}");
    }
}

// #[tauri::command(rename_all = "snake_case")]
// fn set_play_head(
//     synth: State<'_, Arc<Mutex<Player>>>,
//     note: Note,
//     channel: Option<usize>,
//     row: usize,
// ) {
//     // warn!("setting the play head location on the back end is not yet implemented");
// }

async fn line_out(window: Window, line_rx: Receiver<usize>) {
    loop {
        // might not be nessesary
        while line_rx.len() > 1 {
            line_rx.recv().unwrap();
        }

        while let Ok(ln) = line_rx.recv() {
            window.emit_all("playhead", ln).unwrap();
        }
    }
}

async fn note_out(window: Window, note_rx: Receiver<(usize, Option<MidiNote>)>) {
    loop {
        // // might not be nessesary
        // while note_rx.len() > 1 {
        //     line_rx.recv().unwrap();
        // }

        while let Ok(note_dat) = note_rx.recv() {
            window.emit_all("note-change", note_dat).unwrap();
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
fn playback(
    // player: State<'_, Arc<Mutex<Player>>>,
    window: Window,
    player_ipc: State<'_, Arc<Mutex<Sender<PlayerCmd>>>>,
    io_threads: State<'_, Arc<Mutex<IO>>>,
    line_rx: State<'_, Receiver<usize>>,
    note_rx: State<'_, Receiver<(usize, Option<MidiNote>)>>,
    playback_cmd: PlaybackCmd,
) {
    // warn!("playback is not yet enabled on the back end is not yet implemented");
    match playback_cmd {
        PlaybackCmd::Play => {
            if let Err(e) = player_ipc.lock().unwrap().send(PlayerCmd::ResumePlayback) {
                error!("failed to play: {e}");
            } else {
                let line_rx = line_rx.inner().clone();
                io_threads.lock().unwrap().line_out = spawn(line_out(window.clone(), line_rx));

                let note_rx = note_rx.inner().clone();
                io_threads.lock().unwrap().note_out = spawn(note_out(window.clone(), note_rx));
            };
        }
        PlaybackCmd::Stop => {
            if let Err(e) = player_ipc.lock().unwrap().send(PlayerCmd::StopPlayback) {
                error!("failed to stop: {e}");
            } else {
                // set all join_handles back to nothing
                let mut threads = io_threads.lock().unwrap();

                (*threads).line_out = spawn(async move { () });
                (*threads).note_out = spawn(async move { () });
            };
        }
        PlaybackCmd::SetCursor(loc) => {
            if let Err(e) = player_ipc.lock().unwrap().send(PlayerCmd::SetCursor(loc)) {
                error!("failed to set cursor loction: {e}");
            }
        }
        _ => warn!("playback is not yet enabled on the back end is not yet implemented"),
    }
}

#[tauri::command(rename_all = "snake_case")]
fn set_tempo(player: State<'_, Arc<Mutex<Sender<PlayerCmd>>>>, tempo: u64) {
    if tempo > 1 {
        let _ = player.lock().unwrap().send(PlayerCmd::SetTempo(tempo));
    }
}

#[tauri::command(rename_all = "snake_case")]
fn set_beat(player: State<'_, Arc<Mutex<Sender<PlayerCmd>>>>, beat: u64) {
    if beat > 1 {
        let _ = player.lock().unwrap().send(PlayerCmd::SetBeat(beat));
    }
}

#[tauri::command(rename_all = "snake_case")]
fn get_state(
    window: Window,
    state: State<'_, Arc<Mutex<TrackerState>>>,
    start_row: usize,
    n_rows: usize,
) {
    let tracker_state = { state.lock().unwrap().copy_from_row(start_row, n_rows) };

    window.emit_all("state-change", tracker_state).unwrap();
}

// #[tokio::main(flavor = "multi_thread", worker_threads = 2)]
// async fn main() -> Result<()> {
fn main() -> Result<()> {
    let (synth, stream_handle, audio) = match init_synth() {
        Ok((synth, stream_handle, audio)) => (synth, stream_handle, audio),
        Err(e) => {
            error!("{e}");
            bail!("{e}");
        }
    };

    info!("starting audio stream");
    stream_handle.play_raw(audio).unwrap();
    info!("initializing tracker state");
    let state = Arc::new(Mutex::new(TrackerState::default()));

    info!("initializing player");
    let (player, (player_ipc, line_rx, note_rx)) = Player::new(state.clone(), synth.clone());
    let player_ipc = Arc::new(Mutex::new(player_ipc));
    let _midi_thread = spawn(player);
    let io = Arc::new(Mutex::new(IO {
        line_out: spawn(async move { () }),
        note_out: spawn(async move { () }),
    }));

    tauri::Builder::default()
        .manage(synth)
        // .manage(Arc::new(Mutex::new(player)))
        .manage(state)
        .manage(player_ipc)
        .manage(io)
        .manage(line_rx)
        .manage(note_rx)
        .invoke_handler(tauri::generate_handler![
            // play_note,
            // stop_note,
            send_midi, playback, add_note, get_state, rm_note, set_tempo, set_beat
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
