// Lowers the future resolution cap when the 3D world cannot hold 60 fps (spec §9).
const STEPS = [1440, 1080, 720, 540];
const WINDOW = 90; // frames
const BUDGET_MS = 1000 / 60 + 1.5;

export class FutureCap {
  private level = 0;
  private samples: number[] = [];

  constructor(private onChange: (maxHeight: number) => void) {
    onChange(STEPS[0]);
  }

  /** Current cap, for applying to a (re)created handle. */
  current(): number {
    return STEPS[this.level];
  }

  sample(frameMs: number, inFuture: boolean) {
    if (!inFuture || frameMs > 250) {
      this.samples = [];
      return;
    }
    this.samples.push(frameMs);
    if (this.samples.length < WINDOW) return;
    const sorted = [...this.samples].sort((a, b) => a - b);
    const median = sorted[sorted.length >> 1];
    this.samples = [];
    if (median > BUDGET_MS && this.level < STEPS.length - 1) {
      this.level++;
      this.onChange(STEPS[this.level]);
      console.info(`future resolution capped at ${STEPS[this.level]}p`);
    }
  }
}
