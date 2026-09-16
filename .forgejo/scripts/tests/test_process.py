import unittest
from unittest.mock import patch

from lib.blob import Refusal
from lib.process import environment


class Environment(unittest.TestCase):
    def test_managed_transport_setting_is_not_inherited(self):
        contract = {"inherit": ["PATH"], "managed": ["CARGO_HTTP_CAINFO"], "reject": ["CARGO_*"]}
        with patch.dict("os.environ", {"PATH": "/tools", "CARGO_HTTP_CAINFO": "/ambient/ca"}, clear=True):
            self.assertEqual(environment(contract, {}), {"PATH": "/tools"})

    def test_other_undeclared_cargo_settings_still_refuse(self):
        contract = {"inherit": ["PATH"], "managed": ["CARGO_HTTP_CAINFO"], "reject": ["CARGO_*"]}
        with patch.dict("os.environ", {"CARGO_ENCODED_RUSTFLAGS": "unexpected"}, clear=True):
            with self.assertRaises(Refusal):
                environment(contract, {})

    def test_managed_value_only_comes_from_the_execution_owner(self):
        contract = {"inherit": [], "managed": ["CARGO_HTTP_CAINFO"], "reject": ["CARGO_*"]}
        with patch.dict("os.environ", {"CARGO_HTTP_CAINFO": "/ambient/ca"}, clear=True):
            self.assertEqual(environment(contract, {"CARGO_HTTP_CAINFO": "/declared/ca"}),
                             {"CARGO_HTTP_CAINFO": "/declared/ca"})
