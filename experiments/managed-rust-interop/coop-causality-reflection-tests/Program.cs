// SPDX-License-Identifier: MIT

using AiAscension.Sts2GameMod.CoopCausalityReflection;
using System.Buffers.Binary;
using System.Reflection;
using System.Reflection.Emit;
using System.Reflection.Metadata;
using System.Reflection.PortableExecutable;

string fixturePath = Path.Combine(AppContext.BaseDirectory, "CoopCausalityReflectionFixture.dll");
string sentinelPath = Path.Combine(Path.GetTempPath(), "coop-causality-reflection-sentinel-" + Guid.NewGuid());
Environment.SetEnvironmentVariable("COOP_CAUSALITY_REFLECTION_SENTINEL", sentinelPath);

try
{
    MetadataInspectionReport report = MetadataCausalityInspector.Inspect(fixturePath);
    Assert(!File.Exists(sentinelPath), "metadata inspection executed the fixture static constructor");
    Assert(report.RelevantMethods.Any(method => method.DeclaringType.EndsWith("PrivateStaticMessage", StringComparison.Ordinal)
        && method.Name == "Hidden" && method.IsStatic && !method.IsPublic), "private static message method missing");
    Assert(report.RelevantMethods.Any(method => method.Name == "RegisterMessageHandler"), "registration method missing");
    Assert(report.RelevantMethods.Any(method => method.GenericConstraints.Any(value => value.EndsWith("INetMessage", StringComparison.Ordinal))),
        "generic message constraint missing");
    Assert(report.RelevantMethods.Any(method => method.DeclaringType.EndsWith("InheritedMessage", StringComparison.Ordinal)),
        "inherited interface message type missing");
    Assert(report.RelevantMethods.Any(method => method.CalledMembers.Any(value => value.Contains("RegisterMessageHandler", StringComparison.Ordinal))),
        "generic method-specification call missing");
    string[] registrationTargets = report.RelevantMethods.SelectMany(method => method.CalledMembers)
        .Where(value => value.Contains("RegisterMessageHandler", StringComparison.Ordinal)).ToArray();
    Assert(registrationTargets.Length == 2 && registrationTargets.Distinct(StringComparer.Ordinal).Count() == 2
        && registrationTargets.All(value => value.Contains("[0x", StringComparison.Ordinal)),
        "overloaded call targets lack distinct tokenized signatures");
    Assert(report.Evidence == "metadata_only_no_target_assembly_load_or_execution", "incorrect evidence label");
    Assert(report.CausalProvenance == "unproven", "metadata must not claim causal provenance");
    Assert(report.RelevantMethods.Count(method => method.DeclaringType.EndsWith("InterfaceDerivedMessage", StringComparison.Ordinal)
        && method.Name == "Overload") == 2, "interface-derived overloaded members missing");
    AssertCliDirectoryContract(fixturePath);
    AssertCorruptTokenIsAccountedFor(fixturePath);
    GuardSource();
    Console.WriteLine("coop causality metadata inspection tests passed");
    return 0;
}
finally
{
    Environment.SetEnvironmentVariable("COOP_CAUSALITY_REFLECTION_SENTINEL", null);
    if (File.Exists(sentinelPath)) File.Delete(sentinelPath);
}

static void GuardSource()
{
    string root = Directory.GetCurrentDirectory();
    string sourceDirectory = Path.Combine(root, "experiments", "managed-rust-interop", "coop-causality-reflection");
    foreach (string source in Directory.EnumerateFiles(sourceDirectory, "*.cs"))
    {
        string contents = File.ReadAllText(source);
        foreach (string forbidden in new[] { "Assembly.Load", "GetTypes(", "MetadataLoadContext", "AssemblyLoadContext" })
            Assert(!contents.Contains(forbidden, StringComparison.Ordinal), "unsafe reflection API in " + Path.GetFileName(source));
    }
}

static void AssertCliDirectoryContract(string fixturePath)
{
    string directory = Path.Combine(Path.GetTempPath(), "coop-causality-reflection-cli-" + Guid.NewGuid());
    Directory.CreateDirectory(directory);
    try
    {
        File.Copy(fixturePath, Path.Combine(directory, "sts2.dll"));
        using var output = new StringWriter();
        using var error = new StringWriter();
        Assert(MetadataCausalityCli.Execute(new[] { directory }, output, error) == 0, "CLI rejected data directory");
        Assert(output.ToString().Contains("metadata_only_no_target_assembly_load_or_execution", StringComparison.Ordinal),
            "CLI did not inspect sts2.dll in data directory");
        Assert(error.ToString().Length == 0, "CLI reported an unexpected directory-contract error");
    }
    finally { Directory.Delete(directory, recursive: true); }
}

static void AssertCorruptTokenIsAccountedFor(string fixturePath)
{
    string corruptedPath = Path.Combine(Path.GetTempPath(), "coop-causality-reflection-corrupt-" + Guid.NewGuid() + ".dll");
    try
    {
        byte[] bytes = File.ReadAllBytes(fixturePath);
        int operandOffset = FindMethodSpecificationOperand(bytes);
        BinaryPrimitives.WriteInt32LittleEndian(bytes.AsSpan(operandOffset, 4), 0x2b00ffff);
        File.WriteAllBytes(corruptedPath, bytes);
        MetadataInspectionReport report = MetadataCausalityInspector.Inspect(corruptedPath);
        const string marker = "token 0x2B00FFFF";
        Assert(report.UnresolvedInventory.Any(value => value.Contains(marker, StringComparison.Ordinal)),
            "unresolved token missing from aggregate inventory");
        Assert(report.RelevantMethods.Any(method => method.Unresolved?.Contains(marker, StringComparison.Ordinal) == true),
            "unresolved token missing from per-method report");
    }
    finally
    {
        if (File.Exists(corruptedPath)) File.Delete(corruptedPath);
    }
}

static int FindMethodSpecificationOperand(byte[] bytes)
{
    using var stream = new MemoryStream(bytes, writable: false);
    using var pe = new PEReader(stream);
    MetadataReader reader = pe.GetMetadataReader();
    MethodDefinition method = reader.TypeDefinitions.Select(reader.GetTypeDefinition)
        .Where(type => reader.GetString(type.Name) == "PrivateStaticMessage")
        .SelectMany(type => type.GetMethods()).Select(reader.GetMethodDefinition)
        .Single(candidate => reader.GetString(candidate.Name) == "Hidden");
    int rva = method.RelativeVirtualAddress;
    SectionHeader section = pe.PEHeaders.SectionHeaders.Single(header => rva >= header.VirtualAddress
        && rva < header.VirtualAddress + Math.Max(header.VirtualSize, header.SizeOfRawData));
    int methodOffset = rva - section.VirtualAddress + section.PointerToRawData;
    int headerSize = (bytes[methodOffset] & 0x3) == 0x2 ? 1 : ((bytes[methodOffset] | bytes[methodOffset + 1] << 8) >> 12) * 4;
    byte[] il = pe.GetMethodBody(rva).GetILBytes() ?? Array.Empty<byte>();
    for (int offset = 0; offset < il.Length;)
    {
        OpCode? opcode = NextOpcode(il, ref offset);
        Assert(opcode is not null, "fixture has an unrecognized IL opcode");
        int size = OperandSize(opcode!.Value.OperandType, il, offset);
        Assert(size >= 0 && offset + size <= il.Length, "fixture has an invalid IL operand");
        if (opcode.Value.OperandType == OperandType.InlineMethod)
        {
            int token = BinaryPrimitives.ReadInt32LittleEndian(il.AsSpan(offset, 4));
            if ((token & unchecked((int)0xff000000)) == 0x2b000000 && (token & 0x00ffffff) != 0)
                return methodOffset + headerSize + offset;
        }
        offset += size;
    }
    throw new InvalidOperationException("fixture Hidden method has no MethodSpecification operand");
}

static OpCode? NextOpcode(byte[] il, ref int offset)
{
    if (offset >= il.Length) return null;
    short value = il[offset++];
    if (value == 0xfe)
    {
        if (offset >= il.Length) return null;
        value = unchecked((short)(0xfe00 | il[offset++]));
    }
    return typeof(OpCodes).GetFields(BindingFlags.Public | BindingFlags.Static)
        .Select(field => (OpCode)field.GetValue(null)!).FirstOrDefault(opcode => opcode.Value == value);
}

static int OperandSize(OperandType type, byte[] il, int offset) => type switch
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

static int SwitchOperandSize(byte[] il, int offset)
{
    if (offset < 0 || offset + 4 > il.Length) return -1;
    int count = BinaryPrimitives.ReadInt32LittleEndian(il.AsSpan(offset, 4));
    int available = (il.Length - offset - 4) / 4;
    return count < 0 || count > available ? -1 : 4 + count * 4;
}

static void Assert(bool condition, string message)
{
    if (!condition) throw new InvalidOperationException(message);
}
