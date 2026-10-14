import { spawn } from "node:child_process";
import { cpSync, existsSync } from "node:fs";
import path from "node:path";

const rootDir = process.cwd();
const standaloneDir = path.join(rootDir, ".next", "standalone");
const standaloneNextDir = path.join(standaloneDir, ".next");
const builtStaticDir = path.join(rootDir, ".next", "static");
const standaloneStaticDir = path.join(standaloneNextDir, "static");
const publicDir = path.join(rootDir, "public");
const standalonePublicDir = path.join(standaloneDir, "public");
const serverEntry = path.join(standaloneDir, "server.js");

if (!existsSync(serverEntry)) {
  throw new Error("Missing .next/standalone/server.js. Run `npm run build` first.");
}

if (!existsSync(builtStaticDir)) {
  throw new Error("Missing .next/static. Run `npm run build` first.");
}

cpSync(builtStaticDir, standaloneStaticDir, { recursive: true, force: true });

if (existsSync(publicDir)) {
  cpSync(publicDir, standalonePublicDir, { recursive: true, force: true });
}

const child = spawn(process.execPath, [serverEntry], {
  stdio: "inherit",
  env: {
    ...process.env,
    HOSTNAME: process.env.HOSTNAME || "0.0.0.0",
  },
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 0);
});
