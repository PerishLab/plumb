import hashlib
import hmac

from .blob import digest


def sign(method, path, headers, identity):
    names = ";".join(sorted(headers))
    canonical = "".join(name + ":" + " ".join(headers[name].split()) + "\n"
                        for name in sorted(headers))
    request = "\n".join((method, path, "", canonical, names, headers["x-amz-content-sha256"]))
    date = headers["x-amz-date"][:8]
    scope = "/".join((date, identity["region"], "s3", "aws4_request"))
    message = "\n".join(("AWS4-HMAC-SHA256", headers["x-amz-date"], scope, digest(request.encode())))
    key = ("AWS4" + identity["secret"]).encode()
    for part in (date, identity["region"], "s3", "aws4_request"):
        key = hmac.new(key, part.encode(), hashlib.sha256).digest()
    signature = hmac.new(key, message.encode(), hashlib.sha256).hexdigest()
    authorization = (f"AWS4-HMAC-SHA256 Credential={identity['access']}/{scope},"
                     f"SignedHeaders={names},Signature={signature}")
    return {**headers, "authorization": authorization}
