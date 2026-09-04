// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class SettingsPersistence
{
    internal static void WriteAllLines(string path, IEnumerable<string> lines)
    {
        string destination = Path.GetFullPath(path);
        string directory = Path.GetDirectoryName(destination)!;
        Directory.CreateDirectory(directory);
        string temporary = Path.Combine(directory, $".{Path.GetFileName(destination)}.{Guid.NewGuid():N}.tmp");
        bool created = false;
        try
        {
            using (var stream = new FileStream(temporary, FileMode.CreateNew, FileAccess.Write, FileShare.None))
            {
                created = true;
                using (var writer = new StreamWriter(stream, new UTF8Encoding(false), leaveOpen: true))
                {
                    foreach (string line in lines) writer.WriteLine(line);
                    writer.Flush();
                }
                stream.Flush(flushToDisk: true);
            }

            // Same-directory rename publishes only the complete, closed file. Never truncate
            // the existing settings before writing or fall back to copying over them.
            File.Move(temporary, destination, overwrite: true);
        }
        finally
        {
            if (created && File.Exists(temporary)) File.Delete(temporary);
        }
    }
}
