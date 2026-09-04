// SPDX-License-Identifier: MIT
using System;
using System.IO;
using AiAscension.Sts2GameMod.Runtime;
namespace AiAscension.Sts2GameMod.WorkshopValidationProbe;
internal static class NegativeCases
{
    internal static void Run(string root, Action validate)
    {
        string path = Path.Combine(root, WorkshopPackageValidator.ManifestFileName);
        string original = File.ReadAllText(path);
        File.WriteAllText(path, original.Replace("\"consumer_app_id\":480",
            "\"consumer_app_id\":999,\"consumer_app_id\":480", StringComparison.Ordinal));
        Expect("malformed_manifest", validate);
        File.WriteAllText(path, original.Replace("\"role\":\"managed_assembly\"",
            "\"role\":\"native_library\",\"role\":\"managed_assembly\"", StringComparison.Ordinal));
        Expect("malformed_manifest", validate);
        File.WriteAllText(path, new string(' ', 8 * 1024 * 1024));
        long before = GC.GetAllocatedBytesForCurrentThread();
        Expect("manifest_too_large", validate);
        if (GC.GetAllocatedBytesForCurrentThread() - before > 512 * 1024)
            throw new InvalidOperationException("Oversized manifest allocated beyond its fixed read bound.");
        File.WriteAllText(path, original);
        string link = root + "-link";
        try
        {
            try { Directory.CreateSymbolicLink(link, root); }
            catch (Exception exception) when (exception is UnauthorizedAccessException or PlatformNotSupportedException)
            {
                Console.WriteLine("UNVERIFIED: symlink test unavailable: " + exception.GetType().Name);
                return;
            }
            Expect("reparse_point", () => WorkshopPackageValidator.ValidateDirectory(
                link, 480, 123456789, "0.107.1", "windows-x86_64"));
        }
        finally { if (Directory.Exists(link)) Directory.Delete(link); }
    }
    private static void Expect(string code, Action action)
    {
        try { action(); }
        catch (WorkshopPackageValidationException exception) when (exception.Code == code) { return; }
        throw new InvalidOperationException("Expected package rejection: " + code);
    }
}
