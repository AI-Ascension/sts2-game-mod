using System.Reflection;
using System.Reflection.Metadata;
using System.Reflection.PortableExecutable;
using System.Collections.Immutable;

using var stream = File.OpenRead(args[0]);
using var pe = new PEReader(stream);
var reader = pe.GetMetadataReader();
foreach (var handle in reader.TypeDefinitions) {
  var type = reader.GetTypeDefinition(handle);
  var name = reader.GetString(type.Namespace) + "." + reader.GetString(type.Name);
  if (!args.Skip(1).Any(pattern => name.Contains(pattern, StringComparison.OrdinalIgnoreCase))) continue;
  Console.WriteLine(name);
  foreach (var methodHandle in type.GetMethods()) {
    var method = reader.GetMethodDefinition(methodHandle);
    if ((method.Attributes & MethodAttributes.MemberAccessMask) != MethodAttributes.Public) continue;
    var signature = method.DecodeSignature(new TextProvider(), "");
    Console.WriteLine($"  method {signature.ReturnType} {reader.GetString(method.Name)}({string.Join(",", signature.ParameterTypes)})");
  }
  foreach (var fieldHandle in type.GetFields()) {
    var field = reader.GetFieldDefinition(fieldHandle);
    Console.WriteLine($"  field {(field.Attributes & FieldAttributes.FieldAccessMask)} {field.DecodeSignature(new TextProvider(), "")} {reader.GetString(field.Name)}");
  }
  foreach (var propertyHandle in type.GetProperties()) {
    var property = reader.GetPropertyDefinition(propertyHandle);
    Console.WriteLine($"  property {reader.GetString(property.Name)}");
  }
}

sealed class TextProvider : ISignatureTypeProvider<string, string> {
  public string GetArrayType(string e, ArrayShape s) => e + "[]";
  public string GetByReferenceType(string e) => e + "&";
  public string GetFunctionPointerType(MethodSignature<string> s) => "fn";
  public string GetGenericInstantiation(string g, ImmutableArray<string> a) => g + "<" + string.Join(",", a) + ">";
  public string GetGenericMethodParameter(string c, int i) => "!!" + i;
  public string GetGenericTypeParameter(string c, int i) => "!" + i;
  public string GetModifiedType(string m, string u, bool r) => u;
  public string GetPinnedType(string e) => e;
  public string GetPointerType(string e) => e + "*";
  public string GetPrimitiveType(PrimitiveTypeCode t) => t.ToString();
  public string GetSZArrayType(string e) => e + "[]";
  public string GetTypeFromDefinition(MetadataReader r, TypeDefinitionHandle h, byte raw) { var t=r.GetTypeDefinition(h); return r.GetString(t.Namespace)+"."+r.GetString(t.Name); }
  public string GetTypeFromReference(MetadataReader r, TypeReferenceHandle h, byte raw) { var t=r.GetTypeReference(h); return r.GetString(t.Namespace)+"."+r.GetString(t.Name); }
  public string GetTypeFromSpecification(MetadataReader r, string c, TypeSpecificationHandle h, byte raw) => r.GetTypeSpecification(h).DecodeSignature(this,c);
}
