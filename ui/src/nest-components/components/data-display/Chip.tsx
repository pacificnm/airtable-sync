// Vendored from core/crates/nest-react-components/src/components/data-display/Chip.tsx
// (commit ffb8f6d, 2026-07-12) — see ../../README.md for why and how to re-sync.

import { forwardRef, type ReactNode } from 'react';
import { cn } from '../../lib/cn';
import { X } from 'lucide-react';

export type ChipVariant = 'filled' | 'outlined';
export type ChipColor = 'default' | 'primary' | 'secondary' | 'accent' | 'success' | 'warning' | 'error' | 'info';
export type ChipSize = 'small' | 'medium';

export interface ChipProps {
  /**
   * The component to render as.
   * @default 'div'
   */
  component?: React.ElementType;
  /**
   * The variant of the chip.
   * @default 'filled'
   */
  variant?: ChipVariant;
  /**
   * The color of the chip.
   * @default 'default'
   */
  color?: ChipColor;
  /**
   * The size of the chip.
   * @default 'medium'
   */
  size?: ChipSize;
  /**
   * The label content of the chip.
   */
  label?: ReactNode;
  /**
   * Icon element to display before the label.
   */
  icon?: ReactNode;
  /**
   * Callback fired when the delete icon is clicked.
   * If set, a delete icon will be displayed.
   */
  onDelete?: () => void;
  /**
   * The label for the delete icon (for accessibility).
   * @default 'Delete'
   */
  deleteLabel?: string;
  /**
   * If true, the chip will be clickable (applies hover styles).
   * @default false
   */
  clickable?: boolean;
  /**
   * If true, the chip will be disabled.
   * @default false
   */
  disabled?: boolean;
  /**
   * Additional CSS classes.
   */
  className?: string;
  /**
   * Click handler for the chip.
   */
  onClick?: () => void;
}

const VARIANT_STYLES: Record<ChipVariant, Record<ChipColor, string>> = {
  filled: {
    default: 'bg-nest-surface text-nest-foreground',
    primary: 'bg-nest-primary text-white',
    secondary: 'bg-nest-secondary text-white',
    accent: 'bg-nest-accent text-white',
    success: 'bg-nest-success text-white',
    warning: 'bg-nest-warning text-white',
    error: 'bg-nest-error text-white',
    info: 'bg-nest-info text-white',
  },
  outlined: {
    default: 'border border-nest-border text-nest-foreground',
    primary: 'border border-nest-primary text-nest-primary',
    secondary: 'border border-nest-secondary text-nest-secondary',
    accent: 'border border-nest-accent text-nest-accent',
    success: 'border border-nest-success text-nest-success',
    warning: 'border border-nest-warning text-nest-warning',
    error: 'border border-nest-error text-nest-error',
    info: 'border border-nest-info text-nest-info',
  },
};

const SIZE_STYLES: Record<ChipSize, string> = {
  small: 'h-6 text-xs gap-1 px-2',
  medium: 'h-8 text-sm gap-1.5 px-3',
};

/**
 * Chip component - a compact element that represents an input, attribute, or action.
 * Follows MUI Chip API conventions.
 *
 * Chips can be used to display information, select from options, trigger actions,
 * or remove items from a list.
 *
 * @example
 * // Basic chip
 * <Chip label="Basic Chip" />
 *
 * @example
 * // Chip with delete
 * <Chip label="Deletable" onDelete={() => console.log('deleted')} />
 *
 * @example
 * // Chip with icon
 * <Chip label="With Icon" icon={<StarIcon />} />
 *
 * @example
 * // Clickable chip
 * <Chip label="Clickable" clickable onClick={() => console.log('clicked')} />
 */
export const Chip = forwardRef<HTMLDivElement, ChipProps>(function Chip(
  {
    className,
    component,
    variant = 'filled',
    color = 'default',
    size = 'medium',
    label,
    icon,
    onDelete,
    deleteLabel = 'Delete',
    clickable = false,
    disabled = false,
    onClick,
    ...props
  }: ChipProps & React.HTMLAttributes<HTMLDivElement>,
  ref: React.Ref<HTMLDivElement>
) {
  const Component = component ?? (clickable || onClick ? 'button' : 'div');

  const baseStyles = 'inline-flex items-center rounded-full font-body transition-all duration-150';

  const variantStyles = VARIANT_STYLES[variant][color];

  const sizeStyles = SIZE_STYLES[size];

  const clickableStyles = clickable || onClick ? 'cursor-pointer hover:opacity-80 active:scale-95' : '';

  const disabledStyles = disabled ? 'opacity-50 cursor-not-allowed pointer-events-none' : '';

  const handleDeleteClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    if (onDelete && !disabled) {
      onDelete();
    }
  };

  return (
    <Component
      ref={ref as any}
      className={cn(baseStyles, variantStyles, sizeStyles, clickableStyles, disabledStyles, className)}
      onClick={onClick}
      disabled={disabled}
      {...(props as any)}
    >
      {icon && <span className="shrink-0">{icon}</span>}
      <span className="truncate">{label}</span>
      {onDelete && (
        <button
          type="button"
          className="flex shrink-0 items-center justify-center rounded-full p-0.5 hover:bg-black/10 focus:outline-none focus:ring-2 focus:ring-nest-primary/50"
          onClick={handleDeleteClick}
          aria-label={deleteLabel}
          disabled={disabled}
        >
          <X className="h-3.5 w-3.5" />
        </button>
      )}
    </Component>
  );
});
