export type AudioSystem = {
  unlock(): Promise<void>;
  playSfx(name: string): void;
  playMusic(name: string): void;
};

// Placeholder asset paths. These are intentionally non-copyrighted and may be
// replaced by real assets later.
const SFX_ASSETS: Record<string, string> = {
  jump: '/audio/sfx/jump.wav',
  dash: '/audio/sfx/dash.wav',
};

export function createAudio(): AudioSystem {
  let ctx: AudioContext | null = null;
  let unlocked = false;

  const buffers = new Map<string, AudioBuffer>();

  function ensureContext(): AudioContext {
    if (!ctx) {
      const Ctx = window.AudioContext || (window as any).webkitAudioContext;
      if (!Ctx) throw new Error('WebAudio not supported');
      ctx = new Ctx();
    }
    return ctx;
  }

  async function loadBuffer(name: string): Promise<AudioBuffer | null> {
    const existing = buffers.get(name);
    if (existing) return existing;

    const url = SFX_ASSETS[name];
    if (!url) return null;

    try {
      const res = await fetch(url);
      if (!res.ok) return null;
      const arr = await res.arrayBuffer();
      const audioBuffer = await ensureContext().decodeAudioData(arr);
      buffers.set(name, audioBuffer);
      return audioBuffer;
    } catch {
      return null;
    }
  }

  function playBuffer(buffer: AudioBuffer) {
    const c = ensureContext();
    const src = c.createBufferSource();
    src.buffer = buffer;
    src.connect(c.destination);
    src.start();
  }

  return {
    async unlock() {
      if (unlocked) return;
      const c = ensureContext();

      // Resume context (required on many browsers until user gesture).
      if (c.state !== 'running') {
        await c.resume();
      }

      // Prime: play a silent buffer to fully unlock on iOS/Safari.
      const silent = c.createBuffer(1, 1, c.sampleRate);
      const src = c.createBufferSource();
      src.buffer = silent;
      src.connect(c.destination);
      src.start(0);

      unlocked = true;

      // Best-effort preload.
      await Promise.all(Object.keys(SFX_ASSETS).map((k) => loadBuffer(k)));
    },

    playSfx(name: string) {
      if (!unlocked) return;
      void loadBuffer(name).then((buf) => {
        if (buf) playBuffer(buf);
      });
    },

    playMusic(_name: string) {
      // Placeholder: no music system yet.
    },
  };
}
