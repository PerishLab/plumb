import os
import sys

from lib.acceptance import commit
from lib.blob import Refusal, sha


def select(source, declaration, marker, plumb, repository):
    sha(declaration)
    if source:
        commit(source)
        if marker or plumb or repository:
            raise Refusal("source acceptance cannot carry Ship publication inputs")
        return "source"
    if not all((marker, plumb, repository)):
        raise Refusal("ordinary Ship requires its marker, controller and product repository")
    return "ship"


if __name__ == "__main__":
    try:
        mode = select(*(os.environ.get("PLUMB_INPUT_" + name, "") for name in
                        ("SOURCE", "DECLARATION", "MARKER", "PLUMB", "REPOSITORY")))
        with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
            output.write("mode=" + mode + "\n")
    except (Refusal, OSError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
