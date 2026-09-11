// SPDX-License-Identifier: MIT

using System.Text.Json;

namespace AiAscension.Sts2GameMod.CoopCausalityReflection;

internal static class MetadataCausalityCli
{
    internal static int Execute(string[] args, TextWriter output, TextWriter error)
    {
        if (args.Length != 1)
        {
            error.WriteLine("usage: CoopCausalityReflection <path-to-sts2-data-directory>");
            return 2;
        }

        string assemblyPath;
        try { assemblyPath = Path.Combine(Path.GetFullPath(args[0]), "sts2.dll"); }
        catch
        {
            error.WriteLine("assembly path is invalid");
            return 2;
        }

        if (!File.Exists(assemblyPath))
        {
            error.WriteLine("sts2.dll does not exist in the supplied data directory");
            return 2;
        }

        try
        {
            output.WriteLine(JsonSerializer.Serialize(MetadataCausalityInspector.Inspect(assemblyPath)));
            return 0;
        }
        catch (BadImageFormatException)
        {
            error.WriteLine("assembly has no readable managed metadata");
            return 1;
        }
        catch (IOException)
        {
            error.WriteLine("assembly metadata could not be read");
            return 1;
        }
    }
}
