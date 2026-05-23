import type { PrescanSummary } from "./api";

type Props = {
  scan: PrescanSummary;
  onConfirm: () => void;
  onCancel: () => void;
  busy?: boolean;
};

export function ImportConfirmation({ scan, onConfirm, onCancel, busy }: Props) {
  const breakdown = Object.entries(scan.by_extension)
    .map(([ext, n]) => `${n} ${ext}`)
    .join(", ");
  return (
    <div style={{ padding: 16, border: "1px solid #888", borderRadius: 6 }}>
      <p>
        <strong>{scan.total}</strong> images ready for ingress from{" "}
        <code>{scan.root}</code>
      </p>
      {breakdown && <p style={{ fontSize: 12, opacity: 0.8 }}>{breakdown}</p>}
      <p>
        About to import {scan.total} images from <code>{scan.root}</code> into
        Strata. Source files will be moved to the Trash after each successful
        import. Non-image files will be left in place. Continue?
      </p>
      <div style={{ display: "flex", gap: 8 }}>
        <button type="button" onClick={onConfirm} disabled={busy || scan.total === 0}>
          Confirm import
        </button>
        <button type="button" onClick={onCancel} disabled={busy}>
          Cancel
        </button>
      </div>
    </div>
  );
}
