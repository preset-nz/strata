import { open } from "@tauri-apps/plugin-dialog";

type Props = {
  onPicked: (path: string) => void;
  disabled?: boolean;
};

export function FolderPicker({ onPicked, disabled }: Props) {
  async function pick() {
    const result = await open({ directory: true, multiple: false });
    if (typeof result === "string") {
      onPicked(result);
    }
  }
  return (
    <button type="button" onClick={pick} disabled={disabled}>
      Choose folder
    </button>
  );
}
