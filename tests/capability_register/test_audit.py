from __future__ import annotations

import copy
import json
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools"))
from audit_capabilities import (  # noqa: E402
    BASELINE,
    CONFLICTS,
    LINE_GROUPS,
    REGION_FAMILIES,
    SNAPSHOT,
    SOURCES,
    WORKBOOK,
    acceptance_findings,
    conflict_texts,
    load_json,
    load_workbook,
    structural_findings,
)

PYTHON = ROOT / ".venv" / "bin" / "python"
AUDITOR = ROOT / "tools" / "audit_capabilities.py"


class RegisterAuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workbook = load_workbook(WORKBOOK)
        cls.baseline = load_json(BASELINE)
        cls.sources = load_json(SOURCES)
        cls.conflicts = load_json(CONFLICTS)
        cls.requirements = cls.workbook["requirements"]
        cls.by_id = {row["id"]: row for row in cls.requirements}

    def findings_for(self, workbook=None, baseline=None, sources=None, conflicts=None, snapshot_matches=True):
        return structural_findings(
            workbook if workbook is not None else self.workbook,
            baseline if baseline is not None else self.baseline,
            sources if sources is not None else self.sources,
            conflicts if conflicts is not None else self.conflicts,
            snapshot_matches,
        )

    def test_workbook_has_the_seven_named_sheets(self):
        self.assertEqual(
            self.workbook["sheet_names"],
            ["Dashboard", "Requirements", "Acceptance", "Sources", "Source Conflicts", "Stitch Families", "Delivery Gates"],
        )

    def test_register_has_210_unique_records(self):
        identifiers = [row["id"] for row in self.requirements]
        self.assertEqual(len(identifiers), 210)
        self.assertEqual(len(set(identifiers)), 210)

    def test_origin_split_is_54_inherited_and_156_new(self):
        origins = {}
        for row in self.requirements:
            origins[row["scope_origin"]] = origins.get(row["scope_origin"], 0) + 1
        self.assertEqual(origins["inherited_0.2"], 54)
        self.assertEqual(sum(origins.values()) - origins["inherited_0.2"], 156)

    def test_status_counts_match_the_supplied_ledger(self):
        counts = {}
        for row in self.requirements:
            counts[row["implementation_status"]] = counts.get(row["implementation_status"], 0) + 1
        self.assertEqual(counts, {
            "PLANNED": 138,
            "UNQUALIFIED": 64,
            "LIBRARY_SUBSET": 4,
            "CANDIDATE_SUBSET": 3,
            "REFERENCE_UTILITY": 1,
        })

    def test_no_scope_is_accepted(self):
        self.assertTrue(all(row["acceptance_status"] == "NOT_RUN" for row in self.requirements))
        self.assertTrue(all(row["evidence"] == "" for row in self.workbook["acceptance"]))
        self.assertEqual(acceptance_findings(self.requirements), ["0 of 210 rows have accepted evidence"])

    def test_inherited_order_and_labels_match_the_0_2_file(self):
        inherited = [row for row in self.requirements if row["scope_origin"] == "inherited_0.2"]
        original = self.baseline["requirements"]
        self.assertEqual([row["id"] for row in inherited], [row["id"] for row in original])
        self.assertEqual([row["feature"] for row in inherited], [row["subject"] for row in original])

    def test_inherited_source_text_snapshot_is_byte_identical(self):
        self.assertEqual(BASELINE.read_bytes(), SNAPSHOT.read_bytes())
        self.assertTrue(all(row["source_text"] for row in self.baseline["requirements"]))

    def test_baseline_statuses_were_not_promoted(self):
        statuses = {row["implementation_status"] for row in self.baseline["requirements"]}
        self.assertEqual(statuses, {"not_qualified", "planned_not_qualified", "partial_developer_utility_only"})
        inherited = [row for row in self.requirements if row["scope_origin"] == "inherited_0.2"]
        self.assertTrue(all(row["implementation_status"] == "UNQUALIFIED" for row in inherited))
        self.assertTrue(all("No implementation status promoted." in row["notes"] for row in inherited))

    def test_nine_line_groups_keep_supplied_names(self):
        names = [row["feature"] for row in self.requirements if row["domain"] == "Line stitch families"]
        self.assertEqual(tuple(names), LINE_GROUPS)
        self.assertIn("E/V", names)
        self.assertNotIn("L02", [row["id"] for row in self.requirements])
        self.assertEqual(self.by_id["L02X"]["feature"], "Zigzag")

    def test_fourteen_region_families_keep_supplied_names(self):
        names = [row["feature"] for row in self.requirements if row["domain"] == "Region stitch families"]
        self.assertEqual(tuple(names), REGION_FAMILIES)

    def test_editing_ids_do_not_collide_with_inherited_e_series(self):
        editing = {row["id"] for row in self.requirements if row["id"].startswith("ED")}
        inherited = {row["id"] for row in self.baseline["requirements"] if row["id"].startswith("E")}
        self.assertEqual(editing, {f"ED{number:02d}" for number in range(1, 29)})
        self.assertFalse(editing & inherited)

    def test_six_source_conflicts_remain_unresolved(self):
        text = conflict_texts(self.workbook["conflicts"])
        self.assertEqual([row["id"] for row in self.workbook["conflicts"]], [f"CF0{number}" for number in range(1, 7)])
        self.assertIn("100 built-in fonts", text)
        self.assertIn("130 fonts with filter and mapping", text)
        self.assertIn("Hundreds of built-in fonts", text)
        self.assertIn("do not sum or silently pick", text)
        self.assertIn("Windows 11/10/8.1/7", text)
        self.assertIn("Windows 7/8.1/10", text)
        self.assertIn("1.5 GHz", text)
        self.assertIn("512 MB", text)
        self.assertIn("360 x 360", text)
        self.assertIn("272 x 408", text)
        self.assertIn("Stitch Stimulation", text)
        self.assertIn("product identity not established", text)

    def test_contract_json_matches_the_workbook(self):
        self.assertEqual(self.sources["sources"], self.workbook["sources"])
        self.assertEqual(self.conflicts["conflicts"], self.workbook["conflicts"])

    def test_every_cited_source_resolves(self):
        known = {row["id"] for row in self.workbook["sources"]}
        cited = set()
        for row in self.requirements:
            cited.update(part.strip() for part in row["source_ids"].split(",") if part.strip())
        self.assertTrue(cited <= known)
        self.assertTrue({"U1", "U2", "L02", "M1", "R1", "B1", "B10", "E1", "N1", "W1", "W3", "I1", "P1"} <= known)

    def test_preparation_statements_are_preserved(self):
        m1 = next(row for row in self.workbook["sources"] if row["id"] == "M1")
        self.assertIn("not passed", m1["interpretation"])
        self.assertIn("98303601416a3b0d68eae37e939031732bd77c45", next(row["url"] for row in self.workbook["sources"] if row["id"] == "R1"))
        self.assertIn("DST writer cannot satisfy", self.workbook["acceptance"][self.requirements.index(self.by_id["SW01"])]["negative_case"])
        self.assertIn("never substitute a running stitch", self.by_id["L09"]["requirement"])

    def test_library_subset_is_not_general_parity(self):
        subsets = [row["id"] for row in self.requirements if row["implementation_status"] == "LIBRARY_SUBSET"]
        self.assertEqual(subsets, ["L01", "S01", "S02", "Q10"])
        self.assertEqual(self.by_id["G03"]["implementation_status"], "REFERENCE_UTILITY")
        self.assertEqual(
            sorted(row["id"] for row in self.requirements if row["implementation_status"] == "CANDIDATE_SUBSET"),
            ["ED18", "G01", "T18"],
        )

    def test_dashboard_formulas_recompute_without_error_tokens(self):
        self.assertEqual(self.findings_for(), [])
        xml = self.workbook["xml_text"]
        for token in ("#REF!", "#VALUE!", "#DIV/0!", "#NAME?", "#N/A", "#NULL!", "#NUM!"):
            self.assertNotIn(token, xml)

    def test_duplicate_identifier_is_rejected(self):
        mutated = copy.deepcopy(self.workbook)
        mutated["requirements"][1]["id"] = mutated["requirements"][0]["id"]
        self.assertTrue(any("collide" in finding for finding in self.findings_for(workbook=mutated)))

    def test_changed_inherited_source_text_is_detected(self):
        mutated = copy.deepcopy(self.baseline)
        mutated["requirements"][0]["source_text"] = ""
        self.assertTrue(any("source text" in finding for finding in self.findings_for(baseline=mutated)))

    def test_snapshot_drift_is_detected(self):
        self.assertTrue(any("snapshot" in finding for finding in self.findings_for(snapshot_matches=False)))

    def test_acceptance_without_evidence_fails_the_structural_audit(self):
        mutated = copy.deepcopy(self.workbook)
        mutated["requirements"][0]["acceptance_status"] = "ACCEPTED_SCOPED"
        self.assertTrue(any("without an evidence reference" in finding for finding in self.findings_for(workbook=mutated)))

    def test_one_accepted_row_does_not_close_the_programme(self):
        mutated = copy.deepcopy(self.requirements)
        mutated[0]["acceptance_status"] = "ACCEPTED_SCOPED"
        self.assertEqual(acceptance_findings(mutated), ["1 of 210 rows have accepted evidence"])

    def test_accepted_dependency_must_itself_be_accepted(self):
        mutated = copy.deepcopy(self.workbook)
        target = next(row for row in mutated["requirements"] if row["id"] == "A01")
        target["acceptance_status"] = "ACCEPTED_SCOPED"
        acceptance = next(row for row in mutated["acceptance"] if row["id"] == "A01")
        acceptance["evidence"] = "synthetic-ledger-entry"
        findings = self.findings_for(workbook=mutated)
        self.assertTrue(any("depends on" in finding for finding in findings))

    def test_structural_command_exits_zero(self):
        completed = subprocess.run([str(PYTHON), str(AUDITOR)], cwd=ROOT, text=True, capture_output=True, check=False)
        self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
        self.assertIn("structural audit PASS", completed.stdout)
        report = json.loads((ROOT / "evidence" / "baseline-0.3" / "registry-audit.json").read_text())
        self.assertFalse(report["authenticates_evidence"])
        self.assertFalse(report["machine_release_allowed"])
        self.assertFalse(report["iso_certification"])
        self.assertFalse(report["qualifies_platform"])

    def test_new_hardware_floor_does_not_replace_the_0_2_baseline(self):
        self.assertIn("do not replace the earlier 800 MB baseline", self.by_id["H06"]["requirement"])
        self.assertIn("without changing the earlier 1.5 GHz target", self.by_id["H03"]["requirement"])
        self.assertIn("1.5 GHz Intel or AMD processor", self.baseline["requirements"][0]["source_text"])

    def test_lettering_and_catalogue_counts_are_not_delivered_assets(self):
        self.assertIn("not delivered assets", self.by_id["F01"]["notes"])
        self.assertIn("at least 130 qualified families", self.by_id["F01"]["requirement"])
        self.assertEqual(self.by_id["F01"]["acceptance_status"], "NOT_RUN")

    def test_process_artifact_kinds_stay_separate(self):
        self.assertIn("decorative_sewing", self.by_id["G05"]["requirement"])
        self.assertIn("cutting", self.by_id["G05"]["requirement"])
        self.assertIn("seven workflow responsibilities", self.by_id["C11"]["requirement"])

    def test_delivery_gates_remain_proposed_targets(self):
        self.assertEqual(len(self.workbook["gates"]), 7)
        joined = "\n".join(row["pass_condition"] for row in self.workbook["gates"])
        self.assertIn("30 cases", joined)
        self.assertIn("25,000-event", joined)
        self.assertIn("non-inferiority", self.workbook["gates"][-1]["pass_condition"])

    def test_acceptance_command_exits_one(self):
        completed = subprocess.run(
            [str(PYTHON), str(AUDITOR), "--require-acceptance"],
            cwd=ROOT, text=True, capture_output=True, check=False,
        )
        self.assertEqual(completed.returncode, 1, completed.stdout + completed.stderr)
        self.assertIn("0 of 210 rows have accepted evidence", completed.stdout)
        gate = json.loads((ROOT / "evidence" / "baseline-0.3" / "acceptance-gate.json").read_text())
        self.assertEqual(gate["result"], "FAIL")
        self.assertFalse(gate["machine_release_allowed"])


if __name__ == "__main__":
    unittest.main()
