// SPDX-License-Identifier: MIT

using System.Collections.Immutable;
using System.Reflection.Metadata;

namespace AiAscension.Sts2GameMod.CoopCausalityReflection;

/// <summary>Decodes metadata signatures without resolving or loading referenced assemblies.</summary>
internal sealed class MetadataNames : ISignatureTypeProvider<string, object?>
{
    private readonly MetadataReader _reader;
    private readonly Dictionary<TypeDefinitionHandle, string> _types = new();

    internal MetadataNames(MetadataReader reader) => _reader = reader;

    internal string AssemblyName() => _reader.GetString(_reader.GetAssemblyDefinition().Name);

    internal string TypeName(TypeDefinitionHandle handle)
    {
        if (_types.TryGetValue(handle, out string? name)) return name;
        TypeDefinition type = _reader.GetTypeDefinition(handle);
        string prefix = _reader.GetString(type.Namespace);
        name = (prefix.Length == 0 ? string.Empty : prefix + ".") + _reader.GetString(type.Name);
        _types.Add(handle, name);
        return name;
    }

    internal bool Implements(TypeDefinitionHandle typeHandle, string target)
    {
        var pending = new Stack<EntityHandle>();
        pending.Push(typeHandle);
        var visited = new HashSet<EntityHandle>();
        while (pending.Count > 0)
        {
            EntityHandle current = pending.Pop();
            if (!visited.Add(current)) continue;
            if (TypeName(current) == target) return true;
            if (current.Kind != HandleKind.TypeDefinition) continue;
            TypeDefinition type = _reader.GetTypeDefinition((TypeDefinitionHandle)current);
            if (!type.BaseType.IsNil) pending.Push(type.BaseType);
            foreach (InterfaceImplementationHandle implementation in type.GetInterfaceImplementations())
                pending.Push(_reader.GetInterfaceImplementation(implementation).Interface);
        }
        return false;
    }

    internal string[] Constraints(GenericParameterHandleCollection parameters)
    {
        var result = new List<string>();
        foreach (GenericParameterHandle handle in parameters)
        {
            GenericParameter parameter = _reader.GetGenericParameter(handle);
            foreach (GenericParameterConstraintHandle constraintHandle in parameter.GetConstraints())
                result.Add(TypeName(_reader.GetGenericParameterConstraint(constraintHandle).Type));
        }
        result.Sort(StringComparer.Ordinal);
        return result.ToArray();
    }

    internal static string Format(MethodSignature<string> signature) => signature.ReturnType + " ("
        + string.Join(", ", signature.ParameterTypes) + ")";

    internal string Member(MemberReferenceHandle handle)
    {
        MemberReference member = _reader.GetMemberReference(handle);
        string owner = TypeName(member.Parent);
        return owner + "." + _reader.GetString(member.Name) + " " + Format(member.DecodeMethodSignature(this, null));
    }

    private string TypeName(EntityHandle handle) => handle.Kind switch
    {
        HandleKind.TypeDefinition => TypeName((TypeDefinitionHandle)handle),
        HandleKind.TypeReference => TypeName((TypeReferenceHandle)handle),
        HandleKind.TypeSpecification => GetTypeFromSpecification(_reader, null, (TypeSpecificationHandle)handle, 0),
        _ => "<unresolved-type:" + handle.Kind + ">"
    };

    private string TypeName(TypeReferenceHandle handle)
    {
        TypeReference type = _reader.GetTypeReference(handle);
        string prefix = _reader.GetString(type.Namespace);
        return (prefix.Length == 0 ? string.Empty : prefix + ".") + _reader.GetString(type.Name);
    }

    public string GetArrayType(string elementType, ArrayShape shape) => elementType + "[" + new string(',', Math.Max(0, shape.Rank - 1)) + "]";
    public string GetByReferenceType(string elementType) => elementType + "&";
    public string GetFunctionPointerType(MethodSignature<string> signature) => "fnptr " + Format(signature);
    public string GetGenericInstantiation(string genericType, ImmutableArray<string> typeArguments) => genericType + "<" + string.Join(",", typeArguments) + ">";
    public string GetGenericMethodParameter(object? genericContext, int index) => "!!" + index;
    public string GetGenericTypeParameter(object? genericContext, int index) => "!" + index;
    public string GetModifiedType(string modifier, string unmodifiedType, bool isRequired) => unmodifiedType + (isRequired ? " modreq(" : " modopt(") + modifier + ")";
    public string GetPinnedType(string elementType) => elementType + " pinned";
    public string GetPointerType(string elementType) => elementType + "*";
    public string GetPrimitiveType(PrimitiveTypeCode typeCode) => typeCode.ToString();
    public string GetSZArrayType(string elementType) => elementType + "[]";
    public string GetTypeFromDefinition(MetadataReader reader, TypeDefinitionHandle handle, byte rawTypeKind) => TypeName(handle);
    public string GetTypeFromReference(MetadataReader reader, TypeReferenceHandle handle, byte rawTypeKind) => TypeName(handle);
    public string GetTypeFromSpecification(MetadataReader reader, object? genericContext, TypeSpecificationHandle handle, byte rawTypeKind) => reader.GetTypeSpecification(handle).DecodeSignature(this, genericContext);
}
