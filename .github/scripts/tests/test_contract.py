import copy
import tempfile
import unittest
from pathlib import Path

from lib.blob import Conflict, Refusal, Unknown, decode, digest, encode, fingerprint
from lib.declaration import Declaration
from lib.inventory import Inventory
from lib.planner import action_key
from record import complete
from support import Fixture, Memory, declaration, reference


class Contract(unittest.TestCase):
    def test_key_has_only_the_three_declared_surfaces(self):
        node = declaration()["nodes"][0]
        expected = {"contract": "plumb.blob-key/v3",
                    "configuration": {"inputs": sorted(node["inputs"]),
                                      "outputs": sorted(node["outputs"]), "evidence": [], "materialize": []},
                    "references": node["inputs"]}
        self.assertEqual(action_key(node, node["inputs"]), fingerprint(expected))
        changed = copy.deepcopy(expected)
        changed["contract"] = "plumb.blob-key/v4"
        self.assertNotEqual(action_key(node, node["inputs"]), fingerprint(changed))

    def test_duplicate_json_fields_refuse(self):
        with self.assertRaises(Refusal):
            decode(b'{"key": 1, "key": 2}')

    def test_nonfinite_values_refuse(self):
        with self.assertRaises(Refusal):
            decode(b'{"value": NaN}')

    def test_symbolic_inputs_are_not_production_digests(self):
        value = declaration()
        value["nodes"][0]["inputs"]["source"] = "source-A"
        with self.assertRaises(Refusal):
            Declaration(value)

    def test_missing_implementation_refuses(self):
        value = declaration()
        del value["nodes"][0]["inputs"]["implementation"]
        with self.assertRaises(Refusal):
            Declaration(value)

    def test_cycle_including_requires_refuses(self):
        value = declaration()
        value["nodes"][0]["requires"] = [reference("deliver", "receipt")]
        with self.assertRaises(Refusal):
            Declaration(value)

    def test_unknown_output_refuses(self):
        value = declaration()
        value["nodes"][1]["inputs"]["content"]["output"] = "missing"
        with self.assertRaises(Refusal):
            Declaration(value)

    def test_unknown_fields_refuse(self):
        value = declaration()
        value["business"] = "not generic"
        with self.assertRaises(Refusal):
            Declaration(value)

    def test_invalid_targets_refuse(self):
        for target in ({}, "absent"):
            value = declaration()
            value["targets"] = [target]
            with self.assertRaises(Refusal):
                Declaration(value)

    def test_routes_and_node_labels_do_not_invalidate_content(self):
        node = declaration()["nodes"][0]
        other = copy.deepcopy(node)
        other["id"] = "renamed"
        other["execution"]["capability"] = "another-runner"
        self.assertEqual(action_key(node, node["inputs"]), action_key(other, other["inputs"]))

    def test_output_contract_is_in_key(self):
        node = declaration()["nodes"][0]
        other = copy.deepcopy(node)
        other["outputs"] = ["different"]
        self.assertNotEqual(action_key(node, node["inputs"]), action_key(other, other["inputs"]))

    def test_missing_blob_cannot_create_completion(self):
        store = Memory()
        inventory = Inventory(store)
        with self.assertRaises(Refusal):
            inventory.complete(digest(b"key"), {"receipt": digest(b"missing")})
        self.assertEqual(store.writes, [])

    def test_invisible_upload_never_marks_complete(self):
        store = Memory()
        store.create = lambda key, body: None
        inventory = Inventory(store)
        with self.assertRaises(Refusal):
            inventory.upload(b"new")
        self.assertEqual(store.objects, {})

    def test_wrong_record_identity_refuses(self):
        fixture = Fixture()
        fixture.seed()
        key = fixture.plan()["nodes"]["produce"]["key"]
        route = fixture.inventory.record(key)
        record = decode(fixture.store.objects[route])
        record["key"] = digest(b"wrong")
        fixture.store.objects[route] = encode(record)
        with self.assertRaises(Refusal):
            fixture.plan()

    def test_record_requires_the_executed_key(self):
        fixture = Fixture()
        graph = Declaration(fixture.value)
        with self.assertRaises(Refusal):
            complete(graph, "produce",
                     {"key": digest(b"wrong"), "outputs": {"content": "/unused"}},
                     fixture.inventory)
        self.assertEqual(fixture.store.writes, [])

    def test_record_waits_for_prerequisites(self):
        fixture = Fixture()
        fixture.seed()
        fixture.change("check", "implementation", b"changed")
        key = fixture.plan()["nodes"]["deliver"]["key"]
        with self.assertRaises(Refusal):
            complete(Declaration(fixture.value), "deliver",
                     {"key": key, "outputs": {"receipt": "/unused"}}, fixture.inventory)
        self.assertEqual(fixture.store.writes, [])

    def test_record_replays_exact_completion_without_replacement(self):
        fixture = Fixture()
        key = fixture.plan()["nodes"]["produce"]["key"]
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "body"
            path.write_bytes(b"output")
            result = {"key": key, "outputs": {"content": str(path)}}
            first = complete(Declaration(fixture.value), "produce", result, fixture.inventory)
            self.assertEqual(first, complete(Declaration(fixture.value), "produce",
                                             result, fixture.inventory))
            path.write_bytes(b"different")
            with self.assertRaises(Conflict):
                complete(Declaration(fixture.value), "produce", result, fixture.inventory)

    def test_lost_write_response_is_reconciled_from_exact_evidence(self):
        store = Memory()
        create = store.create
        def uncertain(key, body):
            create(key, body)
            raise Unknown("lost acknowledgement")
        store.create = uncertain
        inventory = Inventory(store)
        output = inventory.upload(b"body")
        record = inventory.complete(digest(b"key"), {"receipt": output})
        self.assertEqual(record["outputs"]["receipt"], digest(b"body"))

    def test_missing_after_uncertain_completion_stays_unknown(self):
        store = Memory()
        inventory = Inventory(store)
        output = inventory.upload(b"body")
        def uncertain(key, body):
            raise Unknown("lost acknowledgement")
        store.create = uncertain
        with self.assertRaises(Unknown):
            inventory.complete(digest(b"key"), {"receipt": output})

    def test_record_does_not_depend_on_unrelated_branch_inventory(self):
        fixture = Fixture()
        held = fixture.plan()
        unrelated = fixture.inventory.record(held["nodes"]["independent"]["key"])
        read = fixture.store.read
        def restricted(key):
            if key == unrelated:
                raise Unknown("unrelated read failure")
            return read(key)
        fixture.store.read = restricted
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "body"
            path.write_bytes(b"output")
            complete(Declaration(fixture.value), "produce",
                     {"key": held["nodes"]["produce"]["key"],
                      "outputs": {"content": str(path)}}, fixture.inventory)


if __name__ == "__main__":
    unittest.main()
