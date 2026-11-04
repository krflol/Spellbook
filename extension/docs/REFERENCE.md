# Spellbook macro reference

Use Ctrl+Space for context-aware completions. Problems checks structure; it does not execute Lua or simulate gameplay. Samples target Forever / Classic-style mechanics. Verify learned form slots and available spells in game.

## Commands

### #showtooltip

Choose the action button's tooltip and icon. With no argument, infer them from the macro.

```wowmacro
#showtooltip [mod:shift] Frostbolt; Fireball
```

### #show

Choose the action button's icon and count.

```wowmacro
#show Healing Potion
```

### /cast

Cast a spell. Conditional groups are checked from left to right.

```wowmacro
/cast [@mouseover,help,nodead][] Flash Heal
```

### /use

Use an item by name, item ID, equipment slot, or bag and slot; also accepts spells.

```wowmacro
/use [combat] 13
```

### /castsequence

Advance a sequence on successful casts. Each press attempts one action. reset= is optional.

```wowmacro
/castsequence reset=target/10 Corruption, Curse of Agony
```

### /castrandom

Choose an action from a comma-separated list.

```wowmacro
/castrandom Fireball, Frostbolt
```

### /userandom

Choose an item from a comma-separated list.

```wowmacro
/userandom Healing Potion, Lesser Healing Potion
```

### /stopcasting

Stop the current cast before processing subsequent macro lines.

```wowmacro
/stopcasting
```

### /stopmacro

Stop processing the macro when the conditions match.

```wowmacro
/stopmacro [noexists][dead]
```

### /cancelaura

Remove a named cancellable buff.

```wowmacro
/cancelaura Ice Block
```

### /cancelform

Leave the current shapeshift form.

```wowmacro
/cancelform [form]
```

### /startattack

Start auto-attacking the target.

```wowmacro
/startattack [harm,nodead]
```

### /stopattack

Stop auto-attacking.

```wowmacro
/stopattack
```

### /target

Select a unit by name or unit token.

```wowmacro
/target [@mouseover,exists]
```

### /tar

Short form of /target.

```wowmacro
/tar Hogger
```

### /targetexact

Select a unit whose name matches exactly.

```wowmacro
/targetexact Hogger
```

### /targetenemy

Cycle to an enemy target.

```wowmacro
/targetenemy [noharm][dead]
```

### /targetfriend

Cycle to a friendly target.

```wowmacro
/targetfriend
```

### /targetlasttarget

Return to the previous target.

```wowmacro
/targetlasttarget
```

### /targetlastenemy

Return to the previous enemy target.

```wowmacro
/targetlastenemy
```

### /targetlastfriend

Return to the previous friendly target.

```wowmacro
/targetlastfriend
```

### /cleartarget

Clear the current target.

```wowmacro
/cleartarget [dead]
```

### /focus

Set the focus unit.

```wowmacro
/focus [@mouseover,exists][]
```

### /clearfocus

Clear the focus unit.

```wowmacro
/clearfocus [@focus,dead]
```

### /assist

Select another unit's target.

```wowmacro
/assist [@focus,exists]
```

### /petattack

Send your pet to attack.

```wowmacro
/petattack [@target,harm,nodead]
```

### /petfollow

Call your pet back to follow.

```wowmacro
/petfollow
```

### /petstay

Tell your pet to stay.

```wowmacro
/petstay
```

### /petpassive

Set your pet to passive.

```wowmacro
/petpassive
```

### /petdefensive

Set your pet to defensive.

```wowmacro
/petdefensive
```

### /petautocaston

Enable autocast for a named pet ability.

```wowmacro
/petautocaston Growl
```

### /petautocastoff

Disable autocast for a named pet ability.

```wowmacro
/petautocastoff Growl
```

### /petautocasttoggle

Toggle autocast for a named pet ability.

```wowmacro
/petautocasttoggle Growl
```

### /equip

Equip an item by name.

```wowmacro
/equip Worn Shortsword
```

### /equipslot

Equip an item into a numbered equipment slot.

```wowmacro
/equipslot 16 Worn Shortsword
```

### /equipset

Equip a saved equipment set if supported by the client.

```wowmacro
/equipset Tank
```

### /dismount

Dismount when the conditions match.

```wowmacro
/dismount [mounted]
```

### /leavevehicle

Leave the current vehicle.

```wowmacro
/leavevehicle
```

### /changeactionbar

Switch to a numbered action bar page.

```wowmacro
/changeactionbar 2
```

### /swapactionbar

Swap between two action bar pages.

```wowmacro
/swapactionbar 1 2
```

### /click

Click a named UI button, subject to the client's protected-action restrictions.

```wowmacro
/click CharacterMicroButton
```

### /run

One line of Lua. Syntax is highlighted; Spellbook does not execute or fully type-check Lua.

```wowmacro
/run print(UnitName("player"))
```

### /script

Alias of /run.

```wowmacro
/script print(GetTime())
```

### /console

Set a console variable.

```wowmacro
/console Sound_EnableMusic 0
```

### /say

Send text to nearby players. %t expands to your target's name.

```wowmacro
/say Hello, %t!
```

### /s

Short form of /say.

```wowmacro
/s Hello!
```

### /party

Send text to your party.

```wowmacro
/party Ready!
```

### /p

Short form of /party.

```wowmacro
/p Ready!
```

### /raid

Send text to your raid.

```wowmacro
/raid Ready!
```

### /rw

Send a raid warning when permitted.

```wowmacro
/rw Move away!
```

### /yell

Yell text to nearby players.

```wowmacro
/yell For the Alliance!
```

### /emote

Send a custom emote.

```wowmacro
/emote waves at %t.
```

### /whisper

Whisper a named player.

```wowmacro
/whisper Player Hello!
```

### /w

Short form of /whisper.

```wowmacro
/w Player Hello!
```

### /guild

Send text to your guild.

```wowmacro
/guild Hello!
```

### /g

Short form of /guild.

```wowmacro
/g Hello!
```

### /reload

Reload the game's interface.

```wowmacro
/reload
```

### /readycheck

Start a group ready check when permitted.

```wowmacro
/readycheck
```

### /clearworldmarker

Clear a numbered world marker when supported.

```wowmacro
/clearworldmarker 1
```

### /wm

Place a numbered world marker when supported.

```wowmacro
/wm 1
```

## Conditions

### help

The selected unit is friendly.

```wowmacro
[help,nodead]
```

### harm

The selected unit is hostile.

```wowmacro
[harm,nodead]
```

### exists

The selected unit exists.

```wowmacro
[@focus,exists]
```

### dead

The selected unit is dead.

```wowmacro
[dead]
```

### nodead

The selected unit is alive.

```wowmacro
[help,nodead]
```

### noexists

The selected unit does not exist.

```wowmacro
[@focus,noexists]
```

### combat

You are in combat.

```wowmacro
[combat]
```

### nocombat

You are out of combat.

```wowmacro
[nocombat]
```

### mod

A modifier key is held; optionally specify shift, ctrl, or alt.

```wowmacro
[mod:shift]
```

### nomod

No modifier key is held; optionally specify which modifier.

```wowmacro
[nomod]
```

### modifier

Long form of mod.

```wowmacro
[modifier:ctrl]
```

### btn

The macro was clicked with this mouse button number.

```wowmacro
[btn:2]
```

### button

Long form of btn.

```wowmacro
[button:2]
```

### stance

You are in a numbered stance; 0 means no stance/form.

```wowmacro
[stance:1/2]
```

### form

Alias of stance; numbering depends on your class and available forms.

```wowmacro
[form:1]
```

### stealth

You are stealthed.

```wowmacro
[stealth]
```

### nostealth

You are not stealthed.

```wowmacro
[nostealth]
```

### mounted

You are mounted.

```wowmacro
[mounted]
```

### nomounted

You are not mounted.

```wowmacro
[nomounted]
```

### swimming

You are swimming.

```wowmacro
[swimming]
```

### flying

You are flying, where supported.

```wowmacro
[flying]
```

### flyable

The area permits flying, where supported.

```wowmacro
[flyable]
```

### indoors

You are indoors.

```wowmacro
[indoors]
```

### outdoors

You are outdoors.

```wowmacro
[outdoors]
```

### group

You are in a group, optionally party or raid.

```wowmacro
[group:party]
```

### party

The selected unit belongs to your party.

```wowmacro
[party]
```

### raid

The selected unit belongs to your raid.

```wowmacro
[raid]
```

### pet

You have a pet; optionally specify a name or family.

```wowmacro
[pet]
```

### nopet

You have no active pet.

```wowmacro
[nopet]
```

### channeling

You are channeling, optionally a named spell.

```wowmacro
[nochanneling]
```

### nochanneling

You are not channeling, optionally a named spell.

```wowmacro
[nochanneling]
```

### equipped

You have an item or item type equipped.

```wowmacro
[equipped:Shields]
```

### worn

Alias of equipped.

```wowmacro
[worn:Shields]
```

### actionbar

A numbered action bar is active.

```wowmacro
[actionbar:1]
```

### spec

Your specialization index, when supported by the client.

```wowmacro
[spec:1]
```

### known

You know the spell ID or name, when supported by the client.

```wowmacro
[known:133]
```

### talent

A talent row/column is selected, when supported by the client.

```wowmacro
[talent:1/2]
```

### vehicleui

The vehicle interface is active.

```wowmacro
[vehicleui]
```

### possessbar

A possession bar is active.

```wowmacro
[possessbar]
```

### overridebar

An override action bar is active.

```wowmacro
[overridebar]
```

### bonusbar

A numbered bonus action bar is active.

```wowmacro
[bonusbar:1]
```

### unithasvehicleui

The selected unit has a vehicle UI.

```wowmacro
[unithasvehicleui]
```

### canexitvehicle

You can exit the current vehicle.

```wowmacro
[canexitvehicle]
```

## Targets

### @mouseover

Use the unit under your cursor without changing your target.

```wowmacro
[@mouseover,help,nodead]
```

### @target

Use your current target.

```wowmacro
[@target,harm,nodead]
```

### @player

Use yourself as the target.

```wowmacro
[@player]
```

### @focus

Use your focus unit.

```wowmacro
[@focus,harm,nodead]
```

### @targettarget

Use your target's target.

```wowmacro
[@targettarget,help]
```

### @focustarget

Use your focus's target.

```wowmacro
[@focustarget,harm]
```

### @pet

Use your pet.

```wowmacro
[@pet,exists]
```

### @pettarget

Use your pet's target.

```wowmacro
[@pettarget,harm]
```

### @cursor

Place a supported ground-targeted action at the cursor.

```wowmacro
[@cursor]
```

### @party1

Use the first other party member. Also party2 through party4.

```wowmacro
[@party1,help,nodead]
```

### @party2

Use the second other party member.

```wowmacro
[@party2,help,nodead]
```

### @party3

Use the third other party member.

```wowmacro
[@party3,help,nodead]
```

### @party4

Use the fourth other party member.

```wowmacro
[@party4,help,nodead]
```

### @raid1

Use raid member 1. Other raid indices may be used.

```wowmacro
[@raid1,help,nodead]
```

## Lua & API

### print

print(...) — print values to your chat frame.

```wowmacro
print(UnitName("player"))
```

### UnitName

UnitName(unit) — return a unit's name.

```wowmacro
UnitName("player")
```

### UnitClass

UnitClass(unit) — return a unit's class names and ID.

```wowmacro
UnitClass("player")
```

### UnitLevel

UnitLevel(unit) — return a unit's level.

```wowmacro
UnitLevel("player")
```

### UnitExists

UnitExists(unit) — test whether a unit exists.

```wowmacro
UnitExists("target")
```

### UnitGUID

UnitGUID(unit) — return a unit's GUID.

```wowmacro
UnitGUID("target")
```

### GetTime

GetTime() — return elapsed client time in seconds.

```wowmacro
GetTime()
```

### GetNumShapeshiftForms

GetNumShapeshiftForms() — count your learned stance/form slots.

```wowmacro
GetNumShapeshiftForms()
```

### GetShapeshiftFormInfo

GetShapeshiftFormInfo(index) — return icon, active, castable, and spell ID for a stance/form slot.

```wowmacro
GetShapeshiftFormInfo(1)
```

### C_Spell.GetSpellName

C_Spell.GetSpellName(spellID) — look up the name of a client spell; may return nil.

```wowmacro
C_Spell.GetSpellName(768)
```

### GetMoney

GetMoney() — return your money in copper.

```wowmacro
GetMoney()
```

### GetCVar

GetCVar(name) — read a console variable.

```wowmacro
GetCVar("Sound_EnableMusic")
```

### SetCVar

SetCVar(name, value) — set a console variable.

```wowmacro
SetCVar("Sound_EnableMusic", 0)
```

### IsInGroup

IsInGroup() — test group membership.

```wowmacro
IsInGroup()
```

### IsInRaid

IsInRaid() — test raid membership.

```wowmacro
IsInRaid()
```

### GetNumGroupMembers

GetNumGroupMembers() — return group size.

```wowmacro
GetNumGroupMembers()
```

### SendChatMessage

SendChatMessage(text, type, language, target) — send chat, subject to client restrictions.

```wowmacro
SendChatMessage("Hello", "PARTY")
```

### PlaySound

PlaySound(soundKitID) — play a sound kit.

```wowmacro
PlaySound(12867)
```

### C_Map.GetBestMapForUnit

C_Map.GetBestMapForUnit(unit) — return the best map ID for a unit.

```wowmacro
C_Map.GetBestMapForUnit("player")
```

### string.format

string.format(format, ...) — format a string.

```wowmacro
string.format("%s", UnitName("player"))
```

### string.find

string.find(text, pattern) — find a Lua pattern.

```wowmacro
string.find("hello", "ell")
```

### string.len

string.len(text) — count bytes in a string.

```wowmacro
string.len("hello")
```

### table.concat

table.concat(values, separator) — join a list.

```wowmacro
table.concat({1, 2}, ", ")
```

### tostring

tostring(value) — convert a value to text.

```wowmacro
tostring(42)
```

### tonumber

tonumber(value) — convert text to a number.

```wowmacro
tonumber("42")
```

### pairs

pairs(table) — iterate a table.

```wowmacro
for k,v in pairs(t) do print(k,v) end
```

### ipairs

ipairs(table) — iterate an array.

```wowmacro
for i,v in ipairs(t) do print(i,v) end
```

### local

Declare a local variable.

```wowmacro
local name = UnitName("player")
```

### if

Conditional statement.

```wowmacro
if UnitExists("target") then print(UnitName("target")) end
```
