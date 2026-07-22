import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";

export type StatusVariant = "info" | "success" | "warning" | "error";

const DEFAULT_MESSAGE = "Ready";

type StatusBarContextValue = {
  message: string;
  variant: StatusVariant;
  setStatus: (
    message: string,
    options?: { variant?: StatusVariant; timeoutMs?: number },
  ) => void;
  clearStatus: () => void;
};

const StatusBarContext = createContext<StatusBarContextValue | null>(null);

export function StatusBarProvider({ children }: { children: ReactNode }) {
  const [message, setMessage] = useState(DEFAULT_MESSAGE);
  const [variant, setVariant] = useState<StatusVariant>("info");
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const clearStatus = useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = undefined;
    }
    setMessage(DEFAULT_MESSAGE);
    setVariant("info");
  }, []);

  const setStatus = useCallback(
    (
      next: string,
      options?: { variant?: StatusVariant; timeoutMs?: number },
    ) => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = undefined;
      }
      setMessage(next);
      setVariant(options?.variant ?? "info");
      if (options?.timeoutMs !== undefined) {
        timeoutRef.current = setTimeout(clearStatus, options.timeoutMs);
      }
    },
    [clearStatus],
  );

  useEffect(
    () => () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    },
    [],
  );

  return (
    <StatusBarContext.Provider
      value={{ message, variant, setStatus, clearStatus }}
    >
      {children}
    </StatusBarContext.Provider>
  );
}

export function useStatusBar(): StatusBarContextValue {
  const context = useContext(StatusBarContext);
  if (!context) {
    throw new Error("useStatusBar must be used within StatusBarProvider");
  }
  return context;
}
