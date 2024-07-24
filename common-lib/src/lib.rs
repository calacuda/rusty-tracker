use std::path::PathBuf;

use anyhow::{bail, ensure, Result};
use serde::{Deserialize, Serialize};
use synth_8080_lib::OscType;
pub use synth_8080_lib::{notes::Note, Float};

#[cfg(feature = "bevy")]
use bevy::prelude::*;

pub type MidiNote = u8;
pub type CmdArg = u32;
pub type Cmd = char;
pub type ChannelIndex = u8;

pub const LINE_LEN: usize = 0xFFFF;

#[cfg_attr(feature = "bevy", derive(Resource))]
#[derive(Serialize, Deserialize, Clone, Debug, Copy, Eq, Hash, PartialEq)]
pub enum MidiNoteCmd {
    PlayNote(MidiNote),
    StopNote(MidiNote),
    HoldNote,
}

#[cfg_attr(feature = "bevy", derive(Resource))]
#[derive(Serialize, Deserialize, Default, Clone, Debug, Copy, Eq, Hash, PartialEq)]
pub struct RowData {
    pub notes: [Option<MidiNoteCmd>; 4],
    pub cmds: [Option<(Cmd, Option<CmdArg>)>; 2],
}

#[cfg_attr(feature = "bevy", derive(Resource))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrackerState {
    pub sequences: Vec<Vec<RowData>>,
    pub display_start: usize,
}

impl Default for TrackerState {
    fn default() -> Self {
        let mut def: Vec<RowData> = Vec::with_capacity(LINE_LEN);

        (0..LINE_LEN)
            .into_iter()
            .for_each(|_| def.push(RowData::default()));

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
        note: Option<MidiNoteCmd>,
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

        self.sequences[channel][row].notes[note_num] = note;

        Ok(())
    }

    pub fn rm_note(&mut self, channel: ChannelIndex, row: usize, note_num: usize) -> Result<()> {
        ensure!(note_num < 4, "lines can only have 4 notes per line");

        let channel = self.channel_len_check(channel)?;

        if self.sequences[channel].len() <= row {
            for sequence in self.sequences.iter_mut() {
                for _ in 0..row - sequence.len() {
                    sequence.push(RowData::default());
                }
            }
        }

        // self.sequences[channel][i].notes[note_num]
        let mut i = row;

        while Some(MidiNoteCmd::HoldNote) == self.sequences[channel][i].notes[note_num] || i == row
        {
            self.sequences[channel][i].notes[note_num] = None;

            i += 1;
        }

        self.sequences[channel][i].notes[note_num] = None;

        if row > 0 {
            let mut i = row - 1;

            while Some(MidiNoteCmd::HoldNote) == self.sequences[channel][i].notes[note_num]
                || i == row - 1
            {
                self.sequences[channel][i].notes[note_num] = None;

                if i == 0 {
                    break;
                }

                i -= 1;
            }

            self.sequences[channel][i].notes[note_num] = None;
        }

        Ok(())
    }

    pub fn empty() -> Self {
        let def: Vec<RowData> = vec![RowData::default()];

        Self {
            sequences: [
                def.clone(),
                def.clone(),
                def.clone(),
                def.clone(),
                // def.clone(),
            ]
            .into_iter()
            .collect(),
            display_start: 0,
        }
    }

    pub fn copy_from_row(&self, row: usize, n_rows: usize) -> Self {
        Self {
            display_start: self.display_start,
            sequences: vec![
                self.sequences[0][row..row + n_rows].to_vec(),
                self.sequences[1][row..row + n_rows].to_vec(),
                self.sequences[2][row..row + n_rows].to_vec(),
                self.sequences[3][row..row + n_rows].to_vec(),
            ],
        }
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
pub enum Wavetable {
    BuiltIn(OscType),
    FromFile(PathBuf),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PlayerCmd {
    // PlayNote(MidiNote),
    // StopNote(MidiNote),
    // ExecCmd((Cmd, Option<CmdArg>)),
    VolumeSet((Float, Option<ChannelIndex>)),
    PausePlayback,
    ResumePlayback,
    StopPlayback,
    SetPlayingChannels(Channel),
    SetTarget(MidiTarget),
    SetCursor(usize),
    SetTempo(u64),
    SetBeat(u64),
    SetWavetable((ChannelIndex, Wavetable)),
}

pub fn get_cmd_arg_val(arg: CmdArg) -> usize {
    ((arg as Float / CmdArg::MAX as Float) * 100.0).round() as usize
}
