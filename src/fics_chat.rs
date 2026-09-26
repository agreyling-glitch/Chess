//! Session-only conversations layered over the FICS text protocol.
const MAX_MESSAGES: usize = 200;
const MAX_TABS: usize = 32;

#[derive(Default)]
pub struct Chats {
    pub tabs: Vec<Chat>,
    pub active: Option<String>,
    pub new_target: String,
    pub error: String,
    continuation: Option<String>,
}

pub struct Chat {
    pub target: String,
    pub messages: Vec<String>,
    pub draft: String,
    pub unread: usize,
}

impl Chat {
    pub fn title(&self) -> String {
        if self.target.bytes().all(|b| b.is_ascii_digit()) {
            format!("Channel {}", self.target)
        } else {
            self.target.clone()
        }
    }

    fn push(&mut self, message: String) {
        self.messages.push(message);
        if self.messages.len() > MAX_MESSAGES {
            self.messages.remove(0);
        }
    }
}

pub fn target(input: &str) -> Option<String> {
    let input = input.trim();
    let channel = input.strip_prefix('#').unwrap_or(input);
    if !channel.is_empty() && channel.bytes().all(|b| b.is_ascii_digit()) {
        return channel.parse::<u16>().ok().filter(|n| *n <= 255).map(|n| n.to_string());
    }
    (input.len() >= 3 && input.len() <= 17 && input.bytes().all(|b| b.is_ascii_alphabetic()))
        .then(|| input.to_owned())
}

fn sender(input: &str) -> Option<String> {
    let name = input.split('(').next()?;
    let suffix = &input[name.len()..];
    // Titles such as (*), (U), (TD), and (C) belong to the display name,
    // never to the recipient of a reply.
    if !suffix.is_empty() && (!suffix.starts_with('(') || !suffix.ends_with(')') || suffix.contains(' ')) {
        return None;
    }
    target(name).filter(|name| name.bytes().all(|b| b.is_ascii_alphabetic()))
}

fn incoming(line: &str) -> Option<String> {
    for separator in [" tells you: ", " says: "] {
        if let Some((name, _)) = line.split_once(separator) {
            return sender(name);
        }
    }
    let (prefix, _) = line.split_once("): ")?;
    let (name, channel) = prefix.rsplit_once('(')?;
    sender(name)?;
    if channel.is_empty() || !channel.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    target(channel)
}

pub fn command(recipient: &str, message: &str) -> Option<String> {
    let recipient = target(recipient)?;
    let message = message.trim();
    let command = format!("tell {recipient} {message}");
    (!message.is_empty() && command.len() <= 256
        && message.bytes().all(|b| (32..=126).contains(&b)))
        .then_some(command)
}

impl Chats {
    pub fn ensure(&mut self, recipient: &str) -> Option<usize> {
        let recipient = target(recipient)?;
        if let Some(index) = self.tabs.iter().position(|tab| tab.target.eq_ignore_ascii_case(&recipient)) {
            return Some(index);
        }
        if self.tabs.len() >= MAX_TABS { return None; }
        self.tabs.push(Chat { target: recipient, messages: Vec::new(), draft: String::new(), unread: 0 });
        Some(self.tabs.len() - 1)
    }

    pub fn receive(&mut self, line: &str, visible: bool) {
        let line = line.trim();
        if let Some(recipient) = incoming(line) {
            self.continuation = Some(recipient.clone());
            if let Some(index) = self.ensure(&recipient) {
                let tab = &mut self.tabs[index];
                tab.push(line.to_owned());
                if !visible || self.active.as_ref().is_none_or(|active| !active.eq_ignore_ascii_case(&recipient)) {
                    tab.unread += 1;
                }
            }
        } else if let Some(text) = line.strip_prefix('\\') {
            if let Some(recipient) = &self.continuation {
                if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.target.eq_ignore_ascii_case(recipient)) {
                    if let Some(last) = tab.messages.last_mut() {
                        last.push(' ');
                        // Bound wrapped server messages as well as the number of rows.
                        let remaining = 16384_usize.saturating_sub(last.len());
                        last.extend(text.trim().chars().take(remaining));
                    }
                }
            }
        } else {
            self.continuation = None;
        }
    }

    pub fn submitted(&mut self, recipient: &str, message: &str) {
        if let Some(index) = self.ensure(recipient) {
            self.tabs[index].push(format!("You (submitted): {}", message.trim()));
        }
    }

    pub fn close(&mut self, recipient: &str) {
        self.tabs.retain(|tab| !tab.target.eq_ignore_ascii_case(recipient));
        if self.active.as_deref() == Some(recipient) { self.active = None; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_tells_titles_channels_and_wrapped_messages() {
        let mut chats = Chats::default();
        chats.receive("adminBOT(C) tells you: You have been muted", false);
        chats.receive("\\ because guests cannot post.", false);
        chats.receive("Alice(*)(50): Hello channel", false);
        chats.receive("Bob says: Good game", false);
        assert_eq!(chats.tabs.len(), 3);
        assert_eq!(chats.tabs[0].target, "adminBOT");
        assert_eq!(chats.tabs[0].messages[0], "adminBOT(C) tells you: You have been muted because guests cannot post.");
        assert_eq!(chats.tabs[1].target, "50");
        assert_eq!(chats.tabs[2].target, "Bob");
        chats.receive("Game information", false);
        chats.receive("\\ not chat", false);
        assert_eq!(chats.tabs[2].messages.len(), 1);
    }

    #[test]
    fn unread_and_case_insensitive_conversations() {
        let mut chats = Chats::default();
        chats.active = Some("alice".into());
        chats.receive("Alice tells you: Hello", true);
        chats.receive("ALICE tells you: Again", false);
        assert_eq!(chats.tabs.len(), 1);
        assert_eq!(chats.tabs[0].unread, 1);
        assert_eq!(chats.tabs[0].messages.len(), 2);
    }

    #[test]
    fn validates_targets_and_prevents_command_injection() {
        assert_eq!(command("Alice", "hi"), Some("tell Alice hi".into()));
        assert_eq!(command("#050", "hi"), Some("tell 50 hi".into()));
        for recipient in ["!", "^", ".", "Alice\nquit", "Alice Bob", "256", "#Alice"] {
            assert!(command(recipient, "hi").is_none());
        }
        assert!(command("Alice", "hi\nquit").is_none());
        assert!(command("Alice", &"x".repeat(256)).is_none());
        assert!(command("Alice", " ").is_none());
    }

    #[test]
    fn bounds_history_and_tabs() {
        let mut chats = Chats::default();
        for _ in 0..250 { chats.receive("Alice tells you: hi", false); }
        assert_eq!(chats.tabs[0].messages.len(), MAX_MESSAGES);
        for n in 0..100 { chats.ensure(&n.to_string()); }
        assert_eq!(chats.tabs.len(), MAX_TABS);
        chats.close("Alice");
        assert!(chats.ensure("Bob").is_some());
    }
}
