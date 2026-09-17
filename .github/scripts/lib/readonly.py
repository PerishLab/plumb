import http.client
import re
from urllib.parse import urlsplit

from .blob import Refusal, Unknown, digest, sha
from .transport import exchange


class Readonly:
    """Public delivery of the same inventory, without any writer authority."""

    def __init__(self, origin):
        if not isinstance(origin, str) or any(char.isspace() or ord(char) < 32 for char in origin):
            raise Refusal("invalid public inventory origin")
        address = urlsplit(origin)
        if (address.scheme != "https" or not address.hostname or address.username
                or address.password or address.query or address.fragment
                or address.path not in ("", "/")):
            raise Refusal("public inventory requires an HTTPS origin without credentials or a route")
        self.origin = origin.rstrip("/")
        self.host = address.netloc

    def read(self, key):
        if not isinstance(key, str) or not re.fullmatch(
                r"v2/(?:blobs/sha256/[0-9a-f]{64}|records/[0-9a-f]{64}\.json)", key):
            raise Refusal("inventory key outside the blob contract")
        try:
            status, _, body = exchange(self.origin + "/" + key, "GET",
                                       {"host": self.host, "cache-control": "no-cache"}, b"")
        except (OSError, http.client.HTTPException) as error:
            raise Unknown("public inventory transport unavailable: " + type(error).__name__) from error
        if status == 200:
            return body
        # A public HTTP 404 is not authoritative S3 NoSuchKey evidence.
        # Refuse uncertainty instead of scheduling a potentially duplicate build.
        raise Unknown(f"public inventory read failed (HTTP {status}); existence is unknown")

    def verify(self, key, expected):
        if digest(self.read(key)) != sha(expected):
            raise Refusal("public inventory blob differs from its SHA-256")

    def create(self, key, body):
        raise Refusal("public inventory cannot publish blobs or completion records")
