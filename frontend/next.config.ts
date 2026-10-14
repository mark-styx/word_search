import type { NextConfig } from "next";
import { networkInterfaces } from "node:os";

function getDefaultAllowedDevOrigins() {
  const defaults = new Set<string>(["127.0.0.1", "*.local"]);
  const interfaces = networkInterfaces();

  for (const addresses of Object.values(interfaces)) {
    for (const address of addresses ?? []) {
      if (address.family === "IPv4" && !address.internal) {
        defaults.add(address.address);
      }
    }
  }

  return [...defaults];
}

const allowedDevOrigins = [
  ...getDefaultAllowedDevOrigins(),
  ...(process.env.NEXT_ALLOWED_DEV_ORIGINS
    ?.split(",")
    .map((origin) => origin.trim())
    .filter(Boolean) ?? []),
];

const nextConfig: NextConfig = {
  output: "standalone",
  devIndicators: false,
  allowedDevOrigins,
};

export default nextConfig;
