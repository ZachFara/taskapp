// Per-device sidebar choices, kept in this browser only.
const KEY = 'sidebar.showProjects';
const read = () => {
  try {
    return localStorage.getItem(KEY) === '1';
  } catch {
    return false;
  }
};

/** Whether the Projects section lists every project (off: it only links "All projects"). */
export const sidebarPrefs = $state({ showProjects: read() });

export function toggleShowProjects() {
  sidebarPrefs.showProjects = !sidebarPrefs.showProjects;
  try {
    localStorage.setItem(KEY, sidebarPrefs.showProjects ? '1' : '0');
  } catch {
    /* private window: the switch still works, it just is not remembered */
  }
}
