// oruvelle: small Rust port of the Alexa control calls.
// Modes:  oruvelle                         -> MCP server over stdio
//         oruvelle notify <msg> [colour] [light-or-room] [blink]
// Needs session.json, written by `node session.js` in the parent folder.

use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::thread::sleep;
use std::time::Duration;

const LOCALE: &str = "en-US";

struct Session {
    cookie: String,
    csrf: String,
    host: String,
    session: Value,
}

fn load() -> Result<Session, String> {
    let path = std::env::var("ORUVELLE_SESSION").unwrap_or_else(|_| {
        let exe = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf()));
        // target/release/oruvelle.exe lives three folders below the project root
        exe.map(|d| d.join("../../../session.json").to_string_lossy().into_owned())
            .unwrap_or_else(|| "session.json".into())
    });
    let text = std::fs::read_to_string(&path).map_err(|e| format!("cannot read {path}: {e}. Run `node session.js` first."))?;
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(Session {
        cookie: v["cookie"].as_str().unwrap_or_default().into(),
        csrf: v["csrf"].as_str().unwrap_or_default().into(),
        host: v["host"].as_str().unwrap_or("alexa.amazon.com").into(),
        session: v,
    })
}

// Call Amazon's Alexa web API with the same headers alexa-remote2 sends.
fn api(s: &Session, method: &str, path: &str, body: Option<Value>) -> Result<Value, String> {
    let url = format!("https://{}{}", s.host, path);
    let req = ureq::request(method, &url)
        .timeout(Duration::from_secs(20))
        .set("Cookie", &s.cookie)
        .set("csrf", &s.csrf)
        .set("Accept", "application/json; charset=utf-8")
        .set("Content-Type", "application/json; charset=utf-8")
        .set("Accept-Language", LOCALE)
        .set("Origin", &format!("https://{}", s.host))
        .set("Referer", &format!("https://{}/spa/index.html", s.host))
        .set("User-Agent", "AppleWebKit PitanguiBridge/2.2.595606.0-[HARDWARE=iPhone14_7][SOFTWARE=17.4.1][DEVICE=iPhone]");
    let res = match body {
        Some(b) => req.send_string(&b.to_string()),
        None => req.call(),
    };
    match res {
        Ok(r) => Ok(r.into_string().ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(json!({"success": true}))),
        Err(ureq::Error::Status(401, _)) => Err("session expired: run `node session.js` in C:\\Alexa".into()),
        Err(ureq::Error::Status(code, r)) => Err(format!("HTTP {code}: {}", r.into_string().unwrap_or_default())),
        Err(e) => Err(e.to_string()),
    }
}

fn device<'a>(s: &'a Session, name: Option<&str>) -> Result<(&'a str, &'a Value), String> {
    let default = std::env::var("ALEXA_DEVICE").unwrap_or_else(|_| "Vishal's Echo Show 5 2nd Gen".into());
    let wanted = name.map(str::to_string).unwrap_or(default);
    let (k, v) = s.session["devices"]
        .as_object()
        .and_then(|m| m.iter().find(|(k, _)| k.eq_ignore_ascii_case(&wanted)))
        .ok_or_else(|| format!("no Echo named \"{wanted}\""))?;
    Ok((k.as_str(), v))
}

// Build a one-step sequence and send it to a device, like sendSequenceCommand.
fn send_node(s: &Session, dev: Option<&str>, ty: &str, payload: Value, skill: Option<&str>) -> Result<Value, String> {
    let (_, d) = device(s, dev)?;
    let mut node = json!({
        "@type": "com.amazon.alexa.behaviors.model.OpaquePayloadOperationNode",
        "type": ty,
        "operationPayload": merge(json!({
            "deviceType": d["deviceType"], "deviceSerialNumber": d["serialNumber"],
            "locale": LOCALE, "customerId": d["deviceOwnerCustomerId"],
        }), payload),
    });
    if let Some(id) = skill {
        node["skillId"] = json!(id);
    }
    let seq = json!({"@type": "com.amazon.alexa.behaviors.model.Sequence", "startNode": node});
    send_sequence(s, &seq, "PREVIEW")
}

fn merge(mut a: Value, b: Value) -> Value {
    if let (Some(am), Some(bm)) = (a.as_object_mut(), b.as_object()) {
        for (k, v) in bm {
            am.insert(k.clone(), v.clone());
        }
    }
    a
}

fn send_sequence(s: &Session, seq: &Value, behavior: &str) -> Result<Value, String> {
    let body = json!({"behaviorId": behavior, "sequenceJson": seq.to_string(), "status": "ENABLED"});
    api(s, "POST", "/api/behaviors/preview", Some(body))
}

fn speak(s: &Session, dev: Option<&str>, text: &str) -> Result<Value, String> {
    send_node(s, dev, "Alexa.Speak", json!({"textToSpeak": text}), None)
}

fn text_command(s: &Session, dev: Option<&str>, text: &str) -> Result<Value, String> {
    send_node(s, dev, "Alexa.TextCommand", json!({"text": text.to_lowercase()}), Some("amzn1.ask.1p.tellalexa"))
}

fn volume(s: &Session, dev: Option<&str>, level: i64) -> Result<Value, String> {
    if !(0..=100).contains(&level) {
        return Err("volume must be between 0 and 100".into());
    }
    send_node(s, dev, "Alexa.DeviceControls.Volume", json!({"value": level}), None)
}

fn routine(s: &Session, name: &str, dev: Option<&str>) -> Result<Value, String> {
    let (_, d) = device(s, dev)?;
    let hit = s.session["routines"]
        .as_array()
        .and_then(|rs| {
            let lower = name.to_lowercase();
            rs.iter()
                .find(|r| r["name"].as_str().map(|n| n.to_lowercase()) == Some(lower.clone()))
                .or_else(|| rs.iter().find(|r| r["name"].as_str().map(|n| n.to_lowercase().contains(&lower)).unwrap_or(false)))
        })
        .ok_or_else(|| format!("no routine matches \"{name}\""))?;
    // Fill the same placeholders alexa-remote2 replaces for the chosen device.
    let text = hit["sequence"]
        .to_string()
        .replace("\"deviceType\":\"ALEXA_CURRENT_DEVICE_TYPE\"", &format!("\"deviceType\":\"{}\"", d["deviceType"].as_str().unwrap_or("")))
        .replace("\"deviceSerialNumber\":\"ALEXA_CURRENT_DSN\"", &format!("\"deviceSerialNumber\":\"{}\"", d["serialNumber"].as_str().unwrap_or("")))
        .replace("\"customerId\":\"ALEXA_CUSTOMER_ID\"", &format!("\"customerId\":\"{}\"", d["deviceOwnerCustomerId"].as_str().unwrap_or("")))
        .replace("\"locale\":\"ALEXA_CURRENT_LOCALE\"", &format!("\"locale\":\"{LOCALE}\""));
    let seq: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let behavior = if seq.get("sequenceId").is_some() { hit["automationId"].as_str().unwrap_or("PREVIEW") } else { "PREVIEW" };
    send_sequence(s, &seq, behavior)
}

// Light ids for a light name or room group name.
fn light_ids(s: &Session, target: &str) -> Result<Vec<String>, String> {
    let t = target.to_lowercase();
    let lights = s.session["lights"].as_array().cloned().unwrap_or_default();
    let group: Vec<String> = s.session["groups"]
        .as_object()
        .and_then(|g| g.iter().find(|(k, _)| k.to_lowercase() == t))
        .and_then(|(_, v)| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let ids: Vec<String> = lights
        .iter()
        .filter_map(|l| {
            let id = l["applianceId"].as_str()?;
            let by_name = l["name"].as_str()?.to_lowercase() == t;
            (by_name || group.iter().any(|g| g == id)).then(|| id.to_string())
        })
        .collect();
    if ids.is_empty() {
        return Err(format!("no lights match \"{target}\""));
    }
    Ok(ids)
}

fn light(s: &Session, target: &str, parameters: Value) -> Result<Value, String> {
    let ids = light_ids(s, target)?;
    let controls: Vec<Value> = ids.iter().map(|id| json!({"entityId": id, "entityType": "APPLIANCE", "parameters": parameters})).collect();
    let res = api(s, "PUT", "/api/phoenix/state", Some(json!({"controlRequests": controls})))?;
    if let Some(errs) = res["errors"].as_array().filter(|e| !e.is_empty()) {
        return Err(format!("Alexa did not apply the change: {}", Value::Array(errs.clone())));
    }
    Ok(res)
}

fn blink(s: &Session, target: &str, colour: &str, times: u32) -> Result<Value, String> {
    for _ in 0..times {
        light(s, target, json!({"action": "setColor", "colorName": colour}))?;
        sleep(Duration::from_millis(500));
        light(s, target, json!({"action": "turnOff"}))?;
        sleep(Duration::from_millis(500));
    }
    light(s, target, json!({"action": "setColor", "colorName": colour}))
}

// ---- MCP over stdio (JSON-RPC 2.0, one message per line) ----

fn tools() -> Value {
    let dev = json!({"type": "string", "description": "Echo name. Defaults to the main Echo."});
    json!([
        {"name": "alexa_devices", "description": "List Echo devices on the account.", "inputSchema": {"type": "object", "properties": {}}},
        {"name": "alexa_speak", "description": "Make an Echo say text out loud.", "inputSchema": {"type": "object", "properties": {"device": dev, "text": {"type": "string"}}, "required": ["text"]}},
        {"name": "alexa_command", "description": "Send any command to Alexa as if spoken: music, timers, reminders, lists, questions.", "inputSchema": {"type": "object", "properties": {"device": dev, "text": {"type": "string"}}, "required": ["text"]}},
        {"name": "alexa_volume", "description": "Set an Echo speaker volume, 0-100.", "inputSchema": {"type": "object", "properties": {"device": dev, "level": {"type": "integer", "minimum": 0, "maximum": 100}}, "required": ["level"]}},
        {"name": "alexa_lights_list", "description": "List smart lights by name.", "inputSchema": {"type": "object", "properties": {}}},
        {"name": "alexa_light", "description": "Turn a light or room group on/off, set brightness 0-100, or set a colour.", "inputSchema": {"type": "object", "properties": {"target": {"type": "string"}, "action": {"type": "string", "enum": ["turnOn", "turnOff", "setBrightness", "setColor"]}, "brightness": {"type": "integer", "minimum": 0, "maximum": 100}, "color": {"type": "string"}}, "required": ["target", "action"]}},
        {"name": "alexa_blink", "description": "Blink a light or room in a colour, then leave it on.", "inputSchema": {"type": "object", "properties": {"target": {"type": "string"}, "color": {"type": "string"}, "times": {"type": "integer", "minimum": 1, "maximum": 20}}, "required": ["target", "color"]}},
        {"name": "alexa_routines_list", "description": "List Alexa routines by name.", "inputSchema": {"type": "object", "properties": {}}},
        {"name": "alexa_routine", "description": "Run an Alexa routine by name (partial match is fine).", "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}, "device": dev}, "required": ["name"]}},
        {"name": "alexa_notify", "description": "Signal an IDE event: set or blink a light colour and speak a message.", "inputSchema": {"type": "object", "properties": {"target": {"type": "string"}, "color": {"type": "string"}, "blink": {"type": "boolean"}, "message": {"type": "string"}, "device": dev}}}
    ])
}

fn call_tool(s: &Session, name: &str, a: &Value) -> Result<Value, String> {
    let dev = a["device"].as_str();
    let str_arg = |k: &str| a[k].as_str().unwrap_or_default();
    match name {
        "alexa_devices" => Ok(json!(s.session["devices"].as_object().map(|m| m.keys().cloned().collect::<Vec<_>>()).unwrap_or_default())),
        "alexa_speak" => speak(s, dev, str_arg("text")),
        "alexa_command" => text_command(s, dev, str_arg("text")),
        "alexa_volume" => volume(s, dev, a["level"].as_i64().unwrap_or(-1)),
        "alexa_lights_list" => Ok(json!(s.session["lights"].as_array().map(|l| l.iter().filter_map(|x| x["name"].as_str().map(String::from)).collect::<Vec<_>>()).unwrap_or_default())),
        "alexa_light" => {
            let p = match str_arg("action") {
                "setBrightness" => json!({"action": "setBrightness", "brightness": a["brightness"]}),
                "setColor" => json!({"action": "setColor", "colorName": a["color"]}),
                other => json!({"action": other}),
            };
            light(s, str_arg("target"), p)
        }
        "alexa_blink" => blink(s, str_arg("target"), str_arg("color"), a["times"].as_u64().unwrap_or(3) as u32),
        "alexa_routines_list" => Ok(json!(s.session["routines"].as_array().map(|r| r.iter().filter_map(|x| x["name"].as_str().map(String::from)).collect::<Vec<_>>()).unwrap_or_default())),
        "alexa_routine" => routine(s, str_arg("name"), dev),
        "alexa_notify" => {
            let mut out = json!({});
            if let (Some(t), Some(c)) = (a["target"].as_str(), a["color"].as_str()) {
                out["lights"] = if a["blink"].as_bool().unwrap_or(false) { blink(s, t, c, 3)? } else { light(s, t, json!({"action": "setColor", "colorName": c}))? };
            }
            if let Some(m) = a["message"].as_str() {
                out["speech"] = speak(s, dev, m)?;
            }
            Ok(out)
        }
        _ => Err(format!("unknown tool {name}")),
    }
}

fn reply(out: &mut impl Write, id: &Value, result: Value) {
    let _ = writeln!(out, "{}", json!({"jsonrpc": "2.0", "id": id, "result": result}));
    let _ = out.flush();
}

fn serve() {
    let stdin = io::stdin();
    let mut out = io::stdout();
    for line in stdin.lock().lines().map_while(Result::ok) {
        let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
        let id = msg.get("id").cloned();
        let Some(id) = id else { continue }; // notifications need no reply
        match msg["method"].as_str().unwrap_or("") {
            "initialize" => reply(&mut out, &id, json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "oruvelle", "version": "0.1.0"}
            })),
            "tools/list" => reply(&mut out, &id, json!({"tools": tools()})),
            "tools/call" => {
                let name = msg["params"]["name"].as_str().unwrap_or("").to_string();
                let args = msg["params"]["arguments"].clone();
                let result = match load().and_then(|s| call_tool(&s, &name, &args)) {
                    Ok(v) => json!({"content": [{"type": "text", "text": v.to_string()}]}),
                    Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
                };
                reply(&mut out, &id, result);
            }
            _ => {
                let _ = writeln!(out, "{}", json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": "method not found"}}));
                let _ = out.flush();
            }
        }
    }
}

fn notify(args: &[String]) -> Result<(), String> {
    let s = load()?;
    let msg = args.first().map(String::as_str).unwrap_or("");
    let colour = args.get(1).map(String::as_str).unwrap_or("green");
    let target = args.get(2).map(String::as_str).unwrap_or("Lamp 1");
    if args.get(3).is_some() {
        blink(&s, target, colour, 3)?;
    } else {
        light(&s, target, json!({"action": "setColor", "colorName": colour}))?;
    }
    if !msg.is_empty() {
        speak(&s, None, msg)?;
    }
    println!("ok");
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("notify") {
        if let Err(e) = notify(&args[1..]) {
            eprintln!("{e}");
            std::process::exit(1);
        }
    } else {
        serve();
    }
}
