// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Reflection.Emit;
using System.Reflection.Metadata;

namespace AiAscension.Sts2GameMod.CoopCausalityReflection;

internal static partial class MetadataCausalityInspector
{
    private static string UnsupportedToken(int token, string methodLabel, List<string> issues)
    {
        issues.Add(methodLabel + " token 0x" + token.ToString("X8", System.Globalization.CultureInfo.InvariantCulture)
            + ": unsupported method token");
        return "<unresolved-method-token:0x" + token.ToString("X8", System.Globalization.CultureInfo.InvariantCulture) + ">";
    }

    private static OpCode? NextOpcode(byte[] il, ref int offset)
    {
        if (offset >= il.Length) return null;
        short value = il[offset++];
        if (value == 0xfe)
        {
            if (offset >= il.Length) return null;
            value = unchecked((short)(0xfe00 | il[offset++]));
        }
        return OpCodes.TryGetValue(value, out OpCode opcode) ? opcode : null;
    }

    private static int OperandSize(OperandType type, byte[] il, int offset) => type switch
    {
        OperandType.InlineNone => 0,
        OperandType.ShortInlineBrTarget or OperandType.ShortInlineI or OperandType.ShortInlineVar => 1,
        OperandType.InlineVar => 2,
        OperandType.InlineBrTarget or OperandType.InlineField or OperandType.InlineI or OperandType.InlineMethod
            or OperandType.InlineSig or OperandType.InlineString or OperandType.InlineTok or OperandType.InlineType
            or OperandType.ShortInlineR => 4,
        OperandType.InlineI8 or OperandType.InlineR => 8,
        OperandType.InlineSwitch => SwitchOperandSize(il, offset),
        _ => -1
    };

    private static int SwitchOperandSize(byte[] il, int offset)
    {
        if (offset < 0 || offset + 4 > il.Length) return -1;
        int count = BitConverter.ToInt32(il.AsSpan(offset, 4));
        int available = (il.Length - offset - 4) / 4;
        return count < 0 || count > available ? -1 : 4 + count * 4;
    }

    private static string AssemblyName(MetadataReader reader, List<string> unresolved)
    {
        try { return new MetadataNames(reader).AssemblyName(); }
        catch (Exception error)
        {
            unresolved.Add(Unresolved("assembly name", error));
            return "<unresolved-assembly-name>";
        }
    }

    private static string Unresolved(string scope, Exception error) => scope + ": " + error.GetType().Name;
}
