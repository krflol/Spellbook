"""Collect license notices from the locked Rust dependency sources (Python 3)."""
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
metadata = json.loads(subprocess.check_output(
    ["cargo", "metadata", "--locked", "--format-version", "1"], cwd=ROOT
))
tree = subprocess.check_output(["cargo", "tree", "--locked", "--target", "x86_64-pc-windows-msvc", "--edges", "normal,build", "--prefix", "none", "--format", "{p}"], cwd=ROOT, text=True)
included = set(re.findall(r"(?m)^([\w-]+) v([^\s]+)", tree))
sections = [
    "Spellbook third-party notices\n"
    "Generated from the locked Rust dependency sources for Windows x64.\n"
    "This inventory includes runtime and build dependencies.\n"
    "Each dependency remains under its own license.\n"
]
missing = []
for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
    if package["name"] == "spellbook" or (package["name"], package["version"]) not in included:
        continue
    directory = Path(package["manifest_path"]).parent
    files = set()
    if package.get("license_file"):
        path = directory / package["license_file"]
        if path.is_file():
            files.add(path)
    for path in directory.rglob("*"):
        if path.is_file() and (path.name.lower().startswith(("license", "licence", "copying", "notice", "copyright")) or path.name in ("OFL.txt", "UFL.txt", "Hack-Regular.txt") or "license" in path.name.lower()):
            files.add(path)
    header = f"\n{'=' * 72}\n{package['name']} {package['version']}\nLicense: {package.get('license') or 'See included notice'}\n"
    if package.get("repository"):
        header += f"Source: {package['repository']}\n"
    sections.append(header)
    if not files:
        supplement = ROOT / "assets" / "licenses" / (package["name"] + ".txt")
        if supplement.exists():
            sections.append(supplement.read_text(encoding="utf-8"))
        else:
            missing.append(package["name"])
            sections.append("License declaration provided in the crate manifest; see the source repository.\n")
    for path in sorted(files):
        try:
            content = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        sections.append(f"\n--- {path.relative_to(directory).as_posix()} ---\n{content}\n")
notices = "\n".join(line.rstrip() for line in "\n".join(sections).splitlines()).rstrip() + "\n"
(ROOT / "THIRD_PARTY_NOTICES.txt").write_text(notices, encoding="utf-8")
print(f"Collected {len(included) - 1} dependency notices.")
if missing:
    print("No standalone notice file in crates: " + ", ".join(missing))
