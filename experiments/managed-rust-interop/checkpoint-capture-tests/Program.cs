// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class Program
{
    private static int Main()
    {
        try
        {
            CaptureSeamTests.CaptureRefusesOffHostThread();
            CaptureSeamTests.CaptureRefusesMidEffectAndEnemyExecutionSettlement();
            CaptureSeamTests.CaptureRefusesPendingSelectionTransition();
            CaptureSeamTests.CaptureRefusesSettlementChangeAndReentryInsideTheWindow();
            CaptureSeamTests.CaptureCopiesOwnedBytesAndReleasesHostReferences();
            CaptureSeamTests.CaptureDoesNotAdvanceObservationGenerationOrRng();
            CaptureSeamTests.CaptureRejectsRequiredUnknownAndMisplacedFamilies();
            CaptureSeamTests.CaptureRejectsBoundViolationsAndHostReadFailures();
            CaptureSeamTests.EveryAdvertisedPhaseStillReportsUnavailable();
            PayloadSchemaTests.ValidatorAgreesWithPinnedConformanceFixtures();
            PayloadSchemaTests.CapturedRecordValidatesAgainstPayloadSchemaV1();
            Console.WriteLine("CheckpointCaptureProbe: PASS");
            return 0;
        }
        catch (Exception exception)
        {
            Console.Error.WriteLine("CheckpointCaptureProbe: FAIL: " + exception.Message);
            return 1;
        }
    }

    internal static void Check(bool condition, string message)
    {
        if (!condition)
            throw new InvalidOperationException(message);
        Console.WriteLine("PASS: " + message);
    }
}
