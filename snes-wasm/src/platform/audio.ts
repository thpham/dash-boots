// WebAudio host: plays the cues emitted by the WASM core (see wasm/src/audio.rs).
// Music is rendered incrementally by a wasm MusicRenderTask pumped on a timer —
// never synchronously (a whole-track render is seconds of CPU; review Critical 1).

import { MusicRenderTask } from '../../wasm/pkg/snes_wasm';

const MUSIC_PLAY_BASE = 64;
const MUSIC_FUTURE_ON = 251;
const MUSIC_FUTURE_OFF = 252;
const MUSIC_PAUSE = 253;
const MUSIC_RESUME = 254;
const MUSIC_STOP = 255;

const MUSIC_VOLUME = 0.85;
const SFX_VOLUME = 0.2;
/** -/+ key step, matched in main.ts. */
export const VOLUME_STEP = 0.05;
const VOLUME_KEY = 'dash-boots-volume';

type Volumes = { music: number; sfx: number };

/** Persisted so a refresh keeps the levels the player settled on. */
function loadVolumes(): Volumes {
  try {
    const raw = JSON.parse(localStorage.getItem(VOLUME_KEY) ?? '') as Partial<Volumes>;
    if (
      typeof raw.music === 'number' &&
      typeof raw.sfx === 'number' &&
      raw.music >= 0 &&
      raw.music <= 1 &&
      raw.sfx >= 0 &&
      raw.sfx <= 1
    ) {
      return { music: raw.music, sfx: raw.sfx };
    }
  } catch {
    // Storage unavailable: defaults.
  }
  return { music: MUSIC_VOLUME, sfx: SFX_VOLUME };
}

export type AudioHost = {
  /** Call from a user gesture: browsers keep audio suspended until then. */
  unlock(): void;
  handle(codes: Uint8Array): void;
  /** Returns true when now muted. */
  toggleMute(): boolean;
  /** Shifts the music bus volume by `delta` (clamped 0..1). New value back. */
  adjustMusic(delta: number): number;
  /** Same for the effects bus. */
  adjustSfx(delta: number): number;
  /** Pre-renders a track in the background (cached, played when wanted). */
  prerenderMusic(id: number): void;
  /** Drops a level's track from the cache. Campaign flow only: the call
   * sites must know the level is finished and never replayed. */
  forgetMusic(id: number): void;
  /** Debug: ids currently in the music cache (e2e + in-page inspection). */
  debugMusicCache(): number[];
};

/** Rendered in-wasm (see wasm/src/music/): stereo interleaved f32, 22050 Hz. */
export type AudioRenderer = {
  sfx(id: number): Float32Array;
};

const MUSIC_SR = 22050;

/** Stereo interleaved samples -> native AudioBuffer 22050 Hz (browser resamples). */
function toBuffer(ctx: AudioContext, raw: Float32Array): AudioBuffer {
  const frames = Math.floor(raw.length / 2);
  const buf = ctx.createBuffer(2, frames, MUSIC_SR);
  const l = buf.getChannelData(0);
  const r = buf.getChannelData(1);
  for (let i = 0; i < frames; i++) {
    l[i] = raw[2 * i];
    r[i] = raw[2 * i + 1];
  }
  return buf;
}

/** Stereo exponentially decaying noise: a cheap, deterministic hall reverb. */
function impulseResponse(ctx: AudioContext, seconds: number): AudioBuffer {
  const len = Math.floor(ctx.sampleRate * seconds);
  const buf = ctx.createBuffer(2, len, ctx.sampleRate);
  let seed = 1234567;
  for (let ch = 0; ch < 2; ch++) {
    const data = buf.getChannelData(ch);
    for (let i = 0; i < len; i++) {
      seed = (seed * 1103515245 + 12345) & 0x7fffffff;
      data[i] = ((seed / 0x7fffffff) * 2 - 1) * Math.pow(1 - i / len, 3);
    }
  }
  return buf;
}

export function createAudio(render: AudioRenderer): AudioHost {
  const Ctx =
    window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  if (!Ctx) {
    console.warn('WebAudio not supported: running silent');
    return {
      unlock() {},
      handle() {},
      toggleMute: () => true,
      adjustMusic: () => MUSIC_VOLUME,
      adjustSfx: () => SFX_VOLUME,
      prerenderMusic() {},
      forgetMusic() {},
      debugMusicCache: () => [],
    };
  }

  // Created early (suspended) so assets decode while the title screen shows.
  const ctx = new Ctx();
  const volumes = loadVolumes();
  const master = ctx.createGain();
  const musicBus = ctx.createGain();
  const sfxBus = ctx.createGain();
  musicBus.gain.value = volumes.music;
  sfxBus.gain.value = volumes.sfx;
  // Future: the same song "heard through the present" (low-pass + wide reverb).
  const filter = ctx.createBiquadFilter();
  filter.type = 'lowpass';
  filter.frequency.value = 20000;
  const dry = ctx.createGain();
  const wet = ctx.createGain();
  wet.gain.value = 0;
  const reverb = ctx.createConvolver();
  reverb.buffer = impulseResponse(ctx, 2.5);
  musicBus.connect(filter);
  filter.connect(dry);
  filter.connect(reverb);
  reverb.connect(wet);
  dry.connect(master);
  wet.connect(master);

  function setFuture(on: boolean) {
    const t = ctx.currentTime;
    const ramp = 0.4; // time constant: settles over ~1.2 s, like the camera
    filter.frequency.setTargetAtTime(on ? 1200 : 20000, t, ramp);
    wet.gain.setTargetAtTime(on ? 0.6 : 0, t, ramp);
    dry.gain.setTargetAtTime(on ? 0.7 : 1, t, ramp);
  }
  sfxBus.connect(master);
  master.connect(ctx.destination);

  const sfx = new Map<number, AudioBuffer>();
  const music = new Map<number, AudioBuffer>();

  // Music state: the track that *should* play, so a cue that arrives while a
  // lazy render is pending (or while paused) is honoured later.
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
    if (!buf) {
      beginRender(wanted, true);
      return;
    }
    source = ctx.createBufferSource();
    source.buffer = buf;
    source.loop = true;
    source.connect(musicBus);
    const at = offset % buf.duration;
    startedAt = ctx.currentTime - at;
    source.start(0, at);
  }

  /** In-flight renders, keyed by track id (one task per track at a time).
   * `brisk` is the pace: wanted tracks step at ~50% CPU, background
   * pre-renders at ~10% so the game's rAF never feels them. */
  const rendering = new Map<number, { task: MusicRenderTask; brisk: boolean }>();

  function beginRender(id: number, brisk: boolean) {
    if (music.has(id)) return;
    const cur = rendering.get(id);
    if (cur) {
      cur.brisk = cur.brisk || brisk; // promote on demand, never demote
      return;
    }
    let task: MusicRenderTask;
    try {
      task = new MusicRenderTask(id);
    } catch (err) {
      console.warn('audio: music render failed', id, err);
      return;
    }
    rendering.set(id, { task, brisk });
    const pump = () => {
      const entry = rendering.get(id);
      // superseded (forgotten while rendering): free, drop it entirely.
      if (!entry || entry.task !== task) {
        task.free();
        return;
      }
      const budget = entry.brisk ? 4 : 2;
      const deadline = performance.now() + budget;
      let done = false;
      while (!done && performance.now() < deadline) {
        done = task.step(4);
      }
      if (!done) {
        setTimeout(pump, budget);
        return;
      }
      rendering.delete(id);
      let buf: AudioBuffer;
      try {
        // take() consumes the task: wasm-bindgen frees it on this side of
        // the call, so no manual free() afterwards.
        buf = toBuffer(ctx, task.take());
      } catch (err) {
        console.warn('audio: music render failed', id, err);
        task.free();
        return;
      }
      music.set(id, buf);
      // render finished while the cue is still wanted: honour it (like the
      // former async fetch did).
      if (wanted === id && !paused) startMusic();
    };
    setTimeout(pump, brisk ? 4 : 16);
  }

  // SFX: rendered once at boot (short, synchronous, deterministic).
  // The alpine stems (bell / wheel rattle / crust / moo) ride along.
  for (let id = 0; id < 17; id++) {
    try {
      sfx.set(id, toBuffer(ctx, render.sfx(id)));
    } catch (err) {
      console.warn(`audio: sfx render failed for ${id}`, err);
    }
  }

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
        } else if (code === MUSIC_FUTURE_ON || code === MUSIC_FUTURE_OFF) {
          setFuture(code === MUSIC_FUTURE_ON);
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
    adjustMusic(delta) {
      return adjust('music', delta);
    },
    adjustSfx(delta) {
      return adjust('sfx', delta);
    },
    prerenderMusic(id) {
      beginRender(id, false);
    },
    forgetMusic(id) {
      rendering.get(id)?.task.free(); // no-op when already drained to the cache
      rendering.delete(id);
      music.delete(id);
    },
    debugMusicCache() {
      return [...music.keys()].sort((a, b) => a - b);
    },
  };

  function adjust(which: 'music' | 'sfx', delta: number): number {
    const cur = which === 'music' ? musicBus.gain.value : sfxBus.gain.value;
    const next = Math.min(1, Math.max(0, cur + delta));
    if (which === 'music') musicBus.gain.value = next;
    else sfxBus.gain.value = next;
    volumes[which] = next;
    try {
      localStorage.setItem(VOLUME_KEY, JSON.stringify(volumes));
    } catch {
      // Persistence unavailable: the level simply lasts this session.
    }
    return next;
  }
}
