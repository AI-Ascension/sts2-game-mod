using System;
using System.Text.Json;
namespace AiAscension.Sts2GameMod.Runtime;
internal static class ContentManifestWireContract { internal static bool ValidIdentity(string value) => !string.IsNullOrEmpty(value); internal static bool ValidLocale(string value) => !string.IsNullOrEmpty(value); }
public static partial class ModEntry
{
    private const int RuntimeAccepted = 200; private const int RuntimeRejected = 409;
    private static LiveCardCapturedSnapshot _testSnapshot = LiveCardCapturedSnapshot.Unavailable("unset");
    private readonly struct RuntimeContext {
        internal RuntimeContext(string i,string c,string s,string l,string e,string r,string o) { InstanceId=i;CallerId=c;SessionId=s;LeaseId=l;LeaseEpoch=e;CorrelationId=r;Locale=o; }
        internal string InstanceId{get;} internal string CallerId{get;} internal string SessionId{get;} internal string LeaseId{get;} internal string LeaseEpoch{get;} internal string CorrelationId{get;} internal string Locale{get;}
    }
    private static ulong ParseEpoch(string value) => ulong.Parse(value);
    private static string? StringField(System.Text.Json.JsonElement value, string name) =>
        value.TryGetProperty(name, out var result) ? result.GetString() : null;
    private static bool TryAuthorizeRuntimeV2Context(RuntimeContext _, out string error) { error=""; return true; }
    internal static LiveCardCapturedSnapshot ReadRetainedLiveCardSnapshot(string _,string __,ulong ___) => _testSnapshot;
    internal static void SetSnapshot(LiveCardCapturedSnapshot snapshot) => _testSnapshot=snapshot;
    internal static (int,string) Invoke(string body) => ProcessGameInformationQueryWork(new RuntimeContext("instance","caller","session","lease","1","corr","en-US"),body);
    internal static void SeedBinding(LiveCardCapturedSnapshot snapshot) {
        using JsonDocument document=JsonDocument.Parse("{\"project_id\":\"project\",\"run_id\":\"run\",\"episode_id\":\"episode\",\"agent_id\":\"agent\",\"authority_epoch\":1}");
        AssociateLiveCardBinding(new RuntimeContext("instance","caller","session","lease","1","corr","en-US"), document.RootElement, snapshot);
    }
}
