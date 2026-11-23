// Animation speed scales every motion duration: the root's --motion-scale multiplies the --dur tokens. Per device,
// like the interface size. Reduced motion (the OS setting) means Off until the user picks a speed themselves.
export const SPEEDS = { off: 0, fast: 0.6, normal: 1, slow: 1.6 } as const;
export type Speed = keyof typeof SPEEDS;
const ORDER: Speed[] = ['off', 'fast', 'normal', 'slow'];
const KEY = 'cairn.animationSpeed';

function stored(): Speed | null {
  try {
    const v = localStorage.getItem(KEY);
    return v !== null && v in SPEEDS ? (v as Speed) : null;
  } catch {
    return null;
  }
}

export function readSpeed(): Speed {
  const saved = stored();
  if (saved) return saved;
  const reduced = globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
  return reduced ? 'off' : 'normal';
}

export const motion = $state<{ speed: Speed }>({ speed: readSpeed() });

// base.css turns animations off for reduced motion only while no speed was picked (data-motion-set marks a pick).
function apply() {
  const root = document.documentElement;
  root.style.setProperty('--motion-scale', String(SPEEDS[motion.speed]));
  root.toggleAttribute('data-motion-set', stored() !== null);
}
apply();

export function setSpeed(speed: Speed) {
  motion.speed = speed;
  try {
    localStorage.setItem(KEY, speed);
  } catch {
    // Not remembered without storage; the speed still applies.
  }
  apply();
}

/** One step slower (-1) or faster (1) along Off, Fast, Normal, Slow; the ends hold. */
export function stepSpeed(dir: 1 | -1) {
  const i = ORDER.indexOf(motion.speed);
  setSpeed(ORDER[Math.min(ORDER.length - 1, Math.max(0, i + dir))]);
}
