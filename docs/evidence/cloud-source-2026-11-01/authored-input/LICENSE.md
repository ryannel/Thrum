# WDAS cloud shape attribution and license

The source cloud dataset is Copyright 2017 Disney Enterprises, Inc., licensed
under [Creative Commons Attribution-ShareAlike 3.0 Unported](https://creativecommons.org/licenses/by-sa/3.0/).
The reference photograph is by Kevin Udy, Colorado Clouds Blog,
[26 July 2012 cumulus photograph](https://coclouds.com/436/cumulus/2012-07-26/),
also under CC BY-SA 3.0.

Source: [Walt Disney Animation Studios Cloud Data Set](https://www.disneyanimation.com/resources/clouds/).
Official [dataset license notice](https://media.disneyanimation.com/uploads/production/data_set_asset/6/asset/License_Cloud.pdf).

`shape.r8` is a Thrum-authored adaptation of the quarter-resolution density grid:
uniform geometry scale 0.6, translated to a fixed cube, sampled at 128 cubed,
with a fixed authored optical conversion and R8 quantization. These are changes
to the original; this is not the original dataset or its physical-unit calibration.
The derived shape and its accompanying shape metadata are distributed under
CC BY-SA 3.0. Preserve this attribution and the source/license links when sharing
the adaptation. No endorsement by Disney or the photographer is implied.

The generating Python recipe is separate tooling; the dataset license governs
the adapted density asset and its descriptive metadata.
