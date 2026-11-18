//! JSON round trip of every event kind, every message type and both answers.

mod common;
use battle::json::Json;
use battle::*;

const EVENTS: &[&str] = &[
    r#"{"k":"turn","n":3}"#,
    r#"{"k":"refused","order":{"kind":"switch","mon":"garchomp"}}"#,
    r#"{"k":"refused","order":{"kind":"move","move":"earthquake","mega":true}}"#,
    r#"{"k":"end","result":"win"}"#,
    r#"{"k":"enter","side":"opp","mon":"garchomp"}"#,
    r#"{"k":"enter","side":"opp","mon":"garchomp","forced":true}"#,
    r#"{"k":"withdraw","side":"own","mon":"golisopod"}"#,
    r#"{"k":"withdraw","side":"own","mon":"golisopod","pivot":true}"#,
    r#"{"k":"move","side":"own","mon":"gholdengo","move":"makeitrain"}"#,
    r#"{"k":"move","side":"opp","mon":"dragonite","move":"extremespeed","priority":2}"#,
    r#"{"k":"move","side":"own","mon":"cinderace","move":"pyroball","called_by":"sleeptalk"}"#,
    r#"{"k":"prepare","side":"own","mon":"primarina"}"#,
    r#"{"k":"prepare","side":"own","mon":"primarina","move":"solarbeam"}"#,
    r#"{"k":"cant","side":"opp","mon":"garchomp","reason":"par"}"#,
    r#"{"k":"protect","side":"opp","mon":"garchomp"}"#,
    r#"{"k":"miss","side":"own","mon":"garchomp"}"#,
    r#"{"k":"fail"}"#,
    r#"{"k":"fail","side":"opp","mon":"gengar"}"#,
    r#"{"k":"result","how":"super"}"#,
    r#"{"k":"result","how":"immune","side":"opp","mon":"gengar"}"#,
    r#"{"k":"result","how":"mostly_ineffective"}"#,
    r#"{"k":"hits","n":3}"#,
    r#"{"k":"hp","side":"own","mon":"gholdengo","dir":"lost","before":162,"after":100}"#,
    r#"{"k":"hp","side":"opp","mon":"garchomp","dir":"restored","before_pct":50,"after_pct":56.5}"#,
    r#"{"k":"cause","effect":"brn","side":"own","mon":"gholdengo"}"#,
    r#"{"k":"cause","effect":"rockyhelmet","side":"own","mon":"garchomp","of":{"side":"opp","mon":"corviknight"}}"#,
    r#"{"k":"cause","effect":"painsplit"}"#,
    r#"{"k":"faint","side":"opp","mon":"gengar"}"#,
    r#"{"k":"status","side":"opp","mon":"gengar","status":"tox"}"#,
    r#"{"k":"cure","side":"own","mon":"garchomp","status":"par"}"#,
    r#"{"k":"cure","side":"own","mon":"garchomp","status":"par","item":"lumberry"}"#,
    r#"{"k":"volatile","side":"own","mon":"garchomp","effect":"encore"}"#,
    r#"{"k":"volatile","side":"own","mon":"garchomp","effect":"disable","detail":"earthquake"}"#,
    r#"{"k":"volatile","side":"own","mon":"garchomp","effect":"perishsong","detail":2}"#,
    r#"{"k":"volatile_end","side":"own","mon":"garchomp","effect":"taunt"}"#,
    r#"{"k":"volatile_hit","effect":"substitute"}"#,
    r#"{"k":"volatile_hit","effect":"endure","side":"opp","mon":"garchomp"}"#,
    r#"{"k":"boost","side":"opp","mon":"garchomp","stats":["atk"],"by":-1}"#,
    r#"{"k":"boost_limit","side":"opp","mon":"garchomp","stats":["atk"],"dir":"up"}"#,
    r#"{"k":"boost_reset","what":"all"}"#,
    r#"{"k":"boost_reset","side":"own","mon":"garchomp","what":"negative"}"#,
    r#"{"k":"boost_reset","side":"own","mon":"garchomp","what":"copy","of":{"side":"opp","mon":"gengar"}}"#,
    r#"{"k":"blocked","effect":"psychicterrain","side":"own","mon":"garchomp"}"#,
    r#"{"k":"blocked","effect":"unnerve","what":"berries"}"#,
    r#"{"k":"ability","side":"opp","mon":"corviknight","ability":"pressure"}"#,
    r#"{"k":"ability_change","side":"opp","mon":"gengar","from":"trace","to":"intimidate"}"#,
    r#"{"k":"item","side":"opp","mon":"garchomp","item":"sitrusberry","what":"activated"}"#,
    r#"{"k":"item","side":"opp","mon":"garchomp","item":"leftovers","what":"removed","of":{"side":"own","mon":"golisopod"}}"#,
    r#"{"k":"mega_stone","side":"own","mon":"garchomp","stone":"garchompitez"}"#,
    r#"{"k":"mega","side":"own","mon":"garchomp","form":"garchompmegaz"}"#,
    r#"{"k":"form","side":"opp","mon":"mimikyu","form":"mimikyubusted"}"#,
    r#"{"k":"transform","side":"opp","mon":"ditto","into":"garchomp"}"#,
    r#"{"k":"types","side":"own","mon":"cinderace","type":"fire"}"#,
    r#"{"k":"weather","weather":"sandstorm","what":"start"}"#,
    r#"{"k":"field","effect":"trickroom","what":"end"}"#,
    r#"{"k":"side","side":"own","effect":"stealthrock","what":"acted"}"#,
    r#"{"k":"once","effect":"perishsong"}"#,
    r#"{"k":"once","effect":"destinybond","side":"opp","mon":"gengar"}"#,
    r#"{"k":"revive","side":"own","mon":"garchomp"}"#,
    r#"{"k":"unread","text":"Something odd happened!"}"#,
    r#"{"k":"unread","text":"Held Item: X's Berrv","category":"Held Item"}"#,
    r#"{"k":"gap","what":"turn","n":7}"#,
    r#"{"k":"gap","what":"rows"}"#,
];

#[test]
fn every_event_kind_round_trips() {
    let mut kinds = std::collections::BTreeSet::new();
    for text in EVENTS {
        let j = Json::parse(text).unwrap();
        let e = Event::from_json(&j).unwrap_or_else(|err| panic!("{}: {}", text, err));
        kinds.insert(e.kind());
        // same JSON value back, and the same event from it
        assert_eq!(e.to_json(), j, "{}", text);
        assert_eq!(Event::from_json(&Json::parse(&e.to_json().to_string()).unwrap()).unwrap(), e);
    }
    // all 40 kinds of LIVE.md section 4
    assert_eq!(kinds.len(), 40, "{:?}", kinds);
}

#[test]
fn bad_events_are_errors() {
    for text in [
        r#"{"k":"nonsense"}"#,
        r#"{"k":"enter","side":"p1","mon":"garchomp"}"#,
        r#"{"k":"enter","mon":"garchomp"}"#,
        r#"{"k":"fail","side":"own"}"#,
        r#"{"k":"result","how":"great"}"#,
        r#"{"k":"hp","side":"own","mon":"x","dir":"lost","before_pct":5,"after_pct":2}"#,
    ] {
        assert!(Event::from_json(&Json::parse(text).unwrap()).is_err(), "{}", text);
    }
}

#[test]
fn messages_round_trip() {
    let texts = [
        r#"{"v":"0.3","battle":"b-1","type":"preview_request","deadline_ms":45000}"#,
        r#"{"v":"0.3","battle":"b-1","type":"events","events":[{"k":"turn","n":1},{"k":"fail"}]}"#,
        r#"{"v":"0.5","battle":"b-1","type":"events","events":[],"fallback":{"seq":7,"order":{"kind":"move","move":"earthquake","mega":false},"reason":"timeout"}}"#,
        r#"{"v":"0.5","battle":"b-1","type":"events","events":[],"fallback":{"order":{"kind":"switch","mon":"garchomp"}}}"#,
        r#"{"v":"0.3","battle":"b-1","type":"decision_request","seq":7,"kind":"turn","deadline_ms":12000}"#,
        r#"{"v":"0.3","battle":"b-1","type":"decision_request","seq":8,"kind":"replace","deadline_ms":12000,"refused":{"order":{"kind":"switch","mon":"garchomp"}},"screen_offer":{"moves":["earthquake"]}}"#,
        r#"{"v":"0.5","battle":"b-1","type":"decision_request","seq":9,"kind":"turn","retry":true}"#,
        r#"{"v":"0.3","battle":"b-1","type":"battle_end","result":"forfeit"}"#,
    ];
    for t in texts {
        let j = Json::parse(t).unwrap();
        let m = Message::from_json(&j).unwrap_or_else(|e| panic!("{}: {}", t, e));
        assert_eq!(m.to_json(), j, "{}", t);
    }
    let start = common::msg(Body::BattleStart { own_team: common::team(), opp_preview: common::preview() });
    let mut with_points = start.clone();
    if let Body::BattleStart { own_team, opp_preview } = &mut with_points.body {
        own_team[0].points = Some([2, 32, 0, 32, 0, 0]);
        own_team[0].nature = Some("modest".into());
        own_team[1].item = None;
        opp_preview[2] = None;
    }
    for m in [start, with_points] {
        let text = m.to_json().to_string();
        assert_eq!(Message::parse(&text).unwrap(), m);
    }
}

/// Streams written before 0.5 sent the fallback as a bare ORDER: it still reads, and is written back
/// in the contract's shape.
#[test]
fn bare_fallback_order_still_reads() {
    let t = r#"{"v":"0.3","battle":"b-1","type":"events","events":[],"fallback":{"kind":"move","move":"earthquake","mega":false}}"#;
    let m = Message::parse(t).unwrap();
    let order = Order::Move { mv: "earthquake".into(), mega: false };
    match &m.body {
        Body::Events { fallback: Some(f), .. } => assert_eq!(f, &vec![Fallback { seq: None, order: Some(order.clone()), reason: None, picks: None }]),
        b => panic!("{:?}", b),
    }
    let back = m.to_json();
    assert_eq!(back.get("fallback").unwrap().to_string(), r#"{"order":{"kind":"move","mega":false,"move":"earthquake"}}"#);
    assert_eq!(Message::from_json(&back).unwrap(), m);
    // the documented shape keeps seq and reason
    let t = r#"{"v":"0.5","battle":"b-1","type":"events","events":[],"fallback":{"seq":4,"order":{"kind":"switch","mon":"garchomp"},"reason":"late"}}"#;
    match Message::parse(t).unwrap().body {
        Body::Events { fallback: Some(f), .. } => {
            assert_eq!(f, vec![Fallback { seq: Some(4), order: Some(Order::Switch { mon: "garchomp".into() }), reason: Some("late".into()), picks: None }])
        }
        b => panic!("{:?}", b),
    }
    // no order (0.6, LV5Q-1): the game chose and the controller could not read what
    match Message::parse(r#"{"v":"0.6","battle":"b-1","type":"events","events":[],"fallback":{"seq":4,"reason":"late"}}"#).unwrap().body {
        Body::Events { fallback: Some(f), .. } => assert_eq!(f, vec![Fallback { seq: Some(4), order: None, reason: Some("late".into()), picks: None }]),
        b => panic!("{:?}", b),
    }
}

/// 0.7 (S8-1, S8-3): a preview fallback with the picks, and several fallbacks in one message, in order.
#[test]
fn preview_fallback_and_fallback_lists() {
    let t = r#"{"v":"0.7","battle":"b-1","type":"events","events":[],"fallback":{"picks":[3,0,1],"reason":"overruled"}}"#;
    let m = Message::parse(t).unwrap();
    match &m.body {
        Body::Events { fallback: Some(f), .. } => assert_eq!(f, &vec![Fallback { seq: None, order: None, reason: Some("overruled".into()), picks: Some(vec![3, 0, 1]) }]),
        b => panic!("{:?}", b),
    }
    assert_eq!(m.to_json(), Json::parse(t).unwrap());
    let t = r#"{"v":"0.7","battle":"b-1","type":"events","events":[],"fallback":[{"seq":4,"reason":"late"},{"order":{"kind":"move","move":"earthquake"},"seq":5}]}"#;
    let m = Message::parse(t).unwrap();
    match &m.body {
        Body::Events { fallback: Some(f), .. } => {
            assert_eq!(f.iter().map(|x| x.seq).collect::<Vec<_>>(), vec![Some(4), Some(5)]);
            // `mega` left out reads as false
            assert_eq!(f[1].order, Some(Order::Move { mv: "earthquake".into(), mega: false }));
        }
        b => panic!("{:?}", b),
    }
    assert_eq!(Message::from_json(&m.to_json()).unwrap(), m);
    // an empty list is no fallback; a bad entry fails the message
    assert!(matches!(Message::parse(r#"{"v":"0.7","battle":"b","type":"events","events":[],"fallback":[]}"#).unwrap().body, Body::Events { fallback: None, .. }));
    assert!(Message::parse(r#"{"v":"0.7","battle":"b","type":"events","events":[],"fallback":[{"picks":[9]}]}"#).is_err());
    // CG always writes `mega` on a move
    assert_eq!(Order::Move { mv: "earthquake".into(), mega: false }.to_json().to_string(), r#"{"kind":"move","mega":false,"move":"earthquake"}"#);
}

#[test]
fn retry_flag_reads_and_writes() {
    let t = r#"{"v":"0.5","battle":"b-1","type":"decision_request","seq":3,"kind":"turn","retry":true}"#;
    match Message::parse(t).unwrap().body {
        Body::DecisionRequest { retry, refused, .. } => assert!(retry && refused.is_none()),
        b => panic!("{:?}", b),
    }
    let t = r#"{"v":"0.5","battle":"b-1","type":"decision_request","seq":3,"kind":"turn"}"#;
    let m = Message::parse(t).unwrap();
    assert!(matches!(m.body, Body::DecisionRequest { retry: false, .. }));
    assert!(m.to_json().get("retry").is_none());
}

#[test]
fn answers_round_trip() {
    let p = Picks { picks: vec![4, 1, 2] };
    assert_eq!(p.to_json().to_string(), r#"{"picks":[4,1,2]}"#);
    assert_eq!(Picks::from_json(&p.to_json()).unwrap(), p);
    for t in [
        r#"{"order":{"kind":"move","mega":false,"move":"earthquake"},"seq":7}"#,
        r#"{"order":{"kind":"switch","mon":"garchomp"},"seq":7}"#,
        r#"{"notes":{"why":"test"},"order":{"kind":"move","mega":true,"move":"outrage"},"seq":9}"#,
    ] {
        let a = Answer::from_json(&Json::parse(t).unwrap()).unwrap();
        assert_eq!(a.to_json().to_string(), t);
    }
}
