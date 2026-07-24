import { useEffect, useMemo, useState } from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";
import { docsList, docsRead, type DocEntry } from "../lib/docs";
import { Icon } from "../shell";
import { faFileLines } from "../lib/fontawesome";

const INDEX_PATH = "index.md";

/** Resolves a Markdown link's `href` (relative to `basePath`) to a docs-root-relative path. */
function resolveDocPath(basePath: string, href: string): string {
  const withoutAnchor = href.split("#")[0];
  const baseDir = basePath.includes("/") ? basePath.slice(0, basePath.lastIndexOf("/") + 1) : "";
  const segments = `${baseDir}${withoutAnchor}`.split("/");
  const stack: string[] = [];
  for (const segment of segments) {
    if (segment === "" || segment === ".") {
      continue;
    }
    if (segment === "..") {
      stack.pop();
      continue;
    }
    stack.push(segment);
  }
  return stack.join("/");
}

/** Full-page Markdown documentation viewer: left table of contents, rendered page on the right. */
export function DocsView() {
  const [entries, setEntries] = useState<DocEntry[]>([]);
  const [activePath, setActivePath] = useState(INDEX_PATH);
  const [content, setContent] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void docsList()
      .then(setEntries)
      .catch((loadError) => setError(String(loadError)));
  }, []);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    docsRead(activePath)
      .then((text) => {
        if (!cancelled) {
          setContent(text);
          setError(null);
        }
      })
      .catch((readError) => {
        if (!cancelled) {
          setError(String(readError));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [activePath]);

  const components = useMemo<Components>(
    () => ({
      h1: ({ children }) => (
        <h1 className="mb-3 text-xl font-semibold text-nest-foreground">{children}</h1>
      ),
      h2: ({ children }) => (
        <h2 className="mb-2 mt-6 border-b border-nest-border pb-1 text-lg font-semibold text-nest-foreground">
          {children}
        </h2>
      ),
      h3: ({ children }) => (
        <h3 className="mb-2 mt-4 text-base font-semibold text-nest-foreground">{children}</h3>
      ),
      h4: ({ children }) => (
        <h4 className="mb-1 mt-3 text-sm font-semibold text-nest-foreground">{children}</h4>
      ),
      p: ({ children }) => (
        <p className="mb-3 text-sm leading-relaxed text-nest-foreground">{children}</p>
      ),
      ul: ({ children }) => (
        <ul className="mb-3 list-disc space-y-1 pl-6 text-sm text-nest-foreground">{children}</ul>
      ),
      ol: ({ children }) => (
        <ol className="mb-3 list-decimal space-y-1 pl-6 text-sm text-nest-foreground">
          {children}
        </ol>
      ),
      li: ({ children }) => <li>{children}</li>,
      hr: () => <hr className="my-4 border-nest-border" />,
      blockquote: ({ children }) => (
        <blockquote className="mb-3 border-l-2 border-nest-border pl-3 text-sm italic text-nest-muted">
          {children}
        </blockquote>
      ),
      code: ({ className, children }) => {
        const isBlock = /language-/.test(className ?? "");
        if (isBlock) {
          return <code className={className}>{children}</code>;
        }
        return (
          <code className="rounded bg-nest-muted/15 px-1 py-0.5 font-mono text-[0.85em] text-nest-foreground">
            {children}
          </code>
        );
      },
      pre: ({ children }) => (
        <pre className="mb-3 overflow-x-auto rounded-nest-md border border-nest-border bg-nest-surface p-3 font-mono text-[12px] leading-relaxed">
          {children}
        </pre>
      ),
      table: ({ children }) => (
        <div className="mb-3 overflow-x-auto rounded-nest-sm border border-nest-border">
          <table className="w-full border-collapse text-left text-xs">{children}</table>
        </div>
      ),
      thead: ({ children }) => (
        <thead className="bg-nest-surface text-nest-foreground">{children}</thead>
      ),
      th: ({ children }) => <th className="px-2 py-1.5 font-semibold">{children}</th>,
      td: ({ children }) => (
        <td className="border-t border-nest-border/60 px-2 py-1.5 align-top text-nest-foreground">
          {children}
        </td>
      ),
      a: ({ href, children }) => {
        if (href && !/^[a-z]+:/i.test(href) && href.endsWith(".md")) {
          const target = resolveDocPath(activePath, href);
          return (
            <button
              type="button"
              onClick={() => setActivePath(target)}
              className="text-nest-primary underline decoration-dotted underline-offset-2 hover:text-nest-secondary"
            >
              {children}
            </button>
          );
        }
        return (
          <a
            href={href}
            target="_blank"
            rel="noreferrer"
            className="text-nest-primary underline decoration-dotted underline-offset-2 hover:text-nest-secondary"
          >
            {children}
          </a>
        );
      },
    }),
    [activePath],
  );

  return (
    <div className="flex h-full min-h-0">
      <nav className="w-56 shrink-0 overflow-auto border-r border-nest-border bg-nest-surface py-2">
        {entries.length === 0 ? (
          <p className="px-3 py-2 text-xs text-nest-muted">
            {error ? "Unable to load docs." : "Loading…"}
          </p>
        ) : (
          <ul>
            {entries.map((entry) => {
              const selected = entry.path === activePath;
              return (
                <li key={entry.path}>
                  <button
                    type="button"
                    onClick={() => setActivePath(entry.path)}
                    title={entry.path}
                    style={{ paddingLeft: `${12 + entry.depth * 14}px` }}
                    className={[
                      "flex w-full items-center gap-1.5 py-1.5 pr-3 text-left text-xs transition-colors",
                      selected
                        ? "bg-nest-primary/10 font-medium text-nest-primary"
                        : "text-nest-foreground hover:bg-nest-muted/10",
                    ].join(" ")}
                  >
                    <span className="min-w-0 flex-1 truncate">{entry.name}</span>
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </nav>

      <div className="min-h-0 flex-1 overflow-auto">
        <div className="mx-auto max-w-3xl px-6 py-6">
          {error ? (
            <p className="text-sm text-nest-error">{error}</p>
          ) : loading ? (
            <p className="text-sm text-nest-muted">Loading…</p>
          ) : content.trim() === "" ? (
            <div className="flex flex-col items-center gap-2 py-12 text-center text-nest-muted">
              <Icon icon={faFileLines} className="size-6 opacity-50" />
              <p className="text-sm">Nothing to show yet.</p>
            </div>
          ) : (
            <ReactMarkdown remarkPlugins={[remarkGfm]} components={components}>
              {content}
            </ReactMarkdown>
          )}
        </div>
      </div>
    </div>
  );
}
