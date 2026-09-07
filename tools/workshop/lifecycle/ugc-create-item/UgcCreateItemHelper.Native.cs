// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading;

// This is the deliberately separate mutating gate for one new Workshop item.
// It creates an empty item only. Upload and update remain SteamCMD operations
// guarded by tools/workshop/lifecycle/workshop_operator.

internal static partial class Program
{
    private static Journal CreateJournal(Options options, Result result, PackageSnapshot package)
    {
        Journal journal = new()
        {
            Schema = "sts2-steam-ugc-create-item-journal-v2",
            Phase = "pre_call",
            Outcome = "not_started",
            ExpectedAppId = options.ExpectedAppId,
            ActualAppId = result.AppId,
            Platform = options.Platform,
            PackageDirectory = options.PackageDirectory,
            PackageManifestSha256 = package.ManifestSha256,
            PackageContentDigest = package.ContentDigest,
            PackageFiles = package.Files,
            LibraryPath = options.LibraryPath,
            ResolvedLibraryPath = options.ResolvedLibraryPath,
            LibrarySha256 = result.LibrarySha256,
            AppEnvironmentMatches = result.AppEnvironmentMatches,
            AppIdMatches = result.AppIdMatches,
            LoggedOn = result.LoggedOn,
            Subscribed = result.Subscribed,
            JournalPath = options.JournalPath,
            UpdatedAtUtc = DateTimeOffset.UtcNow
        };
        PersistNewJournal(journal, options.JournalPath);
        return journal;
    }

    private static void PersistNewJournal(Journal journal, string path)
    {
        string directory = Path.GetDirectoryName(path) ?? ".";
        Directory.CreateDirectory(directory);
        byte[] data = JsonSerializer.SerializeToUtf8Bytes(journal, JsonOptions);
        using (FileStream stream = new(path, FileMode.CreateNew, FileAccess.Write, FileShare.None, 4096, FileOptions.WriteThrough))
        {
            SetPrivateMode(path);
            stream.Write(data);
            stream.Flush(flushToDisk: true);
        }
    }

    private static void PersistJournal(Journal journal, string path)
    {
        journal.UpdatedAtUtc = DateTimeOffset.UtcNow;
        string directory = Path.GetDirectoryName(path) ?? ".";
        string temporary = Path.Combine(directory, $".{Path.GetFileName(path)}.{Environment.ProcessId}.{Guid.NewGuid():N}.tmp");
        byte[] data = JsonSerializer.SerializeToUtf8Bytes(journal, JsonOptions);
        try
        {
            using (FileStream stream = new(temporary, FileMode.CreateNew, FileAccess.Write, FileShare.None, 4096, FileOptions.WriteThrough))
            {
                SetPrivateMode(temporary);
                stream.Write(data);
                stream.Flush(flushToDisk: true);
            }
            File.Move(temporary, path, overwrite: true);
            temporary = string.Empty;
        }
        finally
        {
            if (temporary.Length != 0)
            {
                try { File.Delete(temporary); } catch { /* Preserve the old journal. */ }
            }
        }
    }

    private static void TryPersistJournal(Journal journal, string path)
    {
        try { PersistJournal(journal, path); } catch { /* The original journal remains a retry barrier. */ }
    }

    private static void SetPrivateMode(string path)
    {
        if (!OperatingSystem.IsWindows())
        {
            File.SetUnixFileMode(path, UnixFileMode.UserRead | UnixFileMode.UserWrite);
        }
    }

    private static void WriteJson(string path, Result value)
    {
        string directory = Path.GetDirectoryName(path) ?? ".";
        Directory.CreateDirectory(directory);
        string temporary = Path.Combine(directory, $".{Path.GetFileName(path)}.{Environment.ProcessId}.{Guid.NewGuid():N}.tmp");
        byte[] data = JsonSerializer.SerializeToUtf8Bytes(value, JsonOptions);
        try
        {
            using (FileStream stream = new(temporary, FileMode.CreateNew, FileAccess.Write, FileShare.None, 4096, FileOptions.WriteThrough))
            {
                SetPrivateMode(temporary);
                stream.Write(data);
                stream.Flush(flushToDisk: true);
            }
            File.Move(temporary, path, overwrite: false);
            temporary = string.Empty;
        }
        finally
        {
            if (temporary.Length != 0)
            {
                try { File.Delete(temporary); } catch { /* Keep any prior result untouched. */ }
            }
        }
    }

    private static void VerifyCreateItemResultLayout()
    {
        if (Marshal.SizeOf<CreateItemResult>() != CreateItemResultSize
            || Marshal.OffsetOf<CreateItemResult>(nameof(CreateItemResult.ResultCode)).ToInt32() != 0
            || Marshal.OffsetOf<CreateItemResult>(nameof(CreateItemResult.PublishedFileId)).ToInt32() != 4
            || Marshal.OffsetOf<CreateItemResult>(nameof(CreateItemResult.UserNeedsLegalAgreement)).ToInt32() != 12)
        {
            throw new InvalidOperationException("unexpected Linux CreateItemResult_t layout; expected size 16 offsets 0,4,12");
        }
    }

    private static bool HasExpectedEnvironment(uint appId)
    {
        string expected = appId.ToString(System.Globalization.CultureInfo.InvariantCulture);
        return string.Equals(Environment.GetEnvironmentVariable("SteamAppId"), expected, StringComparison.Ordinal)
            && string.Equals(Environment.GetEnvironmentVariable("SteamGameId"), expected, StringComparison.Ordinal);
    }

    private static (string Category, bool DiagnosticPresent) ClassifyInitFailure(nint buffer)
    {
        byte[] bytes = ReadBoundedBytes(buffer, ErrorBufferCapacity);
        if (bytes.Length == 0)
        {
            return ("other", false);
        }
        string text = Encoding.UTF8.GetString(bytes);
        if (ContainsAny(text, ["appid", "app id", "steam_appid.txt"])) return ("app_id_missing", true);
        if (ContainsAny(text, ["incompatible", "unsupported", "interface version", "no steamclient"])) return ("incompatible", true);
        if (ContainsAny(text, ["not running", "could not connect", "cannot connect", "must be running"])) return ("client_not_running", true);
        if (ContainsAny(text, ["dlopen", "steam_api", "shared library", "library not found"])) return ("library_error", true);
        return ("other", true);
    }

    private static byte[] ReadBoundedBytes(nint buffer, int capacity)
    {
        if (buffer == 0 || capacity <= 0) return [];
        List<byte> values = new(capacity);
        for (int index = 0; index < capacity; index++)
        {
            byte value = Marshal.ReadByte(buffer, index);
            if (value == 0) break;
            values.Add(value);
        }
        return [.. values];
    }

    private static bool ContainsAny(string value, IEnumerable<string> markers)
        => markers.Any(marker => value.Contains(marker, StringComparison.OrdinalIgnoreCase));

    private static nint RequiredExport(nint library, string name)
        => NativeLibrary.GetExport(library, name);

    private static T Export<T>(nint address) where T : Delegate
        => Marshal.GetDelegateForFunctionPointer<T>(address);

    private static string? LoadedModulePath(nint address)
    {
        if (!OperatingSystem.IsLinux() || Marshal.SizeOf<DlInfo>() != 4 * IntPtr.Size || DlAddr(address, out DlInfo info) == 0 || info.FileName == 0)
        {
            return null;
        }
        return Marshal.PtrToStringAnsi(info.FileName);
    }

    private static bool PathsEqual(string first, string second)
        => string.Equals(Path.GetFullPath(first), Path.GetFullPath(second), StringComparison.Ordinal);

    private static bool IsSha256(string value)
    {
        if (value.Length != 64) return false;
        return value.All(character => character is >= '0' and <= '9' or >= 'a' and <= 'f' or >= 'A' and <= 'F');
    }

    private static bool PathExists(string path)
        => File.Exists(path) || Directory.Exists(path) || new FileInfo(path).LinkTarget is not null;

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull
    };

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate int InitFlatDelegate(nint errorMessage);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate void ShutdownDelegate();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate nint InterfaceGetter();
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate uint GetAppIdDelegate(nint utils);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate byte ByteResult(nint self);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate byte SubscribedResult(nint self, uint appId);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate ulong CreateItemDelegate(nint ugc, uint appId, int fileType);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate byte IsCompletedDelegate(nint utils, ulong call, out byte failed);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate byte GetResultDelegate(nint utils, ulong call, nint callback, int size, int callbackId, out byte failed);
    [UnmanagedFunctionPointer(CallingConvention.Cdecl)] private delegate void RunCallbacksDelegate();

    [StructLayout(LayoutKind.Sequential, Pack = 4)]
    private struct CreateItemResult
    {
        public int ResultCode;
        public ulong PublishedFileId;
        public byte UserNeedsLegalAgreement;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct DlInfo
    {
        public nint FileName;
        public nint BaseAddress;
        public nint SymbolName;
        public nint SymbolAddress;
    }

    [DllImport("libdl.so.2", EntryPoint = "dladdr", CallingConvention = CallingConvention.Cdecl)]
    private static extern int DlAddr(nint address, out DlInfo info);

}
