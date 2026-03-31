#!/usr/bin/env python3
"""Compare fidelity artifacts for the canvas-first editor migration.

Supports current repo outputs:
- layout JSON from `to_layout_json()`
- page-map JSON from `get_page_map_json()`
- future scene-style JSON with page bounds/content rects
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


@dataclass
class Rect:
    x: float
    y: float
    width: float
    height: float


@dataclass
class PageSummary:
    index: int
    width: float
    height: float
    content_rect: Optional[Rect]
    block_count: Optional[int] = None
    item_count: Optional[int] = None
    node_count: Optional[int] = None
    para_split_count: Optional[int] = None
    table_chunk_count: Optional[int] = None
    block_rects: Optional[List[Rect]] = None


@dataclass
class Tolerances:
    max_page_size_delta_pt: float = 0.25
    max_content_rect_delta_pt: float = 0.25
    max_block_rect_delta_pt: float = 1.0
    allow_count_mismatch: bool = False


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Compare engine, DOM, or canvas fidelity JSON artifacts.")
    parser.add_argument("--reference", help="Path to reference JSON artifact.")
    parser.add_argument("--candidate", help="Path to candidate JSON artifact.")
    parser.add_argument("--manifest", help="Optional corpus manifest JSON path.")
    parser.add_argument("--case-id", help="Optional case id from the manifest.")
    parser.add_argument(
        "--artifact-kind",
        choices=["layout", "page_map"],
        default="layout",
        help="Which artifact paths to read from the manifest when --case-id is used.",
    )
    parser.add_argument("--candidate-key", default="canvas_candidate", help="Manifest candidate prefix: canvas_candidate or dom_baseline.")
    parser.add_argument("--json", action="store_true", help="Emit the full report as JSON.")
    parser.add_argument("--max-page-size-delta-pt", type=float)
    parser.add_argument("--max-content-rect-delta-pt", type=float)
    parser.add_argument("--max-block-rect-delta-pt", type=float)
    parser.add_argument("--allow-count-mismatch", action="store_true")
    return parser.parse_args()


def load_json(path: Path) -> Dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def parse_rect(obj: Optional[Dict[str, Any]]) -> Optional[Rect]:
    if not isinstance(obj, dict):
        return None
    try:
        return Rect(
            x=float(obj["x"]),
            y=float(obj["y"]),
            width=float(obj["width"]),
            height=float(obj["height"]),
        )
    except (KeyError, TypeError, ValueError):
        return None


def rect_from_margins(page: Dict[str, Any]) -> Optional[Rect]:
    try:
        width = float(page["width"])
        height = float(page["height"])
        left = float(page.get("marginLeft", 0.0))
        right = float(page.get("marginRight", 0.0))
        top = float(page.get("marginTop", 0.0))
        bottom = float(page.get("marginBottom", 0.0))
        return Rect(x=left, y=top, width=width - left - right, height=height - top - bottom)
    except (TypeError, ValueError, KeyError):
        return None


def detect_format(data: Dict[str, Any]) -> str:
    pages = data.get("pages")
    if isinstance(pages, list) and pages:
        page = pages[0]
        if "pageNum" in page or "nodeIds" in page:
            return "page_map"
        if "contentArea" in page or "blocks" in page or "floatingImages" in page:
            return "layout_json"
        if "content_rect_pt" in page or "bounds_pt" in page or "items" in page or "block_rects_pt" in page or "node_count" in page:
            return "scene"
    if "page_count" in data and "default_page_size_pt" in data:
        return "scene"
    raise ValueError("Unsupported fidelity artifact format")


def extract_layout_block_rects(page: Dict[str, Any]) -> List[Rect]:
    rects: List[Rect] = []
    for block in page.get("blocks", []):
        if not isinstance(block, dict):
            continue
        rect = parse_rect(block.get("bounds"))
        if rect is None:
            rect = parse_rect(block)
        if rect is not None:
            rects.append(rect)
    return rects


def extract_scene_block_rects(page: Dict[str, Any]) -> List[Rect]:
    rects: List[Rect] = []
    for raw_rect in page.get("block_rects_pt", []):
        rect = parse_rect(raw_rect)
        if rect is not None:
            rects.append(rect)
    if rects:
        return rects
    for block in page.get("blocks", []):
        if not isinstance(block, dict):
            continue
        rect = parse_rect(block.get("bounds_pt"))
        if rect is None:
            rect = parse_rect(block.get("bounds"))
        if rect is not None:
            rects.append(rect)
    return rects


def normalize_layout_json(data: Dict[str, Any]) -> List[PageSummary]:
    pages: List[PageSummary] = []
    for i, page in enumerate(data.get("pages", [])):
        content_rect = parse_rect(page.get("contentArea"))
        block_rects = extract_layout_block_rects(page)
        item_count = 0
        for block in page.get("blocks", []):
            if isinstance(block, dict) and block.get("type") == "paragraph":
                for line in block.get("lines", []):
                    item_count += len(line.get("runs", [])) if isinstance(line, dict) else 0
        pages.append(
            PageSummary(
                index=int(page.get("index", i)),
                width=float(page["width"]),
                height=float(page["height"]),
                content_rect=content_rect,
                block_count=len(page.get("blocks", [])),
                item_count=item_count,
                block_rects=block_rects,
            )
        )
    return pages


def normalize_page_map(data: Dict[str, Any]) -> List[PageSummary]:
    pages: List[PageSummary] = []
    for i, page in enumerate(data.get("pages", [])):
        pages.append(
            PageSummary(
                index=int(page.get("pageNum", i + 1)) - 1,
                width=float(page["width"]),
                height=float(page["height"]),
                content_rect=rect_from_margins(page),
                node_count=len(page.get("nodeIds", [])),
                para_split_count=len(page.get("paraSplits", [])),
                table_chunk_count=len(page.get("tableChunks", [])),
            )
        )
    return pages


def normalize_scene(data: Dict[str, Any]) -> List[PageSummary]:
    pages: List[PageSummary] = []
    raw_pages = data.get("pages", [])
    default_size = data.get("default_page_size_pt", {})
    for i, page in enumerate(raw_pages):
        bounds = parse_rect(page.get("bounds_pt"))
        width = bounds.width if bounds else float(default_size.get("width", 0.0))
        height = bounds.height if bounds else float(default_size.get("height", 0.0))
        item_count = page.get("item_count")
        node_count = page.get("node_count")
        block_rects = extract_scene_block_rects(page)
        pages.append(
            PageSummary(
                index=int(page.get("page_index", i)),
                width=width,
                height=height,
                content_rect=parse_rect(page.get("content_rect_pt")) or parse_rect(page.get("content_rect")),
                item_count=int(item_count) if item_count is not None else None,
                node_count=int(node_count) if node_count is not None else None,
                block_rects=block_rects or None,
            )
        )
    return pages


def normalize(data: Dict[str, Any]) -> Tuple[str, List[PageSummary]]:
    fmt = detect_format(data)
    if fmt == "layout_json":
        return fmt, normalize_layout_json(data)
    if fmt == "page_map":
        return fmt, normalize_page_map(data)
    return fmt, normalize_scene(data)


def rect_delta(a: Optional[Rect], b: Optional[Rect]) -> Optional[float]:
    if a is None or b is None:
        return None
    return max(
        abs(a.x - b.x),
        abs(a.y - b.y),
        abs(a.width - b.width),
        abs(a.height - b.height),
    )


def compare_rect_lists(reference: List[Rect], candidate: List[Rect]) -> Dict[str, Any]:
    count = min(len(reference), len(candidate))
    if count == 0:
        return {"compared": 0, "max_delta_pt": None, "mean_delta_pt": None}
    deltas = [rect_delta(reference[i], candidate[i]) or 0.0 for i in range(count)]
    return {
        "compared": count,
        "max_delta_pt": max(deltas),
        "mean_delta_pt": sum(deltas) / len(deltas),
    }


def compare_pages(reference: List[PageSummary], candidate: List[PageSummary]) -> Dict[str, Any]:
    page_pairs = list(zip(reference, candidate))
    page_size_deltas = []
    content_rect_deltas = []
    block_rect_max = []
    count_mismatches = []

    for ref_page, cand_page in page_pairs:
        page_size_deltas.append(max(abs(ref_page.width - cand_page.width), abs(ref_page.height - cand_page.height)))
        content_delta = rect_delta(ref_page.content_rect, cand_page.content_rect)
        if content_delta is not None:
            content_rect_deltas.append(content_delta)

        if ref_page.block_rects is not None and cand_page.block_rects is not None:
            rect_report = compare_rect_lists(ref_page.block_rects, cand_page.block_rects)
            if rect_report["max_delta_pt"] is not None:
                block_rect_max.append(rect_report["max_delta_pt"])
            if len(ref_page.block_rects) != len(cand_page.block_rects):
                count_mismatches.append({
                    "page_index": ref_page.index,
                    "kind": "block_rect_count",
                    "reference": len(ref_page.block_rects),
                    "candidate": len(cand_page.block_rects),
                })

        for field_name in ("block_count", "item_count", "node_count", "para_split_count", "table_chunk_count"):
            ref_value = getattr(ref_page, field_name)
            cand_value = getattr(cand_page, field_name)
            if ref_value is not None and cand_value is not None and ref_value != cand_value:
                count_mismatches.append({
                    "page_index": ref_page.index,
                    "kind": field_name,
                    "reference": ref_value,
                    "candidate": cand_value,
                })

    return {
        "reference_page_count": len(reference),
        "candidate_page_count": len(candidate),
        "page_count_match": len(reference) == len(candidate),
        "page_size_delta_max_pt": max(page_size_deltas) if page_size_deltas else None,
        "content_rect_delta_max_pt": max(content_rect_deltas) if content_rect_deltas else None,
        "block_rect_delta_max_pt": max(block_rect_max) if block_rect_max else None,
        "count_mismatches": count_mismatches,
    }


def resolve_case_paths(args: argparse.Namespace) -> Tuple[Path, Path, Optional[Dict[str, Any]], Tolerances]:
    case = None
    tolerances = Tolerances()
    if args.manifest and args.case_id:
        manifest_path = Path(args.manifest)
        manifest = load_json(manifest_path)
        cases = manifest.get("cases", [])
        case = next((entry for entry in cases if entry.get("id") == args.case_id), None)
        if case is None:
            raise SystemExit(f"Case '{args.case_id}' not found in {manifest_path}")
        profile_name = case.get("tolerance_profile")
        profile = manifest.get("tolerance_profiles", {}).get(profile_name, {})
        tolerances = Tolerances(**profile)
        suffix = "layout_json" if args.artifact_kind == "layout" else "page_map_json"
        reference_key = f"engine_reference_{suffix}"
        candidate_key = f"{args.candidate_key}_{suffix}"
        reference_path = root_path(manifest_path, case[reference_key])
        candidate_path = root_path(manifest_path, case[candidate_key])
    else:
        if not args.reference or not args.candidate:
            raise SystemExit("Either provide --reference/--candidate or --manifest/--case-id")
        reference_path = Path(args.reference)
        candidate_path = Path(args.candidate)

    if args.max_page_size_delta_pt is not None:
        tolerances.max_page_size_delta_pt = args.max_page_size_delta_pt
    if args.max_content_rect_delta_pt is not None:
        tolerances.max_content_rect_delta_pt = args.max_content_rect_delta_pt
    if args.max_block_rect_delta_pt is not None:
        tolerances.max_block_rect_delta_pt = args.max_block_rect_delta_pt
    if args.allow_count_mismatch:
        tolerances.allow_count_mismatch = True

    return reference_path, candidate_path, case, tolerances


def root_path(manifest_path: Path, raw: str) -> Path:
    path = Path(raw)
    if path.is_absolute():
        return path
    # Manifest paths are relative to the project root.
    # Walk up from the manifest directory to find the project root
    # (identified by Cargo.toml or .git).
    candidate = manifest_path.parent.resolve()
    for _ in range(10):
        if (candidate / "Cargo.toml").exists() or (candidate / ".git").exists():
            return candidate / path
        candidate = candidate.parent
    # Fallback: assume manifest is at tests/fidelity/
    return manifest_path.parent.parent.parent / path


def evaluate(report: Dict[str, Any], tolerances: Tolerances) -> Tuple[bool, List[str]]:
    failures: List[str] = []
    if not report["page_count_match"]:
        failures.append("page count mismatch")
    if report["page_size_delta_max_pt"] is not None and report["page_size_delta_max_pt"] > tolerances.max_page_size_delta_pt:
        failures.append(
            f"page size delta {report['page_size_delta_max_pt']:.3f}pt exceeds {tolerances.max_page_size_delta_pt:.3f}pt"
        )
    if report["content_rect_delta_max_pt"] is not None and report["content_rect_delta_max_pt"] > tolerances.max_content_rect_delta_pt:
        failures.append(
            f"content rect delta {report['content_rect_delta_max_pt']:.3f}pt exceeds {tolerances.max_content_rect_delta_pt:.3f}pt"
        )
    if report["block_rect_delta_max_pt"] is not None and report["block_rect_delta_max_pt"] > tolerances.max_block_rect_delta_pt:
        failures.append(
            f"block rect delta {report['block_rect_delta_max_pt']:.3f}pt exceeds {tolerances.max_block_rect_delta_pt:.3f}pt"
        )
    if report["count_mismatches"] and not tolerances.allow_count_mismatch:
        failures.append(f"count mismatches present ({len(report['count_mismatches'])})")
    return not failures, failures


def print_human(report: Dict[str, Any], failures: List[str]) -> None:
    print(f"reference format: {report['reference_format']}")
    print(f"candidate format: {report['candidate_format']}")
    print(f"page count: {report['metrics']['reference_page_count']} vs {report['metrics']['candidate_page_count']}")
    print(f"page size max delta: {fmt(report['metrics']['page_size_delta_max_pt'])}")
    print(f"content rect max delta: {fmt(report['metrics']['content_rect_delta_max_pt'])}")
    print(f"block rect max delta: {fmt(report['metrics']['block_rect_delta_max_pt'])}")
    print(f"count mismatches: {len(report['metrics']['count_mismatches'])}")
    if report.get("case_id"):
        print(f"case id: {report['case_id']}")
    if failures:
        print("status: FAIL")
        for failure in failures:
            print(f"- {failure}")
    else:
        print("status: PASS")


def fmt(value: Optional[float]) -> str:
    return "n/a" if value is None else f"{value:.3f}pt"


def main() -> int:
    args = parse_args()
    reference_path, candidate_path, case, tolerances = resolve_case_paths(args)
    reference_data = load_json(reference_path)
    candidate_data = load_json(candidate_path)
    reference_format, reference_pages = normalize(reference_data)
    candidate_format, candidate_pages = normalize(candidate_data)
    metrics = compare_pages(reference_pages, candidate_pages)
    report = {
        "case_id": case.get("id") if case else None,
        "reference_path": str(reference_path),
        "candidate_path": str(candidate_path),
        "reference_format": reference_format,
        "candidate_format": candidate_format,
        "tolerances": asdict(tolerances),
        "metrics": metrics,
    }
    passed, failures = evaluate(metrics, tolerances)
    report["passed"] = passed
    report["failures"] = failures
    if args.json:
        print(json.dumps(report, indent=2, sort_keys=True))
    else:
        print_human(report, failures)
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
