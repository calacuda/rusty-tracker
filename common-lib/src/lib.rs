use anyhow::{bail, ensure, Result};
use serde::{Deserialize, Serialize};
pub use synth_8080_lib::{notes::Note, Float};

pub type MidiNote = u8;
pub type CmdArg = u32;
pub type Cmd = char;
pub type ChannelIndex = u8;

#[derive(Serialize, Deserialize, Default, Clone, Debug, Copy, Eq, Hash, PartialEq)]
pub struct RowData {
    pub notes: [Option<MidiNote>; 3],
    pub cmds: [Option<(Cmd, Option<CmdArg>)>; 2],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrackerState {
    pub sequences: Vec<Vec<RowData>>,
    pub display_start: usize,
}

impl Default for TrackerState {
    fn default() -> Self {
        let def: Vec<RowData> = [RowData::default(); 0xFF].into_iter().collect();

        Self {
            sequences: [
                def.clone(),
                def.clone(),
                def.clone(),
                def.clone(),
                // def.clone(),
                // def.clone(),
                // def.clone(),
                // def.clone(),
                // def.clone(),
                // def.clone(),
                // def.clone(),
                // def.clone(),
                // def.clone(),
                // def.clone(),
                // def.clone(),
            ]
            .into_iter()
            .collect(),
            display_start: 0,
        }
    }
}

impl TrackerState {
    fn channel_len_check(&mut self, channel: ChannelIndex) -> Result<usize> {
        let channel = channel as usize;

        if channel >= self.sequences.len() {
            let mesg = format!("the channel {channel} does not exist.");
            // error!(mesg);
            bail!(mesg);
        }

        Ok(channel)
    }

    pub fn add_note(
        &mut self,
        note: MidiNote,
        channel: ChannelIndex,
        row: usize,
        note_num: usize,
    ) -> Result<()> {
        ensure!(note_num < 4, "lines can only have 4 notes per line");

        let channel = self.channel_len_check(channel)?;

        if self.sequences[channel].len() <= row {
            for sequence in self.sequences.iter_mut() {
                for _ in 0..row - sequence.len() {
                    sequence.push(RowData::default());
                }
            }
        }

        self.sequences[channel][row].notes[note_num] = Some(note);

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum PlaybackCmd {
    Play,
    Pause,
    Stop,
    Restart,
    SetCursor(usize),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PlaybackState {
    /// holds the current row
    Playing(usize),
    /// holds the row where playback is paused
    Paused(usize),
    NotPlaying,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MidiTarget {
    BuiltinSynth,
    MidiOut,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Channel {
    AllChannels,
    SomeChannels(Vec<ChannelIndex>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PlayerCmd {
    // PlayNote(MidiNote),
    // StopNote(MidiNote),
    // ExecCmd((Cmd, Option<CmdArg>)),
    VolumeSet((Float, ChannelIndex)),
    PausePlayback,
    ResumePlayback,
    StopPlayback,
    SetPlayingChannels(Channel),
    SetTarget(MidiTarget),
    SetCursor(usize),
}

pub fn get_cmd_arg_val(arg: CmdArg) -> usize {
    ((arg as Float / CmdArg::MAX as Float) * 100.0).round() as usize
}
