// "Show tooltips": hover and focus hints are off until the user turns them on. Per device, like the interface size.
const KEY = 'cairn.showTooltips';

function read(): boolean {
  try {
    return localStorage.getItem(KEY) === '1';
  } catch {
    return false;
  }
}

export const tooltips = $state<{ show: boolean }>({ show: read() });

export function setTooltips(show: boolean) {
  tooltips.show = show;
  try {
    localStorage.setItem(KEY, show ? '1' : '0');
  } catch {
    // Not remembered without storage; the choice still applies.
  }
}
