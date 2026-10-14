import { ImageResponse } from "next/og";

export const size = {
  width: 180,
  height: 180,
};

export const contentType = "image/png";

export default function AppleIcon() {
  return new ImageResponse(
    (
      <div
        style={{
          display: "flex",
          height: "100%",
          width: "100%",
          alignItems: "center",
          justifyContent: "center",
          borderRadius: 36,
          background:
            "linear-gradient(145deg, #0f172a 0%, #1d4ed8 45%, #22c55e 100%)",
          color: "white",
          fontSize: 72,
          fontWeight: 800,
          letterSpacing: "-0.08em",
        }}
      >
        WS
      </div>
    ),
    size
  );
}
