// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Reflection;
using System.Threading;
using System.Threading.Tasks;
using AiAscension.Sts2ModelDbRegistryProbe;
using AiAscension.Sts2ModelDbRegistryProbe.Tests;
using MegaCrit.Sts2.Core.Modding;
using MegaCrit.Sts2.Core.Models;

namespace AiAscension.Sts2ModelDbRegistryProbe.Tests;

internal static partial class Program
{
    private static readonly RegistryProbeLimits NormalLimits = new(
        MaxEntries: 32,
        MaxIdentityBytes: 32,
        MaxTypeNameBytes: 128,
        MaxOwnedStringBytes: 1024,
        MaxCaptureDuration: TimeSpan.FromSeconds(30));

    private static async Task<int> Main()
    {
        try
        {
            TestStaticFieldReadCanInitializeOwnerType();
            TestExactFieldGuard();
            TestCaptureCopiesOnlyOwnedValues();
            TestLoaderReadinessSignals();
            TestVersionThreadAndCancellationRefuse();
            TestCountStringAndDeadlineBounds();
            TestMutationAndResolverFailuresRefuse();
            TestBoundedReport();
            await TestBinaryHashPreflight();
            Console.WriteLine("All ModelDb registry probe tests passed.");
            return 0;
        }
        catch (Exception exception)
        {
            Console.Error.WriteLine(exception.Message);
            return 1;
        }
    }

    private static void TestExactFieldGuard()
    {
        FieldInfo? valid = ModelDbRegistryCapture.FindPinnedField(typeof(ModelDb));
        Check(valid is not null, "finds the exact synthetic private static field");
        Check(ModelDbRegistryCapture.RequirePinnedField(typeof(ModelDb), valid).FieldType
            == typeof(Dictionary<ModelId, AbstractModel>), "accepts only the closed registry type");

        ExpectFailure("registry_shape_unsupported", () =>
            ModelDbRegistryCapture.RequirePinnedField(typeof(ModelDb), null));
        ExpectFailure("registry_shape_unsupported", () =>
            ModelDbRegistryCapture.RequirePinnedField(typeof(ModelDb),
                typeof(WrongOwner).GetField("_contentById", BindingFlags.NonPublic | BindingFlags.Static)));
        ExpectFailure("registry_shape_unsupported", () =>
            ModelDbRegistryCapture.RequirePinnedField(typeof(WrongOwner),
                typeof(WrongOwner).GetField("_contentById", BindingFlags.NonPublic | BindingFlags.Static)));
        ExpectFailure("registry_shape_unsupported", () =>
            ModelDbRegistryCapture.RequirePinnedField(typeof(ModelDb),
                typeof(WrongType).GetField("_contentById", BindingFlags.NonPublic | BindingFlags.Static)));
        ExpectFailure("registry_shape_unsupported", () =>
            ModelDbRegistryCapture.RequirePinnedField(typeof(ModelDb),
                typeof(PublicRegistry).GetField("_contentById", BindingFlags.Public | BindingFlags.Static)));
        ExpectFailure("registry_shape_unsupported", () =>
            ModelDbRegistryCapture.RequirePinnedField(typeof(ModelDb),
                typeof(InstanceRegistry).GetField("_contentById", BindingFlags.NonPublic | BindingFlags.Instance)));
        ExpectFailure("registry_unavailable", () =>
            ModelDbRegistryCapture.ResolvePinnedRegistry(typeof(ModelDb), valid, null));
    }

    private static void TestStaticFieldReadCanInitializeOwnerType()
    {
        SyntheticStaticConstructorMarker.CallCount = 0;
        FieldInfo field = typeof(UninitializedSyntheticRegistry).GetField(
            "_contentById", BindingFlags.NonPublic | BindingFlags.Static)
            ?? throw new InvalidOperationException("synthetic registry field was not found");
        Check(SyntheticStaticConstructorMarker.CallCount == 0,
            "metadata inspection alone does not initialize the synthetic owner type");
        int modelConstructors = AbstractModel.ConstructorCount;
        _ = field.GetValue(null);
        Check(SyntheticStaticConstructorMarker.CallCount == 1,
            "static reflection read executes the owner type initializer");
        Check(AbstractModel.ConstructorCount == modelConstructors,
            "the synthetic owner initializer does not construct model definitions");

        SyntheticModelDbInitializerMarker.CallCount = 0;
        modelConstructors = AbstractModel.ConstructorCount;
        Dictionary<ModelId, AbstractModel> registry =
            ModelDbRegistryCapture.ReadPinnedRegistry();
        Check(registry.Count == 0 && AbstractModel.ConstructorCount == modelConstructors,
            "the production pinned-field reader initializes only the empty registry");
        Check(SyntheticModelDbInitializerMarker.CallCount == 1,
            "the production pinned-field reader may run the owner type initializer");
        Check(ModelDb.InitializationCalls == 0,
            "the pinned-field reader does not invoke ModelDb.Init");
    }

    private static void TestCaptureCopiesOnlyOwnedValues()
    {
        ResetRegistry();
        var relic = new SyntheticRelic();
        var card = new SyntheticCard();
        ModelDb.AddForTest("relic", "zeta", relic);
        ModelDb.AddForTest("card", "alpha", card);
        int constructedBeforeCapture = AbstractModel.ConstructorCount;

        RegistryProbeSnapshot snapshot = Capture();
        Check(snapshot.Count == 2, "captures current registry count");
        Check(snapshot.Items[0].IdCategory == "card" && snapshot.Items[0].IdEntry == "alpha",
            "sorts copied IDs ordinally");
        Check(snapshot.Items[0].CategoryType == typeof(SyntheticCategory).FullName,
            "copies category type names");
        Check(AbstractModel.ConstructorCount == constructedBeforeCapture,
            "does not construct definitions during capture");
        Check(relic.SemanticTextReads == 0 && relic.DisplayTextReads == 0
            && relic.DebugFormattingCalls == 0 && card.SemanticTextReads == 0
            && card.DisplayTextReads == 0 && card.DebugFormattingCalls == 0,
            "does not read semantic/display text or debug formatting");
        Check(snapshot.HasSameOwnedValues(Capture()),
            "accepts identical consecutive owned-value captures");
        Check(Array.TrueForAll(snapshot.Items[0].GetType().GetProperties(),
                property => property.PropertyType == typeof(string)),
            "owned output contains only four string fields and no host references");
    }

    private static void TestVersionThreadAndCancellationRefuse()
    {
        ResetRegistry();
        ModelDb.AddForTest("card", "one", new SyntheticCard());
        ExpectFailure("game_build_mismatch", () =>
            ModelDbRegistryCapture.Capture(ModelDbRegistryCaptureTestAccess.Registry(),
                ModelDb.GetCategoryType, "v0.108.0", Environment.CurrentManagedThreadId,
                CancellationToken.None, NormalLimits));
        ExpectFailure("wrong_owner_thread", () =>
            ModelDbRegistryCapture.Capture(ModelDbRegistryCaptureTestAccess.Registry(),
                ModelDb.GetCategoryType, ModelDbRegistryCapture.ExpectedGameBuild,
                Environment.CurrentManagedThreadId + 1, CancellationToken.None, NormalLimits));

        using var canceled = new CancellationTokenSource();
        canceled.Cancel();
        ExpectFailure("probe_cancelled", () =>
            ModelDbRegistryCapture.Capture(ModelDbRegistryCaptureTestAccess.Registry(),
                ModelDb.GetCategoryType, ModelDbRegistryCapture.ExpectedGameBuild,
                Environment.CurrentManagedThreadId, canceled.Token, NormalLimits));
    }

    private static void TestLoaderReadinessSignals()
    {
        Check(ProbeReadinessGate.EvaluateManagerState(ModManagerState.None)
            == ProbeReadiness.Wait, "waits until ModManager reaches initialized state");
        Check(ProbeReadinessGate.EvaluateRegistryCount(registryCount: 0)
            == ProbeReadiness.Wait, "an empty registry is not treated as successful initialization");
        Check(ProbeReadinessGate.EvaluateManagerState(ModManagerState.Initialized)
                == ProbeReadiness.Observed
            && ProbeReadinessGate.EvaluateRegistryCount(registryCount: 1)
                == ProbeReadiness.Observed,
            "requires initialized ModManager state and a populated registry");
        Check(ProbeReadinessGate.EvaluateManagerState(ModManagerState.Skipped)
            == ProbeReadiness.Refuse, "refuses when mod loading was skipped");
        ResetRegistry();
        ExpectFailure("registry_not_initialized", () =>
            ModelDbRegistryCapture.Capture(ModelDbRegistryCaptureTestAccess.Registry(),
                ModelDb.GetCategoryType, ModelDbRegistryCapture.ExpectedGameBuild,
                Environment.CurrentManagedThreadId, CancellationToken.None, NormalLimits));
    }

    private static void TestCountStringAndDeadlineBounds()
    {
        ResetRegistry();
        ModelDb.AddForTest("card", "one", new SyntheticCard());
        ModelDb.AddForTest("relic", "two", new SyntheticRelic());
        ExpectFailure("registry_entry_limit_exceeded", () =>
            ModelDbRegistryCapture.Capture(ModelDbRegistryCaptureTestAccess.Registry(),
                ModelDb.GetCategoryType, ModelDbRegistryCapture.ExpectedGameBuild,
                Environment.CurrentManagedThreadId, CancellationToken.None,
                NormalLimits with { MaxEntries = 1 }));
        ExpectFailure("registry_string_limit_exceeded", () =>
            ModelDbRegistryCapture.Capture(ModelDbRegistryCaptureTestAccess.Registry(),
                ModelDb.GetCategoryType, ModelDbRegistryCapture.ExpectedGameBuild,
                Environment.CurrentManagedThreadId, CancellationToken.None,
                NormalLimits with { MaxOwnedStringBytes = 1 }));
        ExpectFailure("registry_id_malformed", () =>
            ModelDbRegistryCapture.Capture(ModelDbRegistryCaptureTestAccess.Registry(),
                ModelDb.GetCategoryType, ModelDbRegistryCapture.ExpectedGameBuild,
                Environment.CurrentManagedThreadId, CancellationToken.None,
                NormalLimits with { MaxIdentityBytes = 1 }));
        ExpectFailure("registry_type_malformed", () =>
            ModelDbRegistryCapture.Capture(ModelDbRegistryCaptureTestAccess.Registry(),
                ModelDb.GetCategoryType, ModelDbRegistryCapture.ExpectedGameBuild,
                Environment.CurrentManagedThreadId, CancellationToken.None,
                NormalLimits with { MaxTypeNameBytes = 1 }));
        ExpectFailure("capture_timeout", () =>
            ModelDbRegistryCapture.Capture(ModelDbRegistryCaptureTestAccess.Registry(),
                ModelDb.GetCategoryType, ModelDbRegistryCapture.ExpectedGameBuild,
                Environment.CurrentManagedThreadId, CancellationToken.None,
                NormalLimits with { MaxCaptureDuration = TimeSpan.FromTicks(1) }));
    }

    private static RegistryProbeSnapshot Capture() =>
        ModelDbRegistryCapture.Capture(
            ModelDbRegistryCaptureTestAccess.Registry(),
            ModelDb.GetCategoryType,
            ModelDbRegistryCapture.ExpectedGameBuild,
            Environment.CurrentManagedThreadId,
            CancellationToken.None,
            NormalLimits);

    private static void ResetRegistry()
    {
        ModelDb.ClearForTest();
        AbstractModel.ResetConstructorCount();
    }

    private static void ExpectFailure(string code, Action action)
    {
        try
        {
            action();
        }
        catch (ProbeFailure failure)
        {
            Check(failure.Code == code, $"refuses with sanitized {code}");
            return;
        }
        throw new InvalidOperationException("expected failure " + code);
    }

    private static void Check(bool condition, string message)
    {
        if (!condition)
            throw new InvalidOperationException(message);
        Console.WriteLine("PASS: " + message);
    }
}

internal static class ModelDbRegistryCaptureTestAccess
{
    internal static Dictionary<ModelId, AbstractModel> Registry()
    {
        FieldInfo field = ModelDbRegistryCapture.RequirePinnedField(
            typeof(ModelDb),
            ModelDbRegistryCapture.FindPinnedField(typeof(ModelDb)));
        object? fieldValue = field.GetValue(null);
        return ModelDbRegistryCapture.ResolvePinnedRegistry(typeof(ModelDb), field, fieldValue);
    }
}
