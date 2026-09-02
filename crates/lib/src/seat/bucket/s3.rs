use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod probe;

const REGION: &str = "auto";
const SERVICE: &str = "s3";
const ATTEMPTS: usize = 3;

pub struct Control {
    access: String,
    secret: String,
    bucket: String,
    endpoint: String,
    host: String,
}

pub struct Object {
    pub body: Vec<u8>,
    pub etag: Option<String>,
}

pub struct Policy<'a> {
    pub media: &'a str,
    pub cache: &'a str,
}

pub enum Condition<'a> {
    Absent,
    Match(&'a str),
}

pub enum Outcome<T> {
    Held(T),
    Missing,
    Stale,
}

impl Control {
    pub fn new(
        access: String,
        secret: String,
        bucket: String,
        endpoint: String,
    ) -> Result<Self, String> {
        let endpoint = endpoint.trim_end_matches('/').to_string();
        let (_, host) = endpoint
            .split_once("://")
            .ok_or_else(|| "bucket endpoint has no scheme".to_string())?;
        if host.is_empty() || host.contains(['/', '?', '#']) {
            return Err("bucket endpoint must name one origin".to_string());
        }
        if access.is_empty() || secret.is_empty() || bucket.is_empty() {
            return Err("bucket authority is incomplete".to_string());
        }
        let host = host.to_ascii_lowercase();
        Ok(Self {
            access,
            secret,
            bucket,
            endpoint,
            host,
        })
    }

    pub fn read(&self, key: &str) -> Result<Outcome<Object>, String> {
        for attempt in 0..ATTEMPTS {
            pause(attempt);
            let signed = self.sign("GET", key, &[], &[])?;
            match self.get(signed) {
                Ok(outcome) => return Ok(outcome),
                Err(error) if retryable(&error) && attempt + 1 < ATTEMPTS => continue,
                Err(error) => return Err(format!("cannot read bucket object {key}: {error}")),
            }
        }
        unreachable!()
    }

    fn get(&self, signed: Signed) -> Result<Outcome<Object>, ureq::Error> {
        match ureq::get(&signed.url)
            .header("host", &self.host)
            .header("x-amz-content-sha256", signed.payload)
            .header("x-amz-date", signed.time)
            .header("authorization", signed.authorization)
            .call()
        {
            Ok(response) => {
                let etag = response
                    .headers()
                    .get("etag")
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_string);
                let body = response.into_body().read_to_vec()?;
                Ok(Outcome::Held(Object { body, etag }))
            }
            Err(ureq::Error::StatusCode(404)) => Ok(Outcome::Missing),
            Err(error) => Err(error),
        }
    }

    pub fn write(
        &self,
        key: &str,
        body: &[u8],
        policy: Policy<'_>,
        condition: Condition<'_>,
    ) -> Result<Outcome<()>, String> {
        let (name, value) = match condition {
            Condition::Absent => ("if-none-match", "*"),
            Condition::Match(etag) => ("if-match", etag),
        };
        let fields = [
            ("cache-control", policy.cache),
            ("content-type", policy.media),
            (name, value),
        ];
        for attempt in 0..ATTEMPTS {
            pause(attempt);
            let signed = self.sign("PUT", key, body, &fields)?;
            let request = ureq::put(&signed.url)
                .header("host", &self.host)
                .header("x-amz-content-sha256", signed.payload)
                .header("x-amz-date", signed.time)
                .header("authorization", signed.authorization)
                .header("content-type", policy.media)
                .header("cache-control", policy.cache)
                .header(name, value);
            match request.send(body) {
                Ok(_) => return Ok(Outcome::Held(())),
                Err(ureq::Error::StatusCode(412)) => return Ok(Outcome::Stale),
                Err(error) if retryable(&error) && attempt + 1 < ATTEMPTS => continue,
                Err(error) => return Err(format!("cannot write bucket object {key}: {error}")),
            }
        }
        unreachable!()
    }

    fn sign(
        &self,
        method: &str,
        key: &str,
        body: &[u8],
        extra: &[(&str, &str)],
    ) -> Result<Signed, String> {
        let path = format!("/{}/{}", encode(&self.bucket), encode(key));
        let url = format!("{}{}", self.endpoint, path);
        let stamp = Stamp::now()?;
        let payload = hex(&Sha256::digest(body));
        let mut fields = BTreeMap::new();
        for (name, value) in extra {
            fields.insert(*name, value.trim());
        }
        fields.insert("host", &self.host);
        fields.insert("x-amz-content-sha256", &payload);
        fields.insert("x-amz-date", &stamp.time);
        let headers = fields
            .iter()
            .map(|(name, value)| format!("{name}:{value}\n"))
            .collect::<String>();
        let names = fields.keys().copied().collect::<Vec<_>>().join(";");
        let canonical = format!("{method}\n{path}\n\n{headers}\n{names}\n{payload}");
        let scope = format!("{}/{REGION}/{SERVICE}/aws4_request", stamp.date);
        let wanted = format!(
            "AWS4-HMAC-SHA256\n{}\n{scope}\n{}",
            stamp.time,
            hex(&Sha256::digest(canonical.as_bytes()))
        );
        let date = hmac(
            format!("AWS4{}", self.secret).as_bytes(),
            stamp.date.as_bytes(),
        );
        let region = hmac(&date, REGION.as_bytes());
        let service = hmac(&region, SERVICE.as_bytes());
        let signing = hmac(&service, b"aws4_request");
        let signature = hex(&hmac(&signing, wanted.as_bytes()));
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={names}, Signature={signature}",
            self.access
        );
        Ok(Signed {
            url,
            payload,
            time: stamp.time,
            authorization,
        })
    }
}

fn retryable(error: &ureq::Error) -> bool {
    match error {
        ureq::Error::StatusCode(code) => matches!(code, 408 | 429 | 500..=599),
        ureq::Error::Io(_)
        | ureq::Error::Protocol(_)
        | ureq::Error::Timeout(_)
        | ureq::Error::HostNotFound
        | ureq::Error::ConnectionFailed => true,
        _ => false,
    }
}

fn pause(attempt: usize) {
    if attempt > 0 {
        std::thread::sleep(Duration::from_millis(200 * attempt as u64));
    }
}

struct Signed {
    url: String,
    payload: String,
    time: String,
    authorization: String,
}

struct Stamp {
    date: String,
    time: String,
}

impl Stamp {
    fn now() -> Result<Self, String> {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("cannot read system time: {error}"))?
            .as_secs() as i64;
        let days = seconds / 86_400;
        let clock = seconds % 86_400;
        let (year, month, day) = civil(days);
        let hour = clock / 3_600;
        let minute = clock % 3_600 / 60;
        let second = clock % 60;
        let date = format!("{year:04}{month:02}{day:02}");
        Ok(Self {
            time: format!("{date}T{hour:02}{minute:02}{second:02}Z"),
            date,
        })
    }
}

fn civil(days: i64) -> (i64, i64, i64) {
    let held = days + 719_468;
    let era = if held >= 0 { held } else { held - 146_096 } / 146_097;
    let doe = held - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

fn encode(value: &str) -> String {
    let mut held = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~' | b'/') {
            held.push(char::from(byte));
        } else {
            held.push_str(&format!("%{byte:02X}"));
        }
    }
    held
}

fn hmac(key: &[u8], value: &[u8]) -> [u8; 32] {
    let mut pad = [0u8; 64];
    if key.len() > pad.len() {
        pad[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        pad[..key.len()].copy_from_slice(key);
    }
    let mut outer = [0x5c; 64];
    let mut inner = [0x36; 64];
    for index in 0..64 {
        outer[index] ^= pad[index];
        inner[index] ^= pad[index];
    }
    let mut sponge = Sha256::new();
    sponge.update(inner);
    sponge.update(value);
    let digest = sponge.finalize();
    let mut sponge = Sha256::new();
    sponge.update(outer);
    sponge.update(digest);
    sponge.finalize().into()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
