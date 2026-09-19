// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// The launch precondition the production live runtime evaluates before it replaces the save
/// backend, and the recovery code a runtime consumer reads when that precondition refuses.
///
/// A refused contract and a lane that never declared one are different failures that answered the
/// same code, and the refusal additionally suppressed the listener, so a consumer could not tell a
/// misdeclared user directory from a port that never opened or from a runtime nobody asked for
/// (sts2-game-mod#185). The stable reason token the refusal already carried in <c>game.log</c> now
/// crosses the wire as the recovery code.
///
/// Only the token crosses. The diagnostic names the directories the check compared, which is
/// operator evidence, and stays in <c>game.log</c>.
///
/// This type is deliberately host-independent. It depends only on the base class library, so the
/// same classification compiles into the source-only managed probe that runs without a game.
/// </summary>
internal static class LaunchContractRefusal
{
    /// <summary>Code for a lane that never declared a launch contract. Unchanged.</summary>
    internal const string NotDeclared = "host_not_configured";

    /// <summary>Prefix every refusal code carries, so a consumer can classify without a list.</summary>
    internal const string RefusedPrefix = "launch_contract_refused";

    /// <summary>Code for a refusal whose reason cannot be named on the wire.</summary>
    internal const string Unclassified = RefusedPrefix;

    /// <summary>Stable reason token for the campaign precondition, which no host check reports.</summary>
    internal const string CampaignRequiredReason = "campaign_required";

    /// <summary>
    /// Bounded so a reason cannot produce an unbounded code. Every token the refusal currently names
    /// is far shorter; the bound only fails closed.
    /// </summary>
    private const int MaxReasonLength = 64;

    private static volatile string _recorded = string.Empty;

    /// <summary>The refusal recorded at load, or empty when no launch contract was refused.</summary>
    internal static string Recorded => _recorded;

    /// <summary>
    /// The code a consumer reads while no host is configured: the recorded refusal when the launch
    /// contract was refused, otherwise the never-declared code that predates this vocabulary.
    /// </summary>
    internal static string UnconfiguredCode => _recorded.Length == 0 ? NotDeclared : _recorded;

    /// <summary>True when a code reports a refused launch contract rather than a missing one.</summary>
    internal static bool IsRefusal(string code) =>
        string.Equals(code, RefusedPrefix, StringComparison.Ordinal)
        || code.StartsWith(RefusedPrefix + "_", StringComparison.Ordinal);

    /// <summary>
    /// Composes the recovery code for a refusal reason token. The token is the vocabulary the refusal
    /// already names in <c>game.log</c>, so a reason added there is answered here without a second
    /// list to keep in step. A token that cannot be a wire identity degrades to the unclassified
    /// refusal, because the consumer must still learn that the contract was refused.
    /// </summary>
    internal static string CodeFor(string reason) =>
        IsWireToken(reason) ? RefusedPrefix + "_" + reason : Unclassified;

    /// <summary>
    /// Records a refusal immediately before it is thrown, so the game log line and the recovery code
    /// name the same reason. Written once during load, before the listener accepts a request.
    /// </summary>
    internal static void Record(string reason) => _recorded = CodeFor(reason);

    /// <summary>Clears the record, so a probe can observe both vocabularies in one process.</summary>
    internal static void Clear() => _recorded = string.Empty;

    private static bool IsWireToken(string value)
    {
        if (value.Length == 0 || value.Length > MaxReasonLength)
        {
            return false;
        }

        foreach (char character in value)
        {
            if (!char.IsAsciiLetterOrDigit(character) && character != '_' && character != '-')
            {
                return false;
            }
        }

        return true;
    }
}
