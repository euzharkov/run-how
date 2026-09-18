// Downloads the matching rhow release binary from GitHub Releases and verifies its SHA256.
// Node is only a distribution layer; the application itself is the native Rust binary.
"use strict";
const fs = require("fs");
const path = require("path");
const https = require("https");
const crypto = require("crypto");
const zlib = require("zlib");
const { execFileSync } = require("child_process");

const pkg = require("./package.json");
const version = pkg.version;
const repo = "line-19/rhow";

const targets = {
  "darwin-arm64": "aarch64-apple-darwin",
  "darwin-x64": "x86_64-apple-darwin",
  "linux-arm64": "aarch64-unknown-linux-musl",
  "linux-x64": "x86_64-unknown-linux-musl",
  "win32-arm64": "aarch64-pc-windows-msvc",
  "win32-x64": "x86_64-pc-windows-msvc",
};
const key = `${process.platform}-${process.arch}`;
const target = targets[key];
if (!target) {
  console.error(`rhow: unsupported platform ${key}`);
  process.exit(1);
}
const isWin = process.platform === "win32";
const name = `rhow-${version}-${target}`;
const archive = isWin ? `${name}.zip` : `${name}.tar.gz`;
const base = `https://github.com/${repo}/releases/download/v${version}`;

function get(url) {
  return new Promise((resolve, reject) => {
    https.get(url, { headers: { "user-agent": "rhow-npm" } }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) return resolve(get(res.headers.location));
      if (res.statusCode !== 200) return reject(new Error(`${url}: HTTP ${res.statusCode}`));
      const chunks = [];
      res.on("data", (c) => chunks.push(c));
      res.on("end", () => resolve(Buffer.concat(chunks)));
      res.on("error", reject);
    }).on("error", reject);
  });
}

(async () => {
  const binDir = path.join(__dirname, "bin");
  const out = path.join(binDir, isWin ? "rhow.exe" : "rhow");
  const [data, sums] = await Promise.all([get(`${base}/${archive}`), get(`${base}/SHA256SUMS`)]);
  const line = sums.toString().split("\n").find((l) => l.trim().endsWith(archive));
  if (!line) throw new Error(`no checksum for ${archive}`);
  const expected = line.split(/\s+/)[0];
  const actual = crypto.createHash("sha256").update(data).digest("hex");
  if (expected !== actual) throw new Error(`checksum mismatch for ${archive}`);
  const tmp = fs.mkdtempSync(path.join(require("os").tmpdir(), "rhow-"));
  const archivePath = path.join(tmp, archive);
  fs.writeFileSync(archivePath, data);
  if (isWin) {
    execFileSync("powershell", ["-NoProfile", "-Command", `Expand-Archive -Path '${archivePath}' -DestinationPath '${tmp}' -Force`]);
  } else {
    execFileSync("tar", ["-xzf", archivePath, "-C", tmp]);
  }
  fs.copyFileSync(path.join(tmp, name, isWin ? "rhow.exe" : "rhow"), out);
  if (!isWin) fs.chmodSync(out, 0o755);
  fs.rmSync(tmp, { recursive: true, force: true });
  void zlib;
})().catch((e) => {
  console.error(`rhow: failed to install native binary: ${e.message}`);
  process.exit(1);
});
