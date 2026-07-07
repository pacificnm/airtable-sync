import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";

export type ToastVariant = "error" | "warning" | "success" | "info";

export type ToastOptions = {
  variant?: ToastVariant;
  durationMs?: number;
};

export type ToastItem = {
  id: string;
  message: string;
  variant: ToastVariant;
  durationMs: number;
  createdAt: number;
};

const DEFAULT_DURATION: Record<ToastVariant, number> = {
  success: 4000,
  info: 5000,
  warning: 6000,
  error: 8000,
};

const MAX_TOASTS = 5;

type ToastContextValue = {
  toasts: ToastItem[];
  show: (message: string, options?: ToastOptions) => string;
  success: (message: string, durationMs?: number) => string;
  info: (message: string, durationMs?: number) => string;
  warning: (message: string, durationMs?: number) => string;
  error: (message: string, durationMs?: number) => string;
  dismiss: (id: string) => void;
  clear: () => void;
};

const ToastContext = createContext<ToastContextValue | null>(null);

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const timeoutsRef = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  const dismiss = useCallback((id: string) => {
    const timeout = timeoutsRef.current.get(id);
    if (timeout) {
      clearTimeout(timeout);
      timeoutsRef.current.delete(id);
    }
    setToasts((prev) => prev.filter((toast) => toast.id !== id));
  }, []);

  const clear = useCallback(() => {
    for (const timeout of timeoutsRef.current.values()) {
      clearTimeout(timeout);
    }
    timeoutsRef.current.clear();
    setToasts([]);
  }, []);

  const scheduleDismiss = useCallback(
    (id: string, durationMs: number) => {
      const timeout = setTimeout(() => dismiss(id), durationMs);
      timeoutsRef.current.set(id, timeout);
    },
    [dismiss],
  );

  const show = useCallback(
    (message: string, options?: ToastOptions): string => {
      const variant = options?.variant ?? "info";
      const durationMs = options?.durationMs ?? DEFAULT_DURATION[variant];
      const id = crypto.randomUUID();
      const toast: ToastItem = {
        id,
        message,
        variant,
        durationMs,
        createdAt: Date.now(),
      };

      setToasts((prev) => {
        const next = [...prev, toast];
        return next.length > MAX_TOASTS ? next.slice(next.length - MAX_TOASTS) : next;
      });
      scheduleDismiss(id, durationMs);
      return id;
    },
    [scheduleDismiss],
  );

  const success = useCallback(
    (message: string, durationMs?: number) =>
      show(message, { variant: "success", durationMs }),
    [show],
  );

  const info = useCallback(
    (message: string, durationMs?: number) =>
      show(message, { variant: "info", durationMs }),
    [show],
  );

  const warning = useCallback(
    (message: string, durationMs?: number) =>
      show(message, { variant: "warning", durationMs }),
    [show],
  );

  const error = useCallback(
    (message: string, durationMs?: number) =>
      show(message, { variant: "error", durationMs }),
    [show],
  );

  useEffect(
    () => () => {
      for (const timeout of timeoutsRef.current.values()) {
        clearTimeout(timeout);
      }
      timeoutsRef.current.clear();
    },
    [],
  );

  const value = useMemo(
    () => ({ toasts, show, success, info, warning, error, dismiss, clear }),
    [toasts, show, success, info, warning, error, dismiss, clear],
  );

  return <ToastContext.Provider value={value}>{children}</ToastContext.Provider>;
}

export function useToast(): ToastContextValue {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error("useToast must be used within ToastProvider");
  }
  return context;
}
