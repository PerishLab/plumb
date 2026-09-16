import json
import tomllib

from .blob import Refusal, decode, encode, object_fields


def project(body, recipe):
    object_fields(recipe, ("format", "set"))
    if recipe["format"] == "json":
        value = decode(body)
    elif recipe["format"] == "toml":
        try:
            value = tomllib.loads(body.decode("utf-8"))
        except (ValueError, UnicodeError) as error:
            raise Refusal("invalid projected TOML") from error
    else:
        raise Refusal("unsupported projection format")
    if not isinstance(recipe["set"], dict):
        raise Refusal("projection set must be a pointer map")
    pointers = sorted(recipe["set"])
    for index, pointer in enumerate(pointers):
        if any(other.startswith(pointer + "/") for other in pointers[index + 1:]):
            raise Refusal("projection pointers overlap")
        replace(value, pointer, recipe["set"][pointer])
    try:
        if recipe["format"] == "json":
            return encode(value)
        return toml(value)
    except (TypeError, ValueError, UnicodeError) as error:
        raise Refusal("projected document cannot be represented losslessly") from error


def replace(value, pointer, replacement):
    if not isinstance(pointer, str) or not pointer.startswith("/"):
        raise Refusal("projection requires a JSON pointer")
    tokens = pointer[1:].split("/")
    for index, token in enumerate(tokens):
        if "~" in token.replace("~0", "").replace("~1", ""):
            raise Refusal("invalid JSON pointer escape")
        token = token.replace("~1", "/").replace("~0", "~")
        if not isinstance(value, (dict, list)):
            raise Refusal("projection pointer crosses a scalar")
        if isinstance(value, list):
            if not token.isascii() or not token.isdecimal() or str(int(token)) != token:
                raise Refusal("invalid JSON pointer index")
            token = int(token)
        try:
            previous = value[token]
            if index == len(tokens) - 1:
                value[token] = replacement
            else:
                value = previous
        except (KeyError, IndexError, TypeError) as error:
            raise Refusal("projection pointer is absent") from error


def atom(value):
    if isinstance(value, dict):
        return "{" + ", ".join(f"{atom(key)} = {atom(held)}"
                               for key, held in sorted(value.items())) + "}"
    if isinstance(value, list):
        return "[" + ", ".join(atom(held) for held in value) + "]"
    if value is None:
        raise Refusal("TOML projection cannot contain null")
    return json.dumps(value, ensure_ascii=False, allow_nan=False)


def toml(value):
    if not isinstance(value, dict):
        raise Refusal("TOML document must be a table")
    body = "".join(f"{atom(key)} = {atom(held)}\n" for key, held in sorted(value.items()))
    if encode(tomllib.loads(body)) != encode(value):
        raise Refusal("TOML projection changed document values")
    return body.encode("utf-8")
