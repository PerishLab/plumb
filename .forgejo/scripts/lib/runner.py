import os
import tempfile
from pathlib import Path

from .blob import Refusal, decode, encode
from .completion import complete


class Runner:
    def __init__(self, execution, state):
        self.execution = execution
        self.state = Path(state).resolve()

    def run(self, identity, invoke, preparation=None):
        prepared = self.execution.prepare(identity)
        if prepared["state"] == "prepare":
            if preparation is None:
                raise Refusal("execution preparation has no declared local executor")
            for dependency in prepared["nodes"]:
                self.run(dependency, preparation(dependency), preparation)
            prepared = self.execution.prepare(identity)
            if prepared["state"] == "prepare":
                raise Refusal("execution preparation did not complete")
        if prepared["state"] == "reuse":
            return prepared
        request = prepared["request"]
        seat = self.state / request["key"]
        seat.mkdir(parents=True, exist_ok=True)
        receipt = seat / "result.json"
        if receipt.exists():
            result = decode(receipt.read_bytes())
        else:
            result = invoke(request, seat)
            if not isinstance(result, dict) or result.get("key") != request["key"]:
                raise Refusal("executor result differs from the executed key")
            self.retain(receipt, result)
        held = complete(self.execution.graph, identity, result, self.execution.inventory)
        return {"state": "complete", "completion": held}

    def retain(self, receipt, result):
        with tempfile.NamedTemporaryFile(dir=receipt.parent, delete=False) as temporary:
            path = Path(temporary.name)
            try:
                temporary.write(encode(result))
                temporary.flush()
                os.fsync(temporary.fileno())
            except BaseException:
                path.unlink(missing_ok=True)
                raise
        try:
            path.replace(receipt)
        finally:
            path.unlink(missing_ok=True)
