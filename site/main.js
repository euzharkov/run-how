/* rhow site: interactive hero terminal, tabs, copy buttons, reveal-on-scroll.
   No dependencies. The terminal is a small model of one sample project ("my-app")
   rendered the way the real binary renders it. */
(() => {
  "use strict";
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;

  /* ------------------------------------------------------------------ */
  /* Sample project: mirrors rhow's npm-basic fixture output              */
  /* ------------------------------------------------------------------ */
  const PROJECT = {
    name: "my-app",
    techs: ["Node.js", "Vite", "Vitest", "Prisma", "Playwright"],
    tool: "npm",
    actions: [
      a("dev", "vite", "Start Vite development server", "development", { notes: ["long-running"] }),
      a("build", "tsc && vite build", "Compile TypeScript sources, then build app with Vite", "build"),
      a("preview", "vite preview", "Preview production build with Vite", "development", { notes: ["long-running"] }),
      a("test", "vitest", "Run Vitest tests", "testing"),
      a("test:e2e", "playwright test", "Run Playwright end-to-end tests", "testing"),
      a("lint", "eslint .", "Check source code with ESLint", "quality"),
      a("typecheck", "tsc --noEmit", "Check TypeScript types", "quality"),
      a("format", 'prettier --write "src/**/*.{ts,tsx}"', "Format source code with Prettier", "quality"),
      a("db:migrate", "prisma migrate deploy", "Apply pending Prisma migrations", "database"),
      a("db:reset", "prisma migrate reset --force", "Reset database and re-apply migrations", "database", { risk: "destructive" }),
      a("release", "npm publish", "Publish package to npm", "release", { risk: "external" }),
      a("start", "NODE_ENV=production node dist/server.js", "Run dist/server.js", "development"),
      a("prepare", "husky", "Install git hooks with Husky", "quality", { notes: ["git"] }),
      a("postinstall", "patch-package", "Apply patches to dependencies", "build"),
    ],
  };
  function a(id, runs, description, category, extra = {}) {
    return { id, name: id, command: "npm run " + id, runs, description, category,
      risk: "safe", notes: [], ...extra };
  }
  const GROUP_ORDER = ["development", "build", "release", "testing", "quality", "database"];

  /* ------------------------------------------------------------------ */
  /* Rendering helpers (escape everything, then add spans)                */
  /* ------------------------------------------------------------------ */
  const esc = (s) => s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const pad = (s, n) => s + " ".repeat(Math.max(0, n - s.length));

  function labels(x) {
    const out = [];
    for (const n of x.notes) {
      const glyph = { "long-running": "∞", device: "⊙", download: "↓", git: "∆" }[n] || "";
      out.push(`<span class="t-dim">${glyph} ${n}</span>`);
    }
    if (x.risk !== "safe") out.push(`<span class="${x.risk === "destructive" ? "t-red" : "t-yel"}">● ${x.risk}</span>`);
    return out.join("  ");
  }
  function commandRow(x, width) {
    const l = labels(x);
    const desc = esc(x.description) + (l ? "  " + l : "");
    return { cls: "line line--cmd", html: `<span class="c">${esc(pad(x.command, width))}</span><span class="d">${desc}</span>` };
  }
  /* rows in listing order for the given flags */
  function rows(group) {
    const acts = PROJECT.actions;
    if (!group) return acts;
    return GROUP_ORDER.flatMap((cat) => acts.filter((x) => x.category === cat));
  }
  function title() {
    return { cls: "line", html: `<span class="t-title">${esc(PROJECT.name)}</span>  <span class="t-dim">[${PROJECT.techs.join(", ")}]</span>` };
  }
  const BLANK = { cls: "blank", html: "" };
  const plain = (s, cls = "") => ({ cls: "line", html: cls ? `<span class="${cls}">${esc(s)}</span>` : esc(s) });

  function renderList(group) {
    const acts = rows(group);
    const w = Math.max(...acts.map((x) => x.command.length));
    const lines = [title(), BLANK];
    let prev = null;
    acts.forEach((x) => {
      if (group && prev && prev !== x.category) lines.push(BLANK);
      prev = x.category;
      lines.push(commandRow(x, w));
    });
    return lines;
  }
  function renderJson(obj) {
    const s = JSON.stringify(obj, null, 2)
      .split("\n").map((line) => esc(line)
        .replace(/^(\s*)(&quot;[^&]*&quot;):/, '$1<span class="t-key">$2</span>:')
        .replace(/: (&quot;.*&quot;)(,?)$/, ': <span class="t-str">$1</span>$2')
        .replace(/: (\d+|true|false|null)(,?)$/, ': <span class="t-num">$1</span>$2'));
    return s.map((html) => ({ cls: "line", html }));
  }
  const jsonAction = (x) => ({ id: x.id, name: x.name, description: x.description, command: x.command,
    working_directory: ".", source: "declared", confidence: "exact", risk: x.risk, category: x.category,
    tool: PROJECT.tool, notes: x.notes, opaque: false });
  function renderJsonModel() {
    return renderJson({ schema: 2, version: "0.1.0", root: "/home/you/code/my-app", name: PROJECT.name,
      projects: [{ name: PROJECT.name, path: ".", kind: "java-script", tools: [PROJECT.tool], techs: PROJECT.techs,
        actions: PROJECT.actions.slice(0, 4).map(jsonAction).concat(["…"]) }] });
  }

  const HELP = [
    plain("Discover how to run and operate any repository: one command lists the useful actions of a project"),
    BLANK,
    plain("Usage: rhow [OPTIONS] [DIR]", "t-title"),
    plain("       rhow support [OPTIONS] [DIR]", "t-title"),
    BLANK,
    plain("Options:"),
    plain("  -g, --group            Order each project's commands by type (run, build, deploy, test, …)"),
    plain("      --ci               Show the CI pipelines (GitHub Actions, GitLab CI) as workflows, jobs and steps"),
    plain("      --json             Output as JSON"),
    plain("  -h, --help             Print help"),
    plain("  -V, --version          Print version"),
    BLANK,
    plain("Examples:"),
    plain("  rhow                 List actions grouped by category"),
    plain("  rhow --group         Order each project's commands by type"),
    plain("  rhow path/to/dir     Inspect exactly that directory"),
    plain("  rhow support         Show which tool versions this build has been verified against"),
  ];
  const SUPPORT = [
    plain("Toolsets this build of rhow has been verified against", "t-title"),
    { cls: "line", html: `  <span class="t-dim">Node.js      </span> up to Node 24` },
    { cls: "line", html: `  <span class="t-dim">npm          </span> up to npm 11` },
    { cls: "line", html: `  <span class="t-dim">pnpm         </span> up to pnpm 10` },
    { cls: "line", html: `  <span class="t-dim">Rust / Cargo </span> editions 2015-2024` },
    { cls: "line", html: `  <span class="t-dim">Go           </span> up to Go 1.26` },
    { cls: "line", html: `  <span class="t-dim">Python       </span> up to Python 3.14` },
    { cls: "line", html: `  <span class="t-dim">…            </span>` },
    BLANK,
    plain("Detected in my-app", "t-title"),
    { cls: "line", html: `  <span class="t-dim">npm </span> 10.5.0  package.json  (packageManager)` },
  ];
  const CI = [
    { cls: "line", html: `<span class="t-title">CI</span>  <span class="t-dim">.github/workflows/ci.yml  on push main, pull_request</span>` },
    { cls: "line", html: `  <span class="t-title">test</span>  <span class="t-dim">ubuntu-latest</span>` },
    { cls: "line", html: `    <span class="t-cmd">npm ci       </span>  Install dependencies with npm  <span class="t-dim">↓ download</span>` },
    { cls: "line", html: `    <span class="t-cmd">npm test     </span>  Run Vitest tests` },
    { cls: "line", html: `    <span class="t-cmd">npm run build</span>  Compile TypeScript sources, then build app with Vite` },
  ];

  /* ------------------------------------------------------------------ */
  /* Command dispatch                                                     */
  /* ------------------------------------------------------------------ */
  function run(input) {
    const argv = input.trim().split(/\s+/).filter(Boolean);
    if (!argv.length) return [];
    const [cmd, ...rest] = argv;
    if (cmd === "clear") return "clear";
    if (cmd === "help") return [plain("This is a demo shell. Try: rhow, rhow --group, rhow --ci, rhow --json, rhow support", "t-dim")];
    if (cmd === "cd" || cmd === "ls" || cmd === "cat" || cmd === "pwd")
      return [plain(`${cmd}: this demo only runs rhow. Try \`rhow\`.`, "t-dim")];
    if (cmd !== "rhow") return [plain(`sh: command not found: ${cmd}`, "t-red"), plain("Try `rhow`.", "t-dim")];

    const flags = new Set(rest.filter((x) => x.startsWith("-")));
    const args = rest.filter((x) => !x.startsWith("-"));
    const json = flags.has("--json");
    if (flags.has("-h") || flags.has("--help")) return HELP;
    if (flags.has("-V") || flags.has("--version")) return [plain("rhow 0.1.0")];
    const oneRepo = [plain("rhow: this demo has one repository, my-app. Install rhow to inspect your own.", "t-dim")];
    if (flags.has("-C") || flags.has("--directory")) return oneRepo;
    for (const f of flags) if (!["-g", "--group", "--ci", "--json", "--color", "--no-runtime"].includes(f))
      return [plain(`error: unexpected argument '${f}' found`, "t-red"), plain("For more information, try '--help'.", "t-dim")];
    if (flags.has("--group") && json) return [plain("error: --group is a terminal ordering and cannot be combined with --json", "t-red")];

    const group = flags.has("-g") || flags.has("--group");
    if (args[0] === "support") return args[1] ? oneRepo : json ? renderJson({ schema: 2, tools: "…" }) : SUPPORT;
    if (args[0]) return oneRepo;
    if (flags.has("--ci")) return json ? renderJson({ schema: 2, pipelines: "…" }) : CI;
    if (json) return renderJsonModel();
    return renderList(group);
  }

  /* ------------------------------------------------------------------ */
  /* Terminal UI                                                          */
  /* ------------------------------------------------------------------ */
  const term = document.getElementById("hero-term");
  if (term) {
    const screen = term.querySelector(".term__screen");
    const out = document.getElementById("hero-out");
    const form = document.getElementById("hero-form");
    const input = document.getElementById("hero-input");
    const typed = term.querySelector(".term__typed");
    const history = [];
    let hist = -1, busy = false, autoplayed = false;

    const sync = () => { typed.textContent = input.value; };
    input.addEventListener("input", () => { sync(); toBottom(); });
    input.addEventListener("focus", () => toBottom());
    term.addEventListener("click", (e) => { if (!e.target.closest("a")) input.focus({ preventScroll: true }); });

    function echo(cmdText) {
      const el = document.createElement("div");
      el.className = "line";
      el.innerHTML = `<span class="t-ps">$</span> ${esc(cmdText)}`;
      out.appendChild(el);
      toBottom();
    }
    function append(lines, animate) {
      return new Promise((resolve) => {
        const step = reduced || !animate ? 0 : 22;
        let i = 0;
        const tick = () => {
          if (i >= lines.length) { resolve(); return; }
          const l = lines[i++];
          const el = document.createElement("div");
          el.className = l.cls;
          el.innerHTML = l.html;
          if (step) el.style.animationDelay = "0ms";
          out.appendChild(el);
          toBottom();
          step ? setTimeout(tick, l === BLANK ? 0 : step) : tick();
        };
        tick();
      });
    }
    let pending = null;
    async function submit(text, animate = true) {
      if (busy) { pending = text; return; } // typed while output was still printing: run it next
      busy = true;
      screen.setAttribute("data-typing", "");
      const trimmed = text.trim();
      echo(trimmed);
      input.value = ""; sync();
      if (trimmed) { history.unshift(trimmed); hist = -1; }
      const result = run(trimmed);
      if (result === "clear") out.innerHTML = "";
      else {
        if (result.length) out.appendChild(Object.assign(document.createElement("div"), { className: "blank" }));
        await append(result, animate);
        if (result.length) out.appendChild(Object.assign(document.createElement("div"), { className: "blank" }));
      }
      screen.removeAttribute("data-typing");
      busy = false;
      toBottom();
      if (pending !== null) { const next = pending; pending = null; submit(next, animate); }
    }
    function toBottom() { screen.scrollTop = screen.scrollHeight; }
    form.addEventListener("submit", (e) => { e.preventDefault(); submit(input.value); });
    input.addEventListener("keydown", (e) => {
      if (e.key === "ArrowUp") { e.preventDefault(); if (hist < history.length - 1) { hist++; input.value = history[hist]; sync(); } }
      else if (e.key === "ArrowDown") { e.preventDefault(); hist = Math.max(-1, hist - 1); input.value = hist < 0 ? "" : history[hist]; sync(); }
      else if (e.key === "Tab" && !e.shiftKey && input.value && "rhow".startsWith(input.value) && input.value !== "rhow") { e.preventDefault(); input.value = "rhow"; sync(); }
      else if (e.key === "l" && e.ctrlKey) { e.preventDefault(); out.innerHTML = ""; }
      else if (e.key === "c" && e.ctrlKey) { e.preventDefault(); input.value = ""; sync(); }
    });

    /* autoplay: type `rhow` on load, unless the visitor got there first */
    async function autoplay() {
      if (autoplayed) return;
      autoplayed = true;
      if (reduced) { await submit("rhow", false); return; }
      await new Promise((r) => setTimeout(r, 700));
      if (input.value || document.activeElement === input) { return; }
      screen.setAttribute("data-typing", "");
      for (const ch of "rhow") {
        input.value += ch; sync();
        await new Promise((r) => setTimeout(r, 90 + Math.random() * 70));
      }
      await new Promise((r) => setTimeout(r, 260));
      screen.removeAttribute("data-typing");
      await submit("rhow");
    }
    input.addEventListener("focus", () => { autoplayed = true; }, { once: true });
    const inView = () => { const r = term.getBoundingClientRect(); return r.top < innerHeight * 0.85 && r.bottom > 0; };
    if (inView()) autoplay();
    else if ("IntersectionObserver" in window) {
      new IntersectionObserver((entries, obs) => {
        if (entries.some((e) => e.isIntersecting)) { obs.disconnect(); autoplay(); }
      }, { threshold: 0.35 }).observe(term);
    } else autoplay();
  }

  /* ------------------------------------------------------------------ */
  /* Tabs (WAI-ARIA pattern, arrow-key navigation)                        */
  /* ------------------------------------------------------------------ */
  document.querySelectorAll("[data-tabs]").forEach((root) => {
    const tabs = [...root.querySelectorAll('[role="tab"]')];
    const panels = tabs.map((t) => document.getElementById(t.getAttribute("aria-controls")));
    const select = (i, focus) => {
      tabs.forEach((t, j) => { const on = i === j; t.setAttribute("aria-selected", on); t.tabIndex = on ? 0 : -1; panels[j].hidden = !on; });
      if (focus) tabs[i].focus();
    };
    tabs.forEach((t, i) => {
      t.addEventListener("click", () => select(i));
      t.addEventListener("keydown", (e) => {
        const map = { ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: tabs.length - 1 };
        if (e.key in map) { e.preventDefault(); select((map[e.key] + tabs.length) % tabs.length, true); }
      });
    });
  });

  /* ------------------------------------------------------------------ */
  /* Copy buttons                                                         */
  /* ------------------------------------------------------------------ */
  document.querySelectorAll("[data-copy]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const el = document.querySelector(btn.dataset.copy);
      try {
        await navigator.clipboard.writeText(el.textContent.trim());
        btn.textContent = "Copied"; btn.setAttribute("data-done", "");
      } catch { btn.textContent = "Select"; getSelection().selectAllChildren(el); }
      setTimeout(() => { btn.textContent = "Copy"; btn.removeAttribute("data-done"); }, 1600);
    });
  });

  /* ------------------------------------------------------------------ */
  /* Reveal on scroll                                                     */
  /* ------------------------------------------------------------------ */
  const reveals = document.querySelectorAll(".reveal");
  if (reduced || !("IntersectionObserver" in window)) reveals.forEach((el) => el.classList.add("is-in"));
  else {
    const io = new IntersectionObserver((entries) => {
      entries.forEach((e) => { if (e.isIntersecting) { e.target.classList.add("is-in"); io.unobserve(e.target); } });
    }, { rootMargin: "0px 0px -8% 0px", threshold: 0.08 });
    reveals.forEach((el) => io.observe(el));
  }
})();
