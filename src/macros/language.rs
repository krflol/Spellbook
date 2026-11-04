//! A conservative macro lexer, completion engine, and structural diagnostics.
//! This is not a Lua interpreter or a simulation of protected WoW actions.
pub use super::snippets::SNIPPETS;
use super::{byte_index, BODY_LIMIT};
use std::ops::Range;

#[derive(Clone, Copy)]
pub struct Reference {
    pub name: &'static str,
    pub description: &'static str,
    pub example: &'static str,
}
macro_rules! references {
    ($(($name:literal,$description:literal,$example:literal)),* $(,)?) => {
        &[$(Reference { name:$name, description:$description, example:$example }),*]
    }
}
pub const COMMANDS: &[Reference] = references![
    ("#showtooltip", "Choose the action button's tooltip and icon. With no argument, infer them from the macro.", "#showtooltip [mod:shift] Frostbolt; Fireball"),
    ("#show", "Choose the action button's icon and count.", "#show Healing Potion"),
    ("/cast", "Cast a spell. Conditional groups are checked from left to right.", "/cast [@mouseover,help,nodead][] Flash Heal"),
    ("/use", "Use an item by name, item ID, equipment slot, or bag and slot; also accepts spells.", "/use [combat] 13"),
    ("/castsequence", "Advance a sequence on successful casts. Each press attempts one action. reset= is optional.", "/castsequence reset=target/10 Corruption, Curse of Agony"),
    ("/castrandom", "Choose an action from a comma-separated list.", "/castrandom Fireball, Frostbolt"),
    ("/userandom", "Choose an item from a comma-separated list.", "/userandom Healing Potion, Lesser Healing Potion"),
    ("/stopcasting", "Stop the current cast before processing subsequent macro lines.", "/stopcasting"),
    ("/stopmacro", "Stop processing the macro when the conditions match.", "/stopmacro [noexists][dead]"),
    ("/cancelaura", "Remove a named cancellable buff.", "/cancelaura Ice Block"),
    ("/cancelform", "Leave the current shapeshift form.", "/cancelform [form]"),
    ("/startattack", "Start auto-attacking the target.", "/startattack [harm,nodead]"),
    ("/stopattack", "Stop auto-attacking.", "/stopattack"),
    ("/target", "Select a unit by name or unit token.", "/target [@mouseover,exists]"),
    ("/tar", "Short form of /target.", "/tar Hogger"),
    ("/targetexact", "Select a unit whose name matches exactly.", "/targetexact Hogger"),
    ("/targetenemy", "Cycle to an enemy target.", "/targetenemy [noharm][dead]"),
    ("/targetfriend", "Cycle to a friendly target.", "/targetfriend"),
    ("/targetlasttarget", "Return to the previous target.", "/targetlasttarget"),
    ("/targetlastenemy", "Return to the previous enemy target.", "/targetlastenemy"),
    ("/targetlastfriend", "Return to the previous friendly target.", "/targetlastfriend"),
    ("/cleartarget", "Clear the current target.", "/cleartarget [dead]"),
    ("/focus", "Set the focus unit.", "/focus [@mouseover,exists][]"),
    ("/clearfocus", "Clear the focus unit.", "/clearfocus [@focus,dead]"),
    ("/assist", "Select another unit's target.", "/assist [@focus,exists]"),
    ("/petattack", "Send your pet to attack.", "/petattack [@target,harm,nodead]"),
    ("/petfollow", "Call your pet back to follow.", "/petfollow"),
    ("/petstay", "Tell your pet to stay.", "/petstay"),
    ("/petpassive", "Set your pet to passive.", "/petpassive"),
    ("/petdefensive", "Set your pet to defensive.", "/petdefensive"),
    ("/petautocaston", "Enable autocast for a named pet ability.", "/petautocaston Growl"),
    ("/petautocastoff", "Disable autocast for a named pet ability.", "/petautocastoff Growl"),
    ("/petautocasttoggle", "Toggle autocast for a named pet ability.", "/petautocasttoggle Growl"),
    ("/equip", "Equip an item by name.", "/equip Worn Shortsword"),
    ("/equipslot", "Equip an item into a numbered equipment slot.", "/equipslot 16 Worn Shortsword"),
    ("/equipset", "Equip a saved equipment set if supported by the client.", "/equipset Tank"),
    ("/dismount", "Dismount when the conditions match.", "/dismount [mounted]"),
    ("/leavevehicle", "Leave the current vehicle.", "/leavevehicle"),
    ("/changeactionbar", "Switch to a numbered action bar page.", "/changeactionbar 2"),
    ("/swapactionbar", "Swap between two action bar pages.", "/swapactionbar 1 2"),
    ("/click", "Click a named UI button, subject to the client's protected-action restrictions.", "/click CharacterMicroButton"),
    ("/run", "One line of Lua. Syntax is highlighted; Spellbook does not execute or fully type-check Lua.", "/run print(UnitName(\"player\"))"),
    ("/script", "Alias of /run.", "/script print(GetTime())"),
    ("/console", "Set a console variable.", "/console Sound_EnableMusic 0"),
    ("/say", "Send text to nearby players. %t expands to your target's name.", "/say Hello, %t!"),
    ("/s", "Short form of /say.", "/s Hello!"),
    ("/party", "Send text to your party.", "/party Ready!"),
    ("/p", "Short form of /party.", "/p Ready!"),
    ("/raid", "Send text to your raid.", "/raid Ready!"),
    ("/rw", "Send a raid warning when permitted.", "/rw Move away!"),
    ("/yell", "Yell text to nearby players.", "/yell For the Alliance!"),
    ("/emote", "Send a custom emote.", "/emote waves at %t."),
    ("/whisper", "Whisper a named player.", "/whisper Player Hello!"),
    ("/w", "Short form of /whisper.", "/w Player Hello!"),
    ("/guild", "Send text to your guild.", "/guild Hello!"),
    ("/g", "Short form of /guild.", "/g Hello!"),
    ("/reload", "Reload the game's interface.", "/reload"),
    ("/readycheck", "Start a group ready check when permitted.", "/readycheck"),
    ("/clearworldmarker", "Clear a numbered world marker when supported.", "/clearworldmarker 1"),
    ("/wm", "Place a numbered world marker when supported.", "/wm 1"),
];
pub const CONDITIONS: &[Reference] = references![
    ("help", "The selected unit is friendly.", "[help,nodead]"),
    ("harm", "The selected unit is hostile.", "[harm,nodead]"),
    ("exists", "The selected unit exists.", "[@focus,exists]"),
    ("dead", "The selected unit is dead.", "[dead]"),
    ("nodead", "The selected unit is alive.", "[help,nodead]"),
    (
        "noexists",
        "The selected unit does not exist.",
        "[@focus,noexists]"
    ),
    ("combat", "You are in combat.", "[combat]"),
    ("nocombat", "You are out of combat.", "[nocombat]"),
    (
        "mod",
        "A modifier key is held; optionally specify shift, ctrl, or alt.",
        "[mod:shift]"
    ),
    (
        "nomod",
        "No modifier key is held; optionally specify which modifier.",
        "[nomod]"
    ),
    ("modifier", "Long form of mod.", "[modifier:ctrl]"),
    (
        "btn",
        "The macro was clicked with this mouse button number.",
        "[btn:2]"
    ),
    ("button", "Long form of btn.", "[button:2]"),
    (
        "stance",
        "You are in a numbered stance; 0 means no stance/form.",
        "[stance:1/2]"
    ),
    (
        "form",
        "Alias of stance; numbering depends on your class and available forms.",
        "[form:1]"
    ),
    ("stealth", "You are stealthed.", "[stealth]"),
    ("nostealth", "You are not stealthed.", "[nostealth]"),
    ("mounted", "You are mounted.", "[mounted]"),
    ("nomounted", "You are not mounted.", "[nomounted]"),
    ("swimming", "You are swimming.", "[swimming]"),
    ("flying", "You are flying, where supported.", "[flying]"),
    (
        "flyable",
        "The area permits flying, where supported.",
        "[flyable]"
    ),
    ("indoors", "You are indoors.", "[indoors]"),
    ("outdoors", "You are outdoors.", "[outdoors]"),
    (
        "group",
        "You are in a group, optionally party or raid.",
        "[group:party]"
    ),
    (
        "party",
        "The selected unit belongs to your party.",
        "[party]"
    ),
    ("raid", "The selected unit belongs to your raid.", "[raid]"),
    (
        "pet",
        "You have a pet; optionally specify a name or family.",
        "[pet]"
    ),
    ("nopet", "You have no active pet.", "[nopet]"),
    (
        "channeling",
        "You are channeling, optionally a named spell.",
        "[nochanneling]"
    ),
    (
        "nochanneling",
        "You are not channeling, optionally a named spell.",
        "[nochanneling]"
    ),
    (
        "equipped",
        "You have an item or item type equipped.",
        "[equipped:Shields]"
    ),
    ("worn", "Alias of equipped.", "[worn:Shields]"),
    (
        "actionbar",
        "A numbered action bar is active.",
        "[actionbar:1]"
    ),
    (
        "spec",
        "Your specialization index, when supported by the client.",
        "[spec:1]"
    ),
    (
        "known",
        "You know the spell ID or name, when supported by the client.",
        "[known:133]"
    ),
    (
        "talent",
        "A talent row/column is selected, when supported by the client.",
        "[talent:1/2]"
    ),
    (
        "vehicleui",
        "The vehicle interface is active.",
        "[vehicleui]"
    ),
    ("possessbar", "A possession bar is active.", "[possessbar]"),
    (
        "overridebar",
        "An override action bar is active.",
        "[overridebar]"
    ),
    (
        "bonusbar",
        "A numbered bonus action bar is active.",
        "[bonusbar:1]"
    ),
    (
        "unithasvehicleui",
        "The selected unit has a vehicle UI.",
        "[unithasvehicleui]"
    ),
    (
        "canexitvehicle",
        "You can exit the current vehicle.",
        "[canexitvehicle]"
    ),
];
pub const TARGETS: &[Reference] = references![
    (
        "@mouseover",
        "Use the unit under your cursor without changing your target.",
        "[@mouseover,help,nodead]"
    ),
    (
        "@target",
        "Use your current target.",
        "[@target,harm,nodead]"
    ),
    ("@player", "Use yourself as the target.", "[@player]"),
    ("@focus", "Use your focus unit.", "[@focus,harm,nodead]"),
    (
        "@targettarget",
        "Use your target's target.",
        "[@targettarget,help]"
    ),
    (
        "@focustarget",
        "Use your focus's target.",
        "[@focustarget,harm]"
    ),
    ("@pet", "Use your pet.", "[@pet,exists]"),
    ("@pettarget", "Use your pet's target.", "[@pettarget,harm]"),
    (
        "@cursor",
        "Place a supported ground-targeted action at the cursor.",
        "[@cursor]"
    ),
    (
        "@party1",
        "Use the first other party member. Also party2 through party4.",
        "[@party1,help,nodead]"
    ),
    (
        "@party2",
        "Use the second other party member.",
        "[@party2,help,nodead]"
    ),
    (
        "@party3",
        "Use the third other party member.",
        "[@party3,help,nodead]"
    ),
    (
        "@party4",
        "Use the fourth other party member.",
        "[@party4,help,nodead]"
    ),
    (
        "@raid1",
        "Use raid member 1. Other raid indices may be used.",
        "[@raid1,help,nodead]"
    ),
];
pub const LUA: &[Reference] = references![
    ("print", "print(...) — print values to your chat frame.", "print(UnitName(\"player\"))"),
    ("UnitName", "UnitName(unit) — return a unit's name.", "UnitName(\"player\")"),
    ("UnitClass", "UnitClass(unit) — return a unit's class names and ID.", "UnitClass(\"player\")"),
    ("UnitLevel", "UnitLevel(unit) — return a unit's level.", "UnitLevel(\"player\")"),
    ("UnitExists", "UnitExists(unit) — test whether a unit exists.", "UnitExists(\"target\")"),
    ("UnitGUID", "UnitGUID(unit) — return a unit's GUID.", "UnitGUID(\"target\")"),
    ("GetTime", "GetTime() — return elapsed client time in seconds.", "GetTime()"),
    ("GetNumShapeshiftForms", "GetNumShapeshiftForms() — count your learned stance/form slots.", "GetNumShapeshiftForms()"),
    ("GetShapeshiftFormInfo", "GetShapeshiftFormInfo(index) — return icon, active, castable, and spell ID for a stance/form slot.", "GetShapeshiftFormInfo(1)"),
    ("C_Spell.GetSpellName", "C_Spell.GetSpellName(spellID) — look up the name of a client spell; may return nil.", "C_Spell.GetSpellName(768)"),
    ("GetMoney", "GetMoney() — return your money in copper.", "GetMoney()"),
    ("GetCVar", "GetCVar(name) — read a console variable.", "GetCVar(\"Sound_EnableMusic\")"),
    ("SetCVar", "SetCVar(name, value) — set a console variable.", "SetCVar(\"Sound_EnableMusic\", 0)"),
    ("IsInGroup", "IsInGroup() — test group membership.", "IsInGroup()"),
    ("IsInRaid", "IsInRaid() — test raid membership.", "IsInRaid()"),
    ("GetNumGroupMembers", "GetNumGroupMembers() — return group size.", "GetNumGroupMembers()"),
    ("SendChatMessage", "SendChatMessage(text, type, language, target) — send chat, subject to client restrictions.", "SendChatMessage(\"Hello\", \"PARTY\")"),
    ("PlaySound", "PlaySound(soundKitID) — play a sound kit.", "PlaySound(12867)"),
    ("C_Map.GetBestMapForUnit", "C_Map.GetBestMapForUnit(unit) — return the best map ID for a unit.", "C_Map.GetBestMapForUnit(\"player\")"),
    ("string.format", "string.format(format, ...) — format a string.", "string.format(\"%s\", UnitName(\"player\"))"),
    ("string.find", "string.find(text, pattern) — find a Lua pattern.", "string.find(\"hello\", \"ell\")"),
    ("string.len", "string.len(text) — count bytes in a string.", "string.len(\"hello\")"),
    ("table.concat", "table.concat(values, separator) — join a list.", "table.concat({1, 2}, \", \")"),
    ("tostring", "tostring(value) — convert a value to text.", "tostring(42)"),
    ("tonumber", "tonumber(value) — convert text to a number.", "tonumber(\"42\")"),
    ("pairs", "pairs(table) — iterate a table.", "for k,v in pairs(t) do print(k,v) end"),
    ("ipairs", "ipairs(table) — iterate an array.", "for i,v in ipairs(t) do print(i,v) end"),
    ("local", "Declare a local variable.", "local name = UnitName(\"player\")"),
    ("if", "Conditional statement.", "if UnitExists(\"target\") then print(UnitName(\"target\")) end"),
];

pub fn command(name: &str) -> Option<&'static Reference> {
    COMMANDS.iter().find(|r| r.name.eq_ignore_ascii_case(name))
}
fn supports_conditions(name: &str) -> bool {
    !matches!(
        name,
        "/run"
            | "/script"
            | "/say"
            | "/s"
            | "/party"
            | "/p"
            | "/raid"
            | "/rw"
            | "/yell"
            | "/emote"
            | "/whisper"
            | "/w"
            | "/guild"
            | "/g"
            | "/console"
            | "/reload"
            | "/readycheck"
    ) && command(name).is_some()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    Command,
    Directive,
    Condition,
    Target,
    Operator,
    String,
    Number,
    Keyword,
    Function,
    Comment,
}
#[derive(Debug)]
pub struct Token {
    pub range: Range<usize>,
    pub kind: Kind,
}
fn token(tokens: &mut Vec<Token>, start: usize, end: usize, kind: Kind) {
    if start < end {
        tokens.push(Token {
            range: start..end,
            kind,
        });
    }
}
pub fn highlight(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let start = line.len() - trimmed.len();
        token(&mut tokens, offset, offset + start, Kind::Text);
        let end = start + trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
        let cmd = line[start..end].to_ascii_lowercase();
        let command_kind = if cmd.starts_with('#') {
            Kind::Directive
        } else if cmd.starts_with('/') {
            Kind::Command
        } else {
            Kind::Text
        };
        token(&mut tokens, offset + start, offset + end, command_kind);
        let lua = matches!(cmd.as_str(), "/run" | "/script");
        let conditional = supports_conditions(&cmd);
        let mut i = end;
        let mut in_group = false;
        while i < line.len() {
            let begin = i;
            let ch = line[i..].chars().next().unwrap();
            let kind;
            if lua && line[i..].starts_with("--") {
                i = line.len();
                kind = Kind::Comment;
            } else if lua && (ch == '\'' || ch == '"') {
                i += 1;
                let mut escaped = false;
                while i < line.len() {
                    let c = line[i..].chars().next().unwrap();
                    i += c.len_utf8();
                    if !escaped && c == ch {
                        break;
                    }
                    escaped = !escaped && c == '\\';
                }
                kind = Kind::String;
            } else if conditional && (ch == '[' || ch == ']') {
                in_group = ch == '[';
                i += 1;
                kind = Kind::Operator;
            } else if conditional && in_group && !ch.is_whitespace() && !",:/=]".contains(ch) {
                i += ch.len_utf8();
                while i < line.len()
                    && !line[i..].starts_with(|c: char| c.is_whitespace() || ",:/=]".contains(c))
                {
                    i += line[i..].chars().next().unwrap().len_utf8();
                }
                kind = if ch == '@' || &line[begin..i] == "target" {
                    Kind::Target
                } else if ch.is_ascii_digit() {
                    Kind::Number
                } else {
                    Kind::Condition
                };
            } else if lua && (ch.is_alphabetic() || ch == '_') {
                i += ch.len_utf8();
                while i < line.len()
                    && line[i..].starts_with(|c: char| c.is_alphanumeric() || "_.".contains(c))
                {
                    i += line[i..].chars().next().unwrap().len_utf8();
                }
                let word = &line[begin..i];
                kind = if [
                    "local", "if", "then", "else", "elseif", "end", "for", "while", "do", "repeat",
                    "until", "function", "return", "and", "or", "not", "true", "false", "nil",
                    "in", "break",
                ]
                .contains(&word)
                {
                    Kind::Keyword
                } else if line[i..].trim_start().starts_with('(') {
                    Kind::Function
                } else {
                    Kind::Text
                };
            } else if ch.is_ascii_digit() && (lua || conditional) {
                i += 1;
                while i < line.len()
                    && line[i..].starts_with(|c: char| c.is_ascii_digit() || c == '.')
                {
                    i += 1;
                }
                kind = Kind::Number;
            } else {
                i += ch.len_utf8();
                kind = if (lua && "(){}[]=;,+-*/<>~".contains(ch))
                    || (conditional && ";,!:=".contains(ch))
                {
                    Kind::Operator
                } else {
                    Kind::Text
                };
            }
            token(&mut tokens, offset + begin, offset + i, kind);
        }
        offset += line.len();
    }
    tokens
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}
#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub line: usize,
    pub column: usize,
    pub severity: Severity,
    pub message: String,
}
pub fn diagnose(name: &str, text: &str) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let mut add = |line, column, severity, message: String| {
        out.push(Diagnostic {
            line,
            column,
            severity,
            message,
        })
    };
    let length = text.chars().count();
    if length > BODY_LIMIT {
        add(1,1,Severity::Error,format!("{length} characters: {} over the standard {BODY_LIMIT}-character macro limit. Draft is preserved in full.",length-BODY_LIMIT));
    } else if text.len() > BODY_LIMIT {
        add(1,1,Severity::Warning,format!("{} UTF-8 bytes. Non-ASCII text may exceed the client's byte limit; verify after pasting.",text.len()));
    }
    if name.chars().count() > 16 {
        add(
            1,
            1,
            Severity::Warning,
            "Name exceeds 16 characters; shorten it for the in-game macro name field.".into(),
        );
    }
    if name.trim().is_empty() {
        add(1, 1, Severity::Warning, "Give this macro a name.".into());
    }
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        let cmd = trimmed
            .split_whitespace()
            .next()
            .unwrap()
            .to_ascii_lowercase();
        let line_no = index + 1;
        let base = line.len() - trimmed.len();
        if command(&cmd).is_none() {
            add(
                line_no,
                base + 1,
                Severity::Warning,
                if cmd.starts_with('/') {
                    format!("Unrecognized command {cmd}. It may be supplied by an addon or another client version.")
                } else if cmd.starts_with('#') {
                    format!("Unrecognized directive {cmd}. Expected #show or #showtooltip.")
                } else {
                    "This line has no slash command; WoW may send it as chat text.".into()
                },
            );
        }
        let args = trimmed[cmd.len()..].trim();
        if [
            "/cast",
            "/use",
            "/castsequence",
            "/castrandom",
            "/userandom",
            "/equip",
            "/equipslot",
            "/cancelaura",
            "/run",
            "/script",
        ]
        .contains(&cmd.as_str())
            && args.is_empty()
        {
            add(
                line_no,
                base + cmd.len() + 1,
                Severity::Warning,
                format!("{cmd} needs an argument."),
            );
        }
        if supports_conditions(&cmd) {
            let mut opening = None;
            for (i, ch) in line.char_indices() {
                match ch {
                    '[' => {
                        if opening.is_some() {
                            add(
                                line_no,
                                line[..i].chars().count() + 1,
                                Severity::Error,
                                "Nested conditional brackets are not supported.".into(),
                            );
                        }
                        opening = Some(i);
                    }
                    ']' => {
                        if let Some(start) = opening.take() {
                            let group = &line[start + 1..i];
                            for condition in
                                group.split(',').map(str::trim).filter(|s| !s.is_empty())
                            {
                                if condition.starts_with('@') || condition.starts_with("target=") {
                                    continue;
                                }
                                let key = condition.split(':').next().unwrap().to_ascii_lowercase();
                                let positive = key.strip_prefix("no").unwrap_or(&key);
                                if !CONDITIONS
                                    .iter()
                                    .any(|r| r.name == key || r.name == positive)
                                {
                                    add(line_no,line[..start].chars().count()+2,Severity::Warning,format!("Unrecognized condition '{key}'. Check the spelling and client support."));
                                }
                                if condition.ends_with(':') {
                                    add(
                                        line_no,
                                        line[..start].chars().count() + 2,
                                        Severity::Error,
                                        format!("'{condition}' needs a value after the colon."),
                                    );
                                }
                            }
                        } else {
                            add(
                                line_no,
                                line[..i].chars().count() + 1,
                                Severity::Error,
                                "Closing ] has no matching [.".into(),
                            );
                        }
                    }
                    _ => {}
                }
            }
            if let Some(start) = opening {
                add(
                    line_no,
                    line[..start].chars().count() + 1,
                    Severity::Error,
                    "Unclosed conditional group: add ].".into(),
                );
            }
        }
        if matches!(cmd.as_str(), "/run" | "/script") {
            let spans = highlight(line);
            let mut stack = Vec::new();
            for span in spans.iter().filter(|s| s.kind == Kind::Operator) {
                let ch = line[span.range.clone()].chars().next().unwrap();
                if "({[".contains(ch) {
                    stack.push((ch, span.range.start));
                } else if ")}]".contains(ch) {
                    let expected = match ch {
                        ')' => '(',
                        '}' => '{',
                        _ => '[',
                    };
                    if stack.pop().is_none_or(|(c, _)| c != expected) {
                        add(
                            line_no,
                            line[..span.range.start].chars().count() + 1,
                            Severity::Error,
                            format!("Unmatched Lua delimiter '{ch}'."),
                        );
                    }
                }
            }
            for (ch, pos) in stack {
                add(
                    line_no,
                    line[..pos].chars().count() + 1,
                    Severity::Error,
                    format!("Unclosed Lua delimiter '{ch}'."),
                );
            }
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Spell,
    Item,
}
pub struct Symbol {
    pub name: String,
    pub folded: String,
    pub id: i64,
    pub kind: SymbolKind,
}
#[derive(Default)]
pub struct Catalogue {
    pub symbols: Vec<Symbol>,
}
#[derive(Clone, Debug)]
pub struct Completion {
    pub label: String,
    pub insert: String,
    pub detail: String,
    /// UTF-8 byte offsets in the original document.
    pub range: Range<usize>,
    pub kind: &'static str,
}
fn suggestions(
    refs: &[Reference],
    query: &str,
    range: Range<usize>,
    kind: &'static str,
) -> Vec<Completion> {
    let query = query.to_lowercase();
    refs.iter()
        .filter(|r| r.name.to_lowercase().starts_with(&query))
        .map(|r| Completion {
            label: r.name.into(),
            insert: r.name.into(),
            detail: format!("{}\n{}", r.description, r.example),
            range: range.clone(),
            kind,
        })
        .collect()
}
pub fn complete(text: &str, cursor: usize, catalogue: &Catalogue) -> Vec<Completion> {
    let cursor = byte_index(text, cursor);
    let line_start = text[..cursor].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[cursor..].find('\n').map_or(text.len(), |i| cursor + i);
    let line = &text[line_start..line_end];
    let prefix = &text[line_start..cursor];
    let leading = line.len() - line.trim_start().len();
    let command_end = leading
        + line
            .trim_start()
            .find(char::is_whitespace)
            .unwrap_or(line.trim_start().len());
    if cursor - line_start <= command_end {
        let start = (line_start + leading).min(cursor);
        return suggestions(
            COMMANDS,
            &text[start..cursor],
            start..line_start + command_end,
            "command",
        );
    }
    let cmd = line[leading..command_end].to_ascii_lowercase();
    if matches!(cmd.as_str(), "/run" | "/script") {
        let spans = highlight(&text[line_start..cursor]);
        if spans
            .last()
            .is_some_and(|s| matches!(s.kind, Kind::String | Kind::Comment))
        {
            return vec![];
        }
        let start = prefix
            .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
            .map_or(line_start, |i| line_start + i + 1);
        let end = text[cursor..line_end]
            .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
            .map_or(line_end, |i| cursor + i);
        return suggestions(LUA, &text[start..cursor], start..end, "Lua");
    }
    if !supports_conditions(&cmd) {
        return vec![];
    }
    if let Some(open) = prefix.rfind('[').filter(|&i| !prefix[i..].contains(']')) {
        let start = prefix[open + 1..]
            .rfind(',')
            .map_or(line_start + open + 1, |i| line_start + open + 2 + i);
        let start = start + text[start..cursor].len() - text[start..cursor].trim_start().len();
        let end = text[cursor..line_end]
            .find([',', ']'])
            .map_or(line_end, |i| cursor + i);
        let query = &text[start..cursor];
        if let Some((key, value)) = query.split_once(':') {
            let values: &[&str] = match key.trim_start_matches("no") {
                "mod" | "modifier" => &["shift", "ctrl", "alt"],
                "btn" | "button" => &["1", "2", "3", "4", "5"],
                "stance" | "form" | "spec" | "actionbar" | "bonusbar" => {
                    &["0", "1", "2", "3", "4", "5", "6"]
                }
                "group" => &["party", "raid"],
                _ => &[],
            };
            let part = value.rsplit('/').next().unwrap_or(value);
            let value_start = cursor - part.len();
            return values
                .iter()
                .filter(|s| s.starts_with(part))
                .map(|s| Completion {
                    label: (*s).into(),
                    insert: (*s).into(),
                    detail: format!("{key}:{s}"),
                    range: value_start..end,
                    kind: "value",
                })
                .collect();
        }
        if let Some(unit) = query.strip_prefix("target=") {
            return TARGETS
                .iter()
                .filter(|r| r.name[1..].starts_with(unit))
                .map(|r| Completion {
                    label: format!("target={}", &r.name[1..]),
                    insert: format!("target={}", &r.name[1..]),
                    detail: r.description.into(),
                    range: start..end,
                    kind: "unit",
                })
                .collect();
        }
        let mut matches = suggestions(CONDITIONS, query, start..end, "condition");
        matches.extend(suggestions(TARGETS, query, start..end, "unit"));
        if query.starts_with("no") {
            for r in CONDITIONS.iter().filter(|r| !r.name.starts_with("no")) {
                let negative = format!("no{}", r.name);
                if negative.starts_with(query) && !matches.iter().any(|c| c.label == negative) {
                    matches.push(Completion {
                        label: negative.clone(),
                        insert: negative,
                        detail: format!("Negate: {}", r.description),
                        range: start..end,
                        kind: "condition",
                    });
                }
            }
        }
        return matches;
    }
    let sequence = matches!(cmd.as_str(), "/castsequence" | "/castrandom" | "/userandom");
    let args_start = line_start + command_end;
    let mut start = text[args_start..cursor]
        .rfind(|c| c == ']' || c == ';' || (sequence && c == ','))
        .map_or(args_start, |i| args_start + i + 1);
    start += text[start..cursor].len() - text[start..cursor].trim_start().len();
    if text[start..cursor].starts_with("reset=") {
        if let Some(space) = text[start..cursor].find(char::is_whitespace) {
            start += space + 1;
        } else {
            return [
                "reset=target ",
                "reset=combat ",
                "reset=target/10 ",
                "reset=10 ",
            ]
            .iter()
            .filter(|s| s.starts_with(&text[start..cursor]))
            .map(|s| Completion {
                label: s.trim().into(),
                insert: (*s).into(),
                detail:
                    "Reset the sequence after a target/combat change or this many idle seconds."
                        .into(),
                range: start..cursor,
                kind: "sequence",
            })
            .collect();
        }
    }
    if cmd == "/equipslot" {
        if let Some(space) = text[start..cursor].find(char::is_whitespace) {
            start += space + 1;
        } else {
            return vec![];
        }
    }
    start += text[start..cursor].len() - text[start..cursor].trim_start().len();
    if text[start..cursor].starts_with('!') {
        start += 1;
    }
    let end = text[cursor..line_end]
        .find(|c| c == ';' || (sequence && c == ','))
        .map_or(line_end, |i| cursor + i);
    let end = cursor + text[cursor..end].trim_end().len();
    let query = text[start..cursor].to_lowercase();
    let items = matches!(
        cmd.as_str(),
        "/use" | "/userandom" | "/equip" | "/equipslot"
    );
    let spells = matches!(
        cmd.as_str(),
        "/cast"
            | "/castsequence"
            | "/castrandom"
            | "/cancelaura"
            | "/use"
            | "#show"
            | "#showtooltip"
            | "/petautocaston"
            | "/petautocastoff"
            | "/petautocasttoggle"
    );
    let mut found: Vec<_> = catalogue
        .symbols
        .iter()
        .filter(|s| {
            ((items && s.kind == SymbolKind::Item) || (spells && s.kind == SymbolKind::Spell))
                && (s.folded.contains(&query) || (s.id > 0 && s.id.to_string() == query))
        })
        .collect();
    found.sort_by_key(|s| (!s.folded.starts_with(&query), s.name.len(), &s.name));
    found
        .into_iter()
        .take(30)
        .map(|s| Completion {
            label: s.name.clone(),
            insert: s.name.clone(),
            detail: format!(
                "{}{}. Reference name; check availability in your character's spellbook.",
                if s.kind == SymbolKind::Spell {
                    "Spell"
                } else {
                    "Item"
                },
                if s.id > 0 {
                    format!(" #{}", s.id)
                } else {
                    String::new()
                }
            ),
            range: start..end,
            kind: if s.kind == SymbolKind::Spell {
                "spell"
            } else {
                "item"
            },
        })
        .collect()
}

/// Search returns original UTF-8 ranges, including when lowercasing expands a character.
pub fn find_matches(text: &str, query: &str, match_case: bool) -> Vec<Range<usize>> {
    if query.is_empty() {
        return Vec::new();
    }
    if match_case {
        return text
            .match_indices(query)
            .map(|(i, s)| i..i + s.len())
            .collect();
    }
    let count = query.chars().count();
    let query = query.to_lowercase();
    let boundaries: Vec<_> = text
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(text.len()))
        .collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i + count < boundaries.len() {
        let range = boundaries[i]..boundaries[i + count];
        if text[range.clone()].to_lowercase() == query {
            found.push(range);
            i += count;
        } else {
            i += 1;
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    fn catalogue() -> Catalogue {
        Catalogue {
            symbols: vec![
                Symbol {
                    name: "Éclair de givre".into(),
                    folded: "éclair de givre".into(),
                    id: 1,
                    kind: SymbolKind::Spell,
                },
                Symbol {
                    name: "Fireball".into(),
                    folded: "fireball".into(),
                    id: 133,
                    kind: SymbolKind::Spell,
                },
                Symbol {
                    name: "Healing Potion".into(),
                    folded: "healing potion".into(),
                    id: 929,
                    kind: SymbolKind::Item,
                },
            ],
        }
    }
    #[test]
    fn completion_uses_context_and_replaces_whole_tokens() {
        for (text, needle) in [
            ("/ca", "/cast"),
            ("/cast [@m", "@mouseover"),
            ("/cast [mod:s", "shift"),
            ("/cast [noha", "noharm"),
            ("/cast [target=mo", "target=mouseover"),
            ("/cast [help][] Fi", "Fireball"),
            (
                "/castsequence reset=target/10 Fireball, Éc",
                "Éclair de givre",
            ),
            ("/use Healing P", "Healing Potion"),
            ("/run print(Uni", "UnitName"),
        ] {
            let options = complete(text, text.chars().count(), &catalogue());
            assert!(
                options.iter().any(|c| c.label == needle),
                "{text}: {options:?}"
            );
        }
        let text = "/cast Fireball";
        let c = complete(text, 8, &catalogue()).remove(0);
        let mut body = text.to_owned();
        body.replace_range(c.range, &c.insert);
        assert_eq!(body, text);
        assert!(complete("/say [Hi]", 9, &catalogue()).is_empty());
        assert!(complete("/run print(\"Uni", 15, &catalogue()).is_empty());
    }
    #[test]
    fn diagnostics_distinguish_chat_lua_and_conditionals() {
        for text in [
            "/say [hello",
            "/run print(\"[hi]\")",
            "/cast [@mouseover,help,nodead][] Fireball",
            "/castsequence reset=target/10 Fireball, Fireball",
        ] {
            assert!(diagnose("Example", text).is_empty(), "{text}");
        }
        assert!(diagnose("Test", "/cast [mod:] Fireball")
            .iter()
            .any(|d| d.severity == Severity::Error));
        assert!(diagnose("Test", "/cast [harm Fireball")
            .iter()
            .any(|d| d.message.contains("Unclosed")));
        assert!(diagnose("Test", "/run print(42")
            .iter()
            .any(|d| d.message.contains("Lua")));
        assert!(diagnose("Test", "/addoncmd test")
            .iter()
            .all(|d| d.severity == Severity::Warning));
        assert!(diagnose("Test", &"x".repeat(256))
            .iter()
            .any(|d| d.message.contains("limit")));
    }
    #[test]
    fn lexer_preserves_unicode_text_and_lua_strings() {
        let text = "#showtooltip\n/cast [@mouseover,help] Éclair\n/run local x=\"[a]\\\"\"; print(x) -- comment";
        let spans = highlight(text);
        assert_eq!(
            spans
                .iter()
                .map(|s| &text[s.range.clone()])
                .collect::<String>(),
            text
        );
        assert!(spans.iter().any(|s| s.kind == Kind::String));
        assert!(spans.iter().any(|s| s.kind == Kind::Comment));
        assert!(spans.iter().any(|s| s.kind == Kind::Target));
    }
    #[test]
    fn search_keeps_unicode_offsets_and_nonoverlapping_replacements() {
        let text = "İ test Éclair éclair";
        let hits = find_matches(text, "éclair", false);
        assert_eq!(hits.len(), 2);
        assert_eq!(&text[hits[0].clone()], "Éclair");
        assert_eq!(find_matches("aaaa", "aa", false), vec![0..2, 2..4]);
        assert!(find_matches("text", "", false).is_empty());
    }
}
