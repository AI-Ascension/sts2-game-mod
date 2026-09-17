using System;
using System.Collections.Generic;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;
var card = new LiveCardCapturedCard("card-1", LiveCardField<string>.Available("ironclad:strike"), LiveCardField<string>.Available("manifest"), LiveCardField<string>.Available("owner"), LiveCardField<LiveCardLocation>.Available(new(LiveCardZone.Hand,0)), LiveCardField<ushort>.Available(0), LiveCardField<string>.NotObserved(), LiveCardField<string>.NotObserved(), LiveCardField<string>.Available("Strike"), LiveCardField<bool>.Available(false), LiveCardField<int>.Available(1), LiveCardField<int>.NotObserved(), LiveCardField<int>.NotObserved(), LiveCardField<IReadOnlyList<string>>.NotObserved(), LiveCardField<IReadOnlyList<string>>.NotObserved(), LiveCardField<IReadOnlyDictionary<string,string>>.NotObserved());
var snapshot = new LiveCardCapturedSnapshot("instance","manifest","source","run","snap",7,9,new[]{card},true,null);
ModEntry.SetSnapshot(snapshot);
ModEntry.SeedBinding(snapshot);
string wire = JsonSerializer.Serialize(new { protocol_version="game-information-query-v1", schema_digest="376845b0c86b4afcd2c79ffba753eb7e7e416f5410da26b4dae970cfee2221d9", provenance=new {artifact="sts2-protocol/game-information-query-v1",source="schemas/game-information-query-v1.schema.json",generator="hand-authored"}, correlation_id="corr", kind="query_request", query=new {query_kind="detail",entity_kind="card",fields=new[]{"display_name"},limits=new {item_bytes=4096,page_bytes=65536,page_items=1,text_bytes=4096},binding=new {mode="live",content_manifest_id="manifest",locale="en-US",snapshot_ref=new {snapshot_id="snap",state_generation=9,instance_ref=new {instance_id="instance",run_id="run",epoch=7,entity_kind="card",entity_id="card-1"}}},target=new {definition_ref=new {content_manifest_id="manifest",entity_kind="card",namespaced_id="ironclad:strike",variant=(string?)null},instance_ref=new {instance_id="instance",run_id="run",epoch=7,entity_kind="card",entity_id="card-1"}},parent_observation=new {state_generation=9,snapshot_ref=new {snapshot_id="snap",state_generation=9,instance_ref=new {instance_id="instance",run_id="run",epoch=7,entity_kind="card",entity_id="card-1"}},instance_ref=new {instance_id="instance",run_id="run",epoch=7,entity_kind="card",entity_id="card-1"}}}});
var result=ModEntry.Invoke(wire); if(result.Item1!=200) throw new InvalidOperationException("expected bound live detail: "+result.Item1+" "+result.Item2); Console.WriteLine("PASS: real handler returns bound retained detail");
if (!result.Item2.Contains("\"kind\":\"query_response\"",StringComparison.Ordinal)) throw new InvalidOperationException("missing response envelope");
Console.WriteLine("PASS: response has query envelope");
foreach (string mutation in new[]{"\"run\":\"foreign\"","\"epoch\":8","\"snapshot_id\":\"foreign\"","\"state_generation\":10","\"entity_id\":\"foreign\"","\"entity_kind\":\"relic\""})
{
    var rejected=ModEntry.Invoke(wire.Replace(mutation.Split(':')[0]+":",mutation.Split(':')[0]+":",StringComparison.Ordinal));
}
var duplicate=ModEntry.Invoke(wire.Replace("\"kind\":\"query_request\"","\"kind\":\"query_request\",\"kind\":\"query_request\"",StringComparison.Ordinal));
if(duplicate.Item1!=400) throw new InvalidOperationException("duplicate accepted"); Console.WriteLine("PASS: duplicate key rejected");
