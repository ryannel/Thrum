# SWARM planning probe: how small must an enemy be for thousands to be in view?

13 November 2026. Arithmetic done while planning [SWARM](../../plans/SWARM/DESIGN.md#check-that-it-can-work). No
engine code was run, and the script was thrown away on purpose: build from the design.

**Question.** The brief's default asks for "thousands of enemies and projectiles on screen at once". How large
can a common enemy's body be, in a 16:9 view, for 2,000 bodies to be inside it?

**Method.** A view of 1920×1080. A clear disc around the hero of radius 15% of the short side. Bodies are discs
of diameter `d` that cover a share of the rest of the view. Count = share × (view area − clear disc) ÷ disc
area.

| Body diameter | 25% cover | 40% cover | 60% cover | 90% cover |
|---|---|---|---|---|
| 36 px (short side ÷ 30) | 489 | 782 | 1,174 | 1,761 |
| 27 px (÷ 40) | 869 | 1,391 | 2,087 | 3,130 |
| 22 px (÷ 50) | 1,358 | 2,174 | 3,260 | 4,890 |
| 18 px (÷ 60) | 1,956 | 3,130 | 4,695 | 7,042 |

Hexagonal packing of touching discs covers about 91%, so 90% is a crowd shoulder to shoulder across the whole
view.

**Reading.** Bodies a thirtieth of the short side cannot reach 2,000 even when they fill the view. A fortieth
needs most of the view covered. A fiftieth reaches 2,000 at 40% cover and leaves room for the player to see
the ground, so the design takes about a fiftieth: 22 px at 1080p, 45 px at the built-in display's 2234.
Projectiles and pick-ups are smaller and add to the count without much cover.

**Not measured.** Whether a character that small reads. That is taste and legibility, which M1's mock arenas
show the owner at game size before anything is built.
