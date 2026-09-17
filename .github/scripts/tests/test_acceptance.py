import copy
import unittest

from lib.acceptance import collect, declaration, intent, validate, verify
from lib.blob import Refusal, fingerprint
from support import Fixture


class Acceptance(unittest.TestCase):
    def setUp(self):
        self.fixture = Fixture()
        self.contract = {"schema": "plumb.source-suite/v1", "graph": self.fixture.value,
                         "checks": {"delivery": "deliver", "independent": "independent"}}
        self.context = {"repository": "PerishLab/plumb", "workflow": ".github/workflows/ship.yml",
                        "control": "a" * 40, "run": 17, "attempt": 1}
        self.contract.update(source="b" * 40, configuration="c" * 64, control="a" * 40,
                             jobs={"delivery": "suite (linux)", "independent": "suite (macos)"})
        self.value = intent("b" * 40, "c" * 64, fingerprint(self.contract), self.context)
        self.observation = {key: copy.deepcopy(self.value[key]) for key in
                            ("context", "source", "configuration", "contract")}
        self.observation["conclusion"] = "success"

    def test_candidate_and_trusted_control_are_separate(self):
        self.assertNotEqual(self.value["source"], self.value["context"]["control"])
        self.context["control"] = "d" * 40
        self.assertEqual(self.value["context"]["control"], "a" * 40)
        self.assertEqual(validate(self.value), self.value)

    def test_no_floating_or_external_source(self):
        for source in ("main", "v1.0.0", "HEAD", "a" * 39, "A" * 40, "../source", None):
            with self.subTest(source=source), self.assertRaises(Refusal):
                intent(source, "c" * 64, fingerprint(self.contract), self.context)
        for field, other in (("repository", "other/plumb"), ("workflow", ".github/workflows/other.yml"),
                             ("run", 0), ("attempt", True), ("control", "main")):
            context = dict(self.context, **{field: other})
            with self.subTest(field=field), self.assertRaises(Refusal):
                intent("b" * 40, "c" * 64, fingerprint(self.contract), context)

    def test_no_extra_inputs_or_invalid_digest(self):
        with self.assertRaises(Refusal):
            validate(dict(self.value, skip_guard=True))
        for field in ("configuration", "contract"):
            with self.subTest(field=field), self.assertRaises(Refusal):
                validate(dict(self.value, **{field: "latest"}))

    def test_contract_change_needs_explicit_selection(self):
        changed = copy.deepcopy(self.contract)
        changed["checks"].pop("delivery")
        with self.assertRaises(Refusal):
            declaration(self.value, changed)
        selected = dict(self.value, contract=fingerprint(changed))
        with self.assertRaises(Refusal):
            declaration(selected, changed)

    def test_self_report_does_not_replace_actual_completion(self):
        with self.assertRaises(Refusal):
            collect(self.value, self.contract, self.fixture.inventory, self.observation)

    def test_malformed_check_names_and_targets_refuse(self):
        for checks in ({" ": "deliver"}, {"delivery": []}, {"delivery": {}}):
            changed = dict(self.contract, checks=checks)
            selected = dict(self.value, contract=fingerprint(changed))
            with self.subTest(checks=checks), self.assertRaises(Refusal):
                declaration(selected, changed)

    def test_valid_contract_for_other_source_cannot_be_relabelled(self):
        for field, other in (("source", "d" * 40), ("configuration", "d" * 64), ("control", "d" * 40)):
            changed = dict(self.contract, **{field: other})
            selected = dict(self.value, contract=fingerprint(changed))
            with self.subTest(field=field), self.assertRaises(Refusal):
                declaration(selected, changed)

    def test_independent_run_must_match_every_binding(self):
        self.fixture.seed()
        for field, replacement in (("source", "d" * 40), ("configuration", "d" * 64),
                                   ("contract", "d" * 64), ("conclusion", "failure"),
                                   ("context", dict(self.context, attempt=2))):
            observed = dict(self.observation, **{field: replacement})
            with self.subTest(field=field), self.assertRaises(Refusal):
                collect(self.value, self.contract, self.fixture.inventory, observed)

    def test_receipt_requires_remote_observation_and_blob_readback(self):
        self.fixture.seed()
        receipt = collect(self.value, self.contract, self.fixture.inventory, self.observation)
        self.assertEqual(verify(receipt, self.value, self.contract, self.fixture.inventory,
                                self.observation), fingerprint(receipt))
        changed = copy.deepcopy(receipt)
        changed["checks"].pop("delivery")
        with self.assertRaises(Refusal):
            verify(changed, self.value, self.contract, self.fixture.inventory, self.observation)

    def test_receipt_has_no_release_identity_or_activation_grant(self):
        self.fixture.seed()
        receipt = collect(self.value, self.contract, self.fixture.inventory, self.observation)
        self.assertEqual(set(receipt), {"schema", "intent", "checks"})
        for field in ("marker", "channel", "stable", "published", "guard", "permissions"):
            with self.subTest(field=field), self.assertRaises(Refusal):
                verify(dict(receipt, **{field: True}), self.value, self.contract,
                       self.fixture.inventory, self.observation)

    def test_run_provenance_does_not_invalidate_blob_keys(self):
        self.fixture.seed()
        before = self.fixture.plan()
        retried = copy.deepcopy(self.value)
        retried["context"]["attempt"] = 2
        declaration(retried, self.contract)
        self.assertEqual(before, self.fixture.plan())
        self.assertEqual(self.fixture.store.writes, [])
