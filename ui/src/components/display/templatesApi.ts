import { api } from '../../lib/api';
import type { SavedView, ViewDef } from '../../lib/bindings';

// Shapes the backend will generate (view templates). Until it lands, `templates()` is null and the UI hides templates.
export type ViewTemplate = { id: number; name: string; icon: string | null; color: string | null; def: ViewDef; position: number };

export type TemplateApi = {
  list(): Promise<ViewTemplate[]>;
  create(name: string, def: ViewDef, icon: string | null, color: string | null): Promise<ViewTemplate>;
  update(t: ViewTemplate): Promise<ViewTemplate>;
  delete(id: number): Promise<null>;
  createFromTemplate(templateId: number, name: string | null): Promise<SavedView>;
  saveAsTemplate(viewId: number, name: string | null): Promise<ViewTemplate>;
  resetToTemplate(viewId: number): Promise<SavedView>;
};

// Tests inject a fake here; `undefined` means "look it up on the api", `null` means "not available".
let injected: TemplateApi | null | undefined;
export function setTemplateApi(a: TemplateApi | null) {
  injected = a;
}

type Loose = {
  viewTemplates?: Omit<TemplateApi, 'createFromTemplate' | 'saveAsTemplate' | 'resetToTemplate'>;
  views?: Partial<Pick<TemplateApi, 'createFromTemplate' | 'saveAsTemplate' | 'resetToTemplate'>>;
};

export function templates(): TemplateApi | null {
  if (injected !== undefined) return injected;
  const a = api as unknown as Loose;
  const vt = a.viewTemplates;
  const v = a.views;
  if (!vt || !v?.createFromTemplate || !v.saveAsTemplate || !v.resetToTemplate) return null;
  return {
    list: () => vt.list(),
    create: (name, def, icon, color) => vt.create(name, def, icon, color),
    update: (t) => vt.update(t),
    delete: (id) => vt.delete(id),
    createFromTemplate: (id, name) => v.createFromTemplate!(id, name),
    saveAsTemplate: (id, name) => v.saveAsTemplate!(id, name),
    resetToTemplate: (id) => v.resetToTemplate!(id),
  };
}

const DEFAULT_KEY = 'default_view_template';

export async function defaultTemplateId(): Promise<number | null> {
  const v = await api.settings.get(DEFAULT_KEY);
  return typeof v === 'number' ? v : null;
}

export function setDefaultTemplate(id: number | null) {
  return api.settings.set(DEFAULT_KEY, id);
}
