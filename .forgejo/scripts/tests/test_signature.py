import unittest

from lib.blob import digest
from lib.signature import sign


class Signature(unittest.TestCase):
    def test_aws_published_get_object_vector(self):
        identity = {"access": "AKIAIOSFODNN7EXAMPLE",
                    "secret": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY", "region": "us-east-1"}
        headers = {"host": "examplebucket.s3.amazonaws.com", "range": "bytes=0-9",
                   "x-amz-content-sha256": digest(b""), "x-amz-date": "20130524T000000Z"}
        result = sign("GET", "/test.txt", headers, identity)
        expected = "f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
        self.assertTrue(result["authorization"].endswith("Signature=" + expected))
        self.assertNotIn("authorization", headers)
        self.assertNotIn(identity["secret"], result["authorization"])

    def test_every_object_and_condition_is_signed(self):
        identity = {"access": "fixture", "secret": "fixture", "region": "auto"}
        headers = {"host": "inventory.test", "x-amz-date": "20260916T000000Z",
                   "x-amz-content-sha256": digest(b"body"), "if-none-match": "*"}
        held = sign("PUT", "/inventory/object", headers, identity)["authorization"]
        for method, path, changed in [
            ("GET", "/inventory/object", headers),
            ("PUT", "/inventory/another", headers),
            ("PUT", "/inventory/object", {**headers, "if-none-match": "other"}),
            ("PUT", "/inventory/object", {**headers, "x-amz-content-sha256": digest(b"other")}),
        ]:
            self.assertNotEqual(sign(method, path, changed, identity)["authorization"], held)


if __name__ == "__main__":
    unittest.main()
