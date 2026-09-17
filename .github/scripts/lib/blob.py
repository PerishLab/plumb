import hashlib
import json
import re


class Refusal(ValueError):
    pass


class Unknown(Refusal):
    pass


class Conflict(Refusal):
    pass


def encode(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True,
                      allow_nan=False).encode("utf-8")


def digest(body):
    return hashlib.sha256(body).hexdigest()


def fingerprint(value):
    return digest(encode(value))


def sha(value):
    if not isinstance(value, str) or not re.fullmatch(r"[0-9a-f]{64}", value):
        raise Refusal("expected a lowercase SHA-256 digest")
    return value


def object_fields(value, required, optional=()):
    if not isinstance(value, dict):
        raise Refusal("expected an object")
    if not set(required) <= value.keys() or value.keys() - set(required) - set(optional):
        raise Refusal("missing or unknown contract fields")


def decode(body):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise Refusal("duplicate JSON field")
            result[key] = value
        return result
    try:
        return json.loads(body, object_pairs_hook=unique,
                          parse_constant=lambda value: reject_constant())
    except (ValueError, UnicodeError) as error:
        raise Refusal("invalid JSON evidence") from error


def reject_constant():
    raise Refusal("non-finite JSON number")


def name(value):
    if not isinstance(value, str) or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._/-]*", value):
        raise Refusal("invalid declaration name")
    return value
