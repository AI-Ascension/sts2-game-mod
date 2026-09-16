// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Reflection;
using System.Threading;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Debug;
using MegaCrit.Sts2.Core.Modding;
using MegaCrit.Sts2.Core.Models;

namespace AiAscension.Sts2ModelDbRegistryProbe;

[ModInitializer(nameof(Initialize))]
public static class ContentRegistryProbeEntry
{
    private const string EnableVariable = "STS2_MODELDB_REGISTRY_PROBE";
    private const string PinnedHostSha256 =
        "a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52";
    private const int MaxFrames = 300;
    private const int MaxEncodedReportBytes = 12 * 1024 * 1024;
    private static readonly TimeSpan MaxProbeDuration = TimeSpan.FromSeconds(10);
    private static readonly TimeSpan HashTimeout = TimeSpan.FromSeconds(2);
    private static SceneTree? _tree;
    private static Action? _frameCallback;
    private static CancellationTokenSource? _cancellation;
    private static Task<string>? _hostHashTask;
    private static Stopwatch? _elapsed;
    private static RegistryProbeSnapshot? _previous;
    private static string? _verifiedHostHash;
    private static int? _ownerThreadId;
    private static int _frameCount;

    public static void Initialize()
    {
        if (System.Environment.GetEnvironmentVariable(EnableVariable) != "1")
            return;

        if (Engine.GetMainLoop() is not SceneTree tree)
        {
            GD.PrintErr("modeldb_probe: scene_tree_unavailable");
            return;
        }

        _tree = tree;
        _cancellation = new CancellationTokenSource();
        _cancellation.CancelAfter(MaxProbeDuration);
        _elapsed = Stopwatch.StartNew();
        _frameCallback = OnProcessFrame;
        string hostAssemblyPath = typeof(ModelDb).Assembly.Location;
        if (!string.Equals(Path.GetFileName(hostAssemblyPath), "sts2.dll",
                StringComparison.OrdinalIgnoreCase))
        {
            Fail("host_binary_unavailable");
            return;
        }
        CancellationToken probeToken = _cancellation.Token;
        _hostHashTask = Task.Run(
            () => HostBinaryHashVerifier.VerifyAsync(
                hostAssemblyPath,
                PinnedHostSha256,
                HashTimeout,
                probeToken),
            probeToken);
        tree.ProcessFrame += _frameCallback;
    }

    internal static void CancelForOwner() => _cancellation?.Cancel();

    private static void OnProcessFrame()
    {
        CancellationToken token = _cancellation?.Token ?? new CancellationToken(canceled: true);
        try
        {
            int currentThread = System.Environment.CurrentManagedThreadId;
            if (_ownerThreadId is null)
                _ownerThreadId = currentThread;
            else if (_ownerThreadId.Value != currentThread)
                throw new ProbeFailure("owner_thread_changed");

            _frameCount++;
            if (_elapsed is null || _elapsed.Elapsed >= MaxProbeDuration || _frameCount > MaxFrames)
                throw new ProbeFailure("probe_deadline");
            token.ThrowIfCancellationRequested();

            if (_hostHashTask is null || !_hostHashTask.IsCompleted)
                return;
            _verifiedHostHash = _hostHashTask.GetAwaiter().GetResult();

            string? gameBuild = ReadOfficialGameBuild();
            if (gameBuild is null)
                return;
            if (!string.Equals(gameBuild, ModelDbRegistryCapture.ExpectedGameBuild,
                    StringComparison.Ordinal))
            {
                throw new ProbeFailure("game_build_mismatch");
            }

            ModManagerState modManagerState = ModManager.State;
            ProbeReadiness stateReadiness =
                ProbeReadinessGate.EvaluateManagerState(modManagerState);
            if (stateReadiness == ProbeReadiness.Refuse)
                throw new ProbeFailure("mod_manager_skipped");
            if (stateReadiness != ProbeReadiness.Ready)
                return;

            Dictionary<ModelId, AbstractModel> registry = ReadPinnedRegistry();
            ProbeReadiness registryReadiness =
                ProbeReadinessGate.EvaluateRegistryCount(registry.Count);
            if (registryReadiness != ProbeReadiness.Ready)
                return;

            RegistryProbeSnapshot current = ModelDbRegistryCapture.Capture(
                registry,
                ModelDb.GetCategoryType,
                gameBuild,
                _ownerThreadId.Value,
                token);
            if (_previous is null || !_previous.HasSameOwnedValues(current))
            {
                _previous = current;
                return;
            }

            EmitReport(current, _verifiedHostHash);
            Finish();
        }
        catch (OperationCanceledException)
        {
            Fail(ProbeDeadlineReached() ? "probe_deadline" : "probe_cancelled");
        }
        catch (ProbeFailure failure)
        {
            if (failure.Code is "registry_unavailable" or "registry_not_initialized")
                return;
            string code = failure.Code == "probe_cancelled" && ProbeDeadlineReached()
                ? "probe_deadline"
                : failure.Code;
            Fail(code);
        }
        catch (Exception)
        {
            Fail("probe_failed");
        }
    }

    private static Dictionary<ModelId, AbstractModel> ReadPinnedRegistry()
    {
        Type modelDbType = typeof(ModelDb);
        FieldInfo field = ModelDbRegistryCapture.RequirePinnedField(
            modelDbType,
            ModelDbRegistryCapture.FindPinnedField(modelDbType));
        object? fieldValue;
        try
        {
            fieldValue = field.GetValue(null);
        }
        catch (Exception)
        {
            throw new ProbeFailure("registry_unavailable");
        }

        return ModelDbRegistryCapture.ResolvePinnedRegistry(
            modelDbType, field, fieldValue);
    }

    private static string? ReadOfficialGameBuild()
    {
        try
        {
            return ReleaseInfoManager.Instance?.ReleaseInfo?.Version;
        }
        catch (Exception)
        {
            return null;
        }
    }

    private static void EmitReport(RegistryProbeSnapshot snapshot, string hostHash)
    {
        string encoded = snapshot.SerializeBoundedReport(hostHash, MaxEncodedReportBytes);
        GD.Print(encoded);
    }

    private static void Fail(string code)
    {
        GD.PrintErr("modeldb_probe: " + code);
        Finish();
    }

    private static bool ProbeDeadlineReached() =>
        _elapsed is null || _elapsed.Elapsed >= MaxProbeDuration;

    private static void Finish()
    {
        if (_tree is not null && _frameCallback is not null)
            _tree.ProcessFrame -= _frameCallback;
        _cancellation?.Cancel();
        _cancellation?.Dispose();
        _tree = null;
        _frameCallback = null;
        _cancellation = null;
        _hostHashTask = null;
        _elapsed = null;
        _previous = null;
        _verifiedHostHash = null;
        _ownerThreadId = null;
        _frameCount = 0;
    }
}
