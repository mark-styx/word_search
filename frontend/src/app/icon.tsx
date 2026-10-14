import { ImageResponse } from "next/og";

export const size = {
  width: 512,
  height: 512,
};

export const contentType = "image/png";

export default function Icon() {
  return new ImageResponse(
    (
      <div
        style={{
          display: "flex",
          height: "100%",
          width: "100%",
          alignItems: "center",
          justifyContent: "center",
          background:
            "linear-gradient(145deg, #0f172a 0%, #1d4ed8 45%, #22c55e 100%)",
          color: "white",
          fontSize: 192,
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
