// SPDX-License-Identifier: MIT

using System;
using System.Threading.Tasks;
using MegaCrit.Sts2.Core.Entities.RestSite;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Models.Relics;
using MegaCrit.Sts2.Core.Nodes.RestSite;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private sealed class ExpertRestPending
    {
        internal ExpertRestPending(
            RuntimeV4ExpertRestOperation operation,
            RuntimeV4ExpertRestActionReference action,
            RuntimeV4ExpertGameplayObservation before,
            NRestSiteButton button,
            RestSiteOption option)
        {
            Operation = operation;
            Action = action;
            Before = before;
            Button = button;
            Option = option;
        }

        internal RuntimeV4ExpertRestOperation Operation { get; }
        internal RuntimeV4ExpertRestActionReference Action { get; }
        internal RuntimeV4ExpertGameplayObservation Before { get; }
        internal NRestSiteButton Button { get; }
        internal RestSiteOption Option { get; }
        internal ExpertRestSelector? Selector { get; set; }
        internal Task? Work { get; set; }
        internal RuntimeV4ExpertRestHostCompletion? Completion { get; set; }
        internal ExpertRestNativeExpectation? NativeExpectation { get; set; }
        internal ExpertRestNativeCallback? NativeCallback { get; set; }
    }

    private sealed class ExpertRestNativeExpectation
    {
        internal ExpertRestNativeExpectation(
            string optionId,
            ulong playerId,
            int before,
            PumpkinCandle? kindle,
            Girya? lift)
        {
            OptionId = optionId;
            PlayerId = playerId;
            Before = before;
            Kindle = kindle;
            Lift = lift;
        }

        internal string OptionId { get; }
        internal ulong PlayerId { get; }
        internal int Before { get; }
        internal PumpkinCandle? Kindle { get; }
        internal Girya? Lift { get; }
    }

    private readonly record struct ExpertRestNativeCallback(
        string OptionId,
        bool Success,
        ulong PlayerId);
}
