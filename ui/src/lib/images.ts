import { api } from './api';

const TYPES = ['image/png', 'image/jpeg', 'image/gif', 'image/webp'];
const MAX = 10 * 1024 * 1024;

/** Stores one image file and returns the markdown that shows it. The backend re-checks the bytes; these checks give the clear message first. */
export async function uploadImage(file: File): Promise<string> {
  if (!TYPES.includes(file.type)) throw new Error('Only PNG, JPEG, GIF and WebP images can be attached.');
  if (file.size > MAX) throw new Error('That image is larger than 10 MB.');
  const a = await api.attachments.add(Array.from(new Uint8Array(await file.arrayBuffer())), file.type);
  const alt = file.name.replace(/\.[^.]+$/, '').replace(/[[\]\n]/g, '') || 'image';
  return `![${alt}](attachment:${a.id})`;
}

export const errorText = (e: unknown) => (e as { message?: string })?.message ?? String(e);

/** The image field the person last used, so the command menu can attach to it. */
let target: { el: HTMLElement; pick: () => void } | null = null;
export const setImageTarget = (t: typeof target) => { target = t; };
export const clearImageTarget = (el: HTMLElement) => { if (target?.el === el) target = null; };
export const hasImageTarget = () => !!target?.el.isConnected;
export const pickImage = () => target?.pick();
