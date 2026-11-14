# RIVERS design proofs

Reproductions run while [the design](../../../plans/RIVERS/DESIGN.md#check-that-it-can-work) was written, on 14
November 2026. They are evidence for the design's choices and the source of the bounds in M1 to M3 of
[the done-when file](../../../plans/RIVERS/DONE.json). They are not engine code and no engine test depends on
them. Parts 1 to 5 and 7 are sections in f64, part 6 a small grid in plan; none is a GPU or a visual
measurement. The scheme is first order in space, so its errors are a ceiling for the solver, not its target.

Run `python3 river_proof.py`, about half a minute; its output is `river-proof.txt`.

| Part | What it shows |
|---|---|
| 1 | The scheme. Water at rest round dry ground stays at rest to rounding. A steady flow over a bump puts its hydraulic jump within a cell of the closed form, with a mean depth error of 2.0% of the critical depth at 0.25 m cells and 1.1% at 0.125 m. With friction, over a bed made for a known depth, a slow flow is within 0.2% and a fast one within 12.8%, 5.9% and 2.8% at cells of 0.5, 0.25 and 0.125 m. A thin sheet on a slope that falls more across a cell than the water is deep stands 23% over Manning's normal depth with Audusse's reconstruction and 1% to 5% over with Chen and Noelle's. Friction taken at the new discharge biases the steady state by the time step; taken at the old one it does not |
| 2 | A stream in section: riffle, pool behind a sill, chute, ledge, run. Discharge is the inflow's through every face to 0.001%; the pool is level to 15 mm and stands 0.22 m over its sill; head lost cell by cell sums to the discharge times the head between the ends within 0.3%. The flow's chop comes to 7.7 mm in the riffle, 0.1 mm in the pool and 14.7 mm in the chute. Whitewater power is 529 W/m² at the ledge, 66 W/m² in the chute and under 7 W/m² elsewhere |
| 3 | A reach below a lake, solved at nine levels of the lake. Discharge runs from 0.005 to 0.43 m²/s over 0.6 m of level. The row between two solutions 0.15 m apart is within 2.6 mm in depth and 10% in speed of a solve at that level; 0.30 m apart, 5.6 mm and 19%. Left as prepared at the middle level, the reach carries 63% too little at the top of the range and 30 times too much at the bottom |
| 4 | A fall, by unit discharge and height: speed, throw and thickness, the share of it past its breakup length, and its power. A trickle of 0.005 m²/s is past its breakup length after 1.1 m; 1 m²/s keeps a whole sheet for 6 m. With the lip's face passing the flow at its critical depth, the last cell above a lip stands at 1.06 to 1.15 of the critical depth where the flow above is slow and at its own depth where it is fast; with a lower dry cell in place of that face it stands at 0.22, and a sheet launched from it would be thrown 3.2 times too far. Pressed on the surface, a plunge's whole momentum would make a dent over half a metre deep |
| 5 | Reaching the steady state: 4,160 steps for 240 cells with each cell's own time step, 6,370 with one step from a dry start. A filled start and a coarser grid first save little in a section. A 350 m stream at 0.25 m cells is about 540 million cell updates for each flow sample |
| 6 | A stream in plan through a narrows and past a rock: discharge through six sections within 0.2% of the spring's; the water quickens from 0.58 to 1.46 m/s where the width falls from 5 m to 2 m; it stands 0.12 m higher in front of the rock; 80% of the head lost beyond friction is lost round the rock and below the narrows, in 34% of the wet cells |
| 7 | One mean current for a 64 m interaction window over a riffle, a pool and a chute is 0.94 m/s where the pool's water moves at 0.18 m/s, so rings in the pool would slide 4.7 m in 5 s. A mean weighted by where the waves are gives the pool's own current |
