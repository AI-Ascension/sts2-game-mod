// SPDX-License-Identifier: MIT

using AiAscension.Sts2GameMod.Runtime;

internal static class PersistenceTests
{
    private static readonly string[] InitialLines = { "runtime_enabled=false", "profile_id=2" };
    private static readonly string[] ReplacementLines = { "runtime_enabled=true", "profile_id=3" };
    private static readonly string[] BlockedLines = { "new data" };

    internal static void Run()
    {
        string directory = Path.Combine(Path.GetTempPath(), $"sts2-settings-test-{Guid.NewGuid():N}");
        Directory.CreateDirectory(directory);
        try
        {
            string destination = Path.Combine(directory, "settings");
            SettingsPersistence.WriteAllLines(destination, InitialLines);
            byte[] original = File.ReadAllBytes(destination);
            bool failed = false;
            try { SettingsPersistence.WriteAllLines(destination, BrokenLines()); }
            catch (IOException) { failed = true; }
            Assert(failed, "Partial serialization failure must propagate");
            Assert(original.SequenceEqual(File.ReadAllBytes(destination)), "Previous settings survive partial write");
            Assert(Directory.GetFiles(directory).Length == 1, "Failed write removes its temporary file");

            SettingsPersistence.WriteAllLines(destination, ReplacementLines);
            Assert(File.ReadAllLines(destination).SequenceEqual(ReplacementLines),
                "Successful replacement publishes complete settings");
            Assert(Directory.GetFiles(directory).Length == 1, "Successful replacement leaves no temporary file");

            string blocked = Path.Combine(directory, "blocked");
            Directory.CreateDirectory(blocked);
            failed = false;
            try { SettingsPersistence.WriteAllLines(blocked, BlockedLines); }
            catch (IOException) { failed = true; }
            catch (UnauthorizedAccessException) { failed = true; }
            Assert(failed && Directory.Exists(blocked), "Failed rename preserves destination directory");
            Assert(Directory.GetFiles(directory).Length == 1, "Failed rename removes its temporary file");
        }
        finally { Directory.Delete(directory, recursive: true); }
        Console.WriteLine("Synthetic atomic settings persistence checks passed; only isolated temporary files used.");
    }

    private static IEnumerable<string> BrokenLines()
    {
        // Exceed StreamWriter's buffer before failing, so this exercises partial disk output.
        yield return new string('x', 16384);
        throw new IOException("Synthetic serialization failure");
    }

    private static void Assert(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
