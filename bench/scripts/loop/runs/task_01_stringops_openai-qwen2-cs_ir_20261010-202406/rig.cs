using System.Reflection;
using System.Text.Json;
using System.Text.Json.Nodes;

var cases = new (string Fn, string ArgsJson, string ExpectedJson)[] {
        ("CompactLabel", "[\"hello world\"]", "\"HW\""),
        ("CompactLabel", "[\"hello brave new world\"]", "\"HBNW\""),
        ("CompactLabel", "[\"\"]", "\"-\""),
        ("CompactLabel", "[\"   \"]", "\"-\""),
        ("CompactLabel", "[\"Hello world\"]", "\"HW\""),
        ("CompactLabel", "[\"a b c d e f g h i\"]", "\"ABCDEF\""),
        ("CompactLabel", "[\"  Multiple   Spaces   Here  \"]", "\"MSH\"")
};

static object? Conv(JsonNode n, Type t)
{
    if (n is JsonValue v)
    {
        if (t == typeof(string)) return (string?)v ?? "";
        if (t == typeof(int)) return (int)v!;
        if (t == typeof(long)) return (long)v!;
        if (t == typeof(bool)) return (bool)v!;
        if (t == typeof(double)) return (double)v!;
        if (t == typeof(char)) return (string?)v ?? "";
    }
    return n.Deserialize(t);
}

static (MethodInfo, object?[]) Resolve(string fn, string argsJson)
{
    var argsNode = (JsonArray)JsonNode.Parse(argsJson)!;
    var cand = typeof(Rig).Assembly.GetTypes()
        .SelectMany(t => t.GetMethods(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static))
        .Where(m => m.Name == fn && m.GetParameters().Length == argsNode.Count)
        .Select(m => (t: m.DeclaringType!, m: m))
        .FirstOrDefault();
    if (cand.m is null)
        throw new MissingMethodException($"fn not found: {fn}({argsNode.Count} args)");
    var argv = argsNode.Select((n, i) => Conv(n, cand.m.GetParameters()[i].ParameterType)).ToArray();
    return (cand.m, argv);
}

int fail = 0;
foreach (var (cf, aJson, eJson) in cases)
{
    try
    {
        var (m, argv) = Resolve(cf, aJson);
        var got = m.Invoke(null, argv);
        var gotJson = JsonSerializer.Serialize(got);
        var ok = JsonNode.DeepEquals(JsonNode.Parse(gotJson)!, JsonNode.Parse(eJson)!);
        if (ok) Console.WriteLine($"PASS {cf}({aJson}) -> {gotJson}");
        else { fail++; Console.Error.WriteLine($"FAIL {cf}({aJson}) -> {gotJson} (expected {eJson})"); }
    }
    catch (Exception err)
    {
        fail++;
        Console.Error.WriteLine($"ERROR {cf}({aJson}): {err.Message}");
    }
}
if (fail > 0) { Console.Error.WriteLine($"RESULT: {fail} failing"); return 1; }
Console.WriteLine("RESULT: ALL PASS");
return 0;
