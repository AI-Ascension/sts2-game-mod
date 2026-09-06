# ADR 0028: Native hand upgrade selection

Status: implemented; focused native Linux selection verified. Full campaign continuation pending.

A live Linux Armaments play entered the hand's native upgrade selector rather than an
overlay card grid. The previous adapter projected ordinary combat while that parent
action waited for a choice. The action remained unknown and the harness stopped.

The adapter now projects visible, clickable hand holders in native UpgradeSelect mode
as card selections, associated with exactly one retained parent card action gathering
player choice. Other hand selection modes block ordinary combat actions but are not
admitted by this change. An active overlay takes precedence.

Native diagnostics confirmed that UpgradeSelect leaves each holder's `InSelectMode`
false while the holders remain visible and clickable. Admission therefore uses the
hand's actual mode and those native controls, without the incompatible holder flag.

Selection invokes the holder's public native click signal and then the unique visible
native confirmation control. It does not synthesize OS input or directly upgrade a card.
Completion requires the selector to close, the selected card's upgrade level to increase,
and the retained parent action to finish successfully. An ambiguous control or a changed
surface does not establish settlement. The original unknown live operation is preserved;
source changes do not retroactively settle it.

An explicitly generated-card Linux combat fixture verified Astra playing Armaments,
choosing a Strike, and observing its upgrade with a settled parent action. See the
[focused evidence](../evidence/native-hand-upgrade-20260906.md). Host compilation alone
is compatibility evidence. This focused test does not establish full campaign completion.
