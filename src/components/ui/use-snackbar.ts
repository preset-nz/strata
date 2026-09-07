import { Toast } from "@base-ui/react/toast"

type ShowOptions = {
  message: string
  action?: { label: string; onClick: () => void }
  timeout?: number
}

export function useSnackbar() {
  const manager = Toast.useToastManager()
  return {
    show: (opts: ShowOptions) =>
      manager.add({
        title: opts.message,
        timeout: opts.timeout ?? 5000,
        actionProps: opts.action
          ? { children: opts.action.label, onClick: opts.action.onClick }
          : undefined,
      }),
  }
}
