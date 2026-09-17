// SPDX-License-Identifier: MIT

using System.Globalization;
using System.Reflection;
using System.Reflection.Metadata;
using AiAscension.Sts2GameMod.CoopCausalityReflection;

namespace AiAscension.Sts2GameMod.CheckpointCoverageReflection;

/// <summary>
/// Exact-name member lookup over one assembly's metadata tables. Type names, member names, and
/// member kinds match ordinally; nothing is loaded, instantiated, or executed. Any decode failure
/// or mismatch yields <see cref="CoverageStatus.Unresolved"/> rather than a relaxed match.
/// </summary>
internal sealed class MetadataTypeIndex
{
    private readonly MetadataReader _reader;
    private readonly MetadataNames _names;
    private readonly Dictionary<string, TypeDefinitionHandle> _types = new(StringComparer.Ordinal);

    internal MetadataTypeIndex(MetadataReader reader)
    {
        _reader = reader;
        _names = new MetadataNames(reader);
        foreach (TypeDefinitionHandle handle in reader.TypeDefinitions) _types.TryAdd(FullName(handle), handle);
    }

    internal string AssemblyName => _names.AssemblyName();

    internal MemberResolution Resolve(MemberExpectation expectation)
    {
        if (!_types.TryGetValue(expectation.Type, out TypeDefinitionHandle handle))
            return Unresolved(expectation, "type not found on the pinned assembly");
        try
        {
            TypeDefinition type = _reader.GetTypeDefinition(handle);
            return expectation.Kind switch
            {
                MemberKind.Type => Observed(expectation, "type", Lower(type.Attributes & TypeAttributes.VisibilityMask), null, null),
                MemberKind.Field => ResolveField(expectation, type),
                MemberKind.Property => ResolveProperty(expectation, type),
                MemberKind.Method => ResolveMethod(expectation, type),
                MemberKind.NoMemberContaining => ResolveAbsence(expectation, type),
                _ => Unresolved(expectation, "unsupported member kind"),
            };
        }
        catch (Exception error)
        {
            return Unresolved(expectation, "metadata decode failed: " + error.GetType().Name);
        }
    }

    private MemberResolution ResolveField(MemberExpectation expectation, TypeDefinition type)
    {
        foreach (FieldDefinitionHandle handle in type.GetFields())
        {
            FieldDefinition field = _reader.GetFieldDefinition(handle);
            if (!string.Equals(_reader.GetString(field.Name), expectation.Member, StringComparison.Ordinal)) continue;
            return Observed(expectation, "field", Lower(field.Attributes & FieldAttributes.FieldAccessMask),
                field.DecodeSignature(_names, null), null);
        }
        return Unresolved(expectation, "field not found on the pinned type");
    }

    private MemberResolution ResolveProperty(MemberExpectation expectation, TypeDefinition type)
    {
        foreach (PropertyDefinitionHandle handle in type.GetProperties())
        {
            PropertyDefinition property = _reader.GetPropertyDefinition(handle);
            if (!string.Equals(_reader.GetString(property.Name), expectation.Member, StringComparison.Ordinal)) continue;
            PropertyAccessors accessors = property.GetAccessors();
            MethodDefinitionHandle accessor = accessors.Getter.IsNil ? accessors.Setter : accessors.Getter;
            string visibility = accessor.IsNil ? "none"
                : Lower(_reader.GetMethodDefinition(accessor).Attributes & MethodAttributes.MemberAccessMask);
            return Observed(expectation, "property", visibility, property.DecodeSignature(_names, null).ReturnType,
                accessors.Getter.IsNil ? "no getter" : null);
        }
        return Unresolved(expectation, "property not found on the pinned type");
    }

    private MemberResolution ResolveMethod(MemberExpectation expectation, TypeDefinition type)
    {
        var overloads = new List<string>();
        string? visibility = null;
        foreach (MethodDefinitionHandle handle in type.GetMethods())
        {
            MethodDefinition method = _reader.GetMethodDefinition(handle);
            if (!string.Equals(_reader.GetString(method.Name), expectation.Member, StringComparison.Ordinal)) continue;
            overloads.Add(MetadataNames.Format(method.DecodeSignature(_names, null)));
            string current = Lower(method.Attributes & MethodAttributes.MemberAccessMask);
            visibility = visibility is null || string.Equals(visibility, current, StringComparison.Ordinal) ? current : "mixed";
        }
        if (overloads.Count == 0) return Unresolved(expectation, "method not found on the pinned type");
        overloads.Sort(StringComparer.Ordinal);
        return Observed(expectation, "method", visibility, string.Join(" ; ", overloads),
            "overloads=" + overloads.Count.ToString(CultureInfo.InvariantCulture));
    }

    private MemberResolution ResolveAbsence(MemberExpectation expectation, TypeDefinition type)
    {
        var matches = new List<string>();
        foreach (FieldDefinitionHandle handle in type.GetFields())
            Collect(_reader.GetString(_reader.GetFieldDefinition(handle).Name), expectation.Member, matches);
        foreach (PropertyDefinitionHandle handle in type.GetProperties())
            Collect(_reader.GetString(_reader.GetPropertyDefinition(handle).Name), expectation.Member, matches);
        foreach (MethodDefinitionHandle handle in type.GetMethods())
            Collect(_reader.GetString(_reader.GetMethodDefinition(handle).Name), expectation.Member, matches);
        if (matches.Count > 0)
        {
            matches.Sort(StringComparer.Ordinal);
            return Unresolved(expectation, "members contain the text: " + string.Join(", ", matches));
        }
        return new MemberResolution(expectation.Item, expectation.Type, expectation.Member, "no-member-containing",
            CoverageStatus.MetadataAbsent, null, null, "no field, property, or method name contains the text");
    }

    private static void Collect(string name, string text, List<string> matches)
    {
        if (name.Contains(text, StringComparison.OrdinalIgnoreCase)) matches.Add(name);
    }

    private string FullName(TypeDefinitionHandle handle)
    {
        TypeDefinition type = _reader.GetTypeDefinition(handle);
        TypeDefinitionHandle declaring = type.GetDeclaringType();
        return declaring.IsNil ? _names.TypeName(handle) : FullName(declaring) + "+" + _reader.GetString(type.Name);
    }

    private static string Lower(Enum value) => value.ToString().ToLowerInvariant();

    private static MemberResolution Observed(MemberExpectation expectation, string kind, string? visibility,
        string? declaredType, string? detail) => new(expectation.Item, expectation.Type, expectation.Member, kind,
        CoverageStatus.MetadataObserved, declaredType, visibility, detail);

    private static MemberResolution Unresolved(MemberExpectation expectation, string detail) =>
        new(expectation.Item, expectation.Type, expectation.Member, expectation.Kind.ToString().ToLowerInvariant(),
            CoverageStatus.Unresolved, null, null, detail);
}
