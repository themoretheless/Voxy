# Neutral skeletal and soft-tissue close-up

Current source rendered 241 native Metal frames at 20 fps over a 12 second
walk/jump/settle cycle on Apple M4 Max. motion.gif combines those frames;
poses.png shows t=0, 0.7, 4.5 and 9 seconds. CSV contains actual secondary
node displacement relative to the skeletal reference and energy receipts.

Changes: optional --close-up in the existing snapshot example; live phase and
maximum secondary displacement in millimetres in the existing body_motion window.
No constitutive or anatomical calibration change. The coarse rounded tetrahedral
surface remains visibly faceted and exaggerated; this is a technical mannequin.

Validation: all 11 tissue_demo library tests passed, zero failed/ignored;
GPU validation scope clean; git diff --check passed. Native body_motion launched
from the current release build and left running for interactive use (Space pauses).
Work is local; no commit or push. Broader engine goal remains incomplete.
