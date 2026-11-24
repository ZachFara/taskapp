// Port of crates/core/src/attachments.rs: the same type check by magic bytes, the same refusals, one entry per distinct bytes.
import type { Attachment, AttachmentData } from './bindings';

const MAX = 10 * 1024 * 1024;
const starts = (b: number[], s: string, at = 0) => [...s].every((c, i) => b[at + i] === c.charCodeAt(0));
const mimeOf = (b: number[]): string | null => {
  if (b.length >= 8 && [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a].every((x, i) => b[i] === x)) return 'image/png';
  if (b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff) return 'image/jpeg';
  if (starts(b, 'GIF87a') || starts(b, 'GIF89a')) return 'image/gif';
  if (b.length > 12 && starts(b, 'RIFF') && starts(b, 'WEBP', 8)) return 'image/webp';
  return null;
};

export function attachmentsApi(fail: (kind: string, message: string) => never) {
  const rows: (Attachment & { data: AttachmentData })[] = [];
  return {
    add: async (bytes: number[], claimed: string): Promise<Attachment> => {
      if (bytes.length > MAX) fail('Invalid', 'image is larger than 10 MB');
      const mime = mimeOf(bytes);
      if (!mime) fail('Invalid', 'only PNG, JPEG, GIF and WebP images can be attached');
      if (claimed !== mime) fail('Invalid', `the file says ${claimed} but its bytes are ${mime}`);
      const digest = await crypto.subtle.digest('SHA-256', new Uint8Array(bytes));
      const sha256 = [...new Uint8Array(digest)].map((x) => x.toString(16).padStart(2, '0')).join('');
      let row = rows.find((r) => r.sha256 === sha256);
      if (!row) {
        row = { id: Math.max(0, ...rows.map((r) => r.id)) + 1, sha256, mime: mime!, bytes: bytes.length, created_at: new Date().toISOString(), data: { mime: mime!, bytes } };
        rows.push(row);
      }
      const { data: _data, ...out } = row;
      return out;
    },
    get: async (id: number): Promise<AttachmentData> => {
      const row = rows.find((r) => r.id === id);
      return row ? row.data : fail('NotFound', `attachment ${id}`);
    },
  };
}
