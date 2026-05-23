import { useEffect } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

type Props = {
  onDropped: (path: string) => void;
};

export function DropZone({ onDropped }: Props) {
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      const w = getCurrentWebview();
      const handler = await w.onDragDropEvent((event) => {
        if (event.payload.type === "drop") {
          const paths = event.payload.paths;
          if (paths.length > 0) {
            onDropped(paths[0]);
          }
        }
      });
      unlisten = handler;
    })();
    return () => {
      if (unlisten) unlisten();
    };
  }, [onDropped]);
  return null;
}
