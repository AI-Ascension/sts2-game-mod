// SPDX-License-Identifier: MIT

using System.Globalization;
using System.Reflection;
using System.Reflection.Emit;
using System.Reflection.Metadata;
using System.Reflection.Metadata.Ecma335;
using System.Reflection.PortableExecutable;

namespace AiAscension.Sts2GameMod.CoopCausalityReflection;

internal static partial class MetadataCausalityInspector
{
    private const string NetMessage = "MegaCrit.Sts2.Core.Multiplayer.Serialization.INetMessage";
    private static readonly HashSet<string> SelectedTypes = new(StringComparer.Ordinal)
    {
        NetMessage, "MegaCrit.Sts2.Core.Multiplayer.Game.INetGameService",
        "MegaCrit.Sts2.Core.Multiplayer.Serialization.MessageTypes",
        "MegaCrit.Sts2.Core.Multiplayer.Serialization.NetTypeCache`1",
        "MegaCrit.Sts2.Core.Multiplayer.Serialization.INetMessageSubtypes",
        "MegaCrit.Sts2.Core.Multiplayer.NetMessageBus",
        "MegaCrit.Sts2.Core.GameActions.GameAction",
        "MegaCrit.Sts2.Core.GameActions.Multiplayer.ActionQueueSynchronizer",
        "MegaCrit.Sts2.Core.GameActions.ActionExecutor"
    };
    private static readonly Dictionary<short, OpCode> OpCodes = typeof(OpCodes)
        .GetFields(BindingFlags.Public | BindingFlags.Static).Select(field => (OpCode)field.GetValue(null)!)
        .ToDictionary(opcode => opcode.Value);

    internal static MetadataInspectionReport Inspect(string assemblyPath)
    {
        using FileStream stream = new(assemblyPath, FileMode.Open, FileAccess.Read, FileShare.Read);
        using var pe = new PEReader(stream, PEStreamOptions.LeaveOpen);
        if (!pe.HasMetadata) throw new BadImageFormatException("PE image has no metadata");
        MetadataReader reader = pe.GetMetadataReader();
        var names = new MetadataNames(reader);
        var methods = new List<MetadataMethodReport>();
        var unresolved = new List<string>();
        Dictionary<MethodDefinitionHandle, string> owners = MethodOwners(reader, names, unresolved);
        foreach (TypeDefinitionHandle typeHandle in reader.TypeDefinitions)
        {
            string typeName;
            try { typeName = names.TypeName(typeHandle); }
            catch (Exception error)
            {
                unresolved.Add(Unresolved("type " + MetadataTokens.GetToken(typeHandle).ToString("X8", CultureInfo.InvariantCulture), error));
                continue;
            }
            bool selected = SelectedTypes.Contains(typeName);
            bool messageDerived;
            try { messageDerived = names.Implements(typeHandle, NetMessage); }
            catch (Exception error)
            {
                messageDerived = false;
                unresolved.Add(Unresolved(typeName + " interface hierarchy", error));
            }
            MethodDefinitionHandleCollection typeMethods;
            try { typeMethods = reader.GetTypeDefinition(typeHandle).GetMethods(); }
            catch (Exception error)
            {
                unresolved.Add(Unresolved(typeName + " methods", error));
                continue;
            }
            foreach (MethodDefinitionHandle handle in typeMethods)
            {
                MetadataMethodReport method;
                try { method = Describe(reader, pe, names, owners, unresolved, typeName, handle, selected, messageDerived); }
                catch (Exception error)
                {
                    unresolved.Add(Unresolved(typeName + " method " + MetadataTokens.GetToken(handle).ToString("X8", CultureInfo.InvariantCulture), error));
                    continue;
                }
                if (method.Relevant) methods.Add(method);
                if (method.Unresolved is not null) unresolved.Add(method.Unresolved);
            }
        }
        methods.Sort(static (a, b) => string.CompareOrdinal(a.SortKey, b.SortKey));
        unresolved.Sort(StringComparer.Ordinal);
        return new MetadataInspectionReport("coop-causality-metadata-v2",
            "metadata_only_no_target_assembly_load_or_execution", "unproven",
            AssemblyName(reader, unresolved), methods, unresolved,
            "All selected, message-derived, message-signature, serializer, and registration candidates include every overload, visibility, static member, and inherited-interface declaration.",
            "Unresolved signatures and tokens are retained explicitly. Metadata cannot prove discovery timing, authentication, admission, settlement, or same-peer delivery.");
    }

    private static MetadataMethodReport Describe(MetadataReader reader, PEReader pe, MetadataNames names,
        IReadOnlyDictionary<MethodDefinitionHandle, string> owners, List<string> inventory, string typeName,
        MethodDefinitionHandle handle, bool selected, bool messageDerived)
    {
        string handleText = "0x" + MetadataTokens.GetToken(handle).ToString("X8", CultureInfo.InvariantCulture);
        MethodDefinition method;
        string name;
        try
        {
            method = reader.GetMethodDefinition(handle);
            name = reader.GetString(method.Name);
        }
        catch (Exception error)
        {
            string issue = Unresolved(typeName + " method " + handleText, error);
            inventory.Add(issue);
            return new MetadataMethodReport(typeName, "<unresolved-name>", "<unresolved-signature>",
                Array.Empty<string>(), "<unresolved-attributes>", false, false, selected || messageDerived,
                Array.Empty<string>(), issue);
        }
        string signature;
        var issues = new List<string>();
        string methodLabel = typeName + "." + name + " " + handleText;
        try { signature = MetadataNames.Format(method.DecodeSignature(names, genericContext: null)); }
        catch (Exception error)
        {
            signature = "<unresolved-signature>";
            issues.Add(Unresolved(methodLabel + " signature", error));
        }
        string[] constraints;
        try { constraints = names.Constraints(method.GetGenericParameters()); }
        catch (Exception error)
        {
            constraints = Array.Empty<string>();
            issues.Add(Unresolved(methodLabel + " generic constraints", error));
        }
        string[] calls = Calls(pe, reader, names, owners, method.RelativeVirtualAddress, methodLabel, issues);
        inventory.AddRange(issues);
        bool registration = name is "RegisterMessageHandler" or "UnregisterMessageHandler" or "SendMessage"
            or "SerializeMessage" or "TryDeserializeMessage";
        bool signatureMatch = signature.Contains("INetMessage", StringComparison.Ordinal)
            || constraints.Any(value => value.Contains("INetMessage", StringComparison.Ordinal));
        bool relevant = selected || messageDerived || registration || signatureMatch
            || typeName.Contains("Serialization", StringComparison.Ordinal)
            || typeName.Contains("Message", StringComparison.Ordinal);
        return new MetadataMethodReport(typeName, name, signature, constraints,
            method.Attributes.ToString(), method.Attributes.HasFlag(MethodAttributes.Static),
            method.Attributes.HasFlag(MethodAttributes.Public), relevant, calls,
            issues.Count == 0 ? null : string.Join("; ", issues));
    }

    private static Dictionary<MethodDefinitionHandle, string> MethodOwners(MetadataReader reader,
        MetadataNames names, List<string> inventory)
    {
        var result = new Dictionary<MethodDefinitionHandle, string>();
        foreach (TypeDefinitionHandle type in reader.TypeDefinitions)
        {
            try
            {
                string owner = names.TypeName(type);
                foreach (MethodDefinitionHandle method in reader.GetTypeDefinition(type).GetMethods()) result[method] = owner;
            }
            catch (Exception error)
            {
                inventory.Add(Unresolved("method-owner type 0x" + MetadataTokens.GetToken(type).ToString("X8", CultureInfo.InvariantCulture), error));
            }
        }
        return result;
    }

    private static string[] Calls(PEReader pe, MetadataReader reader, MetadataNames names,
        IReadOnlyDictionary<MethodDefinitionHandle, string> owners, int rva, string methodLabel,
        List<string> issues)
    {
        if (rva == 0) return Array.Empty<string>();
        try
        {
            byte[] il = pe.GetMethodBody(rva).GetILBytes() ?? Array.Empty<byte>();
            var calls = new List<string>();
            for (int offset = 0; offset < il.Length;)
            {
                OpCode? opcode = NextOpcode(il, ref offset);
                if (opcode is null) { issues.Add(methodLabel + " IL opcode: unresolved"); break; }
                int size = OperandSize(opcode.Value.OperandType, il, offset);
                if (size < 0 || offset + size > il.Length) { issues.Add(methodLabel + " IL operand: unresolved"); break; }
                if (opcode.Value.OperandType == OperandType.InlineMethod)
                    calls.Add(Token(reader, names, owners, BitConverter.ToInt32(il.AsSpan(offset, 4)), methodLabel, issues));
                offset += size;
            }
            return calls.Distinct(StringComparer.Ordinal).OrderBy(value => value, StringComparer.Ordinal).ToArray();
        }
        catch (Exception exception)
        {
            issues.Add(Unresolved(methodLabel + " method body", exception));
            return Array.Empty<string>();
        }
    }

    private static string Token(MetadataReader reader, MetadataNames names,
        IReadOnlyDictionary<MethodDefinitionHandle, string> owners, int token, string methodLabel,
        List<string> issues)
    {
        try
        {
            EntityHandle handle = MetadataTokens.EntityHandle(token);
            return handle.Kind switch
            {
                HandleKind.MethodDefinition => MethodOwner(reader, names, owners, (MethodDefinitionHandle)handle, methodLabel, issues),
                HandleKind.MemberReference => Member(reader, names, (MemberReferenceHandle)handle, token),
                HandleKind.MethodSpecification => Specification(reader, names, owners, (MethodSpecificationHandle)handle,
                    token, methodLabel, issues),
                _ => UnsupportedToken(token, methodLabel, issues)
            };
        }
        catch (Exception error)
        {
            issues.Add(Unresolved(methodLabel + " token 0x" + token.ToString("X8", CultureInfo.InvariantCulture), error));
            return "<unresolved-method-token:0x" + token.ToString("X8", CultureInfo.InvariantCulture) + ">";
        }
    }

    private static string MethodOwner(MetadataReader reader, MetadataNames names,
        IReadOnlyDictionary<MethodDefinitionHandle, string> owners,
        MethodDefinitionHandle handle, string methodLabel, List<string> issues)
    {
        if (owners.TryGetValue(handle, out string? owner))
        {
            MethodDefinition method = reader.GetMethodDefinition(handle);
            return owner + "." + reader.GetString(method.Name) + " "
                + MetadataNames.Format(method.DecodeSignature(names, null)) + " [0x"
                + MetadataTokens.GetToken(handle).ToString("X8", CultureInfo.InvariantCulture) + "]";
        }
        issues.Add(methodLabel + " token 0x" + MetadataTokens.GetToken(handle).ToString("X8", CultureInfo.InvariantCulture)
            + ": unresolved method owner");
        return "<unresolved-method-owner>";
    }

    private static string Member(MetadataReader reader, MetadataNames names, MemberReferenceHandle handle, int token) =>
        names.Member(handle) + " [0x" + token.ToString("X8", CultureInfo.InvariantCulture) + "]";

    private static string Specification(MetadataReader reader, MetadataNames names,
        IReadOnlyDictionary<MethodDefinitionHandle, string> owners, MethodSpecificationHandle handle, int token,
        string methodLabel, List<string> issues)
    {
        MethodSpecification specification = reader.GetMethodSpecification(handle);
        string target = specification.Method.Kind switch
        {
            HandleKind.MethodDefinition => MethodOwner(reader, names, owners,
                (MethodDefinitionHandle)specification.Method, methodLabel, issues),
            HandleKind.MemberReference => Member(reader, names, (MemberReferenceHandle)specification.Method,
                MetadataTokens.GetToken(specification.Method)),
            _ => UnsupportedToken(MetadataTokens.GetToken(specification.Method), methodLabel, issues)
        };
        try
        {
            return "method-spec(" + target + "<" + string.Join(",", specification.DecodeSignature(names, null))
                + ">) [0x" + token.ToString("X8", CultureInfo.InvariantCulture) + "]";
        }
        catch (Exception error)
        {
            issues.Add(Unresolved(methodLabel + " method specification 0x" + token.ToString("X8", CultureInfo.InvariantCulture), error));
            return "<unresolved-method-specification:0x" + token.ToString("X8", CultureInfo.InvariantCulture) + ">";
        }
    }

}

internal sealed record MetadataInspectionReport(string SchemaVersion, string Evidence,
    string CausalProvenance, string AssemblyName, IReadOnlyList<MetadataMethodReport> RelevantMethods,
    IReadOnlyList<string> UnresolvedInventory, string EnumerationRule, string Limitation);

internal sealed record MetadataMethodReport(string DeclaringType, string Name, string Signature,
    IReadOnlyList<string> GenericConstraints, string Attributes, bool IsStatic, bool IsPublic,
    bool Relevant, IReadOnlyList<string> CalledMembers, string? Unresolved)
{
    internal string SortKey => DeclaringType + "." + Name + "(" + Signature + ")";
}
