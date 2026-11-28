// "Ask for the solution link when an issue is completed": on until turned off. Per device, like the tooltip choice.
const KEY = 'cairn.linkSolutionPrompt';

function read(): boolean {
  try {
    return localStorage.getItem(KEY) !== '0';
  } catch {
    return true;
  }
}

export const linkPrompt = $state<{ on: boolean }>({ on: read() });

export function setLinkPrompt(on: boolean) {
  linkPrompt.on = on;
  try {
    localStorage.setItem(KEY, on ? '1' : '0');
  } catch {
    // Not remembered without storage; the choice still applies.
  }
}
