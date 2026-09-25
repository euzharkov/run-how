#!/usr/bin/env python3
"""Run rhow on every cloned repository, save its output, and grade coverage and output quality.

Repositories are cloned one at a time (depth 1, no tags), checked, then deleted, so the
disk only ever holds one of them. Pass repository slugs or stack names as arguments to run
a subset: `check.py go` or `check.py BurntSushi/ripgrep`.

Every repository is pinned to a commit and rhow's default view on it is compared with
expected/<owner>__<repo>.txt. UPDATE_EXPECTED=1 records the current output instead (mount
tests/integration/expected read-write for that). The commands in must.txt were chosen by hand
and are never re-recorded: each must stay in the default view whatever the expected file says.
rhow shows every action it reports, so the other modes are checked against the default view:
--group lists the same rows in another order, --ci prints one row per CI step, and --json holds
exactly the actions the listing shows.

Logs land in $LOGS (default /logs): one directory per repository with the default, --group,
--ci and --json outputs, plus summary.md and summary.json across all of them.
Exit status is non-zero when rhow crashes, times out, emits invalid JSON, misses the stack a
repository was chosen for, drops a must-have command, or when the modes disagree.
"""
import difflib, json, os, re, shutil, subprocess, sys, time
from collections import defaultdict

WORK, LOGS, RHOW = "/work", os.environ.get("LOGS", "/logs"), "rhow"
TIMEOUT = 60
# Size control: a checkout above MAX_MB is reported and not checked. The whole tree at the
# commit is fetched: a blob filter would not help, since checkout fetches every missing blob.
MAX_MB = int(os.environ.get("MAX_MB", "300"))
REPOS_FILE = os.environ.get("REPOS_FILE", "/integration/repos.txt")
EXPECTED = os.environ.get("EXPECTED", "/integration/expected")
MUST_FILE = os.environ.get("MUST_FILE", "/integration/must.txt")
# A repository is a thin test case when rhow finds little to exercise (few actions,
# or a low complexity score). Such picks only prove detection, not the rules.
THIN_VISIBLE, THIN_SCORE = 8, 25
# Stack code in repos.txt -> names, one of which rhow must report as some project's kind, tool
# or tech (whole name, case-insensitive: "go" is not satisfied by "cargo" or "Django"). Only
# listed where it differs from the code itself.
STACK_TECH = {"node": ("node.js",), "compose": ("compose",), "swift": ("swift", "xcode"),
              "expo": ("expo", "react native"), "taskfile": ("task",), "dotnet": (".net",)}
SLOW_MS = 2000

def run(args, cwd):
    t = time.monotonic()
    try:
        p = subprocess.run([RHOW, "--no-runtime", "--color", "never", *args], cwd=cwd,
                           capture_output=True, text=True, timeout=TIMEOUT)
        return p.returncode, p.stdout, p.stderr, int((time.monotonic() - t) * 1000)
    except subprocess.TimeoutExpired:
        return "timeout", "", "", TIMEOUT * 1000

def entries():
    by_slug = defaultdict(lambda: {"stacks": [], "expect": []})
    for line in open(REPOS_FILE):
        if line.strip() and not line.startswith("#"):
            stack, slug, commit = line.split()[:3]
            by_slug[slug]["stacks"].append(stack)
            by_slug[slug]["expect"].append(STACK_TECH.get(stack, (stack,)))
            by_slug[slug]["commit"] = commit
    for line in open(MUST_FILE):
        if line.strip() and not line.startswith("#"):
            slug, path, command = line.split(None, 2)
            if slug not in by_slug:
                sys.exit(f"must.txt names {slug}, which repos.txt does not list")
            if path != "-":  # "-" records why a product has no must-have yet
                by_slug[slug].setdefault("must", []).append((path, command.strip()))
    return by_slug

def fetch_commit(slug, commit, d):
    """Check out exactly `commit`: depth 1, no tags."""
    os.makedirs(d)
    steps = [["git", "init", "--quiet"],
             ["git", "remote", "add", "origin", f"https://github.com/{slug}.git"],
             ["git", "fetch", "--quiet", "--depth", "1", "--no-tags", "origin", commit],
             ["git", "checkout", "--quiet", "FETCH_HEAD"]]
    for step in steps:
        p = subprocess.run(step, cwd=d, capture_output=True, text=True)
        if p.returncode != 0:
            return p
    return p

def grade(slug, spec):
    d = os.path.join(WORK, slug.replace("/", "_"))
    for _ in range(2):  # one retry: GitHub occasionally drops a shallow fetch
        shutil.rmtree(d, ignore_errors=True)
        clone = fetch_commit(slug, spec["commit"], d)
        if clone.returncode == 0:
            break
    try:
        if clone.returncode == 0:
            mb = int(subprocess.run(["du", "-sm", d], capture_output=True, text=True).stdout.split()[0])
            if mb > MAX_MB:
                return {"repo": slug, "stacks": spec["stacks"], "warnings": [], "checkout_mb": mb,
                        "problems": [f"checkout {mb} MB exceeds MAX_MB={MAX_MB}; pick a smaller repo"]}
        r = check(slug, spec, d, clone)
        if clone.returncode == 0:
            r["checkout_mb"] = mb
        return r
    finally:
        shutil.rmtree(d, ignore_errors=True)

def check(slug, spec, d, clone):
    out = os.path.join(LOGS, slug.replace("/", "_"))
    os.makedirs(out, exist_ok=True)
    r = {"repo": slug, "stacks": spec["stacks"], "problems": [], "warnings": []}
    if clone.returncode != 0:
        last = (clone.stderr.strip().splitlines() or [f"exit {clone.returncode}"])[-1]
        r["problems"].append("clone failed: " + last[:120])
        return r
    for name, args in [("default", []), ("group", ["--group"]), ("ci", ["--ci"]), ("json", ["--json"])]:
        code, so, se, ms = run(args, d)
        open(os.path.join(out, f"{name}.txt" if name != "json" else "model.json"), "w").write(so)
        if se:
            open(os.path.join(out, f"{name}.stderr"), "w").write(se)
        r[f"{name}_ms"] = ms
        if code == "timeout":
            r["problems"].append(f"{name}: timed out after {TIMEOUT}s")
        elif code != 0:
            r["problems"].append(f"{name}: exit {code}")
        if "panicked" in se:
            r["problems"].append(f"{name}: panic")
        if name == "json" and code == 0:
            try:
                model = json.loads(so)
            except ValueError as e:
                r["problems"].append(f"json: invalid ({e})")
                continue
            quality(r, model, spec)
            must_have(r, spec, model)
            if not any(p.split(":")[0] in ("default", "group", "ci") for p in r["problems"]):
                modes_agree(r, out, model)
    compare_expected(r, slug, out)
    # Speed: wall-clock from starting rhow until its output is fully written, per mode.
    with open(os.path.join(out, "timing.txt"), "w") as f:
        for name in ["default", "group", "ci", "json"]:
            f.write(f"{name:<8} {r.get(name + '_ms', '-')} ms\n")
    if r.get("default_ms", 0) > SLOW_MS:
        r["warnings"].append(f"slow: {r['default_ms']} ms")
    if r.get("default_ms") is not None:
        lines = open(os.path.join(out, "default.txt")).read().count("\n")
        r["default_lines"] = lines
        if lines > 150:
            r["warnings"].append(f"default view is long: {lines} lines")
    return r

def compare_expected(r, slug, out):
    """The default view is the specification of rhow's behaviour on this commit. It must equal
    expected/<owner>__<repo>.txt; UPDATE_EXPECTED=1 rewrites that file instead."""
    actual = open(os.path.join(out, "default.txt")).read()
    path = os.path.join(EXPECTED, slug.replace("/", "__") + ".txt")
    diff_path = os.path.join(out, "default.diff")
    if os.path.exists(diff_path):  # from an earlier run; only a fresh mismatch may leave one
        os.remove(diff_path)
    if os.environ.get("UPDATE_EXPECTED") == "1":
        os.makedirs(EXPECTED, exist_ok=True)
        open(path, "w").write(actual)
        return
    if not os.path.exists(path):
        r["problems"].append("no expected output; run with UPDATE_EXPECTED=1 to record it")
        return
    expected = open(path).read()
    if expected != actual:
        diff = difflib.unified_diff(expected.splitlines(True), actual.splitlines(True), "expected", "actual")
        open(diff_path, "w").writelines(diff)
        r["problems"].append("default view differs from expected output (see default.diff)")

def quality(r, model, spec):
    projects = model.get("projects", [])
    actions = [a for p in projects for a in p.get("actions", [])]
    visible = actions  # nothing is hidden: every action is in the listing
    r.update(projects=len(projects), actions=len(actions),
             declared=sum(a.get("source") == "declared" for a in actions))
    # "Run <program>" means rhow had no knowledge of the command: an explanation gap.
    generic = [a for a in visible if re.fullmatch(r"Run \S+", a.get("description") or "")]
    r["unexplained"] = len(generic)
    r["unexplained_sample"] = sorted({a["command"].split()[0] for a in generic if a.get("command")})[:10]
    r["no_description"] = sum(not a.get("description") for a in visible)
    ids = [a["id"] for a in actions]
    if len(ids) != len(set(ids)):
        r["problems"].append("duplicate action ids")
    names = {n.lower() for p in projects for n in [p.get("kind") or ""] + p.get("tools", []) + p.get("techs", [])}
    missing = [" or ".join(e) for e in spec["expect"] if not any(x.lower() in names for x in e)]
    if missing:
        r["problems"].append("stack not detected: " + ", ".join(missing))
    if not visible:
        r["problems"].append("no actions")
    # Complexity: how much of rhow's rule set this repository exercises. Used to choose
    # repositories, never to pass or fail them.
    tools = {t for p in projects for t in p.get("tools", [])}
    techs = {t for p in projects for t in p.get("techs", [])}
    categories = {a.get("category") for a in visible}
    r["score"] = (min(len(projects), 20) + min(r["declared"], 100) // 5 + 2 * len(tools)
                  + 2 * len(techs) + 3 * len(categories))
    if len(visible) < THIN_VISIBLE or r["score"] < THIN_SCORE:
        r["warnings"].append(f"thin test case: {len(visible)} actions, score {r['score']}")
    if visible and r["unexplained"] / len(visible) > 0.25:
        r["warnings"].append(f"{r['unexplained']}/{len(visible)} actions unexplained")

def must_have(r, spec, model):
    """Every hand-picked command in must.txt is listed, in the project at that path."""
    shown = {(p.get("path"), a.get("command")) for p in model.get("projects", [])
             for a in p.get("actions", [])}
    for path, command in spec.get("must", []):
        if (path, command) not in shown:
            r["problems"].append(f"must-have not in the default view: {path}: {command}")

def rows_by_project(text):
    """Action rows of a listing (two-space indent), whitespace-normalised, per project header."""
    sections, current = {}, None
    for line in text.splitlines():
        if line and not line.startswith(" "):
            current = line.split("  ")[0]
            sections.setdefault(current, [])
        elif line.startswith("  ") and not line.startswith("   ") and current is not None:
            sections[current].append(" ".join(line.split()))
    return sections

def modes_agree(r, out, model):
    """One check per extra mode, each against what the others print for the same commit."""
    read = lambda name: open(os.path.join(out, name)).read()
    default, grouped = rows_by_project(read("default.txt")), rows_by_project(read("group.txt"))
    # --group: the same rows under the same projects, only reordered.
    moved = [p for p in set(default) | set(grouped)
             if sorted(default.get(p, [])) != sorted(grouped.get(p, []))]
    if moved:
        r["problems"].append(f"--group lists other rows than the default view in {len(moved)} project(s), first: {moved[0]}")
    # --ci: one four-space-indented row per step; a description spanning lines breaks it.
    steps = sum(len(j.get("steps", [])) for p in model.get("ci", []) for j in p.get("jobs", []))
    step_rows = sum(line.startswith("    ") for line in read("ci.txt").splitlines())
    if step_rows != steps:
        r["problems"].append(f"--ci prints {step_rows} step rows for {steps} CI steps")
    # --json: exactly the actions the listing shows, one row each.
    actions = [a for p in model.get("projects", []) for a in p.get("actions", [])]
    rows = sum(map(len, default.values()))
    if len(actions) != rows:
        r["problems"].append(f"--json has {len(actions)} actions but the listing shows {rows} rows")

def speed_section(results):
    """How long `rhow` takes from start until its default view is fully printed."""
    timed = sorted((r["default_ms"], r["repo"]) for r in results if "default_ms" in r)
    if not timed:
        return "No timings."
    ms = [t for t, _ in timed]
    median = ms[len(ms) // 2]
    p95 = ms[min(len(ms) - 1, int(len(ms) * 0.95))]
    slowest = ", ".join(f"{repo} {t} ms" for t, repo in reversed(timed[-5:]))
    return (f"Speed of `rhow` (default view, start to last byte): median {median} ms, "
            f"p95 {p95} ms, max {ms[-1]} ms. Slowest: {slowest}.")

def main():
    os.makedirs(LOGS, exist_ok=True)
    only = set(sys.argv[1:])
    todo = [(s, spec) for s, spec in sorted(entries().items())
            if not only or s in only or only & set(spec["stacks"])]
    results = []
    for i, (s, spec) in enumerate(todo, 1):
        r = grade(s, spec)
        print(f"[{i}/{len(todo)}] {s}: " + ("; ".join(r["problems"]) or "ok"), file=sys.stderr, flush=True)
        results.append(r)
    json.dump(results, open(os.path.join(LOGS, "summary.json"), "w"), indent=2)
    bad = [r for r in results if r["problems"]]
    md = ["# rhow real-repository integration run", "",
          f"{len(results)} repositories, {len(bad)} with problems, "
          f"{sum(bool(r['warnings']) for r in results)} with warnings.", "",
          f"Peak disk use is one checkout: largest {max((r.get('checkout_mb', 0) for r in results), default=0)} MB, "
          f"sum {sum(r.get('checkout_mb', 0) for r in results)} MB cloned over the run.", "",
          speed_section(results), "",
          "| repo | stacks | MB | score | projects | actions | declared | unexplained | rhow ms | lines | problems / warnings |",
          "| --- | --- | --: | --: | --: | --: | --: | --: | --: | --: | --- |"]
    for r in results:
        notes = "; ".join(["**" + p + "**" for p in r["problems"]] + r["warnings"])
        if r.get("unexplained_sample"):
            notes += (" " if notes else "") + "unexplained: `" + " ".join(r["unexplained_sample"]) + "`"
        md.append(f"| {r['repo']} | {','.join(r['stacks'])} | {r.get('checkout_mb','-')} | {r.get('score','-')} | {r.get('projects','-')} | "
                  f"{r.get('actions','-')} | {r.get('declared','-')} | "
                  f"{r.get('unexplained','-')} | {r.get('default_ms','-')} | {r.get('default_lines','-')} | {notes} |")
    open(os.path.join(LOGS, "summary.md"), "w").write("\n".join(md) + "\n")
    print("\n".join(md))
    sys.exit(1 if bad else 0)

main()
