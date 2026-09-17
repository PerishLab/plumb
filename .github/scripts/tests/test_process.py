import unittest
from unittest.mock import patch

from lib.blob import Refusal
from lib.process import environment


class Environment(unittest.TestCase):
    def test_ambient_wrappers_are_replaced_only_by_the_execution_owner(self):
        contract = {"inherit": ["PATH"], "managed": ["RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"],
                    "reject": ["RUST*"]}
        ambient = {"PATH": "/tools", "RUSTC_WRAPPER": "ambient", "RUSTC_WORKSPACE_WRAPPER": "ambient"}
        with patch.dict("os.environ", ambient, clear=True):
            self.assertEqual(environment(contract, {}), {"PATH": "/tools"})
            self.assertEqual(environment(contract, {"RUSTC_WRAPPER": "declared"}),
                             {"PATH": "/tools", "RUSTC_WRAPPER": "declared"})

    def test_declared_mirror_overrides_do_not_leak_into_package_execution(self):
        managed = ["NPM_CONFIG_REGISTRY", "PNPM_CONFIG_REGISTRY",
                   "PNPM_CONFIG_FETCH_TIMEOUT", "NODE_EXTRA_CA_CERTS"]
        contract = {"inherit": ["PATH"], "managed": managed, "reject": ["NPM_CONFIG_*", "PNPM_*", "NODE_*"]}
        with patch.dict("os.environ", {name: "ambient" for name in managed}, clear=True):
            self.assertEqual(environment(contract, {}), {})
        with patch.dict("os.environ", {"NODE_OPTIONS": "unexpected"}, clear=True):
            with self.assertRaises(Refusal):
                environment(contract, {})

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
