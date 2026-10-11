//! The 4 campaign tracks, embedded in the WASM. These demoscene modules come
//! from an archive with no declared license; embedded at the maintainer's
//! request, with full attribution here AND in snes-wasm/README.md.

pub struct MusicTrack {
    pub title: &'static str,
    pub author: &'static str,
    pub license: &'static str,
    /// Origin page of the file (attribution viewable online).
    pub source: &'static str,
    pub data: &'static [u8],
}

pub static TRACKS: [MusicTrack; 4] = [
    MusicTrack {
        title: "SMB3",
        author: "jellydog",
        license: "unlicensed (demoscene archive)",
        source: "https://github.com/thunder-red-star/xm-files-for-player/blob/main/jellydog-super_mario_bros3.xm",
        data: include_bytes!("../../assets/level1.xm"),
    },
    MusicTrack {
        title: "Summer Memories #3",
        author: "Arachno & Dualtrax",
        license: "unlicensed (demoscene archive)",
        source: "https://github.com/thunder-red-star/xm-files-for-player/blob/main/arachno%26dualtrax-summer_memories3.xm",
        data: include_bytes!("../../assets/level2.xm"),
    },
    MusicTrack {
        title: "Shock Therapy",
        author: "Chromag",
        license: "unlicensed (demoscene archive)",
        source: "https://github.com/thunder-red-star/xm-files-for-player/blob/main/chromag-shock_therapy23.xm",
        data: include_bytes!("../../assets/level3.xm"),
    },
    MusicTrack {
        title: "Bar Rocks",
        author: "Cerror",
        license: "unlicensed (demoscene archive)",
        source: "https://github.com/thunder-red-star/xm-files-for-player/blob/main/cerror-bar_rocks.xm",
        data: include_bytes!("../../assets/level4.xm"),
    },
];
