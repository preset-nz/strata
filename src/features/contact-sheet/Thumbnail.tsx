type Props = {
  hash: string;
  filename: string;
  status: "ready" | "missing" | "failed" | string;
};

export function Thumbnail({ hash, filename, status }: Props) {
  if (status !== "ready") {
    return (
      <div
        title={filename}
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          background: "#222",
          color: "#888",
          fontSize: 10,
          padding: 4,
          boxSizing: "border-box",
          textAlign: "center",
        }}
      >
        no thumb
      </div>
    );
  }
  return (
    <img
      src={`thumb://${hash}/256.jpg`}
      alt={filename}
      title={filename}
      loading="lazy"
      style={{
        width: "100%",
        height: "100%",
        objectFit: "cover",
        background: "#111",
        display: "block",
      }}
    />
  );
}
