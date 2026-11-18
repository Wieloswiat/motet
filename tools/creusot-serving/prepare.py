#!/usr/bin/env python3
"""Prepare a standalone Cargo package from Motet's serving decision."""

from argparse import ArgumentParser
from pathlib import Path
import subprocess

parser = ArgumentParser(description=__doc__)
parser.add_argument("scratch", type=Path, help="new directory for the standalone package")
parser.add_argument("--commit", default="dcd2c1292375c1b8920a566be4d471c38d6da56f")
args = parser.parse_args()

recipe = Path(__file__).resolve().parent
source = subprocess.check_output(
    ["git", "show", f"{args.commit}:crates/spaces/src/serving.rs"],
    cwd=recipe.parents[1],
)
anchor = b"/// Whether to serve a reader now.\n#[must_use]\npub fn decide("
if source.count(anchor) != 1:
    raise SystemExit("Cannot find the unique decide annotation point")

# Keep the doc comment attached to decide and leave its signature and body intact.
use_line, ensures = (recipe / "contract.rs.txt").read_bytes().split(b"\n\n", 1)
doc, signature = anchor.split(b"\n", 1)
annotated = source.replace(anchor, use_line + b"\n\n" + doc + b"\n" + ensures + signature)

scratch = args.scratch.resolve()
scratch.mkdir()
(scratch / "src").mkdir()
(scratch / "src/lib.rs").write_bytes(annotated)
for name in ("Cargo.toml", "Cargo.lock", "why3find.json"):
    template = name if name == "why3find.json" else name + ".in"
    (scratch / name).write_bytes((recipe / template).read_bytes())
