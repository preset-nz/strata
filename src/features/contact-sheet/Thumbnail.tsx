type Props = {
  hash: string
  filename: string
  status: "ready" | "missing" | "failed" | string
}

export function Thumbnail({ hash, filename, status }: Props) {
  if (status !== "ready") {
    return (
      <div
        title={filename}
        className="flex h-full w-full items-center justify-center bg-muted p-1 text-center text-[10px] text-muted-foreground"
      >
        no thumb
      </div>
    )
  }
  return (
    <img
      src={`thumb://${hash}/256.jpg`}
      alt={filename}
      title={filename}
      loading="lazy"
      className="block h-full w-full object-cover"
    />
  )
}
