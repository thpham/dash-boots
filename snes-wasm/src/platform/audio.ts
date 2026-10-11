// WebAudio host: plays the cues emitted by the WASM core (see wasm/src/audio.rs).

const MUSIC_PLAY_BASE = 64;
const MUSIC_PAUSE = 253;
const MUSIC_RESUME = 254;
const MUSIC_STOP = 255;

const MUSIC_VOLUME = 0.45;
const SFX_VOLUME = 0.7;

export type AudioHost = {
  /** Call from a user gesture: browsers keep audio suspended until then. */
  unlock(): void;
  handle(codes: Uint8Array): void;
  /** Returns true when now muted. */
  toggleMute(): boolean;
};

export function createAudio(baseUrl: string, sfxNames: string[], musicNames: string[]): AudioHost {
  const Ctx =
    window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  if (!Ctx) {
    console.warn('WebAudio not supported: running silent');
    return { unlock() {}, handle() {}, toggleMute: () => true };
  }

  // Created early (suspended) so assets decode while the title screen shows.
  const ctx = new Ctx();
  const master = ctx.createGain();
  const musicBus = ctx.createGain();
  const sfxBus = ctx.createGain();
  musicBus.gain.value = MUSIC_VOLUME;
  sfxBus.gain.value = SFX_VOLUME;
  musicBus.connect(master);
  sfxBus.connect(master);
  master.connect(ctx.destination);

  const sfx = new Map<number, AudioBuffer>();
  const music = new Map<number, AudioBuffer>();

  async function load(url: string): Promise<AudioBuffer | null> {
    try {
      const res = await fetch(url);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      return await ctx.decodeAudioData(await res.arrayBuffer());
    } catch (err) {
      console.warn(`audio: failed to load ${url}`, err);
      return null;
    }
  }

  // Music state: the track that *should* play, so a cue that arrives before
  // decoding finishes (or while paused) is honoured later.
  let wanted: number | null = null;
  let paused = false;
  let source: AudioBufferSourceNode | null = null;
  let startedAt = 0;
  let offset = 0;

  function stopSource() {
    if (!source) return;
    try {
      source.stop();
    } catch {
      // already stopped
    }
    source.disconnect();
    source = null;
  }

  function startMusic() {
    stopSource();
    if (wanted === null || paused) return;
    const buf = music.get(wanted);
    if (!buf) return;
    source = ctx.createBufferSource();
    source.buffer = buf;
    source.loop = true;
    source.connect(musicBus);
    const at = offset % buf.duration;
    startedAt = ctx.currentTime - at;
    source.start(0, at);
  }

  sfxNames.forEach((name, id) => {
    void load(`${baseUrl}audio/sfx/${name}.wav`).then((b) => b && sfx.set(id, b));
  });
  musicNames.forEach((name, id) => {
    void load(`${baseUrl}audio/music/${name}.wav`).then((b) => {
      if (!b) return;
      music.set(id, b);
      if (wanted === id && !source) startMusic();
    });
  });

  function playSfx(id: number) {
    const buf = sfx.get(id);
    if (!buf || ctx.state !== 'running') return;
    const src = ctx.createBufferSource();
    src.buffer = buf;
    src.connect(sfxBus);
    src.start();
  }

  return {
    unlock() {
      if (ctx.state !== 'running') void ctx.resume();
    },
    handle(codes) {
      for (const code of codes) {
        if (code < MUSIC_PLAY_BASE) {
          playSfx(code);
        } else if (code === MUSIC_STOP) {
          wanted = null;
          offset = 0;
          stopSource();
        } else if (code === MUSIC_PAUSE) {
          if (source) offset = ctx.currentTime - startedAt;
          paused = true;
          stopSource();
        } else if (code === MUSIC_RESUME) {
          paused = false;
          startMusic();
        } else {
          wanted = code - MUSIC_PLAY_BASE;
          offset = 0;
          paused = false;
          startMusic();
        }
      }
    },
    toggleMute() {
      const mute = master.gain.value > 0;
      master.gain.value = mute ? 0 : 1;
      return mute;
    },
  };
}
