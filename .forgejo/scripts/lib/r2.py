import base64
import os
import re
import subprocess
import tempfile
from pathlib import Path
from urllib.parse import urlsplit

from .blob import Refusal, Unknown, decode, digest


class R2:
    def __init__(self, endpoint, bucket, environment):
        address = urlsplit(endpoint)
        if address.scheme != "https" or not address.netloc or address.username or address.password:
            raise Refusal("inventory requires an HTTPS S3 endpoint")
        if address.query or address.fragment or address.path not in ("", "/"):
            raise Refusal("inventory endpoint must not carry a route or query")
        if not re.fullmatch(r"[a-z0-9][a-z0-9.-]{1,61}[a-z0-9]", bucket):
            raise Refusal("invalid inventory bucket")
        self.endpoint = endpoint.rstrip("/")
        self.bucket = bucket
        self.environment = environment

    @classmethod
    def environment(cls):
        prefix = "PLUMB_WORKFLOW_INVENTORY_"
        values = {key: os.environ.get(prefix + key, "") for key in
                  ("ACCESS", "SECRET", "BUCKET", "ENDPOINT")}
        if not all(values.values()):
            raise Refusal("incomplete workflow inventory authority")
        environment = dict(os.environ)
        environment.update(AWS_ACCESS_KEY_ID=values["ACCESS"],
                           AWS_SECRET_ACCESS_KEY=values["SECRET"],
                           AWS_DEFAULT_REGION="auto", AWS_EC2_METADATA_DISABLED="true",
                           AWS_MAX_ATTEMPTS="2", AWS_RETRY_MODE="standard")
        environment.pop("AWS_SESSION_TOKEN", None)
        return cls(values["ENDPOINT"], values["BUCKET"], environment)

    def call(self, operation, key, arguments):
        if not re.fullmatch(r"v2/(?:blobs/sha256/[0-9a-f]{64}|records/[0-9a-f]{64}\.json)", key):
            raise Refusal("inventory key outside the blob contract")
        command = ["aws", "--endpoint-url", self.endpoint, "--cli-connect-timeout", "5",
                   "--cli-read-timeout", "20", "s3api", operation,
                   "--bucket", self.bucket, "--key", key, "--no-cli-pager", *arguments]
        try:
            result = subprocess.run(command, env=self.environment, capture_output=True, timeout=60)
        except (OSError, subprocess.TimeoutExpired) as error:
            raise Unknown("inventory transport unavailable") from error
        if result.returncode == 0:
            return result.stdout
        error = re.search(rb"An error occurred \(([^)]+)\) when calling", result.stderr)
        code = error[1].decode("ascii", errors="replace") if error else ""
        if operation in ("get-object", "head-object") and code == "NoSuchKey":
            return None
        if operation == "put-object" and code == "PreconditionFailed":
            return None
        raise Unknown("inventory operation failed; existence is unknown")

    def read(self, key):
        with tempfile.TemporaryDirectory(prefix="plumb-blob-read-") as directory:
            path = Path(directory) / "body"
            if self.call("get-object", key, [str(path)]) is None:
                return None
            return path.read_bytes()

    def create(self, key, body):
        with tempfile.TemporaryDirectory(prefix="plumb-blob-write-") as directory:
            path = Path(directory) / "body"
            path.write_bytes(body)
            checksum = base64.b64encode(bytes.fromhex(digest(body))).decode("ascii")
            return self.call("put-object", key,
                             ["--body", str(path), "--if-none-match", "*",
                              "--checksum-algorithm", "SHA256",
                              "--checksum-sha256", checksum])

    def verify(self, key, expected):
        body = self.call("head-object", key, ["--checksum-mode", "ENABLED"])
        if body is None:
            raise Refusal("completion references a missing blob")
        held = decode(body)
        checksum = base64.b64encode(bytes.fromhex(expected)).decode("ascii")
        if held.get("ChecksumSHA256") != checksum:
            raise Refusal("blob lacks matching server-verified SHA-256 evidence")
