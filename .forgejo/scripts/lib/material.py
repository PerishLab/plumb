from pathlib import Path
from urllib.parse import urlsplit
from urllib.request import urlopen

from .blob import Refusal, Unknown, digest, object_fields, sha


def download(reference, destination):
    object_fields(reference, ("digest", "reuse"))
    expected = sha(reference["digest"])
    carrier = reference["reuse"]
    object_fields(carrier, ("type", "source"))
    address = urlsplit(carrier["source"])
    if carrier["type"] != "workload" or address.scheme != "https" or not address.hostname:
        raise Refusal("materialization requires an HTTPS workload")
    if address.username or address.password or address.query or address.fragment:
        raise Refusal("workload address must not carry credentials or parameters")
    if address.path != "/v2/blobs/sha256/" + expected:
        raise Refusal("workload address differs from its blob digest")
    try:
        with urlopen(carrier["source"], timeout=30) as response:
            if urlsplit(response.url).scheme != "https":
                raise Refusal("workload redirected outside HTTPS")
            body = response.read()
    except OSError as error:
        raise Unknown("workload download unavailable") from error
    if digest(body) != expected:
        raise Refusal("downloaded workload differs from its digest")
    path = Path(destination)
    with path.open("xb") as output:
        output.write(body)
    if digest(path.read_bytes()) != expected:
        raise Refusal("materialized workload failed readback")
    return path
