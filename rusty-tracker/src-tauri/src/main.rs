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
use tauri::{async_runtime::spawn, State};
use tracing::*;
use tracker_lib::{
    Channel, ChannelIndex, Cmd, CmdArg, Float, MidiNote, MidiTarget, PlaybackCmd, PlaybackState,
    PlayerCmd, TrackerState,
};

pub const MAX_COL_LEN: usize = 0xFFFF;

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
    /// time till next event in nano_seconds
    // ttne: Mutex<usize>,
    /// the instant that the last beat was processed
    last_event: Instant,
    /// the amount of time between beats
    beat_time: Duration,
}

impl Player {
    pub fn new(
        song: Arc<Mutex<TrackerState>>,
        synth: Arc<Mutex<TrackerSynth>>,
    ) -> (Self, Sender<PlayerCmd>) {
        let (tx, rx) = unbounded();

        (
            Player {
                state: PlaybackState::NotPlaying,
                target: MidiTarget::BuiltinSynth,
                channels: Channel::AllChannels,
                ipc: rx,
                song,
                // ttne: Mutex::new(0),
                last_event: Instant::now(),
                beat_time: Duration::from_nanos((1_000_000_000.0 as Float / (110.0)).round() as u64),
                synth,
            },
            tx,
        )
    }

    fn send_note(&mut self, note: MidiNote, channel: usize) {
        let note = Note::from(note);

        match self.target {
            MidiTarget::BuiltinSynth => {
                if let Err(e) = self.synth.lock().unwrap().play(note, channel) {
                    error!("the built in synth failed to play \"{note}\" on channel \"{channel}\". failed with error {e}.")
                }
            }
            _ => error!("not implemented yet"),
        }
    }

    fn send_cmd(&mut self, _command: (Cmd, Option<CmdArg>), _channel: usize) {
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
}

impl Future for Player {
    type Output = ();

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let s = Pin::<&mut Player>::into_inner(self);

        // TODO: read from self.ipc and do the thing it says
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
            }
        }

        if let PlaybackState::Playing(line_i) = s.state {
            // let mut last_event = s.last_event.lock().unwrap();

            if Instant::now().duration_since(s.last_event) >= s.beat_time {
                s.last_event = Instant::now();
                s.state = PlaybackState::Playing(line_i + 1);
                info!("playback state: {:?}", s.state);

                let notes: Vec<(usize, Vec<MidiNote>)> = s
                    .song
                    .lock()
                    .unwrap()
                    .sequences
                    .iter()
                    .enumerate()
                    .map(|(i, sequence)| {
                        // if let Some(lines) = sequence {
                        let row_dat = sequence[line_i % sequence.len()];

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

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
// #[tauri::command(rename_all = "snake_case")]
// fn play_note(synth: State<'_, Arc<Mutex<TrackerSynth>>>, note: Note, channel: usize) {
//     if let Err(_e) = synth.lock().unwrap().play(note, channel) {
//         // TODO: send window event with error message
//     }
// }
//
// #[tauri::command(rename_all = "snake_case")]
// fn stop_note(synth: State<'_, Arc<Mutex<TrackerSynth>>>, note: Note, channel: usize) {
//     if let Err(_e) = synth.lock().unwrap().stop(note, channel) {
//         // TODO: send window event with error message
//     }
// }

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
    row: usize,
    note_number: usize,
) {
    // println!("inside add_note");
    if let Err(e) = state
        .lock()
        .unwrap()
        .add_note(note, channel, row, note_number)
    {
        error!("failed to add {note}, to channel {channel}, at location {row}. this process failed with error: {e}");
    }
    // else {
    //     info!("added note {note} successfully");
    // }
}

#[tauri::command(rename_all = "snake_case")]
fn set_play_head(
    synth: State<'_, Arc<Mutex<Player>>>,
    note: Note,
    channel: Option<usize>,
    row: usize,
) {
    // warn!("setting the play head location on the back end is not yet implemented");
}

#[tauri::command(rename_all = "snake_case")]
fn playback(
    // player: State<'_, Arc<Mutex<Player>>>,
    player_ipc: State<'_, Arc<Mutex<Sender<PlayerCmd>>>>,
    playback_cmd: PlaybackCmd,
) {
    // warn!("playback is not yet enabled on the back end is not yet implemented");
    match playback_cmd {
        PlaybackCmd::Play => {
            if let Err(e) = player_ipc.lock().unwrap().send(PlayerCmd::ResumePlayback) {
                error!("failed to play: {e}");
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

    // let _audio_gen_thread = spawn(a_io);
    // spawn(async move {
    //     stream_handle.play_raw(audio).unwrap();
    // });

    info!("starting audio stream");
    stream_handle.play_raw(audio).unwrap();
    let state = Arc::new(Mutex::new(TrackerState::default()));

    let (player, player_ipc) = Player::new(state.clone(), synth.clone());
    let _midi_thread = spawn(player);

    tauri::Builder::default()
        .manage(synth)
        // .manage(Arc::new(Mutex::new(player)))
        .manage(state)
        .manage(Arc::new(Mutex::new(player_ipc)))
        .invoke_handler(tauri::generate_handler![
            // play_note,
            // stop_note,
            send_midi,
            set_play_head,
            playback,
            add_note
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
