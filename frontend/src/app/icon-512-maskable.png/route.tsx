import { ImageResponse } from "next/og";

export const runtime = "edge";

export async function GET() {
  return new ImageResponse(
    (
      <div
        style={{
          display: "flex",
          height: "100%",
          width: "100%",
          alignItems: "center",
          justifyContent: "center",
          background: "#0f172a",
          padding: 64,
        }}
      >
        <div
          style={{
            display: "flex",
            height: "100%",
            width: "100%",
            alignItems: "center",
            justifyContent: "center",
            borderRadius: 120,
            background:
              "linear-gradient(145deg, #0f172a 0%, #1d4ed8 45%, #22c55e 100%)",
            color: "white",
            fontSize: 168,
            fontWeight: 800,
            letterSpacing: "-0.08em",
          }}
        >
          WS
        </div>
      </div>
    ),
    { width: 512, height: 512 }
  );
}
