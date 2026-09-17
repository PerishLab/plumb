import base64
import datetime
import http.client
import os
import re
from urllib.parse import urlsplit
from xml.etree import ElementTree

from .blob import Refusal, Unknown, digest
from .signature import sign
from .transport import exchange


def error_code(body):
    try:
        code = ElementTree.fromstring(body).findtext("Code", "")
    except (ElementTree.ParseError, ValueError):
        return ""
    return code if re.fullmatch(r"[A-Za-z0-9_-]{1,64}", code) else ""


class R2:
    def __init__(self, endpoint, bucket, identity):
        address = urlsplit(endpoint)
        if address.scheme != "https" or not address.netloc or address.username or address.password:
            raise Refusal("inventory requires an HTTPS S3 endpoint")
        if address.query or address.fragment or address.path not in ("", "/"):
            raise Refusal("inventory endpoint must not carry a route or query")
        if any(character.isspace() or ord(character) < 32 for character in endpoint):
            raise Refusal("invalid inventory endpoint")
        if not re.fullmatch(r"[a-z0-9][a-z0-9.-]{1,61}[a-z0-9]", bucket):
            raise Refusal("invalid inventory bucket")
        if any(not identity.get(key) or any(ord(char) < 33 or ord(char) > 126
                                           for char in identity[key]) for key in ("access", "secret")):
            raise Refusal("incomplete workflow inventory authority")
        self.endpoint = endpoint.rstrip("/")
        self.bucket = bucket
        self.identity = {**identity, "region": "auto"}

    @classmethod
    def environment(cls):
        prefix = "PLUMB_WORKFLOW_INVENTORY_"
        values = {key: os.environ.get(prefix + key, "") for key in
                  ("ACCESS", "SECRET", "BUCKET", "ENDPOINT")}
        if not all(values.values()):
            raise Refusal("incomplete workflow inventory authority")
        return cls(values["ENDPOINT"], values["BUCKET"],
                   {"access": values["ACCESS"], "secret": values["SECRET"]})

    def call(self, method, key, body=b""):
        if not re.fullmatch(r"v2/(?:blobs/sha256/[0-9a-f]{64}|records/[0-9a-f]{64}\.json)", key):
            raise Refusal("inventory key outside the blob contract")
        operations = {"GET": "get-object", "HEAD": "head-object", "PUT": "put-object"}
        if method not in operations:
            raise Refusal("inventory operation outside the blob contract")
        operation = operations[method]
        path = "/" + self.bucket + "/" + key
        headers = {"host": urlsplit(self.endpoint).netloc,
                   "x-amz-date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ"),
                   "x-amz-content-sha256": digest(body)}
        if method == "PUT":
            headers.update({"if-none-match": "*", "content-length": str(len(body)),
                            "x-amz-checksum-sha256": base64.b64encode(bytes.fromhex(digest(body))).decode()})
        if method == "HEAD":
            headers["x-amz-checksum-mode"] = "ENABLED"
        try:
            status, held, content = exchange(self.endpoint + path, method,
                                              sign(method, path, headers, self.identity), body)
        except (OSError, http.client.HTTPException) as error:
            raise Unknown(f"inventory {operation} transport unavailable: {type(error).__name__}") from error
        if status == 200:
            return {name.lower(): value for name, value in held.items()}, content
        code = error_code(content)
        if method == "GET" and status == 404 and code == "NoSuchKey":
            return None
        if method == "PUT" and status == 412 and code == "PreconditionFailed":
            return None
        diagnostic = code or f"HTTP {status}"
        raise Unknown(f"inventory {operation} failed ({diagnostic}); existence is unknown")

    def read(self, key):
        result = self.call("GET", key)
        return None if result is None else result[1]

    def create(self, key, body):
        result = self.call("PUT", key, body)
        return None if result is None else result[1]

    def verify(self, key, expected):
        headers, _ = self.call("HEAD", key)
        checksum = base64.b64encode(bytes.fromhex(expected)).decode("ascii")
        if headers.get("x-amz-checksum-sha256") != checksum:
            raise Refusal("blob lacks matching server-verified SHA-256 evidence")
