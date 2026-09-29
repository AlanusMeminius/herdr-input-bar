# herdr-input-bar

A herdr plugin that gives a terminal a dedicated place to compose text and submit it.

## Language

**Input Bar**:
A persistent, text-only composer that sits directly below one Target Pane and submits what the user writes into it.
_Avoid_: input box, prompt box, command line

**Target Pane**:
The single terminal an Input Bar is bound to at the moment it opens; every Submission goes there, regardless of later focus changes.
_Avoid_: current pane, focused pane, destination

**Draft**:
The text currently being composed in an Input Bar, possibly spanning multiple lines. Submitting an empty Draft delivers a bare Enter.
_Avoid_: buffer, input

**Submission**:
Delivering a Draft to the Target Pane together with a confirming Enter, after which the Draft is cleared.
_Avoid_: send, paste

**Agent Target**:
A Target Pane that herdr recognizes as running an AI agent; Submissions to it are delivered as agent prompts rather than raw keystrokes.

**Shell Target**:
A Target Pane that is not an Agent Target; Submissions to it are delivered as literal text followed by Enter.

**Rejected Submission**:
A Submission the Target Pane refused (e.g. the agent is waiting on an approval); the Draft is kept and the reason is shown in the Input Bar.

**History**:
The Submissions made from one Input Bar during its life, recallable into the Draft; not kept after the Input Bar closes.

## Relationships

- An **Input Bar** is bound to exactly one **Target Pane** for its whole life
- A **Target Pane** has at most one **Input Bar**; opening again refocuses the existing one
- When the **Target Pane** closes, its **Input Bar** closes too
- Leaving an **Input Bar** (returning focus to the **Target Pane**) keeps it open and keeps its **Draft**
- A **Submission** carries one **Draft** to one **Target Pane**
- Whether a **Target Pane** is an **Agent Target** or a **Shell Target** is decided at Submission time, not at open time

## Flagged ambiguities

- "Vim-like" meant a Vim command-line style single-purpose text entry, not modal (Normal/Insert) editing.
