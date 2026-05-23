import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { Thumbnail } from "../contact-sheet/Thumbnail";
import { ThumbGrid } from "../contact-sheet/ThumbGrid";
import type { ImportedRow } from "../contact-sheet/api";
import { listImages } from "./api";

type Cell = {
  key: string;
  hash: string;
  filename: string;
  status: string;
};

const PAGE_SIZE = 100;

function toCell(row: ImportedRow): Cell {
  return {
    key: row.id,
    hash: row.content_hash,
    filename: row.original_filename,
    status: row.thumbnails_status,
  };
}

export function LibrarySheet() {
  const [items, setItems] = useState<Cell[]>([]);
  const [hasMore, setHasMore] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const loadingRef = useRef(false);
  const offsetRef = useRef(0);

  const loadNext = useCallback(async () => {
    if (loadingRef.current || !hasMore) return;
    loadingRef.current = true;
    try {
      const rows = await listImages(offsetRef.current, PAGE_SIZE);
      offsetRef.current += rows.length;
      setItems((prev) => prev.concat(rows.map(toCell)));
      if (rows.length < PAGE_SIZE) setHasMore(false);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      loadingRef.current = false;
    }
  }, [hasMore]);

  const reset = useCallback(async () => {
    loadingRef.current = false;
    offsetRef.current = 0;
    setHasMore(true);
    setItems([]);
    try {
      loadingRef.current = true;
      const rows = await listImages(0, PAGE_SIZE);
      offsetRef.current = rows.length;
      setItems(rows.map(toCell));
      if (rows.length < PAGE_SIZE) setHasMore(false);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      loadingRef.current = false;
    }
  }, []);

  useEffect(() => {
    void reset();
  }, [reset]);

  useEffect(() => {
    let off: UnlistenFn | undefined;
    (async () => {
      off = await listen("ingest://batch-done", () => {
        void reset();
      });
    })();
    return () => off?.();
  }, [reset]);

  return (
    <section style={{ marginTop: 16 }}>
      <h2 style={{ margin: "8px 0", display: "flex", alignItems: "baseline", gap: 8 }}>
        Library
        <span style={{ fontWeight: "normal", fontSize: 14, opacity: 0.8 }}>
          {items.length} loaded{hasMore ? " (more on scroll)" : ""}
        </span>
      </h2>
      {error && <p style={{ color: "salmon", fontSize: 12 }}>Error: {error}</p>}
      <ThumbGrid
        items={items}
        emptyLabel="Nothing imported yet - drop a folder or pick one above."
        onEndReached={loadNext}
        renderCell={(it) => (
          <Thumbnail hash={it.hash} filename={it.filename} status={it.status} />
        )}
      />
    </section>
  );
}
