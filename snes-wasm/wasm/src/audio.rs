//! Audio cues emitted by the simulation; the host maps them to sounds.
//!
//! Encoding of the event byte stream:
//! - `0..64`: play SFX with that id (`Sfx as u8`)
//! - `MUSIC_PLAY_BASE + id`: start looping music track `id`
//! - `MUSIC_PAUSE`, `MUSIC_RESUME`, `MUSIC_STOP`
//! - `MUSIC_FUTURE_ON`, `MUSIC_FUTURE_OFF`: toggle the future music filter

/// Host-swap signal: the player pressed Start/Jump on the clear screen; the
/// browser host decides what happens next (load the next level, or replay).
pub const LEVEL_CLEAR: u8 = 250;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Sfx {
    Jump = 0,
    Dash,
    Coin,
    Stomp,
    Hurt,
    Death,
    Powerup,
    Checkpoint,
    Clear,
    Bump,
    Pause,
    Rift,
    Return,
}

/// File stems, indexed by `Sfx as usize` (`audio/sfx/<name>.wav`).
pub const SFX_NAMES: [&str; 13] = [
    "jump",
    "dash",
    "coin",
    "stomp",
    "hurt",
    "death",
    "powerup",
    "checkpoint",
    "clear",
    "bump",
    "pause",
    "rift",
    "return",
];

/// File stems, indexed by music id (`audio/music/<name>.wav`).
pub const MUSIC_NAMES: [&str; 4] = ["level1", "level2", "level3", "level4"];

pub const MUSIC_PLAY_BASE: u8 = 64;
pub const MUSIC_FUTURE_ON: u8 = 251;
pub const MUSIC_FUTURE_OFF: u8 = 252;
pub const MUSIC_PAUSE: u8 = 253;
pub const MUSIC_RESUME: u8 = 254;
pub const MUSIC_STOP: u8 = 255;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioEvent {
    Sfx(Sfx),
    PlayMusic(u8),
    PauseMusic,
    ResumeMusic,
    StopMusic,
    MusicFuture(bool),
    LevelClear,
}

impl AudioEvent {
    pub fn code(self) -> u8 {
        match self {
            AudioEvent::Sfx(s) => s as u8,
            AudioEvent::PlayMusic(id) => MUSIC_PLAY_BASE + id,
            AudioEvent::PauseMusic => MUSIC_PAUSE,
            AudioEvent::ResumeMusic => MUSIC_RESUME,
            AudioEvent::StopMusic => MUSIC_STOP,
            AudioEvent::LevelClear => LEVEL_CLEAR,
            AudioEvent::MusicFuture(on) => {
                if on {
                    MUSIC_FUTURE_ON
                } else {
                    MUSIC_FUTURE_OFF
                }
            }
        }
    }
}
