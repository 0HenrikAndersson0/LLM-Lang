public static class CompactLabelOps
{
    public static string CompactLabel(string s)
    {
        var t = s.Trim();
        if (t.Length == 0) return "-";
        var ws = t.Split(' ').Where(w => w.Length > 0).ToArray();
        var label = string.Join("", ws.Select(w => char.ToUpper(w[0])));
        return label.Length > 6 ? label.Substring(0, 6) : label;
    }
}
