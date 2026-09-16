import fnmatch
import os

from .blob import Refusal, object_fields


def environment(contract, managed):
    object_fields(contract, ("inherit", "managed", "reject"))
    for patterns in contract.values():
        if not isinstance(patterns, list) or any(not isinstance(pattern, str) for pattern in patterns):
            raise Refusal("execution environment requires explicit pattern lists")

    def matches(name, patterns):
        return any(fnmatch.fnmatchcase(name.upper() if os.name == "nt" else name,
                                      pattern.upper() if os.name == "nt" else pattern)
                   for pattern in patterns)

    inherited = {}
    for name, value in os.environ.items():
        if name in managed or matches(name, contract["managed"]):
            continue
        if matches(name, contract["inherit"]):
            inherited[name] = value
        elif matches(name, contract["reject"]):
            raise Refusal("undeclared execution environment variable: " + name)
    return {**inherited, **managed}
