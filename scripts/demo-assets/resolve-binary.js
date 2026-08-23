// Resolves a bare command name (e.g. "ffmpeg") to an absolute path before
// it's ever passed to execFileSync, instead of relying on execFileSync's
// own implicit PATH search at call time. PATH is sanitized first (empty/
// relative entries stripped) so nothing earlier in it can shadow the real
// binary — see the "generate-demo-assets" skill for the "why" (SonarCloud
// S4036, a Vulnerability, not just a Code Smell).
import path from "node:path";
import fs from "node:fs";

process.env.PATH = (process.env.PATH || "")
  .split(path.delimiter)
  .filter((dir) => path.isAbsolute(dir))
  .join(path.delimiter);

const WINDOWS_EXTENSIONS = ["", ".exe", ".cmd", ".bat"];

export function resolveBinary(name) {
  const dirs = process.env.PATH.split(path.delimiter);
  const suffixes = process.platform === "win32" ? WINDOWS_EXTENSIONS : [""];
  for (const dir of dirs) {
    for (const suffix of suffixes) {
      const candidate = path.join(dir, name + suffix);
      if (fs.existsSync(candidate)) return candidate;
    }
  }
  return null;
}

export function requireBinary(name, installHint) {
  const resolved = resolveBinary(name);
  if (!resolved) {
    console.error(`error: '${name}' not found on PATH — install it (${installHint}) and retry.`);
    process.exit(1);
  }
  return resolved;
}
