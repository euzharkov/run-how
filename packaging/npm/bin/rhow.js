#!/usr/bin/env node
// Launcher: execs the native rhow binary installed next to this file.
"use strict";
const path = require("path");
const { spawnSync } = require("child_process");
const bin = path.join(__dirname, process.platform === "win32" ? "rhow.exe" : "rhow");
const r = spawnSync(bin, process.argv.slice(2), { stdio: "inherit" });
if (r.error) {
  console.error(`rhow: native binary missing (${r.error.message}). Reinstall @euzharkov/rhow.`);
  process.exit(127);
}
process.exit(r.status == null ? 1 : r.status);
