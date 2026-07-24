// Vendored from core/crates/nest-react-components/src/lib/cn.ts
// (commit ffb8f6d, 2026-07-12) — see ../README.md for why and how to re-sync.

import { type ClassValue, clsx } from 'clsx';
import { twMerge } from 'tailwind-merge';

/**
 * Merges class names using clsx and tailwind-merge for optimal Tailwind class resolution.
 * Use this instead of template literals for component className composition.
 *
 * @example
 * cn('btn', 'btn-primary', className)
 * cn({ 'btn-disabled': disabled, 'btn-loading': loading })
 */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
