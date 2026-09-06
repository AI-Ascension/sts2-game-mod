# ADR 0028: Native hand upgrade selection

Status: source implemented; native verification pending.

A live Linux Armaments play entered the hand's native upgrade selector rather than an
overlay card grid. The previous adapter projected ordinary combat while that parent
action waited for a choice. The action remained unknown and the harness stopped.

The adapter now projects visible, clickable hand holders in native UpgradeSelect mode
as card selections, associated with exactly one retained parent card action gathering
player choice. Other hand selection modes block ordinary combat actions but are not
admitted by this change. An active overlay takes precedence.

Selection invokes the holder's public native click signal and then the unique visible
native confirmation control. It does not synthesize OS input or directly upgrade a card.
Completion requires the selector to close, the selected card's upgrade level to increase,
and the retained parent action to finish successfully. An ambiguous control or a changed
surface does not establish settlement. The original unknown live operation is preserved;
source changes do not retroactively settle it.

Host compilation is compatibility evidence only. A fresh native replay to this boundary
and model-controlled selection must be verified before claiming runtime support.
