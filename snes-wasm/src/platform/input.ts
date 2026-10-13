// Keyboard + gamepad → button bitmask consumed by the WASM core (see wasm/src/input.rs).

export const BTN_LEFT = 1 << 0;
export const BTN_RIGHT = 1 << 1;
export const BTN_UP = 1 << 2;
export const BTN_DOWN = 1 << 3;
export const BTN_JUMP = 1 << 4;
export const BTN_DASH = 1 << 5;
export const BTN_START = 1 << 6;

const KEYMAP: Record<string, number> = {
  ArrowLeft: BTN_LEFT,
  KeyA: BTN_LEFT,
  ArrowRight: BTN_RIGHT,
  KeyD: BTN_RIGHT,
  ArrowUp: BTN_UP,
  KeyW: BTN_UP,
  ArrowDown: BTN_DOWN,
  KeyS: BTN_DOWN,
  Space: BTN_JUMP,
  KeyK: BTN_JUMP,
  KeyZ: BTN_JUMP,
  ShiftLeft: BTN_DASH,
  ShiftRight: BTN_DASH,
  KeyJ: BTN_DASH,
  KeyX: BTN_DASH,
  Enter: BTN_START,
  Escape: BTN_START,
  KeyP: BTN_START,
};

const AXIS_DEADZONE = 0.5;

export type Input = {
  /** Held buttons since the last poll; taps shorter than a frame are latched. */
  poll(): number;
  /** Release everything (focus lost). */
  reset(): void;
  /** Buttons held right now, without consuming latched taps (level swaps). */
  held(): number;
};

export function createInput(target: Window = window): Input {
  let held = 0;
  let latched = 0;

  target.addEventListener('keydown', (e) => {
    const bit = KEYMAP[e.code];
    if (bit === undefined) return;
    e.preventDefault(); // arrows/space must not scroll the page
    held |= bit;
    latched |= bit;
  });
  target.addEventListener('keyup', (e) => {
    const bit = KEYMAP[e.code];
    if (bit === undefined) return;
    e.preventDefault();
    held &= ~bit;
  });

  function reset() {
    held = 0;
    latched = 0;
  }
  target.addEventListener('blur', reset);

  function pollGamepad(): number {
    const pads = navigator.getGamepads?.() ?? [];
    let bits = 0;
    for (const pad of pads) {
      if (!pad || !pad.connected) continue;
      const b = (i: number) => !!pad.buttons[i]?.pressed;
      const x = pad.axes[0] ?? 0;
      const y = pad.axes[1] ?? 0;
      // "standard" mapping: 0 bottom, 1 right, 2 left, 3 top, 9 start, 12-15 d-pad.
      // SNES-style: bottom/right = jump, left/top/shoulders = run & dash.
      if (b(14) || x < -AXIS_DEADZONE) bits |= BTN_LEFT;
      if (b(15) || x > AXIS_DEADZONE) bits |= BTN_RIGHT;
      if (b(12) || y < -AXIS_DEADZONE) bits |= BTN_UP;
      if (b(13) || y > AXIS_DEADZONE) bits |= BTN_DOWN;
      if (b(0) || b(1)) bits |= BTN_JUMP;
      if (b(2) || b(3) || b(4) || b(5) || b(6) || b(7)) bits |= BTN_DASH;
      if (b(9) || b(8)) bits |= BTN_START;
    }
    return bits;
  }

  return {
    poll() {
      const bits = held | latched | pollGamepad();
      latched = 0;
      return bits;
    },
    reset,
    held: () => held | pollGamepad(),
  };
}
