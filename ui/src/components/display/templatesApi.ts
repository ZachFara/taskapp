import { api } from '../../lib/api';
import type { SavedView, ViewDef, ViewTemplate } from '../../lib/bindings';

export type { ViewTemplate };
export type TemplateApi = {
  list(): Promise<ViewTemplate[]>;
  create(name: string, def: ViewDef, icon: string | null, color: string | null): Promise<ViewTemplate>;
  update(t: ViewTemplate): Promise<ViewTemplate>;
  delete(id: number): Promise<null>;
  createFromTemplate(templateId: number, name: string | null): Promise<SavedView>;
  saveAsTemplate(viewId: number, name: string | null): Promise<ViewTemplate>;
  resetToTemplate(viewId: number): Promise<SavedView>;
};

// Tests inject a fake here; null goes back to the real api.
let injected: TemplateApi | null = null;
export function setTemplateApi(a: TemplateApi | null) {
  injected = a;
}

export const templates = (): TemplateApi =>
  injected ?? {
    list: () => api.view.templatesList(),
    create: (name, def, icon, color) => api.view.templatesCreate(name, def, icon, color),
    update: (t) => api.view.templatesUpdate(t),
    delete: (id) => api.view.templatesDelete(id),
    createFromTemplate: (id, name) => api.views.createFromTemplate(id, name),
    saveAsTemplate: (id, name) => api.views.saveAsTemplate(id, name),
    resetToTemplate: (id) => api.views.resetToTemplate(id),
  };

const DEFAULT_KEY = 'default_view_template';

export async function defaultTemplateId(): Promise<number | null> {
  const v = await api.settings.get(DEFAULT_KEY);
  return typeof v === 'number' ? v : null;
}

export function setDefaultTemplate(id: number | null) {
  return api.settings.set(DEFAULT_KEY, id);
}
