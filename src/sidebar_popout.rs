//! Decode local input for the detached panel's own UI context.
use egui::{Event, Key, Modifiers, MouseWheelUnit, PointerButton, RawInput, pos2, vec2};

pub fn append_input(raw: &mut RawInput, json: &str) {
    let Ok(events) = serde_json::from_str::<Vec<serde_json::Value>>(json) else { return; };
    for event in events.into_iter().take(512) {
        let number = |key: &str| event[key].as_f64().filter(|n| n.is_finite() && n.abs() <= 1_000_000.0).map(|n|n as f32);
        let flag = |key: &str| event[key].as_bool().unwrap_or(false);
        let modifiers = Modifiers {alt:flag("alt"),ctrl:flag("ctrl"),shift:flag("shift"),
            mac_cmd:flag("meta"),command:flag("ctrl")||flag("meta")};
        match event["type"].as_str().unwrap_or("") {
            "move" => if let Some((x,y))=number("x").zip(number("y")){raw.events.push(Event::PointerMoved(pos2(x,y)));},
            "button" => if let Some((x,y))=number("x").zip(number("y")) {
                let button=match event["button"].as_u64(){Some(0)=>PointerButton::Primary,Some(1)=>PointerButton::Middle,Some(2)=>PointerButton::Secondary,_=>continue};
                raw.events.push(Event::PointerButton{pos:pos2(x,y),button,pressed:flag("pressed"),modifiers});
            },
            "gone"=>raw.events.push(Event::PointerGone),
            "wheel"=>if let Some((x,y))=number("x").zip(number("y")) {
                raw.events.push(Event::MouseWheel{unit:match event["mode"].as_u64(){Some(1)=>MouseWheelUnit::Line,Some(2)=>MouseWheelUnit::Page,_=>MouseWheelUnit::Point},delta:vec2(x,y),phase:egui::TouchPhase::Move,modifiers});
            },
            "key"=>{
                raw.events.push(Event::ModifiersChanged(modifiers));
                if let Some(key)=event["key"].as_str().and_then(Key::from_name) {
                    raw.events.push(Event::Key{key,physical_key:None,pressed:flag("pressed"),repeat:flag("repeat"),modifiers});
                }
            },
            "text"|"paste"=>if let Some(text)=event["text"].as_str().filter(|s|s.len()<=1_000_000) {
                raw.events.push(if event["type"]=="paste"{Event::Paste(text.to_owned())}else{Event::Text(text.to_owned())});
            },
            "copy"=>raw.events.push(Event::Copy),
            "cut"=>raw.events.push(Event::Cut),
            _=>{},
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sidebar_input_preserves_local_coordinates_and_text() {
        let mut raw=RawInput::default();
        append_input(&mut raw,r#"[{"type":"move","x":820,"y":120},{"type":"button","x":820,"y":120,"button":0,"pressed":true},{"type":"text","text":"Study ♟"},{"type":"wheel","x":0,"y":-50,"mode":0}]"#);
        assert_eq!(raw.events[0],Event::PointerMoved(pos2(820.0,120.0)));
        assert!(matches!(raw.events[1],Event::PointerButton{pressed:true,button:PointerButton::Primary,..}));
        assert_eq!(raw.events[2],Event::Text("Study ♟".into()));
        assert!(matches!(raw.events[3],Event::MouseWheel{delta,..} if delta.y == -50.0));
    }
    #[test]
    fn sidebar_input_ignores_bad_coordinates_and_unknown_buttons() {
        let mut raw=RawInput::default();
        append_input(&mut raw,r#"[{"type":"move","x":1e99,"y":2},{"type":"button","x":1,"y":2,"button":7},{"type":"unknown"}]"#);
        assert!(raw.events.is_empty());
    }
}
