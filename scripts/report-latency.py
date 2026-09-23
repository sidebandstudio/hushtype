#!/usr/bin/env python3
"""Summarize real app timing records. This script never generates dictations."""
import argparse
import json
import math
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("log", nargs="?", type=Path, default=Path.home() / "Library/Logs/dev.hushtype.app/dictation-timings.jsonl")
args = parser.parse_args()
rows = []
if args.log.exists():
    for line in args.log.read_text().splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        if row.get("paste_outcome") in {"clipboard_read", "clipboard_fallback", "clipboard_changed", "clipboard_error", "sent_unverified"} and row.get("outcome", "").startswith(("cleaned", "raw_")):
            rows.append(row)
rows = rows[-20:]
report = {"source": str(args.log), "completed_refine_attempts": len(rows), "required": 20}
if rows:
    def percentile(key, p, records=rows):
        values = sorted(row[key] for row in records)
        return values[max(0, math.ceil(len(values) * p) - 1)]
    near_ten_seconds = [r for r in rows if 8000 <= r.get("audio_ms", 0) <= 12000]
    report.update({
        "paste_outcomes": {outcome: sum(r["paste_outcome"] == outcome for r in rows) for outcome in sorted({r["paste_outcome"] for r in rows})},
        "raw_fallbacks": sum(r["outcome"].startswith("raw_") for r in rows),
        "refine_p50_ms": percentile("refine_ms", 0.5),
        "end_to_end_p95_ms": percentile("total_ms", 0.95),
        "stt_p50_ms": percentile("stt_ms", 0.5),
        "near_10s_utterances": len(near_ten_seconds),
        "near_10s_stt_max_ms": max((r.get("stt_done_ms", r["capture_stop_ms"] + r["stt_ms"]) for r in near_ten_seconds), default=None),
        "targets": {"refine_p50_ms": 1200, "end_to_end_p95_ms": 2500, "near_10s_stt_max_ms": 600},
    })
    report["sample_complete"] = len(rows) == 20 and bool(near_ten_seconds)
else:
    report["sample_complete"] = False
    report["reason"] = "No completed real dictation attempts with refinement and settled paste. Latency acceptance is unmeasured."
print(json.dumps(report, indent=2))
raise SystemExit(0 if report["sample_complete"] else 2)
