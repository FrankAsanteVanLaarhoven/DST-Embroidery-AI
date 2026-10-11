#!/usr/bin/env python3
"""Check the EmbroideryOS capability register, draft 0.3.

The checker examines register shape, inherited baseline integrity, workbook
formulas and source conflicts. It does not authenticate a reviewer, inspect a
machine file, connect to a device, or authorise a release. A passing structural
audit means the ledger is internally consistent. It does not mean a feature
exists.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
import zipfile
import xml.etree.ElementTree as ET
from pathlib import Path

NS = "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}"
ROOT = Path(__file__).resolve().parents[1]
WORKBOOK = ROOT / "contracts" / "EmbroideryOS_Master_Capability_Register_0_3.xlsx"
BASELINE = ROOT / "contracts" / "compatibility-coverage.json"
SNAPSHOT = ROOT / "contracts" / "inherited" / "compatibility-coverage-0.2.json"
SOURCES = ROOT / "contracts" / "sources.json"
CONFLICTS = ROOT / "contracts" / "claim-conflicts.json"
EVIDENCE = ROOT / "evidence" / "baseline-0.3"

SHEET_FILES = {
    "Dashboard": "xl/worksheets/sheet1.xml",
    "Requirements": "xl/worksheets/sheet2.xml",
    "Acceptance": "xl/worksheets/sheet3.xml",
    "Sources": "xl/worksheets/sheet4.xml",
    "Source Conflicts": "xl/worksheets/sheet5.xml",
    "Stitch Families": "xl/worksheets/sheet6.xml",
    "Delivery Gates": "xl/worksheets/sheet7.xml",
}
REQUIREMENT_FIELDS = (
    "id", "domain", "feature", "scope_origin", "implementation_status",
    "acceptance_status", "owner_role", "milestone", "required_operations",
    "requirement", "source_ids", "source_url", "notes",
)
ACCEPTANCE_FIELDS = (
    "id", "feature", "positive_case", "negative_case", "dependencies",
    "required_operations", "evidence",
)
SOURCE_FIELDS = ("id", "title", "source_type", "url", "interpretation")
CONFLICT_FIELDS = ("id", "topic", "supplied_variants", "resolution", "sources")
FAMILY_FIELDS = (
    "id", "source_family", "category", "implementation_scope",
    "engineering_requirement", "acceptance_boundary",
)
GATE_FIELDS = ("gate", "timing", "pass_condition", "evidence_owner")
MASTER_STATUSES = {
    "PLANNED", "UNQUALIFIED", "LIBRARY_SUBSET", "CANDIDATE_SUBSET", "REFERENCE_UTILITY",
}
BASELINE_STATUSES = {
    "not_qualified", "planned_not_qualified", "partial_developer_utility_only",
}
ACCEPTANCE_STATUSES = {"NOT_RUN", "FAILED", "ACCEPTED_SCOPED"}
LINE_GROUPS = (
    "Running", "Zigzag", "Triple", "Motif", "Stem", "Candlewicking", "E/V",
    "Chain", "Cutting",
)
REGION_FAMILIES = (
    "Satin", "Fill", "Programmable fill", "Motif", "Piping", "Cross Stitch",
    "Concentric Circle", "Radial", "Spiral", "Flexible Spiral", "Stippling",
    "Net Fill", "Zigzag Net Fill", "Decorative Fill",
)
ERROR_TOKENS = ("#REF!", "#VALUE!", "#DIV/0!", "#NAME?", "#N/A", "#NULL!", "#NUM!", "#GETTING_DATA")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def column_index(ref: str) -> tuple[int, int]:
    letters = []
    digits = []
    for char in ref:
        if char.isalpha():
            letters.append(char)
        else:
            digits.append(char)
    index = 0
    for char in letters:
        index = index * 26 + ord(char) - 64
    return index, int("".join(digits))


def worksheet_cells(xml_bytes: bytes) -> tuple[dict[tuple[int, int], str], dict[tuple[int, int], str]]:
    root = ET.fromstring(xml_bytes)
    values: dict[tuple[int, int], str] = {}
    formulas: dict[tuple[int, int], str] = {}
    for cell in root.findall(".//" + NS + "c"):
        ref = cell.get("r")
        if not ref:
            continue
        column, row_number = column_index(ref)
        key = (row_number, column)
        formula = cell.find(NS + "f")
        if formula is not None and formula.text:
            formulas[key] = formula.text
        if cell.get("t") == "inlineStr":
            values[key] = "".join(node.text or "" for node in cell.iter(NS + "t"))
            continue
        stored = cell.find(NS + "v")
        values[key] = stored.text if stored is not None and stored.text is not None else ""
    return values, formulas


def table_rows(values: dict[tuple[int, int], str], fields: tuple[str, ...]) -> list[dict[str, str]]:
    rows = []
    row_number = 2
    while (row_number, 1) in values and values[(row_number, 1)].strip():
        rows.append({
            field: values.get((row_number, index), "")
            for index, field in enumerate(fields, start=1)
        })
        row_number += 1
    return rows


def load_workbook(path: Path) -> dict:
    raw = path.read_bytes()
    with zipfile.ZipFile(path) as package:
        names = set(package.namelist())
        workbook = ET.fromstring(package.read("xl/workbook.xml"))
        sheets = [
            node.get("name")
            for node in workbook.findall(NS + "sheets/" + NS + "sheet")
        ]
        parsed = {}
        formulas = {}
        for sheet, filename in SHEET_FILES.items():
            values, sheet_formulas = worksheet_cells(package.read(filename))
            parsed[sheet] = values
            formulas[sheet] = sheet_formulas
        xml_text = "\n".join(
            package.read(name).decode("utf-8")
            for name in package.namelist()
            if name.endswith(".xml")
        )
    return {
        "path": str(path),
        "sha256": sha256_bytes(raw),
        "sheet_names": sheets,
        "package_names": names,
        "cells": parsed,
        "formulas": formulas,
        "requirements": table_rows(parsed["Requirements"], REQUIREMENT_FIELDS),
        "acceptance": table_rows(parsed["Acceptance"], ACCEPTANCE_FIELDS),
        "sources": table_rows(parsed["Sources"], SOURCE_FIELDS),
        "conflicts": table_rows(parsed["Source Conflicts"], CONFLICT_FIELDS),
        "families": table_rows(parsed["Stitch Families"], FAMILY_FIELDS),
        "gates": table_rows(parsed["Delivery Gates"], GATE_FIELDS),
        "xml_text": xml_text,
    }


def load_json(path: Path):
    return json.loads(path.read_text())


def cited_source_ids(record: dict) -> list[str]:
    return [part.strip() for part in record.get("source_ids", "").split(",") if part.strip()]


def dependency_ids(record: dict) -> list[str]:
    return [part.strip() for part in record.get("dependencies", "").split(",") if part.strip()]


def duplicate_ids(rows: list[dict]) -> list[str]:
    seen = {}
    duplicates = []
    for row in rows:
        identifier = row["id"]
        if identifier in seen:
            duplicates.append(identifier)
        seen[identifier] = True
    return duplicates


def workbook_error_tokens(xml_text: str) -> list[str]:
    return [token for token in ERROR_TOKENS if token in xml_text]


def recomputed_dashboard(requirements: list[dict]) -> dict[str, str]:
    statuses = {}
    for row in requirements:
        statuses[row["implementation_status"]] = statuses.get(row["implementation_status"], 0) + 1
    inherited = sum(row["scope_origin"] == "inherited_0.2" for row in requirements)
    accepted = sum(row["acceptance_status"] == "ACCEPTED_SCOPED" for row in requirements)
    lines = sum(row["domain"] == "Line stitch families" for row in requirements)
    regions = sum(row["domain"] == "Region stitch families" for row in requirements)
    total = len(requirements)
    return {
        "B8": str(total),
        "E8": str(statuses.get("PLANNED", 0)),
        "B9": str(inherited),
        "E9": str(statuses.get("UNQUALIFIED", 0)),
        "B10": str(total - inherited),
        "E10": str(statuses.get("LIBRARY_SUBSET", 0)),
        "B11": str(accepted),
        "E11": str(statuses.get("CANDIDATE_SUBSET", 0)),
        "B12": str(total - accepted),
        "E12": str(statuses.get("REFERENCE_UTILITY", 0)),
        "B20": str(lines),
        "B21": str(regions),
    }


def dashboard_address(label: str) -> tuple[int, int]:
    letters = "".join(char for char in label if char.isalpha())
    digits = "".join(char for char in label if char.isdigit())
    column = 0
    for char in letters:
        column = column * 26 + ord(char) - 64
    return int(digits), column


def structural_findings(workbook: dict, baseline: dict, sources: dict, conflicts: dict, snapshot_matches: bool) -> list[str]:
    findings = []
    requirements = workbook["requirements"]
    acceptance = workbook["acceptance"]
    if workbook["sheet_names"] != list(SHEET_FILES):
        findings.append("Workbook sheet names or order differ from the 0.3 register.")
    if duplicate_ids(requirements):
        findings.append("Requirement identifiers collide: " + ", ".join(duplicate_ids(requirements)))
    if len(requirements) != 210:
        findings.append(f"Requirement count is {len(requirements)}, expected 210.")
    inherited = [row for row in requirements if row["scope_origin"] == "inherited_0.2"]
    base_rows = baseline["requirements"]
    if [row["id"] for row in inherited] != [row["id"] for row in base_rows]:
        findings.append("Inherited identifiers or order differ from compatibility coverage 0.2.")
    else:
        for master, original in zip(inherited, base_rows):
            if master["feature"] != original["subject"]:
                findings.append(f"{master['id']} feature label differs from the inherited subject.")
            if not original.get("source_text"):
                findings.append(f"{original['id']} has lost its inherited source text.")
            if master["implementation_status"] != "UNQUALIFIED":
                findings.append(f"{master['id']} inherited status was promoted in the master register.")
            if "No implementation status promoted." not in master["notes"]:
                findings.append(f"{master['id']} no longer records that its original status was preserved.")
    if not snapshot_matches:
        findings.append("The live 0.2 coverage file differs from the preserved snapshot.")
    base_statuses = {row["implementation_status"] for row in base_rows}
    if not base_statuses <= BASELINE_STATUSES:
        findings.append("The 0.2 baseline contains a status outside its original vocabulary.")
    # Eight inherited rows cite the 0.2 developer-utility log. That citation stays
    # in the preserved file and does not promote a master acceptance status.
    unexpected = sorted({row["implementation_status"] for row in requirements} - MASTER_STATUSES)
    if unexpected:
        findings.append("Unexpected implementation status: " + ", ".join(unexpected))
    if [row["id"] for row in acceptance] != [row["id"] for row in requirements]:
        findings.append("Acceptance rows do not follow the requirement identifiers.")
    by_id = {}
    for row in requirements:
        by_id.setdefault(row["id"], row)
    for row in acceptance:
        master = by_id.get(row["id"])
        if master is None:
            findings.append(f"{row['id']} is in acceptance but not in requirements.")
            continue
        status = master["acceptance_status"]
        if status not in ACCEPTANCE_STATUSES:
            findings.append(f"{row['id']} has acceptance status {status or 'blank'}.")
        if status == "NOT_RUN" and row["evidence"].strip():
            findings.append(f"{row['id']} has evidence while its acceptance status is NOT_RUN.")
        if status == "ACCEPTED_SCOPED" and not row["evidence"].strip():
            findings.append(f"{row['id']} is marked accepted without an evidence reference.")
        if status == "ACCEPTED_SCOPED":
            for dependency in dependency_ids(row):
                parent = by_id.get(dependency)
                if parent is None or parent["acceptance_status"] != "ACCEPTED_SCOPED":
                    findings.append(f"{row['id']} depends on {dependency}, which is not accepted.")
    line_rows = [row for row in requirements if row["domain"] == "Line stitch families"]
    region_rows = [row for row in requirements if row["domain"] == "Region stitch families"]
    if tuple(row["feature"] for row in line_rows) != LINE_GROUPS:
        findings.append("The nine supplied line groups are missing or reordered.")
    if tuple(row["feature"] for row in region_rows) != REGION_FAMILIES:
        findings.append("The fourteen supplied region families are missing or reordered.")
    if any(row["id"] == "L02" for row in requirements):
        findings.append("Line-group identifier L02 collides with the inherited source identifier.")
    editing = {row["id"] for row in requirements if row["id"].startswith("ED")}
    inherited_editing = {row["id"] for row in base_rows if row["id"].startswith("E")}
    if editing & inherited_editing:
        findings.append("New editing identifiers collide with inherited E-series identifiers.")
    if editing != {f"ED{number:02d}" for number in range(1, 29)}:
        findings.append("Editing rows are not exactly ED01–ED28.")
    source_ids = {row["id"] for row in workbook["sources"]}
    if [row["id"] for row in sources.get("sources", [])] != [row["id"] for row in workbook["sources"]]:
        findings.append("contracts/sources.json does not match the workbook Sources sheet.")
    conflict_ids = [row["id"] for row in workbook["conflicts"]]
    if [row["id"] for row in conflicts.get("conflicts", [])] != conflict_ids:
        findings.append("contracts/claim-conflicts.json does not match the workbook conflicts.")
    if conflict_ids != [f"CF0{number}" for number in range(1, 7)]:
        findings.append("The six source conflicts are not retained as CF01–CF06.")
    for row in requirements:
        unknown = [item for item in cited_source_ids(row) if item not in source_ids]
        if unknown:
            findings.append(f"{row['id']} cites unknown sources: {', '.join(unknown)}")
    for token in workbook_error_tokens(workbook["xml_text"]):
        findings.append(f"Workbook contains spreadsheet error token {token}.")
    expected = recomputed_dashboard(requirements)
    dashboard = workbook["cells"]["Dashboard"]
    for label, value in expected.items():
        row_number, column = dashboard_address(label)
        cached = dashboard.get((row_number, column), "")
        if cached != value:
            findings.append(f"Dashboard {label} cache is {cached or 'blank'}, recomputed {value}.")
        if label != "B22" and (row_number, column) not in workbook["formulas"]["Dashboard"]:
            findings.append(f"Dashboard {label} is no longer a formula.")
    conflict_count = dashboard.get(dashboard_address("B22"), "")
    if conflict_count != "6":
        findings.append("Dashboard no longer records six preserved source conflicts.")
    return findings


def conflict_texts(conflicts: list[dict]) -> str:
    return "\n".join(
        "\n".join((row["topic"], row["supplied_variants"], row["resolution"]))
        for row in conflicts
    )


def acceptance_findings(requirements: list[dict]) -> list[str]:
    accepted = sum(row["acceptance_status"] == "ACCEPTED_SCOPED" for row in requirements)
    total = len(requirements)
    if accepted != total or total == 0:
        return [f"{accepted} of {total} rows have accepted evidence"]
    return []


def summary(workbook: dict, findings: list[str], snapshot_hash: str) -> dict:
    requirements = workbook["requirements"]
    statuses = {}
    origins = {}
    for row in requirements:
        statuses[row["implementation_status"]] = statuses.get(row["implementation_status"], 0) + 1
        origins[row["scope_origin"]] = origins.get(row["scope_origin"], 0) + 1
    inherited = origins.get("inherited_0.2", 0)
    return {
        "schema": "embroideryos-capability-audit/0.3",
        "structural_audit": "PASS" if not findings else "FAIL",
        "findings": findings,
        "records": len(requirements),
        "inherited": inherited,
        "new_or_expanded": len(requirements) - inherited,
        "implementation_status": statuses,
        "acceptance_status": {
            row["acceptance_status"]: sum(item["acceptance_status"] == row["acceptance_status"] for item in requirements)
            for row in requirements
        },
        "scope_origin": origins,
        "accepted_scopes": sum(row["acceptance_status"] == "ACCEPTED_SCOPED" for row in requirements),
        "line_groups": sum(row["domain"] == "Line stitch families" for row in requirements),
        "region_families": sum(row["domain"] == "Region stitch families" for row in requirements),
        "source_conflicts": len(workbook["conflicts"]),
        "workbook_sha256": workbook["sha256"],
        "inherited_snapshot_sha256": snapshot_hash,
        "authenticates_evidence": False,
        "machine_release_allowed": False,
        "iso_certification": False,
        "qualifies_platform": False,
    }


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")


def audit(workbook_path: Path, baseline_path: Path, snapshot_path: Path, sources_path: Path, conflicts_path: Path) -> tuple[dict, list[str]]:
    workbook = load_workbook(workbook_path)
    baseline = load_json(baseline_path)
    sources = load_json(sources_path)
    conflicts = load_json(conflicts_path)
    snapshot_matches = baseline_path.read_bytes() == snapshot_path.read_bytes()
    findings = structural_findings(workbook, baseline, sources, conflicts, snapshot_matches)
    report = summary(workbook, findings, sha256_file(snapshot_path))
    return report, acceptance_findings(workbook["requirements"])


def export_contract_views(workbook_path: Path, sources_path: Path, conflicts_path: Path) -> None:
    workbook = load_workbook(workbook_path)
    write_json(sources_path, {"schema": "embroideryos-sources/0.3", "sources": workbook["sources"]})
    write_json(conflicts_path, {"schema": "embroideryos-claim-conflicts/0.3", "conflicts": workbook["conflicts"]})


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Audit the EmbroideryOS 0.3 capability register.")
    parser.add_argument("--workbook", type=Path, default=WORKBOOK)
    parser.add_argument("--baseline", type=Path, default=BASELINE)
    parser.add_argument("--snapshot", type=Path, default=SNAPSHOT)
    parser.add_argument("--sources", type=Path, default=SOURCES)
    parser.add_argument("--conflicts", type=Path, default=CONFLICTS)
    parser.add_argument("--output", type=Path, default=EVIDENCE)
    parser.add_argument("--require-acceptance", action="store_true")
    parser.add_argument("--export-contracts", action="store_true")
    args = parser.parse_args(argv)
    if args.export_contracts:
        export_contract_views(args.workbook, args.sources, args.conflicts)
    report, acceptance = audit(args.workbook, args.baseline, args.snapshot, args.sources, args.conflicts)
    write_json(args.output / "registry-audit.json", report)
    workbook = load_workbook(args.workbook)
    write_json(args.output / "workbook-check.json", {
        "schema": "embroideryos-workbook-check/0.3",
        "sheet_names": workbook["sheet_names"],
        "requirement_rows": len(workbook["requirements"]),
        "formula_cache": recomputed_dashboard(workbook["requirements"]),
        "spreadsheet_errors": workbook_error_tokens(workbook["xml_text"]),
        "rendered_in_spreadsheet_application": False,
    })
    print(f"structural audit {report['structural_audit']}")
    print(
        f"records {report['records']}; inherited {report['inherited']}; "
        f"new_or_expanded {report['new_or_expanded']}; line_groups {report['line_groups']}; "
        f"region_families {report['region_families']}; conflicts {report['source_conflicts']}"
    )
    if report["findings"]:
        for finding in report["findings"]:
            print(finding)
    if args.require_acceptance:
        write_json(args.output / "acceptance-gate.json", {
            "schema": "embroideryos-acceptance-gate/0.3",
            "result": "PASS" if not acceptance else "FAIL",
            "findings": acceptance,
            "authenticates_evidence": False,
            "machine_release_allowed": False,
        })
        if acceptance:
            print("acceptance gate FAIL")
            for finding in acceptance:
                print(finding)
            return 1
        print("acceptance gate PASS")
    return 0 if report["structural_audit"] == "PASS" else 2


if __name__ == "__main__":
    sys.exit(main())
