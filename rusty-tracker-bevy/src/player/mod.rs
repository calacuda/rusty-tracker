use crate::{state::AppState, PlayerSynth};
use bevy::prelude::*;
use crossbeam::channel::{unbounded, Receiver, Sender};
use std::{
    ops::DerefMut,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use synth_lib::{audio::TrackerSynth, Note};
use tracker_lib::Float;
use tracker_lib::{
    Channel, Cmd, CmdArg, MidiNoteCmd, MidiTarget, PlaybackState, PlayerCmd, TrackerState,
};

pub struct SetupPlugin;

impl Plugin for SetupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, step_player.run_if(in_state(AppState::Main)));
    }
}

fn step_player(mut player: ResMut<PlayerSynth>, mut state: ResMut<TrackerState>) {
    player.0.poll(state.deref_mut());
}

pub struct Player {
    /// describes the state of playback e.g. playing, paused, etc.
    state: PlaybackState,
    /// describes where the midi data should be sent.
    target: MidiTarget,
    /// used to describe which channels should be played. all not here are ignored during playback.
    channels: Channel,
    /// usedd to receive control commands from other threads.
    ipc: Receiver<PlayerCmd>,
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
        // song: Arc<Mutex<TrackerState>>,
        synth: Arc<Mutex<TrackerSynth>>,
    ) -> (Self, Sender<PlayerCmd>) {
        let (tx, rx) = unbounded();

        (
            Player {
                state: PlaybackState::NotPlaying,
                target: MidiTarget::BuiltinSynth,
                channels: Channel::AllChannels,
                ipc: rx,
                // song,
                // ttne: Mutex::new(0),
                last_event: Instant::now(),
                beat_time: Duration::from_nanos((1_000_000_000.0 as Float / (110.0)).round() as u64),
                synth,
            },
            tx,
        )
    }

    fn send_note(&mut self, note: MidiNoteCmd, channel: usize) {
        // let note = Note::from(note);
        let (note, play) = match note {
            MidiNoteCmd::PlayNote(note) => (Note::from(note), true),
            MidiNoteCmd::StopNote(note) => (Note::from(note), false),
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

    fn poll(&mut self, song: &mut TrackerState) {
        // let s = Pin::<&mut Player>::into_inner(self);

        // TODO: read from self.ipc and do the thing it says
        if let Ok(cmd_msg) = self.ipc.try_recv() {
            match cmd_msg {
                PlayerCmd::VolumeSet((vol, channel)) => {
                    if let Err(e) = self.synth.lock().unwrap().set_volume(vol, channel) {
                        error!("{e}");
                    }
                }
                PlayerCmd::SetPlayingChannels(channels) => self.channels = channels,
                PlayerCmd::SetTarget(target) => self.target = target,
                PlayerCmd::PausePlayback => match self.state {
                    PlaybackState::Playing(line_num) => {
                        self.state = PlaybackState::Paused(line_num)
                    }
                    PlaybackState::Paused(_) => error!("playback is already paused."),
                    PlaybackState::NotPlaying => error!("can't pause, not playing."),
                },
                PlayerCmd::ResumePlayback => match self.state {
                    PlaybackState::Playing(_) => error!("can't play while already playing."),
                    PlaybackState::Paused(line_num) => {
                        self.state = PlaybackState::Playing(line_num)
                    }
                    PlaybackState::NotPlaying => self.state = PlaybackState::Playing(0),
                },
                PlayerCmd::StopPlayback => {
                    if let PlaybackState::NotPlaying = self.state {
                        error!("can't stop playing while already not playing");
                    } else {
                        self.state = PlaybackState::NotPlaying;
                    }
                }
                PlayerCmd::SetCursor(loc) => match self.state {
                    PlaybackState::Playing(_) => self.state = PlaybackState::Playing(loc),
                    PlaybackState::Paused(_) => self.state = PlaybackState::Paused(loc),
                    PlaybackState::NotPlaying => {
                        error!("can't set cursor location when there is no cursor location to set.")
                    }
                },
            }
        }

        if let PlaybackState::Playing(line_i) = self.state {
            // let mut last_event = s.last_event.lock().unwrap();

            if Instant::now().duration_since(self.last_event) >= self.beat_time {
                self.last_event = Instant::now();
                self.state = PlaybackState::Playing((line_i + 1) % song.sequences[0].len());
                info!("playback state: {:0X}", line_i);

                let notes: Vec<(usize, Vec<MidiNoteCmd>)> = song
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

                let cmds: Vec<(usize, Vec<(Cmd, Option<CmdArg>)>)> = song
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
                        .for_each(|note| self.send_note(note, channel))
                });

                cmds.into_iter().for_each(|(channel, cmds)| {
                    cmds.into_iter().for_each(|cmd| self.send_cmd(cmd, channel))
                });
            }
        }
    }
}
