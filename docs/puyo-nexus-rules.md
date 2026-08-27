# Puyo Puyo rules — a local copy of the Puyo Nexus wiki

A vendored snapshot of every page of the [Puyo Nexus Wiki](https://puyonexus.com/wiki/) that
carries a *rule*, scraped on **2026-08-27** so that the rules of the game being cloned in
[`puyo-rusto/`](../puyo-rusto) are available to read in full, offline, without a browser.

**This file is temporary.** It is scaffolding for the phases that implement the rules, and it
comes out again once they are done — so lean on it while writing `puyo-rusto`, but do not build
anything on it that would miss it when it goes. Cite the *live* page in code comments, as the
modules already do; this copy is for finding the rule, not for being the record of it.

## Why this exists

Puyo Nexus rejects automated fetches, so [the plan](puyo-puyo-plan.md) told each agent to read
the pages in a browser. That works, but it only finds the pages you think to look for. Phase 1
implemented the hidden thirteenth row as an ordinary row, because the rule that a **ghost puyo
cannot pop** is not on any page in `Category:Rules` — it is three paragraphs inside a page
filed under *Gameplay Guides*. Alex found it; the code was wrong until then.

So this file is deliberately **wider than the category**. If you are implementing a rule, search
here first, and search for the mechanic rather than the page you expect it to be on.

## How to use it

* **Search it, do not read it.** It is a quarter of a megabyte. Every section carries the URL it
  came from; that page, not this copy, is the authority.
* **It is a snapshot.** The wiki is edited. If something here disagrees with the live page, the
  live page wins — and this file should be re-scraped.
* **The "areas of interest" list in *Reverse Engineering (index)* is the list of known
  unknowns.** It is how phase 1 established that the nuisance scatter pattern is undocumented
  rather than merely unfound, which is worth knowing before guessing at something.

## What is here, and what is not

Twenty-five pages: the twelve in `Category:Rules`, three more that carry rules but are filed
elsewhere, and ten from the Mega Drive/arcade reverse-engineering effort, which is where the
exact algorithms and frame timings live.

Left out on purpose: the reverse-engineering pages about the ROM rather than the game (hardware
platforms, memory maps and allocators, debugging tools, Game Genie codes), and every page about
a game mode this compendium does not implement.

Diagrams are dropped. The wiki illustrates these pages heavily and each picture arrives as a
bare URL carrying no rules text — over half the bytes of the Dropset page were image links.
Captions are prose and have been kept; where a diagram matters, follow the source link.

## Provenance

Scraped from <https://puyonexus.com/wiki/> with a local Firecrawl instance
(`POST /v2/scrape`, markdown format), 2026-08-27. Text is the work of Puyo Nexus Wiki's
contributors and is reproduced here unaltered but for the removal of site navigation and image
links. The wiki states no content licence. *Puyo Puyo* is a trademark of SEGA; this project is
an unaffiliated clone.

---

# Contents

**Category: Rules** — The twelve pages the wiki files under [Category:Rules](https://puyonexus.com/wiki/Category:Rules).

- [All clear](#all-clear)
- [Basic rules](#basic-rules)
- [Dropset](#dropset)
- [List of attack powers](#list-of-attack-powers)
- [Margin time](#margin-time)
- [Nuisance queue](#nuisance-queue)
- [Offset rule](#offset-rule)
- [Rotation](#rotation)
- [Scoring](#scoring)
- [Staircase maneuver](#staircase-maneuver)
- [Super attack](#super-attack)
- [Types of Puyo](#types-of-puyo)

**Rules that are not in Category:Rules** — The trap this document exists for. The ruleset we implement, the ghost puyo and the ceiling all live outside that category, filed as gameplay guides.

- [Tsu (rule)](#tsu-rule)
- [Special Maneuvers and Mechanics](#special-maneuvers-and-mechanics)
- [Garbage Management: Digging and Counters](#garbage-management-digging-and-counters)

**Puyo Puyo Tsu, reverse engineered** — Mechanics read out of the Mega Drive and arcade ROMs: the exact algorithms and frame timings, and - just as usefully - a list of what nobody has worked out yet.

- [Reverse Engineering (index)](#reverse-engineering-index)
- [Upcoming Pair Randomizer](#upcoming-pair-randomizer)
- [Falling Pair Spawning Process](#falling-pair-spawning-process)
- [Falling Pair Control](#falling-pair-control)
- [Rotation, collision and push back](#rotation-collision-and-push-back)
- [Pair Lateral Movement](#pair-lateral-movement)
- [Soft Drop](#soft-drop)
- [Free fall](#free-fall)
- [Frame Data Tables](#frame-data-tables)
- [Random Number Generator](#random-number-generator)

---

# Category: Rules

The twelve pages the wiki files under [Category:Rules](https://puyonexus.com/wiki/Category:Rules).


## All clear

*Source: <https://puyonexus.com/wiki/All_clear> &mdash; 2 diagrams omitted*

A visual indicator is shown to the player after they perform an all clear.

An **all clear** (also known as 全消し, **zen-keshi**) is triggered immediately after a player clears all Puyo on their field, including any Nuisance Puyo. The effect it produces depends on the game and the current rule set. All clears were first introduced in _[Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu)_.

*   Under the original **[Puyo Puyo](https://puyonexus.com/wiki/Puyo_Puyo_(rule))
    ** rule, no bonus is earned for achieving an all clear.
*   Under **[Tsu](https://puyonexus.com/wiki/Tsu_(rule))
    ** rules, you will send 30 extra Nuisance Puyo (An extra Rock Puyo) for the next chain you clear.
*   Under **[Sun](https://puyonexus.com/wiki/Sun_(rule))
    ** rules, Sun Puyo will fall in your game field. The amount of Sun Puyo that falls is equal to the length of the chain you cleared that produced the All Clear (clearing a 2 chain will produce 2 Sun Puyo).
*   In _[Puyo Puyo~n](https://puyonexus.com/wiki/Puyo_Puyo~n) _, the All Clear works the same as it does in Tsu.
*   Under _[Fever](https://puyonexus.com/wiki/Fever_(rule)) _ rules and the **[Transformation](https://puyonexus.com/wiki/Transformation)
    ** rule, you get a preset 4 chain along with a 5 second bonus on your fever or transformation time. If you All Clear while entering Fever or Transformation, you will instead earn a 2 chain bonus in Fever/Transformation along with the 5 second bonus time.
*   In gimmick modes in _[Puyo Puyo! 15th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary) _ and _[Puyo Puyo!! 20th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!!_20th_Anniversary) _, you get a preset 4 chain.
*   In **[Mega Puyo Rush](https://puyonexus.com/wiki/Transformation#Mega_Puyo_Rush)
    **, you earn a time bonus in _[Puyo Puyo 7](https://puyonexus.com/wiki/Puyo_Puyo_7) _ and a chain bonus in _Puyo Puyo!! 20th Anniversary_.
*   In [Puyo Puyo!! Quest Arcade](https://puyonexus.com/wiki/Puyo_Puyo!!_Quest_Arcade) , you get a preset 5 chain along with 10 seconds in Fever Mode.

#### Gallery

*    The All Clear (zen-keshi) Icon in _Puyo Puyo!! 20th Anniversary_.


## Basic rules

*Source: <https://puyonexus.com/wiki/Basic_rules>*

To play any _Puyo Puyo_ game you must manipulate the falling Puyo and form groups of the same colors.

#### Contents

*   [1 Manipulating Puyo](https://puyonexus.com/wiki/Basic_rules#Manipulating_Puyo)

*   [2 Win and loss conditions](https://puyonexus.com/wiki/Basic_rules#Win_and_loss_conditions)

*   [3 Chains](https://puyonexus.com/wiki/Basic_rules#Chains)

*   [4 Garbage](https://puyonexus.com/wiki/Basic_rules#Garbage)


#### Manipulating Puyo

You can move the Puyo left or right within the board (which is usually 6x12 squares in size) by pressing the left or right buttons. You can make it fall faster by holding the down button. You can also rotate the group clockwise or anticlockwise by pressing an action button.

There are currently four types of groups you may receive, though in all games before [Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever) only the first type is available. The types are:

*   I-block - two Puyo starting vertically rotate around the lower one. They may be of the same or different color.
*   L-block - three Puyo in an L-shape rotate around the one in the corner. They may either all be of the same color, or there will be a vertical same-color pair and a single Puyo in a different color on the lower-right.
*   II-block - four Puyo in a square rotate around the center. The left two Puyo are the same color as each other and so are the right two Puyo. The two sides will never be the same color as each other.
*   O-block - four Puyo in a square do not rotate and are all the same color. You can change the color by attempting to rotate the Puyo.

#### Win and loss conditions

When a square marked with a red X or a number is filled with a Puyo, you lose; when all other players have lost, you win. In games before Puyo Puyo Fever, there are no red Xs or numbers, but the game acts as if there was a red X in the square on the first row, third column. The marked position, with a few exceptions, tends to be the spawn point of the Puyo, and the size of the largest possible Puyo groups.

#### Chains

When you form a group of four or more cardinally connected same-color Puyo, they will explode, and any Puyo above will fall down. If in the process another group is formed, this also explodes and so on until there are no more groups left. An _n-chain_ refers to a series of _n_ such explosions. Chains score more and produce more garbage as you make them longer and bigger. The scores are calculated using an algorithm along with [chain powers](https://puyonexus.com/wiki/Chain_Power_Table)
.

#### Garbage

When you make chains you will produce garbage, which is sent to the opponent. Upon certain conditions (depending on the game) it will then fall on their board. Garbage Puyo cannot be cleared by being grouped but instead they will be cleared if an ordinary group is cleared next to them. Making big chains and sending garbage to the opponent is the key to defeating your opponent.

Garbage generation is calculated based on score, which, in turn, is calculated with [powers](https://puyonexus.com/wiki/Chain_Power_Table)
.


## Dropset

*Source: <https://puyonexus.com/wiki/Dropset> &mdash; 860 diagrams omitted*

In all major Puyo entries since _[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_, a **dropset** is a character's sequence of piece shapes in certain game modes. In classic game modes, drops were only made up of 2 Puyo, arranged vertically. In modes that use dropsets, the drops can be made up of up to 4 Puyos.

Dropsets are featured in the following game modes:

*   [Fever](https://puyonexus.com/wiki/Fever_(rule))

*   [Transformation](https://puyonexus.com/wiki/Transformation)

*   [Searchlight](https://puyonexus.com/wiki/Searchlight)

*   [Slot](https://puyonexus.com/wiki/Slot)

*   [Pair Puyo](https://puyonexus.com/wiki/Pair_Puyo)

*   [Party](https://puyonexus.com/wiki/Party)

*   Endless Tiny Puyo

[Non-Stop Fever](https://puyonexus.com/wiki/Non-Stop_Fever) and [Mini Puyo Fever](https://puyonexus.com/wiki/Transformation#Mini_Puyo_Fever) use unique preset dropsets regardless of character. [Quartet](https://puyonexus.com/wiki/Quartet) and [Fusion](https://puyonexus.com/wiki/Fusion) have their own version of dropsets that are different from all other modes.

#### Contents

*   [1 Arrangements](https://puyonexus.com/wiki/Dropset#Arrangements)

*   [2 _Puyo Puyo Fever 1_/_2_](https://puyonexus.com/wiki/Dropset#Puyo_Puyo_Fever_1/2)

*   [3 List of Dropsets](https://puyonexus.com/wiki/Dropset#List_of_Dropsets)

*   [4 Character Types](https://puyonexus.com/wiki/Dropset#Character_Types)
    *   [4.1 _Puyo Puyo Fever_](https://puyonexus.com/wiki/Dropset#Puyo_Puyo_Fever)

    *   [4.2 _Puyo Puyo! 15th Anniversary_](https://puyonexus.com/wiki/Dropset#Puyo_Puyo!_15th_Anniversary)

    *   [4.3 _Puyo Puyo 7_](https://puyonexus.com/wiki/Dropset#Puyo_Puyo_7)

    *   [4.4 _Puyo Puyo!! 20th Anniversary_](https://puyonexus.com/wiki/Dropset#Puyo_Puyo!!_20th_Anniversary)

    *   [4.5 _Puyo Puyo Tetris_ / _Puyo Puyo Tetris 2_](https://puyonexus.com/wiki/Dropset#Puyo_Puyo_Tetris_/_Puyo_Puyo_Tetris_2)

    *   [4.6 _Puyo Puyo Champions_](https://puyonexus.com/wiki/Dropset#Puyo_Puyo_Champions)

    *   [4.7 _Puyo Puyo Puzzle Pop_](https://puyonexus.com/wiki/Dropset#Puyo_Puyo_Puzzle_Pop)

*   [5 Notes/Trivia](https://puyonexus.com/wiki/Dropset#Notes/Trivia)


#### Arrangements

Dropset arrangements in Puyo Puyo are very particular. Drops are always either monochromatic or dichromatic. The dropset can specify up to 5 different types of arrangements.

*   2 puyo, in an I arrangement; The bottom puyo is the first color, the top is the second color.
*   3 puyo, in an L/J arrangement; Can be configured in the following ways, determined by the dropset:

*   With the vertical stretch of 2 puyo as the first color, and the remaining puyo as the second color
*   With the horizontal stretch of 2 puyo as the first color, and the remaining puyo as the second color

*   4 puyo, in an O arrangement; can be configured in the following ways, determined by the dropset:

*   Monochromatic: A 2x2 blob. Instead of rotating, the rotation buttons cycle colors.
*   Dichromatic: The bottom horizontal stretch is the first color, and the top horizontal stretch is the second color.

Special Note: The Dichromatic block never chooses the same colors (for obvious reasons.) This is in contrast to other drops, as other drops, as specified by the dropset, are monochromatic or dichromatic at random. In _Tetris 2'_s Party Mode, the Single Color Puyos powerup makes it possible to receive Dichromatic blocks with both sides having the same color.

#### _Puyo Puyo Fever 1_/_2_

In _[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_ and _[Puyo Puyo Fever 2](https://puyonexus.com/wiki/Puyo_Puyo_Fever_2)_, the behavior of L vs. J is slightly more complicated than in future games. _Fever 1_ and _2_ do not make a distinction between the L and J pieces at the character selection screen. If one were actually use the character in a match, there is a specific preset number and placement of L and J pieces within the characters dropset.

Another mechanic present in only _Fever 1_ and _2_ is L and J piece cycling. After every 16 pieces, characters with odd numbers of Giant Puyo will have their L pieces become J pieces, and J pieces become L pieces for the next 16 drops. Characters with even numbers of Giant Puyo, on the other hand, will only use their designated dropset.

#### List of Dropsets

Certain characters that do not appear after _[Puyo Puyo Fever 2](https://puyonexus.com/wiki/Puyo_Puyo_Fever_2)_ have the aforementioned L vs. J present in the dropset as one big trio.

DropsetUsersDropset Debut

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 16  | 0   | 0   | 0   | 32  |

[Arle](https://puyonexus.com/wiki/Arle)
, [Dark Arle](https://puyonexus.com/wiki/Dark_Arle)
• [_Puyo Puyo_ (1991)](https://puyonexus.com/wiki/Puyo_Puyo_(1991))

• _[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 10  | 3   | 1   | 2   | 41  |

[Schezo](https://puyonexus.com/wiki/Schezo)
, [Ragnus](https://puyonexus.com/wiki/Ragnus)
_[Puyo Puyo! 15th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 12  | 0   | 2   | 2   | 40  |

[Rulue](https://puyonexus.com/wiki/Rulue)
_[Puyo Puyo! 15th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 8   | 6   | 1   | 1   | 42  |

[Accord](https://puyonexus.com/wiki/Accord)
, [Ecolo](https://puyonexus.com/wiki/Ecolo)
 (_7_), [Ex](https://puyonexus.com/wiki/Ex)
, [Alex](https://puyonexus.com/wiki/Alex)
_[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 9   | 4   | 2   | 1   | 42  |

[Klug](https://puyonexus.com/wiki/Klug)
 (_20th_ onwards), [Possessed Klug](https://puyonexus.com/wiki/Possessed_Klug)
 (_20th_)_[Puyo Puyo!! 20th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!!_20th_Anniversary)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 9   | 4   | 2   | 1   | 42  |

[Klug](https://puyonexus.com/wiki/Klug)
 (_Fever_ to _7_), [Ciel](https://puyonexus.com/wiki/Ciel)
_[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 9   | 4   | 2   | 1   | 42  |

[Lemres](https://puyonexus.com/wiki/Lemres)
, [Hartmann](https://puyonexus.com/wiki/Hartmann)
_[Puyo Puyo Fever 2](https://puyonexus.com/wiki/Puyo_Puyo_Fever_2)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 10  | 4   | 2   | 0   | 40  |

[Suketoudara](https://puyonexus.com/wiki/Suketoudara)
, [Hed](https://puyonexus.com/wiki/Hed)
_[Puyo Puyo! 15th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 8   | 4   | 2   | 2   | 44  |

[Paprisu](https://puyonexus.com/wiki/Paprisu)
_[Puyo Puyo Champions](https://puyonexus.com/wiki/Puyo_Puyo_Champions)_ August Update

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 8   | 6   | 1   | 1   | 42  |

[Lidelle](https://puyonexus.com/wiki/Lidelle)
, [Draco](https://puyonexus.com/wiki/Draco)
 (_7_), [Ess](https://puyonexus.com/wiki/Ess)
, [Penglai](https://puyonexus.com/wiki/Penglai)
_[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 8   | 4   | 2   | 2   | 44  |

[Feli](https://puyonexus.com/wiki/Feli)
, [Angelic Feli](https://puyonexus.com/wiki/Angelic_Feli)
, [Sultana](https://puyonexus.com/wiki/Sultana)
_[Puyo Puyo Fever 2](https://puyonexus.com/wiki/Puyo_Puyo_Fever_2)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 7   | 7   | 1   | 1   | 43  |

[Ecolo](https://puyonexus.com/wiki/Ecolo)
 (_20th_ onwards), [Unusual Ecolo](https://puyonexus.com/wiki/Unusual_Ecolo)
_[Puyo Puyo!! 20th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!!_20th_Anniversary)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 10  | 4   | 1   | 1   | 40  |

[Draco](https://puyonexus.com/wiki/Draco)
 (_20th_ onwards)_[Puyo Puyo!! 20th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!!_20th_Anniversary)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 13  | 1   | 1   | 1   | 37  |

[Dapper Bones](https://puyonexus.com/wiki/Dapper_Bones)
, [Skeleton T](https://puyonexus.com/wiki/Skeleton_T)
, [Ally](https://puyonexus.com/wiki/Ally)
_[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 12  | 1   | 1   | 2   | 39  |

[Donguri Gaeru](https://puyonexus.com/wiki/Donguri_Gaeru)
, [Maguro](https://puyonexus.com/wiki/Maguro)
 (_7_), [Ai](https://puyonexus.com/wiki/Ai)
_[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 10  | 0   | 2   | 1   | 32  |

[Tartar](https://puyonexus.com/wiki/Tartar)
 (predates _15th_)_[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 9   | 4   | 2   | 1   | 42  |

[Ocean Prince](https://puyonexus.com/wiki/Ocean_Prince)
, [Zed](https://puyonexus.com/wiki/Zed)
, [Serilly](https://puyonexus.com/wiki/Serilly)
_[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 8   | 4   | 2   | 2   | 44  |

[Zoh Daimaoh](https://puyonexus.com/wiki/Zoh_Daimaoh)
, [Rafisol](https://puyonexus.com/wiki/Rafisol)
_[Puyo Puyo! 15th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 6   | 4   | 3   | 3   | 48  |

[Carbuncle](https://puyonexus.com/wiki/Carbuncle)
_[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 6   | 4   | 3   | 3   | 48  |

[Baldanders](https://puyonexus.com/wiki/Baldanders)
_[Puyo Puyo Fever 2](https://puyonexus.com/wiki/Puyo_Puyo_Fever_2)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 9   | 3   | 2   | 2   | 43  |

[Yu](https://puyonexus.com/wiki/Yu)
 (& [Rei](https://puyonexus.com/wiki/Rei)
), [Ringo](https://puyonexus.com/wiki/Ringo)
 (_7_), [Jay & Elle](https://puyonexus.com/wiki/Jay_%26_Elle)
, [Harpy](https://puyonexus.com/wiki/Harpy)
_[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 9   | 4   | 2   | 1   | 42  |

[Ringo](https://puyonexus.com/wiki/Ringo)
 (_20th_ onwards), [Tee](https://puyonexus.com/wiki/Tee)
_[Puyo Puyo!! 20th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!!_20th_Anniversary)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 11  | 2   | 1   | 2   | 40  |

[Maguro](https://puyonexus.com/wiki/Maguro)
 (_20th_ onwards)_[Puyo Puyo!! 20th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!!_20th_Anniversary)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 12  | 2   | 1   | 1   | 38  |

[Risukuma](https://puyonexus.com/wiki/Risukuma)
 (_20th_ onwards)_[Puyo Puyo!! 20th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!!_20th_Anniversary)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 12  | 0   | 0   | 1   | 28  |

[Hoho](https://puyonexus.com/wiki/Hoho)
 (predates _15th_)_[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 10  | 2   | 3   | 1   | 42  |

[Akuma](https://puyonexus.com/wiki/Akuma)
_[Puyo Puyo Fever 2](https://puyonexus.com/wiki/Puyo_Puyo_Fever_2)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 9   | 0   | 1   | 1   | 26  |

[Frankensteins](https://puyonexus.com/wiki/Frankensteins)
 (predates _15th_)_[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 12  | 1   | 2   | 1   | 39  |

[Dark Prince](https://puyonexus.com/wiki/Satan)
, [Yellow Satan](https://puyonexus.com/wiki/Yellow_Satan)
_[Puyo Puyo! 15th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 10  | 0   | 2   | 1   | 32  |

[Popoi](https://puyonexus.com/wiki/Popoi)
 (predates _15th_)_[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 7   | 5   | 2   | 2   | 45  |

[Possessed Klug](https://puyonexus.com/wiki/Possessed_Klug)
 (Non _20th_)_[Puyo Puyo Fever 2](https://puyonexus.com/wiki/Puyo_Puyo_Fever_2)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 9   | 0   | 3   | 2   | 38  |

[Gogotte](https://puyonexus.com/wiki/Gogotte)
 (predates _15th_)_[Puyo Puyo Fever 2](https://puyonexus.com/wiki/Puyo_Puyo_Fever_2)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 12  | 2   | 1   | 1   | 38  |

[Amitie](https://puyonexus.com/wiki/Amitie)
_[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 11  | 3   | 1   | 1   | 39  |

[Raffina](https://puyonexus.com/wiki/Raffina)
_[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 11  | 2   | 1   | 2   | 40  |

[Sig](https://puyonexus.com/wiki/Sig)
_[Puyo Puyo Fever 2](https://puyonexus.com/wiki/Puyo_Puyo_Fever_2)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 11  | 3   | 1   | 1   | 39  |

[Witch](https://puyonexus.com/wiki/Witch)
_[Puyo Puyo!! 20th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!!_20th_Anniversary)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 13  | 1   | 1   | 1   | 37  |

[Onion Pixie](https://puyonexus.com/wiki/Onion_Pixie)
, [Risukuma](https://puyonexus.com/wiki/Risukuma)
 (_7_), [O](https://puyonexus.com/wiki/O)
_[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 8   | 3   | 3   | 2   | 45  |

[Nasu Grave](https://puyonexus.com/wiki/Nasu_Grave)
_[Puyo Puyo! 15th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 10  | 0   | 3   | 3   | 44  |

[Marle](https://puyonexus.com/wiki/Marle)
_[Puyo Puyo Tetris 2](https://puyonexus.com/wiki/Puyo_Puyo_Tetris_2)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 10  | 4   | 1   | 1   | 40  |

[Squares](https://puyonexus.com/wiki/Squares)
_[Puyo Puyo Tetris 2](https://puyonexus.com/wiki/Puyo_Puyo_Tetris_2)_

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 10  | 4   | 2   | 0   | 40  |

[Sonic](https://puyonexus.com/wiki/Sonic)
_[Puyo Puyo Tetris 2](https://puyonexus.com/wiki/Puyo_Puyo_Tetris_2)_ January Update

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 12  | 2   | 1   | 1   | 38  |

[Legamünt](https://puyonexus.com/wiki/Legam%C3%BCnt)
_[Puyo Puyo Tetris 2](https://puyonexus.com/wiki/Puyo_Puyo_Tetris_2)_ March Update

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 11  | 4   | 1   | 0   | 38  |

[Rozatte](https://puyonexus.com/wiki/Rozatte)
_[Puyo Puyo Tetris 2](https://puyonexus.com/wiki/Puyo_Puyo_Tetris_2)_ March Update

| Pieces |     |     |     |     |     |     |     |     |     |     |     |     |     |     |     | Total |     |     |     |     |     |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  |  |  |  |  | Puyo |
|  |  |  |  |  |  |  |  |  |  |  |  |  |  |  |  | 11  | 2   | 1   | 2   | 40  |

[Meena](https://puyonexus.com/wiki/Meena)
_[Puyo Puyo Puzzle Pop](https://puyonexus.com/wiki/Puyo_Puyo_Puzzle_Pop)_ Version 1.5.0

#### Character Types

Each character has a specific title given to them on the character select screen that describes the playstyle they're most suited for. These titles are named differently in each game, but are generally consistent with each character for the most part.

##### _Puyo Puyo Fever_

| Title | Character(s) |
| --- | --- |
| Well-Balanced Player | Amitie, Arle |
| Build Up to Fever | Oshare Bones, Ocean Prince |
| Likes Large Chains | Klug |
| Keep the pressure on! | Dongurigaeru, Tarutaru, Frankensteins |
| Loves Chains | Rider |
| Constant Fever | Onion Pixy, Yu, Hohow Bird, Popoi |
| Only does Big Chains | Raffine, Accord |
| Well-Balanced Player? | Carbuncle |

##### _Puyo Puyo! 15th Anniversary_

| Title | Character(s) |
| --- | --- |
| Well-Balanced Player | Amitie, Lemres |
| Sets up for Strong Fever | Dapper Bones, Ocean Prince, Sig, Yu & Rei, Suketoudara |
| Builds up large chains | Klug, Schezo, Ms. Accord, |
| Keep foes under pressure | Dongurigaeru, Nasu Grave, Baldanders |
| Loves to build chains | Lidelle, Arle, Satan |
| Maintains constant Fever | Onion Pixy, Feli, Zoh Daimaoh, |
| Only goes for big chains | Raffina, Akuma, Rulue |

##### _Puyo Puyo 7_

| Title | Character(s) |
| --- | --- |
| Well-balanced Player | Amitie, Lemres |
| Sets up for strong Fever | Ringo, Sig, Suketoudara, Skeleton-T |
| Builds up large chains | Rulue, Raffina, Carbuncle |
| Keep foes under pressure | Maguro |
| Loves to build chains | Arle, Satan, Dark Arle, Draco |
| Maintains constant Fever | Feli, Risukuma |
| Only goes for big chains | Klug, Ecolo, Schezo |

##### _Puyo Puyo!! 20th Anniversary_

| Title | Character(s) |
| --- | --- |
| Well-balanced Player | Amitie, Lemres |
| Sets up for strong Fever | Ringo, Sig, Suketoudara, Yu & Rei, Ocean Prince, Witch |
| Builds large chains | Klug, Ecolo, Schezo, Ms. Accord |
| Keep foes under pressure | Maguro, Dongurigaeru |
| Loves to build chains | Arle, Satan, Lidelle, Draco |
| Maintains constant Fever | Feli, Risukuma, Onion Pixie |
| Only goes for big chains | Raffina, Carbuncle, Rulue |

##### _Puyo Puyo Tetris_ / _Puyo Puyo Tetris 2_

| Title | Character(s) |
| --- | --- |
| Balanced! | Ringo, Sig, Witch, Draco, Tee, Ai, Lemres, Ally, Serilly, Ragnus, Rozatte |
| Throws Garbage! | Arle, Carbuncle, Risukuma, Schezo, Ess, Jay & Elle, Klug, Feli, The Ocean Prince, Yu & Rei, Harpy |
| Helps Themself! | Amitie, Maguro, Suketoudara, O, Zed, Raffina, Rulue, Sonic, Lidelle, Legamünt |
| Special Type! | Dark Prince, Marle, Ecolo, Ex, Squares, Ms. Accord, Rafisol, Possessed Klug |

##### _Puyo Puyo Champions_

| Title | Character(s) |
| --- | --- |
| Uses Small Chains! | Arle, Penglai, Ally, Draco, Dark Prince |
| Strongly Balanced! | Amitie, Hartmann |
| Reverses Fever! | Ringo, Hed, Sig, Suketoudara, Witch, Harpy, Serilly |
| Aims for Fever! | Sultana, Risukuma, Paprisu |
| Aims for Big Chains! | Schezo, Ciel, Alex, Rafisol |
| Attacks Fast! | Maguro, Ragnus |
| Big Chains or Nothing! | Rulue, Raffina, Carbuncle |

##### _Puyo Puyo Puzzle Pop_

| Title | Character(s) |
| --- | --- |
| Well-Balanced | Amitie, Lemres |
| Chain Lover | Arle, Draco, Lidelle, Ally, Dark Prince |
| Fever Table-Turner | Ringo, Suketoudara, Witch, Ocean Prince, Yu & Rei, Sig |
| Big Chain Obsessive | Raffina, Rulue, Carbuncle |
| Big Chain Lover | Klug, Schezo, Ms. Accord, Ecolo, Rafisol |
| Fever-Focused | Feli, Risukuma |
| Pressure Dropper | Maguro, Meena |

#### Notes/Trivia

*   Arle's dropset solely consists of pairs, a reference to how previous Puyo rulesets worked. This gives her the smallest dropset in every game with Fever mode.
*   Carbuncle & Baldanders tie for the largest dropset. Their dropsets are mostly the same, with a few swapped placements of their 4 Puyo.
*   Counting Yu, Rei, Jay and Elle separately, the dropset attached to the most characters is Yu's at 6. When stacked, they tie with Accord's and Lidelle's at 4.
*   No new dropsets were introduced between 20th and the 2020 _Champions_ patch, meaning there was a 9 year gap from the last new dropsets. Arle's is technically the oldest, coming from 1991.
*   Popoi's dropset has not been used since 2003, and is ongoing, being the longest period in which a dropset went unused.
*   Currently, both _Puyo Puyo Tetris_ (Ringo & Tee) games and _[Puyo Puyo Champions](https://puyonexus.com/wiki/Puyo_Puyo_Champions) _ (Schezo & Ragnus, Suketoudara & Hed) are the only installments where two unrelated characters have totally identical dropsets. Dark Arle in _7_ and the _20th_ alternate characters are still connected to the characters they share the dropset of.
*   The following characters have went from sharing a dropset with somebody else, to having their own: Ringo, Maguro, Risukuma, Ecolo, and Draco. All of these characters made their Fever-era debut in 7, and recieved new dropsets in 20th as the characters they borrowed from were present. In addition, Klug's dropset was slightly altered.
*   Possessed Klug is the only character in the series to be introduced with an original dropset, then end up getting said Dropset replaced with the one of another character (Klug), then regain the original Dropset back in another game.
*   Marle, Squares, Sonic, Legamünt and Rozatte have the only dropsets that cannot be used in Fever mode, as it does not exist in Puyo Puyo Tetris 2. Their dropsets are instead exclusive to [Party](https://puyonexus.com/wiki/Party) mode and Endless Tiny Puyo Mode.
*   Meena’s dropset is the first in the series to start off with a dichromatic piece.


## List of attack powers

*Source: <https://puyonexus.com/wiki/List_of_attack_powers>*

This is a list of the attack powers used in all the _Puyo Puyo_ games.

Attack Power is the multiplier applied to the points earned when Puyo are popped, corresponding to the current Chain (so a Chain of 1 will have the multiplier at entry 1, a Chain of 2 will add entry 2, and so on). A Chain Power value of 0 gets set to 1.

#### Contents

*   [1 Classic Rules](https://puyonexus.com/wiki/List_of_attack_powers#Classic_Rules)

*   [2 _Puyo Puyo Fever_](https://puyonexus.com/wiki/List_of_attack_powers#Puyo_Puyo_Fever)
    *   [2.1 Normal](https://puyonexus.com/wiki/List_of_attack_powers#Normal)

    *   [2.2 Fever](https://puyonexus.com/wiki/List_of_attack_powers#Fever)

*   [3 _Puyo Puyo Fever 2_](https://puyonexus.com/wiki/List_of_attack_powers#Puyo_Puyo_Fever_2)
    *   [3.1 Normal](https://puyonexus.com/wiki/List_of_attack_powers#Normal_2)

    *   [3.2 Fever](https://puyonexus.com/wiki/List_of_attack_powers#Fever_2)

*   [4 _Puyo Puyo! 15th Anniversary_](https://puyonexus.com/wiki/List_of_attack_powers#Puyo_Puyo!_15th_Anniversary)
    *   [4.1 Normal](https://puyonexus.com/wiki/List_of_attack_powers#Normal_3)

    *   [4.2 Fever](https://puyonexus.com/wiki/List_of_attack_powers#Fever_3)

*   [5 _Puyo Puyo 7_](https://puyonexus.com/wiki/List_of_attack_powers#Puyo_Puyo_7)
    *   [5.1 Normal](https://puyonexus.com/wiki/List_of_attack_powers#Normal_4)

    *   [5.2 Fever](https://puyonexus.com/wiki/List_of_attack_powers#Fever_4)

    *   [5.3 Mega Puyo Rush](https://puyonexus.com/wiki/List_of_attack_powers#Mega_Puyo_Rush)

    *   [5.4 Mini Puyo Fever](https://puyonexus.com/wiki/List_of_attack_powers#Mini_Puyo_Fever)

*   [6 _Puyo Puyo!! 20th Anniversary_](https://puyonexus.com/wiki/List_of_attack_powers#Puyo_Puyo!!_20th_Anniversary)
    *   [6.1 Normal](https://puyonexus.com/wiki/List_of_attack_powers#Normal_5)

    *   [6.2 Fever](https://puyonexus.com/wiki/List_of_attack_powers#Fever_5)

*   [7 _Puyo Puyo Chronicle_](https://puyonexus.com/wiki/List_of_attack_powers#Puyo_Puyo_Chronicle)
    *   [7.1 Normal](https://puyonexus.com/wiki/List_of_attack_powers#Normal_6)

    *   [7.2 Fever](https://puyonexus.com/wiki/List_of_attack_powers#Fever_6)


#### Classic Rules

_[Puyo Puyo Sun](https://puyonexus.com/wiki/Puyo_Puyo_Sun)_, _[Puyo Puyo~n](https://puyonexus.com/wiki/Puyo_Puyo~n)_, _[Minna de Puyo Puyo](https://puyonexus.com/wiki/Minna_de_Puyo_Puyo)_, and all non-Fever based modes from _[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_ and on use the attack powers from _[Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu)_.

_Puyo Puyo~n_ contains attack powers for up to a 120 chain. In this game, the attack power for every chain past the 24th chain increases by 32 until it hits the maximum attack power of 999.

| Rule | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  | 17  | 18  | 19  | 20  | 21  | 22  | 23  | 24+ |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [_Puyo Puyo_ (1992)](https://puyonexus.com/wiki/Puyo_Puyo_(1992)) | 0   | 8   | 16  | 32  | 64  | 128 | 256 | 512 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 999 |
| _[Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu)<br>_ | 0   | 8   | 16  | 32  | 64  | 96  | 128 | 160 | 192 | 224 | 256 | 288 | 320 | 352 | 384 | 416 | 448 | 480 | 512 | 544 | 576 | 608 | 640 | 672 |
| _Puyo Puyo Tsu_  <br>(Single player) | 4   | 20  | 24  | 32  | 48  | 96  | 160 | 240 | 320 | 480 | 600 | 700 | 800 | 900 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 999 |

#### _[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_

##### Normal

This game contains attack powers for up to a 24 chain. As all characters will reach the maximum attack power of 999 by their 19th chain, attack powers past that chain have been omitted.

| Character | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  | 17  | 18  | 19+ | Tier |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [Accord](https://puyonexus.com/wiki/Accord) | 4   | 11  | 24  | 33  | 51  | 106 | 179 | 274 | 371 | 472 | 600 | 732 | 882 | 999 | 999 | 999 | 999 | 999 | 999 | 2   |
| [Amitie](https://puyonexus.com/wiki/Amitie) | 4   | 12  | 24  | 32  | 48  | 96  | 160 | 240 | 320 | 400 | 500 | 600 | 700 | 800 | 900 | 999 | 999 | 999 | 999 | 6   |
| [Arle](https://puyonexus.com/wiki/Arle) | 4   | 12  | 24  | 33  | 50  | 101 | 169 | 254 | 341 | 428 | 538 | 648 | 763 | 876 | 990 | 999 | 999 | 999 | 999 | 4   |
| [Carbuncle](https://puyonexus.com/wiki/Carbuncle) | 4   | 12  | 24  | 33  | 50  | 101 | 169 | 254 | 341 | 428 | 538 | 648 | 763 | 876 | 990 | 999 | 999 | 999 | 999 | 4   |
| [Dapper Bones](https://puyonexus.com/wiki/Dapper_Bones) | 4   | 11  | 22  | 30  | 45  | 91  | 153 | 230 | 309 | 388 | 488 | 588 | 693 | 796 | 900 | 999 | 999 | 999 | 999 | 7   |
| [Donguri Gaeru](https://puyonexus.com/wiki/Donguri_Gaeru) | 4   | 13  | 25  | 33  | 49  | 96  | 158 | 235 | 310 | 384 | 475 | 564 | 644 | 728 | 810 | 890 | 968 | 999 | 999 | 8   |
| [Frank & Stein](https://puyonexus.com/wiki/Frankensteins) | 4   | 13  | 25  | 32  | 47  | 91  | 150 | 221 | 290 | 356 | 438 | 516 | 581 | 652 | 720 | 785 | 847 | 888 | 999 | 10  |
| [Hoho](https://puyonexus.com/wiki/Hoho) | 4   | 11  | 22  | 29  | 43  | 86  | 144 | 216 | 288 | 360 | 450 | 540 | 630 | 720 | 810 | 900 | 990 | 999 | 999 | 9   |
| [Klug](https://puyonexus.com/wiki/Klug) | 4   | 11  | 24  | 34  | 53  | 110 | 188 | 288 | 392 | 500 | 638 | 780 | 945 | 999 | 999 | 999 | 999 | 999 | 999 | 1   |
| [Ocean Prince](https://puyonexus.com/wiki/Ocean_Prince) | 4   | 11  | 22  | 30  | 45  | 91  | 153 | 230 | 309 | 388 | 488 | 588 | 693 | 796 | 900 | 999 | 999 | 999 | 999 | 7   |
| [Onion Pixie](https://puyonexus.com/wiki/Onion_Pixie) | 4   | 11  | 22  | 30  | 45  | 91  | 153 | 230 | 309 | 388 | 488 | 588 | 693 | 796 | 900 | 999 | 999 | 999 | 999 | 7   |
| [Popoi](https://puyonexus.com/wiki/Popoi) | 4   | 11  | 22  | 29  | 43  | 86  | 144 | 216 | 288 | 360 | 450 | 540 | 630 | 720 | 810 | 900 | 990 | 999 | 999 | 9   |
| [Raffina](https://puyonexus.com/wiki/Raffina) | 4   | 11  | 24  | 33  | 51  | 106 | 179 | 274 | 371 | 472 | 600 | 732 | 882 | 999 | 999 | 999 | 999 | 999 | 999 | 2   |
| [Lidelle](https://puyonexus.com/wiki/Lidelle) | 4   | 13  | 26  | 35  | 53  | 106 | 176 | 264 | 352 | 440 | 550 | 660 | 770 | 880 | 990 | 999 | 999 | 999 | 999 | 3   |
| [Tartar](https://puyonexus.com/wiki/Tartar) | 4   | 13  | 25  | 33  | 49  | 96  | 158 | 235 | 310 | 384 | 475 | 564 | 644 | 728 | 810 | 890 | 968 | 999 | 999 | 8   |
| [Yu](https://puyonexus.com/wiki/Yu) | 4   | 11  | 23  | 31  | 47  | 96  | 162 | 245 | 330 | 416 | 525 | 636 | 756 | 872 | 990 | 999 | 999 | 999 | 999 | 5   |

##### Fever

| Character | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  | 17  | 18  | 19  | 20  | 21  | 22  | 23  | 24+ | Tier |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [Accord](https://puyonexus.com/wiki/Accord) | 4   | 10  | 18  | 21  | 29  | 46  | 76  | 113 | 150 | 223 | 259 | 266 | 313 | 364 | 398 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 8   |
| [Amitie](https://puyonexus.com/wiki/Amitie) | 4   | 10  | 18  | 22  | 30  | 48  | 80  | 120 | 160 | 240 | 280 | 288 | 342 | 400 | 440 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 6   |
| [Arle](https://puyonexus.com/wiki/Arle) | 4   | 10  | 18  | 21  | 29  | 46  | 76  | 113 | 150 | 223 | 259 | 266 | 313 | 364 | 398 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 8   |
| [Carbuncle](https://puyonexus.com/wiki/Carbuncle) | 4   | 9   | 17  | 20  | 28  | 46  | 76  | 115 | 154 | 233 | 273 | 282 | 337 | 396 | 438 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 7   |
| [Dapper Bones](https://puyonexus.com/wiki/Dapper_Bones) | 4   | 11  | 20  | 25  | 34  | 55  | 92  | 139 | 186 | 281 | 329 | 339 | 405 | 476 | 526 | 576 | 624 | 672 | 720 | 768 | 816 | 864 | 912 | 960 | 3   |
| [Donguri Gaeru](https://puyonexus.com/wiki/Donguri_Gaeru) | 4   | 10  | 18  | 21  | 29  | 46  | 76  | 113 | 150 | 223 | 259 | 266 | 313 | 364 | 398 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 8   |
| [Frank & Stein](https://puyonexus.com/wiki/Frankensteins) | 4   | 11  | 19  | 22  | 29  | 46  | 75  | 110 | 145 | 214 | 245 | 250 | 290 | 332 | 359 | 384 | 416 | 448 | 480 | 512 | 544 | 576 | 608 | 640 | 10  |
| [Hoho](https://puyonexus.com/wiki/Hoho) | 5   | 12  | 22  | 26  | 36  | 58  | 96  | 144 | 192 | 288 | 336 | 346 | 410 | 480 | 528 | 576 | 624 | 672 | 720 | 768 | 816 | 864 | 912 | 960 | 2   |
| [Klug](https://puyonexus.com/wiki/Klug) | 4   | 9   | 16  | 20  | 27  | 43  | 72  | 108 | 144 | 216 | 252 | 259 | 308 | 360 | 396 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 9   |
| [Ocean Prince](https://puyonexus.com/wiki/Ocean_Prince) | 4   | 10  | 19  | 24  | 34  | 55  | 93  | 142 | 191 | 290 | 343 | 355 | 428 | 508 | 565 | 624 | 676 | 728 | 780 | 832 | 884 | 936 | 988 | 999 | 1   |
| [Onion Pixie](https://puyonexus.com/wiki/Onion_Pixie) | 5   | 12  | 21  | 25  | 34  | 53  | 87  | 130 | 171 | 254 | 294 | 301 | 353 | 408 | 444 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 5   |
| [Popoi](https://puyonexus.com/wiki/Popoi) | 5   | 12  | 22  | 26  | 36  | 58  | 96  | 144 | 192 | 288 | 336 | 346 | 410 | 480 | 528 | 576 | 624 | 672 | 720 | 768 | 816 | 864 | 912 | 960 | 2   |
| [Raffina](https://puyonexus.com/wiki/Raffina) | 4   | 9   | 17  | 20  | 28  | 46  | 76  | 115 | 154 | 233 | 273 | 282 | 337 | 396 | 438 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 7   |
| [Lidelle](https://puyonexus.com/wiki/Lidelle) | 3   | 8   | 14  | 18  | 24  | 38  | 64  | 96  | 128 | 192 | 224 | 230 | 274 | 320 | 352 | 384 | 416 | 448 | 480 | 512 | 544 | 576 | 608 | 640 | 11  |
| [Tartar](https://puyonexus.com/wiki/Tartar) | 4   | 10  | 18  | 21  | 29  | 46  | 76  | 113 | 150 | 223 | 259 | 266 | 313 | 364 | 398 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 8   |
| [Yu](https://puyonexus.com/wiki/Yu) | 4   | 10  | 19  | 23  | 32  | 53  | 89  | 134 | 181 | 274 | 322 | 333 | 400 | 472 | 524 | 576 | 624 | 672 | 720 | 768 | 816 | 864 | 912 | 960 | 4   |

#### _[Puyo Puyo Fever 2](https://puyonexus.com/wiki/Puyo_Puyo_Fever_2)_

##### Normal

This game contains attack powers for up to a 24 chain. As all characters will reach the maximum attack power of 999 by their 21st chain, attack powers past that chain have been omitted.

| Character | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  | 17  | 18  | 19  | 20  | 21+ | Tier |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [Accord](https://puyonexus.com/wiki/Accord) | 4   | 11  | 24  | 33  | 51  | 106 | 179 | 274 | 371 | 472 | 600 | 732 | 882 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 2   |
| [Akuma](https://puyonexus.com/wiki/Akuma) | 3   | 10  | 21  | 29  | 46  | 96  | 163 | 250 | 339 | 432 | 550 | 672 | 812 | 944 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 4   |
| [Amitie](https://puyonexus.com/wiki/Amitie) | 4   | 12  | 24  | 32  | 48  | 96  | 160 | 240 | 320 | 400 | 500 | 600 | 700 | 800 | 900 | 999 | 999 | 999 | 999 | 999 | 999 | 6   |
| [Arle](https://puyonexus.com/wiki/Arle) | 4   | 13  | 27  | 36  | 55  | 110 | 185 | 278 | 373 | 468 | 588 | 708 | 833 | 956 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 3   |
| [Baldanders](https://puyonexus.com/wiki/Baldanders) | 4   | 13  | 24  | 31  | 45  | 86  | 141 | 206 | 269 | 328 | 400 | 468 | 518 | 576 | 630 | 680 | 726 | 744 | 910 | 980 | 999 | 13  |
| [Dapper Bones](https://puyonexus.com/wiki/Dapper_Bones) | 4   | 11  | 22  | 29  | 43  | 86  | 144 | 216 | 288 | 360 | 450 | 540 | 630 | 720 | 810 | 900 | 990 | 999 | 999 | 999 | 999 | 9   |
| [Donguri Gaeru](https://puyonexus.com/wiki/Donguri_Gaeru) | 4   | 13  | 25  | 33  | 49  | 96  | 158 | 235 | 310 | 384 | 475 | 564 | 644 | 728 | 810 | 890 | 968 | 999 | 999 | 999 | 999 | 8   |
| [Feli](https://puyonexus.com/wiki/Feli) | 4   | 11  | 21  | 28  | 41  | 82  | 135 | 202 | 267 | 332 | 413 | 492 | 567 | 644 | 720 | 795 | 869 | 936 | 999 | 999 | 999 | 12  |
| [Frank & Stein](https://puyonexus.com/wiki/Frankensteins) | 4   | 13  | 25  | 32  | 47  | 91  | 150 | 221 | 290 | 356 | 438 | 516 | 581 | 652 | 720 | 785 | 847 | 888 | 999 | 999 | 999 | 11  |
| [Gogotte](https://puyonexus.com/wiki/Gogotte) | 4   | 11  | 21  | 28  | 41  | 82  | 135 | 202 | 267 | 332 | 413 | 492 | 567 | 644 | 720 | 795 | 869 | 936 | 999 | 999 | 999 | 12  |
| [Hoho](https://puyonexus.com/wiki/Hoho) | 4   | 11  | 22  | 29  | 43  | 86  | 144 | 216 | 288 | 360 | 450 | 540 | 630 | 720 | 810 | 900 | 990 | 999 | 999 | 999 | 999 | 9   |
| [Klug](https://puyonexus.com/wiki/Klug) | 4   | 11  | 24  | 34  | 53  | 110 | 188 | 288 | 392 | 500 | 638 | 780 | 945 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 1   |
| [Lemres](https://puyonexus.com/wiki/Lemres) | 4   | 12  | 24  | 32  | 48  | 96  | 160 | 240 | 320 | 400 | 500 | 600 | 700 | 800 | 900 | 999 | 999 | 999 | 999 | 999 | 999 | 6   |
| [Ocean Prince](https://puyonexus.com/wiki/Ocean_Prince) | 4   | 11  | 21  | 28  | 41  | 82  | 135 | 202 | 267 | 332 | 413 | 492 | 567 | 644 | 720 | 795 | 869 | 936 | 999 | 999 | 999 | 12  |
| [Onion Pixie](https://puyonexus.com/wiki/Onion_Pixie) | 4   | 11  | 22  | 30  | 45  | 91  | 153 | 230 | 309 | 388 | 488 | 588 | 693 | 796 | 900 | 999 | 999 | 999 | 999 | 999 | 999 | 7   |
| [Possessed Klug](https://puyonexus.com/wiki/Possessed_Klug) | 4   | 11  | 23  | 31  | 47  | 96  | 162 | 245 | 330 | 416 | 525 | 636 | 756 | 872 | 990 | 999 | 999 | 999 | 999 | 999 | 999 | 5   |
| [Raffina](https://puyonexus.com/wiki/Raffina) | 4   | 11  | 24  | 33  | 51  | 106 | 179 | 274 | 371 | 472 | 600 | 732 | 882 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 2   |
| [Lidelle](https://puyonexus.com/wiki/Lidelle) | 4   | 13  | 27  | 36  | 55  | 110 | 185 | 278 | 373 | 468 | 588 | 708 | 833 | 956 | 999 | 999 | 999 | 999 | 999 | 999 | 999 | 3   |
| [Sig](https://puyonexus.com/wiki/Sig) | 3   | 10  | 20  | 27  | 40  | 82  | 137 | 206 | 277 | 348 | 438 | 528 | 623 | 716 | 810 | 905 | 999 | 999 | 999 | 999 | 999 | 10  |
| [Tartar](https://puyonexus.com/wiki/Tartar) | 4   | 13  | 25  | 33  | 49  | 96  | 158 | 235 | 310 | 384 | 475 | 564 | 644 | 728 | 810 | 890 | 968 | 999 | 999 | 999 | 999 | 8   |
| [Yu](https://puyonexus.com/wiki/Yu)<br> & [Rei](https://puyonexus.com/wiki/Rei) | 4   | 11  | 22  | 29  | 43  | 86  | 144 | 216 | 288 | 360 | 450 | 540 | 630 | 720 | 810 | 900 | 990 | 999 | 999 | 999 | 999 | 9   |

##### Fever

| Character | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  | 17  | 18  | 19  | 20  | 21  | 22  | 23  | 24+ | Tier |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [Accord](https://puyonexus.com/wiki/Accord) | 4   | 9   | 16  | 20  | 27  | 43  | 72  | 108 | 144 | 216 | 252 | 259 | 308 | 360 | 396 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 12  |
| [Akuma](https://puyonexus.com/wiki/Akuma) | 3   | 8   | 15  | 20  | 28  | 46  | 77  | 118 | 159 | 242 | 287 | 298 | 360 | 428 | 477 | 528 | 572 | 616 | 660 | 704 | 748 | 792 | 836 | 880 | 5   |
| [Amitie](https://puyonexus.com/wiki/Amitie) | 4   | 10  | 18  | 22  | 30  | 48  | 80  | 120 | 160 | 240 | 280 | 288 | 342 | 400 | 440 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 8   |
| [Arle](https://puyonexus.com/wiki/Arle) | 4   | 10  | 18  | 22  | 30  | 48  | 80  | 120 | 160 | 240 | 280 | 288 | 342 | 400 | 440 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 8   |
| [Baldanders](https://puyonexus.com/wiki/Baldanders) | 4   | 11  | 18  | 22  | 28  | 43  | 70  | 103 | 134 | 197 | 224 | 227 | 261 | 296 | 317 | 336 | 364 | 392 | 420 | 448 | 476 | 504 | 532 | 560 | 16  |
| [Dapper Bones](https://puyonexus.com/wiki/Dapper_Bones) | 4   | 11  | 20  | 25  | 34  | 55  | 92  | 139 | 186 | 281 | 329 | 339 | 405 | 476 | 526 | 576 | 624 | 672 | 720 | 768 | 816 | 864 | 912 | 960 | 1   |
| [Donguri Gaeru](https://puyonexus.com/wiki/Donguri_Gaeru) | 4   | 11  | 19  | 23  | 31  | 48  | 79  | 118 | 155 | 230 | 266 | 272 | 319 | 368 | 400 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 10  |
| [Feli](https://puyonexus.com/wiki/Feli) | 4   | 11  | 19  | 24  | 32  | 50  | 84  | 125 | 166 | 247 | 287 | 294 | 347 | 404 | 442 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 7   |
| [Frank & Stein](https://puyonexus.com/wiki/Frankensteins) | 4   | 11  | 19  | 22  | 29  | 46  | 75  | 110 | 145 | 214 | 245 | 250 | 290 | 332 | 359 | 384 | 416 | 448 | 480 | 512 | 544 | 576 | 608 | 640 | 14  |
| [Gogotte](https://puyonexus.com/wiki/Gogotte) | 4   | 10  | 18  | 23  | 31  | 50  | 84  | 127 | 170 | 257 | 301 | 310 | 371 | 436 | 482 | 528 | 572 | 616 | 660 | 704 | 748 | 792 | 836 | 880 | 4   |
| [Hoho](https://puyonexus.com/wiki/Hoho) | 5   | 12  | 21  | 26  | 35  | 55  | 92  | 137 | 182 | 271 | 315 | 323 | 382 | 444 | 486 | 528 | 572 | 616 | 660 | 704 | 748 | 792 | 836 | 880 | 3   |
| [Klug](https://puyonexus.com/wiki/Klug) | 3   | 8   | 15  | 18  | 25  | 41  | 68  | 103 | 138 | 209 | 245 | 253 | 302 | 356 | 394 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 13  |
| [Lemres](https://puyonexus.com/wiki/Lemres) | 4   | 10  | 18  | 21  | 29  | 46  | 76  | 113 | 150 | 223 | 259 | 266 | 313 | 364 | 398 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 11  |
| [Ocean Prince](https://puyonexus.com/wiki/Ocean_Prince) | 4   | 10  | 19  | 23  | 32  | 53  | 89  | 134 | 181 | 274 | 322 | 333 | 400 | 472 | 524 | 576 | 624 | 672 | 720 | 768 | 816 | 864 | 912 | 960 | 2   |
| [Onion Pixie](https://puyonexus.com/wiki/Onion_Pixie) | 5   | 12  | 21  | 25  | 34  | 53  | 87  | 130 | 171 | 254 | 294 | 301 | 353 | 408 | 444 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 6   |
| [Possessed Klug](https://puyonexus.com/wiki/Possessed_Klug) | 4   | 9   | 17  | 20  | 28  | 46  | 76  | 115 | 154 | 233 | 273 | 282 | 337 | 396 | 438 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 9   |
| [Raffina](https://puyonexus.com/wiki/Raffina) | 3   | 8   | 15  | 20  | 28  | 46  | 77  | 118 | 159 | 242 | 287 | 298 | 360 | 428 | 477 | 528 | 572 | 616 | 660 | 704 | 748 | 792 | 836 | 880 | 5   |
| [Lidelle](https://puyonexus.com/wiki/Lidelle) | 4   | 9   | 16  | 19  | 26  | 41  | 68  | 101 | 134 | 199 | 231 | 237 | 279 | 324 | 354 | 384 | 416 | 448 | 480 | 512 | 544 | 576 | 608 | 640 | 15  |
| [Sig](https://puyonexus.com/wiki/Sig) | 4   | 11  | 20  | 25  | 34  | 55  | 92  | 139 | 186 | 281 | 329 | 339 | 405 | 476 | 526 | 576 | 624 | 672 | 720 | 768 | 816 | 864 | 912 | 960 | 1   |
| [Tartar](https://puyonexus.com/wiki/Tartar) | 4   | 10  | 18  | 21  | 29  | 46  | 76  | 113 | 150 | 223 | 259 | 266 | 313 | 364 | 398 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 11  |
| [Yu](https://puyonexus.com/wiki/Yu)<br> & [Rei](https://puyonexus.com/wiki/Rei) | 4   | 10  | 18  | 23  | 31  | 50  | 84  | 127 | 170 | 257 | 301 | 310 | 371 | 436 | 482 | 528 | 572 | 616 | 660 | 704 | 748 | 792 | 836 | 880 | 4   |

#### _[Puyo Puyo! 15th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary)_

These attack powers apply only to the [Fever rule](https://puyonexus.com/wiki/Fever_(rule)) and rules derived from it.

##### Normal

This game contains attack powers for up to a 24 chain. As all characters will reach the maximum attack power of 999 by their 19th chain, attack powers past that chain have been omitted.

| Character | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  | 17  | 18  | 19+ | Tier |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [Accord](https://puyonexus.com/wiki/Accord) | 4   | 11  | 24  | 33  | 51  | 106 | 179 | 274 | 371 | 472 | 600 | 732 | 882 | 999 | 999 | 999 | 999 | 999 | 999 | 2   |
| [Akuma](https://puyonexus.com/wiki/Akuma) | 4   | 11  | 23  | 32  | 49  | 101 | 170 | 259 | 350 | 444 | 563 | 684 | 819 | 948 | 999 | 999 | 999 | 999 | 999 | 4   |
| [Amitie](https://puyonexus.com/wiki/Amitie) | 4   | 12  | 24  | 32  | 48  | 96  | 160 | 240 | 320 | 400 | 500 | 600 | 700 | 800 | 900 | 999 | 999 | 999 | 999 | 7   |
| [Arle](https://puyonexus.com/wiki/Arle) | 4   | 12  | 24  | 33  | 50  | 101 | 169 | 254 | 341 | 428 | 538 | 648 | 763 | 876 | 990 | 999 | 999 | 999 | 999 | 6   |
| [Baldanders](https://puyonexus.com/wiki/Baldanders) | 4   | 13  | 25  | 32  | 47  | 91  | 150 | 221 | 290 | 356 | 438 | 516 | 581 | 652 | 720 | 785 | 847 | 888 | 999 | 11  |
| [Dapper Bones](https://puyonexus.com/wiki/Dapper_Bones) | 4   | 11  | 22  | 30  | 45  | 91  | 153 | 230 | 309 | 388 | 488 | 588 | 693 | 796 | 900 | 999 | 999 | 999 | 999 | 8   |
| [Donguri Gaeru](https://puyonexus.com/wiki/Donguri_Gaeru) | 4   | 13  | 25  | 33  | 48  | 96  | 158 | 235 | 310 | 384 | 475 | 564 | 644 | 728 | 810 | 890 | 968 | 999 | 999 | 9   |
| [Feli](https://puyonexus.com/wiki/Feli) | 4   | 11  | 21  | 28  | 41  | 82  | 135 | 202 | 267 | 332 | 413 | 492 | 567 | 644 | 720 | 795 | 869 | 936 | 999 | 12  |
| [Klug](https://puyonexus.com/wiki/Klug) | 4   | 11  | 23  | 33  | 51  | 110 | 188 | 288 | 392 | 500 | 638 | 780 | 945 | 999 | 999 | 999 | 999 | 999 | 999 | 1   |
| [Lemres](https://puyonexus.com/wiki/Lemres) | 4   | 12  | 24  | 32  | 48  | 96  | 160 | 240 | 320 | 400 | 500 | 600 | 700 | 800 | 900 | 999 | 999 | 999 | 999 | 7   |
| [Nasu Grave](https://puyonexus.com/wiki/Nasu_Grave) | 4   | 13  | 25  | 32  | 47  | 91  | 150 | 221 | 290 | 356 | 438 | 516 | 581 | 652 | 720 | 785 | 847 | 888 | 999 | 11  |
| [Ocean Prince](https://puyonexus.com/wiki/Ocean_Prince) | 4   | 11  | 22  | 29  | 43  | 86  | 144 | 216 | 288 | 360 | 450 | 540 | 630 | 720 | 810 | 900 | 990 | 999 | 999 | 10  |
| [Onion Pixie](https://puyonexus.com/wiki/Onion_Pixie) | 4   | 11  | 22  | 30  | 45  | 91  | 153 | 230 | 309 | 388 | 488 | 588 | 693 | 796 | 900 | 999 | 999 | 999 | 999 | 8   |
| [Raffina](https://puyonexus.com/wiki/Raffina) | 4   | 11  | 24  | 33  | 51  | 106 | 179 | 274 | 371 | 472 | 600 | 732 | 882 | 999 | 999 | 999 | 999 | 999 | 999 | 2   |
| [Lidelle](https://puyonexus.com/wiki/Lidelle) | 4   | 12  | 25  | 34  | 52  | 106 | 178 | 269 | 362 | 456 | 575 | 696 | 826 | 952 | 999 | 999 | 999 | 999 | 999 | 3   |
| [Rulue](https://puyonexus.com/wiki/Rulue) | 4   | 11  | 24  | 33  | 51  | 106 | 179 | 274 | 371 | 472 | 600 | 732 | 882 | 999 | 999 | 999 | 999 | 999 | 999 | 2   |
| [Satan](https://puyonexus.com/wiki/Satan) | 4   | 11  | 23  | 33  | 51  | 101 | 167 | 250 | 331 | 412 | 513 | 612 | 766 | 966 | 999 | 999 | 999 | 999 | 999 | 5   |
| [Schezo](https://puyonexus.com/wiki/Schezo) | 4   | 11  | 23  | 33  | 51  | 110 | 188 | 288 | 392 | 500 | 638 | 780 | 945 | 999 | 999 | 999 | 999 | 999 | 999 | 1   |
| [Sig](https://puyonexus.com/wiki/Sig) | 4   | 11  | 22  | 29  | 43  | 86  | 144 | 216 | 288 | 360 | 450 | 540 | 630 | 720 | 810 | 900 | 990 | 999 | 999 | 10  |
| [Suketoudara](https://puyonexus.com/wiki/Suketoudara) | 4   | 11  | 21  | 28  | 41  | 82  | 135 | 202 | 267 | 332 | 413 | 492 | 567 | 644 | 720 | 795 | 869 | 936 | 999 | 12  |
| [Yu](https://puyonexus.com/wiki/Yu)<br> & [Rei](https://puyonexus.com/wiki/Rei) | 4   | 11  | 22  | 29  | 43  | 86  | 144 | 216 | 288 | 360 | 450 | 540 | 630 | 720 | 810 | 900 | 990 | 999 | 999 | 10  |
| [Zoh Daimaoh](https://puyonexus.com/wiki/Zoh_Daimaoh) | 4   | 11  | 22  | 29  | 43  | 86  | 144 | 216 | 288 | 360 | 450 | 540 | 630 | 720 | 810 | 900 | 990 | 999 | 999 | 10  |

##### Fever

| Character | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  | 17  | 18  | 19  | 20  | 21  | 22  | 23  | 24+ | Tier |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [Accord](https://puyonexus.com/wiki/Accord) | 4   | 9   | 16  | 20  | 27  | 43  | 72  | 108 | 144 | 216 | 252 | 259 | 308 | 360 | 396 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 11  |
| [Akuma](https://puyonexus.com/wiki/Akuma) | 4   | 9   | 17  | 20  | 28  | 46  | 76  | 115 | 154 | 233 | 273 | 282 | 337 | 396 | 438 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 9   |
| [Amitie](https://puyonexus.com/wiki/Amitie) | 4   | 10  | 18  | 22  | 30  | 48  | 80  | 120 | 160 | 240 | 280 | 288 | 342 | 400 | 440 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 8   |
| [Arle](https://puyonexus.com/wiki/Arle) | 4   | 9   | 16  | 20  | 27  | 43  | 72  | 108 | 144 | 216 | 252 | 259 | 308 | 360 | 396 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 11  |
| [Baldanders](https://puyonexus.com/wiki/Baldanders) | 4   | 10  | 17  | 21  | 28  | 43  | 71  | 106 | 139 | 206 | 238 | 243 | 284 | 328 | 356 | 384 | 416 | 448 | 480 | 512 | 544 | 576 | 608 | 640 | 13  |
| [Dapper Bones](https://puyonexus.com/wiki/Dapper_Bones) | 4   | 11  | 20  | 25  | 34  | 55  | 92  | 139 | 186 | 281 | 329 | 339 | 405 | 476 | 526 | 576 | 624 | 672 | 720 | 768 | 816 | 864 | 912 | 960 | 2   |
| [Donguri Gaeru](https://puyonexus.com/wiki/Donguri_Gaeru) | 4   | 10  | 18  | 21  | 29  | 46  | 76  | 113 | 150 | 223 | 259 | 266 | 313 | 364 | 398 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 10  |
| [Feli](https://puyonexus.com/wiki/Feli) | 4   | 11  | 19  | 24  | 32  | 50  | 84  | 125 | 166 | 247 | 287 | 294 | 347 | 404 | 442 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 7   |
| [Klug](https://puyonexus.com/wiki/Klug) | 4   | 9   | 16  | 20  | 27  | 43  | 72  | 108 | 144 | 216 | 252 | 259 | 308 | 360 | 396 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 11  |
| [Lemres](https://puyonexus.com/wiki/Lemres) | 4   | 10  | 18  | 21  | 29  | 46  | 76  | 113 | 150 | 223 | 259 | 266 | 313 | 364 | 398 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 10  |
| [Nasu Grave](https://puyonexus.com/wiki/Nasu_Grave) | 4   | 10  | 17  | 21  | 28  | 43  | 71  | 106 | 139 | 206 | 238 | 243 | 284 | 328 | 356 | 384 | 416 | 448 | 480 | 512 | 544 | 576 | 608 | 640 | 13  |
| [Ocean Prince](https://puyonexus.com/wiki/Ocean_Prince) | 4   | 10  | 19  | 24  | 34  | 55  | 93  | 142 | 191 | 290 | 343 | 355 | 428 | 508 | 565 | 624 | 676 | 728 | 780 | 832 | 884 | 936 | 988 | 999 | 1   |
| [Onion Pixie](https://puyonexus.com/wiki/Onion_Pixie) | 5   | 12  | 21  | 25  | 34  | 53  | 87  | 130 | 171 | 254 | 294 | 301 | 353 | 408 | 444 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 6   |
| [Raffina](https://puyonexus.com/wiki/Raffina) | 3   | 8   | 15  | 20  | 28  | 46  | 77  | 118 | 159 | 242 | 287 | 298 | 360 | 428 | 477 | 528 | 572 | 616 | 660 | 704 | 748 | 792 | 836 | 880 | 4   |
| [Lidelle](https://puyonexus.com/wiki/Lidelle) | 3   | 8   | 14  | 18  | 24  | 38  | 64  | 96  | 128 | 192 | 224 | 230 | 274 | 320 | 352 | 384 | 416 | 448 | 480 | 512 | 544 | 576 | 608 | 640 | 14  |
| [Rulue](https://puyonexus.com/wiki/Rulue) | 3   | 8   | 15  | 20  | 28  | 46  | 77  | 112 | 151 | 229 | 272 | 283 | 342 | 406 | 453 | 501 | 543 | 585 | 627 | 668 | 710 | 752 | 794 | 836 | 5   |
| [Satan](https://puyonexus.com/wiki/Satan) | 3   | 8   | 15  | 18  | 25  | 41  | 68  | 103 | 138 | 209 | 245 | 253 | 302 | 356 | 394 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 12  |
| [Schezo](https://puyonexus.com/wiki/Schezo) | 3   | 8   | 15  | 18  | 25  | 41  | 68  | 103 | 138 | 209 | 245 | 253 | 302 | 356 | 394 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 12  |
| [Sig](https://puyonexus.com/wiki/Sig) | 4   | 11  | 20  | 25  | 34  | 55  | 92  | 139 | 186 | 281 | 329 | 339 | 405 | 476 | 526 | 576 | 624 | 672 | 720 | 768 | 816 | 864 | 912 | 960 | 2   |
| [Suketoudara](https://puyonexus.com/wiki/Suketoudara) | 4   | 11  | 20  | 25  | 34  | 55  | 92  | 139 | 186 | 281 | 329 | 339 | 405 | 476 | 526 | 576 | 624 | 672 | 720 | 768 | 816 | 864 | 912 | 960 | 2   |
| [Yu](https://puyonexus.com/wiki/Yu)<br> & [Rei](https://puyonexus.com/wiki/Rei) | 4   | 9   | 17  | 22  | 31  | 50  | 85  | 130 | 175 | 266 | 315 | 326 | 394 | 468 | 521 | 576 | 624 | 672 | 720 | 768 | 816 | 864 | 912 | 960 | 3   |
| [Zoh Daimaoh](https://puyonexus.com/wiki/Zoh_Daimaoh) | 4   | 11  | 19  | 24  | 32  | 50  | 84  | 125 | 166 | 247 | 287 | 294 | 347 | 404 | 442 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 7   |

#### _[Puyo Puyo 7](https://puyonexus.com/wiki/Puyo_Puyo_7)_

These attack powers apply only to the [Transformation](https://puyonexus.com/wiki/Transformation) and [Fever rule](https://puyonexus.com/wiki/Fever_(rule)) rules.

As Trio is not playable in the following rules, their tier has been omitted.

##### Normal

This game contains attack powers for up to a 24 chain. As all characters will reach the maximum attack power of 999 by their 19th chain, attack powers past that chain have been omitted.

| Character | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  | 17  | 18  | 19+ | Tier |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [Amitie](https://puyonexus.com/wiki/Amitie) | 4   | 12  | 24  | 32  | 48  | 96  | 160 | 240 | 320 | 400 | 500 | 600 | 700 | 800 | 900 | 999 | 999 | 999 | 999 | 7   |
| [Arle](https://puyonexus.com/wiki/Arle) | 4   | 12  | 24  | 33  | 50  | 101 | 169 | 254 | 341 | 428 | 538 | 648 | 763 | 876 | 990 | 999 | 999 | 999 | 999 | 5   |
| [Carbuncle](https://puyonexus.com/wiki/Carbuncle) | 4   | 12  | 24  | 32  | 48  | 96  | 160 | 240 | 320 | 400 | 500 | 600 | 700 | 800 | 900 | 999 | 999 | 999 | 999 | 7   |
| [Dark Arle](https://puyonexus.com/wiki/Dark_Arle) | 4   | 11  | 23  | 30  | 45  | 96  | 162 | 252 | 336 | 420 | 526 | 632 | 743 | 852 | 968 | 999 | 999 | 999 | 999 | 6   |
| [Draco Centauros](https://puyonexus.com/wiki/Draco_Centauros) | 4   | 12  | 25  | 34  | 52  | 106 | 178 | 269 | 362 | 456 | 575 | 696 | 826 | 952 | 999 | 999 | 999 | 999 | 999 | 3   |
| [Ecolo](https://puyonexus.com/wiki/Ecolo) | 4   | 11  | 24  | 33  | 51  | 106 | 179 | 274 | 371 | 472 | 600 | 732 | 882 | 999 | 999 | 999 | 999 | 999 | 999 | 2   |
| [Feli](https://puyonexus.com/wiki/Feli) | 4   | 11  | 21  | 28  | 41  | 82  | 135 | 202 | 267 | 332 | 413 | 492 | 567 | 644 | 720 | 795 | 869 | 936 | 999 | 11  |
| [Klug](https://puyonexus.com/wiki/Klug) | 4   | 11  | 23  | 33  | 51  | 110 | 188 | 288 | 392 | 500 | 638 | 780 | 945 | 999 | 999 | 999 | 999 | 999 | 999 | 1   |
| [Lemres](https://puyonexus.com/wiki/Lemres) | 4   | 12  | 24  | 32  | 48  | 96  | 160 | 240 | 320 | 400 | 500 | 600 | 700 | 800 | 900 | 999 | 999 | 999 | 999 | 7   |
| [Maguro](https://puyonexus.com/wiki/Maguro) | 4   | 13  | 25  | 33  | 48  | 96  | 158 | 235 | 310 | 384 | 475 | 564 | 644 | 728 | 810 | 890 | 968 | 999 | 999 | 9   |
| [Raffina](https://puyonexus.com/wiki/Raffina) | 4   | 11  | 24  | 33  | 51  | 106 | 179 | 274 | 371 | 472 | 600 | 732 | 882 | 999 | 999 | 999 | 999 | 999 | 999 | 2   |
| [Ringo](https://puyonexus.com/wiki/Ringo) | 4   | 11  | 22  | 29  | 43  | 86  | 144 | 216 | 288 | 360 | 450 | 540 | 630 | 720 | 810 | 900 | 990 | 999 | 999 | 10  |
| [Risukuma](https://puyonexus.com/wiki/Risukuma) | 4   | 11  | 22  | 30  | 45  | 91  | 153 | 230 | 309 | 388 | 488 | 588 | 693 | 796 | 900 | 999 | 999 | 999 | 999 | 8   |
| [Rulue](https://puyonexus.com/wiki/Rulue) | 4   | 11  | 24  | 33  | 51  | 106 | 179 | 274 | 371 | 472 | 600 | 732 | 882 | 999 | 999 | 999 | 999 | 999 | 999 | 2   |
| [Satan](https://puyonexus.com/wiki/Satan) | 4   | 11  | 23  | 33  | 51  | 101 | 167 | 250 | 331 | 412 | 513 | 612 | 766 | 966 | 999 | 999 | 999 | 999 | 999 | 4   |
| [Schezo](https://puyonexus.com/wiki/Schezo) | 4   | 11  | 23  | 33  | 51  | 110 | 188 | 288 | 392 | 500 | 638 | 780 | 945 | 999 | 999 | 999 | 999 | 999 | 999 | 1   |
| [Sig](https://puyonexus.com/wiki/Sig) | 4   | 11  | 22  | 29  | 43  | 86  | 144 | 216 | 288 | 360 | 450 | 540 | 630 | 720 | 810 | 900 | 990 | 999 | 999 | 10  |
| [Skeleton T](https://puyonexus.com/wiki/Skeleton_T) | 4   | 11  | 22  | 30  | 45  | 91  | 153 | 230 | 309 | 388 | 488 | 588 | 693 | 796 | 900 | 999 | 999 | 999 | 999 | 8   |
| [Suketoudara](https://puyonexus.com/wiki/Suketoudara) | 4   | 11  | 21  | 28  | 41  | 82  | 135 | 202 | 267 | 332 | 413 | 492 | 567 | 644 | 720 | 795 | 869 | 936 | 999 | 11  |
| Everybody | 4   | 12  | 24  | 32  | 48  | 96  | 160 | 240 | 320 | 400 | 500 | 600 | 700 | 800 | 900 | 999 | 999 | 999 | 999 | \-  |

##### Fever

| Character | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  | 17  | 18  | 19  | 20  | 21  | 22  | 23  | 24+ | Tier |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [Amitie](https://puyonexus.com/wiki/Amitie) | 4   | 10  | 18  | 22  | 30  | 48  | 80  | 120 | 160 | 240 | 280 | 288 | 342 | 400 | 440 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 7   |
| [Arle](https://puyonexus.com/wiki/Arle) | 4   | 9   | 16  | 20  | 27  | 43  | 72  | 108 | 144 | 216 | 252 | 259 | 308 | 360 | 396 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 10  |
| [Carbuncle](https://puyonexus.com/wiki/Carbuncle) | 4   | 10  | 18  | 22  | 30  | 48  | 80  | 120 | 160 | 240 | 280 | 288 | 342 | 400 | 440 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 7   |
| [Dark Arle](https://puyonexus.com/wiki/Dark_Arle) | 4   | 9   | 17  | 22  | 29  | 46  | 75  | 112 | 150 | 228 | 267 | 279 | 326 | 378 | 412 | 450 | 488 | 530 | 564 | 608 | 644 | 672 | 698 | 720 | 8   |
| [Draco Centauros](https://puyonexus.com/wiki/Draco_Centauros) | 3   | 8   | 14  | 18  | 24  | 38  | 64  | 96  | 128 | 192 | 224 | 230 | 274 | 320 | 352 | 384 | 416 | 448 | 480 | 512 | 544 | 576 | 608 | 640 | 12  |
| [Ecolo](https://puyonexus.com/wiki/Ecolo) | 4   | 9   | 16  | 20  | 27  | 43  | 72  | 108 | 144 | 216 | 252 | 259 | 308 | 360 | 396 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 10  |
| [Feli](https://puyonexus.com/wiki/Feli) | 4   | 11  | 19  | 24  | 32  | 50  | 84  | 125 | 166 | 247 | 287 | 294 | 347 | 404 | 442 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 6   |
| [Klug](https://puyonexus.com/wiki/Klug) | 4   | 9   | 16  | 20  | 27  | 43  | 72  | 108 | 144 | 216 | 252 | 259 | 308 | 360 | 396 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 10  |
| [Lemres](https://puyonexus.com/wiki/Lemres) | 4   | 10  | 18  | 21  | 29  | 46  | 76  | 113 | 150 | 223 | 259 | 266 | 313 | 364 | 398 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 9   |
| [Maguro](https://puyonexus.com/wiki/Maguro) | 4   | 10  | 18  | 21  | 29  | 46  | 76  | 113 | 150 | 223 | 259 | 266 | 313 | 364 | 398 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 9   |
| [Raffina](https://puyonexus.com/wiki/Raffina) | 3   | 8   | 15  | 20  | 28  | 46  | 77  | 118 | 159 | 242 | 287 | 298 | 360 | 428 | 477 | 528 | 572 | 616 | 660 | 704 | 748 | 792 | 836 | 880 | 3   |
| [Ringo](https://puyonexus.com/wiki/Ringo) | 4   | 9   | 17  | 22  | 31  | 50  | 85  | 130 | 175 | 266 | 315 | 326 | 394 | 468 | 521 | 576 | 624 | 672 | 720 | 768 | 816 | 864 | 912 | 960 | 2   |
| [Risukuma](https://puyonexus.com/wiki/Risukuma) | 5   | 12  | 21  | 25  | 34  | 53  | 87  | 130 | 171 | 254 | 294 | 301 | 353 | 408 | 444 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | 5   |
| [Rulue](https://puyonexus.com/wiki/Rulue) | 3   | 8   | 15  | 20  | 28  | 46  | 77  | 112 | 151 | 229 | 272 | 283 | 342 | 406 | 453 | 501 | 543 | 585 | 627 | 668 | 710 | 752 | 794 | 836 | 4   |
| [Satan](https://puyonexus.com/wiki/Satan) | 3   | 8   | 15  | 18  | 25  | 41  | 68  | 103 | 138 | 209 | 245 | 253 | 302 | 356 | 394 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 11  |
| [Schezo](https://puyonexus.com/wiki/Schezo) | 3   | 8   | 15  | 18  | 25  | 41  | 68  | 103 | 138 | 209 | 245 | 253 | 302 | 356 | 394 | 432 | 468 | 504 | 540 | 576 | 612 | 648 | 684 | 720 | 11  |
| [Sig](https://puyonexus.com/wiki/Sig) | 4   | 11  | 20  | 25  | 34  | 55  | 92  | 139 | 186 | 281 | 329 | 339 | 405 | 476 | 526 | 576 | 624 | 672 | 720 | 768 | 816 | 864 | 912 | 960 | 1   |
| [Skeleton T](https://puyonexus.com/wiki/Skeleton_T) | 4   | 11  | 20  | 25  | 34  | 55  | 92  | 139 | 186 | 281 | 329 | 339 | 405 | 476 | 526 | 576 | 624 | 672 | 720 | 768 | 816 | 864 | 912 | 960 | 1   |
| [Suketoudara](https://puyonexus.com/wiki/Suketoudara) | 4   | 11  | 20  | 25  | 34  | 55  | 92  | 139 | 186 | 281 | 329 | 339 | 405 | 476 | 526 | 576 | 624 | 672 | 720 | 768 | 816 | 864 | 912 | 960 | 1   |
| Everybody | 4   | 10  | 18  | 22  | 30  | 48  | 80  | 120 | 160 | 240 | 280 | 288 | 342 | 400 | 440 | 480 | 520 | 560 | 600 | 640 | 680 | 720 | 760 | 800 | \-  |

##### Mega Puyo Rush

The attack power for every chain past the 24th chain increases by 6 until it hits the maximum attack power of 999.

| Character | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  | 17  | 18  | 19  | 20  | 21  | 22  | 23  | 24+ | Tier |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [Amitie](https://puyonexus.com/wiki/Amitie) | 1   | 8   | 14  | 20  | 28  | 30  | 32  | 34  | 36  | 38  | 40  | 42  | 44  | 46  | 48  | 50  | 52  | 54  | 56  | 58  | 62  | 68  | 74  | 80  | 4   |
| [Arle](https://puyonexus.com/wiki/Arle) | 1   | 7   | 13  | 19  | 26  | 28  | 30  | 32  | 34  | 36  | 38  | 40  | 42  | 44  | 46  | 48  | 49  | 51  | 53  | 55  | 59  | 65  | 71  | 76  | 6   |
| [Carbuncle](https://puyonexus.com/wiki/Carbuncle) | 1   | 7   | 13  | 19  | 27  | 29  | 31  | 33  | 35  | 37  | 39  | 41  | 43  | 45  | 47  | 49  | 50  | 52  | 54  | 56  | 60  | 66  | 72  | 78  | 5   |
| [Dark Arle](https://puyonexus.com/wiki/Dark_Arle) | 1   | 7   | 13  | 19  | 27  | 29  | 31  | 33  | 35  | 37  | 39  | 41  | 43  | 45  | 47  | 49  | 50  | 52  | 54  | 56  | 60  | 66  | 72  | 78  | 5   |
| [Draco Centauros](https://puyonexus.com/wiki/Draco_Centauros) | 1   | 7   | 13  | 19  | 27  | 29  | 31  | 33  | 35  | 37  | 39  | 41  | 43  | 45  | 47  | 49  | 50  | 52  | 54  | 56  | 60  | 66  | 72  | 78  | 5   |
| [Ecolo](https://puyonexus.com/wiki/Ecolo) | 1   | 7   | 13  | 19  | 27  | 29  | 31  | 33  | 35  | 37  | 39  | 41  | 43  | 45  | 47  | 49  | 50  | 52  | 54  | 56  | 60  | 66  | 72  | 78  | 5   |
| [Feli](https://puyonexus.com/wiki/Feli) | 1   | 8   | 14  | 21  | 29  | 31  | 33  | 36  | 38  | 40  | 42  | 44  | 46  | 48  | 50  | 53  | 55  | 57  | 59  | 61  | 65  | 72  | 78  | 84  | 1   |
| [Klug](https://puyonexus.com/wiki/Klug) | 1   | 7   | 13  | 19  | 27  | 29  | 31  | 33  | 35  | 37  | 39  | 41  | 43  | 45  | 47  | 49  | 50  | 52  | 54  | 56  | 60  | 66  | 72  | 78  | 5   |
| [Lemres](https://puyonexus.com/wiki/Lemres) | 1   | 7   | 13  | 19  | 26  | 28  | 30  | 32  | 34  | 36  | 38  | 40  | 42  | 44  | 46  | 48  | 49  | 51  | 53  | 55  | 59  | 65  | 71  | 76  | 6   |
| [Maguro](https://puyonexus.com/wiki/Maguro) | 1   | 8   | 14  | 20  | 28  | 30  | 32  | 34  | 36  | 38  | 40  | 42  | 44  | 46  | 48  | 50  | 52  | 54  | 56  | 58  | 62  | 68  | 74  | 80  | 4   |
| [Raffina](https://puyonexus.com/wiki/Raffina) | 1   | 8   | 14  | 20  | 28  | 30  | 32  | 34  | 36  | 38  | 40  | 42  | 44  | 46  | 48  | 50  | 52  | 54  | 56  | 58  | 62  | 68  | 74  | 80  | 4   |
| [Ringo](https://puyonexus.com/wiki/Ringo) | 1   | 8   | 14  | 21  | 29  | 31  | 33  | 36  | 38  | 40  | 42  | 44  | 46  | 48  | 50  | 53  | 55  | 57  | 59  | 61  | 65  | 72  | 78  | 84  | 1   |
| [Risukuma](https://puyonexus.com/wiki/Risukuma) | 1   | 8   | 14  | 20  | 28  | 30  | 32  | 34  | 36  | 38  | 40  | 42  | 44  | 46  | 48  | 51  | 53  | 55  | 57  | 59  | 63  | 69  | 75  | 81  | 3   |
| [Rulue](https://puyonexus.com/wiki/Rulue) | 1   | 7   | 13  | 19  | 26  | 28  | 30  | 32  | 34  | 36  | 38  | 40  | 42  | 44  | 46  | 48  | 49  | 51  | 53  | 55  | 59  | 65  | 71  | 76  | 6   |
| [Satan](https://puyonexus.com/wiki/Satan) | 1   | 7   | 13  | 19  | 27  | 29  | 31  | 33  | 35  | 37  | 39  | 41  | 43  | 45  | 47  | 49  | 50  | 52  | 54  | 56  | 60  | 66  | 72  | 78  | 5   |
| [Schezo](https://puyonexus.com/wiki/Schezo) | 1   | 7   | 13  | 19  | 26  | 28  | 30  | 32  | 34  | 36  | 38  | 40  | 42  | 44  | 46  | 48  | 49  | 51  | 53  | 55  | 59  | 65  | 71  | 76  | 6   |
| [Sig](https://puyonexus.com/wiki/Sig) | 1   | 8   | 14  | 21  | 29  | 31  | 33  | 36  | 38  | 40  | 42  | 44  | 46  | 48  | 50  | 53  | 55  | 57  | 59  | 61  | 65  | 72  | 78  | 84  | 1   |
| [Skeleton T](https://puyonexus.com/wiki/Skeleton_T) | 1   | 8   | 14  | 20  | 29  | 31  | 33  | 35  | 37  | 39  | 41  | 43  | 45  | 47  | 49  | 52  | 54  | 56  | 58  | 60  | 64  | 70  | 76  | 83  | 2   |
| [Suketoudara](https://puyonexus.com/wiki/Suketoudara) | 1   | 8   | 14  | 21  | 29  | 31  | 33  | 36  | 38  | 40  | 42  | 44  | 46  | 48  | 50  | 53  | 55  | 57  | 59  | 61  | 65  | 72  | 78  | 84  | 1   |
| Everybody | 1   | 8   | 14  | 20  | 28  | 30  | 32  | 34  | 36  | 38  | 40  | 42  | 44  | 46  | 48  | 50  | 52  | 54  | 56  | 58  | 62  | 68  | 74  | 80  | \-  |

##### Mini Puyo Fever

The attack power for every chain past the 24th chain increases by 6 until it hits the maximum attack power of 999.

| Character | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  | 17  | 18  | 19  | 20  | 21  | 22  | 23  | 24+ | Tier |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [Amitie](https://puyonexus.com/wiki/Amitie) | 1   | 2   | 4   | 6   | 8   | 10  | 12  | 14  | 16  | 20  | 24  | 28  | 32  | 40  | 48  | 57  | 65  | 73  | 81  | 89  | 97  | 106 | 114 | 122 | 3   |
| [Arle](https://puyonexus.com/wiki/Arle) | 1   | 2   | 3   | 5   | 7   | 9   | 11  | 13  | 15  | 19  | 23  | 27  | 31  | 39  | 47  | 54  | 62  | 70  | 78  | 86  | 94  | 101 | 109 | 117 | 4   |
| [Carbuncle](https://puyonexus.com/wiki/Carbuncle) | 1   | 2   | 3   | 5   | 7   | 9   | 11  | 13  | 15  | 19  | 23  | 27  | 31  | 39  | 47  | 54  | 62  | 70  | 78  | 86  | 94  | 101 | 109 | 117 | 4   |
| [Dark Arle](https://puyonexus.com/wiki/Dark_Arle) | 1   | 2   | 3   | 5   | 7   | 9   | 11  | 13  | 15  | 19  | 23  | 27  | 31  | 39  | 47  | 54  | 62  | 70  | 78  | 86  | 94  | 101 | 109 | 117 | 4   |
| [Draco Centauros](https://puyonexus.com/wiki/Draco_Centauros) | 1   | 2   | 3   | 5   | 7   | 9   | 11  | 13  | 15  | 19  | 23  | 26  | 30  | 38  | 46  | 53  | 61  | 69  | 76  | 84  | 92  | 99  | 107 | 115 | 5   |
| [Ecolo](https://puyonexus.com/wiki/Ecolo) | 1   | 2   | 3   | 5   | 7   | 9   | 11  | 13  | 15  | 19  | 23  | 27  | 31  | 39  | 47  | 54  | 62  | 70  | 78  | 86  | 94  | 101 | 109 | 117 | 4   |
| [Feli](https://puyonexus.com/wiki/Feli) | 1   | 2   | 4   | 6   | 8   | 10  | 12  | 14  | 16  | 20  | 24  | 29  | 33  | 41  | 49  | 58  | 66  | 74  | 83  | 91  | 99  | 108 | 116 | 124 | 2   |
| [Klug](https://puyonexus.com/wiki/Klug) | 1   | 2   | 3   | 5   | 7   | 9   | 11  | 13  | 15  | 19  | 23  | 26  | 30  | 38  | 46  | 53  | 61  | 69  | 76  | 84  | 92  | 99  | 107 | 115 | 5   |
| [Lemres](https://puyonexus.com/wiki/Lemres) | 1   | 2   | 3   | 5   | 7   | 9   | 11  | 13  | 15  | 19  | 23  | 27  | 31  | 39  | 47  | 54  | 62  | 70  | 78  | 86  | 94  | 101 | 109 | 117 | 4   |
| [Maguro](https://puyonexus.com/wiki/Maguro) | 1   | 2   | 4   | 6   | 8   | 10  | 12  | 14  | 16  | 20  | 24  | 28  | 32  | 40  | 48  | 57  | 65  | 73  | 81  | 89  | 97  | 106 | 114 | 122 | 3   |
| [Raffina](https://puyonexus.com/wiki/Raffina) | 1   | 2   | 4   | 6   | 8   | 10  | 12  | 14  | 16  | 20  | 24  | 28  | 32  | 40  | 48  | 57  | 65  | 73  | 81  | 89  | 97  | 106 | 114 | 122 | 3   |
| [Ringo](https://puyonexus.com/wiki/Ringo) | 1   | 2   | 4   | 6   | 8   | 10  | 12  | 14  | 16  | 20  | 24  | 29  | 33  | 41  | 49  | 58  | 66  | 74  | 83  | 91  | 99  | 108 | 116 | 124 | 2   |
| [Risukuma](https://puyonexus.com/wiki/Risukuma) | 1   | 2   | 4   | 6   | 8   | 10  | 12  | 14  | 16  | 20  | 24  | 28  | 32  | 40  | 48  | 57  | 65  | 73  | 81  | 89  | 97  | 106 | 114 | 122 | 3   |
| [Rulue](https://puyonexus.com/wiki/Rulue) | 1   | 2   | 3   | 5   | 7   | 9   | 11  | 13  | 15  | 19  | 23  | 27  | 31  | 39  | 47  | 54  | 62  | 70  | 78  | 86  | 94  | 101 | 109 | 117 | 4   |
| [Satan](https://puyonexus.com/wiki/Satan) | 1   | 2   | 3   | 5   | 7   | 9   | 11  | 13  | 15  | 19  | 23  | 27  | 31  | 39  | 47  | 54  | 62  | 70  | 78  | 86  | 94  | 101 | 109 | 117 | 4   |
| [Schezo](https://puyonexus.com/wiki/Schezo) | 1   | 2   | 3   | 5   | 7   | 9   | 11  | 13  | 15  | 19  | 23  | 27  | 31  | 39  | 47  | 54  | 62  | 70  | 78  | 86  | 94  | 101 | 109 | 117 | 4   |
| [Sig](https://puyonexus.com/wiki/Sig) | 1   | 2   | 4   | 6   | 8   | 10  | 12  | 14  | 16  | 20  | 24  | 29  | 33  | 41  | 49  | 58  | 66  | 74  | 83  | 91  | 99  | 108 | 116 | 124 | 2   |
| [Skeleton T](https://puyonexus.com/wiki/Skeleton_T) | 1   | 2   | 4   | 6   | 8   | 10  | 12  | 14  | 16  | 21  | 25  | 29  | 33  | 42  | 50  | 59  | 67  | 76  | 84  | 93  | 101 | 110 | 118 | 127 | 1   |
| [Suketoudara](https://puyonexus.com/wiki/Suketoudara) | 1   | 2   | 4   | 6   | 8   | 10  | 12  | 14  | 16  | 20  | 24  | 29  | 33  | 41  | 49  | 58  | 66  | 74  | 83  | 91  | 99  | 108 | 116 | 124 | 2   |
| Everybody | 1   | 2   | 4   | 6   | 8   | 10  | 12  | 14  | 16  | 20  | 24  | 28  | 32  | 40  | 48  | 56  | 64  | 72  | 80  | 88  | 96  | 104 | 112 | 120 | \-  |

#### _[Puyo Puyo!! 20th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!!_20th_Anniversary)_

These attack powers apply only to the [Fever rule](https://puyonexus.com/wiki/Fever_(rule)) and rules derived from it.

##### Normal

This game contains attack powers for up to a 24 chain. As all characters will reach the maximum attack power of 699 by their 19th chain, attack powers past that chain have been omitted.

| Character | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  | 17  | 18  | 19+ | Tier |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [Accord](https://puyonexus.com/wiki/Accord) | 0   | 8   | 17  | 23  | 36  | 74  | 125 | 192 | 260 | 330 | 420 | 512 | 617 | 699 | 699 | 699 | 699 | 699 | 699 | 2   |
| [Amitie](https://puyonexus.com/wiki/Amitie) | 0   | 8   | 17  | 22  | 34  | 67  | 112 | 168 | 224 | 280 | 350 | 420 | 490 | 560 | 630 | 699 | 699 | 699 | 699 | 6   |
| [Arle](https://puyonexus.com/wiki/Arle) | 0   | 8   | 17  | 23  | 35  | 71  | 118 | 178 | 239 | 300 | 377 | 454 | 534 | 613 | 693 | 699 | 699 | 699 | 699 | 5   |
| [Carbuncle](https://puyonexus.com/wiki/Carbuncle) | 0   | 8   | 17  | 22  | 34  | 67  | 112 | 168 | 224 | 280 | 350 | 420 | 490 | 560 | 630 | 699 | 699 | 699 | 699 | 6   |
| [Donguri Gaeru](https://puyonexus.com/wiki/Donguri_Gaeru) | 0   | 9   | 17  | 23  | 34  | 67  | 111 | 164 | 217 | 269 | 332 | 395 | 451 | 510 | 567 | 623 | 678 | 699 | 699 | 9   |
| [Draco Centauros](https://puyonexus.com/wiki/Draco_Centauros) | 0   | 8   | 17  | 24  | 36  | 74  | 125 | 188 | 253 | 319 | 402 | 487 | 578 | 666 | 699 | 699 | 699 | 699 | 699 | 3   |
| [Ecolo](https://puyonexus.com/wiki/Ecolo) | 0   | 8   | 17  | 23  | 36  | 74  | 125 | 192 | 260 | 330 | 420 | 512 | 617 | 699 | 699 | 699 | 699 | 699 | 699 | 2   |
| [Feli](https://puyonexus.com/wiki/Feli) | 0   | 8   | 15  | 20  | 29  | 57  | 94  | 141 | 187 | 232 | 289 | 344 | 397 | 451 | 504 | 556 | 608 | 655 | 699 | 11  |
| [Klug](https://puyonexus.com/wiki/Klug) | 0   | 8   | 16  | 23  | 36  | 77  | 132 | 202 | 274 | 350 | 447 | 546 | 661 | 699 | 699 | 699 | 699 | 699 | 699 | 1   |
| [Lemres](https://puyonexus.com/wiki/Lemres) | 0   | 8   | 17  | 22  | 34  | 67  | 112 | 168 | 224 | 280 | 350 | 420 | 490 | 560 | 630 | 699 | 699 | 699 | 699 | 6   |
| [Maguro](https://puyonexus.com/wiki/Maguro) | 0   | 9   | 17  | 23  | 34  | 67  | 111 | 164 | 217 | 269 | 332 | 395 | 451 | 510 | 567 | 623 | 678 | 699 | 699 | 9   |
| [Ocean Prince](https://puyonexus.com/wiki/Ocean_Prince) | 0   | 8   | 17  | 23  | 36  | 74  | 125 | 192 | 260 | 330 | 420 | 512 | 617 | 699 | 699 | 699 | 699 | 699 | 699 | 2   |
| [Onion Pixie](https://puyonexus.com/wiki/Onion_Pixie) | 0   | 8   | 15  | 21  | 31  | 64  | 107 | 161 | 216 | 272 | 342 | 412 | 485 | 557 | 630 | 699 | 699 | 699 | 699 | 7   |
| [Raffina](https://puyonexus.com/wiki/Raffina) | 0   | 8   | 17  | 23  | 36  | 74  | 125 | 192 | 260 | 330 | 420 | 512 | 617 | 699 | 699 | 699 | 699 | 699 | 699 | 2   |
| [Lidelle](https://puyonexus.com/wiki/Lidelle) | 0   | 8   | 17  | 24  | 36  | 74  | 125 | 188 | 253 | 319 | 402 | 487 | 578 | 666 | 699 | 699 | 699 | 699 | 699 | 3   |
| [Ringo](https://puyonexus.com/wiki/Ringo) | 0   | 8   | 15  | 20  | 30  | 60  | 101 | 151 | 202 | 252 | 315 | 378 | 441 | 504 | 567 | 630 | 693 | 699 | 699 | 10  |
| [Risukuma](https://puyonexus.com/wiki/Risukuma) | 0   | 8   | 15  | 21  | 31  | 64  | 107 | 161 | 216 | 272 | 342 | 412 | 485 | 557 | 630 | 699 | 699 | 699 | 699 | 7   |
| [Rulue](https://puyonexus.com/wiki/Rulue) | 0   | 8   | 17  | 23  | 36  | 74  | 125 | 192 | 260 | 330 | 420 | 512 | 617 | 699 | 699 | 699 | 699 | 699 | 699 | 2   |
| [Satan](https://puyonexus.com/wiki/Satan) | 0   | 8   | 16  | 23  | 36  | 71  | 117 | 175 | 232 | 288 | 359 | 428 | 536 | 676 | 699 | 699 | 699 | 699 | 699 | 4   |
| [Schezo](https://puyonexus.com/wiki/Schezo) | 0   | 8   | 16  | 23  | 36  | 77  | 132 | 202 | 274 | 350 | 447 | 546 | 661 | 699 | 699 | 699 | 699 | 699 | 699 | 1   |
| [Sig](https://puyonexus.com/wiki/Sig) | 0   | 8   | 15  | 20  | 30  | 60  | 101 | 151 | 202 | 252 | 315 | 378 | 441 | 504 | 567 | 630 | 693 | 699 | 699 | 10  |
| [Suketoudara](https://puyonexus.com/wiki/Suketoudara) | 0   | 8   | 15  | 20  | 29  | 57  | 94  | 141 | 187 | 232 | 289 | 344 | 397 | 451 | 504 | 556 | 608 | 655 | 699 | 11  |
| [Witch](https://puyonexus.com/wiki/Witch) | 0   | 8   | 16  | 21  | 32  | 64  | 106 | 160 | 213 | 266 | 333 | 399 | 465 | 532 | 598 | 628 | 654 | 678 | 699 | 8   |
| [Yu](https://puyonexus.com/wiki/Yu)<br> & [Rei](https://puyonexus.com/wiki/Rei) | 0   | 8   | 15  | 20  | 30  | 60  | 101 | 151 | 202 | 252 | 315 | 378 | 441 | 504 | 567 | 630 | 693 | 699 | 699 | 10  |

##### Fever

| Character | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  | 17  | 18  | 19  | 20  | 21  | 22  | 23  | 24+ | Tier |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [Accord](https://puyonexus.com/wiki/Accord) | 0   | 6   | 11  | 14  | 19  | 30  | 50  | 76  | 101 | 151 | 176 | 181 | 216 | 252 | 277 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 10  |
| [Amitie](https://puyonexus.com/wiki/Amitie) | 0   | 7   | 13  | 15  | 21  | 34  | 56  | 84  | 112 | 168 | 196 | 202 | 239 | 280 | 308 | 336 | 364 | 392 | 420 | 448 | 476 | 504 | 532 | 560 | 8   |
| [Arle](https://puyonexus.com/wiki/Arle) | 0   | 6   | 11  | 14  | 19  | 30  | 50  | 76  | 101 | 151 | 176 | 181 | 216 | 252 | 277 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 10  |
| [Carbuncle](https://puyonexus.com/wiki/Carbuncle) | 0   | 7   | 13  | 15  | 21  | 34  | 56  | 84  | 112 | 168 | 196 | 202 | 239 | 280 | 308 | 336 | 364 | 392 | 420 | 448 | 476 | 504 | 532 | 560 | 8   |
| [Donguri Gaeru](https://puyonexus.com/wiki/Donguri_Gaeru) | 0   | 7   | 13  | 15  | 20  | 32  | 53  | 79  | 105 | 156 | 181 | 186 | 219 | 255 | 279 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 9   |
| [Draco Centauros](https://puyonexus.com/wiki/Draco_Centauros) | 0   | 6   | 10  | 13  | 17  | 27  | 45  | 67  | 90  | 134 | 157 | 161 | 192 | 224 | 246 | 269 | 291 | 314 | 336 | 358 | 381 | 403 | 426 | 448 | 12  |
| [Ecolo](https://puyonexus.com/wiki/Ecolo) | 0   | 6   | 11  | 14  | 19  | 30  | 50  | 76  | 101 | 151 | 176 | 181 | 216 | 252 | 277 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 10  |
| [Feli](https://puyonexus.com/wiki/Feli) | 0   | 8   | 13  | 17  | 22  | 35  | 59  | 87  | 116 | 173 | 201 | 206 | 243 | 283 | 309 | 336 | 364 | 392 | 420 | 448 | 476 | 504 | 532 | 560 | 7   |
| [Klug](https://puyonexus.com/wiki/Klug) | 0   | 6   | 11  | 14  | 19  | 30  | 50  | 76  | 101 | 151 | 176 | 181 | 216 | 252 | 277 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 10  |
| [Lemres](https://puyonexus.com/wiki/Lemres) | 0   | 7   | 13  | 15  | 20  | 32  | 53  | 79  | 105 | 156 | 181 | 186 | 219 | 255 | 279 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 9   |
| [Maguro](https://puyonexus.com/wiki/Maguro) | 0   | 7   | 13  | 15  | 20  | 32  | 53  | 79  | 105 | 156 | 181 | 186 | 219 | 255 | 279 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 9   |
| [Ocean Prince](https://puyonexus.com/wiki/Ocean_Prince) | 0   | 6   | 11  | 14  | 19  | 30  | 50  | 76  | 101 | 151 | 176 | 181 | 216 | 252 | 277 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 10  |
| [Onion Pixie](https://puyonexus.com/wiki/Onion_Pixie) | 0   | 8   | 15  | 17  | 24  | 37  | 61  | 91  | 120 | 178 | 206 | 211 | 247 | 286 | 311 | 336 | 364 | 392 | 420 | 448 | 476 | 504 | 532 | 560 | 6   |
| [Raffina](https://puyonexus.com/wiki/Raffina) | 0   | 6   | 10  | 14  | 20  | 32  | 54  | 83  | 111 | 169 | 201 | 209 | 252 | 300 | 334 | 370 | 400 | 431 | 462 | 493 | 524 | 554 | 585 | 616 | 3   |
| [Lidelle](https://puyonexus.com/wiki/Lidelle) | 0   | 6   | 10  | 13  | 17  | 27  | 45  | 67  | 90  | 134 | 157 | 161 | 192 | 224 | 246 | 269 | 291 | 314 | 336 | 358 | 381 | 403 | 426 | 448 | 12  |
| [Ringo](https://puyonexus.com/wiki/Ringo) | 0   | 6   | 12  | 15  | 22  | 35  | 59  | 91  | 122 | 186 | 220 | 228 | 276 | 328 | 365 | 403 | 437 | 470 | 504 | 538 | 571 | 605 | 638 | 672 | 2   |
| [Risukuma](https://puyonexus.com/wiki/Risukuma) | 0   | 8   | 15  | 17  | 24  | 37  | 61  | 91  | 120 | 178 | 206 | 211 | 247 | 286 | 311 | 336 | 364 | 392 | 420 | 448 | 476 | 504 | 532 | 560 | 6   |
| [Rulue](https://puyonexus.com/wiki/Rulue) | 0   | 6   | 10  | 14  | 20  | 32  | 54  | 78  | 106 | 160 | 190 | 198 | 239 | 284 | 317 | 351 | 380 | 409 | 439 | 468 | 497 | 526 | 556 | 585 | 5   |
| [Satan](https://puyonexus.com/wiki/Satan) | 0   | 6   | 10  | 13  | 17  | 29  | 48  | 72  | 97  | 146 | 171 | 177 | 211 | 249 | 276 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 11  |
| [Schezo](https://puyonexus.com/wiki/Schezo) | 0   | 6   | 10  | 13  | 17  | 29  | 48  | 72  | 97  | 146 | 171 | 177 | 211 | 249 | 276 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 11  |
| [Sig](https://puyonexus.com/wiki/Sig) | 0   | 8   | 14  | 17  | 24  | 38  | 64  | 97  | 130 | 197 | 230 | 237 | 283 | 333 | 368 | 403 | 437 | 470 | 504 | 538 | 571 | 605 | 638 | 672 | 1   |
| [Suketoudara](https://puyonexus.com/wiki/Suketoudara) | 0   | 8   | 14  | 17  | 24  | 38  | 64  | 97  | 130 | 197 | 230 | 237 | 283 | 333 | 368 | 403 | 437 | 470 | 504 | 538 | 571 | 605 | 638 | 672 | 1   |
| [Witch](https://puyonexus.com/wiki/Witch) | 0   | 7   | 13  | 16  | 22  | 34  | 57  | 87  | 115 | 174 | 204 | 209 | 250 | 293 | 323 | 353 | 382 | 412 | 441 | 470 | 500 | 529 | 559 | 588 | 4   |
| [Yu](https://puyonexus.com/wiki/Yu)<br> & [Rei](https://puyonexus.com/wiki/Rei) | 0   | 6   | 12  | 15  | 22  | 35  | 59  | 91  | 122 | 186 | 220 | 228 | 276 | 328 | 365 | 403 | 437 | 470 | 504 | 538 | 571 | 605 | 638 | 672 | 2   |

#### _[Puyo Puyo Chronicle](https://puyonexus.com/wiki/Puyo_Puyo_Chronicle)_

These attack powers apply only to the [Fever rule](https://puyonexus.com/wiki/Fever_(rule)) and rules derived from it.

##### Normal

This game contains attack powers for up to a 24 chain. As all characters will reach the maximum attack power of 699 by their 19th chain, attack powers past that chain have been omitted.

| Character | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  | 17  | 18  | 19+ | Tier |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [Accord](https://puyonexus.com/wiki/Accord) | 0   | 8   | 17  | 23  | 36  | 74  | 125 | 192 | 260 | 330 | 420 | 512 | 617 | 699 | 699 | 699 | 699 | 699 | 699 | 2   |
| [Ally](https://puyonexus.com/wiki/Ally) | 0   | 8   | 17  | 23  | 35  | 71  | 118 | 178 | 239 | 300 | 377 | 454 | 534 | 613 | 693 | 699 | 699 | 699 | 699 | 5   |
| [Amitie](https://puyonexus.com/wiki/Amitie) | 0   | 8   | 17  | 22  | 34  | 67  | 112 | 168 | 224 | 280 | 350 | 420 | 490 | 560 | 630 | 699 | 699 | 699 | 699 | 6   |
| [Arle](https://puyonexus.com/wiki/Arle) | 0   | 8   | 17  | 23  | 35  | 71  | 118 | 178 | 239 | 300 | 377 | 454 | 534 | 613 | 693 | 699 | 699 | 699 | 699 | 5   |
| [Carbuncle](https://puyonexus.com/wiki/Carbuncle) | 0   | 8   | 17  | 22  | 34  | 67  | 112 | 168 | 224 | 280 | 350 | 420 | 490 | 560 | 630 | 699 | 699 | 699 | 699 | 6   |
| [Draco Centauros](https://puyonexus.com/wiki/Draco_Centauros) | 0   | 8   | 17  | 24  | 36  | 74  | 125 | 188 | 253 | 319 | 402 | 487 | 578 | 666 | 699 | 699 | 699 | 699 | 699 | 3   |
| [Ecolo](https://puyonexus.com/wiki/Ecolo) | 0   | 8   | 17  | 23  | 36  | 74  | 125 | 192 | 260 | 330 | 420 | 512 | 617 | 699 | 699 | 699 | 699 | 699 | 699 | 2   |
| [Feli](https://puyonexus.com/wiki/Feli) | 0   | 8   | 15  | 20  | 29  | 57  | 94  | 141 | 187 | 232 | 289 | 344 | 397 | 451 | 504 | 556 | 608 | 655 | 699 | 11  |
| [Klug](https://puyonexus.com/wiki/Klug) | 0   | 8   | 16  | 23  | 36  | 77  | 132 | 202 | 274 | 350 | 447 | 546 | 661 | 699 | 699 | 699 | 699 | 699 | 699 | 1   |
| [Lemres](https://puyonexus.com/wiki/Lemres) | 0   | 8   | 17  | 22  | 34  | 67  | 112 | 168 | 224 | 280 | 350 | 420 | 490 | 560 | 630 | 699 | 699 | 699 | 699 | 6   |
| [Maguro](https://puyonexus.com/wiki/Maguro) | 0   | 9   | 17  | 23  | 34  | 67  | 111 | 164 | 217 | 269 | 332 | 395 | 451 | 510 | 567 | 623 | 678 | 699 | 699 | 9   |
| [Ocean Prince](https://puyonexus.com/wiki/Ocean_Prince) | 0   | 8   | 15  | 20  | 30  | 60  | 101 | 151 | 202 | 252 | 315 | 378 | 441 | 504 | 567 | 630 | 693 | 699 | 699 | 10  |
| [Raffina](https://puyonexus.com/wiki/Raffina) | 0   | 8   | 17  | 23  | 36  | 74  | 125 | 192 | 260 | 330 | 420 | 512 | 617 | 699 | 699 | 699 | 699 | 699 | 699 | 2   |
| [Rafisol](https://puyonexus.com/wiki/Rafisol) | 0   | 8   | 16  | 23  | 36  | 77  | 132 | 202 | 274 | 350 | 447 | 546 | 661 | 699 | 699 | 699 | 699 | 699 | 699 | 1   |
| [Lidelle](https://puyonexus.com/wiki/Lidelle) | 0   | 8   | 17  | 24  | 36  | 74  | 125 | 188 | 253 | 319 | 402 | 487 | 578 | 666 | 699 | 699 | 699 | 699 | 699 | 3   |
| [Ringo](https://puyonexus.com/wiki/Ringo) | 0   | 8   | 15  | 20  | 30  | 60  | 101 | 151 | 202 | 252 | 315 | 378 | 441 | 504 | 567 | 630 | 693 | 699 | 699 | 10  |
| [Risukuma](https://puyonexus.com/wiki/Risukuma) | 0   | 8   | 15  | 21  | 31  | 64  | 107 | 161 | 216 | 272 | 342 | 412 | 485 | 557 | 630 | 699 | 699 | 699 | 699 | 7   |
| [Rulue](https://puyonexus.com/wiki/Rulue) | 0   | 8   | 17  | 23  | 36  | 74  | 125 | 192 | 260 | 330 | 420 | 512 | 617 | 699 | 699 | 699 | 699 | 699 | 699 | 2   |
| [Satan](https://puyonexus.com/wiki/Satan) | 0   | 8   | 16  | 23  | 36  | 71  | 117 | 175 | 232 | 288 | 359 | 428 | 536 | 676 | 699 | 699 | 699 | 699 | 699 | 4   |
| [Schezo](https://puyonexus.com/wiki/Schezo) | 0   | 8   | 16  | 23  | 36  | 77  | 132 | 202 | 274 | 350 | 447 | 546 | 661 | 699 | 699 | 699 | 699 | 699 | 699 | 1   |
| [Sig](https://puyonexus.com/wiki/Sig) | 0   | 8   | 15  | 20  | 30  | 60  | 101 | 151 | 202 | 252 | 315 | 378 | 441 | 504 | 567 | 630 | 693 | 699 | 699 | 10  |
| [Suketoudara](https://puyonexus.com/wiki/Suketoudara) | 0   | 8   | 15  | 20  | 29  | 57  | 94  | 141 | 187 | 232 | 289 | 344 | 397 | 451 | 504 | 556 | 608 | 655 | 699 | 11  |
| [Witch](https://puyonexus.com/wiki/Witch) | 0   | 8   | 16  | 21  | 32  | 64  | 106 | 160 | 213 | 266 | 333 | 399 | 465 | 532 | 598 | 628 | 654 | 678 | 699 | 8   |
| [Yu](https://puyonexus.com/wiki/Yu)<br> & [Rei](https://puyonexus.com/wiki/Rei) | 0   | 8   | 15  | 20  | 30  | 60  | 101 | 151 | 202 | 252 | 315 | 378 | 441 | 504 | 567 | 630 | 693 | 699 | 699 | 10  |

##### Fever

| Character | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | 10  | 11  | 12  | 13  | 14  | 15  | 16  | 17  | 18  | 19  | 20  | 21  | 22  | 23  | 24+ | Tier |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| [Accord](https://puyonexus.com/wiki/Accord) | 0   | 6   | 11  | 14  | 19  | 30  | 50  | 76  | 101 | 151 | 176 | 181 | 216 | 252 | 277 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 11  |
| [Ally](https://puyonexus.com/wiki/Ally) | 0   | 6   | 11  | 14  | 19  | 30  | 50  | 76  | 101 | 151 | 176 | 181 | 216 | 252 | 277 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 11  |
| [Amitie](https://puyonexus.com/wiki/Amitie) | 0   | 7   | 13  | 15  | 21  | 34  | 56  | 84  | 112 | 168 | 196 | 202 | 239 | 280 | 308 | 336 | 364 | 392 | 420 | 448 | 476 | 504 | 532 | 560 | 9   |
| [Arle](https://puyonexus.com/wiki/Arle) | 0   | 6   | 11  | 14  | 19  | 30  | 50  | 76  | 101 | 151 | 176 | 181 | 216 | 252 | 277 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 11  |
| [Carbuncle](https://puyonexus.com/wiki/Carbuncle) | 0   | 7   | 13  | 15  | 21  | 34  | 56  | 84  | 112 | 168 | 196 | 202 | 239 | 280 | 308 | 336 | 364 | 392 | 420 | 448 | 476 | 504 | 532 | 560 | 9   |
| [Draco Centauros](https://puyonexus.com/wiki/Draco_Centauros) | 0   | 6   | 10  | 13  | 17  | 27  | 45  | 67  | 90  | 134 | 157 | 161 | 192 | 224 | 246 | 269 | 291 | 314 | 336 | 358 | 381 | 403 | 426 | 448 | 13  |
| [Ecolo](https://puyonexus.com/wiki/Ecolo) | 0   | 6   | 11  | 14  | 19  | 30  | 50  | 76  | 101 | 151 | 176 | 181 | 216 | 252 | 277 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 11  |
| [Feli](https://puyonexus.com/wiki/Feli) | 0   | 8   | 13  | 17  | 22  | 35  | 59  | 87  | 116 | 173 | 201 | 206 | 243 | 283 | 309 | 336 | 364 | 392 | 420 | 448 | 476 | 504 | 532 | 560 | 8   |
| [Klug](https://puyonexus.com/wiki/Klug) | 0   | 6   | 11  | 14  | 19  | 30  | 50  | 76  | 101 | 151 | 176 | 181 | 216 | 252 | 277 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 11  |
| [Lemres](https://puyonexus.com/wiki/Lemres) | 0   | 7   | 13  | 15  | 20  | 32  | 53  | 79  | 105 | 156 | 181 | 186 | 219 | 255 | 279 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 10  |
| [Maguro](https://puyonexus.com/wiki/Maguro) | 0   | 7   | 13  | 15  | 20  | 32  | 53  | 79  | 105 | 156 | 181 | 186 | 219 | 255 | 279 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 10  |
| [Ocean Prince](https://puyonexus.com/wiki/Ocean_Prince) | 0   | 7   | 13  | 17  | 24  | 38  | 65  | 99  | 134 | 203 | 240 | 248 | 300 | 356 | 395 | 437 | 473 | 510 | 546 | 582 | 619 | 655 | 692 | 699 | 1   |
| [Raffina](https://puyonexus.com/wiki/Raffina) | 0   | 6   | 10  | 14  | 20  | 32  | 54  | 83  | 111 | 169 | 201 | 209 | 252 | 300 | 334 | 370 | 400 | 431 | 462 | 493 | 524 | 554 | 585 | 616 | 4   |
| [Rafisol](https://puyonexus.com/wiki/Rafisol) | 0   | 6   | 11  | 14  | 19  | 30  | 50  | 76  | 101 | 151 | 176 | 181 | 216 | 252 | 277 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 11  |
| [Lidelle](https://puyonexus.com/wiki/Lidelle) | 0   | 6   | 10  | 13  | 17  | 27  | 45  | 67  | 90  | 134 | 157 | 161 | 192 | 224 | 246 | 269 | 291 | 314 | 336 | 358 | 381 | 403 | 426 | 448 | 13  |
| [Ringo](https://puyonexus.com/wiki/Ringo) | 0   | 6   | 12  | 15  | 22  | 35  | 59  | 91  | 122 | 186 | 220 | 228 | 276 | 328 | 365 | 403 | 437 | 470 | 504 | 538 | 571 | 605 | 638 | 672 | 3   |
| [Risukuma](https://puyonexus.com/wiki/Risukuma) | 0   | 8   | 15  | 17  | 24  | 37  | 61  | 91  | 120 | 178 | 206 | 211 | 247 | 286 | 311 | 336 | 364 | 392 | 420 | 448 | 476 | 504 | 532 | 560 | 7   |
| [Rulue](https://puyonexus.com/wiki/Rulue) | 0   | 6   | 10  | 14  | 20  | 32  | 54  | 78  | 106 | 160 | 190 | 198 | 239 | 284 | 317 | 351 | 380 | 409 | 439 | 468 | 497 | 526 | 556 | 585 | 6   |
| [Satan](https://puyonexus.com/wiki/Satan) | 0   | 6   | 10  | 13  | 17  | 29  | 48  | 72  | 97  | 146 | 171 | 177 | 211 | 249 | 276 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 12  |
| [Schezo](https://puyonexus.com/wiki/Schezo) | 0   | 6   | 10  | 13  | 17  | 29  | 48  | 72  | 97  | 146 | 171 | 177 | 211 | 249 | 276 | 302 | 328 | 353 | 378 | 403 | 428 | 454 | 479 | 504 | 12  |
| [Sig](https://puyonexus.com/wiki/Sig) | 0   | 8   | 14  | 17  | 24  | 38  | 64  | 97  | 130 | 197 | 230 | 237 | 283 | 333 | 368 | 403 | 437 | 470 | 504 | 538 | 571 | 605 | 638 | 672 | 2   |
| [Suketoudara](https://puyonexus.com/wiki/Suketoudara) | 0   | 8   | 14  | 17  | 24  | 38  | 64  | 97  | 130 | 197 | 230 | 237 | 283 | 333 | 368 | 403 | 437 | 470 | 504 | 538 | 571 | 605 | 638 | 672 | 2   |
| [Witch](https://puyonexus.com/wiki/Witch) | 0   | 7   | 13  | 16  | 22  | 34  | 57  | 87  | 115 | 174 | 204 | 209 | 250 | 293 | 323 | 353 | 382 | 412 | 441 | 470 | 500 | 529 | 559 | 588 | 5   |
| [Yu](https://puyonexus.com/wiki/Yu)<br> & [Rei](https://puyonexus.com/wiki/Rei) | 0   | 6   | 12  | 15  | 22  | 35  | 59  | 91  | 122 | 186 | 220 | 228 | 276 | 328 | 365 | 403 | 437 | 470 | 504 | 538 | 571 | 605 | 638 | 672 | 3   |


## Margin time

*Source: <https://puyonexus.com/wiki/Margin_time> &mdash; 3 diagrams omitted*

The default margin time of [Puyo Puyo 2](https://puyonexus.com/wiki/Tsu_(rule)) in _[Puyo Puyo Champions](https://puyonexus.com/wiki/Puyo_Puyo_Champions)_.

 The default margin time of [Fever](https://puyonexus.com/wiki/Fever_(rule)) in _[Puyo Puyo Champions](https://puyonexus.com/wiki/Puyo_Puyo_Champions)_.

 The Nuisance Puyo multiplier in _[Puyo Puyo!! 20th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!!_20th_Anniversary)_.

**Margin time** is the time period that indicates how long it takes before the target points starts to decrease. When the number of seconds in the match reaches the target points, the target points begin to decrease (initially by 4/3, otherwise known as being multiplied by 3/4 or 0.75), and will continue to decrease every 16 seconds after that until the target points have been reduced to 1 or this has gone on for 14 iterations, whichever comes first.

In _[Puyo Puyo!! 20th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!!_20th_Anniversary)_, this is shown to the player as a Nuisance Puyo multiplier.

Margin Time was first implemented in _[Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu)_ to ensure that matches did not last too long, and has been present in every Puyo Puyo game since.

#### Contents

*   [1 Pseudocode](https://puyonexus.com/wiki/Margin_time#Pseudocode)

*   [2 Examples](https://puyonexus.com/wiki/Margin_time#Examples)
    *   [2.1 Target Points = 120](https://puyonexus.com/wiki/Margin_time#Target_Points_=_120)

    *   [2.2 Target Points = 990](https://puyonexus.com/wiki/Margin_time#Target_Points_=_990)

    *   [2.3 Target Points = 10](https://puyonexus.com/wiki/Margin_time#Target_Points_=_10)

*   [3 Resources](https://puyonexus.com/wiki/Margin_time#Resources)


#### Pseudocode

The way the margin time works is similar to this, shown using code (note that _targetPoints_ is always rounded down):

`   _targetPoints_ = _initalTargetPoints_  _previousTargetPoints_ = 0  _numIterations_ = 0  Set _timer_ to call **ReduceTargetPoints** in _marginTime_ seconds     ``       ``   Function **ReduceTargetPoints**  _currentTargetPoints_ = _targetPoints_  If _numIterations_ is equal to 0  _targetPoints_ = _initialTargetPoints_ * 0.75  Else  _targetPoints_ = _previousTargetPoints_ / 2  _previousTargetPoints_ = _currentTargetPoints_  Increase _numIterations_ by 1  If _targetPoints_ is greater than 1 and _numIterations_ is less than 14  Set _timer_ to call **ReduceTargetPoints** in 16 seconds  End Function     `

In Puyo Puyo!! 20th Anniversary, the reduction of target points is shown as an increase to the Nuisance Puyo multiplier. The Nuisance Puyo multiplier works like this, with the Nuisance Puyo multiplier always being rounded down to 2 decimal places:

`   _nuisancePuyoMultiplier_ = _initialTargetPoints_ / _targetPoints_     `

#### Examples

##### Target Points = 120

This is the standard used for Fever matches.

| Iteration | Target Points | Nuisance Puyo Multiplier |
| --- | --- | --- |
| 0 (Initial) | 120 | x1.00 |
| 1   | 90  | x1.33 |
| 2   | 60  | x2.00 |
| 3   | 45  | x2.66 |
| 4   | 30  | x4.00 |
| 5   | 22  | x5.45 |
| 6   | 15  | x8.00 |
| 7   | 11  | x10.90 |
| 8   | 7   | x17.14 |
| 9   | 5   | x24.00 |
| 10  | 3   | x40.00 |
| 11  | 2   | x60.00 |
| 12  | 1   | x120.00 |

##### Target Points = 990

This is to show that the target points will never reach 1.

| Iteration | Target Points | Nuisance Puyo Multiplier |
| --- | --- | --- |
| 0 (Initial) | 990 | x1.00 |
| 1   | 742 | x1.33 |
| 2   | 495 | x2.00 |
| 3   | 371 | x2.66 |
| 4   | 247 | x4.00 |
| 5   | 185 | x5.35 |
| 6   | 123 | x8.04 |
| 7   | 92  | x10.76 |
| 8   | 61  | x16.22 |
| 9   | 46  | x21.52 |
| 10  | 30  | x33.00 |
| 11  | 23  | x43.04 |
| 12  | 15  | x66.00 |
| 13  | 11  | x90.00 |
| 14  | 7   | x141.42 |

##### Target Points = 10

This is to show that the target points will reach 1 before it goes through all its iterations.

| Iteration | Target Points | Nuisance Puyo Multiplier |
| --- | --- | --- |
| 0 (Initial) | 10  | x1.00 |
| 1   | 7   | x1.42 |
| 2   | 5   | x2.00 |
| 3   | 3   | x3.33 |
| 4   | 2   | x5.00 |
| 5   | 1   | x10.00 |

#### Resources

*   [Inosendo's guide on margin time](http://www.inosendo.com/puyo/margintime.html)
     (Japanese)


## Nuisance queue

*Source: <https://puyonexus.com/wiki/Nuisance_queue> &mdash; 36 diagrams omitted*

> A waiting area for Garbage Puyos sent by your opponent. These appear in various sizes and shapes relative to the number of Garbage Puyos waiting to rain down any moment.
>
> —_Puyo Puyo Tetris 2_ Gameplay Manual[\[1\]](https://puyonexus.com/wiki/Nuisance_queue#cite_note-1)

 Garbage queues in a four-player battle.

The **Nuisance Queue**, or **Garbage Block Preview** for _Tetris_, is an area located above a character's playing field that shows how many Garbage Puyos are waiting to fall. All games in the _[Puyo Puyo](https://puyonexus.com/wiki/Puyo_Puyo)_ series use symbols instead of numbers to warn the player of incoming Garbage Puyos.

#### Contents

*   [1 General](https://puyonexus.com/wiki/Nuisance_queue#General)
    *   [1.1 Standard](https://puyonexus.com/wiki/Nuisance_queue#Standard)

    *   [1.2 Other symbols](https://puyonexus.com/wiki/Nuisance_queue#Other_symbols)

    *   [1.3 Fever queue](https://puyonexus.com/wiki/Nuisance_queue#Fever_queue)

    *   [1.4 _Puyo Puyo Tetris 1_ / _2_](https://puyonexus.com/wiki/Nuisance_queue#Puyo_Puyo_Tetris_1_/_2)

*   [2 _Puyo Puyo~n_](https://puyonexus.com/wiki/Nuisance_queue#Puyo_Puyo~n)

*   [3 Trivia](https://puyonexus.com/wiki/Nuisance_queue#Trivia)

*   [4 References](https://puyonexus.com/wiki/Nuisance_queue#References)


#### General

Most _Puyo Puyo_ games use a similar set of symbols to indicate to players for any pending garbage. In newer games, their appearances can be changed by selecting a different Puyo skin. The nuisance queue can show up to six symbols before using another symbol that represents a larger amount of garbage.

##### Standard

From _[Puyo Puyo! 15th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary)_ onward, the games consistently use the following garbage symbols:

| Symbol |     | No. | Notes |
| --- | --- | --- | --- |
|  | Small | 1   | Single units of garbage may no longer be displayed if the player receives too much Garbage Puyos. |
|  | Large | 6   | On a regular playing field, this equates to one row of Garbage Puyos. |
|  | Rock | 30  | This is the maximum number of Garbage Puyos that can fall at once; this equates to five rows of Garbage Puyos.  <br>Games that use the [Puyo Puyo rule](https://puyonexus.com/wiki/Puyo_Puyo_(rule))<br> cap the garbage symbols at this icon. |
|  | Star | 180 | If a player receives two star symbols in their nuisance queue, the game will display them as one moon symbol.  <br>This pattern is observed up to the penultimate garbage symbol. |
|  | Moon | 360 |     |
|  | Crown | 720 | This is the penultimate indicator of garbage.  <br>_Puyo Puyo Fever_ and _Puyo Puyo Fever 2_ caps the garbage symbol at this icon. |
|  | Comet | 1440 | First appeared in _Puyo Puyo Sun_. Starting _Puyo Puyo! 15th Anniversary_, this icon represents  <br>the greatest number of Garbage Puyos. All succeeding games cap the garbage symbol at this icon. |

##### Other symbols

| Symbol |     | No. | Notes |
| --- | --- | --- | --- |
|  | Mushroom[\[2\]](https://puyonexus.com/wiki/Nuisance_queue#cite_note-2) | 200 | _Puyo Puyo Tsu_ uses this icon as a representative for 200 Garbage Puyos before being replaced with the star symbol. |
|  | Star (_Tsu_) | 300 | _Puyo Puyo Tsu_ uses this icon as a representative for 300 Garbage Puyos before being replaced with the crown symbol. |
|  | Crown (_Tsu_) | 420 | _Puyo Puyo Tsu_ uses this icon as a representative for 420 Garbage Puyos and caps the garbage symbol at this icon. |
|  | Comet (_Sun_) | 720 | _Puyo Puyo Sun_ uses this icon as a representative for 720 Garbage Puyos before being replaced with the Saturn symbol. |
|  | Saturn (_Sun_) | 1440 | _Puyo Puyo Sun_ uses this icon as a representative for 1440 Garbage Puyos and caps the garbage symbol at this icon. |

##### Fever queue

> _Main article: [Fever (rule)](https://puyonexus.com/wiki/Fever_(rule))
> _

Entering [Fever mode](https://puyonexus.com/wiki/Fever_mode) renders the main garbage queue inactive, visually depicted by it darkening and getting "pushed" in the background. All garbage received during Fever will be displayed in a second queue, and _will_ fall if not neutralized. Garbage Puyos in the main queue can still be neutralized if the second queue is empty.

If the player receives nuisance in the second queue but then releases a chain in order to offset, then they will have to neutralise the second queue first, then they would be able to offset their first queue, and thus would send it to one of the two queues for the other player(s), depending if the other player(s) is/are in Fever mode or not, as usual.

Once Fever mode ends, any remaining garbage from the second queue will be added to the main one as the player returns to their original playing field.

##### _Puyo Puyo Tetris 1_ / _2_

Unlike other _Tetris_ games, _[Puyo Puyo Tetris](https://puyonexus.com/wiki/Puyo_Puyo_Tetris)_ and _[Puyo Puyo Tetris 2](https://puyonexus.com/wiki/Puyo_Puyo_Tetris_2)_ also use symbols to warn _Tetris_ players of incoming garbage lines. A maximum of seven garbage lines (one large and small Garbage Puyo symbol) can rise at once if a _Tetris_ player fails to offset garbage or clear a line.

#### _Puyo Puyo~n_

_[Puyo Puyo~n](https://puyonexus.com/wiki/Puyo_Puyo~n)_ uses nearly the same garbage symbols as its predecessors along with other symbols that denote very high quantities of Garbage Puyos. Sending massive quantities of garbage is possible with the use of Point Puyos or the very small playing field in the Endless Puyo Puyo mode exclusive to the Dreamcast version.[\[3\]](https://puyonexus.com/wiki/Nuisance_queue#cite_note-3)

| Symbol |     | No. | Notes |
| --- | --- | --- | --- |
|  | Small | 1   |     |
|  | Large | 6   | Equal to six Garbage Puyos; this equates to 1 row on the regular field. |
|  | Rock | 30  | Equal to 5 rows (30 nuisance Puyo), which is the maximum that can fall at once. |
|  | Star | 90  | Created by 3 Rock Puyos. |
|  | Moon | 180 | Created by two Star Puyos. |
|  | Comet | 360 | Created from two Moon Puyos. |
|  | Saturn | 720 | Created from two Comet Puyos. The Saturn Puyo also appears in _Sun_. |
|  | Club | 1000 | Equal to one Saturn Puyo, one Moon Puyo, one Star Puyo, one Large Puyo and four Small Puyos combined. |
|  | Diamond | 5000 | Created from 5 Club Puyos. |
|  | Heart | 20000 | Created from 4 Diamond Puyos. |
|  | Spade | 100000 | Created from 5 Heart Puyos. |
|  | Crown | 500000 | Created from 5 Spade Puyos. |
|  | Mushroom | 2 million | Created from 4 Crown Puyos. |
|  | [Sun](https://puyonexus.com/wiki/Types_of_Puyo#Sun_Puyo) | 10 million | Created from 5 Mushroom Puyos. |
|  | Top Hat | 50 million | Created from 5 Sun Puyos. |
|  | Ball | 200 million | Created from 4 Top Hat Puyos. |
|  | Tent | 1 billion | Created from 5 Ball Puyos. |
|  | GD-ROM | 5 billion | Created from 5 Tent Puyos. |
|  | Blue Swirl | 10 billion | Created from 2 GD-ROM Puyos. |
|  | Green Swirl | 50 billion | Created from 5 Blue Swirl Puyos. |
|  | Yellow Swirl | 100 billion | Created from 2 Green Swirl Puyos. |
|  | Purple Swirl | 500 billion | Created from 5 Yellow Swirl Puyos. |
|  | Red Swirl | 1 trillion | Created from 2 Purple Swirl Puyos. |

#### Trivia

*   In _[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever) _ and [its sequel](https://puyonexus.com/wiki/Puyo_Puyo_Fever_2) , the garbage queue "bars" were visible.
*   The nuisance queue "update" animation slightly varies depending on the game.
    *   In the [Compile](https://puyonexus.com/wiki/Compile)
        \-era games, the icons slide away from the center.
    *   In the Fever series, the icons slide away from the left.
    *   Starting _Puyo Puyo! 15th Anniversary_, the icons now slide towards then away from the center.
*   In _[Puyo Puyo Sun](https://puyonexus.com/wiki/Puyo_Puyo_Sun) _, the Saturn Garbage Puyo occupies a larger space than other symbols - a maximum of four can be displayed at once.
*   Formerly, in Compile-era _Puyo Puyo_ games, there could be more than 6 symbols displayed at once. In _Puyo Puyo Tsu_ (Mega Drive version), it is presumed that up to 9 symbols can be displayed at once, especially to warn players 29 nuisance Puyo and similar.

#### References

1.  [↑](https://puyonexus.com/wiki/Nuisance_queue#cite_ref-1)
     [https://cdn.akamai.steamstatic.com/steam/apps/1259790/manuals/tenpex](https://cdn.akamai.steamstatic.com/steam/apps/1259790/manuals/tenpex) WM 210216 Steam en.pdf?t=1616537257
2.  [↑](https://puyonexus.com/wiki/Nuisance_queue#cite_ref-2)
     [https://twitter.com/s2lsoftener/status/976444435750899712](https://twitter.com/s2lsoftener/status/976444435750899712) 3.  [↑](https://puyonexus.com/wiki/Nuisance_queue#cite_ref-3)
     [http://Puyonexus.com/forum/viewtopic.php?f=11&t=3448](https://puyonexus.com/forum/viewtopic.php?f=11&t=3448)


## Offset rule

*Source: <https://puyonexus.com/wiki/Offset_rule> &mdash; 1 diagrams omitted*

The **offset rule** (相殺 _Sousai_, **neutralizing** in the English version of _[Puyo Puyo Tetris](https://puyonexus.com/wiki/Puyo_Puyo_Tetris)_) is a game mechanic introduced in _[Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu)_, having many variants in subsequent games. In every implementation, the player can reduce or "offset" the number of incoming Garbage Puyos with their own chain. Should the player completely negate all incoming Garbage Puyos, any excess will be sent to the opponent.

#### Contents

*   [1 Offset variants](https://puyonexus.com/wiki/Offset_rule#Offset_variants)

*   [2 Mechanics related to the offset rule](https://puyonexus.com/wiki/Offset_rule#Mechanics_related_to_the_offset_rule)
    *   [2.1 Puyo Puyo SUN](https://puyonexus.com/wiki/Offset_rule#Puyo_Puyo_SUN)

    *   [2.2 Puyo Puyo~n](https://puyonexus.com/wiki/Offset_rule#Puyo_Puyo~n)

    *   [2.3 Puyo Puyo Fever and Puyo Puyo Fever 2](https://puyonexus.com/wiki/Offset_rule#Puyo_Puyo_Fever_and_Puyo_Puyo_Fever_2)

    *   [2.4 Puyo Puyo 7](https://puyonexus.com/wiki/Offset_rule#Puyo_Puyo_7)

*   [3 Chain Requirement](https://puyonexus.com/wiki/Offset_rule#Chain_Requirement)

*   [4 Aesthetics](https://puyonexus.com/wiki/Offset_rule#Aesthetics)

*   [5 Trivia](https://puyonexus.com/wiki/Offset_rule#Trivia)


#### Offset variants

There are two primary offset rule variants, described and (unofficially) named below:

*   **Classic offset** is the offset rule originating in _Puyo Puyo Tsu_. In games that utilize Classic offset, any garbage waiting above a player's field drops into play as soon as that player's chain finishes. This means that the player generally, but not always, has exactly one chain to try to mitigate the garbage that waits for them.
*   **Continuous offset** is the offset rule originating in _[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever) _. Unlike Classic offset, any chain that the player creates will prevent Garbage Puyo from dropping into their field; this continues until the player places a piece that doesn't clear any Puyo.

#### Mechanics related to the offset rule

In subsequent games after _[Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu)_, offsetting may also include added effects aside from simply negating queued Garbage Puyos.

##### Puyo Puyo SUN

> _Main article: [Sun (rule)](https://puyonexus.com/wiki/Sun_(rule))
> _

In _[Puyo Puyo Sun](https://puyonexus.com/wiki/Puyo_Puyo_Sun)_, offsetting gives the player [Sun Puyos](https://puyonexus.com/wiki/Types_of_Puyo#Sun_Puyo)
. They increase the amount of sent garbage, although provides no score bonus to a chain. Light beams will appear on the player's field, indicating the columns where Sun Puyos will fall. No Garbage Puyos will fall on a turn that the player receives Sun Puyo into their field.

##### Puyo Puyo~n

> _Main article: [Yon (rule)](https://puyonexus.com/wiki/Yon_(rule))
> _

In _[Puyo Puyo~n](https://puyonexus.com/wiki/Puyo_Puyo~n)_, offsetting gives additional charge for [super attacks](https://puyonexus.com/wiki/Super_attack)
.

##### Puyo Puyo Fever and Puyo Puyo Fever 2

> _Main article: [Fever (rule)](https://puyonexus.com/wiki/Fever_(rule))
> _

In _[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_ and its [sequel](https://puyonexus.com/wiki/Puyo_Puyo_Fever_2)
, offsetting adds a point to the player's [Fever gauge](https://puyonexus.com/wiki/Fever_(rule)#Fever_gauge)
. In _[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_, _[Puyo Puyo Fever 2](https://puyonexus.com/wiki/Puyo_Puyo_Fever_2)_, _[Puyo Puyo!! 20th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!!_20th_Anniversary)_, and _[Puyo Puyo Champions](https://puyonexus.com/wiki/Puyo_Puyo_eSports)_, offsetting adds one second to the opponent's Fever time while in _[Puyo Puyo! 15th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary)_, and _[Puyo Puyo 7](https://puyonexus.com/wiki/Puyo_Puyo_7)_, making chains without offsetting adds one second to the player's own Fever time.

##### Puyo Puyo 7

> _Main article: [Transformation](https://puyonexus.com/wiki/Transformation)
> _

In the Transformation rule in _[Puyo Puyo 7](https://puyonexus.com/wiki/Puyo_Puyo_7)_, offsetting adds one point to the player's Transformation gauge. Aside from this, all other offsetting mechanics in Transformation work similarly to the Fever rule.

#### Chain Requirement

A related rule also introduced in _[Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu)_ is **Rensa Shibari** (lit. Chain Requirement, a.k.a. Target Chain). When active, no garbage will be sent until the specified chain length requirement is met. For instance, setting this rule to "5" will only cause chains longer than five to send garbage, thus a 4-chain or less will not send any garbage. This is important in games with more complicated variants of the offset rule, as the extra action triggered when offsetting does _not_ occur even if a chain is made.

#### Aesthetics

 An example of the offset indicator in _[Puyo Puyo Tetris 2](https://puyonexus.com/wiki/Puyo_Puyo_Tetris_2)_. Here, [Lidelle](https://puyonexus.com/wiki/Lidelle) is offsetting the Garbage Puyos sent by [Marle](https://puyonexus.com/wiki/Marle)
, with the former jostling the latter as the amount of the queued Garbage Puyos decreases.

Aesthetically, in _Puyo Puyo Tsu_ and _[Puyo Puyo Sun](https://puyonexus.com/wiki/Puyo_Puyo_Sun)_, sparkles can be seen on a player's Garbage queue when offsetting. Starting with _[Puyo Puyo! 15th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary)_ (except _[Puyo Puyo 7](https://puyonexus.com/wiki/Puyo_Puyo_7)_ for unknown reasons), when a player clears all the Garbage Puyos in the queue and sending them back to the opponent with at least 3 chain, the character vocalizes a unique line (these are referred by "Counter" on the chants table on the respective character page, or their Chants sub-page). In _[Puyo Puyo Champions](https://puyonexus.com/wiki/Puyo_Puyo_eSports)_ and _[Puyo Puyo Tetris 2](https://puyonexus.com/wiki/Puyo_Puyo_Tetris_2)_, the action of offsetting gained a visual indicator (except when a single offset is repeated in [Fever](https://puyonexus.com/wiki/Fever_(rule)) in _Champions_, and in [Party](https://puyonexus.com/wiki/Party) and [Fusion](https://puyonexus.com/wiki/Fusion) in _Tetris 2_) above the Garbage queue, which having the offsetting character jostling the opposing character (and vice versa if the opponent makes a chain during the offset). If there are more than 2 players are playing, a smaller indicator appears below the offsetting player's field, and the portrait of the opposing character is replaced by that of a pile of Garbage Puyos.

#### Trivia

*   Despite being formally introduced in _[Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu) _, the Offset rule was already accessible in _Super Puyo Puyo_ through the use of a special options menu accessed through cheats.


## Rotation

*Source: <https://puyonexus.com/wiki/Rotation> &mdash; 9 diagrams omitted*

#### Contents

*   [1 Floor kick](https://puyonexus.com/wiki/Rotation#Floor_kick)

*   [2 Wall kick](https://puyonexus.com/wiki/Rotation#Wall_kick)

*   [3 YON rule](https://puyonexus.com/wiki/Rotation#YON_rule)

*   [4 Double rotate](https://puyonexus.com/wiki/Rotation#Double_rotate)


#### Floor kick

A player performs a floor kick when a Puyo group is rotated while the target space below for a Puyo is already occupied. When this happens, the whole Puyo group is pushed upwards so the rotation is a valid one after all.

An example for a Pair of Puyos

*   Step 1: The player wants to rotate the green Puyo clockwise.


*   Step 2: The ground blocks the green Puyo.


*   Step 3: The Puyo group is pushed upwards.


#### Wall kick

The wall kick is similar to the floor kick. When an attempt is made to rotate a Puyo towards a wall, the Puyo group is pushed left or right. The wall kick will not take place if the target space after pushing is also occupied. In that case the Double rotate rule applies.

An example for a Pair of Puyos

*   Step 1: The player wants to rotate the green Puyo clockwise.


*   Step 2: The wall blocks the green Puyo.


*   Step 3: The Puyo group is pushed right.


#### YON rule

In [YON](https://puyonexus.com/wiki/YON) there is one additional rotation rule, that negates a wall kick and performs a floor kick instead. When the player rotates its Puyo into an obstacle (any kind of Puyo on the field) and the space above the obstacle is free, the Puyos are pushed upwards: a floor kick. In every other Puyo Puyo game, this action would normally result in a wall kick.

An example for a Pair of Puyos

*   Step 1: The player wants to rotate the green Puyo counterclockwise.


*   Step 2: An obstacle obstructs the Puyo (illustrated as wall blocks)


*   Step 3: The Puyo group is pushed upwards


#### Double rotate

**Double rotate**, also known as **Quick Turn** is a Puyo Puyo game rule available in all games except [the first one](https://puyonexus.com/wiki/Puyo_Puyo_(1992))
 (and original mode in [15th Anni](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary)
). It lets you flip over a 2-group by pressing either rotate button twice while the group is wedged between two columns of Puyo where there is no room for a 90 degree rotation.


## Scoring

*Source: <https://puyonexus.com/wiki/Scoring> &mdash; 137 diagrams omitted*

#### Contents

*   [1 Scoring Formula](https://puyonexus.com/wiki/Scoring#Scoring_Formula)
    *   [1.1 Color Bonus](https://puyonexus.com/wiki/Scoring#Color_Bonus)

    *   [1.2 Group Bonus](https://puyonexus.com/wiki/Scoring#Group_Bonus)

    *   [1.3 Variations](https://puyonexus.com/wiki/Scoring#Variations)
        *   [1.3.1 Non-standard Puyo to Clear Amount](https://puyonexus.com/wiki/Scoring#Non-standard_Puyo_to_Clear_Amount)

        *   [1.3.2 Point Puyo Bonus](https://puyonexus.com/wiki/Scoring#Point_Puyo_Bonus)

*   [2 Nuisance Formula](https://puyonexus.com/wiki/Scoring#Nuisance_Formula)

*   [3 Drop Bonus](https://puyonexus.com/wiki/Scoring#Drop_Bonus)
    *   [3.1 Drop Bonus Glitch](https://puyonexus.com/wiki/Scoring#Drop_Bonus_Glitch)

*   [4 List of Chain Scores](https://puyonexus.com/wiki/Scoring#List_of_Chain_Scores)


#### Scoring Formula

The formula that the game uses to calculate the score from the chain is as follows:

   Score = (10 \* PC) \* (CP + CB + GB) where:

*   PC = Number of puyo cleared in the chain.
*   CP = Chain Power (These values are listed in the [Chain Power Table](https://puyonexus.com/wiki/Chain_Power_Table) .)
*   CB = [Color Bonus](https://puyonexus.com/wiki/Scoring#Color_Bonus)

*   GB = [Group Bonus](https://puyonexus.com/wiki/Scoring#Group_Bonus)

*   The value of (CP + CB + GB) is limited to between 1 and 999 inclusive.

##### Color Bonus

The color bonus is calculated from the following table, depending on how many different color puyo were cleared in the chain. Note that the values used are different for classic scoring and fever scoring:

| Classic Scoring |     | Fever Scoring |     |
| --- | --- | --- | --- |
| Colors | Bonus | Colors | Bonus |
| 1   | 0   | 1   | 0   |
| 2   | 3   | 2   | 2   |
| 3   | 6   | 3   | 4   |
| 4   | 12  | 4   | 8   |
| 5   | 24  | 5   | 16  |

_[Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu)_ has a color bonus defined for 6 colors cleared, which is 48.

##### Group Bonus

The bonus for each group of puyo is calculated from the following table by how many puyo are in the group. These values are all added together for the group bonus. Note that the values used are different for classic scoring and fever scoring:

| Classic Scoring |     | Fever Scoring |     |
| --- | --- | --- | --- |
| Puyo in Group | Bonus | Puyo in Group | Bonus |
| 4   | 0   | 4   | 0   |
| 5   | 2   | 5   | 1   |
| 6   | 3   | 6   | 2   |
| 7   | 4   | 7   | 3   |
| 8   | 5   | 8   | 4   |
| 9   | 6   | 9   | 5   |
| 10  | 7   | 10  | 6   |
| 11+ | 10  | 11+ | 8   |

##### Variations

###### Non-standard Puyo to Clear Amount

When the puyo to clear amount is less than 4, the group bonus will start at whatever the puyo to clear amount is and continue up from there. When the puyo to clear amount is greater than 4, the group bonus will act as if the puyo to clear amount is still 4 (that is, if the puyo to clear amount is 6, making a chain where 6 puyo are cleared will generate a group bonus of 3 (referring to classic scoring), not 0.)

###### Point Puyo Bonus

When point puyo are involved in the chain, the scoring formula changes slightly to include the addition of a point puyo bonus:

   Score = (10 \* PC + PB) \* (CP + CB + GB) where:

*   PC = Number of puyo cleared in the chain.
*   CP = Chain Power (These values are listed in the [Chain Power Table](https://puyonexus.com/wiki/Chain_Power_Table) .)
*   CB = [Color Bonus](https://puyonexus.com/wiki/Scoring#Color_Bonus)

*   GB = [Group Bonus](https://puyonexus.com/wiki/Scoring#Group_Bonus)

*   PB = Point Puyo Bonus
*   The value of (CP + CB + GB) is limited to between 1 and 999 inclusive.

The point puyo bonus is calculated from the total value of the point puyo cleared in the chain (which is usually 50 points per point puyo).

#### Nuisance Formula

The formula that the game uses to calculate the amount of nuisance to send to the opponent is as follows:

   NP = SC / TP + NL
   NC = ⌊ NP ⌋
   NL = NP - NC

where:

*   NP = Calculated nuisance points.
*   SC = Current chain score.
*   TP = Target points, or score per nuisance puyo. Default is 70.
*   NL = Leftover nuisance points, a decimal between 0 and 1.
*   NC = Number of nuisance puyo to send, rounded down.

Any nuisance lost in the rounding process is carried over to the next opportunity to send nuisance. If nuisance points comes out to 1.70, then 1 nuisance puyo would be sent and 0.70 nuisance would be added to the next chain.

#### Drop Bonus

Players are awarded extra score by dropping the puyo faster. This extra score is always displayed on the score counter on-screen. However, whether or not this extra score is used in the calculation of the nuisance depends on the game. Games that do add the drop bonus to the nuisance amount:

*   [Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu)

*   [Puyo Puyo! 15th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary)

*   [Puyo Puyo 7](https://puyonexus.com/wiki/Puyo_Puyo_7)

*   [Puyo Puyo!! 20th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!!_20th_Anniversary)

*   [Puyo Puyo Tetris](https://puyonexus.com/wiki/Puyo_Puyo_Tetris)

*   [Puyo Puyo Chronicle](https://puyonexus.com/wiki/Puyo_Puyo_Chronicle) In Puyo Puyo! 15th Anniversary and Puyo Puyo 7 the drop bonus is added to the amount of nuisance even in Fever rules, and not just in Tsu rules.

##### Drop Bonus Glitch

In Puyo Puyo!! 20th Anniversary and Puyo Puyo Tetris PS4 (pre-August 2017 patch), mashing rotates while soft dropping on the floor will build huge amounts of drop bonus while delaying the piece lock. This can be abused to make all 1 Chains send garbage. [https://twitter.com/S2LSOFTENER/status/887524028264718338](https://twitter.com/S2LSOFTENER/status/887524028264718338)

#### List of Chain Scores

This is a list of all the scores and nuisance produced by chains entirely formed by links made of 4 Puyos. This list assumes [single player Puyo Puyo Tsu attack powers](https://puyonexus.com/wiki/List_of_attack_powers#Classic_Rules) for calculating the scores, and 70 points as target points. This results in a gross estimate of the score of these chains, which might not be 100% accurate for real chains, but serves as a general idea of how big chains actually are.

| Chain length | Individual link points | Total chain points | Individual link nuisance | Total chain nuisance | Nuisance queue | Nuisance queue leftover |
| --- | --- | --- | --- | --- | --- | --- |
| 1   | 40  | 40  | 0   | 0   |     |     |
| 2   | 320 | 360 | 4   | 5   | <br><br><br><br> |     |
| 3   | 640 | 1000 | 9   | 14  | <br><br> <br> |     |
| 4   | 1280 | 2280 | 18  | 32  | <br> <br> |     |
| 5   | 2560 | 4840 | 36  | 69  | <br><br> <br> <br><br> |     |
| 6   | 3840 | 8680 | 54  | 124 | <br><br><br><br> <br> | <br> |
| 7   | 5120 | 13800 | 73  | 197 | <br> <br><br> <br><br> | <br> |
| 8   | 6400 | 20200 | 91  | 288 | <br> <br><br><br> <br> |  |
| 9   | 7680 | 27880 | 109 | 398 | <br> <br> <br> <br> |     |
| 10  | 8960 | 36840 | 128 | 526 | <br> <br><br><br><br> | <br><br> <br><br><br> |
| 11  | 10240 | 47080 | 146 | 672 | <br> <br> <br><br><br> | <br> |
| 12  | 11520 | 58600 | 182 | 837 | <br> <br><br><br> <br> | <br><br> <br><br> |
| 13  | 12800 | 71400 | 182 | 1020 | <br> <br> <br><br><br> |
| 14  | 14080 | 85480 | 201 | 1221 | <br> <br> <br><br><br> | <br><br><br> <br><br> |
| 15  | 15360 | 100840 | 219 | 1440 |  |     |
| 16  | 16640 | 117480 | 237 | 1678 | <br> <br> <br> <br><br> | <br> <br><br><br> |
| 17  | 17920 | 135400 | 256 | 1934 | <br> <br> <br><br><br> | <br><br> <br> |
| 18  | 19200 | 154600 | 274 | 2208 | <br> <br> <br> <br><br> |     |
| 19  | 20480 | 175080 | 292 | 2501 | <br> <br> <br> <br><br> | <br><br> <br> <br><br><br><br> |


## Staircase maneuver

*Source: <https://puyonexus.com/wiki/Staircase_maneuver> &mdash; 12 diagrams omitted*

The staircase maneuver is a trick that makes use of the [Rotation](https://puyonexus.com/wiki/Rotation) rules. By making use of floor kick and the double rotation (if needed), a group of Puyos can be rotated over the top of a pile of Puyos. These Puyos must be in the shape of a staircase, as it is not possible to rotate over a stack of 2 Puyos. Two possible methods are given to perform the staircase maneuver. This maneuver is present in every Puyo Puyo game except [YON](https://puyonexus.com/wiki/YON)
.


As shown here, the Puyos on the left border are rotated over a pile of other Puyos to reach the right border.

#### Method 1

The first method is by consequently pressing one rotation button. In the case where one wants to move the Puyos right, the counterclockwise rotation is used. This method needs to have the buttons pressed extremely fast in order to perform the maneuver.


*   The counterclockwise rotation button is pressed.

*   One Puyo rotates and now rests on the neighbouring Puyo.

*   The rotate button is pressed again while holding the right move button.

*   The pair is moved on to the next step of the Puyo.



*   The rotate button is pressed twice again. A floor kick is performed and the whole process is repeated.

#### Method 2

The second way to perform the staircase maneuver is by alternating the counterclock and clockwise rotation button. This method has more chances of succes to perform, as it does not need to be done as fast as method 1. However a second button is needed and chances of mistakes may be higher.

*   The clockwise rotation button is pressed.

*   The green Puyo rotates so the Puyos can slide on the next step.

*   The right move button is held.

*   The counterclockwise button is pressed and the Puyos perform a floor kick. The clockwise rotation button must be pressed again to repeat the process.


## Super attack

*Source: <https://puyonexus.com/wiki/Super_attack> &mdash; 4 diagrams omitted*

Basic field of Puyo Puyo~n. The number of super attacks the player has built up is indicated by "SP" at the bottom of the screen, along with the icons.

 Ruipanko! Arle using her super attack.

 ...which the game simply calls "Arle Shield," in complete English.

 Arle's super attack in action, which prevents Garbage Puyos from falling for 15 seconds.

**Super attacks** are a feature that has first appeared and is prominently featured in _[Puyo Puyo~n](https://puyonexus.com/wiki/Puyo_Puyo~n)_. They can allow things such as preventing Garbage Puyos from falling for a certain period of time, eliminating all Garbage Puyos, eliminating all Puyos of a certain color, and etc. In the console versions, Super Attacks are specific to the character, while in _Pocket Puyo Puyo~n_ they are independent and can be freely selected.

#### Contents

*   [1 Building up super attacks](https://puyonexus.com/wiki/Super_attack#Building_up_super_attacks)

*   [2 _Puyo Puyo~n_ super attacks](https://puyonexus.com/wiki/Super_attack#Puyo_Puyo~n_super_attacks)

*   [3 _Pocket Puyo Puyo~n_ super attacks](https://puyonexus.com/wiki/Super_attack#Pocket_Puyo_Puyo~n_super_attacks)

*   [4 _Puyo Puyo Box_ super attacks](https://puyonexus.com/wiki/Super_attack#Puyo_Puyo_Box_super_attacks)


#### Building up super attacks

Super attacks are charged by simply making chains. How much you gain is dependent on how many super attacks you have already earned. The more super attacks you have earned, the longer it will take for you to build up another one.

#### _Puyo Puyo~n_ super attacks

_Puyo Puyo~n_ super attacks are entirely character-specific. They are noted on their respective characters' pages.

Yon supers that remove Puyo interact with All Clears in a way that changes with the version being played. In the Dreamcast version, they can cause All Clears on their own, but this is not the case in either the PlayStation or Nintendo 64 versions. In those, at least a 2 Chain is required before the All Clear is considered valid.

#### _Pocket Puyo Puyo~n_ super attacks

Most of _Pocket Puyo Puyo~n'_s super attacks are tied to specific AI characters, and must be unlocked for player use. The player is given temporary access to a character's super attack after defeating them in Story. On the other hand, a super attack is permanently unlocked for all modes by raising its character's defeat percentage to 100% or higher in Challenge or getting a specific message after a battle with a defeat rate of 100% and higher in the same mode. Six non-character-specific powers are also unlocked through Challenge mode.

Wall blocks, after being placed, remain suspended in midair. All Iron Puyo and wall blocks can be erased from the player's field by simultaneously clearing 6 Puyo.

*   **00-None**: No effect.
*   **01-Nuisance Barrier**: A wall is built across the player's entire top row. Garbage Puyos can still drop into the hidden row.
*   **02-Hard Shield**: A wall is built across columns 2,3, and 4 on the player's top row. Garbage Puyos can still drop into the hidden row.
*   **03-Banish**: Up to 18 Garbage Puyos are removed from the player's field. The bottom of the field seems to be prioritized.
*   **04-Diet**: The highest two Garbage Puyos in each of the player's columns are erased.
*   **05-Sympathy**: All of the player's non-connected Puyos change into a single random color.
*   **06-Dark Slash**: A random row in the opponent's field is changed to six Iron Puyos.
*   **07-Jammer Wall**: A 1x2 wall is placed at a random location on the opponent's field; it is capable over overwriting any placed Puyo.
*   **08-Slash**: A random row in the opponent's field is occupied by six Garbage Puyos.
*   **09-Neo Slash**: A random row in the opponent's field is occupied by six Hard Puyos.
*   **10-Please Kaa-kun**: Carbuncle drops onto the field and walks across some of the player's Puyos. Every Puyo that he visits changes to a single color.
*   **11-Come BIG Puyo**: The player receives a Giant Puyo that, upon being placed, squashes everything below it.
*   **12-Nuisance Drop**: The opponent receives 6 Garbage Puyos.
*   **13-Hard Drop**: The opponent receives 6 Hard Puyos.
*   **14-Iron Drop**: The opponent receives 6 Iron Puyos.
*   **15-Transfer**: The top three color Puyos of each of the player's columns are sent to the opponent. Any Garbage Puyos above the third-highest color Puyos of each column is erased.
*   **16-Iron Bomb**: For a short while, the opponent will receive both Garbage Puyos and Iron Puyos as garbage, though they will receive less garbage overall.
*   **17-Wall Bomb**: For a short while, the opponent's garbage will consist of Sun Puyos and wall blocks. They will receive less garbage overall.
*   **18-Bone's Curse**: For a short while, the opponent cannot rotate their Puyos.
*   **19-Slowdown**: For a short while, the opponent cannot manually drop their Puyos.
*   **20-Straight**: A random row in the player's field is changed to six Puyos of the same, randomly-chosen color.
*   **21-Line Clear**: A random row in the player's field is cleared. Notably, the game treats this as if it were a normal chain.
*   **22-Thunder**: A random column in the player's field is changed to 12 Puyos of the same, randomly-chosen color.
*   **23-Vertical Clear**: A random column in the player's field is cleared. Notably, the game treats this as if it were a normal chain.
*   **24-Color Coat**: The highest two Garbage Puyos in each of the player's columns change into a single, randomly-chosen color.
*   **25-Freeze**: All of the opponent's Garbage Puyos become Hard Puyos.
*   **26-Build Up**: The opponent's bottom three rows become walls.
*   **27-Wildcard**: All of the player's Garbage Puyos change into random colors.
*   **28-Great Transfer**: The top five color Puyos of each of the player's columns are sent to the opponent. Any Garbage Puyos above the fifth-highest color Puyos of each column is erased.

#### _Puyo Puyo Box_ super attacks

Super attacks reappear in _[Puyo Puyo Box](https://puyonexus.com/wiki/Puyo_Puyo_Box)_. In this game, the player is allowed to choose which super attack they would like to use for the match. Each _Yon_ opponent in Rally mode is assigned a super attack that they will always choose.

*   **Slash**: All Puyos in one of the opponent's rows change to Garbage Puyos.
*   **Neo Slash**: All Puyos in one of the opponent's rows change to Hard Puyos.
*   **Break**: All Puyos in the opponent's shortest column change to Garbage Puyos.
*   **Neo Break**: All Puyos in the opponent's tallest column change to Garbage Puyos.
*   **Freeze**: All of the opponent's Garbage Puyos become Hard Puyo.
*   **Solitude**: All of the opponent's non-connected Puyos become Garbage Puyos.
*   **Pinpoint**: Both players receive a single Garbage Puyo in the column that contained the activating player's "pivot" Puyo.
*   **Solar Ray**: Both players receive a single Sun Puyo in the column that contained the activating player's "pivot" Puyo.
*   **Strike**: The opponent receives 6 Garbage Puyos.
*   **Double Strike**: The opponent receives 12 Nuisance Puyos.
*   **Hard Strike**: The opponent receives 6 Hard Puyos.
*   **Teleport**: The top three Puyos of each of the player's columns are sent to the opponent.
*   **Transfer**: The top two color Puyos of each of the player's columns are sent to the opponent. Any Garbage Puyos above the second-highest color Puyos of each column is erased.
*   **Neo Transfer**: The top four color Puyos of each of the player's columns are sent to the opponent. Any Garbage Puyos above the fourth-highest color Puyos of each column is erased.
*   **Barrier**: Garbage Puyos do not fall into the player's field for 5 seconds.
*   **Hyper Barrier**: Garbage Puyos do not fall into the player's field for 20 seconds.
*   **Resist**: For the next 10 pairs, the player will only receive 1 Garbage Puyo at a time.
*   **Split**: Erase all of the player's Puyos in the middle two columns.
*   **Slice**: Erase every odd row of the player's Puyos, with the bottom-most row counting as Row 1.
*   **Healing**: Remove the top 5 Garbage Puyos from each of the player's columns.
*   **Sweep**: Remove all of the player's Garbage Puyos.
*   **Cut**: Erase everything above the player's bottom four rows.
*   **Wildcard**: All of the player's Garbage Puyos change into random colors.
*   **Sympathy**: All of the player's non-connected Puyos change into a single random color.
*   **Thunder**: All Puyos in the column that contained the player's "pivot" Puyo changes to a single random color.
*   **Parallel Wave**: Two of the player's rows are chosen at random. The Puyos in each chosen row changes into a single random color.
*   **Triple Wave**: Three of the player's rows are chosen at random. The Puyos in each chosen row changes into a single random color.


## Types of Puyo

*Source: <https://puyonexus.com/wiki/Types_of_Puyo> &mdash; 37 diagrams omitted*

> _For the various skins you can make Puyo appear as, see [List of Puyo skins](https://puyonexus.com/wiki/List_of_Puyo_skins)
>  and [PPQ:Puyo Design](https://puyonexus.com/wiki/PPQ:Puyo_Design)
> ._

This is a list of all the Puyo types that appear throughout the _Puyo Puyo_ series.

#### Contents

*   [1 Colored Puyos](https://puyonexus.com/wiki/Types_of_Puyo#Colored_Puyos)

*   [2 Garbage types](https://puyonexus.com/wiki/Types_of_Puyo#Garbage_types)
    *   [2.1 Garbage Puyo](https://puyonexus.com/wiki/Types_of_Puyo#Garbage_Puyo)

    *   [2.2 Point Puyo](https://puyonexus.com/wiki/Types_of_Puyo#Point_Puyo)

    *   [2.3 Hard Puyo](https://puyonexus.com/wiki/Types_of_Puyo#Hard_Puyo)

    *   [2.4 Garbage Chu Puyo](https://puyonexus.com/wiki/Types_of_Puyo#Garbage_Chu_Puyo)

    *   [2.5 Iron Puyo](https://puyonexus.com/wiki/Types_of_Puyo#Iron_Puyo)

*   [3 Miscellaneous types](https://puyonexus.com/wiki/Types_of_Puyo#Miscellaneous_types)
    *   [3.1 Sun Puyo](https://puyonexus.com/wiki/Types_of_Puyo#Sun_Puyo)

    *   [3.2 Bomb Puyo](https://puyonexus.com/wiki/Types_of_Puyo#Bomb_Puyo)

    *   [3.3 Big Puyo](https://puyonexus.com/wiki/Types_of_Puyo#Big_Puyo)

*   [4 Nuisance queue icons](https://puyonexus.com/wiki/Types_of_Puyo#Nuisance_queue_icons)
    *   [4.1 Standard](https://puyonexus.com/wiki/Types_of_Puyo#Standard)

    *   [4.2 Other symbols](https://puyonexus.com/wiki/Types_of_Puyo#Other_symbols)

    *   [4.3 Exclusive to _Puyo Puyo~n_](https://puyonexus.com/wiki/Types_of_Puyo#Exclusive_to_Puyo_Puyo~n)

*   [5 Non-Puyo](https://puyonexus.com/wiki/Types_of_Puyo#Non-Puyo)
    *   [5.1 Block](https://puyonexus.com/wiki/Types_of_Puyo#Block)

    *   [5.2 Bomb](https://puyonexus.com/wiki/Types_of_Puyo#Bomb)


#### Colored Puyos

.png) Colored Puyos are the Puyos that you play with. Under standard rules, you connect 4 Puyos of the same color to clear them and make a chain. Matches generally use 3, 4, or 5 different colors of Puyo (Red, Green, Blue, Yellow, and Violet or Purple), with 4 being the standard for competitive matches. In the Famicom and MSX2 _[Puyo Puyo](https://puyonexus.com/wiki/Puyo_Puyo_(1991))_, it is possible to play with a total of six colors (Red, Yellow, Green, Gray, Lime, and Blue), however the sixth color would be scrapped once the series reached arcades, with five becoming the maximum.

#### Garbage types

##### Garbage Puyo

.png) Brazilian Portuguese: Puyo Lixoso ([Puyo Puyo Puzzle Pop](https://puyonexus.com/wiki/Puyo_Puyo_Puzzle_Pop)
) Puyo de lixo\[[sic](https://en.wikipedia.org/wiki/Sic)\
\] ([Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu) Nintendo Classics description) A Garbage Puyo serves no purpose except to clog up your field and mess up your chains, hence the name Garbage Puyo. Garbage Puyos can be cleared by clearing a colored Puyo that touches the Garbage Puyo (either horizontally or vertically, not diagonally).

##### Point Puyo

 A Point Puyo is a Garbage Puyo except clearing it will produce a point bonus for the chain. This bonus is 50 points in most games. In _[Puyo Puyo~n](https://puyonexus.com/wiki/Puyo_Puyo~n)_, the bonus varies and ranges from 50 to 500,000 (500K). In _[Puyo Puyo~n](https://puyonexus.com/wiki/Puyo_Puyo~n)_, the bonus that the Point Puyo produces is indicated on the Puyo. Point Puyos are usually colored differently to distinguish them from Garbage Puyos or any of the other garbage types. In _[Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu)_, they are perfectly spherical and blue in color, with a violet-colored core. In _[Puyo Puyo~n](https://puyonexus.com/wiki/Puyo_Puyo~n)_, they remain their original circular shape, but are now tinted yellow.

##### Hard Puyo

.png) A Hard Puyo is a stronger version of a Garbage Puyo. Clearing a Hard Puyo will turn it into a Garbage Puyo which can then be cleared normally. In _[Puyo Puyo~n](https://puyonexus.com/wiki/Puyo_Puyo~n)_, there are varying strengths of Hard Puyo, which range from Hard-1 (Puyos must be cleared once before it turns into a Garbage Puyo) to Hard-9 (Puyos must be cleared 9 times before it turns into a Garbage Puyo). In _[Puyo Puyo~n](https://puyonexus.com/wiki/Puyo_Puyo~n)_, the strength of the Hard Puyo, when it needs to be cleared more than once, is indicated on the Puyo. Hard Puyos are also created when bombs explode in _[Puyo Puyo! 15th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary)_. Hard Puyos are square or rectangular in order to make them distinguishable from Garbage Puyos or any of the other garbage types.

Clearing two or more normal Puyos adjacent to a Hard Puyo counts as multiple clears, and could remove it immediately skipping the Garbage Puyo state. In most games a crack sound effect is given, and the Hard Puyo's eyes seen floating up out of the field.

##### Garbage Chu Puyo

 A Garbage Chu Puyo is a variant of Garbage Puyo that was introduced in _[Puyo Puyo Fever 2](https://puyonexus.com/wiki/Puyo_Puyo_Fever_2)_. Chu Puyos appear at intervals in the Endless Chu Panic mode. They function identically to regular Garbage Puyos.

##### Iron Puyo

 An Iron Puyo is a Garbage Puyo that can't be cleared by normal means. They were introduced in _[Super Nazo Puyo: Rulue no Roux](https://puyonexus.com/wiki/Super_Nazo_Puyo:_Rulue_no_Roux)_ and can also be summoned by three super attacks in _[Pocket Puyo Puyo~n](https://puyonexus.com/wiki/Pocket_Puyo_Puyo~n)_. In the former game, they can't be cleared by any means; in the latter, all Iron Puyos are erased by clearing six Puyos simultaneously.

#### Miscellaneous types

##### Sun Puyo

.png) A Sun Puyo is a type of Puyo that plays a major role in _[Puyo Puyo Sun](https://puyonexus.com/wiki/Puyo_Puyo_Sun)_. Clearing a Sun Puyo increases the amount of Garbage Puyos sent to your opponent (only for garbage sent at the moment it is cleared,) but provides no point bonus to your chain. They are tied to games with the [Sun rule](https://puyonexus.com/wiki/Sun_(rule))
, but the "Solar Ray" super attack in _[Puyo Puyo Box](https://puyonexus.com/wiki/Puyo_Puyo_Box)
'_s "[Yon](https://puyonexus.com/wiki/Yon_(rule))
" style allows the player to summon a single Sun Puyo.

##### Bomb Puyo

.png) A Bomb Puyo is a special type of Puyo that only appears in the [Bomb Puyo](https://puyonexus.com/wiki/Bomb_Puyo) mode in _Puyo Puyo~n Party_ for the Nintendo 64. In this mode, there is always a Bomb Puyo in one of the four playfields. Clearing any colored Puyos adjacent to a Bomb Puyo will displace it to another player's playfield. When the Bomb Puyo's timer (seen at the top of the screen) expires, it explodes, eliminating the player who last had it on their playfield.

##### Big Puyo

A bigger variant of a single Puyo, which drop as 4 Puyo of the same color at a time. These Puyo, while cannot be rotated/turned, typically changes colors as players press the rotate buttons.

The color change order is:

Red, Green, Blue, Yellow, Purple

(The order is reversed for the counter-clockwise rotations.)

#### Nuisance queue icons

> _Main article: [Nuisance queue](https://puyonexus.com/wiki/Nuisance_queue)
> _

The following Puyo do not appear on the board; instead, they only appear on the nuisance queue to indicate a high amount of Garbage Puyo.

##### Standard

*    Small Puyo
*    Large Puyo
*    Rock Puyo
*    Star Puyo
*    Moon Puyo
*    Crown Puyo
*    Comet Puyo

##### Other symbols

*    Mushroom Puyo
*    Star Puyo
*    Crown Puyo
*    Comet Puyo
*    Saturn Puyo

##### Exclusive to _Puyo Puyo~n_

*    Club Puyo
*    Diamond Puyo
*    Heart Puyo
*    Spade Puyo
*    Crown Puyo
*    Mushroom Puyo
*    Top Hat Puyo
*    Ball Puyo
*    Tent Puyo
*    GD-ROM Puyo
*    Blue Swirl Puyo
*    Green Swirl Puyo
*    Yellow Swirl Puyo
*    Purple Swirl Puyo
*    Red Swirl Puyo

#### Non-Puyo

##### Block

 No surprises here; it's a block. Blocks always stay in the position they are placed at and don't fall when there is space below it. They are present in the _Nazo Puyo_ games and the [Blocks](https://puyonexus.com/wiki/Blocks) ruleset, but can be summoned by three super attacks in _Pocket Puyo Puyo~n_; in the case of super attack #17 (Wall Bomb), they drop like Garbage Puyos and only obtain their gravity-defying properties upon being placed. They are unclearable in every game except _Pocket Puyo Puyo~n_, where every Block is erased when the player matches six Puyos simultaneously.

##### Bomb

.png) Bombs are the central mechanic of _[Puyo Puyo! 15th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary)
_'s [Bomber](https://puyonexus.com/wiki/Bomber) mode. When normal Puyos are cleared, small Bombs are sent to an opposing player's [Nuisance queue](https://puyonexus.com/wiki/Nuisance_queue)
. When the queue fills up, they are combined to form a full-sized Bomb. For each full-sized Bomb, a Bomb is dropped onto the field as garbage (while leftover small Bombs will still remain in the queue.) When a Bomb is spawned, it begins with a count of 4, which reduces by 1 for each Puyo placed. When the count reaches 0, surrounding Puyos are converted into Hard Puyos. They can be cleared from the field by clearing any colored Puyo adjacent to them before their timers expire.


---

# Rules that are not in Category:Rules

The trap this document exists for. The ruleset we implement, the ghost puyo and the ceiling all live outside that category, filed as gameplay guides.


## Tsu (rule)

*Source: <https://puyonexus.com/wiki/Tsu_(rule)> &mdash; 3 diagrams omitted*

From Puyo Nexus Wiki

[Jump to navigation](https://puyonexus.com/wiki/Tsu_(rule)#mw-head)
 [Jump to search](https://puyonexus.com/wiki/Tsu_(rule)#searchInput)

| Tsu |     |
| --- | --- |
|  |     |
| [Margin time](https://puyonexus.com/wiki/Margin_time) | 96  |
| [Target points](https://puyonexus.com/wiki/Scoring) | 70  |
| [Offsetting](https://puyonexus.com/wiki/Offset_rule) | Classic |
| [All clear](https://puyonexus.com/wiki/All_clear)<br> bonus | +30 Nuisance Puyo sent on your next chain |

The **Tsu** (ぷよぷよ通, , _Puyopuyo tsū_) rule is the name of the game mode introduced and heavily utilized in _[Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu)_. It brought many improvements over the standard rule used in [_Puyo Puyo_ (1992)](https://puyonexus.com/wiki/Puyo_Puyo_(1992))
, with the most notable ones being [offsetting](https://puyonexus.com/wiki/Offset_rule) and [All Clears](https://puyonexus.com/wiki/All_Clear)
.

In _[Puyo Puyo Fever](https://puyonexus.com/wiki/Puyo_Puyo_Fever)_ and _[Puyo Puyo Fever 2](https://puyonexus.com/wiki/Puyo_Puyo_Fever_2)_, this rule is called **Classic**, as the [Fever rule](https://puyonexus.com/wiki/Fever_(rule)) is the default rule used in said games. In _[Puyo Puyo! 15th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary)_, _[Puyo Puyo 7](https://puyonexus.com/wiki/Puyo_Puyo_7)_, and _[Puyo Puyo!! 20th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!!_20th_Anniversary)_ this rule is called **Puyo Puyo Tsu** (**Puyo Puyo 2** in English translations and **eSports / Champions** English UI), not to be confused with the game of the same name. In _[Puyo Puyo Tetris](https://puyonexus.com/wiki/Puyo_Puyo_Tetris)_ and [its sequel](https://puyonexus.com/wiki/Puyo_Puyo_Tetris_2)
, it is called simply **Versus**, as the counterpart to Standard Rotation System Tetris.

To this day, the Tsu rule is widely considered the standard for Puyo Puyo matches, and has been present in every Puyo Puyo game since.

#### Contents

*   [1 Gameplay](https://puyonexus.com/wiki/Tsu_(rule)#Gameplay)

*   [2 Offsetting](https://puyonexus.com/wiki/Tsu_(rule)#Offsetting)

*   [3 All Clear](https://puyonexus.com/wiki/Tsu_(rule)#All_Clear)

*   [4 Margin time](https://puyonexus.com/wiki/Tsu_(rule)#Margin_time)

*   [5 Difficulty](https://puyonexus.com/wiki/Tsu_(rule)#Difficulty)


#### Gameplay

The basic idea of a match under Tsu rules is the same as under [original Puyo Puyo rules](https://puyonexus.com/wiki/Puyo_Puyo_(rule))
, in that you want the opponent to fill up their third column and lose.

#### Offsetting

 A player offsetting.

> _Main article: [Offset rule](https://puyonexus.com/wiki/Offset_rule)
> _

A major addition to Tsu rule that was absent in the original Puyo Puyo rule is the addition of offsetting. Under the original Puyo Puyo rules, all a player had to do was create a 5 chain in order to guarantee that their opponent would lose. Under Tsu rules, creating chains now first counters Garbage Puyos in the player's nuisance tray before sending Garbage Puyos to the player's opponent. No matter what the player creates a chain or more, Garbage Puyos will still fall in board if not cleared.

#### All Clear

 A player has performed an All Clear.

> _Main article: [All Clear](https://puyonexus.com/wiki/All_Clear)
> _

Another major addition to the Tsu rule is the concept of All Clears. An All Clear is a reward that the player receives from clearing all Puyos, including Garbage Puyos and their variants from their field. In Tsu rules, the reward is that the player will send 30 extra Garbage Puyos on their next chain. In some games the amount of Garbage Puyos sent from an All Clear can be adjusted.

#### Margin time

> _Main article: [Margin time](https://puyonexus.com/wiki/Margin_time)
> _

A less noteworthy addition to the Tsu rule is the concept of margin time, with the intention of preventing matches from lasting too long. Margin time states how long it will take before the target points start to decrease, having the effect of each chain sending more Garbage Puyos. At its most extreme level, it can allow a player to send at least 40 Garbage Puyos from just a 1 chain.

#### Difficulty

The difficulty level effects how many colors the player will receive and how many Garbage Puyos they will start out with on their field.

Very Easy

The player plays with 3 colors.

Easy

The player plays with 3 colors and starts out with 2 rows of Garbage Puyos.

Normal

The player plays with 4 colors.

Hard

The player plays with 5 colors.

Very Hard

The player plays with 5 colors and starts out with 2 rows of Garbage Puyos. The player's Puyos also fall slightly faster.

| [Game Modes](https://puyonexus.com/wiki/Category:Game_Modes) |     |
| --- | --- |
|     |
| Standard modes | [Puyo Puyo](https://puyonexus.com/wiki/Puyo_Puyo_(rule))<br> • Tsu • [Sun](https://puyonexus.com/wiki/Sun_(rule))<br> • [Fever](https://puyonexus.com/wiki/Fever_(rule)) |
|     |
| _[Box](https://puyonexus.com/wiki/Puyo_Puyo_Box)<br>_ | [Puyo Puyo](https://puyonexus.com/wiki/Puyo_Puyo_(rule))<br> • Tsu • [Sun](https://puyonexus.com/wiki/Sun_(rule))<br> • [Yo~n](https://puyonexus.com/wiki/Yon_(rule))<br> • [Treasure](https://puyonexus.com/wiki/Excavation) |
|     |
| _[15th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!_15th_Anniversary)<br>_ | [Puyo Puyo](https://puyonexus.com/wiki/Puyo_Puyo_(rule))<br> • Puyo Puyo Tsu • [Puyo Puyo Fever](https://puyonexus.com/wiki/Fever_(rule))<br> • [Excavation](https://puyonexus.com/wiki/Excavation)<br> • [Bomber](https://puyonexus.com/wiki/Bomber)<br> • [Spinner](https://puyonexus.com/wiki/Spinner)<br> • [Searchlight](https://puyonexus.com/wiki/Searchlight)<br> • [Non-Stop Fever](https://puyonexus.com/wiki/Non-Stop_Fever)<br> • [Underwater](https://puyonexus.com/wiki/Underwater)<br> • [Ice Blocks](https://puyonexus.com/wiki/Ice_Blocks)<br> • [Mission Puyo](https://puyonexus.com/wiki/Mission_Puyo)<br> • [Mega Puyo](https://puyonexus.com/wiki/Mega_Puyo) |
|     |
| _[7](https://puyonexus.com/wiki/Puyo_Puyo_7)<br>_ | [Transformation](https://puyonexus.com/wiki/Transformation)<br> • Puyo Puyo Tsu • [Puyo Puyo Fever](https://puyonexus.com/wiki/Fever_(rule))<br> • [Puyo Puyo](https://puyonexus.com/wiki/Puyo_Puyo_(rule))<br> • [Mission Puyo](https://puyonexus.com/wiki/Mission_Puyo)<br> • [Chain Simulator](https://puyonexus.com/wiki/Chain_Simulator) |
|     |
| _[20th Anniversary](https://puyonexus.com/wiki/Puyo_Puyo!!_20th_Anniversary)<br>_ | [Puyo Puyo](https://puyonexus.com/wiki/Puyo_Puyo_(rule))<br> • Puyo Puyo Tsu • [Puyo Puyo Sun](https://puyonexus.com/wiki/Sun_(rule))<br> • [Puyo Puyo Fever](https://puyonexus.com/wiki/Fever_(rule))<br> • [Mission Puyo](https://puyonexus.com/wiki/Mission_Puyo)<br> • [Mega Puyo Rush](https://puyonexus.com/wiki/Transformation#Mega_Puyo_Rush)<br> • [Mini Puyo Fever](https://puyonexus.com/wiki/Transformation#Mini_Puyo_Fever)<br> • [Mini Puyo Excavation](https://puyonexus.com/wiki/Mini_Puyo_Excavation)<br> • [Gust](https://puyonexus.com/wiki/Gust)<br> • [Slot](https://puyonexus.com/wiki/Slot)<br> • [Foursight](https://puyonexus.com/wiki/Foursight)<br> • [Blocks](https://puyonexus.com/wiki/Blocks)<br> • [Active](https://puyonexus.com/wiki/Active)<br> • [Cross Spinner](https://puyonexus.com/wiki/Cross_Spinner)<br> • [Quartet](https://puyonexus.com/wiki/Quartet)<br> • [Ice Blocks](https://puyonexus.com/wiki/Ice_Blocks)<br> • [Spinner](https://puyonexus.com/wiki/Spinner)<br> • [Mega Puyo](https://puyonexus.com/wiki/Mega_Puyo)<br> • [Non-Stop Fever](https://puyonexus.com/wiki/Non-Stop_Fever)<br> • [Excavation](https://puyonexus.com/wiki/Excavation)<br> • [Pair Puyo Puyo](https://puyonexus.com/wiki/Pair_Puyo_Puyo)<br> • [Chain Simulator](https://puyonexus.com/wiki/Chain_Simulator)<br> • [Ice Puyo (Unplayable)](https://puyonexus.com/wiki/Ice_Puyo) |
|     |
| _[Tetris](https://puyonexus.com/wiki/Puyo_Puyo_Tetris)<br>_ | Versus (Puyo Puyo Tsu) • [Swap](https://puyonexus.com/wiki/Swap)<br> • [Big Bang](https://puyonexus.com/wiki/Big_Bang)<br> • [Party](https://puyonexus.com/wiki/Party)<br> • [Fusion](https://puyonexus.com/wiki/Fusion) |
|     |
| _[Chronicle](https://puyonexus.com/wiki/Puyo_Puyo_Chronicle)<br>_ | [Puyo Puyo](https://puyonexus.com/wiki/Puyo_Puyo_(rule))<br> • Puyo Puyo Tsu • [Puyo Puyo Sun](https://puyonexus.com/wiki/Sun_(rule))<br> • [Puyo Puyo Fever](https://puyonexus.com/wiki/Fever_(rule))<br> • [Mission Puyo](https://puyonexus.com/wiki/Mission_Puyo)<br> • [Mega Puyo Rush](https://puyonexus.com/wiki/Transformation#Mega_Puyo_Rush)<br> • [Blocks](https://puyonexus.com/wiki/Blocks)<br> • [Big Bang](https://puyonexus.com/wiki/Big_Bang)<br> • [Mini Puyo Excavation](https://puyonexus.com/wiki/Mini_Puyo_Excavation)<br> • [Foursight](https://puyonexus.com/wiki/Foursight)<br> • [Active](https://puyonexus.com/wiki/Active)<br> • [Quartet](https://puyonexus.com/wiki/Quartet)<br> • [Excavation](https://puyonexus.com/wiki/Excavation)<br> • [Ice Blocks](https://puyonexus.com/wiki/Ice_Blocks)<br> • [Spinner](https://puyonexus.com/wiki/Spinner)<br> • [Mega Puyo](https://puyonexus.com/wiki/Mega_Puyo)<br> • [Non-Stop Fever](https://puyonexus.com/wiki/Non-Stop_Fever)<br> • [Skill Battle](https://puyonexus.com/wiki/Skill_Battle) |
|     |
| _[Tetris 2](https://puyonexus.com/wiki/Puyo_Puyo_Tetris_2)<br>_ | [Skill Battle](https://puyonexus.com/wiki/Skill_Battle)<br> • Versus (Puyo Puyo Tsu) • [Swap](https://puyonexus.com/wiki/Swap)<br> • [Big Bang](https://puyonexus.com/wiki/Big_Bang)<br> • [Party](https://puyonexus.com/wiki/Party)<br> • [Fusion](https://puyonexus.com/wiki/Fusion) |
|     |
| _[Puzzle Pop](https://puyonexus.com/wiki/Puyo_Puyo_Puzzle_Pop)<br>_ | [Puyo Puyo](https://puyonexus.com/wiki/Puyo_Puyo_(rule))<br> • Puyo Puyo 2 • [Puyo Puyo Sun](https://puyonexus.com/wiki/Sun_(rule))<br> • [Puyo Puyo Fever](https://puyonexus.com/wiki/Fever_(rule))<br> • [Non-Stop Fever](https://puyonexus.com/wiki/Non-Stop_Fever)<br> • [Mission Puyo](https://puyonexus.com/wiki/Mission_Puyo)<br> • [Blocks](https://puyonexus.com/wiki/Blocks)<br> • [Big Bang](https://puyonexus.com/wiki/Big_Bang)<br> • [Mega Puyo](https://puyonexus.com/wiki/Mega_Puyo)<br> • [Mega Puyo Rush](https://puyonexus.com/wiki/Transformation#Mega_Puyo_Rush)<br> • [Mini Dig](https://puyonexus.com/wiki/Mini_Puyo_Excavation)<br> • [Chain Simulator](https://puyonexus.com/wiki/Chain_Simulator)<br> • [Foursight](https://puyonexus.com/wiki/Foursight)<br> • [Active](https://puyonexus.com/wiki/Active)<br> • [Quartet](https://puyonexus.com/wiki/Quartet)<br> • [Dig](https://puyonexus.com/wiki/Excavation)<br> • [Frozen](https://puyonexus.com/wiki/Ice_Blocks)<br> • [Spinner](https://puyonexus.com/wiki/Spinner)<br> • [Bomb](https://puyonexus.com/wiki/Bomber)<br> • [Searchlight](https://puyonexus.com/wiki/Searchlight) |
|     |
| Miscellaneous | [Bomb Puyo](https://puyonexus.com/wiki/Bomb_Puyo)<br> (_Yo~n Party_) • [Trap](https://puyonexus.com/wiki/Trap_(rule))<br> (_Fever_ & _Fever 2_) |


## Special Maneuvers and Mechanics

*Source: <https://puyonexus.com/wiki/Special_Maneuvers_and_Mechanics> &mdash; 12 diagrams omitted*

Some nifty side information and tips that every _Puyo_ player should know.

#### Contents

*   [1 Soft Drop Bonus](https://puyonexus.com/wiki/Special_Maneuvers_and_Mechanics#Soft_Drop_Bonus)

*   [2 Tips for Playing Faster](https://puyonexus.com/wiki/Special_Maneuvers_and_Mechanics#Tips_for_Playing_Faster)
    *   [2.1 Double Rotation Frame Cutting](https://puyonexus.com/wiki/Special_Maneuvers_and_Mechanics#Double_Rotation_Frame_Cutting)

    *   [2.2 Wall Kicks](https://puyonexus.com/wiki/Special_Maneuvers_and_Mechanics#Wall_Kicks)

    *   [2.3 Avoid Excessive Splitting](https://puyonexus.com/wiki/Special_Maneuvers_and_Mechanics#Avoid_Excessive_Splitting)

    *   [2.4 Video: Double Rotation, Wall Kicks, and Splitting](https://puyonexus.com/wiki/Special_Maneuvers_and_Mechanics#Video:_Double_Rotation,_Wall_Kicks,_and_Splitting)

*   [3 Staircase Maneuver](https://puyonexus.com/wiki/Special_Maneuvers_and_Mechanics#Staircase_Maneuver)

*   [4 The 13th Row and Beyond](https://puyonexus.com/wiki/Special_Maneuvers_and_Mechanics#The_13th_Row_and_Beyond)
    *   [4.1 Video: Vanishing Trick and the Staircase Maneuver](https://puyonexus.com/wiki/Special_Maneuvers_and_Mechanics#Video:_Vanishing_Trick_and_the_Staircase_Maneuver)

    *   [4.2 19 Chains](https://puyonexus.com/wiki/Special_Maneuvers_and_Mechanics#19_Chains)

*   [5 Power Chaining](https://puyonexus.com/wiki/Special_Maneuvers_and_Mechanics#Power_Chaining)


#### Soft Drop Bonus

You earn small amounts of Score when you soft drop Puyo. In [Tsu rules](https://puyonexus.com/wiki/Tsu_(rule))
, this score is added to your next chain as small amounts of damage. In other words, you can "charge up" your 1 Chains by playing quickly and avoiding extraneous clearing. Every little bit of damage can count in a close harass battle.

_See [Scoring](https://puyonexus.com/wiki/Scoring) for more details._

#### Tips for Playing Faster

##### Double Rotation Frame Cutting

This is different from the mechanic introduced in Tsu where you can flip your piece 180 degrees if it's stuck in a column.

You should have noticed by now, but the pair of Puyo that you control has two parts. **One Puyo functions as the axis of rotation**, and the other Puyo rotates around it. When the pair comes out of the NEXT Window, the axis of rotation is in the Puyo on the bottom. By rotating twice while you soft drop, you can place the piece slightly faster than normal.

##### Wall Kicks

You can use wall kicks to help you place your Puyo faster and with more precision. It's easier to explain in a video, so watch the one I have below.

##### Avoid Excessive Splitting

When you split Puyo, you have to watch a realllyyyy long animation of your Puyo falling before the game gives you the next piece. Avoid forms that require excessive splitting to maximize your speed.

##### Video: Double Rotation, Wall Kicks, and Splitting

[Puyo Puyo: How to Play a Little Faster](https://www.youtube-nocookie.com/embed/UtZDkfvqyww?autoplay=1&start=35) Load video

YouTube

YouTube might collect personal data. [Privacy Policy](https://www.youtube.com/howyoutubeworks/user-settings/privacy/) ContinueDismiss

#### Staircase Maneuver

By using floorkicks, you can make your piece climb over adjacent columns. For more information, see [Staircase maneuver](https://puyonexus.com/wiki/Staircase_maneuver)
. Method 2 is the best one to learn since you don't have to count your button presses. Watch the video further down this page to see it in action.

#### The 13th Row and Beyond

In all Puyo Puyo games, there is a hidden row above the 12th row that you can place Puyo in using the Staircase Maneuver or by placing pieces vertically. Puyo in the 13th row can't be cleared even if they "connect" in a group of four.

 You can use the 13th row's properties to make chains that won't pop until the Puyo in the 13th row drops down. The Puyo in the 13th row is called the **Ghost Puyo**.

 Placing Puyo beyond the 13th row in rulesets like [Fever](https://puyonexus.com/wiki/Fever_(rule)) or games such as _[Puyo Puyo VS](https://puyonexus.com/wiki/Puyo_Puyo_VS)_ causes Puyo to disappear completely. If you set up a staircase correctly in the 12th and 13th rows, you can stall indefinitely by making all of your pieces **vanish**. Watch the video below to see it in action.

The vanishing trick is not possible in games that use traditional [Tsu](https://puyonexus.com/wiki/Tsu_(rule)) physics because there is a ceiling above the 13th row that prevents rotation into the 14th row.

##### Video: Vanishing Trick and the Staircase Maneuver

[Puyo Puyo: Vanishing Trick and Staircase Maneuver (Climbing) Tutorial](https://www.youtube-nocookie.com/embed/cP4pRwQmOBA?autoplay=1) Load video

YouTube

YouTube might collect personal data. [Privacy Policy](https://www.youtube.com/howyoutubeworks/user-settings/privacy/) ContinueDismiss

##### 19 Chains

In _Puyo_ games except _[Puyo Puyo Sun](https://puyonexus.com/wiki/Puyo_Puyo_Sun)_, the max chain you can make is a 19 chain. If you want to try to make a 19 Chain yourself, you'll need to know how to vanish your Puyo away until you get the colors that you want.

This is the classic 19 Chain form:

|     |     |
| --- | --- |
| <br><br>[Puyo Puyo VS 2 - 19 Chain Practice with Friends!](https://www.youtube-nocookie.com/embed/qsbwES9A9No?autoplay=1&start=199)<br><br>Load video<br><br>YouTube<br><br>YouTube might collect personal data. [Privacy Policy](https://www.youtube.com/howyoutubeworks/user-settings/privacy/)<br><br>ContinueDismiss |  |

#### Power Chaining

**Power Chaining** refers to chains clear a lot more than four Puyo at once. Clearing extra Puyo can yield two different types of bonuses: Group Bonus and Color Bonus. Group Bonus refers to clearing extra Puyo of the same color, and Color Bonus refers to simultaneously clearing different colors.

|     |     |
| --- | --- |
|  |  |
| Group Bonus | Color Bonus |

_For specifics on the mechanics, see [Scoring](https://puyonexus.com/wiki/Scoring)
._


Power Chains are extremely powerful when you consider the [Chain Power table for Tsu](https://puyonexus.com/wiki/Chain_Power_Table#Classic_Modes)
. The chain powers for links 1 through 5 increase exponentially, but after that it increases linearly. Since there isn't a large difference in strength between the higher chains, if you Power Up a 9 Chain, it could potentially beat a 10 chain, even though it's a link shorter.

|     |     |     |
| --- | --- | --- |
| Power 9 Chain: |  | \= 572 garbage |
| Normal 10 Chain: |  | \= only 526 garbage |


Powering Up your chains has two advantages over normal chains:

1.  Adding extra Puyo and colors adds a huge power bonus to your chain, which can be difficult for your opponent to calculate.
2.  Shorter chains have a quicker resolve time, which gives your opponent less time to react.

Also, in some cases, Powering Up your 9 Chain can be more efficient and quicker to execute than trying to beat the 10 chain with a complicated 11. A common tactic for Stairs Pattern users is to Power Up the Tail End using Stair-like shapes. This is an absurdly easy tactic to use, since Stair shapes are intuitive to make.

 You'll be hard-pressed to beat these kinds of players unless you're an expert at harassment, especially since the Tail can double as a Hellfire.

| ← [Efficiency 2: Tailing](https://puyonexus.com/wiki/Efficiency_2:_Tailing) | _[How to Play Puyo Puyo](https://puyonexus.com/wiki/How_to_Play_Puyo_Puyo)<br>_  <br>Part 3: Advanced Techniques | [Garbage Management: Digging and Counters](https://puyonexus.com/wiki/Garbage_Management:_Digging_and_Counters)<br> → |
| --- | --- | --- |


## Garbage Management: Digging and Counters

*Source: <https://puyonexus.com/wiki/Garbage_Management:_Digging_and_Counters> &mdash; 21 diagrams omitted*

If you've talked to any _Puyo_ players, you might've noticed the recurring notion that Harassment is the most powerful technique possible in _Puyo Puyo_. I won't deny that that's true, but personally, I find **expert Garbage Management** to be an equally scary and powerful technique. So before I teach you how to fight with Harassment, I'll teach you how to survive taking a hit.

#### Contents

*   [1 Digging](https://puyonexus.com/wiki/Garbage_Management:_Digging_and_Counters#Digging)
    *   [1.1 Getting hit with more than 2 lines](https://puyonexus.com/wiki/Garbage_Management:_Digging_and_Counters#Getting_hit_with_more_than_2_lines)

*   [2 Counters](https://puyonexus.com/wiki/Garbage_Management:_Digging_and_Counters#Counters)
    *   [2.1 Holy Counter and Devil Counter](https://puyonexus.com/wiki/Garbage_Management:_Digging_and_Counters#Holy_Counter_and_Devil_Counter)


#### Digging

**Digging** refers to clearing garbage to regain access to your chain. Let's say you got hit with a 2 chain:

 Since it's only 1 layer of garbage, you aren't really in a dangerous position. There are two ways you can go about dealing with this:

1.  Clear the garbage on top of the GTR, and then set off your chain
2.  Chain on top of the garbage, and then let the GTR trigger "fall through"

|     |     |     |     |
| --- | --- | --- | --- |
| Option 1: |  | →   |  |
| Option 2: |  |


Obviously, Option 2 is preferred since you're adding length to your main chain.



##### Getting hit with more than 2 lines

If your opponent hits you with 2+ lines of garbage and sets off their main chain as a follow-up, don't panic! Once again, you have two options: (1) rapidly 1 Chain through the garbage until you can access your trigger again, or (2) build a chain on top of the garbage that digs through the garbage and connects to your main chain.

If you're under a lot of pressure from your opponent, you probably don't have time for Option 1. Even if you do manage to dig out your chain, there's no guarantee that it's going to be strong enough to overpower your opponent. So it's Option 2 then. But how do you make a single chain that can dig through multiple layers? Since you're under a time constraint, you're going to have use your ingenuity (and luck) to find a way. I can't teach you every possible case, but I _can_ show you a few examples to help you understand the general idea.


Let's take the above example, except with two lines of garbage instead of one.

 Have you been studying your Tails? One way to dig through the garbage is to make Tail-type chains on top of the trigger.




  Alternatively, you can use a combination of Options 1 and 2 to dig to your chain. Clear a layer, and then chain on top of it.

 →  Whichever way you decide to go about dealing with the garbage, **make sure that none of your excess Puyo go to waste**. Instead of hastily placing the pieces you don't think you can use off to the side, place the pieces in such a way that they Power Up your chain.

#### Counters

Instead of waiting to get hit, though, you should **observe your opponent to anticipate the attack** and then **stack vertically to absorb the garbage**. This is called making a **Counter**. Yeah, it's kind of confusing since "counter" also refers to offsetting garbage. Just roll with it.

Let's take the above chain again. If you see your opponent about to harass you, build vertically on your transition to absorb the damage.

 It won't always be so clean and easy. Here are some other examples.


  Most people design their counters to absorb at least 5 lines (the 3rd chainsim), because 5 lines is the max amount of garbage the game will drop on you for every placement.

_For more examples, see [Counter](https://puyonexus.com/wiki/Counter)
._



##### Holy Counter and Devil Counter

If you've been playing a lot, sometimes the RNG will give you perfect sets of monochrome pieces that result in an automatic [All Clear](https://puyonexus.com/wiki/All_Clear)
. In Tsu, the All Clear bonus adds an extra 5 lines of garbage to your next chain. In other words, you can now fire off a 1 Chain to instantly harass your opponent. When you and/or your opponent obtains an All Clear bonus, a common strategy is to build Holy Counters and Evil Counters.


A [Holy Counter](https://puyonexus.com/wiki/Counter#Holy_Counter) is a counter that only uses the left two columns of the board (filling up the 3rd column results in death). Any arrangement of Puyo that achieves that is considered a Holy Counter. But for reference, the most popular Holy Counter is the following form:

  An [Evil Counter](https://puyonexus.com/wiki/Counter#Evil_Counter) is a counter that only uses the right three columns of the board. Since you have 3 columns to work with, the counter is easier to make, but it might be harder to make a Tail.



| ← [Special Maneuvers and Mechanics](https://puyonexus.com/wiki/Special_Maneuvers_and_Mechanics) | _[How to Play Puyo Puyo](https://puyonexus.com/wiki/How_to_Play_Puyo_Puyo)<br>_  <br>Part 3: Advanced Techniques | [Basics of Observation, Harassment, and Strategy](https://puyonexus.com/wiki/Basics_of_Observation,_Harassment,_and_Strategy)<br> → |
| --- | --- | --- |


---

# Puyo Puyo Tsu, reverse engineered

Mechanics read out of the Mega Drive and arcade ROMs: the exact algorithms and frame timings, and - just as usefully - a list of what nobody has worked out yet.


## Reverse Engineering (index)

*Source: <https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Reverse_Engineering>*

The accurate [Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu) game mechanics and algorithms are currently being determined through a reverse engineering effort on the Genesis and Arcade version of the game (which are mostly the same with respect to all major aspects of gameplay).

This page summarizes the various aspects of the game that have been or are currently being analyzed, as well as various technical information about the platform and the process of reverse engineering.

Forum post are provided for reference, as they were what started this documentation effort. They may not be accurate and/or up to date.

#### Contents

*   [1 Introduction](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Reverse_Engineering#Introduction)

*   [2 Game memory](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Reverse_Engineering#Game_memory)

*   [3 Frame data tables](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Reverse_Engineering#Frame_data_tables)

*   [4 Game mechanics](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Reverse_Engineering#Game_mechanics)

*   [5 Game modification](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Reverse_Engineering#Game_modification)

*   [6 Areas of interest](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Reverse_Engineering#Areas_of_interest)

*   [7 History](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Reverse_Engineering#History)


#### Introduction

People willing to pursue a reverse engineering effort on [Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu) might find interesting pointers throughout the following pages:

*   [Puyo Puyo Tsu/Hardware Platforms](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Hardware_Platforms)

*   [Puyo Puyo Tsu/Debugging and Reverse Engineering Tools](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Debugging_and_Reverse_Engineering_Tools)

*   [Puyo Puyo Tsu/Reverse Engineering Process](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Reverse_Engineering_Process)

*   [Puyo Puyo Tsu/Software Architecture](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Software_Architecture) And on the forums:

*   [The hardware: Genesis/Mega Drive and System C-2/Arcade, tools of the trade](http://puyonexus.net/forum/viewtopic.php?p=41606#p41606)


#### Game memory

These pages explain stuff related to memory management, data structures and locations of interest:

*   [Puyo Puyo Tsu/Memory Mappings](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Memory_Mappings)

*   [Puyo Puyo Tsu/Memory Structures](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Memory_Structures)
     (includes value definitions, such as color codes)
*   [Puyo Puyo Tsu/Memory Allocator](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Memory_Allocator)

*   [Puyo Puyo Tsu/Game Options Variables](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Game_Options_Variables) On the forums:

*   [Puyo Puyo Tsuu fundamentals: memory mappings](http://puyonexus.net/forum/viewtopic.php?p=41607#p41607)


#### Frame data tables

The frame data tables are available on a dedicated page:

*   [Puyo Puyo Tsu/Frame Data Tables](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables) Direct access to the various subtables is provided below:

*   [Gamepad Input Repeat](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Gamepad_Input_Repeat)

*   [Rotation](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Rotation)

*   [Drop speed](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Drop_speed)

*   [Control lockout grace period](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Grace_period)

*   [Bouncing animation](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Bouncing_animation)

*   [Pair split speed](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Pair_split_speed)

*   [Free fall speed](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Free_fall_speed)
     (gravity)

#### Game mechanics

So far, these game mechanics have been covered.

On randomization:

*   [Puyo Puyo Tsu/Random Number Generator](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Random_Number_Generator)

*   [Puyo Puyo Tsu/Upcoming Pair Randomizer](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Upcoming_Pair_Randomizer)

*   [Puyo Puyo Tsu/Falling Pair Spawning Process](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Falling_Pair_Spawning_Process) On input handling:

*   [Puyo Puyo Tsu/Gamepad Input](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Gamepad_Input) On game physics while controlling the falling puyos:

*   [Puyo Puyo Tsu/Falling Pair Control](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Falling_Pair_Control)

*   [Puyo Puyo Tsu/Pair Lateral Movement](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Pair_Lateral_Movement)

*   [Puyo Puyo Tsu/Rotation, collision and push back](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back)

*   [Puyo Puyo Tsu/Soft Drop](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop)
     (with regular drop, bouncing animation and control lockout grace period)
*   [Puyo Puyo Tsu/Free fall](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Free_fall) And the historical posts on the forums are:

*   [Random numbers: how it's done](http://puyonexus.net/forum/viewtopic.php?p=41608#p41608)

*   [Upcoming pairs: how they are picked](http://puyonexus.net/forum/viewtopic.php?p=41609#p41609)

*   [How baby puyos are made: from random generation to the board](http://puyonexus.net/forum/viewtopic.php?p=41669#p41669)

*   [Memory allocation and memory structures](http://puyonexus.net/forum/viewtopic.php?p=41670#p41670)


#### Game modification

Patches and Game Genie codes that modify the gameplay in interesting ways are discussed below:

*   [Puyo Puyo Tsu/Game Genie Codes](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Game_Genie_Codes) On the forums:

*   [Bonus round: game genie codes and elite 1-chain](http://puyonexus.net/forum/viewtopic.php?p=41688#p41688)

*   [Codes to set falling speed and color set](http://puyonexus.net/forum/viewtopic.php?p=41959#p41959)


#### Areas of interest

Those are upcoming topics of a particular interest about the game mechanics:

*   Random number generation (RNG) throughout the game
*   RNG biases and issues
*   Score calculation
*   Identification of routines implementing game mechanics
*   Distribution algorithm of ojama puyos accross a row
*   Frame data and timings for various game sequences
    *   chain resolving time with respect to column height and splitting
    *   ~pair split / falloff speed~
    *   ~input lock period after pair placement (bouncing animation)~
    *   upcoming pair display lag
    *   upcoming pair control lag
    *   ~rotation timings~
    *   grace period before garbage (ojama) drop
    *   ~soft drop speed modifier (down arrow)~
    *   margin time: impact on garbage sent
    *   margin time: when does it occur / takes effect
    *   drop speed increase after x seconds
    *   ~[motion cancel](https://www.youtube.com/watch?v=dIKpoQN-7_w)
        ~

#### History

This project started out of curiosity about the randomization algorithm used in [Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu)
, and was published [on the forums](http://puyonexus.net/forum/viewtopic.php?f=40&t=2304) on August 27, 2013. The efforts evolved into analyzing various game mechanics in order to have an accurate understanding of the game's internals.


## Upcoming Pair Randomizer

*Source: <https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Upcoming_Pair_Randomizer> &mdash; 2 diagrams omitted*

This page describes how [Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu) picks random puyo pairs. Standard battle rules are assumed.

#### Contents

*   [1 Pair pools and buffers](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Upcoming_Pair_Randomizer#Pair_pools_and_buffers)

*   [2 Pair-pool random prefill algorithm](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Upcoming_Pair_Randomizer#Pair-pool_random_prefill_algorithm)

*   [3 Game routines analysis](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Upcoming_Pair_Randomizer#Game_routines_analysis)

*   [4 Notable facts](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Upcoming_Pair_Randomizer#Notable_facts)
    *   [4.1 Loop](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Upcoming_Pair_Randomizer#Loop)

    *   [4.2 First pairs of the game](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Upcoming_Pair_Randomizer#First_pairs_of_the_game)

    *   [4.3 Uniform distribution](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Upcoming_Pair_Randomizer#Uniform_distribution)

    *   [4.4 Randomization Biases](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Upcoming_Pair_Randomizer#Randomization_Biases)


#### Pair pools and buffers

The game keeps 3 main pools or "buffers" that store the puyos you'll get at various steps:

*   there's a buffer for the currently falling pair, for each player;
*   there's a buffer for the upcoming 3 pieces, for each player;
*   there a big buffer that holds the randomized pairs for your game, in their order of appearance, that is shared among all players.

Pairs are passed along these buffers as they are picked from the later one to make their way to the first one.

At the beginning of a battle, the game will generate the big buffer holding all the pairs you'll ever be able to get during that game. This is a pool shared by both players, but depends on the chosen difficulty. Thus, the game actually generates 3 pair pools:

*   Pool #1 at 0xFFAD00, holding pairs chosen among a 3-color subset only;
*   Pool #2 at 0xFFAE00, holding pairs chosen among a 4-color subset only;
*   Pool #3 at 0xFFAF00, holding pairs chosen among a 5-color subset only.

Each pool holds 256 puyos, making 128 pairs. The game will pick a pair from the relevant pool sequentially and loop if it ever reaches the end of the pool.

#### Pair-pool random prefill algorithm

The pool generation works as follows:

*   the full color-set of the game is shuffled in RAM at 0xFFA4E2, on 8 bytes. Initial order is: 0, 1, 3, 5, 4, 6 and 5 random permutations occur. For the sake of the example, we will consider the initial order in the next items;
*   each pool is initialized with the colors taken from that set, from the beginning up to the desired number of different colors, and loops through. With our color-set, that means pool #1 gets 0-1-3-0-1-3-..., pool #2 gets 0-1-3-5-0-1-3-5-... and pool #3 gets 0-1-3-5-4-0-1-3-5-4-... as I said, the true order has been shuffled;
*   each pool is shuffled by performing 256 pseudo-random permutations (swapping 2 puyos). Two puyos get chosen to be swapped: one is picked sequentially, beginning from the last one (number 255) all the way up to puyo number #0 in the pool. The second puyo is chosen by randomly picking an index. How so? Well, the game uses the previously explained random number generator, and only keeps the lowest byte from the 32-bit value. That makes an 8-bit value, that will necessarily be between 0 and 255. A good random index for choosing a random puyo as a swapping candidate;
*   after shuffling the 3 pair pools, the first two pairs of pool #2 and #3 are overwritten by the first two pairs of pool #1. This ensures you'll only get 3 different colors in your first 2 pairs;
*   lastly, the next two pairs of pool #3 are overwritten by the next two pairs of pool #2, effectively limiting the start of this pool to 4 colors.

The game then picks the pairs from those pools, sequentially, as it keeps a counter of how many puyos a player got so far. The counter is on 8 bits, so it will naturally loop after reaching 255.

#### Game routines analysis

Here's the full analysis of the subroutine, with side-by-side comparison of the Genesis and Arcade versions:

 Here's the color-set shuffling subroutine:

#### Notable facts

##### Loop

The game will loop through available pairs. 256 puyos in the randomized pool make 128 pairs, which amounts to about 3.55 full boards before a player will loop through pairs.

##### First pairs of the game

One might notice that the first three pairs you get are not the first 3 of the generated pair pool. Actually, they are in reversed order. The first pair dealt by the game is the third one of the pool, while the third piece is the first in the pool. This is how the game picks the first three pairs only. However, it loops in the correct order of appearance.

##### Uniform distribution

In a 4-color game, the pair pool initially gets exactly 64 puyos of each of those 4 colors. The margin of error amounts to the only 2 pairs which are overwritten, but the buffer is mostly uniform.

This means that, over the course of a battle, if one were to use 128 pairs, he would get exactly the same amount of puyos of each color.

##### Randomization Biases

The permutation algorithm may shift more puyos of one specific color towards the beginning (or the end) of a pool, effectively reducing the odds of getting that color greatly either at the end or the beginning of a battle.

Consequently, it means that if a player gets lots of unicolor pairs and a few other of the same color, he will have quickly depleted the pool from that color and is not likely to get more any time soon.

Actually, the [Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu)
 [Random Number Generator](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Random_Number_Generator) suffers from strong biases and will pick series of numbers resulting in very low short-term variance that may pack same-color puyos close to each other.


## Falling Pair Spawning Process

*Source: <https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Falling_Pair_Spawning_Process> &mdash; 7 diagrams omitted*

#### Game logic overview

The game function which handles most of the sequence of actions throughout a battle looks like this:

 This routine is called right after a player has placed a new pair on the board, and will handle most of the game logic before returning control to him.

_The following paragraph simplifies things a bit and is not 100% accurate._

The logic begins by generating the falling pair (yellow), which may fail if the 3rd column is filled: at least one player has lost the battle, and the game branches to a code path that ends quickly (light green). If the falling pair was successfully spawned, the main logic occurs and makes successive calls to many subroutines which perform the various computations to handle resolving chains, scoring, etc. At the end, a video frame update occurs.

#### Pair spawning

_Function/routine names have been arbitrarily chosen._

Pair generation picks two successive puyos from the randomized pair pools to place them in the "hands" of the player. The game calls function gen\_cur\_fal\_pair() which will either return with a carry flag set in the status register (SR) or not. If the carry is set, the game could not generate a falling pair because the 3rd column was obstructed.

Here's the gen\_cur\_fal\_pair() routine:

 The routine first shifts the next 3 pairs that are displayed (well, only 2 are displayed but 3 are actually in the corresponding buffer). To do so, it calls the distrib\_shift() routine which will put the very next pair in the d0 and d1 registers, while shifting the pairs in memory. distrib\_shift() also picks another pair from the randomized pool by calling distrib\_next\_pair().

Remember, the 3 upcoming pairs for each player are stored at 0xFF85A0 and 0xFF8DA0. Those are the places where the pairs are shifted from one position to the next, before being taken away from the buffer to be put in d0 and d1, then finally in the falling pair buffer.

Here's distrib\_shift():

 And here's distrib\_next\_pair(), displayed in both graph and text views to include a statically predefined array in ROM:

 The "distrib\_offset" array at 0x004C9C stores offsets to select the relevant randomized pair pool (0xFFAD00 + 0x100 = 0xFFAE00, pair pool for difficulty #3, while #4 and #5 will result in a 0x200 offset).

Now back to the gen\_cur\_fal\_pair() routine. After shifting the pairs, the function get\_board\_and\_offset() returns the pointer to the current player's board in RAM (0xFF8000 or 0xFF8800). This allows it to check if byte 0xFF801C or 0xFF881C is non-zero, thus if the upper cell of the 3rd column is obstructed.

Here's the get\_board\_and\_offset() function, which returns the address in a2:

 _(the offset the function computes in d0 is ignored in our case, but will be of importance later)_

If the cell is unobstructed, the game will allocate two structures in memory that will each describe a single puyo of the currently falling pair. Without going into details (they will be included in the very next post), those structures hold the current position of the falling puyo on the board, its color and the current vertical offset within a cell (that's what makes the puyos gradually fall through a cell). The color is taken from the d0 and d1 registers that were previously filled by the distrib\_shift() routine, from the upcoming pairs buffer.

There is a "master" puyo, which is the lowest one when the pair appears, around which the other one revolves. This is the only puyo which gets its absolute position stored. The other puyo is a "slave" puyo, the position of which is relative to the master one.

The slave puyo's position constraints player movements, and when the master puyo's y-axis position tries to get beyond the bottom line 0x0D (after being corrected by the relative position of the slave puyo), the pair is placed on the board by the place\_puyo() routine (called from somewhere else):

 The compute\_offset routine calculates the memory address at which the puyos will be stored: from the beginning of the board's buffer, it's an offset that depends on the puyos coordinates (roughly 2x+num\_rows\*y):

 load\_gfx() takes the puyo's sprite ID and position, and loads it to the VDP, making it appear at the next video frame update.


## Falling Pair Control

*Source: <https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Falling_Pair_Control> &mdash; 7 diagrams omitted*

This page describes overall handling of the falling pairs by the game, with pointers to detailed articles.

#### Contents

*   [1 Falling pair objects](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Falling_Pair_Control#Falling_pair_objects)

*   [2 Main puyo object](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Falling_Pair_Control#Main_puyo_object)

*   [3 Slave puyo object](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Falling_Pair_Control#Slave_puyo_object)

*   [4 Phases](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Falling_Pair_Control#Phases)
    *   [4.1 Initialization](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Falling_Pair_Control#Initialization)

    *   [4.2 Player-controlled fall](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Falling_Pair_Control#Player-controlled_fall)

    *   [4.3 Pair split and free-fall](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Falling_Pair_Control#Pair_split_and_free-fall)

    *   [4.4 Lockout and placement](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Falling_Pair_Control#Lockout_and_placement)

*   [5 Notable facts](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Falling_Pair_Control#Notable_facts)


#### Falling pair objects

Falling pairs are instantiated as [two objects](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Software_Architecture) in memory, within boundaries of the memory pool managed by the [memory allocator](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Memory_Allocator)
, between addresses 0xFFD100 (inclusive) and 0xFFE000 (exclusive).

Each object describes a distinct puyo from that pair, each being attached a different callback that will handle their behavior and display throughout their fall.

The main puyo's callback is not guaranteed to be run before the slave's callback. This can lead to the slave puyo lagging 1 frame behind in very specific circumstances (out of the player's hands).

#### Main puyo object

The main puyo is the only puyo being controlled by the player. The game flickers a white outline around it during its fall. Its object's parent structure points to the relevant player status. All gamepad inputs are handled by its callback routine.

Here's the main puyo callback routine:

#### Slave puyo object

No gamepad inputs are handled by the callback routines of the slave puyo. Its position is a relative position, updated in accordance to the main puyo's current coordinates. The object's parent structure points to the main puyo object. Pair split is made effective by breaking that link to the main puyo: the callback overwrites the parent structure pointer with the address of the relevant player status object. From that moment on, it lives its own life independently from the main puyo and there is no way it could be linked to it again. Position is not relative anymore, but absolute calculations are used.

Here's the slave puyo callback routine:

#### Phases

For each puyo of the falling pair, the following phases occur within their callback routines:

1.  initialization
2.  player-controlled fall
3.  pair split and free-fall
4.  lockout and placement on the board

What each puyo's callback does at each phase differs a bit, but is mostly similar.

##### Initialization

This phase runs for a single frame right after the puyo has been created by the [gen\_cur\_fal\_pair()](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Falling_Pair_Spawning_Process#Pair_spawning) routine.

It initializes a field of the puyo's object and computes its initial absolute coordinates on the screen by calling either get\_pair\_absolute\_pos() (main puyo) or compute\_slave\_puyo\_coords() (slave puyo).

Here's get\_pair\_absolute\_pos():

 The routine calls get\_board\_screen\_pxoffset() to get pixel offsets from which the board is drawn on the screen, depending on the player and current game mode. It then converts absolute board coordinates of the main puyo into pixel-coordinates, accounting for the relevant offset and the size of a single cell (16 pixels \* 16 pixels).

Here's compute\_slave\_puyo\_coords():

 This routine doesn't call the same subroutine to calculate pixel-wise coordinates as the slave puyo's object doesn't store its coordinates on the board. Instead, it looks for those coordinate sin its parent structure, the main puyo. It also has to account for potential rotation, thus reading the current state of a potentially pending rotation animation. Based on that step, it computes (x,y) pixel coordinates displacements values and applies them to the absolute coordinates of the puyo on the screen.

For reference, here is the get\_board\_screen\_pxoffset() routine:

 While the offset table used by the routine only contains standard values, it can be noted that changing those will shift the player's board on the screen. This could have been implemented to have a single board centered on the screen, for an hypothetical endless mode. Changing those offsets will actually work without having the game complaining.

##### Player-controlled fall

Phase 2 of the falling puyo callbacks loops while the player has control over the pair movements. Refer to the callback disassembly screenshots for clarity.

 The following updates occur for the main puyo:

1.  prepare the current player inputs which are considered valid for the current frame
2.  conditionnally update the game's RNG, to make it less predictable
3.  handle [lateral movement](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Pair_Lateral_Movement)
     (d-pad left/right arrows)
4.  handle [rotation and relevant collision](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back) 5.  handle [soft-drop](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop)
     (d-pad down arrow) and blocking

Player inputs are acknowledged in that order: lateral movement has priority over rotation, then over soft-drop. If one were to affect another, that order would matter. Soft drop stops while the right or left arrow is pressed.

 The following updates occur simultaneously for the slave puyo:

1.  conditionnally update the game's RNG, to make it less predictable
2.  advance the current rotation animation by one step, if needed
3.  compute new absolute (on-screen) pixel coordinates

These updates occur while the pair is not blocked by some obstacle under either puyo. This check is done at the end of each sequence. Passing the check advances to the next phase/checkpoint.

During this phase, the slave puyo callback does simply nothing but update the sprite coordinates on the screen.

##### Pair split and free-fall

Once the pair is blocked by something, both callbacks advance to the next checkpoint by branching to loc\_5FAC (main puyo) or loc\_63BC (slave puyo).

An initialization occurs before going through the checkpoint and yielding execution. This takes a single, simultaneous frame for both puyos.

For the main puyo:

1.  initialization updates on-screen coordinates and blocked status under both puyos to check which one should free-fall;
2.  initializes free-fall parameters if the main puyo shall fall;
3.  yields execution.

If the main puyo should not free-fall, the callback skips to the placement phase and yields execution.

For the slave puyo:

1.  initialization updates on-screen coordinates;
2.  splits pair by overwriting the link to the main puyo as a parent object: the new parent object is the relevant player status object;
3.  yields execution.

Gravity initialization takes an additional step (thus an additional frame) in the case of the slave puyo. This is because the callback may be run before the main puyo's callback, hence not reflecting the pair being blocked by an obstacle. During that additional frame, if the puyo should not free-fall, the callback then skips to the placement phase and yields execution again.

Internal details of the game routines relative to gravity are discussed on the page [Puyo Puyo Tsu/Free fall](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Free_fall)
, including formulas speed formulas and parameters.

##### Lockout and placement

When the gravity routine detects that the puyo has reached the floor, execution yields once again, calling the placement routine on the very next frame after free-fall. Both callbacks mark their respective object for cleanup and call the routine which will place the puyo on the virtual board representation in RAM.

#### Notable facts

*   Board placement on the screen could be different, programmers probably accounted for centered boards in an hypothetical single-player endless mode.
*   Pair split takes 2 frames to complete if the main puyo should free-fall, or 3 frames if it is the slave puyo. Technically, from the frame the pair is detected as being blocked, a single frame passes without anything happening, then the main puyo may begin its fall on the second frame after the blocking event. The slave puyo may not begin its fall before the third frame.


## Rotation, collision and push back

*Source: <https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back> &mdash; 5 diagrams omitted*

Rotation handling is performed on the frame the player's input was detected, and also performs two other operations:

*   collision detection, the player's trying to rotate against a wall, or between two obstructing columns;
*   push back, shifting the player's pair whenever possible if he tries to rotate against a blocking element.

This page fully details the rotation-related routines of the game.

Frame data for rotation animations are detailed on the [dedicated page](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Rotation)
.

#### Contents

*   [1 Overview](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back#Overview)

*   [2 Rotation basics](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back#Rotation_basics)

*   [3 Routine steps](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back#Routine_steps)
    *   [3.1 Gamepad settings and readout](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back#Gamepad_settings_and_readout)

    *   [3.2 Target cell check](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back#Target_cell_check)

    *   [3.3 Current row check](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back#Current_row_check)

    *   [3.4 Opposite cell check](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back#Opposite_cell_check)

    *   [3.5 Double-rotation](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back#Double-rotation)

    *   [3.6 Push back](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back#Push_back)

    *   [3.7 Rotation acknowledgement](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back#Rotation_acknowledgement)

*   [4 Pseudo-code algorithm](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back#Pseudo-code_algorithm)

*   [5 Possible tricks](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back#Possible_tricks)


#### Overview

Here's an overview of the routine handling rotation, rotation\_start():

 Click on the image to see the fully commented assembly code. It's a pretty long routine that covers various steps:

*   get gamepad settings and read the input;
*   check the destination cell for the rotating puyo (target cell);
*   check the opposite cell from the target cell, relative to the main puyo;
*   handle double-rotation (double tapping the buttons);
*   push the falling pair back if necessary;
*   acknowledge the rotation and trigger the rotation animation.

#### Rotation basics

To better understand what the routine performs, here's a visual introduction on how the game depicts various elements of the falling pair:

(click on the image for a full size view) The reference point is located at the top left corner of the player's board, starting at (0,0) all the way to (5,13). That makes 14 rows and 6 columns worth of space for your puyos. Row #y=0 and row #y=1 are the two ghost rows in which you can put puyos that don't count towards your chains, unless they fall into the visible board space below (rows #y=2 to 13).

A falling pair is handled by the game in two parts:

1.  the main puyo (yellow and highlighted puyo), center of rotation, is the one the player directly controls through the controller's D-pad. It gets is full coordinates stored in [a dedicated memory structure](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Memory_Structures#Falling_Puyo_structure)
    ;
2.  the "slave" puyo (red puyo), which has a mostly similar in-memory structure, linked to the main one, but it doesn't store the slave puyo's full coordinates.

The main puyo's structure holds a field describing a "rotation ID". This field holds a value between 0 and 3, each referring to a particular rotation. This is depicted in the annotated screenshot above, with P1 having a rotation ID of 0, while P2 has a rotation ID of 3. This ID rather serves as a geometric transformation "identifier". You can safely assume its values are mapped to the corresponding rotated coordinates of the slave puyo, but they have been carefully picked to allow for clever math afterwards (discussed below), when dealing with collision detection and pushing back the falling pair.

Refer to the [memory structures](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Memory_Structures#Falling_Puyo_structure) page for a complete description of the structure's fields.

Red spots around P1's main puyo mark the diagonal cells, while blue spots below P2's pair mark the cells the game will check for collision when deciding if it should finally place the falling pair on the board (this check is done in another routine).

#### Routine steps

It is recommended that you keep the routine code overview (see above) open while reading this section, to get a grasp at the code flow at stake.

##### Gamepad settings and readout

Starting at 0x6296, the routine gets the current player ID to get this player's gamepad configuration (type), and map the rotation buttons accordingly. After mapping the button into d0 (counter-clockwise) and d1 (clockwise), the routine matches counter-clockwise buttons with new inputs on that frame.

This means the rotation events are only triggered if the player has just pushed the button; no input-repeat mechanism will occur. But this also mean one can't input two rotations on two consecutive frames, because of how the gamepad status variable at plyr\_kstatus+1 (actually named plyr\_newkeys) is populated (see [Puyo Puyo Tsu/Gamepad Input](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Gamepad_Input)
).

From 0x62C2 onwards, the routine checks which button has been pressed, and discards the rotation event (beq locret\_6338) if both buttons are pressed at the same time.

Each of the two branches correspond to a rotation direction, and initialize the d0 and d1 registers with appropriate values (negative or positive values). This is the first clever use of the transform IDs: rotating clockwise means incrementing the ID, while decrementing it will account for a counter-clockwise rotation.

##### Target cell check

At 0x62D6, this incrementation/decrementation is performed, with d0 now being the "target" rotation ID. Original (current) rotation ID is a byte read from the memory, at the address a0+0x2B ($2B(a0) in the disassembly).

The resulting value is masked so that only its lowest two bits are kept (remainder of a euclidian division by 4). This trick allows rotating clockwise from rotation ID #3 to go back to 0. Decrementing from 0 will result in a binary value consisting of only 1's, with the lowest two bits accounting for the decimal value "3".

The value is copied to d3. Both d0 and d3 **now hold the target transformation ID**. This value has not yet been committed to memory, thus has not been acknowledged yet. The game wants to check whether or not this is a valid move.

The first check occurs immediately afterward: the check\_target\_and\_diag() routine, takes a desired transformation ID, and will check both the content of the desired cell, and the diagonal cell between the original (current) cell of the slave puyo and its intended destination (see the red spots on the earlier screenshot).

The check\_target\_and\_diag() routine returns a result in the lowest two bits of d0:

*   bit #0: if set to 1, the target cell is full;
*   bit #1: if set to 1, the diagonal cell is full.

But for now, the game only cares about the status of the target cell for the desired rotation. If the cell is empty, the routine skips directly to the [rotation acknowledgement](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back#Rotation_acknowledgement) and performs the action accordingly.

It gets more complicated if the target cell is full: we have a collision and the game will try to decide how to push back the player's pair, if possible.

For reference, here's the check\_target\_and\_diag() routine:

 The following sums up its computations:

*   the return value is prepared in d7, as 0x3 (two lowest bit already set): the routine will unset those bits only if it finds a free cell at the target and diagonal locations;
*   at 0x469C: the routine applies the desired transformation (rotation) to the current coordinates of the main puyo, by selecting the correct offset to apply to each axis from a predefined memory location (16 bytes starting at word\_4706). This gives absolute (x,y) coordinates in the board reference plane for the target cell after rotation;
*   the routine successively checks the target cell (from 0x46AC) then the diagonal cell (from 0x46D6);
    *   for each cell, if the desired coordinates are beyond the board limits, the game skips the check and returns a value indicating an obstructed cell. Indeed, no valid memory location is allocated for out of bounds cells;
    *   if the coordinates are actually valid, the routine calls [compute\_offset()](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Falling_Pair_Spawning_Process#Pair_spawning) to get the desired cell in-memory address, then checks its content and unsets the corresponding bit if the cell was empty.

From now on, the rotation routine determined whether the intended destination cell for the rotating puyo was empty or not. As stated earlier, if it was empty, the routine skips to the end. For the sake of the explanation, we will now consider what happens when the target cell is obstructing.

##### Current row check

We now know the destination cell is obstructed by either a puyo or the board edges.

Before going on to check if there's room to push back the pair, at 0x62EC the game checks whether the player's main puyo is currently in any of the ghost rows (y>=2?).

If that's the case, the game then determines (at 0x62F4) what rotation the player wants to achieve:

*   rotating to a sideway position (left or right of the main puyo, transform IDs 1 and 3) is allowed and the routine will go on;
*   rotating to an upright position (at the top or below the main puyo, transform IDs 0 and 2) is not allowed and the routine promptly exits.

This prevents the player from rotating a piece currently in the ghost rows, if the rotation would have resulted in pushing the pair upward (i.e. if the cell immediately below was obstructing the rotation).

##### Opposite cell check

After checking a "corner case" with the ghost rows, the routine tackles the opposite cell from the target cell, relative to the main puyo. This helps the game determine if it can push the player's pair in that direction: at the bottom of the board, the game will push the pair upwards, while it will push it sideways when rotating against a wall or a column.

Another clever math is done on the target transformation ID: xoring it's value with "2" gives a new and virtual transformation ID, which translates to the opposite cell. check\_target\_and\_diag() is called again to check if it is obstructed or not. While this should never happen when targeting position #2 (bottom, as the upper part of the column should be empty), the check is done anyway. This explicitly handles the case where the player is stuck between two columns and tries to rotate his pair: when pushing the button first, the target cell for the 90° rotation is occupied, as well as its opposite counterpart.

If the opposite cell is empty (0x6304), the routine skips to the [push back](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back#Push_back) section. On the contrary, the double-tap mechanism kicks in.

##### Double-rotation

Another xor (at 0x630A) of the intended rotation ID (with the value "3"), accounts for the two rotations needed when flipping a pair over.

For each individual pair, a rotation attempts counter is initialized at 0. It is only incremented when the player tries to rotate his pair while stuck between columns (at 0x6314).

If the counter holds an odd value after being incremented, the routine ends abruptly, discarding the rotation but keeping in "mind" that such a rotation attempt has occurred.

If the counter holds an even value, the rotation is allowed to go through.

A yet-unknown player-specific game option triggers a different behavior at every attempt, effectively requiring three consecutive button inputs to allow the rotation to go through.

The input button only matters at the even counter values, and will only then determine the rotation direction.

Finally, if the double tap occurred, the target rotation ID is adjusted according to the rotation direction, if needed (only adjusted if the input was a counter-clockwise rotation).



##### Push back

From now on, nothing will cancel the rotation.

The routine finally proceeds to pushing back the pair according to the previous outcome:

*   if the opposite cell is free, the pair is pushed back there (sideways or upwards);
*   if the opposite cell is full, the pair is stuck between two columns; after double tapping the button:
    *   a rotation pushes the pair's main puyo upwards, with the slave puyo taking its place at the bottom;
    *   or the slave puyo ends up at the top with the main puyo being pushed down by one cell.

The transformation applied to the main puyo's coordinates is stored beginning at 0x6384 (couples of x,y word values), and can be seen in the capture below:

 At 0x635C, the routine explicitly sets the puyo's position through its current cell at a predetermined value just above the middle of its height (0x7FFE, while the middle is at 0x8000 and the total height is 0x10000).

Since the game does not carry the old position over after pushing the pair upwards or downwards, it allows for input tricks that will exploit it and keep a pair at the same cell indefinitely when stuck between two columns, or when the bottom cells are obstructed. On the contrary, the sideway motions will carry the old offset over, hence not resetting the delay before the pair carries over to the next cell. This could be exploited to skip some animations and/or lockouts.

##### Rotation acknowledgement

The end of the routine (at 0x6362) acknowledges the rotation by:

*   resetting the double-tap counter to its closest even value;
*   saving the new rotation/transformation ID to the memory structure (0x6372);
*   saving current angle step (0x636E) and target angle step (0x6378) for the sprite animation;
*   setting the rotation animation step "speed" (0x637C): this value will be added to the current angle step until it reaches the target angle. By default, it is incremented by 8, or 16 if a double-rotation occurred.

#### Pseudo-code algorithm

The routine can be summed up with the following pseudo-code. It may help understand flaws and use them for input trickery (discussed further below).

function rotation\_start()
{
  get\_player\_gamepad\_type;
  read\_gamepad\_input;

  if(both\_cc\_and\_ccw\_buttons) exit;

  if(is\_empty(target)) goto acknowledge;

  if(current\_row < 2) if(target\_cell == bottom || target\_cell == top) exit;

  if(is\_empty(opposite)) goto pushback;

  rotation\_counter\_attempts++;

  if(rotation\_counter\_attempts % 2 == 0) goto double\_rotation;

  if(rotation\_counter\_attempts == 18) if(is\_set(player\_bit)) {
      rotation\_counter\_attempts = 17;
      exit;
    }

double\_rotation:
  set\_double\_rotation\_transform\_id; // instead of normal rotation which keeps the original target transformation ID

pushback:
  shift\_main\_puyo\_coordinates;
  if(y-axis-shift != 0) reset\_in-cell\_vertical\_offset;

acknowledge:
  reset\_double\_rotation\_counter;
  write\_new\_rotation\_id;
  prepare\_sprite\_animation;

}

#### Possible tricks

Due to how the push-back mechanism works, it is possible to:

*   keep the falling pair at a specific position indefinitely, when stuck between columns by double-tapping rotation buttons rapidly (before the current battle drop speed increases past soft drop speed);
*   keep the falling pair hovering at the bottom of the board or over other puyos by rapidly rotating the pair back and forth (granted this double rotation occurs within an 8 frame delay);
*   time a rotation to skip the bouncing animation occurring when placing a puyo, because it is triggered before the puyo actually reaches the bottom of its current cell;

It is not possible to ghost puyos by pushing them back up to the 14th row, as the routine will not allow rotating the puyo to the upright position if the cell below on the 12th row is obstruced. However, it is possible to skip over columns full to the 12th row in the horizontal position, to then rotate and reach a free cell on the 12th row, as depicted below (the main puyo being the red one):

 The last rotation has to be performed before the pair reaches the bottom of its current row. In all, from the rotation putting the pair in the horizontal position, there's an 8-frame delay at 2P normal drop speed during which the rotation is possible before the pair is locked and split.


## Pair Lateral Movement

*Source: <https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Pair_Lateral_Movement> &mdash; 1 diagrams omitted*

Lateral pair movement is handled from the main puyo's callback routine. It is the first movement that gets accounted for when the game deals with gamepad input.

The routine itself is pretty simple:

 Here's the sequence going on here:

*   ends if the pair is already undergoing lateral movement (the shift value is not null), clearing this shift value;
*   ends if the pair is locked;
*   ends if no right or left arrow is pressed;
*   prepares coordinate displacement values according to direction;
*   ends if the target cells are blocked;
*   updates the pairs coordinates on the board;
*   updates shift value to have a smooth transition animation;
*   plays the relevant sound effect and exits.

The shift value is an on-screen displacement measured in pixels, and is set to 8 (half the width of a cell).

#### Notable facts

*   The column change is acknowledged immediately on the same frame the input is detected.
*   The animation takes 2 frames to complete.
*   Lateral movement is subject to input repeat, and cannot be entered in two consecutive frames, even in opposite directions (due to the first check of the routine). See [Puyo Puyo Tsu/Gamepad Input](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Gamepad_Input#Consequences) for more information on possible input tricks and [Puyo Puyo Tsu/Frame Data Tables](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Gamepad_Input_Repeat) for input repeat timings.


## Soft Drop

*Source: <https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop> &mdash; 3 diagrams omitted*

Soft drop (down arrow) is handled by a routine that also deals with regular pair drop besides free-fall, along with "locking" the pair once the player should not have control over it anymore.

#### Contents

*   [1 Overview](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop#Overview)

*   [2 Phases](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop#Phases)
    *   [2.1 Pair lock](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop#Pair_lock)

    *   [2.2 Regular drop speed](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop#Regular_drop_speed)

    *   [2.3 Soft drop speed](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop#Soft_drop_speed)

    *   [2.4 Applying drop speed](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop#Applying_drop_speed)

    *   [2.5 Going on to the next cell](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop#Going_on_to_the_next_cell)

    *   [2.6 Starting the bounce animation](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop#Starting_the_bounce_animation)

    *   [2.7 Grace period at the bottom](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop#Grace_period_at_the_bottom)

    *   [2.8 Grace period override](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop#Grace_period_override)

    *   [2.9 Locking the pair](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop#Locking_the_pair)

    *   [2.10 Animation wait](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop#Animation_wait)

*   [3 Visual summary](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop#Visual_summary)

*   [4 Consequences](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop#Consequences)
    *   [4.1 Motion cancel](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop#Motion_cancel)

*   [5 Frame data](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop#Frame_data)


#### Overview

Here's the fully annotated disassembly of the game routine:

 The routine is split into two main paths (left and right), which both do the very same thing, except one path (on the right-hand side of the graph) is executed only when regular drop speed is higher than soft-drop speed.

When not accounting for this particular case, the routine reads the gamepad to check whether the down arrow is pressed or not. It then adds the relevant drop speed to the pair's vertical offset within the current cell. It finally performs a group of tests, checking whether the pair has reached the bottom of the cell, has crossed mid-height, or should be locked in place. The routine also triggers bouncing animations and placement sound effects.

Finally, that routine is responsible for the "grace period" during which the player retains control over the pair while it being at the bottom of the board or right at the top of a filled column.

#### Phases

This section will refer to the disassembly graph found above, by referring to code addresses.

##### Pair lock

The first check at 0x006116 skips right about to the end of the routine (0x0061F0) if the pair is already locked, preventing the player from moving it further.

##### Regular drop speed

At 0x006128, the routine checks whether the current, regular drop speed is greater or equal to 0x8001, while soft drop speed is hard-coded to 0x8000. This simply checks if the regular drop speed is faster than soft-drop speed, in order to skip reading the gamepad. Thus, **contrary to popular belief**, on _[Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu) revision 0 on Genesis_, it is **not possible to slow down the pair drop at latest solo stages**, where drop speed is higher than soft-drop speed. **This behavior may be present in other revisions, platforms and/or arcade.**

If the current drop speed is high enough, the routing branches to the path dedicated to checking what should happen next and triggering animations. This path has exactly the same steps as below, except for the soft-drop speed section.

##### Soft drop speed

If the down arrow is pressed, current drop speed is set to its hardcoded value of 0x8000 (see address 0x006144). This speed amounts to exactly **8 pixels per frame**.

##### Applying drop speed

At 0x006160, the current drop speed is added to the pair's vertical offset counter. This 16-bit value goes from 0x0000 (top of the current cell) to 0xFFFF (bottom). Mid-height is crossed when the counter is equal to or greater than 0x8000. This counter can be incremented by arbitrary amounts, though default drop speed in a 2P versus game is 0x1000, effectively requiring **16 frames to pass through a single cell**, and **only 2 frames while soft dropping**.

##### Going on to the next cell

At 0x006162, the game detects whether the pair should carry on to the next cell by checking if an overflow occurred while adding to the vertical offset counter (the game added a value that would have made the counter go past 0xFFFF).

The pair's y-coordinate is incremented by one, and the vertical offset is cleared (reset to 0). Clearing that counter instead of keeping the remainder of the overflown counter effectively sets the pair back a little. This means soft-dropping cell after reaching mid-height of a cell is suboptimal, if the cell below is empty. Indeed, the player doesn't make use of the full potential of the 8-pixel distance added from soft-dropping. This can be used to put the falling pair to a well-known state (right at the top of a cell).

This means that while soft-dropping, a pair can only be at one of two vertical offsets, for the duration of a single frame:

*   0x0000, right at the top of a cell;
*   0x8000, right after mid-height.

If the cell below is blocked, the routine skips right to the end to lock the pair.

##### Starting the bounce animation

While the pair is dropping, the game checks for a specific event: if the pair has crossed mid-height of the current cell (at 0x00616C) and if the cell below is occupied (at 0x006174), it will trigger the bouncing animation as well as the placement sound effect. This only occurs if the vertical offset went beyond 0x8000 during that frame, so only right when it crosses that line.

Thus, if a pair is moved sideways after it crossed the mid-height line, if the cell below its target is blocked, the bouncing animation will not be triggered.

##### Grace period at the bottom

Once the pair is blocked at the bottom of the board or over a filled column, a counter is decremented as a grace period (at 0x0061CE). When that counter hits zero from an initial value of 0x20 (set at 0x0061C2), the pair will be locked. This means there is a 32-frame grace period before control lockout.

This counter is initialized the first time the pair encounters an obstacle (i.e. it should have carried on to the next cell but was blocked). It is never reset to its initial value afterward, thus going down a stair-like structure will decrement the same global counter.

Hence, a single pair can only spend a total of 32 frames while being blocked but controllable, even if that time is spent over multiple short-duration phases. This grace period is ignored by soft-dropping and the pair is locked right away.

##### Grace period override

Another counter (at 0x006190) overrides the grace period and triggers the control lockout anyway, if the pair crossed any mid-height line 8 times, while the cell below was being blocked. This is a hard limit at how many times one can push his pair back up. When the counter reaches 8 times, control is locked and the pair is forced at the bottom of the current cell. Since [push back](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back#Push_back) sets the pair just above the mid-height line, it is not possible to overcome this limit.

##### Locking the pair

This is the only routine locking the pair by setting a flag in the pair's memory structure (at 0x0061F0). This will prevent further control by the player as this bit is checked by other movement routines.

##### Animation wait

Right after locking the pair, the routine still loops while the current rotation animation has not ended. It is thus impossible to skip the final rotation animation when placing a pair (most probably when soft-dropping). This rotation animation takes 8 frames to complete.

#### Visual summary

Here's a GIF animation summarizing the 16 frames of a regular drop through a single cell, with the mid-height trigger for the bouncing animation explained. Note the actual animation is not properly synchronized with the actual game status, making it difficult to find accurate visual clues.

#### Consequences

*   On Genesis, with Puyo Puyo Tsu revision 0, it is impossible to slow down the falling pair by pressing down is the latest solo stages of the game.
*   It is possible to skip the bouncing animation when placing a pair (motion cancel).
*   It is impossible to skip the rotation animation.
*   It is impossible to skip the bouncing animation of a free-falling puyo.
*   There is a global grace period of 32 frames before control lockout, when the falling pair is blocked. It means a single pair can only spend a total of 32 frames being blocked but controllable, even if that time is spent over multiple short-duration phases. This grace period is ignored by soft-dropping and the pair is locked right away.
*   It is only possible to push a pair back up 8 times for the duration of its lifetime. On the 8th attempt, the pair will be locked out.

##### Motion cancel

Motion cancel is a trick enabling skipping the bouncing animation in order to save time (a few frames at each successful attempt).

The bouncing animation lasts for 16 frames, and is triggered only when crossing mid-height of the last free cell of a column. Thus, by making a pair fall through an empty column (or at least with one more free cell that the target column), the player can wait until the pair reached the desired vertical position, then wait for it to cross the mid-height line on the same column, then moving the pair sideways to the intended destination to skip the trigger point of the bouncing animation.

The following GIF animation explains the whole process, with P1 performing the motion cancel trick while P2 is not.

 The trick can save up to 16 frames only, per successful attempt:

*   the bouncing animation lasts for 16 frames;
*   soft-dropping triggers the bouncing animation on the very last frame of the falling pair's presence in a cell, this frame being the first bouncing.

It is not possible to save time if the trick is performed without soft-dropping, as the grace period will not be skipped, imposing a 32-frame delay before control lockout.

The bouncing animation lasts for 16 frames but starts 1 or 8 frames before hitting the ground. The rotation animation lasts for 8 frames and cannot be skipped when the pair hits the ground.

#### Frame data

Drop speed frame data tables are maintained on the [dedicated page](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Drop_speed)
.


## Free fall

*Source: <https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Free_fall> &mdash; 1 diagrams omitted*

Free fall occurs in various situations:

*   when the player splits his pair in two;
*   when receiving ojama puyos;
*   when a chain disappears, puyos above the hole fill the gap by free-falling.

Their speed is subject to gravity, a force increasing their speed as each frame passes.

#### Contents

*   [1 Gravity routine](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Free_fall#Gravity_routine)

*   [2 Equation parameters](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Free_fall#Equation_parameters)
    *   [2.1 Pair-splitting parameters](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Free_fall#Pair-splitting_parameters)

    *   [2.2 Bulk ojama parameters](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Free_fall#Bulk_ojama_parameters)

*   [3 Frame data](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Free_fall#Frame_data)


#### Gravity routine

 The routine works on absolute, on-screen pixel coordinates. It will apply them displacement values and check whether or not the coordinates crossed a cell boundary to advance the board coordinates accordingly.

beam-x and beam-y are the on-screen coordinates, which are stored on 2 bytes. These value are extended by 16 trailing bits, which will accumulate sub-pixel displacements, but this part is truncated when displaying the sprite, so we only have full pixel values. Hence, the speed and acceleration are represented by 32-bit values, the highest 16 of which directly impact the pixel coordinate, while the lowest 16 will have an effect only because of arithmetic carry.

The routine does the following:

*   applies the displacement to the vertical axis coordinate according to the current free-falling speed;
*   checks if the puyo has crossed to a new board cell;
*   checks if the new cell is blocked, and exits if that's the case, rounding the coordinates to the bottom of the last free cell;
*   updates the puyo's board coordinates accordingly;
*   updates the free-falling speed by applying gravity acceleration (add acceleration value to the speed that will be used at the next frame);
*   cap the speed to terminal velocity.

#### Equation parameters

The gravity parameters differ depending on the situation.

##### Pair-splitting parameters

These parameter are used when splitting a pair in two:

*   initial speed: 0x10000 (1 pixel/frame)
*   terminal velocity: 0x80000 (8 pixels/frame)
*   acceleration: 0x03000 (0.1875 pixels/frame2)

##### Bulk ojama parameters

These parameters are valid when ojamas fall in bulk, i.e. when receiving at least a full rock of garbage:

*   initial speed: 0 (0 pixel/frame)
*   terminal velocity: 0x80000 (8 pixels/frame)
*   acceleration for column 1: 0x02400 (0.140625 pixels/frame2)
*   acceleration for column 2: 0x02600 (0.1484375 pixels/frame2)
*   acceleration for column 3: 0x02000 (0.125 pixels/frame2)
*   acceleration for column 4: 0x02A00 (0.1640625 pixels/frame2)
*   acceleration for column 5: 0x02200 (0.1328125 pixels/frame2)
*   acceleration for column 6: 0x02800 (0.15625 pixels/frame2)

#### Frame data

Free-fall frame data tables are maintained on the [dedicated page](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Free_fall_speed)
.


## Frame Data Tables

*Source: <https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables> &mdash; 1 diagrams omitted*

This page sums up frame data timings for various game sequences, events, animations, etc.

#### Contents

*   [1 Gamepad Input Repeat](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Gamepad_Input_Repeat)

*   [2 Rotation](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Rotation)

*   [3 Drop speed](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Drop_speed)
    *   [3.1 Grace period](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Grace_period)

*   [4 Bouncing animation](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Bouncing_animation)

*   [5 Pair split speed](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Pair_split_speed)

*   [6 Free fall speed](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Free_fall_speed)
    *   [6.1 Free falling puyo after pair split](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Free_falling_puyo_after_pair_split)

    *   [6.2 Free falling bulk ojama puyos (rocks)](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Frame_Data_Tables#Free_falling_bulk_ojama_puyos_(rocks))


#### Gamepad Input Repeat

_**This data is fully reverse-engineered**_

Internal details of the game routines are discussed on the following page: [Puyo Puyo Tsu/Gamepad Input](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Gamepad_Input)
.

This table sums up default frame counts for gamepad input repeat during different phases.

| Phase | Delay until repeat | Button repeat |
| --- | --- | --- |
| During game | 8   | 2   |
| Main menu | 16  | 3   |
| Settings menu | 16  | 5   |
| Solo menu | 32  | 8   |

This means that, during a game, there's an 8-frame delay before a pressed button is first repeated, while subsequent repeats are 2 frames apart (1 frame of inactive button before repetition on the 2nd frame).

An input sequence is as follows:

1.  a button is held down, being active for 1 frame
    *   the button's action may be triggered there
2.  7 frames pass with the button being inactive
    *   the counter is decremented for each frame no new input is detected
3.  a single frame has the button active again
    *   counter value was 1, gets decremented to zero, then gets reset to 2 and the button is thus made active, all in the very same frame
    *   the button's action may be triggered again there
4.  1 frame passes with the button inactive
    *   the counter is decremented and now equals 1
5.  go back to #3 unless a new button is pressed

Note: the game keeps two distinct timers:

*   for Right/Left arrows
*   for Up/Down and Start/A/B/C together

#### Rotation

_**This data is fully reverse-engineered**_

Internal details of the game routines are discussed on the following page: [Puyo Puyo Tsu/Rotation, collision and push back](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Rotation,_collision_and_push_back)
.

Rotation frame data is currently measured at:

*   Action input lock: none. The player can trigger a rotation on two consecutive frames.
*   Same button lock: 1-frame lock-out, due to gamepad readout routine. The player cannot input the same button alone in two consecutive frames.
*   Rotation animation: 7 frames for calculation. May be skipped if a new rotation input occurs before the end.

Rotation is acknowledged by the game on the same frame the input was read from the gamepad. There is no transition phase for the collision data: new coordinates are immediately used upon rotation acknowledgment (on the next frame).

The following breakpoints were used in the MESS debugger to validate this data (they won't work if two players are rotating pieces at the same time):

bpset 62D2, 1, {printf "x%06X ROTATION: clockwise (frame: %d)", pc, frame; temp0=frame; g}
bpset 62CC, 1, {printf "x%06X ROTATION: counter-clockwise (frame: %d)", pc, frame; temp0=frame; g}
bpset 6458, b@(a0+0x37) == b@(a0+0x36), {printf "0x%06X ROTATION: end (frame: %d count: %d)", pc, frame, frame-temp0; g}

Here's a sample of the trace log of rotation animations:

 The following section discusses the input tricks possible with an actual gamepad: [Puyo Puyo Tsu/Gamepad Input](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Gamepad_Input#Consequences)
 (see "Consequences").

#### Drop speed

**This data is _partially_ reverse-engineered**

This table sums up frame timings of falling pairs depending on initial speed and soft dropping (i.e. pressing the down arrow).

| Speed | Regular drop speed | Soft drop speed |
| --- | --- | --- |
| 2P versus Easiest (difficulty level 1) | 16  | 2   |
| 2P versus Easy (difficulty level 2) | 16  | 2   |
| 2P versus Normal (difficulty level 3) | 16  | 2   |
| 2P versus Hard (difficulty level 4) | 16  | 2   |
| 2P versus Hardest (difficulty level 5) | 8   | 2   |
| 1P solo level 1 | 32  | 2   |

_(table to be completed)_

Values are the number of frames needed for a pair to go through a single cell. Thus, going through the entire height of the board will take 12\*16=192 frames (3.2 seconds) at normal speed (2P versus mode), and 32 frames (0.53 seconds) while soft dropping.

Notes:

*   soft drop is interrupted by any other direction (R/L) that would be sent to the game while soft dropping;
*   soft drop is not affected by the button input repeat mechanism: a full gamepad readout is performed for every single frame.

##### Grace period

_**This data is fully reverse-engineered**_

Internal details of the game routines are discussed on the following page: [Puyo Puyo Tsu/Soft Drop##Consequences](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop##Consequences)
.

*   A falling pair **can spend a total of 32 frames while being blocked** by an obstacle below. **Soft-dropping (holding down) will skip this grace period** and lock the pair right away.
*   This grace period is also skipped if the player pushes his falling pair back up 8 times. On the 8th attempt, the pair is locked right away.

#### Bouncing animation

_**This data is fully reverse-engineered**_

Internal details of the game routines are discussed on the following page: [Puyo Puyo Tsu/Soft Drop##Consequences](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop##Consequences)
.

*   Bouncing animation lasts for **16 frames**.
*   It can be skipped with [motion cancel](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Soft_Drop##Motion_cancel) .

#### Pair split speed

_**This data is fully reverse-engineered**_

Internal details of the game routines are discussed on the following page: [Puyo Puyo Tsu/Falling Pair Control#Pair\_split\_and\_free-fall](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Falling_Pair_Control#Pair_split_and_free-fall)
.

From control lock (pair lock) to actual splitting (the moment a puyo starts free-falling), there is a fixed delay:

| Free-falling puyo | Frame delay _before_ free-fall begins |
| --- | --- |
| Main puyo | 1   |
| Slave puyo | 2   |

Those are frames during which _nothing_ is applied to the puyos, which delays actual free-fall and placement on the board.

#### Free fall speed

Free fall speed differs for:

*   puyos controlled by the player, after splitting a pair in two;
*   single ojama puyos;
*   bulk ojama puyos, depending on their respective column;
*   puyo freefalling after a chaining event.

##### Free falling puyo after pair split

_**This data is fully reverse-engineered**_

Internal details of the game routines are discussed on the following page: [Puyo Puyo Tsu/Free fall](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Free_fall)
.

A board cell is 16 pixels high, initial freefall speed is 1 pixel/frame (0x10000), acceleration due to gravity is 0.1875 pixels/frame2 (0x03000). Terminal velocity is capped at 0x80000 (8 pixels/frame, half a cell), in theory reached in 39 frames. Due to roundoff error, terminal velocity is actually attained much quicker, in about 31 frames (after freefalling for 7 cells).

Puyo lockout occurs at the last frame of the freefall.

The following tables shows the accurate number of frames required for a puyo to freefall through a specific number of board cells:

| Height | Total frame count |
| --- | --- |
| 1 cell | 10  |
| 2 cells | 15  |
| 3 cells | 19  |
| 4 cells | 22  |
| 5 cells | 25  |
| 6 cells | 28  |
| 7 cells1 | 31  |
| 8 cells | 33  |
| 9 cells | 35  |
| 10 cells | 37  |
| 11 cells2 | 39  |
| 12 cells | 41  |
| 13 cells3 | 43  |

*   1: reached actual terminal velocity (due to roundoff errors)
*   2: reached theoretical terminal velocity
*   3: game physics prevent this fall from being possible

##### Free falling bulk ojama puyos (rocks)

**This data is _partially_ reverse-engineered**

Internal details of the game routines are discussed on the following page: [Puyo Puyo Tsu/Free fall](https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Free_fall)
.

Initial free-fall speed is 0 pixel/frame (0x0000) in a 2P versus battle, acceleration due to gravity depends on the column. Terminal velocity is capped at 0x80000 (8 pixels/frame, half a cell). Due to roundoff error, terminal velocity is actually attained much quicker that the theoretical value. Some columns have parameters that make reaching terminal velocity impossible.

Ojama puyo lockout occurs at the last frame of the freefall.

The following tables shows the accurate number of frames required for an ojama puyo to freefall through a specific number of board cells, depending on its column:

| Height | Column 1 | Column 2 | Column 3 | Column 4 | Column 5 | Column 6 |
| --- | --- | --- | --- | --- | --- | --- |
| 1 cell | 16  | 16  | 17  | 15  | 17  | 15  |
| 2 cells | 22  | 22  | 24  | 21  | 23  | 21  |
| 3 cells | 27  | 26  | 29  | 25  | 28  | 26  |
| 4 cells | 31  | 30  | 33  | 29  | 32  | 30  |
| 5 cells | 35  | 34  | 37  | 32  | 36  | 33  |
| 6 cells | 38  | 37  | 40  | 35  | 39  | 36  |
| 7 cells | 41  | 40  | 43  | 38  | 42  | 39  |
| 8 cells | 44  | 43  | 46  | **41**1 | 45  | 41  |
| 9 cells | 46  | 45  | 49  | 43  | 48  | 44  |
| 10 cells | **49**1 | 47  | 52  | 45  | 50  | 46  |
| 11 cells | 51  | **50**1 | 54  | 47  | 52  | 48  |
| 12 cells | 53  | 52  | 56  | 49  | **55**1 | **51**1 |
| 13 cells2 | 55  | 54  | 59  | 51  | 57  | 53  |

*   1: reached actual terminal velocity (due to roundoff errors)
*   2: game physics prevent this fall from being possible


## Random Number Generator

*Source: <https://puyonexus.com/wiki/Puyo_Puyo_Tsu/Random_Number_Generator> &mdash; 2 diagrams omitted*

When the game gives the player random pairs, there obviously is some logic to it. In most computer systems, randomization is not actually random and can be predicted: one has to know the algorithm and how it is used, to then infer some properties that make it predictable.

[Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu) is no exception, and has what is called a random number generator (RNG). This page describes how it works.

#### Overall RNG mechanics

*   a variable is declared to stand for the current random value (rng\_val);
*   when the system boots, this variable is set to a predefined value;
*   a function then modifies this value every time it is called, based on various parameters, but should attempt to make it difficult to predict the next value;
*   when needed, read the variable and "iterate" the random value so that the next code chunk that wants a random value does not get the same as yours.

[Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu) has those functions and variables:

*   the game stores the current random value at 0xFFA134;
*   it is initialized at boot time by function sub\_2216 (subroutine which code begins at 0x002216), which initializes the game options to their default values. With instruction 0x222C the routine writes the default "random" value of 0x35879DE2;
*   function sub\_C04 is the "iterator", equivalent to rand() in most programming languages, which picks the next random value based on its calculations and the old value stored in that variable.

_Function offsets are specific to the Genesis version._

On the arcade version, the initialization function lies at 0x23FA and the rand iterator function is at 0x4AA. The initial value of the rng\_val variable is the same.

#### RNG implementation

Here's the initialization function:

 And here's the rand() iterator function:

 This RNG function calculates the next value as follows:

*   assign old value to d0 from the variable in RAM, adding it to the current d0 content
*   add the value of the stack pointer register to d0
*   rotate the contents of d0 by 5 bits to the left: higher bits get placed at the lowest place
*   add the old value to d0
*   increment d0 by one
*   store the new value to the variable in memory

The function also returns the generated value, as it is not erased from d0: that's the return value, a new 32-bit random number.

#### Sample C code

The following C code accurately mimics [Puyo Puyo Tsu](https://puyonexus.com/wiki/Puyo_Puyo_Tsu)
's randomization behaviour:

#include <stdio.h>
#include <stdlib.h>

#define TRIALS 1<<16

unsigned int \_rotl(const unsigned int value, int shift) {
    if ((shift &= sizeof(value)\*8 - 1) == 0) return value;
    return (value << shift) | (value >> (sizeof(value)\*8 - shift));
}

unsigned int \_rotr(const unsigned int value, int shift) {
    if ((shift &= sizeof(value)\*8 - 1) == 0) return value;
    return (value >> shift) | (value << (sizeof(value)\*8 - shift));
}

int main(int argc, const char\*\* argv)
{
   unsigned int seed = 0x35879DE2;
   unsigned int i;
   unsigned int tmp;

   printf("sizeof(unsigned int): %ld bits\\n", sizeof(unsigned int)\*8);
   if(sizeof(unsigned int) != 4) puts("WARNING: integer type is not 32-bit long");

   for (i = 0; i < TRIALS; i++) {
      /\* tmp holds d0 value that depends from where the function is called,
      so is a yet unknown variable.
      The following initialization is accurate for the first 7 values of the RNG,
      if you let the animation play through, on the genesis.

      For the color-set shuffling, it holds 0x00000004.
      For the pool randomization process, it holds the previous RNG value masked
      with 0xFFFF00FF, and is initialized to 0x4AD00000.
      \*/
      if(i%2 == 0) tmp = 0xF1;
      else
         tmp = 0xF0;
      tmp += seed;
      printf("0x%08X ", tmp);
      tmp += (unsigned int) 0x00FF7FE8; /\* 0x00FF7FF4 most likely to be the sp at generation \*/
      printf("0x%08X ", tmp);
      tmp  = \_rotl(tmp,5);
      printf("0x%08X ", tmp);
      tmp += seed;
      printf("0x%08X ", tmp);
      tmp++;
      printf("0x%08X \\n", tmp);
      seed = tmp;

      printf("0x%08x: %3d (0x%02x) \[mod 4: %d\]\\n", seed, seed & 0xFF, seed & 0xFF,(seed & 0xFF)%4);
   }

   return EXIT\_SUCCESS;
}

The code will generate 65536 values and print the results. tmp stands for the d0 register. \_rotl() is the rotation function. Beware, this code sample only works on systems where unsigned int is a 32-bit wide type.

