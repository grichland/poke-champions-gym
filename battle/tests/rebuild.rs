//! A Battle built from the full record in one go equals the Battle built incrementally (LIVE.md rule 9).

mod common;
use battle::*;
use common::*;

/// A battle touching most event kinds, as one-event messages.
fn scripted() -> T {
    let mut t = T::new("garchomp", "gengar");
    t.es(&[
        r#"{"k":"ability","side":"opp","mon":"gengar","ability":"cursedbody"}"#,
        r#"{"k":"weather","weather":"sandstorm","what":"start"}"#,
    ]);
    t.turn(1);
    t.es(&[
        r#"{"k":"mega_stone","side":"own","mon":"garchomp","stone":"garchompitez"}"#,
        r#"{"k":"mega","side":"own","mon":"garchomp","form":"garchompmegaz"}"#,
        r#"{"k":"move","side":"opp","mon":"gengar","move":"shadowball","priority":0}"#,
        &hp_own("garchomp", 183, 120),
        &mv("own", "garchomp", "earthquake"),
        r#"{"k":"result","how":"immune","side":"opp","mon":"gengar"}"#,
        r#"{"k":"cause","effect":"sandstorm","side":"opp","mon":"gengar"}"#,
        &hp_opp("gengar", 100, 94),
        r#"{"k":"item","side":"opp","mon":"gengar","item":"leftovers","what":"activated"}"#,
        &hp_opp("gengar", 94, 100),
    ]);
    t.turn(2);
    t.es(&[
        r#"{"k":"withdraw","side":"opp","mon":"gengar"}"#,
        r#"{"k":"enter","side":"opp","mon":"corviknight"}"#,
        r#"{"k":"ability","side":"opp","mon":"corviknight","ability":"pressure"}"#,
        &mv("own", "garchomp", "outrage"),
        &hp_opp("corviknight", 100, 70),
        r#"{"k":"item","side":"opp","mon":"corviknight","item":"rockyhelmet","what":"activated"}"#,
        r#"{"k":"cause","effect":"rockyhelmet","side":"own","mon":"garchomp","of":{"side":"opp","mon":"corviknight"}}"#,
        &hp_own("garchomp", 120, 90),
    ]);
    t.turn(3);
    t.es(&[
        &mv("opp", "corviknight", "uturn"),
        &hp_own("garchomp", 90, 80),
        r#"{"k":"withdraw","side":"opp","mon":"corviknight","pivot":true}"#,
        r#"{"k":"enter","side":"opp","mon":"dragonite"}"#,
        &mv("own", "garchomp", "outrage"),
        r#"{"k":"result","how":"super"}"#,
        &hp_opp("dragonite", 100, 0),
        r#"{"k":"faint","side":"opp","mon":"dragonite"}"#,
        &vol("own", "garchomp", "confusion"),
        r#"{"k":"enter","side":"opp","mon":"gengar"}"#,
    ]);
    t.turn(4);
    t.es(&[
        r#"{"k":"move","side":"opp","mon":"gengar","move":"taunt"}"#,
        &vol("own", "garchomp", "taunt"),
        r#"{"k":"volatile_hit","effect":"confusion"}"#,
        &hp_own("garchomp", 80, 70),
        r#"{"k":"status","side":"opp","mon":"gengar","status":"brn"}"#,
        r#"{"k":"side","side":"own","effect":"spikes","what":"start"}"#,
        r#"{"k":"unread","text":"?"}"#,
        r#"{"k":"field","effect":"psychicterrain","what":"start"}"#,
        r#"{"k":"boost","side":"opp","mon":"gengar","stats":["spa"],"by":-2}"#,
    ]);
    t.turn(5);
    t.e(r#"{"k":"refused","order":{"kind":"switch","mon":"gholdengo"}}"#);
    t.es(&[
        &mv("own", "garchomp", "earthquake"),
        r#"{"k":"fail"}"#,
        r#"{"k":"move","side":"opp","mon":"gengar","move":"shadowball"}"#,
        &hp_own("garchomp", 70, 0),
        r#"{"k":"faint","side":"own","mon":"garchomp"}"#,
        r#"{"k":"enter","side":"own","mon":"gholdengo"}"#,
        r#"{"k":"end","result":"loss"}"#,
    ]);
    t.input(Input::Msg(msg(Body::BattleEnd { result: "loss".into() })));
    t
}

fn all_events(rec: &[Input]) -> (Vec<Input>, Vec<Event>) {
    let mut head = vec![];
    let mut events = vec![];
    for i in rec {
        match i {
            Input::Msg(Message { body: Body::Events { events: e, .. }, .. }) => events.extend(e.iter().cloned()),
            other if events.is_empty() => head.push(other.clone()),
            _ => {}
        }
    }
    (head, events)
}

#[test]
fn rebuild_equals_incremental() {
    let t = scripted();
    let again = Builder::rebuild(&t.rec);
    assert_eq!(again.battle, t.b.battle);
    assert_eq!(again.anomalies, t.b.anomalies);
    // every prefix
    for k in 0..=t.rec.len() {
        let mut inc = Builder::new();
        for i in &t.rec[..k] {
            inc.apply(i);
        }
        assert_eq!(Builder::rebuild(&t.rec[..k]).battle, inc.battle, "prefix {}", k);
    }
}

#[test]
fn rebuild_from_one_message_and_from_any_split() {
    let t = scripted();
    let (head, events) = all_events(&t.rec);
    let mut one = head.clone();
    one.push(Input::Msg(msg(Body::Events { events: events.clone(), fallback: None })));
    one.push(Input::Msg(msg(Body::BattleEnd { result: "loss".into() })));
    let b1 = Builder::rebuild(&one);
    assert_eq!(b1.battle, t.b.battle);
    assert_eq!(b1.n_events, t.b.n_events);
    for cut in [1, 7, 13, events.len() / 2, events.len() - 1] {
        let mut split = head.clone();
        split.push(Input::Msg(msg(Body::Events { events: events[..cut].to_vec(), fallback: None })));
        split.push(Input::Msg(msg(Body::Events { events: events[cut..].to_vec(), fallback: None })));
        split.push(Input::Msg(msg(Body::BattleEnd { result: "loss".into() })));
        assert_eq!(Builder::rebuild(&split).battle, t.b.battle, "cut {}", cut);
    }
    // a Battle can also be resumed from a copy of itself
    let mut half = Builder::rebuild(&head);
    for e in &events[..10] {
        half.apply_event(e);
    }
    let mut copy = Builder { battle: half.battle, ..Builder::new() };
    for e in &events[10..] {
        copy.apply_event(e);
    }
    copy.apply(&Input::Msg(msg(Body::BattleEnd { result: "loss".into() })));
    assert_eq!(copy.battle, t.b.battle);
}

#[test]
fn record_round_trips_through_json() {
    let t = scripted();
    let mut back = vec![];
    for i in &t.rec {
        back.push(match i {
            Input::Msg(m) => Input::Msg(Message::parse(&m.to_json().to_string()).unwrap()),
            Input::Picks(p) => Input::Picks(Picks::from_json(&json::Json::parse(&p.to_json().to_string()).unwrap()).unwrap()),
            Input::Order(o) => Input::Order(battle::Order::from_json(&json::Json::parse(&o.to_json().to_string()).unwrap()).unwrap()),
        });
    }
    assert_eq!(Builder::rebuild(&back).battle, t.b.battle);
}

#[test]
fn scripted_battle_state() {
    let t = scripted();
    let b = t.bt();
    assert_eq!(b.result, Some(Outcome::Loss));
    assert_eq!(b.unread, 1);
    assert!(b.side(Side::Own).mega_used);
    // Pressure doubled the second Outrage's cost: first use 2 (Pressure up), continuation free
    assert_eq!(t.own(1).pp_used[0], 2);
    assert_eq!(t.own(1).pp_used[1], 2);
    assert_eq!(t.opp(2).item, ItemState::Held(cg_engine::sim::data::ItemId::from_id("rockyhelmet").unwrap()));
    assert_eq!(t.opp(1).status, cg_engine::sim::data::Status::Brn);
    assert!(t.opp(4).fainted);
    println!("Battle is {} bytes; anomalies {:?}", std::mem::size_of::<Battle>(), t.anomalies());
    t.assert_clean();
}
