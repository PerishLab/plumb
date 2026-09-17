from .blob import Refusal, name, object_fields


class Backend:
    def __init__(self, value):
        object_fields(value, ("schema", "jobs", "placement"))
        if value["schema"] != "plumb.blob-backend/v1" or not isinstance(value["jobs"], list):
            raise Refusal("unsupported execution backend")
        self.jobs = {}
        self.ancestors = {}
        for job in value["jobs"]:
            object_fields(job, ("id", "needs", "capabilities"), ("runners",))
            identity = name(job["id"])
            if identity in self.jobs or not isinstance(job["needs"], list):
                raise Refusal("duplicate job or invalid needs")
            if not isinstance(job["capabilities"], list) or not job["capabilities"]:
                raise Refusal("job needs explicit capabilities")
            for capability in job["capabilities"]:
                name(capability)
            runners = job.get("runners", {})
            if not isinstance(runners, dict) or set(runners) - set(job["capabilities"]):
                raise Refusal("runner mapping exceeds declared capabilities")
            for runner in runners.values():
                name(runner)
            parents = set()
            for parent in job["needs"]:
                name(parent)
                if parent not in self.jobs:
                    raise Refusal("backend jobs must list dependencies before consumers")
                parents.add(parent)
                parents.update(self.ancestors[parent])
            self.jobs[identity] = job
            self.ancestors[identity] = parents
        if not isinstance(value["placement"], dict):
            raise Refusal("backend requires explicit entry placement")
        self.placement = value["placement"]
        for entry, job in self.placement.items():
            name(entry)
            name(job)
            if job not in self.jobs:
                raise Refusal("entry placement names an unknown job")

    def place(self, graph):
        placed = {}
        for identity in graph.order:
            node = graph.nodes[identity]
            chosen = self.placement.get(node["execution"]["entry"])
            if chosen is None or node["execution"]["capability"] not in self.jobs[chosen]["capabilities"]:
                raise Refusal("entry has no compatible declared workflow job")
            preparation = node.get("prepare", [])
            content = [ref for ref in node["inputs"].values() if isinstance(ref, dict)]
            predecessors = {placed[ref["node"]] for ref in content + node.get("requires", [])}
            if not predecessors <= self.ancestors[chosen]:
                raise Refusal("declared graph exceeds the finite workflow job skeleton")
            for ref in preparation:
                if placed[ref["node"]] != chosen:
                    raise Refusal("execution preparation must share its consumer workflow job")
                producer = graph.nodes[ref["node"]]
                runners = self.jobs[chosen].get("runners", {})
                if runners.get(producer["execution"]["capability"]) != runners.get(node["execution"]["capability"]):
                    raise Refusal("local preparation requires the same runner as its consumer")
            placed[identity] = chosen
        return placed

    def project(self, graph, plan):
        placed = self.place(graph)
        output = {}
        for identity, job in self.jobs.items():
            nodes = [node for node, seat in placed.items() if seat == identity]
            runnable = [node for node in nodes if plan["nodes"][node]["state"] == "run"]
            deferred = [node for node in nodes if plan["nodes"][node]["state"] == "pending"]
            prerequisites = {placed[ref["node"]] for node in nodes
                             for ref in graph.references(graph.nodes[node])}
            barriers = sorted(set(job["needs"]) - prerequisites)
            output[identity] = {"run": runnable, "pending": deferred,
                                "needed": bool(runnable or deferred),
                                "backend_barriers": barriers}
        return {"schema": "plumb.blob-jobs/v1", "jobs": output, "placement": placed}

    def matrix(self, graph, plan, identity):
        if identity not in self.jobs:
            raise Refusal("unknown workflow job")
        placed = self.place(graph)
        local = {ref["node"] for node in graph.nodes.values() for ref in node.get("prepare", [])}
        include = []
        for node, job in placed.items():
            if node in local and node not in graph.targets:
                continue
            if job != identity or plan["nodes"][node]["state"] not in ("run", "pending"):
                continue
            capability = graph.nodes[node]["execution"]["capability"]
            runner = self.jobs[job].get("runners", {}).get(capability)
            if runner is None:
                raise Refusal("workflow capability has no explicit runner mapping")
            include.append({"node": node, "runner": runner})
        return {"include": include}
