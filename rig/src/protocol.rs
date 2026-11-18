//! The JSON-lines protocol of tools/rig/CONTRACT.md, engine side: the resolver's `new`, `step` with or
//! without `force`, `options`, `coverage` and `release`. A request's `engine` field, if any, must be "sim".

use crate::json::Json;
use crate::newsim::{event_json, SimBattle};
use std::collections::HashMap;

#[derive(Default)]
pub struct Rig {
    sims: HashMap<String, SimBattle>,
    /// the new resolver records the battle log's rows and replies carry them (`rig --rows`)
    pub rows: bool,
}

fn fail(msg: impl Into<String>) -> Json {
    let msg = msg.into();
    let mut o = Json::obj();
    o.set("ok", false);
    if let Some(detail) = msg.strip_prefix("tape:") {
        o.set("error", "tape");
        o.set("detail", detail);
    } else {
        o.set("error", msg);
    }
    o
}

impl Rig {
    pub fn handle_line(&mut self, line: &str) -> String {
        let reply = match Json::parse(line) {
            Ok(req) => self.handle(&req),
            Err(e) => fail(e),
        };
        reply.to_string()
    }

    pub fn handle(&mut self, req: &Json) -> Json {
        let op = req.get("op").and_then(Json::as_str).unwrap_or("");
        let id = req.get("id").and_then(Json::as_str).unwrap_or("").to_string();
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.dispatch(op, &id, req)));
        match res {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => fail(e),
            Err(p) => {
                let msg = p
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "panic".into());
                fail(format!("engine panic: {}", msg))
            }
        }
    }

    fn dispatch(&mut self, op: &str, id: &str, req: &Json) -> Result<Json, String> {
        match req.get("engine").and_then(Json::as_str) {
            None | Some("sim") => self.dispatch_sim(op, id, req),
            Some(e) => Err(format!("unknown engine {:?}", e)),
        }
    }

    fn dispatch_sim(&mut self, op: &str, id: &str, req: &Json) -> Result<Json, String> {
        let reply = |b: &SimBattle, events: Option<Vec<cg_engine::sim::outcome::Event>>| {
            let mut o = Json::obj();
            o.set("ok", true);
            if let Some(ev) = events {
                o.set("events", Json::Arr(ev.iter().map(event_json).collect()));
                o.set("log", Json::Arr(vec![]));
            }
            o.set("state", b.snapshot());
            o.set("options", b.options());
            o.set("unimplemented", b.unimplemented());
            if b.log_rows {
                o.set("rows", b.rows_json());
            }
            o
        };
        match op {
            "new" => {
                if self.sims.contains_key(id) {
                    return Err(format!("battle {} exists", id));
                }
                let (b, events) = match SimBattle::new(req, self.rows) {
                    Ok(x) => x,
                    Err(e) => {
                        let mut o = fail(e.msg);
                        o.set("unimplemented", Json::Arr(e.unimplemented.into_iter().map(Json::Str).collect()));
                        return Ok(o);
                    }
                };
                let o = reply(&b, Some(events));
                self.sims.insert(id.to_string(), b);
                Ok(o)
            }
            "step" => {
                let b = self.sims.get_mut(id).ok_or_else(|| format!("no battle {}", id))?;
                match b.step(req) {
                    Ok(events) => {
                        let mut o = reply(b, Some(events));
                        if !b.refused.is_empty() {
                            let mut r = Json::obj();
                            for (side, why) in &b.refused {
                                r.set(crate::newsim::SIDE_IDS[*side], *why);
                            }
                            o.set("refused", r);
                        }
                        Ok(o)
                    }
                    Err(e) => {
                        let mut o = fail(e);
                        o.set("unimplemented", Json::Arr(b.attempt_unimpl.iter().cloned().map(Json::Str).collect()));
                        o.set("events", Json::Arr(b.attempt_events.iter().map(event_json).collect()));
                        Ok(o)
                    }
                }
            }
            "options" => {
                let b = self.sims.get(id).ok_or_else(|| format!("no battle {}", id))?;
                Ok(reply(b, None))
            }
            "coverage" => {
                // ability handlers that changed something, since the process started
                let mut c = Json::obj();
                for (i, a) in cg_engine::sim::data::ABILITIES.iter().enumerate() {
                    let n = cg_engine::sim::abilities::TRIGGERS[i.min(319)].load(std::sync::atomic::Ordering::Relaxed);
                    if !a.id.is_empty() {
                        c.set(a.id, n as i64);
                    }
                }
                let mut it = Json::obj();
                for (i, x) in cg_engine::sim::data::ITEMS.iter().enumerate() {
                    let n = cg_engine::sim::items::TRIGGERS[i.min(169)].load(std::sync::atomic::Ordering::Relaxed);
                    if !x.id.is_empty() {
                        it.set(x.id, n as i64);
                    }
                }
                let mut o = Json::obj();
                o.set("ok", true);
                o.set("abilities", c);
                o.set("items", it);
                Ok(o)
            }
            "release" => {
                let mut o = Json::obj();
                o.set("ok", self.sims.remove(id).is_some());
                if o.get("ok") == Some(&Json::Bool(false)) {
                    o.set("error", format!("no battle {}", id));
                }
                Ok(o)
            }
            _ => Err(format!("unknown op {:?} for the new resolver", op)),
        }
    }
}
