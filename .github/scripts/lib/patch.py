from .blob import Refusal, digest, object_fields


def apply(body, patch):
    object_fields(patch, ("source", "result", "edits"))
    if digest(body) != patch["source"]:
        raise Refusal("prepared source differs from its exact blob")
    if not isinstance(patch["edits"], list):
        raise Refusal("prepared edits must be an ordered list")
    parts = []
    offset = 0
    for edit in patch["edits"]:
        if not isinstance(edit, list) or len(edit) != 3:
            raise Refusal("invalid prepared byte edit")
        start, end, replacement = edit
        if (type(start) is not int or type(end) is not int or
                not offset <= start < end <= len(body) or not isinstance(replacement, str)):
            raise Refusal("prepared byte edit is overlapping or outside its source")
        parts.extend((body[offset:start], replacement.encode("utf-8")))
        offset = end
    parts.append(body[offset:])
    result = b"".join(parts)
    if digest(result) != patch["result"]:
        raise Refusal("prepared bytes differ from their declared result")
    return result
