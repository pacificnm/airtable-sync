import { Component, type ErrorInfo, type ReactNode } from "react";
import { faRotateRight, faTriangleExclamation } from "../lib/fontawesome";
import { Icon } from "./Icon";

type ErrorBoundaryProps = {
  children: ReactNode;
  /** Optional label describing the boundary location (shown in dev details). */
  label?: string;
};

type ErrorBoundaryState = {
  error: Error | null;
};

/**
 * Catches render/lifecycle errors in the subtree and shows a recovery screen
 * instead of unmounting the whole app (React blanks the tree on uncaught throws).
 */
export class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error(
      `UI error${this.props.label ? ` (${this.props.label})` : ""}:`,
      error,
      info.componentStack,
    );
  }

  handleReset = (): void => {
    this.setState({ error: null });
  };

  handleReload = (): void => {
    window.location.reload();
  };

  render(): ReactNode {
    const { error } = this.state;
    if (!error) {
      return this.props.children;
    }

    return (
      <div className="flex h-full min-h-0 w-full items-center justify-center bg-nest-background p-6 text-nest-foreground">
        <div className="w-full max-w-lg rounded-nest-md border border-nest-border bg-nest-surface p-6 shadow-sm">
          <div className="flex items-center gap-3">
            <span className="flex size-9 shrink-0 items-center justify-center rounded-full bg-nest-error/15 text-nest-error">
              <Icon icon={faTriangleExclamation} className="size-4" />
            </span>
            <div>
              <h1 className="text-sm font-semibold">Something went wrong</h1>
              <p className="text-xs text-nest-muted">
                An unexpected error interrupted this view.
              </p>
            </div>
          </div>

          <pre className="mt-4 max-h-40 overflow-auto rounded-nest-sm border border-nest-border bg-nest-background p-3 text-[11px] leading-relaxed text-nest-muted">
            {error.message || String(error)}
          </pre>

          <div className="mt-5 flex gap-2">
            <button
              type="button"
              onClick={this.handleReset}
              className="inline-flex items-center gap-1.5 rounded-nest-sm bg-nest-primary px-3 py-1.5 text-xs font-medium text-white hover:bg-nest-primary/90"
            >
              <Icon icon={faRotateRight} className="size-3" />
              Try again
            </button>
            <button
              type="button"
              onClick={this.handleReload}
              className="inline-flex items-center gap-1.5 rounded-nest-sm border border-nest-border bg-nest-surface px-3 py-1.5 text-xs font-medium text-nest-foreground hover:bg-nest-primary/10"
            >
              Reload app
            </button>
          </div>
        </div>
      </div>
    );
  }
}
