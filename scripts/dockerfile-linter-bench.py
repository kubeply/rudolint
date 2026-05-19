#!/usr/bin/env python3
"""Benchmark rudolint against other Dockerfile linters.

The script intentionally lives outside the Rust workspace. It measures external
CLI tools with hyperfine and writes checked-in JSON/SVG artifacts for docs.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import shlex
import shutil
import stat
import subprocess
import sys
import time
import urllib.request
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
BENCH_ROOT = ROOT / "benchmarks" / "dockerfile-linters"
TARGET_ROOT = ROOT / "target" / "dockerfile-linter-bench"
CORPUS_ROOT = TARGET_ROOT / "corpus"
TOOLS_ROOT = TARGET_ROOT / "tools"
RESULTS_ROOT = BENCH_ROOT / "results"
NODE_TOOLS_ROOT = BENCH_ROOT / "node-tools"

HADOLINT_REPO = "hadolint/hadolint"
TALLY_NPM = "tally-cli"

HADOLINT_VERSION = "v2.14.0"
TALLY_VERSION = "0.41.0"

SCENARIOS = {
    "small": "Single small Dockerfile",
    "buildkit": "Single BuildKit-heavy Dockerfile",
    "repo-100": "Repository with 100 generated Dockerfiles",
    "repo-1000": "Repository with 1,000 generated Dockerfiles",
    "json-100": "JSON output for 100 generated Dockerfiles",
    "sarif-100": "SARIF output for 100 generated Dockerfiles",
}

HEADLINE_SCENARIO = "repo-1000"


@dataclass(frozen=True)
class Tool:
    key: str
    name: str


TOOLS = [
    Tool("rudolint", "kubeply/rudolint"),
    Tool("hadolint", "hadolint/hadolint"),
    Tool("tally", "wharflab/tally"),
    Tool("docker-build-check", "docker build --check"),
]

SARIF_TOOLS = {"rudolint", "hadolint", "tally"}
JSON_TOOLS = {"rudolint", "hadolint", "tally"}


def run(
    args: list[str],
    *,
    cwd: Path = ROOT,
    env: dict[str, str] | None = None,
    check: bool = True,
    capture: bool = False,
) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        args,
        cwd=cwd,
        env=env,
        check=check,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
    )


def executable_suffix() -> str:
    return ".exe" if platform.system() == "Windows" else ""


def host_asset_suffix() -> str:
    system = platform.system()
    machine = platform.machine().lower()

    if system == "Darwin":
        os_name = "macos"
    elif system == "Linux":
        os_name = "linux"
    else:
        raise RuntimeError(f"unsupported benchmark host OS: {system}")

    if machine in {"arm64", "aarch64"}:
        arch = "arm64"
    elif machine in {"x86_64", "amd64"}:
        arch = "x86_64"
    else:
        raise RuntimeError(f"unsupported benchmark host architecture: {machine}")

    return f"{os_name}-{arch}"


def ensure_hadolint(version: str) -> Path:
    binary = TOOLS_ROOT / "hadolint" / version / f"hadolint{executable_suffix()}"
    if binary.exists():
        return binary

    binary.parent.mkdir(parents=True, exist_ok=True)
    asset = f"hadolint-{host_asset_suffix()}"
    url = f"https://github.com/{HADOLINT_REPO}/releases/download/{version}/{asset}"
    with urllib.request.urlopen(url, timeout=120) as response:
        binary.write_bytes(response.read())
    binary.chmod(binary.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    return binary


def ensure_node_tools(tally_version: str) -> Path:
    package_root = TOOLS_ROOT / "node"
    package_root.mkdir(parents=True, exist_ok=True)
    package_json = package_root / "package.json"
    package_lock = package_root / "package-lock.json"
    if tally_version == TALLY_VERSION:
        shutil.copyfile(NODE_TOOLS_ROOT / "package.json", package_json)
        shutil.copyfile(NODE_TOOLS_ROOT / "package-lock.json", package_lock)
        run(["npm", "ci", "--silent", "--prefix", str(package_root)])
    else:
        package_json.write_text(
            json.dumps(
                {
                    "private": True,
                    "dependencies": {
                        TALLY_NPM: tally_version,
                    },
                },
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        run(["npm", "install", "--silent", "--prefix", str(package_root)])
    return package_root / "node_modules" / ".bin"


def ensure_rudolint() -> Path:
    run(["cargo", "build", "-p", "rudolint", "--release", "--locked"])
    return ROOT / "target" / "release" / f"rudolint{executable_suffix()}"


def ensure_hyperfine() -> Path:
    hyperfine = shutil.which("hyperfine")
    if hyperfine is None:
        raise RuntimeError(
            "hyperfine is required. Install it with `brew install hyperfine`, "
            "`cargo install hyperfine`, or your OS package manager."
        )
    return Path(hyperfine)


def write_dockerfile(path: Path, body: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(body.strip() + "\n", encoding="utf-8")


def generated_dockerfile(index: int) -> str:
    distro = ["alpine:3.20", "debian:12-slim", "ubuntu:24.04"][index % 3]
    package_manager = ["apk", "apt-get", "apt"][index % 3]
    service = f"service-{index:04d}"
    port = 8000 + (index % 100)
    if package_manager == "apk":
        install = "apk add --no-cache ca-certificates curl"
    else:
        install = (
            "apt-get update && apt-get install -y --no-install-recommends "
            "ca-certificates curl && rm -rf /var/lib/apt/lists/*"
        )

    extra = ""
    if index % 5 == 0:
        extra = f"\nRUN --mount=type=cache,target=/root/.cache/{service} echo cached"
    if index % 7 == 0:
        extra += f"\nLABEL org.opencontainers.image.title=\"{service}\""

    return f"""
    # syntax=docker/dockerfile:1.7
    FROM {distro} AS base
    ARG SERVICE_NAME={service}
    WORKDIR /srv/${{SERVICE_NAME}}
    RUN {install}
    COPY src/{service}.txt ./payload.txt
    {extra}
    EXPOSE {port}
    CMD ["sh", "-c", "cat ./payload.txt >/dev/null && sleep 1"]
    """


def generate_repository(count: int) -> Path:
    repo = CORPUS_ROOT / f"repo-{count}"
    if repo.exists():
        shutil.rmtree(repo)
    bake_targets = []
    for index in range(count):
        service_dir = repo / f"services/service-{index:04d}"
        write_dockerfile(service_dir / "Dockerfile", generated_dockerfile(index))
        payload = service_dir / "src" / f"service-{index:04d}.txt"
        payload.parent.mkdir(parents=True, exist_ok=True)
        payload.write_text(f"service-{index:04d}\n", encoding="utf-8")
        bake_targets.append(f"service-{index:04d}")

    bake_lines = [
        'group "default" {',
        "  targets = [",
        *[f'    "{target}",' for target in bake_targets],
        "  ]",
        "}",
        "",
    ]
    for target in bake_targets:
        bake_lines.extend(
            [
                f'target "{target}" {{',
                f'  context = "services/{target}"',
                '  dockerfile = "Dockerfile"',
                "}",
                "",
            ]
        )
    (repo / "docker-bake.hcl").write_text("\n".join(bake_lines), encoding="utf-8")
    return repo


def prepare_corpus() -> None:
    if CORPUS_ROOT.exists():
        shutil.rmtree(CORPUS_ROOT)
    CORPUS_ROOT.mkdir(parents=True, exist_ok=True)

    shutil.copytree(ROOT / "fixtures" / "corpus" / "small", CORPUS_ROOT / "small")
    shutil.copytree(
        ROOT / "fixtures" / "corpus" / "buildkit-heavy", CORPUS_ROOT / "buildkit"
    )
    generate_repository(100)
    generate_repository(1_000)


def dockerfiles_for_scenario(scenario: str) -> list[Path]:
    if scenario == "small":
        return [CORPUS_ROOT / "small" / "Dockerfile"]
    if scenario == "buildkit":
        return [CORPUS_ROOT / "buildkit" / "Dockerfile"]
    if scenario in {"repo-100", "json-100", "sarif-100"}:
        return sorted((CORPUS_ROOT / "repo-100").glob("services/*/Dockerfile"))
    if scenario == "repo-1000":
        return sorted((CORPUS_ROOT / "repo-1000").glob("services/*/Dockerfile"))
    raise RuntimeError(f"unknown scenario: {scenario}")


def repo_root_for_scenario(scenario: str) -> Path:
    if scenario == "small":
        return CORPUS_ROOT / "small"
    if scenario == "buildkit":
        return CORPUS_ROOT / "buildkit"
    if scenario in {"repo-100", "json-100", "sarif-100"}:
        return CORPUS_ROOT / "repo-100"
    if scenario == "repo-1000":
        return CORPUS_ROOT / "repo-1000"
    raise RuntimeError(f"unknown scenario: {scenario}")


def command_path(tool: str) -> Path:
    manifest = read_manifest()
    path = manifest["commands"][tool]
    return Path(path)


def read_manifest() -> dict:
    return json.loads((TARGET_ROOT / "manifest.json").read_text(encoding="utf-8"))


def run_allowing_findings(args: list[str], cwd: Path, allowed_codes: set[int]) -> None:
    result = subprocess.run(
        args,
        cwd=cwd,
        text=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode not in allowed_codes:
        if result.stderr:
            sys.stderr.write(result.stderr)
        raise SystemExit(result.returncode)


def run_strict(args: list[str], cwd: Path) -> None:
    with open(os.devnull, "wb") as devnull:
        subprocess.run(args, cwd=cwd, stdout=devnull, stderr=devnull, check=True)


def exec_tool(tool: str, scenario: str) -> None:
    files = dockerfiles_for_scenario(scenario)
    repo_root = repo_root_for_scenario(scenario)
    binary = command_path(tool)

    if scenario.startswith("json") and tool not in JSON_TOOLS:
        raise SystemExit(2)
    if scenario.startswith("sarif") and tool not in SARIF_TOOLS:
        raise SystemExit(2)

    if tool == "rudolint":
        args = [str(binary), "check", str(repo_root), "--exit-zero", "--no-config"]
        if scenario.startswith("json"):
            args.extend(["--format", "json"])
        elif scenario.startswith("sarif"):
            args.extend(["--format", "sarif"])
        else:
            args.append("--quiet")
        run_allowing_findings(args, ROOT, {0})
        return

    if tool == "tally":
        args = [str(binary), "lint", str(repo_root)]
        if scenario.startswith("json"):
            args.extend(["--format", "json"])
        elif scenario.startswith("sarif"):
            args.extend(["--format", "sarif"])
        run_allowing_findings(args, ROOT, {0, 1})
        return

    if tool == "hadolint":
        args = [str(binary), *map(str, files)]
        if scenario.startswith("json"):
            args.extend(["--format", "json"])
        elif scenario.startswith("sarif"):
            args.extend(["--format", "sarif"])
        run_allowing_findings(args, ROOT, {0, 1})
        return

    if tool == "docker-build-check":
        if scenario in {"repo-100", "repo-1000"}:
            args = [
                str(binary),
                "buildx",
                "bake",
                "--check",
                "--progress=quiet",
                "--file",
                "docker-bake.hcl",
            ]
            run_strict(args, repo_root)
            return
        for path in files:
            args = [
                str(binary),
                "buildx",
                "build",
                "--check",
                "--progress=quiet",
                "-f",
                str(path),
                str(path.parent),
            ]
            run_strict(args, ROOT)
        return

    raise RuntimeError(f"unknown tool: {tool}")


def version_for_command(args: list[str]) -> str:
    result = run(args, capture=True, check=False)
    output = (result.stdout + result.stderr).strip()
    return output.splitlines()[0] if output else "unknown"


def setup(
    hadolint_version: str = HADOLINT_VERSION,
    tally_version: str = TALLY_VERSION,
) -> dict:
    prepare_corpus()
    TOOLS_ROOT.mkdir(parents=True, exist_ok=True)
    node_bin = ensure_node_tools(tally_version)
    docker = shutil.which("docker")
    if docker is None:
        raise RuntimeError(
            "Docker CLI is required for docker build --check benchmarks. "
            "Install Docker and ensure `docker` is on PATH."
        )
    commands = {
        "rudolint": str(ensure_rudolint()),
        "hadolint": str(ensure_hadolint(hadolint_version)),
        "tally": str(node_bin / "tally"),
        "docker-build-check": docker,
    }

    manifest = {
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "host": {
            "system": platform.system(),
            "machine": platform.machine(),
            "python": platform.python_version(),
        },
        "commands": commands,
        "versions": {
            "rudolint": version_for_command([commands["rudolint"], "--version"]),
            "hadolint": version_for_command([commands["hadolint"], "--version"]),
            "tally": version_for_command([commands["tally"], "--version"]),
            "docker-build-check": version_for_command([commands["docker-build-check"], "version", "--format", "{{.Client.Version}}"]),
        },
        "latest_sources": {
            "hadolint": f"pinned GitHub release {hadolint_version}",
            "tally": f"pinned npm {TALLY_NPM}@{tally_version}",
            "docker-build-check": "local Docker CLI",
        },
    }
    TARGET_ROOT.mkdir(parents=True, exist_ok=True)
    (TARGET_ROOT / "manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )
    return manifest


def hyperfine_command(tool: str, scenario: str) -> str:
    script = ROOT / "scripts" / "dockerfile-linter-bench.py"
    return (
        f"{shlex.quote(sys.executable)} "
        f"{shlex.quote(str(script))} "
        f"exec --tool {shlex.quote(tool)} --scenario {shlex.quote(scenario)}"
    )


def public_manifest(manifest: dict) -> dict:
    public = json.loads(json.dumps(manifest))
    public.pop("host", None)
    public["commands"] = {
        "rudolint": "target/release/rudolint",
        "hadolint": "target/dockerfile-linter-bench/tools/hadolint/<version>/hadolint",
        "tally": "target/dockerfile-linter-bench/tools/node/node_modules/.bin/tally",
        "docker-build-check": "docker",
    }
    return public


def run_hyperfine(runs: int, warmup: int, scenarios: list[str]) -> dict:
    ensure_hyperfine()
    raw_dir = TARGET_ROOT / "raw"
    raw_dir.mkdir(parents=True, exist_ok=True)

    all_results = []
    for scenario in scenarios:
        tools = [tool for tool in TOOLS if scenario_supported(tool.key, scenario)]
        args = [
            "hyperfine",
            "--warmup",
            str(warmup),
            "--runs",
            str(runs),
            "--export-json",
            str(raw_dir / f"{scenario}.json"),
        ]
        for tool in tools:
            args.extend(["--command-name", tool.name, hyperfine_command(tool.key, scenario)])
        run(args)
        payload = json.loads((raw_dir / f"{scenario}.json").read_text(encoding="utf-8"))
        for result in payload["results"]:
            all_results.append(
                {
                    "scenario": scenario,
                    "scenario_name": SCENARIOS[scenario],
                    "tool": result["command"],
                    "mean_seconds": result["mean"],
                    "stddev_seconds": result["stddev"],
                    "median_seconds": result["median"],
                    "min_seconds": result["min"],
                    "max_seconds": result["max"],
                }
            )

    return {
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "runs": runs,
        "warmup": warmup,
        "headline_scenario": HEADLINE_SCENARIO,
        "results": all_results,
        "manifest": public_manifest(read_manifest()),
    }


def scenario_supported(tool: str, scenario: str) -> bool:
    if scenario.startswith("json"):
        return tool in JSON_TOOLS
    if scenario.startswith("sarif"):
        return tool in SARIF_TOOLS
    return True


def write_results(payload: dict) -> None:
    RESULTS_ROOT.mkdir(parents=True, exist_ok=True)
    if "manifest" in payload:
        payload["manifest"] = public_manifest(payload["manifest"])
        tool_versions = payload["manifest"]
    else:
        tool_versions = {
            "generated_at": payload.get("generated_at"),
            "commands": {},
            "versions": {},
            "latest_sources": {},
        }
    (RESULTS_ROOT / "latest.json").write_text(
        json.dumps(payload, indent=2) + "\n", encoding="utf-8"
    )
    (RESULTS_ROOT / "tool-versions.json").write_text(
        json.dumps(tool_versions, indent=2) + "\n", encoding="utf-8"
    )
    render_headline_chart(payload, RESULTS_ROOT / "headline.svg")
    render_scenario_chart(payload, RESULTS_ROOT / "scenarios.svg")


def color_for_tool(tool: str) -> str:
    colors = {
        "kubeply/rudolint": "#8b5cf6",
        "hadolint/hadolint": "#64748b",
        "wharflab/tally": "#14b8a6",
        "docker build --check": "#0ea5e9",
    }
    return colors.get(tool, "#94a3b8")


def escape_svg(value: str) -> str:
    return (
        value.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )


def render_headline_chart(payload: dict, output: Path) -> None:
    rows = [
        row
        for row in payload["results"]
        if row["scenario"] == payload["headline_scenario"]
    ]
    if not rows:
        svg = """<svg xmlns="http://www.w3.org/2000/svg" width="980" height="180" viewBox="0 0 980 180" role="img" aria-labelledby="title desc">
  <title id="title">Dockerfile linter benchmark</title>
  <desc id="desc">No data available for the configured headline scenario.</desc>
  <style>
    text { fill: #cbd5e1; font: 600 18px ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
    .small { fill: #94a3b8; font-size: 16px; font-weight: 500; }
    .title { fill: #e2e8f0; font-size: 28px; font-weight: 800; }
  </style>
  <rect width="100%" height="100%" fill="#111827"/>
  <text class="title" x="32" y="54">Dockerfile linter performance</text>
  <text class="small" x="32" y="92">No results for the configured headline scenario in this run.</text>
</svg>
"""
        output.write_text(svg, encoding="utf-8")
        return
    rows.sort(key=lambda row: row["mean_seconds"])
    max_time = max(row["mean_seconds"] for row in rows)
    width = 980
    row_height = 42
    top = 70
    left = 330
    chart_width = 490
    height = top + row_height * len(rows) + 80

    bars = []
    for index, row in enumerate(rows):
        y = top + index * row_height
        bar_width = max(2, row["mean_seconds"] / max_time * chart_width)
        label = escape_svg(row["tool"])
        value = f"{row['mean_seconds']:.2f}s"
        bars.append(
            f'<text x="{left - 14}" y="{y + 24}" text-anchor="end">{label}</text>'
            f'<rect x="{left}" y="{y + 8}" width="{bar_width:.1f}" height="24" '
            f'rx="2" fill="{color_for_tool(row["tool"])}"/>'
            f'<text x="{left + bar_width + 12:.1f}" y="{y + 26}">{value}</text>'
        )

    svg = f"""<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title desc">
  <title id="title">Dockerfile linter benchmark</title>
  <desc id="desc">Mean time to lint 1,000 generated Dockerfiles.</desc>
  <style>
    text {{ fill: #cbd5e1; font: 600 18px ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }}
    .small {{ fill: #94a3b8; font-size: 14px; font-weight: 500; }}
    .title {{ fill: #e2e8f0; font-size: 28px; font-weight: 800; }}
  </style>
  <rect width="100%" height="100%" fill="#111827"/>
  <text class="title" x="32" y="42">Dockerfile linter performance</text>
  <text class="small" x="32" y="66">Linting 1,000 deterministic Dockerfiles. Lower is better.</text>
  {''.join(bars)}
  <text class="small" x="{left}" y="{height - 28}">Generated with hyperfine, {payload["runs"]} runs, {payload["warmup"]} warmup.</text>
</svg>
"""
    output.write_text(svg, encoding="utf-8")


def render_scenario_chart(payload: dict, output: Path) -> None:
    scenarios = ["small", "buildkit", "repo-100", "repo-1000", "json-100", "sarif-100"]
    visible_scenarios = [
        scenario
        for scenario in scenarios
        if any(row["scenario"] == scenario for row in payload["results"])
    ]
    width = 1120
    panel_height = 180
    height = 80 + panel_height * max(1, len(visible_scenarios))
    left = 320
    chart_width = 550
    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" role="img" aria-labelledby="title desc">',
        '<title id="title">Dockerfile linter benchmark scenarios</title>',
        '<desc id="desc">Mean benchmark time by tool and scenario.</desc>',
        "<style>",
        'text { fill: #cbd5e1; font: 600 14px ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }',
        ".small { fill: #94a3b8; font-size: 12px; font-weight: 500; }",
        ".title { fill: #e2e8f0; font-size: 26px; font-weight: 800; }",
        "</style>",
        '<rect width="100%" height="100%" fill="#111827"/>',
        '<text class="title" x="32" y="42">Benchmark scenarios</text>',
        '<text class="small" x="32" y="64">Mean seconds. Lower is better.</text>',
    ]

    for scenario_index, scenario in enumerate(visible_scenarios):
        rows = [row for row in payload["results"] if row["scenario"] == scenario]
        rows.sort(key=lambda row: row["mean_seconds"])
        max_time = max(row["mean_seconds"] for row in rows)
        panel_y = 84 + scenario_index * panel_height
        parts.append(
            f'<text x="32" y="{panel_y}" fill="#e2e8f0">{escape_svg(SCENARIOS[scenario])}</text>'
        )
        for row_index, row in enumerate(rows):
            y = panel_y + 18 + row_index * 22
            bar_width = max(2, row["mean_seconds"] / max_time * chart_width)
            parts.append(
                f'<text x="{left - 12}" y="{y + 14}" text-anchor="end">{escape_svg(row["tool"])}</text>'
                f'<rect x="{left}" y="{y + 3}" width="{bar_width:.1f}" height="14" rx="2" fill="{color_for_tool(row["tool"])}"/>'
                f'<text class="small" x="{left + bar_width + 8:.1f}" y="{y + 14}">{row["mean_seconds"]:.3f}s</text>'
            )

    parts.append("</svg>\n")
    output.write_text("".join(parts), encoding="utf-8")


def print_summary(payload: dict) -> None:
    rows = [
        row
        for row in payload["results"]
        if row["scenario"] == payload["headline_scenario"]
    ]
    rows.sort(key=lambda row: row["mean_seconds"])
    for row in rows:
        print(f"{row['tool']}: {row['mean_seconds']:.3f}s")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)

    run_parser = subcommands.add_parser("run", help="prepare tools, run hyperfine, and render charts")
    run_parser.add_argument("--runs", type=int, default=5)
    run_parser.add_argument("--warmup", type=int, default=2)
    add_version_args(run_parser)
    run_parser.add_argument(
        "--scenario",
        action="append",
        choices=sorted(SCENARIOS),
        help="scenario to run; defaults to all scenarios",
    )

    setup_parser = subcommands.add_parser(
        "setup", help="prepare corpus and tools without running benchmarks"
    )
    add_version_args(setup_parser)

    exec_parser = subcommands.add_parser("exec", help=argparse.SUPPRESS)
    exec_parser.add_argument("--tool", required=True, choices=[tool.key for tool in TOOLS])
    exec_parser.add_argument("--scenario", required=True, choices=sorted(SCENARIOS))

    render_parser = subcommands.add_parser("render", help="render charts from an existing result JSON")
    render_parser.add_argument("--input", type=Path, default=RESULTS_ROOT / "latest.json")

    return parser.parse_args()


def add_version_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--hadolint-version", default=HADOLINT_VERSION)
    parser.add_argument("--tally-version", default=TALLY_VERSION)


def setup_from_args(args: argparse.Namespace) -> dict:
    return setup(
        hadolint_version=args.hadolint_version,
        tally_version=args.tally_version,
    )


def main() -> None:
    args = parse_args()
    if args.command == "setup":
        print(json.dumps(setup_from_args(args), indent=2))
        return
    if args.command == "exec":
        exec_tool(args.tool, args.scenario)
        return
    if args.command == "render":
        payload = json.loads(args.input.read_text(encoding="utf-8"))
        write_results(payload)
        return

    setup_from_args(args)
    scenarios = args.scenario or list(SCENARIOS)
    payload = run_hyperfine(args.runs, args.warmup, scenarios)
    write_results(payload)
    print_summary(payload)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        raise SystemExit(130)
