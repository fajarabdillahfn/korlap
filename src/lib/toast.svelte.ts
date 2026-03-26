export type ToastKind = 'error' | 'success' | 'info';

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
}

let nextId = 0;

export const toasts = $state<Toast[]>([]);

export function showToast(message: string, kind: ToastKind = 'error', durationMs = 5000) {
  const id = ++nextId;
  toasts.push({ id, kind, message });
  setTimeout(() => dismissToast(id), durationMs);
}

export function dismissToast(id: number) {
  const idx = toasts.findIndex(t => t.id === id);
  if (idx !== -1) toasts.splice(idx, 1);
}
