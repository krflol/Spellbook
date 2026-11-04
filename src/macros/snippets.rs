//! Editable examples, not live-character spellbook or combat-state validation.
use super::MacroDocument;

pub struct Snippet {
    pub name: &'static str,
    pub class: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub example: &'static str,
}
impl Snippet {
    pub fn guidance(&self) -> Option<&'static str> {
        match self.category {
            "Stance dancing" => Some("Numbered examples use Battle=1, Defensive=2, Berserker=3. Confirm your slots with Inspect forms. A /cast branch chooses one action per keypress; stance changes can cost rage and still respect cooldowns."),
            "Druid forms" => Some("Numbered examples assume Bear=1 and Cat=3. Learned forms can change these slots: run Inspect forms in game and adjust [form:N] / [noform:N]. Shifting still needs mana and respects cooldowns."),
            _ => None,
        }
    }
    pub fn document(&self) -> MacroDocument {
        MacroDocument {
            name: self.name.into(),
            body: self.example.into(),
            class: self.class.into(),
            tags: self.category.to_lowercase(),
            notes: match self.guidance() {
                Some(guidance) => format!("{}\n\n{guidance}", self.description),
                None => self.description.into(),
            },
            ..Default::default()
        }
    }
    pub fn matches(&self, query: &str, class: &str) -> bool {
        (class == "All classes" || self.class == class || self.class == "Any class")
            && (query.trim().is_empty()
                || format!(
                    "{} {} {} {} {}",
                    self.name, self.class, self.category, self.description, self.example
                )
                .to_lowercase()
                .contains(&query.trim().to_lowercase()))
    }
}

macro_rules! snippets {
    ($(($name:literal,$class:literal,$category:literal,$description:literal,$example:literal)),* $(,)?) => {
        &[$(Snippet { name:$name, class:$class, category:$category, description:$description, example:$example }),*]
    }
}
pub const SNIPPETS: &[Snippet] = snippets![
    ("Mouseover heal", "Priest", "Essentials", "Try a living friendly mouseover, then the default target. Replace the spell for your class.", "#showtooltip\n/cast [@mouseover,help,nodead][] Flash Heal"),
    ("Focus interrupt", "Mage", "Essentials", "Interrupt a living hostile focus, with your current target as fallback.", "#showtooltip Counterspell\n/stopcasting\n/cast [@focus,harm,nodead][] Counterspell"),
    ("Modifier cast", "Mage", "Essentials", "Hold Shift for Frostbolt; otherwise cast Fireball.", "#showtooltip\n/cast [mod:shift] Frostbolt; Fireball"),
    ("Mouseover debuff", "Mage", "Essentials", "Use a living hostile mouseover, then your default target.", "#showtooltip\n/cast [@mouseover,harm,nodead][] Polymorph"),
    ("Self cast", "Priest", "Essentials", "Cast directly on yourself.", "#showtooltip\n/cast [@player] Power Word: Shield"),
    ("Ground target", "Mage", "Essentials", "Place a supported ground-targeted spell at the cursor.", "#showtooltip\n/cast [@cursor] Blizzard"),
    ("Target sequence", "Warlock", "Essentials", "Advance on each successful press; reset on target change or after the idle interval.", "#showtooltip\n/castsequence reset=target/10 Corruption, Curse of Agony"),
    ("Trinkets", "Any class", "Essentials", "Attempt to activate both equipped trinkets; shared cooldowns still apply.", "#showtooltip\n/use 13\n/use 14"),
    ("Pet attack", "Any class", "Essentials", "Send your pet to your living hostile target.", "/petattack [@target,harm,nodead]"),
    ("Mouseover focus", "Any class", "Essentials", "Set focus from mouseover, falling back to the current target.", "/focus [@mouseover,exists][]"),

    ("Stance selector", "Warrior", "Stance dancing", "No modifier: Battle Stance. Shift: Defensive Stance. Ctrl: Berserker Stance. Ctrl takes priority if both keys are held. Put the macro on a key available in every stance.", "#showtooltip\n/cast [mod:ctrl] Berserker Stance; [mod:shift] Defensive Stance; Battle Stance"),
    ("Charge dance", "Warrior", "Stance dancing", "If needed, the first press selects Battle Stance; a later press attempts Charge. This does not bypass Charge's range, combat, or cooldown requirements.", "#showtooltip Charge\n/cast [nostance:1] Battle Stance; Charge"),
    ("Overpower dance", "Warrior", "Stance dancing", "Select Battle Stance when needed, then press again for Overpower. The macro does not check whether Overpower is available or whether you have enough rage.", "#showtooltip Overpower\n/cast [nostance:1] Battle Stance; Overpower"),
    ("Taunt dance", "Warrior", "Stance dancing", "Select Defensive Stance when needed. Once there, taunt a living hostile mouseover or your current target. A stance change and the taunt use separate presses.", "#showtooltip Taunt\n/cast [nostance:2] Defensive Stance; [@mouseover,harm,nodead][] Taunt"),
    ("Pummel dance", "Warrior", "Stance dancing", "Select Berserker Stance when needed, then press again to Pummel a living hostile focus or your current target. Requires Pummel and enough rage.", "#showtooltip Pummel\n/cast [nostance:3] Berserker Stance; [@focus,harm,nodead][] Pummel"),
    ("Charge/Intercept", "Warrior", "Stance dancing", "Out of combat, choose Battle Stance then Charge. In combat, choose Berserker Stance then Intercept. Each press chooses one action; it does not pick based on range or cooldown. Requires both abilities.", "#showtooltip [combat] Intercept; Charge\n/cast [nocombat,nostance:1] Battle Stance; [combat,nostance:3] Berserker Stance; [nocombat] Charge; Intercept"),
    ("Berserker Rage", "Warrior", "Stance dancing", "Select Berserker Stance when needed; press again for Berserker Rage. Remains in Berserker Stance afterward, so returning to another stance is your choice.", "#showtooltip Berserker Rage\n/cast [nostance:3] Berserker Stance; Berserker Rage"),
    ("Stance interrupt", "Warrior", "Stance dancing", "Use Pummel in Berserker Stance, or Shield Bash in Battle/Defensive with a shield equipped. This sample preserves your stance and does not equip a shield for you.", "#showtooltip\n/cast [stance:3] Pummel; [stance:1/2,equipped:Shields] Shield Bash"),
    ("Stance cooldown", "Warrior", "Stance dancing", "Use Retaliation in Battle, Shield Wall in Defensive, or Recklessness in Berserker. Preserves your stance; you must know the selected ability and meet its equipment/cooldown requirements.", "#showtooltip\n/cast [stance:1] Retaliation; [stance:2] Shield Wall; [stance:3] Recklessness"),

    ("Cat form", "Druid", "Druid forms", "Enter Cat Form without toggling it off on repeated presses. The ! prefix prevents toggling off; it does not perform a powershift.", "#showtooltip Cat Form\n/cast !Cat Form"),
    ("Bear form", "Druid", "Druid forms", "Enter Bear Form without toggling it off on repeated presses. Replace Bear Form with Dire Bear Form in both lines if that is the version you use.", "#showtooltip Bear Form\n/cast !Bear Form"),
    ("Travel forms", "Druid", "Druid forms", "Swimming selects Aquatic Form before the outdoor check; otherwise outdoors selects Travel Form and indoors selects Cat Form. Learn each selected form first. The ! prefix prevents toggling it off.", "#showtooltip\n/cast [swimming] !Aquatic Form; [outdoors] !Travel Form; !Cat Form"),
    ("Bear/Cat switch", "Druid", "Druid forms", "From Bear (slot 1), select Cat; from other states, select Bear. Replace Bear Form with Dire Bear Form if appropriate. Check the Bear slot with Inspect forms.", "#showtooltip\n/cast [form:1] !Cat Form; !Bear Form"),
    ("Cat Prowl", "Druid", "Druid forms", "Select Cat Form if needed, then press again to Prowl while out of combat and not already stealthed. Repeated presses keep Prowl active. Assumes Cat is slot 3.", "#showtooltip Prowl\n/cast [noform:3] !Cat Form; [nocombat,nostealth] !Prowl"),
    ("Cat Dash", "Druid", "Druid forms", "Select Cat Form if needed; a later press attempts Dash. Assumes Cat is slot 3. Shifting and Dash still respect their own costs and cooldowns.", "#showtooltip Dash\n/cast [noform:3] !Cat Form; Dash"),
    ("Cat powershift", "Druid", "Druid forms", "Deliberately leave your current form, then attempt Cat Form. A cooldown or insufficient mana can leave you in caster form; this macro cannot test those conditions before canceling. It always returns to Cat, not the previous form.", "#showtooltip Cat Form\n/cancelform [form]\n/cast Cat Form"),
    ("Caster heal", "Druid", "Druid forms", "Leave form, then cast Rejuvenation on a living friendly mouseover, a living friendly target, or yourself. Stays in caster form afterward. The heal still needs mana and an available global cooldown.", "#showtooltip Rejuvenation\n/cancelform [form]\n/cast [@mouseover,help,nodead][help,nodead][@player] Rejuvenation"),
    ("Form attack", "Druid", "Druid forms", "Bear (slot 1): Maul. Cat (slot 3): Claw. Caster (no form): Wrath. Other forms do nothing. Maul queues a melee swing; the macro does not change your form or choose a rotation.", "#showtooltip\n/cast [form:1] Maul; [form:3] Claw; [noform] Wrath"),
    ("Moonkin form", "Druid", "Druid forms", "Enter Moonkin Form without toggling it off on repeated presses. Requires the talent/form to be learned; no numbered form slot is needed.", "#showtooltip Moonkin Form\n/cast !Moonkin Form"),
    ("Cat/caster key", "Druid", "Druid forms", "No Shift: enter Cat Form and stay there. Hold Shift: leave your current form and stop the macro, so it cannot immediately re-enter Cat. This sample does not depend on form-slot numbering.", "#showtooltip Cat Form\n/cancelform [mod:shift,form]\n/stopmacro [mod:shift]\n/cast !Cat Form"),

    ("Ghost Wolf", "Shaman", "Forms & stealth", "Enter Ghost Wolf without toggling it off on repeated presses. Cast time and location restrictions still depend on your character and the client.", "#showtooltip Ghost Wolf\n/cast !Ghost Wolf"),
    ("Shadowform heal", "Priest", "Forms & stealth", "Cancel Shadowform, then Flash Heal a living friendly mouseover, a living friendly target, or yourself. This does not re-enter Shadowform after healing.", "#showtooltip Flash Heal\n/cancelaura Shadowform\n/cast [@mouseover,help,nodead][help,nodead][@player] Flash Heal"),
    ("Stealth opener", "Rogue", "Forms & stealth", "While stealthed, attempt Ambush; otherwise use Sinister Strike. Ambush's weapon, position, and energy requirements still apply. Omits /startattack so a failed opener does not start auto-attacking.", "#showtooltip\n/cast [stealth] Ambush; Sinister Strike"),
    ("Stealth entry", "Rogue", "Forms & stealth", "Enter Stealth out of combat when not already stealthed. Repeated presses do not cancel Stealth; the ability's cooldown still applies.", "#showtooltip Stealth\n/cast [nocombat,nostealth] !Stealth"),

    ("Print location", "Any class", "Utilities", "Print your current map ID using the client API.", "/run print(C_Map.GetBestMapForUnit(\"player\"))"),
    ("Inspect forms", "Any class", "Utilities", "Run in WoW to print each learned stance/form's slot, spell ID, and name; * marks the active form. Use the first number in [form:N] or [stance:N], not the spell ID. Caster/no form is 0.", "/run for i=1,GetNumShapeshiftForms() do local _,a,_,s=GetShapeshiftFormInfo(i); print(i,s,s and C_Spell.GetSpellName(s) or \"?\",a and \"*\" or \"\") end"),
];
