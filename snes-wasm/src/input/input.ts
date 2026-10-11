export type InputState = {
  left: boolean;
  right: boolean;
  down: boolean;
  jump: boolean;
  dash: boolean;
};

export function createInput(): { poll(): InputState } {
  const keys = new Set<string>();

  window.addEventListener('keydown', (e) => {
    keys.add(e.code);
  });
  window.addEventListener('keyup', (e) => {
    keys.delete(e.code);
  });

  function pollKeyboard(): InputState {
    return {
      left: keys.has('ArrowLeft') || keys.has('KeyA'),
      right: keys.has('ArrowRight') || keys.has('KeyD'),
      down: keys.has('ArrowDown') || keys.has('KeyS'),
      jump: keys.has('KeyK') || keys.has('Space'),
      dash: keys.has('KeyJ') || keys.has('ShiftLeft'),
    };
  }

  function pollGamepad(base: InputState): InputState {
    const pads = navigator.getGamepads?.() ?? [];
    const pad = pads[0];
    if (!pad) return base;

    // Standard mapping: axes[0] left/right, buttons[0]=A, buttons[[1]=B
    const axisX = pad.axes[0] ?? 0;
    const left = base.left || axisX < -0.4;
    const right = base.right || axisX > 0.4;

    const jump = base.jump || !!pad.buttons[0]?.pressed;
    const dash = base.dash || !!pad.buttons[1]?.pressed;
    const down = base.down || !!pad.buttons[13]?.pressed; // dpad down

    return { left, right, down, jump, dash };
  }

  return {
    poll(): InputState {
      const kb = pollKeyboard();
      return pollGamepad(kb);
    },
  };
}
