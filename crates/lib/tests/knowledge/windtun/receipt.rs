use super::{
    binary::Binary,
    material::Codec,
    world::{Fixture, GH, ROOT},
};
use serde_json::{Value, json};

impl Fixture {
    pub fn receipt(&mut self) {
        let observer = json!({"run":"10","attempt":"1","wharf":"d".repeat(40)});
        let material =
            json!({"key":"a".repeat(64),"record":"b".repeat(64),"digest":"c".repeat(64)});
        let source = Binary::source("keel", "v1.0.0-rc.1");
        let intent = json!({"schema":"wharf.windtun.intent/v1","source":source,"observer":observer,"started":"2026-10-01T00:00:00Z","scope":"declared","policy":"wharf.windtun.linux/v1","preparation":{"key":"a".repeat(64),"basis":"b".repeat(64),"record":"c".repeat(64)}});
        let path = format!("{ROOT}/attempts/10-1");
        self.set(&format!("{path}/intent.json"), &intent);
        self.set(&format!("{ROOT}/latest.json"),&json!({"schema":"wharf.windtun.index/v1","observer":observer,"started":intent["started"],"intent":Codec::reference(&format!("{path}/intent.json"),&Codec::bytes(&intent))}));
        let distribution = self
            .get("https://releases.keel.perish.uk/v1/releases/rc/v1.0.0-rc.1/distribution.json");
        let runner = &self.pins["runner"]["pointer"];
        let product = &self.pins["product"];
        self.pass = json!({"schema":"wharf.windtun.pass/v1","policy":"wharf.windtun.linux/v1","implementation":"c".repeat(64),"observer":observer,"source":source,"publisher":distribution["completed"],"distribution":Codec::digest(&distribution),"declaration":self.declaration,"requirements":{"platform":"linux-x86_64","declared":["health"],"selected":["health"],"scope":"declared"},"prepared":"f".repeat(64),"runner":{"pointer":Codec::digest(runner),"seal":runner["seal"],"sha256":Codec::sha(&Binary::image(&Binary::source("seatrial","v1.0.0"))),"version":"seatrial v1.0.0"},"scene":self.pins["scene"],"groups":[{"digest":"f".repeat(64),"cases":["health"]}],"products":[{"name":"keel","channel":"rc","version":"v1.0.0-rc.1","commit":"1".repeat(40),"seal":Codec::digest(&product["seal"]),"archive":product["seal"]["artifacts"]["linux-x64"]["sha256"]}],"images":[{"name":"keel","reference":"ghcr.io/perishlab/keel:1.0.0-rc.1","digest":format!("ghcr.io/perishlab/keel@sha256:{}","b".repeat(64))}],"controls":{"probe":format!("docker.io/curlimages/curl:8.10.1@sha256:{}","f".repeat(64))},"cleanup":true,"evidence":material,"intent":Codec::digest(&intent)});
        self.result = json!({"schema":"wharf.windtun.result/v1","source":source,"observer":observer,"intent":Codec::digest(&intent),"state":"passed","ok":true,"evidence":material,"credential":null});
        self.publish();
        self.native();
    }

    pub fn publish(&mut self) {
        let path = format!("{ROOT}/attempts/10-1");
        let passed = self.pass.clone();
        self.result["credential"] =
            Codec::reference(&format!("{path}/pass.json"), &Codec::bytes(&passed));
        self.set(&format!("{path}/pass.json"), &passed);
        self.set(&format!("{path}/result.json"), &self.result.clone());
    }

    fn native(&mut self) {
        let root = format!("{GH}/actions/runs/10/attempts/1");
        let repository = json!({"id":1374358737_u64,"full_name":"PerishLab/wharf"});
        self.set(&root,&json!({"id":10,"run_attempt":1,"head_sha":"d".repeat(40),"head_branch":"main","event":"workflow_dispatch","path":".github/workflows/ship.yml","status":"completed","conclusion":"failure","repository":repository,"head_repository":repository}));
        let roles = [
            (
                "[windtun] prepare attempt",
                vec!["[windtun] admit publisher", "[windtun] prepare attempt"],
            ),
            (
                "[windtun] published artifacts",
                vec![
                    "[windtun] load attempt",
                    "[windtun] published artifacts",
                    "[windtun] judge observation",
                    "[windtun] record attempt",
                ],
            ),
            (
                "[windtun] issue qualification",
                vec!["[windtun] issue qualification"],
            ),
        ];
        let jobs:Vec<Value>=roles.into_iter().enumerate().map(|(index,(name,steps))|json!({"id":index+1,"name":name,"run_id":10,"run_attempt":1,"head_sha":"d".repeat(40),"head_branch":"main","status":"completed","conclusion":"success","started_at":"2026-10-01T00:00:00Z","steps":steps.into_iter().map(|name|json!({"name":name,"status":"completed","conclusion":"success"})).collect::<Vec<_>>()})).collect();
        self.set(
            &format!("{root}/jobs?per_page=100"),
            &json!({"total_count":jobs.len(),"jobs":jobs}),
        );
    }
}
